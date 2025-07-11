use std::sync::{Arc, Mutex};

use crate::gamut::{Oklrcha, gamut_clip_preserve_chroma};
use crate::gl_programs::{GlowProgram, ProgramKind};
use crate::{
    formats::{ColorFormat, format_color, parse_color},
    log_startup,
};
use crate::{lerp, map};
use bevy_color::{ColorToPacked, LinearRgba, Oklcha, Srgba};
use eframe::{
    egui::{self, Color32, DragValue, Pos2, RichText, Stroke, Vec2, ahash::HashMap},
    egui_glow,
    glow::{self},
};
use egui::ahash::HashSet;
use egui::{
    Align2, EventFilter, Id, Key, Margin, PopupAnchor, Rect, Response, Sense, Ui, UiBuilder, Widget,
};
use egui_extras::{Column, Size, StripBuilder, TableBuilder};
use strum::IntoEnumIterator;
use web_time::{Duration, Instant};

fn setup_egui_config(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "my_font".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../src/IBMPlexMono-Regular.ttf"
        ))),
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

    // For some reason persistence breaks switching themes
    ctx.set_theme(egui::Theme::Dark);

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

        let corner_radius = 4.0.into();
        style.visuals.widgets.open.corner_radius = corner_radius;
        style.visuals.widgets.active.corner_radius = corner_radius;
        style.visuals.widgets.hovered.corner_radius = corner_radius;
        style.visuals.widgets.inactive.corner_radius = corner_radius;
        style.visuals.widgets.noninteractive.corner_radius = corner_radius;

        let stroke_width = 1.0;
        style.visuals.widgets.inactive.bg_stroke =
            egui::Stroke::new(stroke_width, style.visuals.widgets.inactive.bg_fill);
        style.visuals.widgets.hovered.bg_stroke.width = stroke_width;
        style.visuals.widgets.active.bg_stroke.width = stroke_width;
        style.visuals.widgets.open.bg_stroke.width = stroke_width;
    });
}

const CHROMA_MAX: f32 = 0.33;

const LINE_COLOR_DARK: Color32 = Color32::from_gray(30);
const LINE_COLOR_LIGHT: Color32 = Color32::from_gray(210);
const LINE_COLOR_LIGHT_FOCUSED: Color32 = Color32::from_gray(210);
const LINE_COLOR_LIGHT_ACTIVE: Color32 = Color32::from_gray(250);

const MID_GRAY: egui::Rgba =
    egui::Rgba::from_rgba_premultiplied(0.18406294, 0.18406294, 0.18406294, 1.);

fn round_precision(value: f32, precision: f32) -> f32 {
    (value / precision).round() * precision
}

fn value_update(value: &mut f32, update_amount: f32, precision: f32, min: f32, max: f32) {
    if update_amount == 0. {
        return;
    }
    *value = round_precision(*value + update_amount * precision, precision).clamp(min, max);
}

#[derive(Clone, Copy, Debug)]
enum CanvasInputKind {
    Picker,
    Slider,
}

#[derive(Clone, Copy, Debug, Default)]
struct CanvasInputKeyOutput {
    vertical: f32,
    horizontal: f32,
}

