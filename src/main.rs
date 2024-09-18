#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    env,
    fmt::{Debug, Display},
    process::ExitCode,
    sync::{Arc, Mutex},
    time::Instant,
};

use bevy_color::{ColorToPacked, LinearRgba, Oklcha, Srgba};
use clap::Parser as _;
use cli::{Cli, CliColorFormat};
use eframe::{
    egui::{self, ahash::HashMap, Color32, DragValue, Pos2, RichText, Stroke, Vec2},
    egui_glow,
    glow::{self},
};
use egui::{ViewportBuilder, Widget};
use egui_extras::{Size, StripBuilder};
use formats::{
    format_color, parse_color, parse_color_unknown_format, ColorFormat, CssColorFormat,
    RawColorFormat,
};
use gamut::{gamut_clip_preserve_chroma, Oklrcha};
use gl_programs::{GlowProgram, ProgramKind};
use once_cell::sync::Lazy;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use strum::IntoEnumIterator;

mod cli;
mod formats;
mod gamut;
mod gl_programs;

fn main() -> ExitCode {
    log_startup_init();

    let cli = Cli::parse();
    log_startup_time("Cli parse");

    let (color, format, use_alpha) = match (cli.color, cli.format) {
        (Some(color_string), Some(cli_format)) => {
            let Some((color, use_alpha)) = parse_color(&color_string, cli_format.into()) else {
                eprintln!(
                    "Invalid color '{}' for specified format '{}'",
                    color_string, cli_format
                );
                return ExitCode::FAILURE;
            };

            (color.into(), cli_format.into(), use_alpha)
        }
        (Some(color_string), None) => {
            let Some((color, format, use_alpha)) = parse_color_unknown_format(&color_string) else {
                eprintln!("Could not detect format for color '{}'", color_string);
                return ExitCode::FAILURE;
            };
            (color.into(), format, use_alpha)
        }
        (None, Some(cli_format)) => (random_color(), cli_format.into(), true),
        (None, None) => (random_color(), CliColorFormat::default().into(), true),
    };
    log_startup_time("Color parse");

    let native_options = eframe::NativeOptions {
        multisampling: 4,
        renderer: eframe::Renderer::Glow,
        viewport: ViewportBuilder::default().with_min_inner_size(Vec2::new(500., 400.)),
        ..Default::default()
    };

    let data = Arc::new((color, format, use_alpha));

    eframe::run_native(
        "Oklch Color Picker",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc, data)))),
    )
    .unwrap();

    ExitCode::SUCCESS
}

static INTERVAL_TIMER: Lazy<Mutex<Instant>> = Lazy::new(|| Mutex::new(Instant::now()));
static STARTUP_TIMER: Lazy<Instant> = Lazy::new(Instant::now);
static STARTUP_TIMER_ENABLED: Lazy<bool> = Lazy::new(|| env::var("STARTUP_TIMER").is_ok());
fn log_startup_init() {
    if *STARTUP_TIMER_ENABLED {
        _ = STARTUP_TIMER.elapsed();
        *INTERVAL_TIMER.lock().unwrap() = Instant::now();
    }
}
fn log_startup_time(name: &str) {
    if *STARTUP_TIMER_ENABLED {
        let mut timer = INTERVAL_TIMER.lock().unwrap();
        println!(
            "{:<20}: {:>10.5}ms delta, {:>10.5}ms total",
            name,
            timer.elapsed().as_secs_f64() * 1000.,
            STARTUP_TIMER.elapsed().as_secs_f64() * 1000.,
        );
        *timer = Instant::now();
    }
}

fn random_color() -> Oklcha {
    let mut rng = SmallRng::from_entropy();
    Oklcha::new(
        rng.gen_range(0.4..0.8),
        rng.gen_range(0.05..0.2),
        rng.gen_range(0.0..360.),
        1.,
    )
}

fn lerp(v0: f32, v1: f32, t: f32) -> f32 {
    (1. - t) * v0 + t * v1
}

fn map(input: f32, from: (f32, f32), to: (f32, f32)) -> f32 {
    ((to.1 - to.0) * (input - from.0) / (from.1 - from.0) + to.0)
        .clamp(to.0.min(to.1), to.1.max(to.0))
}

