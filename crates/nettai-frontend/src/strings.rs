//! The display text a frame shows for content, in the player's language
//! (`--lang`): that language's strings table (a content root's
//! `locales/<lang>.toml`, nettai-content `locale`) where it has the string,
//! else the content's own strings (`Content::strings`), else the
//! definition's key. Presentation only: the battle counts the content's
//! own strings (a description's lines, the run message's characters per
//! line), never a translation (docs/design/text-rendering.md §10).

use nettai_battle::Content;
use nettai_battle::content::strings::Strings;
use nettai_content_api::{ChipHandle, FormHandle, NaviHandle};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::BTreeSet;

/// A string, and whether it is a translation (another language's than the
/// content's own, whose shape the battle doesn't count).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Said<'b> {
    pub text: &'b str,
    pub translated: bool,
}

/// A frame's display text: a language's table, if not the content's own.
#[derive(Debug, Default)]
pub struct DisplayText<'a> {
    language: Option<&'a Strings>,
    /// What the frame showed that a table lacks (`chips.cannon.name`), for
    /// the audit.
    missing: RefCell<BTreeSet<String>>,
}

impl<'a> DisplayText<'a> {
    pub fn new(language: Option<&'a Strings>) -> DisplayText<'a> {
        DisplayText { language, missing: RefCell::default() }
    }

    /// The language's string, else the content's own (each missing one
    /// noted), else none.
    fn pick<'b>(&self, given: Option<&'b str>, own: Option<&'b str>, what: impl Fn() -> String) -> Option<Said<'b>> {
        if self.language.is_some() {
            if let Some(text) = given {
                return Some(Said { text, translated: true });
            }
            if own.is_some() {
                self.missing.borrow_mut().insert(what());
            }
        }
        own.map(|text| Said { text, translated: false })
    }

    /// A chip's name (its key when no table names it).
    pub fn chip_name<'b>(&self, content: &'b Content, chip: ChipHandle) -> &'b str
    where
        'a: 'b,
    {
        let key = &content.defs.chip(chip).key;
        let given = self.language.and_then(|s| s.chip(key)).and_then(|c| c.name.as_deref());
        let own = content.strings.chip(key).and_then(|c| c.name.as_deref());
        self.pick(given, own, || format!("chips.{key}.name")).map_or(key.as_str(), |s| s.text)
    }

    /// A chip's description, if it has one.
    pub fn chip_description<'b>(&self, content: &'b Content, chip: ChipHandle) -> Option<Said<'b>>
    where
        'a: 'b,
    {
        let key = &content.defs.chip(chip).key;
        let given = self.language.and_then(|s| s.chip(key)).and_then(|c| c.description.as_deref());
        let own = content.strings.chip(key).and_then(|c| c.description.as_deref());
        self.pick(given, own, || format!("chips.{key}.description"))
    }

    /// A form's (a Cross's) description, if it has one.
    pub fn form_description<'b>(&self, content: &'b Content, form: FormHandle) -> Option<Said<'b>>
    where
        'a: 'b,
    {
        let key = &content.defs.form(form).key;
        let given = self.language.and_then(|s| s.form(key)).and_then(|f| f.description.as_deref());
        let own = content.strings.form(key).and_then(|f| f.description.as_deref());
        self.pick(given, own, || format!("forms.{key}.description"))
    }

    /// A navi's name (the custom screen's enemy name; its key when no
    /// table names it).
    pub fn navi_name<'b>(&self, content: &'b Content, navi: NaviHandle) -> &'b str
    where
        'a: 'b,
    {
        let key = &content.defs.navi(navi).key;
        let given = self.language.and_then(|s| s.navi(key)).and_then(|n| n.name.as_deref());
        let own = content.strings.navi(key).and_then(|n| n.name.as_deref());
        self.pick(given, own, || format!("navis.{key}.name")).map_or(key.as_str(), |s| s.text)
    }

    /// A navi's no-running message, if it has one.
    pub fn run_message<'b>(&self, content: &'b Content, navi: NaviHandle) -> Option<Said<'b>>
    where
        'a: 'b,
    {
        let key = &content.defs.navi(navi).key;
        let given = self.language.and_then(|s| s.navi(key)).and_then(|n| n.run_message.as_deref());
        let own = content.strings.navi(key).and_then(|n| n.run_message.as_deref());
        self.pick(given, own, || format!("navis.{key}.run_message"))
    }

    /// What the frames showed that the language's table lacks, since the
    /// last call.
    pub fn take_missing(&self) -> Vec<String> {
        std::mem::take(&mut *self.missing.borrow_mut()).into_iter().collect()
    }

    /// The language's table's language, if one.
    pub fn language(&self) -> Option<&str> {
        self.language.map(|s| s.language.as_str())
    }
}

/// A chip's name in the content's own strings, else its key (for the
/// frontend's own text: status lines, folder listings), `spelled` for a
/// terminal.
pub fn own_chip_name(content: &Content, chip: ChipHandle) -> Cow<'_, str> {
    let key = &content.defs.chip(chip).key;
    spelled(content.strings.chip(key).and_then(|c| c.name.as_deref()).unwrap_or(key))
}

/// A string for a terminal: a stacked mark, a character of the Private Use
/// Area that a terminal has no glyph for, spelled as its letters (`Count`
/// and U+E002, the stacked EX, as `CountEX`).
pub fn spelled(text: &str) -> Cow<'_, str> {
    if !text.chars().any(|c| crate::vfont::stacked_letters(c).is_some()) {
        return Cow::Borrowed(text);
    }
    Cow::Owned(text.chars().flat_map(|c| crate::vfont::stacked_letters(c).map_or_else(|| vec![c], Vec::from)).collect())
}

/// A Cross's name in the content's own strings, else its key (for the
/// frontend's own text: live play's Crosses, the plain-text screen's Cross
/// window).
pub fn own_form_name(content: &Content, form: FormHandle) -> &str {
    let key = &content.defs.form(form).key;
    content.strings.form(key).and_then(|f| f.name.as_deref()).unwrap_or(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_terminal_gets_a_stacked_mark_spelled() {
        assert_eq!(spelled("Count\u{E002}"), "CountEX");
        assert_eq!(spelled("TmhkMan\u{E003}"), "TmhkManSP");
        assert!(matches!(spelled("Press Ⓐ"), Cow::Borrowed("Press Ⓐ")));
    }
}