fn canvas_input(
    kind: CanvasInputKind,
    ui: &mut Ui,
    add_contents: impl FnOnce(Response, Option<CanvasInputKeyOutput>, Rect, &mut Ui),
) -> Id {
    ui.scope_builder(UiBuilder::new().sense(Sense::drag()), |ui| {
        let h = ui.available_height();
        let response = ui.response();
        ui.style_mut().visuals.widgets.inactive.bg_stroke.color = MID_GRAY.into();
        let visuals = ui.style().interact(&response);

        let mut key_output = None;

        if response.has_focus() {
            ui.ctx().memory_mut(|m| {
                m.set_focus_lock_filter(
                    response.id,
                    EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: matches!(kind, CanvasInputKind::Picker),
                        ..Default::default()
                    },
                );
            });

            ui.input(|input| {
                if input.modifiers.command {
                    return;
                }
                let mut o = CanvasInputKeyOutput {
                    vertical: input.num_presses(Key::ArrowUp) as f32
                        - input.num_presses(Key::ArrowDown) as f32,
                    horizontal: input.num_presses(Key::ArrowRight) as f32
                        - input.num_presses(Key::ArrowLeft) as f32,
                };

                if input.modifiers.shift {
                    o.horizontal *= 10.;
                    o.vertical *= 10.;
                }

                if o.horizontal != 0. || o.vertical != 0. {
                    key_output = Some(o);
                }
            });
        }

        let (inner_margin, outer_margin) = match kind {
            CanvasInputKind::Picker => (
                7.0,
                egui::Margin {
                    bottom: 9,
                    left: 0,
                    right: 0,
                    top: 0,
                },
            ),
            CanvasInputKind::Slider => (
                4.0,
                egui::Margin {
                    left: 0,
                    right: 10,
                    bottom: (h / 8.) as i8,
                    top: (h / 8.) as i8,
                },
            ),
        };

        egui::Frame::canvas(ui.style())
            .stroke(visuals.bg_stroke)
            .inner_margin(inner_margin)
            .outer_margin(outer_margin)
            .fill(MID_GRAY.into())
            .show(ui, |ui| {
                match kind {
                    CanvasInputKind::Picker => ui.set_width(ui.available_width()),
                    CanvasInputKind::Slider => ui.set_width(ui.available_width() - 110.),
                }
                ui.set_height(ui.available_height());
                let rect = ui.available_rect_before_wrap();
                add_contents(response, key_output, rect, ui);
            })
    })
    .response
    .id
}

fn canvas_final(ui: &mut egui::Ui) -> egui::Frame {
    egui::Frame::canvas(ui.style())
        .inner_margin(5.0)
        .outer_margin(egui::Margin {
            left: 0,
            right: 0,
            bottom: 10,
            top: 4,
        })
        .fill(MID_GRAY.into())
}

pub struct App {
    prev_color: Oklrcha,
    color: Oklrcha,
    format: ColorFormat,
    use_alpha: bool,
    programs: HashMap<ProgramKind, Arc<Mutex<GlowProgram>>>,
    input_text: HashMap<u8, String>,
    first_frame: bool,
    frame_end_labels: Vec<(Rect, RichText)>,
    fallbacks: Fallbacks,
    copied_notice: Option<Instant>,
    first_input: Id,
    text_inputs: HashSet<Id>,
    show_settings: bool,
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
        log_startup::log("App new");
        setup_egui_config(&cc.egui_ctx);
        log_startup::log("Egui custom setup");

        let gl = cc.gl.as_ref().unwrap();

        let programs = ProgramKind::iter()
            .map(|kind| {
                (
                    kind,
                    Arc::new(Mutex::new(GlowProgram::new(gl, &cc.egui_ctx, kind))),
                )
            })
            .collect();

        log_startup::log("Gl programs created");

        let color = data.0.into();

        Self {
            prev_color: color,
            color,
            format: data.1,
            use_alpha: data.2,
            programs,
            input_text: Default::default(),
            first_frame: true,
            frame_end_labels: Default::default(),
            fallbacks: Default::default(),
            copied_notice: None,
            first_input: Id::NULL,
            text_inputs: HashSet::default(),
            show_settings: false,
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

    fn update_pickers(&mut self, builder: StripBuilder) {
        let paint_picker_line =
            |ui: &mut egui::Ui,
             vertical: bool,
             rect: Rect,
             pos: f32,
             name: &str,
             labels: &mut Vec<(Rect, RichText)>| {
                let width = 1.;
                let color = LINE_COLOR_DARK;
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
                            egui::Rect::from_center_size(label_center, egui::vec2(20.0, 10.0));
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
                        let label_center = Pos2::new(rect.left() - 10., pos - 6.);
                        let label_rect =
                            egui::Rect::from_center_size(label_center, egui::vec2(10.0, 10.0));
                        labels.push((label_rect, name.into()));
                    }
                }
            };

