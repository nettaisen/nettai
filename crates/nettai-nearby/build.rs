// The targets by name: `#[cfg(desktop)]` (Windows, macOS, Linux), `#[cfg(web)]` (a browser), `#[cfg(ios)]`,
// `#[cfg(android)]`. Used in `src/platform.rs` alone.
fn main() {
    cfg_aliases::cfg_aliases! {
        web: { target_arch = "wasm32" },
        ios: { target_os = "ios" },
        android: { target_os = "android" },
        desktop: { not(any(web, ios, android)) },
    }
}
