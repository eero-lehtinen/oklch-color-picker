use std::sync::LazyLock;

use bevy_color::{ColorToComponents, Srgba};
use eframe::glow::{self, HasContext};
use egui::Vec2;

use crate::app::{CurrentColors, Fallbacks};

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub enum ProgramKind {
    Picker(u8),
    Slider(u8),
    Final,
}

impl ProgramKind {
    pub fn iter_all() -> impl Iterator<Item = Self> {
        (0..=1)
            .map(ProgramKind::Picker)
            .chain((0..=3).map(ProgramKind::Slider))
            .chain(std::iter::once(ProgramKind::Final))
    }
}

pub struct GlowProgram {
    kind: ProgramKind,
    program: glow::Program,
    vertex_array: glow::VertexArray,
    supersample: u32,
}

fn shader_version() -> &'static str {
    if cfg!(target_arch = "wasm32") {
        "#version 300 es\n"
    } else {
        "#version 330\n"
    }
}

static VERT_SHADER_SOURCE: LazyLock<String> =
    LazyLock::new(|| [shader_version(), include_str!("./shaders/quad_vert.glsl")].concat());

impl GlowProgram {
    pub fn new(gl: &glow::Context, egui_ctx: &egui::Context, kind: ProgramKind) -> Self {
        unsafe {
            let program = gl.create_program().unwrap();
            let frag_shader_source_end = match kind {
                ProgramKind::Picker(0) => include_str!("shaders/picker0_frag.glsl"),
                ProgramKind::Picker(1) => include_str!("shaders/picker1_frag.glsl"),
                ProgramKind::Slider(0) => include_str!("shaders/slider0_frag.glsl"),
                ProgramKind::Slider(1) => include_str!("shaders/slider1_frag.glsl"),
                ProgramKind::Slider(2) => include_str!("shaders/slider2_frag.glsl"),
                ProgramKind::Slider(3) => include_str!("shaders/alpha_frag.glsl"),
                ProgramKind::Final => include_str!("shaders/final_frag.glsl"),
                _ => panic!("Invalid ProgramKind"),
            };
            let define = if cfg!(target_arch = "wasm32") {
                ""
            } else {
                "#define OUTPUT_LINEAR_COLOR\n"
            };

            let frag_shader_source = [
                shader_version(),
                define,
                include_str!("shaders/functions.glsl"),
                frag_shader_source_end,
            ]
            .concat();
            let shader_sources = [
                (glow::VERTEX_SHADER, &*VERT_SHADER_SOURCE),
                (glow::FRAGMENT_SHADER, &frag_shader_source),
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
                        "Failed to compile '{kind:?}' {shader_type}: {}",
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

            // Don't supersample if resolution is already massive (often on web mobile)
            let supersample = if egui_ctx.native_pixels_per_point().is_some_and(|p| p > 2.1) {
                0
            } else {
                1
            };

            Self {
                kind,
                program,
                vertex_array,
                supersample,
            }
        }
    }

    pub fn destroy(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vertex_array);
        }
    }

    pub fn paint(
        &self,
        gl: &glow::Context,
        colors: &CurrentColors,
        fallbacks: &Fallbacks,
        size: Vec2,
    ) {
        unsafe {
            if !cfg!(target_arch = "wasm32") {
                gl.enable(glow::FRAMEBUFFER_SRGB);
            }
            gl.use_program(Some(self.program));

            let uni_loc = |name: &str| gl.get_uniform_location(self.program, name);

            gl.uniform_1_u32(uni_loc("supersample").as_ref(), self.supersample);
            gl.uniform_2_f32(uni_loc("size").as_ref(), size.x, size.y);
            gl.uniform_1_u32(
                uni_loc("mode").as_ref(),
                matches!(colors, CurrentColors::Okhsv(..)) as u32,
            );
            match self.kind {
                // Alpha
                ProgramKind::Slider(3) => {
                    gl.uniform_3_f32_slice(
                        uni_loc("color").as_ref(),
                        &fallbacks.cur.to_f32_array_no_alpha()[..],
                    );
                }
                ProgramKind::Picker(_) | ProgramKind::Slider(_) => {
                    gl.uniform_3_f32_slice(uni_loc("values").as_ref(), &colors.values()[0..3]);
                }
                ProgramKind::Final => {
                    gl.uniform_4_f32_slice(
                        uni_loc("prev_color").as_ref(),
                        &fallbacks.prev.to_f32_array()[..],
                    );
                    gl.uniform_4_f32_slice(
                        uni_loc("color").as_ref(),
                        &fallbacks.cur.to_f32_array()[..],
                    );
                }
            }
            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
    }
}
