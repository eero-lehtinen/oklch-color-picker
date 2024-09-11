#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    fmt::Display,
    sync::{Arc, Mutex},
};

use bevy_color::{ColorToComponents, ColorToPacked, LinearRgba, Oklcha, Srgba};
use eframe::{
    egui::{self, ahash::HashMap, Color32, DragValue, Pos2, RichText, Stroke, Vec2},
    egui_glow,
    glow::{self},
};
use egui_extras::{Size, StripBuilder};
use gamut::gamut_clip_preserve_chroma;
use gl_programs::{GlowProgram, ProgramKind};
use once_cell::sync::Lazy;
use regex::Regex;
use strum::{EnumIter, IntoEnumIterator};

mod gamut;
mod gl_programs;

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

fn lerp(v0: f32, v1: f32, t: f32) -> f32 {
    (1. - t) * v0 + t * v1
}

fn map(input: f32, from: (f32, f32), to: (f32, f32)) -> f32 {
    ((to.1 - to.0) * (input - from.0) / (from.1 - from.0) + to.0)
        .clamp(to.0.min(to.1), to.1.max(to.0))
}

fn setup_custom_fonts(ctx: &egui::Context) {
    // Start with the default fonts (we will be adding to them rather than replacing them).
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
    });

    // Tell egui to use these fonts:
    ctx.set_fonts(fonts);
}

struct App {
    previous_color: Oklcha,
    color: Oklcha,
    format: ColorFormat,
    programs: HashMap<ProgramKind, Arc<Mutex<GlowProgram>>>,
    input_text: HashMap<u8, String>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_fonts(&cc.egui_ctx);

        let gl = cc.gl.as_ref().unwrap();

        Self {
            previous_color: Oklcha::new(0.8, 0.1, 0.0, 1.0),
            color: Oklcha::new(0.8, 0.1, 0.0, 1.0),
            format: ColorFormat::Oklch,
            programs: ProgramKind::iter()
                .map(|kind| (kind, Arc::new(Mutex::new(GlowProgram::new(gl, kind)))))
                .collect(),
            input_text: Default::default(),
        }
    }
}

const CHROMA_MAX: f32 = 0.33;

const LINE_COLOR: Color32 = Color32::from_gray(30);
const LINE_COLOR2: Color32 = Color32::from_gray(220);

fn canvas_picker(ui: &mut egui::Ui) -> egui::Frame {
    egui::Frame::canvas(ui.style())
        .inner_margin(0.0)
        .outer_margin(egui::Margin {
            bottom: 16.,
            ..Default::default()
        })
        .rounding(0.0)
        .stroke(Stroke::NONE)
}

fn canvas_slider(ui: &mut egui::Ui) -> egui::Frame {
    egui::Frame::canvas(ui.style())
        .inner_margin(0.0)
        .outer_margin(egui::Margin {
            left: 10.,
            right: 4.,
            bottom: 4.,
            top: 4.,
        })
        .rounding(0.0)
        .stroke(Stroke::NONE)
}

fn canvas_final(ui: &mut egui::Ui) -> egui::Frame {
    egui::Frame::canvas(ui.style())
        .inner_margin(0.0)
        .outer_margin(egui::Margin {
            left: 0.,
            right: 0.,
            bottom: 4.,
            top: 4.,
        })
        .rounding(0.0)
        .stroke(Stroke::NONE)
}

fn is_fallback(color: Oklcha) -> bool {
    LinearRgba::from(color)
        .to_f32_array()
        .iter()
        .any(|x| *x < 0. || *x > 1.)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter)]
enum ColorFormat {
    Oklch,
    Hex,
    Rgba,
}

impl Display for ColorFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorFormat::Oklch => write!(f, "Oklch"),
            ColorFormat::Hex => write!(f, "Hex"),
            ColorFormat::Rgba => write!(f, "Rgba"),
        }
    }
}

fn float_to_decimal(v: f32, decimals: i32) -> f32 {
    let factor = 10.0f32.powi(decimals);
    (v * factor).round() / factor
}

