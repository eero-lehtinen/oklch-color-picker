use eframe::glow::{self, HasContext};

use crate::display_color::LUT_SIZE;

/// Must not be 0, which egui uses for its own textures.
pub const TEXTURE_UNIT: u32 = 1;

pub struct OutputLut {
    texture: glow::Texture,
}

impl OutputLut {
    pub fn new(gl: &glow::Context, lut: &[f32]) -> Self {
        unsafe {
            let texture = gl.create_texture().expect("Cannot create texture");
            gl.bind_texture(glow::TEXTURE_3D, Some(texture));
            for (param, value) in [
                (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
                (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
                (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
                (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
                (glow::TEXTURE_WRAP_R, glow::CLAMP_TO_EDGE),
            ] {
                gl.tex_parameter_i32(glow::TEXTURE_3D, param, value as i32);
            }
            let this = Self { texture };
            this.upload(gl, lut);
            this
        }
    }

    pub fn upload(&self, gl: &glow::Context, lut: &[f32]) {
        let bytes: Vec<u8> = lut.iter().flat_map(|v| v.to_ne_bytes()).collect();
        let size = LUT_SIZE as i32;
        unsafe {
            gl.bind_texture(glow::TEXTURE_3D, Some(self.texture));
            gl.tex_image_3d(
                glow::TEXTURE_3D,
                0,
                glow::RGB16F as i32,
                size,
                size,
                size,
                0,
                glow::RGB,
                glow::FLOAT,
                glow::PixelUnpackData::Slice(Some(&bytes)),
            );
            gl.bind_texture(glow::TEXTURE_3D, None);
        }
    }

    pub fn texture(&self) -> glow::Texture {
        self.texture
    }

    pub fn destroy(&self, gl: &glow::Context) {
        unsafe { gl.delete_texture(self.texture) };
    }
}