        builder
            .size(Size::remainder())
            .size(Size::exact(4.))
            .size(Size::remainder())
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    let id = canvas_input(
                        CanvasInputKind::Picker,
                        ui,
                        |response, key_output, rect, ui| {
                            self.focus_hotkey(ui, &response, Key::Num1);
                            if let Some(pos) = response.interact_pointer_pos() {
                                self.color.lightness_r =
                                    map(pos.x, (rect.left(), rect.right()), (0., 1.));
                                self.color.chroma =
                                    map(pos.y, (rect.top(), rect.bottom()), (CHROMA_MAX, 0.));
                            }
                            if let Some(o) = key_output {
                                value_update(
                                    &mut self.color.lightness_r,
                                    o.horizontal,
                                    0.01,
                                    0.,
                                    1.,
                                );
                                value_update(
                                    &mut self.color.chroma,
                                    o.vertical,
                                    0.005,
                                    0.,
                                    CHROMA_MAX,
                                );
                            }

                            self.glow_paint(ui, ProgramKind::Picker, rect.size());

                            let l = self.color.lightness_r;
                            paint_picker_line(ui, true, rect, l, "Lr", &mut self.frame_end_labels);
                            let c = self.color.chroma / CHROMA_MAX;
                            paint_picker_line(ui, false, rect, c, "C", &mut self.frame_end_labels);
                        },
                    );
                    self.first_input = id;
                });
                strip.empty();
                strip.cell(|ui| {
                    canvas_input(
                        CanvasInputKind::Picker,
                        ui,
                        |response, key_output, rect, ui| {
                            self.focus_hotkey(ui, &response, Key::Num2);
                            if let Some(pos) = response.interact_pointer_pos() {
                                self.color.hue =
                                    map(pos.x, (rect.left(), rect.right()), (0., 360.));
                                self.color.chroma =
                                    map(pos.y, (rect.top(), rect.bottom()), (CHROMA_MAX, 0.));
                            }
                            if let Some(o) = key_output {
                                value_update(&mut self.color.hue, o.horizontal, 3., 0., 360.);
                                value_update(
                                    &mut self.color.chroma,
                                    o.vertical,
                                    0.005,
                                    0.,
                                    CHROMA_MAX,
                                );
                            }

                            self.glow_paint(ui, ProgramKind::Picker2, rect.size());

                            let h = self.color.hue / 360.;
                            paint_picker_line(ui, true, rect, h, "H", &mut self.frame_end_labels);
                            let c = self.color.chroma / CHROMA_MAX;
                            paint_picker_line(ui, false, rect, c, "", &mut self.frame_end_labels);
                        },
                    );
                });
            });
    }

    fn update_sliders(&mut self, builder: StripBuilder) {
        let slider_thumb_color = self.fallbacks.cur_egui;
        let paint_slider_thumb =
            |ui: &mut egui::Ui, rect: egui::Rect, pos: f32, response: &Response| {
                let center = Pos2::new(
                    lerp(rect.left(), rect.right(), pos),
                    rect.top() + rect.height() / 2.,
                );

                ui.style_mut().visuals.widgets.inactive.bg_stroke.color = LINE_COLOR_LIGHT;
                ui.style_mut().visuals.widgets.hovered.bg_stroke.color = LINE_COLOR_LIGHT_FOCUSED;
                ui.style_mut().visuals.widgets.active.bg_stroke.color = LINE_COLOR_LIGHT_ACTIVE;

                let painter = ui.painter();

                let visuals = ui.style().interact(response);

                let stroke_color = visuals.bg_stroke.color;

                painter.rect(
                    egui::Rect::from_center_size(
                        center,
                        egui::vec2((rect.width() / 85.).clamp(9., 22.), rect.height() + 10.),
                    ),
                    4.,
                    slider_thumb_color,
                    Stroke::new(3.0, stroke_color),
                    egui::StrokeKind::Outside,
                );
            };

        let input_size = Vec2::new(68., 26.);
        let show_label = |ui: &mut egui::Ui, label: &str| {
            let label = egui::Label::new(label);
            ui.add_sized(Vec2::new(12., 26.), label);
        };
        builder.sizes(Size::remainder(), 4).vertical(|mut strip| {
            strip.cell(|ui| {
                ui.horizontal_centered(|ui| {
                    canvas_input(
                        CanvasInputKind::Slider,
                        ui,
                        |response, key_output, rect, ui| {
                            self.focus_hotkey(ui, &response, Key::Num3);
                            if let Some(pos) = response.interact_pointer_pos() {
                                self.color.lightness_r =
                                    map(pos.x, (rect.left(), rect.right()), (0., 1.));
                            }
                            if let Some(o) = key_output {
                                value_update(
                                    &mut self.color.lightness_r,
                                    o.horizontal,
                                    0.01,
                                    0.,
                                    1.,
                                );
                            }

                            self.glow_paint(ui, ProgramKind::Lightness, rect.size());
                            paint_slider_thumb(ui, rect, self.color.lightness_r, &response);
                        },
                    );

                    let get_set = |v: Option<f64>| match v {
                        Some(v) => {
                            self.color.lightness_r = v as f32;
                            v
                        }
                        None => self.color.lightness_r as f64,
                    };
                    let response = ui.add_sized(
                        input_size,
                        DragValue::from_get_set(get_set)
                            .speed(1. * 0.001)
                            .range(0.0..=1.0)
                            .max_decimals(4),
                    );
                    self.text_inputs.insert(response.id);
                    show_label(ui, "Lr");
                });
            });
            strip.cell(|ui| {
                ui.horizontal_centered(|ui| {
                    canvas_input(
                        CanvasInputKind::Slider,
                        ui,
                        |response, key_output, rect, ui| {
                            self.focus_hotkey(ui, &response, Key::Num4);
                            if let Some(pos) = response.interact_pointer_pos() {
                                self.color.chroma =
                                    map(pos.x, (rect.left(), rect.right()), (0., CHROMA_MAX));
                            }
                            if let Some(o) = key_output {
                                value_update(
                                    &mut self.color.chroma,
                                    o.horizontal,
                                    0.005,
                                    0.,
                                    CHROMA_MAX,
                                );
                            }

                            self.glow_paint(ui, ProgramKind::Chroma, rect.size());
                            paint_slider_thumb(ui, rect, self.color.chroma / CHROMA_MAX, &response);
                        },
                    );
                    let get_set = |v: Option<f64>| match v {
                        Some(v) => {
                            self.color.chroma = v as f32;
                            v
                        }
                        None => self.color.chroma as f64,
                    };
                    let reponse = ui.add_sized(
                        input_size,
                        DragValue::from_get_set(get_set)
                            .speed(CHROMA_MAX * 0.001)
                            .range(0.0..=CHROMA_MAX)
                            .max_decimals(4),
                    );
                    self.text_inputs.insert(reponse.id);
                    show_label(ui, "C");
                });
            });

            strip.cell(|ui| {
                ui.horizontal_centered(|ui| {
                    canvas_input(
                        CanvasInputKind::Slider,
                        ui,
                        |response, key_output, rect, ui| {
                            self.focus_hotkey(ui, &response, Key::Num5);
                            if let Some(pos) = response.interact_pointer_pos() {
                                self.color.hue =
                                    map(pos.x, (rect.left(), rect.right()), (0., 360.));
                            }
                            if let Some(o) = key_output {
                                value_update(&mut self.color.hue, o.horizontal, 3., 0., 360.);
                            }

                            self.glow_paint(ui, ProgramKind::Hue, rect.size());
                            paint_slider_thumb(ui, rect, self.color.hue / 360., &response);
                        },
                    );

                    let get_set = |v: Option<f64>| match v {
                        Some(v) => {
                            self.color.hue = v as f32;
                            v
                        }
                        None => self.color.hue as f64,
                    };
                    let response = ui.add_sized(
                        input_size,
                        DragValue::from_get_set(get_set)
                            .speed(360. * 0.001)
                            .range(0.0..=360.0)
                            .max_decimals(2),
                    );
                    self.text_inputs.insert(response.id);
                    show_label(ui, "H");
                });
            });

            strip.cell(|ui| {
                ui.horizontal_centered(|ui| {
                    canvas_input(
                        CanvasInputKind::Slider,
                        ui,
                        |response, key_output, rect, ui| {
                            self.focus_hotkey(ui, &response, Key::Num6);
                            if let Some(pos) = response.interact_pointer_pos() {
                                self.color.alpha =
                                    map(pos.x, (rect.left(), rect.right()), (0., 1.));
                                self.use_alpha = true;
                            }
                            if let Some(o) = key_output {
                                value_update(&mut self.color.alpha, o.horizontal, 0.01, 0., 1.);
                                self.use_alpha = true;
                            }

                            self.glow_paint(ui, ProgramKind::Alpha, rect.size());
                            paint_slider_thumb(ui, rect, self.color.alpha, &response);
                        },
                    );
                    let get_set = |v: Option<f64>| match v {
                        Some(v) => {
                            self.color.alpha = v as f32;
                            self.use_alpha = true;
                            v
                        }
                        None => self.color.alpha as f64,
                    };
                    let response = ui.add_sized(
                        input_size,
                        DragValue::from_get_set(get_set)
                            .speed(1. * 0.001)
                            .range(0.0..=1.0)
                            .max_decimals(4),
                    );
                    self.text_inputs.insert(response.id);
                    show_label(ui, "A");
                });
            });
        });
    }

    fn update_color_edit(&mut self, ui: &mut egui::Ui, prev: bool, fallback: LinearRgba, id: u8) {
        let mut text = if let Some(text) = self.input_text.remove(&id) {
            if let Some((c, use_alpha)) = parse_color(&text, self.format) {
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

            text
        } else {
            format_color(fallback, self.format, self.use_alpha)
        };

        let output = egui::TextEdit::singleline(&mut text)
            .margin(6.0)
            .desired_width(f32::INFINITY)
            .show(ui);
        self.text_inputs.insert(output.response.id);

        if output.response.has_focus() {
            self.input_text.insert(id, text.clone());
        }
    }

    fn update_color_previews(&mut self, builder: StripBuilder) {
        builder
            .size(Size::remainder())
            .size(Size::exact(54.))
            .vertical(|mut strip| {
                strip.cell(|ui| {
                    canvas_final(ui).show(ui, |ui| {
                        let (rect, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), ui.available_height()),
                            egui::Sense::empty(),
                        );
                        self.glow_paint(ui, ProgramKind::Final, rect.size());
                    });
                });

                strip.strip(|builder| {
                    builder
                        .sizes(Size::remainder(), 2)
                        .clip(true)
                        .horizontal(|mut strip| {
                            let color_label = |text: &str, fallback: bool| {
                                egui::Label::new(format!(
                                    "{text}{}",
                                    if fallback { " (fallback)" } else { "" }
                                ))
                                .wrap_mode(egui::TextWrapMode::Truncate)
                            };

                            strip.cell(|ui| {
                                self.update_color_edit(ui, true, self.fallbacks.prev, 0);
                                color_label("Previous Color", self.fallbacks.is_prev_fallback)
                                    .ui(ui);
                            });

                            strip.cell(|ui| {
                                self.update_color_edit(ui, false, self.fallbacks.cur, 1);
                                color_label("New Color", self.fallbacks.is_cur_fallback).ui(ui);
                            });
                        });
                });
            });
    }

    fn update_button_area(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        let style = ui.style_mut();
        style
            .text_styles
            .get_mut(&egui::TextStyle::Button)
            .unwrap()
            .size = 18.;
        style.spacing.button_padding = egui::vec2(4.0, 3.0);

        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("format")
                .width(ui.available_width().min(190.))
                .truncate()
                .selected_text(self.format.to_string())
                .height(500.)
                .show_ui(ui, |ui| {
                    for format in ColorFormat::iter() {
                        ui.selectable_value(&mut self.format, format, format.to_string());
                    }
                });

            let response = ui.add(
                egui::Button::new("?")
                    .min_size(Vec2::new(ui.available_height(), ui.available_height())),
            );
            if response.clicked() {
                self.show_settings = !self.show_settings;
            }

            ui.style_mut().spacing.window_margin = Margin::same(12);

            let mut show_settings = self.show_settings;

            egui::Window::new("Info")
                .open(&mut show_settings)
                .frame(egui::Frame::window(ui.style()))
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .resizable(false)
                .vscroll(true)
                .min_width(600.)
                .min_height(400.)
                .collapsible(false)
                .show(ui.ctx(), |ui| {
                    ui.label(RichText::new("Shortcuts").size(20.).strong());

                    if self.hotkey(ui, Key::Escape) {
                        self.show_settings = false;
                    }

                    ui.add_space(10.);

                    let headers = ["Key", "Action"];
                    let keys = [
                        ("q", "Quit"),
                        ("c", "Copy to clipboard"),
                        ("d", "Done (print result to console)"),
                        ("←/↓/↑/→", "Move focus or control input"),
                        ("h/j/k/l", "Move focus or control input (Vim style)"),
                        ("1/2", "Switch focus to pickers"),
                        ("3/4/5/6", "Switch focus to sliders"),
                        ("Tab/S-Tab", "Cycle focus"),
                        ("Esc/Enter", "Back/Submit"),
                    ];

                    let table = TableBuilder::new(ui)
                        .striped(true)
                        .column(Column::exact(100.))
                        .column(Column::remainder());

                    table
                        .header(26., |mut header| {
                            for h in headers {
                                header.col(|ui| {
                                    ui.strong(h);
                                });
                            }
                        })
                        .body(|mut body| {
                            for (key, action) in keys {
                                if !cfg!(target_arch = "wasm32") && key == "c" {
                                    continue;
                                }
                                if cfg!(target_arch = "wasm32") && (key == "q" || key == "d") {
                                    continue;
                                }
                                body.row(20., |mut row| {
                                    row.col(|ui| {
                                        ui.label(RichText::new(key).size(16.));
                                    });
                                    row.col(|ui| {
                                        ui.label(RichText::new(action).size(16.));
                                    });
                                })
                            }
                        });

                    ui.add_space(10.);
                    ui.label("Hold Ctrl (or Cmd on macOS) to force switching focus when the focused input would consume that key.");
                    ui.add_space(5.);
                    ui.label("Hold Shift to change values in larger steps.");
                });

            if !show_settings {
                self.show_settings = false;
            }
        });
        if self.format.needs_explicit_alpha() {
            ui.add_space(2.);
            if ui
                .add(egui::Checkbox::new(
                    &mut self.use_alpha,
                    RichText::new("Alpha"),
                ))
                .clicked()
                && !self.use_alpha
            {
                self.color.alpha = 1.;
            }
        }

        ui.add_space(5.);
        let max_w = ui.available_size().x;
        let max_h = ui.available_size().y;

        let (pad, font_size) = if cfg!(target_arch = "wasm32") {
            if max_w < 180. || max_h < 65. {
                (egui::vec2(6.0, 2.0), 18.)
            } else {
                (egui::vec2(16.0, 8.0), 26.)
            }
        } else {
            (
                egui::vec2(16.0, 8.0),
                if max_w > 400. && max_h > 120. {
                    42.
                } else if max_w > 250. && max_h > 90. {
                    34.
                } else {
                    26.
                },
            )
        };
        ui.style_mut().spacing.button_padding = pad;

        let text = if cfg!(target_arch = "wasm32") {
            "Copy to clipboard"
        } else {
            "DONE"
        };

        ui.centered_and_justified(|ui| {
            ui.style_mut().visuals.widgets.inactive.bg_stroke =
                Stroke::new(2.0, self.fallbacks.cur_egui);
            ui.style_mut().visuals.widgets.hovered.bg_stroke.width = 2.0;
            ui.style_mut().visuals.widgets.active.bg_stroke.width = 2.0;
            let button = egui::Button::new(RichText::new(text).size(font_size))
                .min_size(Vec2::new(max_w, max_h))
                .wrap_mode(egui::TextWrapMode::Wrap);
            let response = ui.add(button);

            if cfg!(target_arch = "wasm32") {
                let copy = self.hotkey(ui, Key::C);
                if response.clicked() || copy {
                    ui.ctx().copy_text(format_color(
                        self.fallbacks.cur,
                        self.format,
                        self.use_alpha,
                    ));
                    self.copied_notice = Some(Instant::now());
                }
            } else {
                let done = self.hotkey(ui, Key::D);
                let quit = self.hotkey(ui, Key::Q);
                if response.clicked() || done {
                    println!(
                        "{}",
                        format_color(self.fallbacks.cur, self.format, self.use_alpha)
                    );
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close)
                } else if quit {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            if self
                .copied_notice
                .is_some_and(|i| i.elapsed() < Duration::from_millis(400))
            {
                egui::Tooltip::always_open(
                    ui.ctx().clone(),
                    ui.layer_id(),
                    egui::Id::new("copied_tooltip"),
                    PopupAnchor::Pointer,
                )
                .gap(16.0)
                .show(|ui| {
                    ui.label("Copied!");
                });
            }
        });
    }

    fn focus_hotkey(&self, ui: &mut Ui, response: &Response, key: Key) {
        let text_input_focused =
            ui.memory(|m| m.focused().is_some_and(|id| self.text_inputs.contains(&id)));
        if ui.input(|input| {
            (!text_input_focused || input.modifiers.command) && input.key_pressed(key)
        }) {
            response.request_focus();
        }
    }

    fn hotkey(&self, ui: &mut Ui, key: Key) -> bool {
        let text_input_focused =
            ui.memory(|m| m.focused().is_some_and(|id| self.text_inputs.contains(&id)));
        ui.input(|input| (!text_input_focused || input.modifiers.command) && input.key_pressed(key))
    }
}

