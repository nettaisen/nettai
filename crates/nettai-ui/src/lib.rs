//! The nettai app's UI: what `slint-build` makes of `ui/app.slint` (the window, and the globals and structs the
//! app sets), and the languages bundled with it. The app is `crates/nettai`; docs/app.md says how the two fit.

slint::include_modules!();

include!(concat!(env!("OUT_DIR"), "/languages.rs"));
