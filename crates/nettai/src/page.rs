//! The page the app is in, in a browser: what it calls (the app started, the files it keeps: `entry.rs`, in the web
//! module); its window is the page's whole, and follows it as it is resized ([`follow`]); and a field's text typed,
//! through an HTML input over the field ([`text_entry`]). (Elsewhere the window is the system's, which the player
//! sizes, and takes its own text: nothing here does anything there.)

mod platform;
pub mod text_entry;

/// `resized` hears the page's size in its CSS pixels (the window's logical ones) now, and each time it changes.
pub fn follow(resized: impl FnMut(f32, f32) + 'static) {
    platform::system::follow(resized);
}
