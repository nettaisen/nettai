// The platform's codecs, by name: `#[cfg(apple)]` (macOS, iOS), `#[cfg(web)]` (a browser), `#[cfg(ffmpeg)]`
// (Linux and other Unix desktops: system `ffmpeg`); Windows (`target_os = "windows"`) and Android
// (`target_os = "android"`) use Rust's own target names.
fn main() {
    cfg_aliases::cfg_aliases! {
        web: { target_arch = "wasm32" },
        apple: { any(target_os = "macos", target_os = "ios") },
        ffmpeg: { not(any(web, apple, target_os = "windows", target_os = "android")) },
    }
}
