//! No gamepads, ever: what a desktop build without the `sdl3` feature gets. The API stays callable, so a host can drop
//! SDL from its link without touching any call sites.

pub fn init(_app_name: &str) {
    log::info!("gamepad support is not compiled in (no `sdl3` feature)");
}

pub fn next_event() -> Option<crate::Event> {
    None
}
