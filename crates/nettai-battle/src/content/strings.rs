//! The content's display text (its strings): chip names and descriptions,
//! navi names and no-running messages, form names and descriptions, weapon
//! names, records' names (BN6's patch cards), by the definitions' keys. A definition holds no display text; a content root's
//! `locales/<lang>.toml` does, one table a language (the loader, nettai-
//! content `locale`, reads them).
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

/// A form's strings: its name, and a Cross's description (R in the Cross
/// window).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormStrings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A weapon's strings: its name.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponStrings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// A record's strings: its name (a patch card's, which its menu shows).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordStrings {
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
    #[serde(default)]
    pub weapons: BTreeMap<String, WeaponStrings>,
    #[serde(default)]
    pub records: BTreeMap<String, RecordStrings>,
}

/// Presentation: the records hold what the battle reads of the strings (the
/// define phase counts them), and the hash covers those.
impl std::hash::Hash for Strings {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

impl Strings {
    pub fn chip(&self, key: &str) -> Option<&ChipStrings> {
        self.chips.get(key)
    }

    pub fn navi(&self, key: &str) -> Option<&NaviStrings> {
        self.navis.get(key)
    }

    pub fn form(&self, key: &str) -> Option<&FormStrings> {
        self.forms.get(key)
    }

    pub fn weapon(&self, key: &str) -> Option<&WeaponStrings> {
        self.weapons.get(key)
    }

    pub fn record(&self, key: &str) -> Option<&RecordStrings> {
        self.records.get(key)
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

/// A line's glyphs: its characters, a bracketed name (`[B]`) as one.
pub fn glyphs(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(c) = rest.chars().next() {
        let n = match rest.find(']') {
            Some(end) if c == '[' => end + 1,
            _ => c.len_utf8(),
        };
        out.push(&rest[..n]);
        rest = &rest[n..];
    }
    out
}

/// A no-running message's characters by line (up to three), which time
/// its printing.
pub fn message_counts(message: &str) -> Vec<u8> {
    message.split('\n').take(3).map(|l| glyphs(l).len().min(0xFF) as u8).collect()
}

/// The characters that move the speaker's mouth, by line: bit k for the
/// line's character k (`chatbox_8040C44`: the letters and digits, and four
/// kana the Japanese charmap had beside them; a space, punctuation and the
/// bracketed glyphs don't). A bracketed glyph name (`[B]`) is one
/// character.
pub fn talking(message: &str) -> [u32; 3] {
    let mut out = [0; 3];
    for (line, words) in message.split('\n').take(3).enumerate() {
        for (k, glyph) in glyphs(words).into_iter().take(32).enumerate() {
            let mut chars = glyph.chars();
            let talks = match (chars.next(), chars.next()) {
                (Some(c), None) => c.is_ascii_alphanumeric() || "ネノヌナ".contains(c),
                _ => false,
            };
            if talks {
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
        assert_eq!(message_counts("Press [B]!"), [8]);
        assert_eq!(talking("ab c\n[B]1")[..2], [0b1011, 0b10]);
    }
}
