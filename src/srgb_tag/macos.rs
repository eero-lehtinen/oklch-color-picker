use objc2_app_kit::{NSColorSpace, NSView};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

pub fn tag_srgb(window: &impl HasWindowHandle) {
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
