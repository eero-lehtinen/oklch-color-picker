#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy_color::{ColorToComponents, ColorToPacked, Hue, LinearRgba, Oklaba, Oklcha, Srgba};
use enum_map::enum_map;
use macroquad::{conf::UpdateTrigger, prelude::*};
use miniquad::{window::schedule_update, BlendFactor, BlendState, BlendValue, Equation};

mod gamut;

fn window_conf() -> macroquad::conf::Conf {
    macroquad::conf::Conf {
        miniquad_conf: Conf {
            window_title: "Color Picker Test".to_owned(),
            high_dpi: true,
            platform: miniquad::conf::Platform {
                blocking_event_loop: true,
                ..Default::default()
            },
            sample_count: 4,
            window_width: 1400,
            window_height: 1000,
            ..Default::default()
        },
        update_on: Some(UpdateTrigger {
            key_down: true,
            mouse_down: true,
            mouse_up: true,
            mouse_motion: true,
            mouse_wheel: false,
            specific_key: None,
            touch: true,
        }),
        ..Default::default()
    }
}

const CHROMA_MAX: f32 = 0.33;

fn resize_texture(w: usize, h: usize, texture: &mut Texture2D) {
    if w == texture.width() as usize && h == texture.height() as usize {
        return;
    }
    let bytes = vec![0; w * h * 4];
    *texture = Texture2D::from_rgba8(w as u16, h as u16, &bytes);
}

fn scale_rect(rect: Rect, scale: Vec2) -> Rect {
    Rect::new(
        rect.x * scale.x,
        rect.y * scale.y,
        rect.w * scale.x,
        rect.h * scale.y,
    )
}

fn map(x: f32, from: (f32, f32), to: (f32, f32)) -> f32 {
    (to.1 - to.0) * (x - from.0) / (from.1 - from.0) + to.0
}
#[derive(Debug, Copy, Clone, enum_map::Enum)]
enum ElementKind {
    Picker,
    Lightness,
    Chroma,
    Hue,
    Alpha,
    FinalColor,
}

struct Element {
    rect: Rect,
    accept_input: bool,
}

impl Element {
    fn new(rect: Rect, accept_input: bool) -> Self {
        Self { rect, accept_input }
    }
}

