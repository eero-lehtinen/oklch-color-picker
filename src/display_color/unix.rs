use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use x11rb::{
    connection::Connection,
    protocol::xproto::{AtomEnum, ConnectionExt},
};

pub fn init(_: &impl HasWindowHandle) {}

pub fn monitor(_: &impl HasWindowHandle) -> Option<isize> {
    None
}

/// Color-managing Wayland compositors treat untagged surfaces as sRGB, so
/// only X11 needs a profile.
pub fn icc_profile(window: &impl HasWindowHandle) -> Option<Vec<u8>> {
    match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_) => x11_icc_profile(),
        _ => None,
    }
}

/// Reads the profile that colord and similar daemons set on the root window
/// per the X Color Management spec. The per-output `_ICC_PROFILE_n` atoms of
/// multi-monitor setups are not handled.
fn x11_icc_profile() -> Option<Vec<u8>> {
    let (conn, screen) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots.get(screen)?.root;
    let atom = conn
        .intern_atom(true, b"_ICC_PROFILE")
        .ok()?
        .reply()
        .ok()?
        .atom;
    if atom == x11rb::NONE {
        return None;
    }
    let reply = conn
        .get_property(false, root, atom, AtomEnum::ANY, 0, u32::MAX / 4)
        .ok()?
        .reply()
        .ok()?;
    (!reply.value.is_empty()).then_some(reply.value)
}
