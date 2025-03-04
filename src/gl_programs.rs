use bevy_color::{ColorToComponents, LinearRgba, Srgba};
use eframe::glow::{self, HasContext};
use egui::Vec2;
use strum::EnumIter;

use crate::gamut::{lr_to_l, Oklrcha};

#[derive(Default, Clone, Copy, EnumIter, Hash, PartialEq, Eq, Debug)]
pub enum ProgramKind {
    #[default]
    Picker,
    Picker2,
    Hue,
    Lightness,
    Chroma,
    Alpha,
    Final,
}

pub struct GlowProgram {
    kind: ProgramKind,
    program: glow::Program,
    vertex_array: glow::VertexArray,
}

#[cfg(not(target_arch = "wasm32"))]
macro_rules! shader_version {
    () => {
        "#version 330\n"
    };
}

#[cfg(target_arch = "wasm32")]
macro_rules! shader_version {
    () => {
        "#version 300 es\n"
    };
}

impl GlowProgram {
    pub fn new(gl: &glow::Context, kind: ProgramKind) -> Self {
        unsafe {
            let program = gl.create_program().unwrap();
            let vert_shader_source =
                concat!(shader_version!(), include_str!("./shaders/quad_vert.glsl"));
            let frag_shader_source = match kind {
                ProgramKind::Picker => concat!(
                    shader_version!(),
                    include_str!("shaders/functions.glsl"),
                    include_str!("shaders/picker_frag.glsl")
                ),
                ProgramKind::Picker2 => concat!(
                    shader_version!(),
                    include_str!("shaders/functions.glsl"),
                    include_str!("shaders/picker2_frag.glsl")
                ),
                ProgramKind::Hue => concat!(
                    shader_version!(),
                    include_str!("shaders/functions.glsl"),
                    include_str!("shaders/hue_frag.glsl")
                ),
                ProgramKind::Lightness => concat!(
                    shader_version!(),
                    include_str!("shaders/functions.glsl"),
                    include_str!("shaders/lightness_frag.glsl")
                ),
                ProgramKind::Chroma => concat!(
                    shader_version!(),
                    include_str!("shaders/functions.glsl"),
                    include_str!("shaders/chroma_frag.glsl")
                ),
                ProgramKind::Alpha => concat!(
                    shader_version!(),
                    include_str!("shaders/functions.glsl"),
                    include_str!("shaders/alpha_frag.glsl")
                ),
                ProgramKind::Final => concat!(
                    shader_version!(),
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

            Self {
                kind,
                program,
                vertex_array,
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
        color: Oklrcha,
        fallback_color: LinearRgba,
        previous_fallback_color: LinearRgba,
        size: Vec2,
    ) {
        unsafe {
            let uni_loc = |name: &str| gl.get_uniform_location(self.program, name);
            let set_uni_f32 = |name: &str, value: f32| {
                gl.uniform_1_f32(uni_loc(name).as_ref(), value);
            };
            gl.use_program(Some(self.program));
            gl.uniform_1_u32(uni_loc("supersample").as_ref(), 1);
            gl.uniform_2_f32(uni_loc("size").as_ref(), size.x, size.y);
            match self.kind {
                ProgramKind::Picker => {
                    set_uni_f32("hue", color.hue);
                }
                ProgramKind::Picker2 => set_uni_f32("lightness", lr_to_l(color.lightness_r)),
                ProgramKind::Hue => {}
                ProgramKind::Lightness => {
                    set_uni_f32("hue", color.hue);
                    set_uni_f32("chroma", color.chroma);
                }
                ProgramKind::Chroma => {
                    set_uni_f32("hue", color.hue);
                    set_uni_f32("lightness", lr_to_l(color.lightness_r));
                }
                ProgramKind::Alpha => {
                    gl.uniform_3_f32_slice(
                        uni_loc("color").as_ref(),
                        &Srgba::from(fallback_color).to_f32_array_no_alpha()[..],
                    );
                }
                ProgramKind::Final => {
                    gl.uniform_4_f32_slice(
                        uni_loc("prev_color").as_ref(),
                        &Srgba::from(previous_fallback_color).to_f32_array()[..],
                    );
                    gl.uniform_4_f32_slice(
                        uni_loc("color").as_ref(),
                        &Srgba::from(fallback_color).to_f32_array()[..],
                    );
                }
            }
            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
    }
}
