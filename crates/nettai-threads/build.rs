// The targets by name: `#[cfg(web)]` (a browser) and natively the rest.
fn main() {
    cfg_aliases::cfg_aliases! {
        web: { target_arch = "wasm32" },
    }
}
