use objc2_app_kit::{NSColorSpace, NSView};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Tagging the window as sRGB makes the compositor convert it to each
/// display's color space, including when moved between displays.
pub fn init(window: &impl HasWindowHandle) {
    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };
    let view: &NSView = unsafe { handle.ns_view.cast().as_ref() };
    if let Some(window) = view.window() {
        window.setColorSpace(Some(&NSColorSpace::sRGBColorSpace()));
    }
}

pub fn monitor(_: &impl HasWindowHandle) -> Option<isize> {
    None
}

pub fn icc_profile(_: &impl HasWindowHandle) -> Option<Vec<u8>> {
    None
}
