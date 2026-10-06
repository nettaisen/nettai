//! The content's display text (its strings): chip names and descriptions,
//! navi names and no-running messages, the Crosses' names and descriptions,
//! patch cards' names, by the definitions' keys. A
//! definition holds no display text; a content root's
//! `locales/<lang>.toml` does, one table a language (the loader, nettai-
//! content `locale`, reads them). The game's marks are characters: Ⓐ and
//! Ⓑ for its buttons, the Private Use Area's for the glyphs Unicode has
//! none for (docs/design/text-rendering.md §10.5); a mark is one character
//! as the battle counts them.
//!
//! The battle reads one thing of the content's own language's strings
//! (`Content::strings`): their shape. A description's lines and a no-running
//! message's characters per line time the custom screen's chatbox (docs/
//! engine/custom-screen.md §3.5), and the message's characters say which
//! move the speaker's mouth. The define phase counts them into the records
//! (`ChipData::description_lines`, `FormData::description_lines`,
//! `RunMessage::counts` and `talking`), which the content's hash covers; the
//! strings themselves are presentation, left out of it. Another language's
//! table is a frontend's alone: it changes neither the battle nor the hash,
//! so two players of different languages play one netbattle.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A chip's strings.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChipStrings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// What R shows on the custom screen, its lines apart by `\n`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A navi's strings: its name (the custom screen's enemy name), the name
/// the enemy names show instead when its side's rules ask
/// (`battle.set_name_variant`: EXE5's Hub Style, "BCMegaMn"), and its
/// no-running message (L), its lines apart by `\n`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaviStrings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_message: Option<String>,
}

/// A form's strings (a Cross's): its name (a frontend's own text: live
/// play's Crosses, the plain-text screen's Cross window), and its
/// description (R in the Cross window).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormStrings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// An entry's strings (of one of the game's collections: a patch card's, a
/// NaviCust program's): its name, which a menu shows.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryStrings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// One language's strings, by definition key: the core's sections' (chips,
/// navis, forms), and each of the game's collections' under its name
/// (`[patch_cards]`), by id. (A table that is no collection of the content
/// is the locales' check's to refuse.)
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Strings {
    /// Its language (`en`, `ja`).
    pub language: String,
    #[serde(default)]
    pub chips: BTreeMap<String, ChipStrings>,
    #[serde(default)]
    pub navis: BTreeMap<String, NaviStrings>,
    #[serde(default)]
    pub forms: BTreeMap<String, FormStrings>,
    /// The game's collections' entries', by collection, then id.
    #[serde(flatten)]
    pub collections: BTreeMap<String, BTreeMap<String, EntryStrings>>,
}

/// Presentation: the records hold what the battle reads of the strings (the
/// define phase counts them), and the hash covers those.
impl std::hash::Hash for Strings {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

impl Strings {
    /// Add another root's table (qualified: no key meets another root's).
    pub fn merge(&mut self, other: Strings) {
        if self.language.is_empty() {
            self.language = other.language;
        }
        self.chips.extend(other.chips);
        self.navis.extend(other.navis);
        self.forms.extend(other.forms);
        for (c, entries) in other.collections {
            self.collections.entry(c).or_default().extend(entries);
        }
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

    /// Entry `id` of collection `collection`'s strings.
    pub fn entry(&self, collection: &str, id: &str) -> Option<&EntryStrings> {
        self.collections.get(collection)?.get(id)
    }
}

/// The lines of a description (`\n` apart; one to three, as the box
/// holds); none counts as three, what nearly every chip has.
pub fn description_lines(description: Option<&str>) -> u8 {
    description.map_or(3, |d| d.split('\n').count().clamp(1, 3) as u8)
}

/// A description's lines where none is known: three.
pub fn three_lines() -> u8 {
    3
}

/// A no-running message's characters by line (up to three), which time
/// its printing. A mark (Ⓑ) is one character, one glyph of the game's.
pub fn message_counts(message: &str) -> Vec<u8> {
    message.split('\n').take(3).map(|l| l.chars().count().min(0xFF) as u8).collect()
}

/// The characters that move the speaker's mouth, by line: bit k for the
/// line's character k, by the game's rule `custom_screen.talking_characters`
/// (EXE6's `chatbox_8040C44`: the letters and digits, and four kana its
/// charmap has in the tested range; EXE5's: all but a space, a range of
/// punctuation and marks, and the ellipsis).
pub fn talking(message: &str, rule: &super::custom::TalkingCharacters) -> [u32; 3] {
    let mut out = [0; 3];
    for (line, words) in message.split('\n').take(3).enumerate() {
        for (k, c) in words.chars().take(32).enumerate() {
            if rule.talks(c) {
                out[line] |= 1 << k;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_strings_shape_the_timing() {
        assert_eq!(description_lines(None), 3);
        assert_eq!(description_lines(Some("Bounce \nthe puck")), 2);
        assert_eq!(message_counts("Lan,this is no time\nto run away!"), [19, 12]);
        // A mark is one character, and no letter: it doesn't talk.
        assert_eq!(message_counts("Press Ⓑ!"), [8]);
        assert_eq!(message_counts("Count\u{E002}"), [6]);
        let only = |s: &str| crate::content::custom::TalkingCharacters { only: Some(s.into()), all_but: None };
        let all_but = |s: &str| crate::content::custom::TalkingCharacters { only: None, all_but: Some(s.into()) };
        assert_eq!(talking("ab c\nⒷ1", &only("abc1"))[..2], [0b1011, 0b10]);
        // (Every character but those listed: a mark talks unless it is.)
        assert_eq!(talking("ab c\nⒷ1", &all_but(" "))[..2], [0b1011, 0b11]);
        assert_eq!(talking("*a!*", &all_but(" !"))[0], 0b1011);
        // (No rule stated: none talks.)
        assert_eq!(talking("ab c", &Default::default()), [0; 3]);
    }
}
