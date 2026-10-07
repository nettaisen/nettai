//! Compatibility with EXE4's original games (Red Sun and Blue Moon, US and
//! Japanese): their numbers for EXE4's content (content/exe4/compat:
//! compat is per game), and what reads them.
//!
//! - [`Compat`]: the tables, content key to the original's numbers: the
//!   chips' ids, the names the extractor writes EXE4's assets under, the
//!   text encodings. [`Compat::exe4`] is this repository's, built in.
//! - [`save`]: an EXE4 save file, and what a player's setup takes of it.
//!
//! The codecs of EXE4's records, the recordings' decode and the save import
//! come with the port (docs/design/exe4-map.md §13). The verification
//! workspace's tools/exe4/gen_content.py writes the tables from the ROMs.
//!
//! Keys: the tables are keyed by EXE4's ids, local to the game (`cannon`),
//! as content writes them (docs/design/content-model-v2.md §4.0). The engine
//! never reads any of it (a test guards it).

pub mod save;

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// EXE4's game (content/exe4).
pub const ROOT: &str = "exe4";

/// An EXE4 game's version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Version {
    RedSun,
    BlueMoon,
}

impl Version {
    /// The version's name as compat and a pack write it (`redsun`,
    /// `bluemoon`).
    pub fn name(self) -> &'static str {
        match self {
            Version::RedSun => "redsun",
            Version::BlueMoon => "bluemoon",
        }
    }
}

/// A chip (chips.toml): its id (its record's number in the chip table), and
/// for a version's own giga chip its version (the other version's ROM has
/// its record, but no folder of that version holds it).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChipEntry {
    pub id: u16,
    #[serde(default)]
    pub version: Option<Version>,
}

/// EXE4's asset names (assets.toml): the names the extractor writes its
/// assets under. Sprites as "cc-ii" (the category's byte offset in the
/// sprite list and the index), sounds by the song table's numbers.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetNames {
    #[serde(default)]
    pub sprites: BTreeMap<String, String>,
    #[serde(default)]
    pub sounds: BTreeMap<String, u16>,
}

/// EXE4's text encodings (text.toml): what each byte below `first_control`
/// draws (the US ROMs' fonts draw 0x00 to 0x6F), and the Japanese ROMs'
/// (`jp`): its glyphs below `first_control`, then the second page's
/// (`dialogue_glyphs`, E4 xx: glyph 0xE4 + xx).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Text {
    pub first_control: u8,
    pub glyphs: Vec<String>,
    #[serde(default)]
    pub dialogue_glyphs: Vec<String>,
    #[serde(default)]
    pub jp: Encoding,
}

/// The Japanese ROMs' encoding (`Text::jp`), in the same shape.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Encoding {
    pub glyphs: Vec<String>,
    #[serde(default)]
    pub dialogue_glyphs: Vec<String>,
}

/// EXE4's compat tables (content/exe4/compat).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Compat {
    /// chips.toml: by key.
    pub chips: BTreeMap<String, ChipEntry>,
    /// The chips' keys by id.
    pub chip_keys: BTreeMap<u16, String>,
    /// assets.toml: the assets' names.
    pub assets: AssetNames,
    /// text.toml: the text encodings.
    pub text: Text,
}

/// The files of a compat folder.
pub const FILES: [&str; 3] = ["chips.toml", "assets.toml", "text.toml"];

/// This repository's compat (content/exe4/compat), built in.
const EXE4: [(&str, &str); 3] = [
    ("chips.toml", include_str!("../../../content/exe4/compat/chips.toml")),
    ("assets.toml", include_str!("../../../content/exe4/compat/assets.toml")),
    ("text.toml", include_str!("../../../content/exe4/compat/text.toml")),
];

/// A sprite's "cc-ii".
fn parse_sprite(id: &str) -> Option<(u8, u8)> {
    let (c, i) = id.split_once('-')?;
    Some((u8::from_str_radix(c, 16).ok()?, u8::from_str_radix(i, 16).ok()?))
}

