//! The nettai app's program (the app is the library: `nettai::run`).

// A release build on Windows opens no console window beside its own (what it prints goes nowhere; the log is the
// data folder's nettai.log). A development build keeps one, for what it prints.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> Result<(), slint::PlatformError> {
    nettai::run()
}
