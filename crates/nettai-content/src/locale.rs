//! Display text in other languages: a content root's
//! `locale/<language>.toml` (docs/design/text-rendering.md §10).
//!
//! The definitions' own words (a chip's `name` and `description`, a navi's
//! `name` and `run_message`, a form's `description`) are the content's
//! canonical language, and the battle reads what it needs of them (a
//! description's lines, the run message's characters per line) from there.
//! A strings table gives a frontend the same words in another language, by
//! definition key. It is no module and no definition: the content root
//! reader leaves `locale/` out, so a table changes neither what the
//! battle does nor `Content::hash()`, and two players can each read their
//! own language in one netbattle.
//!
//! ```toml
//! language = "ja"
//!
//! [chips]
//! cannon = { name = "キャノン", description = "..." }
//!
//! [navis]
//! megaman = { name = "ロックマン", run_message = "..." }
//!
//! [forms]
//! heatcross = { description = "..." }
//! ```
//!
//! A line break in a description or a message is `\n`, as in the
//! definitions. A translated description may have another number of lines
//! than the definition's: the battle keeps the definition's timing.

use nettai_battle::content::Defs;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The folder of a content root that holds the tables.
pub const DIR: &str = "locale";

/// A chip's words.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChipStrings {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// A navi's words: its name (the enemy names on the round's first custom
/// screen) and its no-running message.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaviStrings {
    pub name: Option<String>,
    pub run_message: Option<String>,
}

/// A form's words: its description (R in the Cross window).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormStrings {
    pub description: Option<String>,
}

/// One language's strings table.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Strings {
    pub language: String,
    #[serde(default)]
    pub chips: BTreeMap<String, ChipStrings>,
    #[serde(default)]
    pub navis: BTreeMap<String, NaviStrings>,
    #[serde(default)]
    pub forms: BTreeMap<String, FormStrings>,
}

/// The table of `lang` in a content root.
pub fn path(root: &Path, lang: &str) -> PathBuf {
    root.join(DIR).join(format!("{lang}.toml"))
}

impl Strings {
    /// Parse a table (`file` names it in messages).
    pub fn parse(text: &str, file: &str) -> Result<Strings, String> {
        toml::from_str(text).map_err(|e| format!("{file}: {e}"))
    }

    /// The content root's table of `lang`; `None` when it has none.
    pub fn load(root: &Path, lang: &str) -> Result<Option<Strings>, String> {
        let path = path(root, lang);
        if !path.is_file() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
        let s = Strings::parse(&text, &path.display().to_string())?;
        if s.language != lang {
            return Err(format!("{} says it is {:?}", path.display(), s.language));
        }
        Ok(Some(s))
    }

    /// The languages a content root has tables of.
    pub fn languages(root: &Path) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(root.join(DIR)) else { return Vec::new() };
        let mut out: Vec<String> = entries
            .flatten()
            .filter_map(|e| e.path().file_name()?.to_str()?.strip_suffix(".toml").map(String::from))
            .collect();
        out.sort();
        out
    }

    pub fn chip(&self, key: &str) -> Option<&ChipStrings> {
        self.chips.get(key)
    }

    pub fn navi(&self, key: &str) -> Option<&NaviStrings> {
        self.navis.get(key)
    }

    pub fn form(&self, key: &str) -> Option<&FormStrings> {
        self.forms.get(key)
    }

    /// What is wrong with the table against the definitions: a key no
    /// definition has, a field the definition has no words for in its own
    /// language (a description of a chip without one), an empty name or
    /// message (a description may be empty: the Japanese games print none
    /// for some chips), a string not in Unicode's composed form (NFC: no
    /// combining marks after a letter they compose with).
    pub fn check(&self, defs: &Defs) -> Vec<String> {
        let mut out = Vec::new();
        let mut text = |what: String, s: &Option<String>| {
            if let Some(s) = s {
                if s.is_empty() && !what.ends_with(".description") {
                    out.push(format!("{what} is empty"));
                }
                if let Some(c) = s.chars().find(|&c| matches!(c, '\u{0300}'..='\u{036F}' | '\u{3099}' | '\u{309A}')) {
                    out.push(format!("{what} has a combining mark ({c:?}): write the composed character"));
                }
            }
        };
        let mut unknown = Vec::new();
        for (key, c) in &self.chips {
            match defs.chip_by_key(key) {
                None => unknown.push(format!("chips.{key}: no chip has this key")),
                Some(h) => {
                    if c.description.is_some() && defs.chip(h).record.description.is_none() {
                        unknown.push(format!("chips.{key}: the chip has no description to translate"));
                    }
                }
            }
            text(format!("chips.{key}.name"), &c.name);
            text(format!("chips.{key}.description"), &c.description);
        }
        for (key, n) in &self.navis {
            if defs.navi_by_key(key).is_none() {
                unknown.push(format!("navis.{key}: no navi has this key"));
            }
            text(format!("navis.{key}.name"), &n.name);
            text(format!("navis.{key}.run_message"), &n.run_message);
        }
        for (key, f) in &self.forms {
            match defs.form_by_key(key) {
                None => unknown.push(format!("forms.{key}: no form has this key")),
                Some(h) => {
                    if f.description.is_some() && defs.form(h).record.description.is_none() {
                        unknown.push(format!("forms.{key}: the form has no description to translate"));
                    }
                }
            }
            text(format!("forms.{key}.description"), &f.description);
        }
        unknown.extend(out);
        unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_table_parses_and_refuses_unknown_fields() {
        let s = Strings::parse(
            "language = \"ja\"\n[chips]\ncannon = { name = \"キャノン\", description = \"a\\nb\" }\n[navis]\nmegaman = { name = \"ロックマン\" }\n",
            "ja.toml",
        )
        .unwrap();
        assert_eq!(s.chip("cannon").and_then(|c| c.name.as_deref()), Some("キャノン"));
        assert_eq!(s.chip("cannon").and_then(|c| c.description.as_deref()), Some("a\nb"));
        assert_eq!(s.navi("megaman").and_then(|n| n.run_message.as_deref()), None);
        assert!(Strings::parse("language = \"ja\"\n[chips]\ncannon = { nmae = \"x\" }\n", "ja.toml").is_err());
    }
}
