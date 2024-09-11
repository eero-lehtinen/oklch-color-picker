#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};

use bevy_color::{ColorToComponents, ColorToPacked, Oklcha, Srgba};
use const_format::concatcp;
use eframe::{
    egui::{self, Color32, Mesh, Pos2, Stroke},
    egui_glow,
    glow::{self, HasContext},
};
use enum_map::{enum_map, Enum, EnumMap};
use gamut::gamut_clip_preserve_chroma;

mod gamut;

fn main() {
    let native_options = eframe::NativeOptions {
        multisampling: 4,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "App",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
    .unwrap();
}

fn setup_custom_fonts(ctx: &egui::Context) {
    // Start with the default fonts (we will be adding to them rather than replacing them).
    let mut fonts = egui::FontDefinitions::default();

    // Install my own font (maybe supporting non-latin characters).
    // .ttf and .otf files supported.
    fonts.font_data.insert(
        "my_font".to_owned(),
        egui::FontData::from_static(include_bytes!("../src/InterVariable.ttf")),
    );

    // Put my font first (highest priority) for proportional text:
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "my_font".to_owned());

    // Put my font as last fallback for monospace:
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("my_font".to_owned());

    // Tell egui to use these fonts:
    ctx.set_fonts(fonts);
}

#[derive(Enum, Default, Clone, Copy)]
enum ProgramKind {
    #[default]
    Picker,
    Picker2,
    Hue,
    Lightness,
    Chroma,
    Alpha,
    Final,
}

struct GlowProgram {
    kind: ProgramKind,
    program: glow::Program,
    vertex_array: glow::VertexArray,
}

impl GlowProgram {
    fn new(gl: &glow::Context, kind: ProgramKind) -> Self {
        unsafe {
            let program = gl.create_program().unwrap();
            let vert_shader_source = include_str!("./shaders/quad_vert.glsl");

            let frag_shader_source = match kind {
                ProgramKind::Picker => concat!(
                    include_str!("shaders/functions.glsl"),
                    include_str!("shaders/picker_frag.glsl")
                ),
                ProgramKind::Picker2 => concat!(
                    include_str!("shaders/functions.glsl"),
                    include_str!("shaders/picker2_frag.glsl")
                ),
                ProgramKind::Hue => concat!(
                    include_str!("shaders/functions.glsl"),
                    include_str!("shaders/hue_frag.glsl")
                ),
                ProgramKind::Lightness => concat!(
                    include_str!("shaders/functions.glsl"),
                    include_str!("shaders/lightness_frag.glsl")
                ),
                ProgramKind::Chroma => concat!(
                    include_str!("shaders/functions.glsl"),
                    include_str!("shaders/chroma_frag.glsl")
                ),
                ProgramKind::Alpha => concat!(
                    include_str!("shaders/functions.glsl"),
                    include_str!("shaders/alpha_frag.glsl")
                ),
                ProgramKind::Final => concat!(
                    include_str!("shaders/functions.glsl"),
                    include_str!("shaders/final_frag.glsl")
                ),
            };

            let shader_sources = [
                (glow::VERTEX_SHADER, vert_shader_source),
                (glow::FRAGMENT_SHADER, frag_shader_source),
            ];

            let shaders: Vec<_> = shader_sources
                .iter()
                .map(|(shader_type, shader_source)| {
                    let shader = gl
                        .create_shader(*shader_type)
                        .expect("Cannot create shader");
                    gl.shader_source(shader, shader_source);
                    gl.compile_shader(shader);
                    assert!(
                        gl.get_shader_compile_status(shader),
                        "Failed to compile {shader_type}: {}",
                        gl.get_shader_info_log(shader)
                    );
                    gl.attach_shader(program, shader);
                    shader
                })
                .collect();

            gl.link_program(program);
            assert!(
                gl.get_program_link_status(program),
                "{}",
                gl.get_program_info_log(program)
            );

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            let vertex_array = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");

            Self {
                kind,
                program,
                vertex_array,
            }
        }
    }

