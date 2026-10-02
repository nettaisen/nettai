//! The words a frame shows for content, in the player's language
//! (`--lang`): a strings table's (a content root's `locale/<lang>.toml`,
//! nettai-content `locale`) where it has them, else the definitions' own.
//! Presentation only: the battle reads the definitions' words (a
//! description's lines, the run message's characters per line) and nothing
//! here (docs/design/text-rendering.md §10).

use nettai_battle::Battle;
use nettai_content::locale::Strings;
use nettai_content_api::{ChipHandle, FormHandle, NaviHandle};
use std::cell::RefCell;
use std::collections::BTreeSet;

/// Words, and whether they are a table's (a translation) rather than the
/// definition's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Said<'b> {
    pub text: &'b str,
    pub translated: bool,
}

/// A frame's words: a language's table, if one.
#[derive(Debug, Default)]
pub struct Words<'a> {
    strings: Option<&'a Strings>,
    /// What the frame showed that the table lacks (`chips.cannon.name`),
    /// for the audit.
    missing: RefCell<BTreeSet<String>>,
}

impl<'a> Words<'a> {
    pub fn new(strings: Option<&'a Strings>) -> Words<'a> {
        Words { strings, missing: RefCell::default() }
    }

    /// The table's word, else the definition's (noted as missing).
    fn pick<'b>(&self, own: &'b str, given: Option<&'b str>, what: impl FnOnce() -> String) -> Said<'b> {
        match (self.strings, given) {
            (Some(_), Some(text)) => Said { text, translated: true },
            (Some(_), None) => {
                self.missing.borrow_mut().insert(what());
                Said { text: own, translated: false }
            }
            (None, _) => Said { text: own, translated: false },
        }
    }

    /// A chip's name.
    pub fn chip_name<'b>(&self, b: &'b Battle, chip: ChipHandle) -> &'b str
    where
        'a: 'b,
    {
        let key = &b.content.defs.chip(chip).key;
        let given = self.strings.and_then(|s| s.chip(key)).and_then(|c| c.name.as_deref());
        self.pick(&b.content.chip(chip).name, given, || format!("chips.{key}.name")).text
    }

    /// A chip's description, if it has one.
    pub fn chip_description<'b>(&self, b: &'b Battle, chip: ChipHandle) -> Option<Said<'b>>
    where
        'a: 'b,
    {
        let own = b.content.chip(chip).description.as_deref()?;
        let key = &b.content.defs.chip(chip).key;
        let given = self.strings.and_then(|s| s.chip(key)).and_then(|c| c.description.as_deref());
        Some(self.pick(own, given, || format!("chips.{key}.description")))
    }

    /// A form's (a Cross's) description, if it has one.
    pub fn form_description<'b>(&self, b: &'b Battle, form: FormHandle) -> Option<Said<'b>>
    where
        'a: 'b,
    {
        let own = b.content.form(form).description.as_deref()?;
        let key = &b.content.defs.form(form).key;
        let given = self.strings.and_then(|s| s.form(key)).and_then(|f| f.description.as_deref());
        Some(self.pick(own, given, || format!("forms.{key}.description")))
    }

    /// A navi's name (the custom screen's enemy name).
    pub fn navi_name<'b>(&self, b: &'b Battle, navi: NaviHandle) -> &'b str
    where
        'a: 'b,
    {
        let key = &b.content.defs.navi(navi).key;
        let given = self.strings.and_then(|s| s.navi(key)).and_then(|n| n.name.as_deref());
        self.pick(&b.content.navi(navi).name, given, || format!("navis.{key}.name")).text
    }

    /// A navi's no-running message.
    pub fn run_message<'b>(&self, b: &'b Battle, navi: NaviHandle) -> Said<'b>
    where
        'a: 'b,
    {
        let key = &b.content.defs.navi(navi).key;
        let given = self.strings.and_then(|s| s.navi(key)).and_then(|n| n.run_message.as_deref());
        self.pick(&b.content.navi(navi).run_message.text, given, || format!("navis.{key}.run_message"))
    }

    /// What the frames showed that the table lacks, since the last call.
    pub fn take_missing(&self) -> Vec<String> {
        std::mem::take(&mut *self.missing.borrow_mut()).into_iter().collect()
    }

    /// The table's language, if one.
    pub fn language(&self) -> Option<&str> {
        self.strings.map(|s| s.language.as_str())
    }
}
