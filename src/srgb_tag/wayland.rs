use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum,
    backend::{Backend, ObjectId},
    delegate_noop,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{wl_registry::WlRegistry, wl_surface::WlSurface},
};
use wayland_protocols::wp::color_management::v1::client::{
    wp_color_management_surface_v1::WpColorManagementSurfaceV1,
    wp_color_manager_v1::{
        self, Feature, Primaries, RenderIntent, TransferFunction, WpColorManagerV1,
    },
    wp_image_description_creator_params_v1::WpImageDescriptionCreatorParamsV1,
    wp_image_description_v1::{self, WpImageDescriptionV1},
};

/// Destroying the color management surface would remove the tag, so this
/// must live as long as the window.
pub struct SrgbSurface {
    _manager: WpColorManagerV1,
    _surface: WpColorManagementSurfaceV1,
    _queue: EventQueue<State>,
}

#[derive(Default)]
struct State {
    parametric: bool,
    perceptual: bool,
    srgb_primaries: bool,
    gamma22: bool,
    srgb_tf: bool,
    description_ready: Option<bool>,
}

/// Returns `None` on X11 and on compositors without the color management
/// protocol.
pub fn tag_srgb(window: &(impl HasWindowHandle + HasDisplayHandle)) -> Option<SrgbSurface> {
    let RawDisplayHandle::Wayland(display) = window.display_handle().ok()?.as_raw() else {
        return None;
    };
    let RawWindowHandle::Wayland(handle) = window.window_handle().ok()?.as_raw() else {
        return None;
    };

    let backend = unsafe { Backend::from_foreign_display(display.display.as_ptr().cast()) };
    let conn = Connection::from_backend(backend);
    let (globals, mut queue) = registry_queue_init::<State>(&conn).ok()?;
    let qh = queue.handle();
    let manager: WpColorManagerV1 = globals.bind(&qh, 1..=1, ()).ok()?;

    let mut state = State::default();
    queue.roundtrip(&mut state).ok()?;
    // The spec recommends gamma22 over the deprecated srgb for computer
    // graphics.
    let tf = match (state.gamma22, state.srgb_tf) {
        (true, _) => TransferFunction::Gamma22,
        (false, true) => TransferFunction::Srgb,
        (false, false) => return None,
    };
    if !(state.parametric && state.perceptual && state.srgb_primaries) {
        return None;
    }

    let creator = manager.create_parametric_creator(&qh, ());
    creator.set_primaries_named(Primaries::Srgb);
    creator.set_tf_named(tf);
    let description = creator.create(&qh, ());
    while state.description_ready.is_none() {
        queue.blocking_dispatch(&mut state).ok()?;
    }
    if state.description_ready != Some(true) {
        description.destroy();
        return None;
    }

    let surface_id =
        unsafe { ObjectId::from_ptr(WlSurface::interface(), handle.surface.as_ptr().cast()) }
            .ok()?;
    let wl_surface = WlSurface::from_id(&conn, surface_id).ok()?;
    let surface = manager.get_surface(&wl_surface, &qh, ());
    surface.set_image_description(&description, RenderIntent::Perceptual);
    description.destroy();
    conn.flush().ok()?;

    Some(SrgbSurface {
        _manager: manager,
        _surface: surface,
        _queue: queue,
    })
}

impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: <WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WpColorManagerV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &WpColorManagerV1,
        event: wp_color_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use wp_color_manager_v1::Event;
        match event {
            Event::SupportedIntent {
                render_intent: WEnum::Value(RenderIntent::Perceptual),
            } => state.perceptual = true,
            Event::SupportedFeature {
                feature: WEnum::Value(Feature::Parametric),
            } => state.parametric = true,
            Event::SupportedPrimariesNamed {
                primaries: WEnum::Value(Primaries::Srgb),
            } => state.srgb_primaries = true,
            Event::SupportedTfNamed {
                tf: WEnum::Value(TransferFunction::Gamma22),
            } => state.gamma22 = true,
            Event::SupportedTfNamed {
                tf: WEnum::Value(TransferFunction::Srgb),
            } => state.srgb_tf = true,
            _ => {}
        }
    }
}

impl Dispatch<WpImageDescriptionV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &WpImageDescriptionV1,
        event: wp_image_description_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wp_image_description_v1::Event::Ready { .. } => state.description_ready = Some(true),
            wp_image_description_v1::Event::Failed { .. } => state.description_ready = Some(false),
            _ => {}
        }
    }
}

delegate_noop!(State: WpImageDescriptionCreatorParamsV1);
delegate_noop!(State: WpColorManagementSurfaceV1);