    fn destroy(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vertex_array);
        }
    }

    fn paint(&self, gl: &glow::Context, color: Oklcha, fallback_color: [f32; 4]) {
        unsafe {
            let set_uni_f32 = |name: &str, value: f32| {
                gl.uniform_1_f32(gl.get_uniform_location(self.program, name).as_ref(), value);
            };
            gl.use_program(Some(self.program));
            match self.kind {
                ProgramKind::Picker => {
                    set_uni_f32("hue", color.hue);
                }
                ProgramKind::Picker2 => {
                    set_uni_f32("lightness", color.lightness);
                }
                ProgramKind::Hue => {}
                ProgramKind::Lightness => {
                    set_uni_f32("hue", color.hue);
                    set_uni_f32("chroma", color.chroma);
                }
                ProgramKind::Chroma => {
                    set_uni_f32("hue", color.hue);
                    set_uni_f32("lightness", color.lightness);
                }
                ProgramKind::Alpha => {
                    gl.uniform_3_f32_slice(
                        gl.get_uniform_location(self.program, "color").as_ref(),
                        &fallback_color[0..3][..],
                    );
                }

                ProgramKind::Final => {
                    gl.uniform_4_f32_slice(
                        gl.get_uniform_location(self.program, "color").as_ref(),
                        &fallback_color[..],
                    );
                }
            }
            gl.bind_vertex_array(Some(self.vertex_array));

            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
    }
}

fn lerp(v0: f32, v1: f32, t: f32) -> f32 {
    (1. - t) * v0 + t * v1
}

fn map(input: f32, from: (f32, f32), to: (f32, f32)) -> f32 {
    (to.1 - to.0) * (input - from.0) / (from.1 - from.0) + to.0
}

struct App {
    color: Oklcha,
    programs: EnumMap<ProgramKind, Arc<Mutex<GlowProgram>>>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_fonts(&cc.egui_ctx);

        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        let gl = cc.gl.as_ref().unwrap();

        let prog = |kind: ProgramKind| Arc::new(Mutex::new(GlowProgram::new(gl, kind)));
        Self {
            color: Oklcha::new(0.8, 0.1, 0.0, 1.0),
            programs: enum_map! {
                ProgramKind::Picker => prog(ProgramKind::Picker),
                ProgramKind::Picker2 => prog(ProgramKind::Picker2),
                ProgramKind::Hue => prog(ProgramKind::Hue),
                ProgramKind::Lightness => prog(ProgramKind::Lightness),
                ProgramKind::Chroma => prog(ProgramKind::Chroma),
                ProgramKind::Alpha => prog(ProgramKind::Alpha),
                ProgramKind::Final => prog(ProgramKind::Final),
            },
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let fallback_color =
            Srgba::from(gamut_clip_preserve_chroma(self.color.into())).to_f32_array();

        ui.vertical_centered_justified(|ui| {
            ui.horizontal(|ui| {});
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                let size = egui::Vec2::new(ui.available_width(), 600.);
                let (rect, response) = ui.allocate_at_least(size, egui::Sense::drag());

                if let Some(pos) = response.interact_pointer_pos() {
                    self.color.lightness =
                        map(pos.x, (rect.left(), rect.right()), (0., 1.)).clamp(0.0, 1.0);
                    self.color.chroma = map(pos.y, (rect.top(), rect.bottom()), (CHROMA_MAX, 0.))
                        .clamp(0.0, CHROMA_MAX);
                }

                let color = self.color;

                let picker = Arc::clone(&self.programs[ProgramKind::Picker]);

                let callback = egui::PaintCallback {
                    rect,
                    callback: Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
                        picker
                            .lock()
                            .unwrap()
                            .paint(painter.gl(), color, fallback_color);
                    })),
                };
                ui.painter().add(callback);

                let painter = ui.painter_at(rect);

                let line_x = lerp(rect.left(), rect.right(), color.lightness);
                let line_y = lerp(rect.bottom(), rect.top(), color.chroma / CHROMA_MAX);