fn line(x1: f32, y1: f32, x2: f32, y2: f32) {
    draw_line(x1, y1, x2, y2, 1., Color::new(0.1, 0.1, 0.1, 1.));
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut images = enum_map! {
        ElementKind::Picker => Texture2D::empty(),
        ElementKind::Lightness => Texture2D::empty(),
        ElementKind::Chroma => Texture2D::empty(),
        ElementKind::Hue => Texture2D::empty(),
        ElementKind::Alpha => Texture2D::empty(),
        ElementKind::FinalColor => Texture2D::empty(),
    };

    let init_lightness = 0.8;
    let init_chrome = 0.1;
    let init_hue = 0.;
    let init_alpha = 1.;

    let mut color = Oklcha::new(init_lightness, init_chrome, init_hue, init_alpha);

    let mut fallback_color: Oklcha;

    let pipeline_params = PipelineParams {
        color_blend: Some(BlendState::new(
            Equation::Add,
            BlendFactor::Value(BlendValue::SourceAlpha),
            BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
        )),
        ..Default::default()
    };

    let picker_mat = load_material(
        ShaderSource::Glsl {
            vertex: include_str!("quad_vert.glsl"),
            fragment: concat!(
                include_str!("functions.glsl"),
                include_str!("picker_frag.glsl")
            ),
        },
        MaterialParams {
            uniforms: vec![UniformDesc::new("hue", UniformType::Float1)],
            pipeline_params,
            ..Default::default()
        },
    )
    .unwrap();

    let lightness_mat = load_material(
        ShaderSource::Glsl {
            vertex: include_str!("quad_vert.glsl"),
            fragment: concat!(
                include_str!("functions.glsl"),
                include_str!("lightness_frag.glsl")
            ),
        },
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("hue", UniformType::Float1),
                UniformDesc::new("chroma", UniformType::Float1),
                UniformDesc::new("width", UniformType::Float1),
            ],
            pipeline_params,
            ..Default::default()
        },
    )
    .unwrap();

    let chroma_mat = load_material(
        ShaderSource::Glsl {
            vertex: include_str!("quad_vert.glsl"),
            fragment: concat!(
                include_str!("functions.glsl"),
                include_str!("chroma_frag.glsl")
            ),
        },
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("hue", UniformType::Float1),
                UniformDesc::new("lightness", UniformType::Float1),
                UniformDesc::new("width", UniformType::Float1),
            ],
            pipeline_params,
            ..Default::default()
        },
    )
    .unwrap();

    let hue_mat = load_material(
        ShaderSource::Glsl {
            vertex: include_str!("quad_vert.glsl"),
            fragment: concat!(
                include_str!("functions.glsl"),
                include_str!("hue_frag.glsl")
            ),
        },
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("chroma", UniformType::Float1),
                UniformDesc::new("lightness", UniformType::Float1),
                UniformDesc::new("width", UniformType::Float1),
            ],
            pipeline_params,
            ..Default::default()
        },
    )
    .unwrap();

    let alpha_mat = load_material(
        ShaderSource::Glsl {
            vertex: include_str!("quad_vert.glsl"),
            fragment: concat!(
                include_str!("functions.glsl"),
                include_str!("alpha_frag.glsl")
            ),
        },
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("color", UniformType::Float4),
                UniformDesc::new("width", UniformType::Float1),
            ],
            pipeline_params,
            ..Default::default()
        },
    )
    .unwrap();

    let final_color_mat = load_material(
        ShaderSource::Glsl {
            vertex: include_str!("quad_vert.glsl"),
            fragment: concat!(
                include_str!("functions.glsl"),
                include_str!("final_frag.glsl")
            ),
        },
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("color", UniformType::Float4),
                UniformDesc::new("width", UniformType::Float1),
            ],
            pipeline_params,
            ..Default::default()
        },
    )
    .unwrap();

    let mut focused: Option<(ElementKind, Vec2)> = None;

    let rect = |x: u32, y: u32, w: u32, h: u32| {
        Rect::new(
            x as f32 * 0.01,
            y as f32 * 0.01,
            w as f32 * 0.01,
            h as f32 * 0.01,
        )
    };

    let mut elements = enum_map! {
        ElementKind::Picker => Element::new(rect(4, 4, 50, 58), true),
        ElementKind::Hue => Element::new(rect(4, 66, 50, 6), true),
        ElementKind::Lightness => Element::new(rect(4, 74, 50, 6), true),
        ElementKind::Chroma => Element::new(rect(4, 82, 50, 6), true),
        ElementKind::Alpha => Element::new(rect(4, 90, 50, 6), true),
        ElementKind::FinalColor => Element::new(rect(56, 4, 40, 46), false),
    };

    loop {
        clear_background(Color::from_rgba(40, 40, 44, 255));

        let screen_size = Vec2::new(screen_width(), screen_height());

        let mouse_pos = Vec2::from(mouse_position()) / screen_size;
        if focused.is_none() && is_mouse_button_pressed(MouseButton::Left) {
            for (kind, input) in elements.iter_mut().filter(|(_, input)| input.accept_input) {
                if input.rect.contains(mouse_pos) {
                    focused = Some((kind, Vec2::ZERO));
                    break;
                }
            }
        }
        if !is_mouse_button_down(MouseButton::Left) {
            focused = None;
        }

        if let Some((kind, relative_input)) = &mut focused {
            let input = &mut elements[*kind];
            relative_input.x = map(
                mouse_pos.x,
                (input.rect.x, input.rect.x + input.rect.w),
                (0., 1.),
            );
            relative_input.y = map(
                mouse_pos.y,
                (input.rect.y, input.rect.y + input.rect.h),
                (0., 1.),
            );

            *relative_input = relative_input.clamp(Vec2::ZERO, Vec2::ONE);

            match kind {
                ElementKind::Picker => {
                    color.lightness = relative_input.x;
                    color.chroma = (1. - relative_input.y) * CHROMA_MAX;
                }
                ElementKind::Hue => {
                    color.hue = relative_input.x * 360.;
                }
                ElementKind::Lightness => {
                    color.lightness = relative_input.x;
                }
                ElementKind::Chroma => {
                    color.chroma = relative_input.x * CHROMA_MAX;
                }
                ElementKind::Alpha => {
                    color.alpha = relative_input.x;
                }
                _ => {}
            }
        }

        let rgba = gamut::gamut_clip_preserve_chroma(color.into());

        fallback_color = rgba.into();

        for (kind, texture) in images.iter_mut() {
            let rect = scale_rect(elements[kind].rect, screen_size);
            resize_texture(rect.w.round() as usize, rect.h.round() as usize, texture);
        }

        let r = scale_rect(elements[ElementKind::Picker].rect, screen_size);

        draw_rectangle_lines(r.x, r.y, r.w, r.h, 2., Color::new(0.6, 0.6, 0.6, 0.5));
        gl_use_material(&picker_mat);
        picker_mat.set_uniform("hue", color.hue);
        draw_rectangle(r.x, r.y, r.w, r.h, WHITE);

        gl_use_material(&lightness_mat);
        lightness_mat.set_uniform("hue", color.hue);
        lightness_mat.set_uniform("chroma", color.chroma);
        let r = scale_rect(elements[ElementKind::Lightness].rect, screen_size);
        lightness_mat.set_uniform("width", r.w / r.h);
        draw_rectangle(r.x, r.y, r.w, r.h, WHITE);

        gl_use_material(&chroma_mat);
        chroma_mat.set_uniform("hue", color.hue);
        chroma_mat.set_uniform("lightness", color.lightness);
        let r = scale_rect(elements[ElementKind::Chroma].rect, screen_size);
        chroma_mat.set_uniform("width", r.w / r.h);
        draw_rectangle(r.x, r.y, r.w, r.h, WHITE);

        gl_use_material(&hue_mat);
        let r = scale_rect(elements[ElementKind::Hue].rect, screen_size);
        hue_mat.set_uniform("width", r.w / r.h);
        hue_mat.set_uniform("chroma", color.chroma);
        hue_mat.set_uniform("lightness", color.lightness);
        draw_rectangle(r.x, r.y, r.w, r.h, WHITE);

        let fallback_srgba = Srgba::from(fallback_color);
        gl_use_material(&alpha_mat);
        let r = scale_rect(elements[ElementKind::Alpha].rect, screen_size);
        alpha_mat.set_uniform("color", fallback_srgba.to_f32_array());
        alpha_mat.set_uniform("width", r.w / r.h);
        draw_rectangle(r.x, r.y, r.w, r.h, WHITE);

        gl_use_material(&final_color_mat);
        let r = scale_rect(elements[ElementKind::FinalColor].rect, screen_size);
        final_color_mat.set_uniform("color", fallback_srgba.to_f32_array());
        final_color_mat.set_uniform("width", r.w / r.h);
        draw_rectangle(r.x, r.y, r.w, r.h, WHITE);

        gl_use_default_material();

        let picker_rect = scale_rect(elements[ElementKind::Picker].rect, screen_size);
        let chroma_line_pos = color.chroma / CHROMA_MAX;
        line(
            picker_rect.x,
            picker_rect.y + (1. - chroma_line_pos) * picker_rect.h,
            picker_rect.x + picker_rect.w,
            picker_rect.y + (1. - chroma_line_pos) * picker_rect.h,
        );
        let lightness_line_pos = color.lightness;
        line(
            picker_rect.x + lightness_line_pos * picker_rect.w,
            picker_rect.y,
            picker_rect.x + lightness_line_pos * picker_rect.w,
            picker_rect.y + picker_rect.h,
        );

        let hue_rect = scale_rect(elements[ElementKind::Hue].rect, screen_size);
        let hue_line_pos = color.hue / 360.;
        line(
            hue_rect.x + hue_line_pos * hue_rect.w,
            hue_rect.y,
            hue_rect.x + hue_line_pos * hue_rect.w,
            hue_rect.y + hue_rect.h,
        );

        let lightness_rect = scale_rect(elements[ElementKind::Lightness].rect, screen_size);
        let lightness_line_pos = color.lightness;
        line(
            lightness_rect.x + lightness_line_pos * lightness_rect.w,
            lightness_rect.y,
            lightness_rect.x + lightness_line_pos * lightness_rect.w,
            lightness_rect.y + lightness_rect.h,
        );

        let chroma_rect = scale_rect(elements[ElementKind::Chroma].rect, screen_size);
        line(
            chroma_rect.x + chroma_line_pos * chroma_rect.w,
            chroma_rect.y,
            chroma_rect.x + chroma_line_pos * chroma_rect.w,
            chroma_rect.y + chroma_rect.h,
        );

        let alpha_rect = scale_rect(elements[ElementKind::Alpha].rect, screen_size);
        let alpha_line_pos = color.alpha;
        line(
            alpha_rect.x + alpha_line_pos * alpha_rect.w,
            alpha_rect.y,
            alpha_rect.x + alpha_line_pos * alpha_rect.w,
            alpha_rect.y + alpha_rect.h,
        );

        next_frame().await
    }
}
