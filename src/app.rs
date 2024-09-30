use std::{
    fmt::{Debug, Display},
    sync::{Arc, Mutex},
};

use crate::formats::{format_color, parse_color, ColorFormat, CssColorFormat, RawColorFormat};
use crate::gamut::{gamut_clip_preserve_chroma, Oklrcha};
use crate::gl_programs::{GlowProgram, ProgramKind};
use crate::{lerp, log_startup_time, map};
use bevy_color::{ColorToPacked, LinearRgba, Oklcha, Srgba};
use eframe::{
    egui::{self, ahash::HashMap, Color32, DragValue, Pos2, RichText, Stroke, Vec2},
    egui_glow,
    glow::{self},
};
use egui::{Rect, Widget};
use egui_extras::{Size, Strip, StripBuilder};
use strum::IntoEnumIterator;

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
        .stroke(Stroke::new(7.0, MID_GRAY))
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
        .stroke(Stroke::new(2.0, MID_GRAY))
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
        .stroke(Stroke::new(1.0, MID_GRAY))
        .rounding(0.)
}

pub struct App {
    prev_color: Oklrcha,
    color: Oklrcha,
    format: ColorFormat,
    use_alpha: bool,
    prev_css: CssColorFormat,
    prev_raw: RawColorFormat,
    programs: HashMap<ProgramKind, Arc<Mutex<GlowProgram>>>,
    input_text: HashMap<u8, String>,
    first_frame: bool,
    frame_end_labels: Vec<(Rect, RichText)>,
    fallbacks: Fallbacks,
}

