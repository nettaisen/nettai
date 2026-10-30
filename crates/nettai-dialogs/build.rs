// The kinds of system this crate is built for, by name: `#[cfg(desktop)]` (Windows, macOS, Linux), `#[cfg(web)]` (a
// browser) and `#[cfg(phone)]` (`ios`, `android`).
fn main() {
    cfg_aliases::cfg_aliases! {
        web: { target_arch = "wasm32" },
        ios: { target_os = "ios" },
        android: { target_os = "android" },
        phone: { any(ios, android) },
        desktop: { not(any(web, phone)) },
    }
}
