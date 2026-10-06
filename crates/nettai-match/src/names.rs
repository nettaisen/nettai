//! Names for messages: a definition's name in the content's own strings
//! (its locales' own language), else its key. A frontend shows its player's
//! language from the locales itself.

use nettai_battle::Content;
use nettai_content_api::{ChipHandle, EntryHandle, FormHandle, NaviHandle};

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

/// An entry's name (of one of the game's collections: a patch card's, a
/// NaviCust program's; a name the locales break in two, on one line), else
/// its id.
pub fn entry(content: &Content, e: EntryHandle) -> String {
    let d = content.defs.entry(e);
    content.strings.entry(&d.collection, d.id()).and_then(|s| s.name.as_deref()).map_or_else(|| d.id().to_string(), |n| n.replace('\n', " "))
}