fn setup_egui_config(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "my_font".to_owned(),
        egui::FontData::from_static(include_bytes!("../src/Inter-Regular.ttf")),
    );

    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "my_font".to_owned());

    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("my_font".to_owned());

    ctx.set_fonts(fonts);

    ctx.style_mut(|style| {
        style
            .text_styles
            .get_mut(&egui::TextStyle::Body)
            .unwrap()
            .size = 16.;

        style
            .text_styles
            .get_mut(&egui::TextStyle::Button)
            .unwrap()
            .size = 14.;
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        style.spacing.icon_width *= 1.8;
        style.spacing.icon_width_inner *= 1.8;
        style.visuals.widgets.open.rounding = 4.0.into();
        style.visuals.widgets.active.rounding = 4.0.into();
        style.visuals.widgets.hovered.rounding = 4.0.into();
        style.visuals.widgets.inactive.rounding = 4.0.into();
        style.visuals.widgets.noninteractive.rounding = 4.0.into();
        style.visuals.widgets.inactive.bg_stroke =
            egui::Stroke::new(1.0, style.visuals.widgets.inactive.bg_fill);
    });
}

struct App {
    previous_color: Oklrcha,
    color: Oklrcha,
    format: ColorFormat,
    prev_css: CssColorFormat,
    prev_raw: RawColorFormat,
    use_alpha: bool,
    programs: HashMap<ProgramKind, Arc<Mutex<GlowProgram>>>,
    input_text: HashMap<u8, String>,
    first_frame: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, data: Arc<(Oklcha, ColorFormat, bool)>) -> Self {
        log_startup_time("App new");
        setup_egui_config(&cc.egui_ctx);
        log_startup_time("Egui custom setup");

        let gl = cc.gl.as_ref().unwrap();

        let programs = ProgramKind::iter()
            .map(|kind| (kind, Arc::new(Mutex::new(GlowProgram::new(gl, kind)))))
            .collect();

        log_startup_time("Gl programs created");

        let color = data.0.into();

        Self {
            previous_color: color,
            color,
            format: data.1,
            prev_css: Default::default(),
            prev_raw: Default::default(),
            use_alpha: data.2,
            programs,
            input_text: Default::default(),
            first_frame: true,
        }
    }
}

const CHROMA_MAX: f32 = 0.33;

const LINE_COLOR: Color32 = Color32::from_gray(30);
const LINE_COLOR2: Color32 = Color32::from_gray(220);

const MID_GRAY: egui::Rgba =
    egui::Rgba::from_rgba_premultiplied(0.18406294, 0.18406294, 0.18406294, 1.);

fn canvas_picker(ui: &mut egui::Ui) -> egui::Frame {
    egui::Frame::canvas(ui.style())
        .inner_margin(0.0)
        .outer_margin(egui::Margin {
            bottom: 9.,
            left: 9.,
            right: 9.,
            top: 0.,
        })
        .fill(MID_GRAY.into())
        .stroke(Stroke::new(14.0, MID_GRAY))
        .rounding(0.)
}

fn canvas_slider(ui: &mut egui::Ui) -> egui::Frame {
    let h = ui.available_height();
    egui::Frame::canvas(ui.style())
        .inner_margin(2.0)
        .outer_margin(egui::Margin {
            left: 10.,
            right: 14.,
            bottom: h / 8.,
            top: h / 8.,
        })
        .fill(MID_GRAY.into())
        .stroke(Stroke::new(4.0, MID_GRAY))
        .rounding(0.)
}