impl Compat {
    /// EXE4's compat, as this repository's content/exe4/compat has it.
    pub fn exe4() -> &'static Compat {
        static EXE4_COMPAT: std::sync::OnceLock<Compat> = std::sync::OnceLock::new();
        EXE4_COMPAT.get_or_init(|| {
            Compat::parse(|file| Ok(EXE4.iter().find(|(f, _)| *f == file).map(|(_, t)| t.to_string()).unwrap_or_default()))
                .unwrap_or_else(|e| panic!("content/exe4/compat: {e}"))
        })
    }

    /// Read a compat folder.
    pub fn read(dir: &Path) -> Result<Compat, String> {
        Compat::parse(|file| {
            let path = dir.join(file);
            std::fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))
        })
    }

    fn parse(text: impl Fn(&str) -> Result<String, String>) -> Result<Compat, String> {
        let chips: BTreeMap<String, ChipEntry> = toml::from_str(&text("chips.toml")?).map_err(|e| format!("chips.toml: {e}"))?;
        let mut chip_keys = BTreeMap::new();
        for (k, c) in &chips {
            if let Some(other) = chip_keys.insert(c.id, k.clone()) {
                return Err(format!("chips.toml: {k} and {other} are both chip {:#05x}", c.id));
            }
        }
        let assets: AssetNames = toml::from_str(&text("assets.toml")?).map_err(|e| format!("assets.toml: {e}"))?;
        for (name, id) in &assets.sprites {
            parse_sprite(id).ok_or_else(|| format!("assets.toml: sprite {name} is {id:?}, not \"cc-ii\""))?;
        }
        let mut sounds = BTreeMap::new();
        for (name, &id) in &assets.sounds {
            if let Some(other) = sounds.insert(id, name) {
                return Err(format!("assets.toml: sounds {name} and {other} are both {id:#05x}"));
            }
        }
        let text: Text = toml::from_str(&text("text.toml")?).map_err(|e| format!("text.toml: {e}"))?;
        if text.glyphs.len() > text.first_control as usize || text.jp.glyphs.len() > text.first_control as usize {
            return Err(format!("text.toml: more glyphs than the bytes below first_control ({:#04x})", text.first_control));
        }
        Ok(Compat { chips, chip_keys, assets, text })
    }

    /// A chip's id (`cannon`) by its number.
    pub fn chip_key(&self, id: u16) -> Option<&str> {
        self.chip_keys.get(&id).map(String::as_str)
    }

    /// A chip's entry by its id.
    pub fn chip_entry(&self, key: &str) -> Option<&ChipEntry> {
        self.chips.get(key)
    }

    /// The sprites' names by (category, index).
    pub fn sprite_names(&self) -> BTreeMap<(u8, u8), String> {
        self.assets.sprites.iter().filter_map(|(name, id)| Some((parse_sprite(id)?, name.clone()))).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_compat_reads() {
        let c = Compat::exe4();
        assert_eq!(c.chip_key(1), Some("cannon"));
        assert_eq!(c.chip_entry("bluemoon").map(|e| (e.id, e.version)), Some((0x135, Some(Version::BlueMoon))));
        assert_eq!(c.sprite_names().get(&(0, 0)).map(String::as_str), Some("megaman"));
        // The US fonts draw 0x00 to 0x6F; the Japanese encoding's second page
        // runs from 0xE4.
        assert_eq!((c.text.first_control, c.text.glyphs.len()), (0xE4, 0x70));
        assert_eq!(c.text.jp.glyphs.len(), 0xE4);
        assert!(!c.text.jp.dialogue_glyphs.is_empty());
    }

    #[test]
    fn a_compat_folder_reads_from_disk_as_built_in() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/exe4/compat");
        assert_eq!(&Compat::read(&dir).unwrap(), Compat::exe4());
    }
}
