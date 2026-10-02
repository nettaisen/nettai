//! Names for messages: a definition's name in the content's own strings
//! (its locales' own language), else its key. A frontend shows its player's
//! language from the locales itself.

use nettai_battle::Content;
use nettai_content_api::{ChipHandle, FormHandle, NaviHandle, PatchCardHandle};

/// A chip's name, else its key.
pub fn chip(content: &Content, c: ChipHandle) -> &str {
    let key = &content.defs.chip(c).key;
    content.strings.chip(key).and_then(|s| s.name.as_deref()).unwrap_or(key)
}

/// A form's (a Cross's) name, else its key.
pub fn form(content: &Content, f: FormHandle) -> &str {
    let key = &content.defs.form(f).key;
    content.strings.form(key).and_then(|s| s.name.as_deref()).unwrap_or(key)
}

/// A navi's name, else its key.
pub fn navi(content: &Content, n: NaviHandle) -> &str {
    let key = &content.defs.navi(n).key;
    content.strings.navi(key).and_then(|s| s.name.as_deref()).unwrap_or(key)
}

/// A patch card's name (its first line, for a name the locales break in
/// two), else its key.
pub fn patch_card(content: &Content, c: PatchCardHandle) -> String {
    let key = &content.defs.patch_card(c).key;
    content.strings.patch_card(key).and_then(|s| s.name.as_deref()).map_or_else(|| key.clone(), |n| n.replace('\n', " "))
}
