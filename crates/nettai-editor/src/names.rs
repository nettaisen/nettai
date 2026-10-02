//! Names in the editor's language: a language's table (a content root's
//! `locales/<lang>.toml`) where it has the name, else the content's own,
//! else the definition's key.

use nettai_battle::Content;
use nettai_battle::content::strings::Strings;
use nettai_content_api::{ChipHandle, FormHandle, NaviHandle, PatchCardHandle};

/// The languages the editor offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    Ja,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Ja => "ja",
        }
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        match code {
            "en" => Some(Lang::En),
            "ja" => Some(Lang::Ja),
            _ => None,
        }
    }
}

/// Display names: the content's own, and another language's table.
#[derive(Default)]
pub struct Names {
    pub other: Option<Strings>,
}

/// A name for one line: a two-line name joined, and the game's stacked
/// marks (U+E002 is EX, as the frontend's font draws it) spelled as their
/// letters, which the editor's font has.
fn line(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\n' => out.push(' '),
            '\u{E000}' => out.push_str("RV"),
            '\u{E001}' => out.push_str("BX"),
            '\u{E002}' => out.push_str("EX"),
            '\u{E003}' => out.push_str("SP"),
            '\u{E004}' => out.push_str("FZ"),
            '\u{E005}' => out.push_str("MB"),
            c => out.push(c),
        }
    }
    out
}

impl Names {
    fn pick(&self, content: &Content, key: &str, get: impl Fn(&Strings, &str) -> Option<String>) -> String {
        self.other
            .as_ref()
            .and_then(|s| get(s, key))
            .or_else(|| get(&content.strings, key))
            .map(|s| line(&s))
            .unwrap_or_else(|| key.to_string())
    }

    pub fn chip(&self, content: &Content, c: ChipHandle) -> String {
        self.pick(content, &content.defs.chip(c).key, |s, k| s.chip(k).and_then(|c| c.name.clone()))
    }

    pub fn form(&self, content: &Content, f: FormHandle) -> String {
        self.pick(content, &content.defs.form(f).key, |s, k| s.form(k).and_then(|c| c.name.clone()))
    }

    pub fn navi(&self, content: &Content, n: NaviHandle) -> String {
        self.pick(content, &content.defs.navi(n).key, |s, k| s.navi(k).and_then(|c| c.name.clone()))
    }

    pub fn navicust_program(&self, content: &Content, p: nettai_content_api::NaviCustProgramHandle) -> String {
        self.pick(content, &content.defs.navicust_program(p).key, |s, k| s.navicust_program(k).and_then(|c| c.name.clone()))
    }

    pub fn patch_card(&self, content: &Content, c: PatchCardHandle) -> String {
        self.pick(content, &content.defs.patch_card(c).key, |s, k| s.patch_card(k).and_then(|c| c.name.clone()))
    }
}
