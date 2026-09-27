//! Tags the window as sRGB so the OS converts it to each display's color
//! space. Untagged OpenGL output is shown unconverted on macOS and on some
//! Wayland compositors, which oversaturates it on wide-gamut displays.
//!
//! Windows needs no tag: with HDR or Auto Color Management on, it treats
//! OpenGL windows as sRGB, and otherwise it converts nothing.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(all(unix, not(target_os = "macos")))]
mod wayland;

pub struct SrgbTag {
    #[cfg(all(unix, not(target_os = "macos")))]
    _wayland: Option<wayland::SrgbSurface>,
}

pub fn tag_window(cc: &eframe::CreationContext<'_>) -> SrgbTag {
    #[cfg(target_os = "macos")]
    macos::tag_srgb(cc);
    #[cfg(not(unix))]
    let _ = cc;

    SrgbTag {
        #[cfg(all(unix, not(target_os = "macos")))]
        _wayland: wayland::tag_srgb(cc),
    }
}
