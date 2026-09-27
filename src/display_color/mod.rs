//! Conversion of the sRGB output to the color space of the display, for
//! platforms where the OS doesn't color-manage the window.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(all(unix, not(target_os = "macos")))]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(not(target_arch = "wasm32"))]
pub use native::DisplayColor;
#[cfg(target_arch = "wasm32")]
pub use web::DisplayColor;

pub const LUT_SIZE: usize = 33;

/// sRGB encoded RGB triplets with red changing fastest, matching the texel
/// order of a 3D texture.
fn identity_lut() -> Vec<f32> {
    let step = 1. / (LUT_SIZE - 1) as f32;
    let mut lut = Vec::with_capacity(LUT_SIZE.pow(3) * 3);
    for b in 0..LUT_SIZE {
        for g in 0..LUT_SIZE {
            for r in 0..LUT_SIZE {
                lut.extend([r as f32 * step, g as f32 * step, b as f32 * step]);
            }
        }
    }
    lut
}

#[cfg(target_arch = "wasm32")]
mod web {
    /// Browsers color-manage the canvas as sRGB.
    pub struct DisplayColor;

    impl DisplayColor {
        pub fn new(_: &eframe::CreationContext<'_>) -> Self {
            Self
        }

        pub fn update(&mut self, _: &eframe::Frame, _: &egui::Context) -> bool {
            false
        }

        pub fn lut(&self) -> Vec<f32> {
            super::identity_lut()
        }

        pub fn to_display(&self, srgb: [f32; 3]) -> [f32; 3] {
            srgb
        }
    }
}
