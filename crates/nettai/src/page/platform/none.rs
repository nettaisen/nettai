//! Elsewhere (the desktop, a phone): the window is the system's, which the player sizes, and takes its own text.

pub fn follow(resized: impl FnMut(f32, f32) + 'static) {
    drop(resized);
}

pub mod text_entry {
    use crate::page::text_entry::{Entry, Place};

    pub fn open(place: Place, text: &str, select_all: bool, heard: impl FnMut(Entry) + 'static) {
        let _ = (place, text, select_all, heard);
    }

    pub fn keep(text: &str) {
        let _ = text;
    }

    pub fn place(place: Place) {
        let _ = place;
    }

    pub fn close() {}

    pub fn is_open() -> bool {
        false
    }
}
