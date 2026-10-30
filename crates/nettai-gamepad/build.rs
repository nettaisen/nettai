// The targets by name: `#[cfg(desktop)]`, `#[cfg(web)]`, `#[cfg(phone)]` (`#[cfg(ios)]`, `#[cfg(android)]`), and
// `#[cfg(sdl)]` (the desktop with the `sdl3` feature).
fn main() {
    cfg_aliases::cfg_aliases! {
        web: { target_arch = "wasm32" },
        ios: { target_os = "ios" },
        android: { target_os = "android" },
        phone: { any(ios, android) },
        desktop: { not(any(web, phone)) },
        sdl: { all(desktop, feature = "sdl3") },
    }
}
