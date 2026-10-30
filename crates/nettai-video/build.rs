// The kinds of system this crate is built for, by name: `#[cfg(desktop)]` (Windows, macOS, Linux), `#[cfg(web)]` (a
// browser) and `#[cfg(phone)]` (iOS, Android), as nettai-dialogs names them.
fn main() {
    cfg_aliases::cfg_aliases! {
        web: { target_arch = "wasm32" },
        phone: { any(target_os = "ios", target_os = "android") },
        desktop: { not(any(web, phone)) },
    }
}
