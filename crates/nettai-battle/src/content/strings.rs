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

/// A navi's strings: its name (the custom screen's enemy name) and its
/// no-running message (L), its lines apart by `\n`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaviStrings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
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

/// A patch card's strings: its name, which its menu shows.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatchCardStrings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// One language's strings, by definition key.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Strings {
    /// Its language (`en`, `ja`).
    pub language: String,
    #[serde(default)]
    pub chips: BTreeMap<String, ChipStrings>,
    #[serde(default)]
    pub navis: BTreeMap<String, NaviStrings>,
    #[serde(default)]
    pub forms: BTreeMap<String, FormStrings>,
    #[serde(default, rename = "patch-cards")]
    pub patch_cards: BTreeMap<String, PatchCardStrings>,
}

/// Presentation: the records hold what the battle reads of the strings (the
/// define phase counts them), and the hash covers those.
impl std::hash::Hash for Strings {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

impl Strings {
    /// Game `root`'s strings (its definitions', ids in full), from loaded
    /// content's.
    pub fn of_root(&self, root: &str) -> Strings {
        use nettai_content_api::keys::root_of;
        fn of<V: Clone>(root: &str, m: &BTreeMap<String, V>) -> BTreeMap<String, V> {
            m.iter().filter(|(k, _)| root_of(k) == Some(root)).map(|(k, v)| (k.clone(), v.clone())).collect()
        }
        Strings {
            language: self.language.clone(),
            chips: of(root, &self.chips),
            navis: of(root, &self.navis),
            forms: of(root, &self.forms),
            patch_cards: of(root, &self.patch_cards),
        }
    }

    /// Add another root's table (qualified: no key meets another root's).
    pub fn merge(&mut self, other: Strings) {
        if self.language.is_empty() {
            self.language = other.language;
        }
        self.chips.extend(other.chips);
        self.navis.extend(other.navis);
        self.forms.extend(other.forms);
        self.patch_cards.extend(other.patch_cards);
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

    pub fn patch_card(&self, key: &str) -> Option<&PatchCardStrings> {
        self.patch_cards.get(key)
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
/// line's character k (`chatbox_8040C44`: the letters and digits, and four
/// kana the Japanese charmap had beside them; a space, punctuation and the
/// marks don't).
pub fn talking(message: &str) -> [u32; 3] {
    let mut out = [0; 3];
    for (line, words) in message.split('\n').take(3).enumerate() {
        for (k, c) in words.chars().take(32).enumerate() {
            if c.is_ascii_alphanumeric() || "ネノヌナ".contains(c) {
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
        assert_eq!(talking("ab c\nⒷ1")[..2], [0b1011, 0b10]);
    }
}
