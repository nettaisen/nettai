//! The content's display text (its strings): chip names and descriptions,
//! navi names and no-running messages, the Crosses' names and descriptions,
//! patch cards' names, by the definitions' keys; the backgrounds' names, by
//! their asset names. A
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

/// A background's strings, by its asset name: its name as a player sees it
/// (a match's places: the app's arenas), the area of the game whose maps
/// draw it as the game's menus name it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackgroundStrings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// An entry's strings (of one of the game's collections: a patch card's, a
/// NaviCust program's): its name, which a menu shows, and its description
/// where the game has one (a NaviCust program's, as its screen's box shows
/// it: `\n` its line breaks).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryStrings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A table of the game's own, at the top level beside the core's sections:
/// a collection's entries' strings by id (`[patch_cards]`), or a text table
/// of key to text that no definition owns (`[patch_card_effects]`), which
/// its game's manifest declares (`text`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Table {
    Entries(BTreeMap<String, EntryStrings>),
    Text(BTreeMap<String, String>),
}

/// One language's strings, by definition key: the core's sections' (chips,
/// navis, forms), and the game's own tables by name: each of its
/// collections' (`[patch_cards]`), by id, and its text tables
/// (`[patch_card_effects]`), by key. (A table that is no collection of the
/// content and no text table its manifest declares is the locales' check's
/// to refuse.)
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
    /// The backgrounds', by their asset names (`lans-hp`).
    #[serde(default)]
    pub backgrounds: BTreeMap<String, BackgroundStrings>,
    /// The game's own tables, by name: its collections' entries'
    /// (`[patch_cards]`) and its text tables, which tools show and nothing
    /// in a battle reads (EXE6's and EXE5's `patch_card_effects`: each line a
    /// patch card's menu shows, by its effect's kind and choice, its numbers
    /// by the effect's fields, `{amount}`).
    #[serde(flatten)]
    pub tables: BTreeMap<String, Table>,
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
        self.backgrounds.extend(other.backgrounds);
        for (name, table) in other.tables {
            match (self.tables.get_mut(&name), table) {
                (Some(Table::Entries(have)), Table::Entries(more)) => have.extend(more),
                (Some(Table::Text(have)), Table::Text(more)) => have.extend(more),
                (_, table) => {
                    self.tables.insert(name, table);
                }
            }
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

    /// Background `name`'s (its asset name).
    pub fn background(&self, name: &str) -> Option<&BackgroundStrings> {
        self.backgrounds.get(name)
    }

    /// Entry `id` of collection `collection`'s strings.
    pub fn entry(&self, collection: &str, id: &str) -> Option<&EntryStrings> {
        match self.tables.get(collection)? {
            Table::Entries(entries) => entries.get(id),
            Table::Text(_) => None,
        }
    }

    /// Key `key` of the game's text table `table`.
    pub fn text(&self, table: &str, key: &str) -> Option<&str> {
        match self.tables.get(table)? {
            Table::Text(lines) => lines.get(key).map(String::as_str),
            Table::Entries(_) => None,
        }
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