fn canvas_final(ui: &mut egui::Ui) -> egui::Frame {
    egui::Frame::canvas(ui.style())
        .inner_margin(4.0)
        .outer_margin(egui::Margin {
            left: 3.,
            right: 3.,
            bottom: 12.,
            top: 4.,
        })
        .fill(MID_GRAY.into())
        .stroke(Stroke::new(2.0, MID_GRAY))
        .rounding(0.)
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if self.first_frame {
            log_startup_time("First frame start");
            self.first_frame = false;
        }

        let frame = egui::Frame::central_panel(&ctx.style()).inner_margin(20.0);

        let central_panel = egui::CentralPanel::default().frame(frame);

        let (fallback_color, is_fallback) =
            gamut_clip_preserve_chroma(Oklcha::from(self.color).into());

        let fallback_u8 = Srgba::from(fallback_color).to_u8_array();
        let fallback_egui_color =
            egui::Color32::from_rgb(fallback_u8[0], fallback_u8[1], fallback_u8[2]);

        let (previous_fallback_color, is_previous_fallback) =
            gamut_clip_preserve_chroma(Oklcha::from(self.previous_color).into());

        let glow_paint = |ui: &mut egui::Ui, program: ProgramKind, color: Oklrcha, size: Vec2| {
            let p = Arc::clone(&self.programs[&program]);
            let rect = ui.min_rect();

            let cb = egui::PaintCallback {
                rect,
                callback: Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
                    p.lock().unwrap().paint(
                        painter.gl(),
                        color,
                        fallback_color,
                        previous_fallback_color,
                        size,
                    );
                })),
            };
            ui.painter().add(cb);
        };

        let draw_line = |ui: &mut egui::Ui,
                         vertical: bool,
                         wide: bool,
                         rect: egui::Rect,
                         pos: f32,
                         name: &str,
                         labels: &mut Vec<(egui::Rect, String)>| {
            let width = if wide { 2. } else { 1. };
            let color = LINE_COLOR;
            let border = 10.;
            if vertical {
                let pos = lerp(rect.left(), rect.right(), pos);
                let rect = rect.expand(border);
                let painter = ui.painter_at(rect);
                painter.add(egui::Shape::line_segment(
                    [
                        Pos2::new(pos, rect.top() - border),
                        Pos2::new(pos, rect.bottom() + border),
                    ],
                    Stroke::new(width, color),
                ));
                if !name.is_empty() {
                    let label_center = Pos2::new(pos, rect.bottom() + 5.);
                    let label_rect =
                        egui::Rect::from_center_size(label_center, egui::vec2(16.0, 10.0));
                    labels.push((label_rect, name.to_owned()));
                }
            } else {
                let pos = lerp(rect.bottom(), rect.top(), pos);
                let rect = rect.expand(border);
                let painter = ui.painter_at(rect);
                painter.add(egui::Shape::line_segment(
                    [Pos2::new(rect.left(), pos), Pos2::new(rect.right(), pos)],
                    Stroke::new(width, color),
                ));

                if !name.is_empty() {
                    let label_center = Pos2::new(rect.left() - 10., pos - 4.);
                    let label_rect =
                        egui::Rect::from_center_size(label_center, egui::vec2(10.0, 10.0));
                    labels.push((label_rect, name.to_owned()));
                }
            }
        };

        let mut labels = Vec::new();

        central_panel.show(ctx, |ui| {
            StripBuilder::new(ui)
                .size(Size::remainder())
                .size(Size::relative(0.01))
                .size(Size::relative(0.20).at_least(120.))
                .size(Size::relative(0.01))
                .size(Size::relative(0.18).at_least(120.))
                .vertical(|mut strip| {
                    strip.strip(|builder| {
                        builder.sizes(Size::remainder(), 2).horizontal(|mut strip| {
                            strip.cell(|ui| {
                                canvas_picker(ui).show(ui, |ui| {
                                    let (rect, response) = ui.allocate_exact_size(
                                        ui.available_size(),
                                        egui::Sense::drag(),
                                    );

                                    if let Some(pos) = response.interact_pointer_pos() {
                                        self.color.lightness_r =
                                            map(pos.x, (rect.left(), rect.right()), (0., 1.));
                                        self.color.chroma = map(
                                            pos.y,
                                            (rect.top(), rect.bottom()),
                                            (CHROMA_MAX, 0.),
                                        );
                                    }

                                    glow_paint(ui, ProgramKind::Picker, self.color, rect.size());

                                    let l = self.color.lightness_r;
                                    draw_line(ui, true, false, rect, l, "Lr", &mut labels);
                                    let c = self.color.chroma / CHROMA_MAX;
                                    draw_line(ui, false, false, rect, c, "C", &mut labels);
                                });
                            });

                            strip.cell(|ui| {
                                canvas_picker(ui).show(ui, |ui| {
                                    let (rect, response) = ui.allocate_exact_size(
                                        ui.available_size(),
                                        egui::Sense::drag(),
                                    );

                                    if let Some(pos) = response.interact_pointer_pos() {
                                        self.color.hue =
                                            map(pos.x, (rect.left(), rect.right()), (0., 360.));
                                        self.color.chroma = map(
                                            pos.y,
                                            (rect.top(), rect.bottom()),
                                            (CHROMA_MAX, 0.),
                                        );
                                    }

                                    glow_paint(ui, ProgramKind::Picker2, self.color, rect.size());

                                    let h = self.color.hue / 360.;
                                    draw_line(ui, true, false, rect, h, "H", &mut labels);
                                    let c = self.color.chroma / CHROMA_MAX;
                                    draw_line(ui, false, false, rect, c, "", &mut labels);
                                });
                            });
                        });
                    });

                    strip.cell(|_| {});

                    strip.strip(|builder| {
                        let draw_slider_line = |ui: &mut egui::Ui, rect: egui::Rect, pos: f32| {
                            let center = Pos2::new(
                                lerp(rect.left(), rect.right(), pos),
                                rect.top() + rect.height() / 2.,
                            );

                            let painter = ui.painter();

                            painter.rect(
                                egui::Rect::from_center_size(
                                    center,
                                    egui::vec2(
                                        (rect.width() / 85.).clamp(9., 22.),
                                        rect.height() + 10.,
                                    ),
                                ),
                                4.,
                                fallback_egui_color,
                                Stroke::new(3.0, LINE_COLOR2),
                            );
                        };
                        let input_size = Vec2::new(66., 26.);
                        let show_label = |ui: &mut egui::Ui, label: &str| {
                            let label = egui::Label::new(label);
                            ui.add_sized(Vec2::new(12., 26.), label);
                        };
                        builder.sizes(Size::remainder(), 4).vertical(|mut strip| {
                            let field_width = Vec2::new(100., 0.);
                            strip.cell(|ui| {
                                ui.horizontal_centered(|ui| {
                                    canvas_slider(ui).show(ui, |ui| {
                                        let (rect, response) = ui.allocate_exact_size(
                                            ui.available_size() - field_width,
                                            egui::Sense::drag(),
                                        );

                                        if let Some(pos) = response.interact_pointer_pos() {
                                            self.color.lightness_r =
                                                map(pos.x, (rect.left(), rect.right()), (0., 1.));
                                        }

                                        glow_paint(
                                            ui,
                                            ProgramKind::Lightness,
                                            self.color,
                                            rect.size(),
                                        );
                                        draw_slider_line(ui, rect, self.color.lightness_r);
                                    });

                                    let get_set = |v: Option<f64>| match v {
                                        Some(v) => {
                                            self.color.lightness_r = v as f32;
                                            v
                                        }
                                        None => self.color.lightness_r as f64,
                                    };
                                    ui.add_sized(
                                        input_size,
                                        DragValue::from_get_set(get_set)
                                            .speed(1. * 0.001)
                                            .range(0.0..=1.0)
                                            .max_decimals(4),
                                    );
                                    show_label(ui, "Lr");
                                });
                            });
                            strip.cell(|ui| {
                                ui.horizontal_centered(|ui| {
                                    canvas_slider(ui).show(ui, |ui| {
                                        let (rect, response) = ui.allocate_exact_size(
                                            ui.available_size() - field_width,
                                            egui::Sense::drag(),
                                        );

                                        if let Some(pos) = response.interact_pointer_pos() {
                                            self.color.chroma = map(
                                                pos.x,
                                                (rect.left(), rect.right()),
                                                (0., CHROMA_MAX),
                                            );
                                        }

                                        glow_paint(
                                            ui,
                                            ProgramKind::Chroma,
                                            self.color,
                                            rect.size(),
                                        );
                                        draw_slider_line(ui, rect, self.color.chroma / CHROMA_MAX);
                                    });
                                    let get_set = |v: Option<f64>| match v {
                                        Some(v) => {
                                            self.color.chroma = v as f32;
                                            v
                                        }
                                        None => self.color.chroma as f64,
                                    };
                                    ui.add_sized(
                                        input_size,
                                        DragValue::from_get_set(get_set)
                                            .speed(CHROMA_MAX * 0.001)
                                            .range(0.0..=CHROMA_MAX)
                                            .max_decimals(4),
                                    );
                                    show_label(ui, "C");
                                });
                            });

                            strip.cell(|ui| {
                                ui.horizontal_centered(|ui| {
                                    canvas_slider(ui).show(ui, |ui| {
                                        let (rect, response) = ui.allocate_exact_size(
                                            ui.available_size() - field_width,
                                            egui::Sense::drag(),
                                        );

                                        if let Some(pos) = response.interact_pointer_pos() {
                                            self.color.hue =
                                                map(pos.x, (rect.left(), rect.right()), (0., 360.));
                                        }

                                        glow_paint(ui, ProgramKind::Hue, self.color, rect.size());
                                        draw_slider_line(ui, rect, self.color.hue / 360.);
                                    });

                                    let get_set = |v: Option<f64>| match v {
                                        Some(v) => {
                                            self.color.hue = v as f32;
                                            v
                                        }
                                        None => self.color.hue as f64,
                                    };
                                    ui.add_sized(
                                        input_size,
                                        DragValue::from_get_set(get_set)
                                            .speed(360. * 0.001)
                                            .range(0.0..=360.0)
                                            .max_decimals(2),
                                    );
                                    show_label(ui, "H");
                                });
                            });

                            strip.cell(|ui| {
                                ui.horizontal_centered(|ui| {
                                    canvas_slider(ui).show(ui, |ui| {
                                        let (rect, response) = ui.allocate_exact_size(
                                            ui.available_size() - field_width,
                                            egui::Sense::drag(),
                                        );

                                        if let Some(pos) = response.interact_pointer_pos() {
                                            self.color.alpha =
                                                map(pos.x, (rect.left(), rect.right()), (0., 1.));
                                            self.use_alpha = true;
                                        }

                                        glow_paint(ui, ProgramKind::Alpha, self.color, rect.size());
                                        draw_slider_line(ui, rect, self.color.alpha);
                                    });
                                    let get_set = |v: Option<f64>| match v {
                                        Some(v) => {
                                            self.color.alpha = v as f32;
                                            self.use_alpha = true;
                                            v
                                        }
                                        None => self.color.alpha as f64,
                                    };
                                    ui.add_sized(
                                        input_size,
                                        DragValue::from_get_set(get_set)
                                            .speed(1. * 0.001)
                                            .range(0.0..=1.0)
                                            .max_decimals(2),
                                    );
                                    show_label(ui, "A");
                                });
                            });
                        });
                    });

                    strip.cell(|_| {});

                    strip.strip(|builder| {
                        builder
                            .size(Size::relative(2. / 3.))
                            .size(Size::exact(10.))
                            .size(Size::remainder())
                            .horizontal(|mut strip| {
                                let mut show_color_edit =
                                    |ui: &mut egui::Ui,
                                     color: &mut Oklrcha,
                                     fallback: LinearRgba,
                                     id: u8| {
                                        let mut text = if let Some(text) = self.input_text.get(&id)
                                        {
                                            if let Some((c, use_alpha)) =
                                                parse_color(text, self.format)
                                            {
                                                self.use_alpha = use_alpha;
                                                *color = Oklcha::from(c).into();
                                            } else {
                                                ui.style_mut().visuals.selection.stroke =
                                                    egui::Stroke::new(
                                                        2.0,
                                                        egui::Color32::from_hex("#ce3c47").unwrap(),
                                                    );
                                            }

                                            text.clone()
                                        } else {
                                            format_color(fallback, self.format, self.use_alpha)
                                        };

                                        let output = egui::TextEdit::singleline(&mut text)
                                            .margin(6.0)
                                            .min_size(Vec2::new(ui.available_width(), 0.))
                                            .show(ui);

                                        if output.response.has_focus() {
                                            self.input_text.insert(id, text.clone());
                                        } else {
                                            self.input_text.remove(&id);
                                        }
                                    };

                                strip.cell(|ui| {
                                    canvas_final(ui).show(ui, |ui| {
                                        let (rect, _) = ui.allocate_exact_size(
                                            Vec2::new(
                                                ui.available_width(),
                                                ui.available_height() / 1.8,
                                            ),
                                            egui::Sense::drag(),
                                        );
                                        glow_paint(ui, ProgramKind::Final, self.color, rect.size());
                                    });
                                    ui.add_space(2.);
                                    ui.columns(2, |ui| {
                                        show_color_edit(
                                            &mut ui[0],
                                            &mut self.previous_color,
                                            previous_fallback_color,
                                            0,
                                        );
                                        let color_label = |text: &str, fallback: bool| {
                                            egui::Label::new(format!(
                                                "{text}{}",
                                                if fallback { " (fallback)" } else { "" }
                                            ))
                                            .wrap_mode(egui::TextWrapMode::Truncate)
                                        };

                                        ui[0].horizontal(|ui| {
                                            color_label("Previous Color", is_previous_fallback)
                                                .ui(ui);
                                        });

                                        show_color_edit(
                                            &mut ui[1],
                                            &mut self.color,
                                            fallback_color,
                                            1,
                                        );
                                        ui[1].horizontal(|ui| {
                                            color_label("New Color", is_fallback).ui(ui);
                                        });
                                    });
                                });

                                strip.cell(|_| {});

                                strip.cell(|ui| {
                                    ui.add_space(4.0);
                                    ui.vertical_centered(|ui| {
                                        let style = ui.style_mut();
                                        style
                                            .text_styles
                                            .get_mut(&egui::TextStyle::Button)
                                            .unwrap()
                                            .size = 18.;

                                        style.spacing.button_padding = egui::vec2(4.0, 3.0);

                                        let App {
                                            color,
                                            format,
                                            prev_css,
                                            prev_raw,
                                            use_alpha,
                                            ..
                                        } = self;
                                        color_format_input(
                                            ui, format, use_alpha, prev_css, prev_raw, color,
                                        );

                                        ui.add_space(1.);

                                        ui.style_mut().spacing.button_padding =
                                            egui::vec2(16.0, 8.0);
                                        ui.horizontal_centered(|ui| {
                                            let button =
                                                egui::Button::new(RichText::new("DONE").size(26.0))
                                                    .min_size(Vec2::new(
                                                        ui.available_size().x,
                                                        ui.available_size().y * 0.9,
                                                    ))
                                                    .stroke(egui::Stroke::new(
                                                        1.0,
                                                        fallback_egui_color,
                                                    ));
                                            if ui.add(button).clicked() {
                                                println!(
                                                    "{}",
                                                    format_color(
                                                        fallback_color,
                                                        self.format,
                                                        self.use_alpha
                                                    )
                                                );
                                                ui.ctx()
                                                    .send_viewport_cmd(egui::ViewportCommand::Close)
                                            }
                                        });
                                    });
                                });
                            });
                    });
                });

            for (rect, label) in labels {
                ui.put(rect, egui::Label::new(RichText::from(label)));
            }
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

fn color_format_input(
    ui: &mut egui::Ui,
    value: &mut ColorFormat,
    use_alpha: &mut bool,
    prev_css: &mut CssColorFormat,
    prev_raw: &mut RawColorFormat,
    color: &mut Oklrcha,
) {
    let mut raw = matches!(*value, ColorFormat::Raw(_));
    let old_raw = raw;

    let text = |r| if r { "RAW" } else { "CSS" };

    ui.horizontal(|ui| {
        egui::ComboBox::from_id_source("raw_or_css")
            .width(90.)
            .selected_text(text(raw))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut raw, false, text(false));
                ui.selectable_value(&mut raw, true, text(true));
            });

        if raw
            && ui
                .add(egui::Checkbox::new(use_alpha, RichText::new("Alpha")))
                .clicked()
            && !*use_alpha
        {
            color.alpha = 1.;
        }
    });

    ui.add_space(4.0);

    if raw != old_raw {
        if raw {
            *value = ColorFormat::Raw(*prev_raw);
        } else {
            *value = ColorFormat::Css(*prev_css);
        }

        match *value {
            ColorFormat::Css(css) => *prev_css = css,
            ColorFormat::Raw(r) => *prev_raw = r,
        }
    }

    match value {
        ColorFormat::Css(css_format) => format_combo(ui, css_format),
        ColorFormat::Raw(raw_format) => format_combo(ui, raw_format),
    }

    ui.add_space(4.0);
}

fn format_combo<T: IntoEnumIterator + Display + Debug + PartialEq + Copy>(
    ui: &mut egui::Ui,
    value: &mut T,
) {
    egui::ComboBox::from_id_source("format")
        .width(150.)
        .selected_text(value.to_string().to_ascii_uppercase())
        .show_ui(ui, |ui| {
            for format in T::iter() {
                ui.selectable_value(value, format, format.to_string().to_ascii_uppercase());
            }
        });
}