#[derive(Default)]
struct Fallbacks {
    prev: LinearRgba,
    is_prev_fallback: bool,
    cur: LinearRgba,
    is_cur_fallback: bool,
    cur_egui: egui::Color32,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, data: Arc<(Oklcha, ColorFormat, bool)>) -> Self {
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
            prev_color: color,
            color,
            format: data.1,
            prev_css: Default::default(),
            prev_raw: Default::default(),
            use_alpha: data.2,
            programs,
            input_text: Default::default(),
            first_frame: true,
            frame_end_labels: Default::default(),
            fallbacks: Default::default(),
        }
    }

    fn calculate_fallbacks(&mut self) {
        let color_rgba: LinearRgba = Oklcha::from(self.color).into();
        let prev_color_rgba: LinearRgba = Oklcha::from(self.prev_color).into();

        let color_fallback = gamut_clip_preserve_chroma(color_rgba);

        let fallback_u8 = Srgba::from(color_fallback).to_u8_array();
        let fallback_egui_color =
            egui::Color32::from_rgb(fallback_u8[0], fallback_u8[1], fallback_u8[2]);

        let prev_color_fallback = gamut_clip_preserve_chroma(prev_color_rgba);

        self.fallbacks = Fallbacks {
            cur: color_fallback,
            is_cur_fallback: color_fallback != color_rgba,
            prev: prev_color_fallback,
            is_prev_fallback: prev_color_fallback != prev_color_rgba,
            cur_egui: fallback_egui_color,
        };
    }

    fn glow_paint(&self, ui: &mut egui::Ui, program: ProgramKind, size: Vec2) {
        let p = Arc::clone(&self.programs[&program]);
        let rect = ui.min_rect();

        let color = self.color;
        let fallback = self.fallbacks.cur;
        let prev_fallback = self.fallbacks.prev;

        let cb = egui::PaintCallback {
            rect,
            callback: Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
                p.lock()
                    .unwrap()
                    .paint(painter.gl(), color, fallback, prev_fallback, size);
            })),
        };
        ui.painter().add(cb);
    }

    fn update_pickers(&mut self, mut strip: Strip) {
        let paint_picker_line =
            |ui: &mut egui::Ui,
             vertical: bool,
             rect: Rect,
             pos: f32,
             name: &str,
             labels: &mut Vec<(Rect, RichText)>| {
                let width = 1.;
                let color = LINE_COLOR;
                let border = 5.5;
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
                        labels.push((label_rect, name.into()));
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
                        labels.push((label_rect, name.into()));
                    }
                }
            };

        strip.cell(|ui| {
            canvas_picker(ui).show(ui, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());

                if let Some(pos) = response.interact_pointer_pos() {
                    self.color.lightness_r = map(pos.x, (rect.left(), rect.right()), (0., 1.));
                    self.color.chroma = map(pos.y, (rect.top(), rect.bottom()), (CHROMA_MAX, 0.));
                }

                self.glow_paint(ui, ProgramKind::Picker, rect.size());

                let l = self.color.lightness_r;
                paint_picker_line(ui, true, rect, l, "Lr", &mut self.frame_end_labels);
                let c = self.color.chroma / CHROMA_MAX;
                paint_picker_line(ui, false, rect, c, "C", &mut self.frame_end_labels);
            });
        });

        strip.cell(|ui| {
            canvas_picker(ui).show(ui, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());

                if let Some(pos) = response.interact_pointer_pos() {
                    self.color.hue = map(pos.x, (rect.left(), rect.right()), (0., 360.));
                    self.color.chroma = map(pos.y, (rect.top(), rect.bottom()), (CHROMA_MAX, 0.));
                }

                self.glow_paint(ui, ProgramKind::Picker2, rect.size());

                let h = self.color.hue / 360.;
                paint_picker_line(ui, true, rect, h, "H", &mut self.frame_end_labels);
                let c = self.color.chroma / CHROMA_MAX;
                paint_picker_line(ui, false, rect, c, "", &mut self.frame_end_labels);
            });
        });
    }

    fn update_sliders(&mut self, builder: StripBuilder) {
        let slider_thumb_color = self.fallbacks.cur_egui;
        let paint_slider_thumb = |ui: &mut egui::Ui, rect: egui::Rect, pos: f32| {
            let center = Pos2::new(
                lerp(rect.left(), rect.right(), pos),
                rect.top() + rect.height() / 2.,
            );

            let painter = ui.painter();

            painter.rect(
                egui::Rect::from_center_size(
                    center,
                    egui::vec2((rect.width() / 85.).clamp(9., 22.), rect.height() + 10.),
                ),
                4.,
                slider_thumb_color,
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

                        self.glow_paint(ui, ProgramKind::Lightness, rect.size());
                        paint_slider_thumb(ui, rect, self.color.lightness_r);
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
                            self.color.chroma =
                                map(pos.x, (rect.left(), rect.right()), (0., CHROMA_MAX));
                        }

                        self.glow_paint(ui, ProgramKind::Chroma, rect.size());
                        paint_slider_thumb(ui, rect, self.color.chroma / CHROMA_MAX);
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
                            self.color.hue = map(pos.x, (rect.left(), rect.right()), (0., 360.));
                        }

                        self.glow_paint(ui, ProgramKind::Hue, rect.size());
                        paint_slider_thumb(ui, rect, self.color.hue / 360.);
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
                            self.color.alpha = map(pos.x, (rect.left(), rect.right()), (0., 1.));
                            self.use_alpha = true;
                        }

                        self.glow_paint(ui, ProgramKind::Alpha, rect.size());
                        paint_slider_thumb(ui, rect, self.color.alpha);
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
    }

    fn update_color_edit(&mut self, ui: &mut egui::Ui, prev: bool, fallback: LinearRgba, id: u8) {
        let mut text = if let Some(text) = self.input_text.get(&id) {
            if let Some((c, use_alpha)) = parse_color(text, self.format) {
                self.use_alpha = use_alpha;
                let color = if prev {
                    &mut self.prev_color
                } else {
                    &mut self.color
                };
                *color = Oklcha::from(c).into();
            } else {
                ui.style_mut().visuals.selection.stroke =
                    egui::Stroke::new(2.0, egui::Color32::from_hex("#ce3c47").unwrap());
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
    }

    fn update_color_previews(&mut self, ui: &mut egui::Ui) {
        canvas_final(ui).show(ui, |ui| {
            let (rect, _) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), ui.available_height() / 1.8),
                egui::Sense::drag(),
            );
            self.glow_paint(ui, ProgramKind::Final, rect.size());
        });

        ui.add_space(2.);
        ui.columns(2, |ui| {
            self.update_color_edit(&mut ui[0], true, self.fallbacks.prev, 0);
            let color_label = |text: &str, fallback: bool| {
                egui::Label::new(format!(
                    "{text}{}",
                    if fallback { " (fallback)" } else { "" }
                ))
                .wrap_mode(egui::TextWrapMode::Truncate)
            };

            ui[0].horizontal(|ui| {
                color_label("Previous Color", self.fallbacks.is_prev_fallback).ui(ui);
            });

            self.update_color_edit(&mut ui[1], false, self.fallbacks.cur, 1);
            ui[1].horizontal(|ui| {
                color_label("New Color", self.fallbacks.is_cur_fallback).ui(ui);
            });
        });
    }

    fn update_button_area(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.vertical_centered(|ui| {
            let style = ui.style_mut();
            style
                .text_styles
                .get_mut(&egui::TextStyle::Button)
                .unwrap()
                .size = 18.;
            style.spacing.button_padding = egui::vec2(4.0, 3.0);

            let mut raw = matches!(self.format, ColorFormat::Raw(_));
            let old_raw = raw;
            let text = |r| if r { "RAW" } else { "CSS" };
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("raw_or_css")
                    .width(90.)
                    .selected_text(text(raw))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut raw, false, text(false));
                        ui.selectable_value(&mut raw, true, text(true));
                    });

                if raw
                    && ui
                        .add(egui::Checkbox::new(
                            &mut self.use_alpha,
                            RichText::new("Alpha"),
                        ))
                        .clicked()
                    && !self.use_alpha
                {
                    self.color.alpha = 1.;
                }
            });
            ui.add_space(4.0);
            if raw != old_raw {
                if raw {
                    self.format = ColorFormat::Raw(self.prev_raw);
                } else {
                    self.format = ColorFormat::Css(self.prev_css);
                }
                match self.format {
                    ColorFormat::Css(css) => self.prev_css = css,
                    ColorFormat::Raw(r) => self.prev_raw = r,
                }
            }
            fn format_combo<T: IntoEnumIterator + Display + Debug + PartialEq + Copy>(
                ui: &mut egui::Ui,
                value: &mut T,
            ) {
                egui::ComboBox::from_id_salt("format")
                    .width(150.)
                    .selected_text(value.to_string().to_ascii_uppercase())
                    .show_ui(ui, |ui| {
                        for format in T::iter() {
                            ui.selectable_value(
                                value,
                                format,
                                format.to_string().to_ascii_uppercase(),
                            );
                        }
                    });
            }
            match &mut self.format {
                ColorFormat::Css(css_format) => format_combo(ui, css_format),
                ColorFormat::Raw(raw_format) => format_combo(ui, raw_format),
            }

            ui.add_space(5.);

            ui.style_mut().spacing.button_padding = egui::vec2(16.0, 8.0);
            ui.horizontal_centered(|ui| {
                let button = egui::Button::new(RichText::new("DONE").size(26.0))
                    .min_size(Vec2::new(
                        ui.available_size().x,
                        ui.available_size().y * 0.9,
                    ))
                    .stroke(egui::Stroke::new(1.0, self.fallbacks.cur_egui));
                if ui.add(button).clicked() {
                    println!(
                        "{}",
                        format_color(self.fallbacks.cur, self.format, self.use_alpha)
                    );
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close)
                }
            });
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if self.first_frame {
            log_startup_time("First frame start");
            self.first_frame = false;
        }

        let central_panel = egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(20.0));

        self.calculate_fallbacks();

        central_panel.show(ctx, |ui| {
            StripBuilder::new(ui)
                .size(Size::remainder())
                .size(Size::relative(0.01))
                .size(Size::relative(0.20).at_least(120.))
                .size(Size::relative(0.01))
                .size(Size::relative(0.18).at_least(120.))
                .vertical(|mut strip| {
                    strip.strip(|builder| {
                        builder.sizes(Size::remainder(), 2).horizontal(|strip| {
                            self.update_pickers(strip);
                        });
                    });
                    strip.cell(|_| {});
                    strip.strip(|builder| self.update_sliders(builder));
                    strip.cell(|_| {});
                    strip.strip(|builder| {
                        builder
                            .size(Size::relative(2. / 3.))
                            .size(Size::exact(10.))
                            .size(Size::remainder())
                            .horizontal(|mut strip| {
                                strip.cell(|ui| {
                                    self.update_color_previews(ui);
                                });
                                strip.cell(|_| {});
                                strip.cell(|ui| {
                                    self.update_button_area(ui);
                                });
                            });
                    });
                });

            for (rect, label) in self.frame_end_labels.drain(..) {
                ui.put(rect, egui::Label::new(label));
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