impl eframe::App for App {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        let text_input_focused =
            ctx.memory(|m| m.focused().is_some_and(|id| self.text_inputs.contains(&id)));

        if !text_input_focused || raw_input.modifiers.command {
            let nothing_focused = ctx.memory(|m| m.focused().is_none());
            let mut wants_something_focused = false;
            let mut wants_move_focus = false;

            let vim_keys = [
                (Key::H, Key::ArrowLeft),
                (Key::J, Key::ArrowDown),
                (Key::K, Key::ArrowUp),
                (Key::L, Key::ArrowRight),
            ];

            raw_input.events.retain_mut(|event| {
                if let egui::Event::Key { key, .. } = event {
                    if let Some((_, arrow_key)) =
                        vim_keys.iter().find(|(vim_key, _)| vim_key == key)
                    {
                        *key = *arrow_key;
                        wants_move_focus = true;
                        if nothing_focused {
                            wants_something_focused = true;
                            return false;
                        }
                    }
                    if vim_keys.iter().any(|(_, arrow_key)| arrow_key == key) {
                        wants_move_focus = true;
                        if nothing_focused {
                            wants_something_focused = true;
                            return false;
                        }
                    }
                }
                if let egui::Event::Text(text) = event {
                    if ["h", "j", "k", "l"].contains(&text.to_ascii_lowercase().as_str()) {
                        *text = String::new();
                    }
                }
                true
            });

            if wants_something_focused {
                ctx.memory_mut(|memory| {
                    memory.request_focus(self.first_input);
                });
            }

            // Make all inputs ignore focus lock if we want to move focus and command is pressed.
            if wants_move_focus && raw_input.modifiers.command {
                ctx.memory_mut(|memory| {
                    if let Some(id) = memory.focused() {
                        memory.set_focus_lock_filter(id, EventFilter::default());
                    }
                });
            }
        }
    }

    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if self.first_frame {
            log_startup::log("First frame start");
            self.first_frame = false;
        }

        let margin = egui::Margin {
            left: 26,
            right: 26,
            top: 20,
            bottom: 20,
        };

        let central_panel = egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(margin));

        self.calculate_fallbacks();

        central_panel.show(ctx, |ui| {
            StripBuilder::new(ui)
                .size(Size::remainder())
                .size(Size::relative(0.01))
                .size(Size::relative(0.20).at_least(120.))
                .size(Size::relative(0.01))
                .size(Size::relative(0.18).at_least(110.))
                .vertical(|mut strip| {
                    strip.strip(|builder| {
                        self.update_pickers(builder);
                    });
                    strip.cell(|_| {});
                    strip.strip(|builder| self.update_sliders(builder));
                    strip.cell(|_| {});
                    strip.strip(|builder| {
                        builder
                            .size(Size::remainder())
                            .size(Size::exact(10.))
                            .size(Size::relative(0.3).at_least(230.))
                            .horizontal(|mut strip| {
                                strip.strip(|builder| {
                                    self.update_color_previews(builder);
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
