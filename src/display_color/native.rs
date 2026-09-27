use std::sync::Arc;

use moxcms::{
    CmsError, ColorProfile, Layout, RenderingIntent, TransformF32Executor, TransformOptions,
};

#[cfg(target_os = "macos")]
use super::macos as platform;
#[cfg(all(unix, not(target_os = "macos")))]
use super::unix as platform;
#[cfg(windows)]
use super::windows as platform;

pub struct DisplayColor {
    checked: bool,
    monitor: Option<isize>,
    profile: Option<Vec<u8>>,
    transform: Option<Arc<TransformF32Executor>>,
}

impl DisplayColor {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        platform::init(cc);
        Self {
            checked: false,
            monitor: None,
            profile: None,
            transform: None,
        }
    }

    /// Returns true if the output transform changed.
    ///
    /// Profiles are re-read when the window moves to another monitor or
    /// regains focus, which covers changing display settings in another app.
    pub fn update(&mut self, frame: &eframe::Frame, ctx: &egui::Context) -> bool {
        let focus_gained = ctx.input(|i| {
            i.raw
                .events
                .iter()
                .any(|e| matches!(e, egui::Event::WindowFocused(true)))
        });
        let monitor = platform::monitor(frame);
        if self.checked && !focus_gained && monitor == self.monitor {
            return false;
        }
        self.checked = true;
        self.monitor = monitor;

        let profile = platform::icc_profile(frame);
        if profile == self.profile {
            return false;
        }
        self.transform = profile
            .as_deref()
            .and_then(|icc| match create_transform(icc) {
                Ok(transform) => Some(transform),
                Err(e) => {
                    eprintln!("Ignoring unsupported display color profile: {e}");
                    None
                }
            });
        self.profile = profile;
        true
    }

    pub fn lut(&self) -> Vec<f32> {
        let mut lut = super::identity_lut();
        if let Some(transform) = &self.transform {
            let src = lut.clone();
            if transform.transform(&src, &mut lut).is_err() {
                return src;
            }
            for v in &mut lut {
                *v = v.clamp(0., 1.);
            }
        }
        lut
    }

    pub fn to_display(&self, srgb: [f32; 3]) -> [f32; 3] {
        let mut out = srgb;
        if let Some(transform) = &self.transform
            && transform.transform(&srgb, &mut out).is_err()
        {
            return srgb;
        }
        out.map(|v| v.clamp(0., 1.))
    }
}

fn create_transform(icc: &[u8]) -> Result<Arc<TransformF32Executor>, CmsError> {
    let display = ColorProfile::new_from_slice(icc)?;
    ColorProfile::new_srgb().create_transform_f32(
        Layout::Rgb,
        &display,
        Layout::Rgb,
        TransformOptions {
            rendering_intent: RenderingIntent::RelativeColorimetric,
            ..Default::default()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p3_display() -> DisplayColor {
        let icc = ColorProfile::new_display_p3().encode().unwrap();
        DisplayColor {
            checked: true,
            monitor: None,
            transform: Some(create_transform(&icc).unwrap()),
            profile: Some(icc),
        }
    }

    fn assert_close(a: [f32; 3], b: [f32; 3]) {
        assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 0.003), "{a:?} != {b:?}");
    }

    #[test]
    fn converts_srgb_to_display_p3() {
        let display = p3_display();
        assert_close(display.to_display([1., 0., 0.]), [0.9175, 0.2003, 0.1386]);
        assert_close(display.to_display([0.5, 0.5, 0.5]), [0.5, 0.5, 0.5]);
    }

    #[test]
    fn lut_matches_direct_conversion() {
        let display = p3_display();
        let lut = display.lut();
        let n = crate::display_color::LUT_SIZE;
        let (r, g, b) = (n - 1, 8, 20);
        let i = ((b * n + g) * n + r) * 3;
        let step = 1. / (n - 1) as f32;
        let srgb = [r as f32 * step, g as f32 * step, b as f32 * step];
        assert_close([lut[i], lut[i + 1], lut[i + 2]], display.to_display(srgb));
    }
}