fn format_color(color: Oklcha, fallback: Srgba, format: ColorFormat) -> String {
    match format {
        ColorFormat::Oklch => {
            format!(
                "oklch({} {} {} / {})",
                float_to_decimal(color.lightness, 4),
                float_to_decimal(color.chroma, 4),
                float_to_decimal(color.hue, 2),
                float_to_decimal(color.alpha, 4)
            )
        }
        ColorFormat::Hex => fallback.to_hex(),
        ColorFormat::Rgba => {
            let c = fallback.to_u8_array_no_alpha();
            format!(
                "rgba({}, {}, {}, {})",
                c[0],
                c[1],
                c[2],
                float_to_decimal(color.alpha, 2)
            )
        }
    }
}

const N: &str = r#"(\d+(?:\.?\d*))"#;

static OKLCH_REGEX: Lazy<Regex> = Lazy::new(|| {
    let r = const_format::formatcp!(r#"^oklch\(\s*{N}(%?)\s+{N}\s+{N}\s*(?:\/\s*{N}(%?)\s*)?\)$"#);
    dbg!(&r);
    Regex::new(r).unwrap()
});

fn parse_color(s: &str, format: ColorFormat) -> Option<Oklcha> {
    match format {
        ColorFormat::Oklch => {
            let caps = OKLCH_REGEX.captures(s)?;
            let mut lightness = caps.get(1)?.as_str().parse::<f32>().ok()?;
            let percent_sign = caps.get(2)?.as_str();
            if !percent_sign.is_empty() {
                lightness /= 100.;
            }
            let chroma = caps.get(3)?.as_str().parse::<f32>().ok()?;
            let hue = caps.get(4)?.as_str().parse::<f32>().ok()?;
            let mut alpha = caps
                .get(5)
                .map_or(Some(1.), |c| c.as_str().parse::<f32>().ok())?;
            let percent_sign = caps.get(6).map_or("", |c| c.as_str());
            if !percent_sign.is_empty() {
                alpha /= 100.;
            }
            Some(Oklcha::new(lightness, chroma, hue, alpha))
        }
        ColorFormat::Hex => Some(Oklcha::default()),
        ColorFormat::Rgba => Some(Oklcha::default()),
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let frame = egui::Frame::central_panel(&ctx.style())
            .inner_margin(20.0)
            .stroke(Stroke::NONE);

        let central_panel = egui::CentralPanel::default().frame(frame);

        let fallback_color = Srgba::from(gamut_clip_preserve_chroma(self.color.into()));

        let fallback_u8 = fallback_color.to_u8_array_no_alpha();
        let fallback_egui_color =
            egui::Color32::from_rgb(fallback_u8[0], fallback_u8[1], fallback_u8[2]);

        let previous_fallback_color =
            Srgba::from(gamut_clip_preserve_chroma(self.previous_color.into()));

        let glow_paint = |ui: &mut egui::Ui, program: ProgramKind, color: Oklcha, width: f32| {
            let p = Arc::clone(&self.programs[&program]);
            let cb = egui::PaintCallback {
                rect: ui.min_rect(),
                callback: Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
                    p.lock().unwrap().paint(
                        painter.gl(),
                        color,
                        fallback_color.to_f32_array(),
                        previous_fallback_color.to_f32_array(),
                        width,
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
            if vertical {
                let painter = ui.painter_at(rect);
                let pos = lerp(rect.left(), rect.right(), pos);
                painter.add(egui::Shape::line_segment(
                    [Pos2::new(pos, rect.top()), Pos2::new(pos, rect.bottom())],
                    Stroke::new(width, color),
                ));
                if !name.is_empty() {
                    let label_center = Pos2::new(pos, rect.bottom() + 7.);
                    let label_rect =
                        egui::Rect::from_center_size(label_center, egui::vec2(10.0, 10.0));
                    labels.push((label_rect, name.to_owned()));
                }
            } else {
                let painter = ui.painter_at(rect);
                let pos = lerp(rect.bottom(), rect.top(), pos);
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
                .size(Size::relative(0.20).at_least(120.))
                .size(Size::relative(0.18).at_least(100.))
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
                                        self.color.lightness =
                                            map(pos.x, (rect.left(), rect.right()), (0., 1.));
                                        self.color.chroma = map(
                                            pos.y,
                                            (rect.top(), rect.bottom()),
                                            (CHROMA_MAX, 0.),
                                        );
                                    }

                                    glow_paint(
                                        ui,
                                        ProgramKind::Picker,
                                        self.color,
                                        rect.aspect_ratio(),
                                    );

                                    let l = self.color.lightness;
                                    draw_line(ui, true, false, rect, l, "L", &mut labels);
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

                                    glow_paint(
                                        ui,
                                        ProgramKind::Picker2,
                                        self.color,
                                        rect.aspect_ratio(),
                                    );

                                    let h = self.color.hue / 360.;
                                    draw_line(ui, true, false, rect, h, "H", &mut labels);
                                    let c = self.color.chroma / CHROMA_MAX;
                                    draw_line(ui, false, false, rect, c, "", &mut labels);
                                });
                            });
                        });
                    });

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
                                    egui::vec2(8., rect.height() + 4.),
                                ),
                                0.,
                                fallback_egui_color,
                                Stroke::new(2.0, LINE_COLOR2),
                            );
                        };
                        let input_size = Vec2::new(66., 26.);
                        let show_label = |ui: &mut egui::Ui, label: &str| {
                            let label = egui::Label::new(label);
                            ui.add_sized(Vec2::new(10., 26.), label);
                        };
                        builder.sizes(Size::remainder(), 4).vertical(|mut strip| {
                            strip.cell(|ui| {
                                ui.horizontal_centered(|ui| {
                                    show_label(ui, "L");
                                    let get_set = |v: Option<f64>| match v {
                                        Some(v) => {
                                            self.color.lightness = v as f32;
                                            v
                                        }
                                        None => self.color.lightness as f64,
                                    };
                                    ui.add_sized(
                                        input_size,
                                        DragValue::from_get_set(get_set)
                                            .speed(1. * 0.001)
                                            .range(0.0..=1.0)
                                            .max_decimals(4),
                                    );

                                    canvas_slider(ui).show(ui, |ui| {
                                        let (rect, response) = ui.allocate_exact_size(
                                            ui.available_size(),
                                            egui::Sense::drag(),
                                        );

                                        if let Some(pos) = response.interact_pointer_pos() {
                                            self.color.lightness =
                                                map(pos.x, (rect.left(), rect.right()), (0., 1.));
                                        }

                                        glow_paint(
                                            ui,
                                            ProgramKind::Lightness,
                                            self.color,
                                            rect.aspect_ratio(),
                                        );
                                        draw_slider_line(ui, rect, self.color.lightness);
                                    });
                                });
                            });
                            strip.cell(|ui| {
                                ui.horizontal_centered(|ui| {
                                    show_label(ui, "C");
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
                                    canvas_slider(ui).show(ui, |ui| {
                                        let (rect, response) = ui.allocate_exact_size(
                                            ui.available_size(),
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
                                            rect.aspect_ratio(),
                                        );
                                        draw_slider_line(ui, rect, self.color.chroma / CHROMA_MAX);
                                    });
                                });
                            });

                            strip.cell(|ui| {
                                ui.horizontal_centered(|ui| {
                                    show_label(ui, "H");
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

                                    canvas_slider(ui).show(ui, |ui| {
                                        let (rect, response) = ui.allocate_exact_size(
                                            ui.available_size(),
                                            egui::Sense::drag(),
                                        );

                                        if let Some(pos) = response.interact_pointer_pos() {
                                            self.color.hue =
                                                map(pos.x, (rect.left(), rect.right()), (0., 360.));
                                        }

                                        glow_paint(
                                            ui,
                                            ProgramKind::Hue,
                                            self.color,
                                            rect.aspect_ratio(),
                                        );
                                        draw_slider_line(ui, rect, self.color.hue / 360.);
                                    });
                                });
                            });

                            strip.cell(|ui| {
                                ui.horizontal_centered(|ui| {
                                    show_label(ui, "A");
                                    let get_set = |v: Option<f64>| match v {
                                        Some(v) => {
                                            self.color.alpha = v as f32;
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
                                    canvas_slider(ui).show(ui, |ui| {
                                        let (rect, response) = ui.allocate_exact_size(
                                            ui.available_size(),
                                            egui::Sense::drag(),
                                        );

                                        if let Some(pos) = response.interact_pointer_pos() {
                                            self.color.alpha =
                                                map(pos.x, (rect.left(), rect.right()), (0., 1.));
                                        }

                                        glow_paint(
                                            ui,
                                            ProgramKind::Alpha,
                                            self.color,
                                            rect.aspect_ratio(),
                                        );
                                        draw_slider_line(ui, rect, self.color.alpha);
                                    });
                                });
                            });
                        });
                    });

                    strip.strip(|builder| {
                        builder.sizes(Size::remainder(), 3).horizontal(|mut strip| {
                            let rect_allocate = |ui: &mut egui::Ui| {
                                let (rect, _) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), ui.available_height() / 1.8),
                                    egui::Sense::drag(),
                                );
                                rect
                            };

                            let mut show_color_edit =
                                |ui: &mut egui::Ui, color: &mut Oklcha, fallback: Srgba, id: u8| {
                                    let mut text = if let Some(text) = self.input_text.get(&id) {
                                        if let Some(c) = parse_color(text, self.format) {
                                            *color = c;
                                        } else {
                                            ui.style_mut().visuals.selection.stroke =
                                                egui::Stroke::new(
                                                    2.0,
                                                    egui::Color32::from_hex("#ce3c47").unwrap(),
                                                );
                                        }

                                        text.clone()
                                    } else {
                                        format_color(*color, fallback, self.format)
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
                                ui.vertical(|ui| {
                                    canvas_final(ui).show(ui, |ui| {
                                        let rect = rect_allocate(ui);
                                        glow_paint(
                                            ui,
                                            ProgramKind::FinalPrevious,
                                            self.color,
                                            rect.aspect_ratio(),
                                        );
                                    });

                                    show_color_edit(
                                        ui,
                                        &mut self.previous_color,
                                        previous_fallback_color,
                                        0,
                                    );
                                    ui.label(format!(
                                        "Previous Color{}",
                                        if is_fallback(self.previous_color) {
                                            " (fallback)"
                                        } else {
                                            ""
                                        }
                                    ));
                                });
                            });

                            strip.cell(|ui| {
                                ui.vertical(|ui| {
                                    canvas_final(ui).show(ui, |ui| {
                                        let rect = rect_allocate(ui);
                                        glow_paint(
                                            ui,
                                            ProgramKind::Final,
                                            self.color,
                                            rect.aspect_ratio(),
                                        );
                                    });

                                    show_color_edit(ui, &mut self.color, fallback_color, 1);
                                    ui.label(format!(
                                        "New Color{}",
                                        if is_fallback(self.color) {
                                            " (fallback)"
                                        } else {
                                            ""
                                        }
                                    ));
                                });
                            });

                            strip.cell(|ui| {
                                ui.vertical_centered_justified(|ui| {
                                    ui.add_space(4.0);
                                    ui.style_mut()
                                        .text_styles
                                        .get_mut(&egui::TextStyle::Button)
                                        .unwrap()
                                        .size = 20.;
                                    ui.horizontal(|ui| {
                                        egui::ComboBox::from_id_source("format")
                                            .selected_text(format!("{:?}", &mut self.format))
                                            .show_ui(ui, |ui| {
                                                for format in ColorFormat::iter() {
                                                    ui.selectable_value(
                                                        &mut self.format,
                                                        format,
                                                        format.to_string(),
                                                    );
                                                }
                                            });

                                        let label = egui::Label::new("Output Format")
                                            .wrap_mode(egui::TextWrapMode::Truncate);
                                        ui.add(label);
                                    });

                                    let button = egui::Button::new(
                                        RichText::new("DONE").strong().size(30.0),
                                    )
                                    .min_size(ui.available_size());
                                    ui.add(button);
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

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn num_test1() {
        let r = Regex::new(N).unwrap();
        assert_eq!(r.captures("0.1").unwrap().get(0).unwrap().as_str(), "0.1");
    }

    #[test]
    fn num_test2() {
        let r = Regex::new(N).unwrap();
        assert_eq!(r.captures("0.").unwrap().get(0).unwrap().as_str(), "0.");
    }

    #[test]
    fn test1() {
        assert_eq!(
            parse_color("oklch(0. 0.1 0.2/0.3)", ColorFormat::Oklch),
            Some(Oklcha::new(0., 0.1, 0.2, 0.3))
        );
    }

    #[test]
    fn test2() {
        assert_eq!(
            parse_color("oklch( 50.% 0. 0.2 / 2% )", ColorFormat::Oklch),
            Some(Oklcha::new(0.5, 0., 0.2, 0.02))
        );
    }

    #[test]
    fn test3() {
        assert_eq!(
            parse_color("oklch(1 1 1)", ColorFormat::Oklch),
            Some(Oklcha::new(1., 1., 1., 1.))
        );
    }

    #[test]
    fn test4() {
        assert_eq!(
            parse_color("oklch(50% 0.1 0.2 /)", ColorFormat::Oklch),
            None
        );
    }
}
