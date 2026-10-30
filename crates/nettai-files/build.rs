// The browser by name: `#[cfg(web)]`.
fn main() {
    cfg_aliases::cfg_aliases! {
        web: { target_arch = "wasm32" },
    }
}
