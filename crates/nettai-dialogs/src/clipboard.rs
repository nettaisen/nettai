//! Text copied to the system's clipboard (an invite's code, an address): the desktop's (arboard), the browser's
//! (`navigator.clipboard`), or a phone's. What goes wrong is said in `log`'s warning.

/// Copy `text`.
pub fn copy(text: &str) {
    crate::platform::system::copy(text);
}