                painter.add(egui::Shape::line_segment(
                    [
                        Pos2::new(line_x, rect.top()),
                        Pos2::new(line_x, rect.bottom()),
                    ],
                    Stroke::new(1.0, LINE_COLOR),
                ));
                painter.add(egui::Shape::line_segment(
                    [
                        Pos2::new(rect.left(), line_y),
                        Pos2::new(rect.right(), line_y),
                    ],
                    Stroke::new(1.0, LINE_COLOR),
                ));
            });

            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(egui::Vec2::new(600.0, 50.), egui::Sense::drag());

                if let Some(pos) = response.interact_pointer_pos() {
                    self.color.lightness =
                        map(pos.x, (rect.left(), rect.right()), (0., 1.)).clamp(0.0, 1.0);
                }

                let color = self.color;

                let hue = Arc::clone(&self.programs[ProgramKind::Lightness]);

                let callback = egui::PaintCallback {
                    rect,
                    callback: Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
                        hue.lock()
                            .unwrap()
                            .paint(painter.gl(), color, fallback_color);
                    })),
                };
                ui.painter().add(callback);

                let line_x = lerp(rect.left(), rect.right(), color.lightness);

                ui.painter().add(egui::Shape::line_segment(
                    [
                        Pos2::new(line_x, rect.top()),
                        Pos2::new(line_x, rect.bottom()),
                    ],
                    Stroke::new(2.0, LINE_COLOR),
                ));
            });

            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(egui::Vec2::new(600.0, 50.), egui::Sense::drag());

                if let Some(pos) = response.interact_pointer_pos() {
                    self.color.chroma = map(pos.x, (rect.left(), rect.right()), (0., CHROMA_MAX))
                        .clamp(0.0, CHROMA_MAX);
                }

                let color = self.color;

                let hue = Arc::clone(&self.programs[ProgramKind::Chroma]);

                let callback = egui::PaintCallback {
                    rect,
                    callback: Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
                        hue.lock()
                            .unwrap()
                            .paint(painter.gl(), color, fallback_color);
                    })),
                };
                ui.painter().add(callback);

                let line_x = lerp(rect.left(), rect.right(), color.chroma / CHROMA_MAX);

                ui.painter().add(egui::Shape::line_segment(
                    [
                        Pos2::new(line_x, rect.top()),
                        Pos2::new(line_x, rect.bottom()),
                    ],
                    Stroke::new(2.0, LINE_COLOR),
                ));
            });

            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(egui::Vec2::new(600.0, 50.), egui::Sense::drag());

                if let Some(pos) = response.interact_pointer_pos() {
                    self.color.hue =
                        map(pos.x, (rect.left(), rect.right()), (0., 360.)).clamp(0.0, 360.);
                }

                let color = self.color;

                let hue = Arc::clone(&self.programs[ProgramKind::Hue]);

                let callback = egui::PaintCallback {
                    rect,
                    callback: Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
                        hue.lock()
                            .unwrap()
                            .paint(painter.gl(), color, fallback_color);
                    })),
                };
                ui.painter().add(callback);

                let line_x = lerp(rect.left(), rect.right(), color.hue / 360.);

                ui.painter().add(egui::Shape::line_segment(
                    [
                        Pos2::new(line_x, rect.top()),
                        Pos2::new(line_x, rect.bottom()),
                    ],
                    Stroke::new(2.0, LINE_COLOR),
                ));
            });

            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(egui::Vec2::new(600.0, 50.), egui::Sense::drag());

                if let Some(pos) = response.interact_pointer_pos() {
                    self.color.alpha =
                        map(pos.x, (rect.left(), rect.right()), (0., 1.)).clamp(0.0, 1.0);
                }

                let color = self.color;

                let hue = Arc::clone(&self.programs[ProgramKind::Alpha]);

                let callback = egui::PaintCallback {
                    rect,
                    callback: Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
                        hue.lock()
                            .unwrap()
                            .paint(painter.gl(), color, fallback_color);
                    })),
                };
                ui.painter().add(callback);

                let line_x = lerp(rect.left(), rect.right(), color.alpha);

                ui.painter().add(egui::Shape::line_segment(
                    [
                        Pos2::new(line_x, rect.top()),
                        Pos2::new(line_x, rect.bottom()),
                    ],
                    Stroke::new(2.0, LINE_COLOR),
                ));
            });

            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::Vec2::new(600.0, 50.), egui::Sense::drag());

                let color = self.color;
                let hue = Arc::clone(&self.programs[ProgramKind::Final]);
                let callback = egui::PaintCallback {
                    rect,
                    callback: Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
                        hue.lock()
                            .unwrap()
                            .paint(painter.gl(), color, fallback_color);
                    })),
                };
                ui.painter().add(callback);
            });

            // ctx.send_viewport_cmd(ViewportCommand::Close);
            //
            let color_text = format!(
                "oklch({:.2}% {:.4} {:.2}{})",
                self.color.lightness * 100.,
                self.color.chroma,
                self.color.hue,
                if self.color.alpha == 1. {
                    String::new()
                } else {
                    format!(" / {:.2}%", self.color.alpha * 100.)
                },
            );
            ui.label(color_text);
            ui.label(Srgba::from_f32_array(fallback_color).to_hex());
        });
    }
}

const CHROMA_MAX: f32 = 0.33;

const LINE_COLOR: Color32 = Color32::from_gray(10);

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui(ui);
        });
    }

    fn on_exit(&mut self, gl: Option<&glow::Context>) {
        if let Some(gl) = gl {
            for prog in self.programs.values() {
                prog.lock().unwrap().destroy(gl);
            }
        }
    }
}
