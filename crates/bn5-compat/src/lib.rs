//! Compatibility with BN5's original games (Team ProtoMan and Team Colonel,
//! US and Japanese): their numbers for BN5's content (content/bn5/compat,
//! rules-in-luau.md §7.2: compat is per root), and what reads them.
//!
//! - [`Compat`]: the tables, content key to the original's numbers (chip
//!   ids, the chips' uses by number, damage formulas, the version
//!   differences) and BN5's panel types. [`Compat::bn5`] is this
//!   repository's, built in.
//! - [`codec`]: BN5's records in the engine's terms: the 0x60-byte
//!   NaviStats with BN5's light/dark value, the panels, the chip blocks.
//! - `trace` (feature `trace`): the chip lab's BN5 recordings, read and
//!   decoded.
//!
//! Keys: content/bn5 writes them unqualified, as its root's loader reads
//! them (rules-in-luau.md R: content/bn5/root.toml names the root `bn5`);
//! at its boundary this crate qualifies the keys it hands out (`bn5:cannon`,
//! [`qualify`]) and strips the ones it is handed ([`strip`]), as bn6-compat
//! does with `bn6`.
//!
//! The engine never reads any of it (a test guards it). BN5's content has
//! no root yet (rules-in-luau.md R): what needs one (handles, kinds, the
//! comparison with a running battle) waits for it, and docs/design/
//! bn5-map.md §13 lists what of BN5's records has no engine counterpart.

pub mod codec;
#[cfg(feature = "trace")]
pub mod trace;

use nettai_battle::field::PanelType;
use nettai_content_api::Pool;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// The root BN5's content and compat are (content/bn5/root.toml's `name`).
pub const ROOT: &str = "bn5";

/// A key of BN5's root, qualified as the loader qualifies it: `bn5:<key>`.
pub fn qualify(key: &str) -> String {
    format!("{ROOT}:{key}")
}

/// A qualified key's own key, if it is BN5's root's.
pub fn strip(key: &str) -> Option<&str> {
    key.strip_prefix(ROOT).and_then(|k| k.strip_prefix(':'))
}

/// BN5's object pools: how many slots each has (bn5-map.md §3.1). The
/// actors' is half BN6's (32); the engine's `object::SLOTS` is one number
/// for every pool, which BN5 needs per pool.
pub fn pool_slots(pool: Pool) -> usize {
    match pool {
        Pool::Actor => 16,
        Pool::Attack => 32,
        Pool::Effect => 32,
    }
}

/// The pool of an object type number as the traces print it (1, 3, 4).
pub fn pool_of_type(t: u8) -> Option<Pool> {
    match t {
        1 => Some(Pool::Actor),
        3 => Some(Pool::Attack),
        4 => Some(Pool::Effect),
        _ => None,
    }
}

/// What Team Colonel's record of a chip has where it differs from Team
/// ProtoMan's (compat's `colonel`): the flags (the version Gigas' library
/// bit) and the extra flags (the navi chips' +0x16).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VersionRecord {
    #[serde(default)]
    pub flags: Option<Vec<String>>,
    #[serde(default)]
    pub extra_flags: Option<Vec<u8>>,
}

/// A chip: its id, its use by number (the action and subtype its record
/// names), its damage formula's number (a damage of 1000 and up, less
/// 1000), and Team Colonel's differences.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChipEntry {
    pub id: u16,
    pub action: u8,
    pub subtype: u8,
    #[serde(default)]
    pub damage_formula: Option<u16>,
    #[serde(default)]
    pub colonel: Option<VersionRecord>,
    /// An e-Reader card's chip (LeadRaid, ChaosLrd): the save slot its
    /// name, description and picture palette are in (bn5-map.md §6.4).
    #[serde(default)]
    pub save_slot: Option<u8>,
}

/// A BN5 panel type: its name, the flag word the game gives it, and the
/// engine's panel type it is (none for BN5's metal and sea panels).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PanelEntry {
    pub name: String,
    pub flags: u32,
    #[serde(default)]
    pub engine: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PanelsFile {
    types: BTreeMap<String, PanelEntry>,
}

/// The original's numbers of BN5's rule definitions (rules.toml).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleNumbers {
    /// Statuses (0x0801CEC4): a hit's status byte, by key.
    #[serde(default)]
    pub statuses: BTreeMap<String, u8>,
}

/// BN5's asset names (assets.toml): the names bn5-extract writes its
/// assets under, by BN6's names for what is BN6's. Sprites as "cc-ii" (the
/// category's byte offset in the sprite list and the index), sounds by the
/// song table's numbers, banners by banner id.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetNames {
    #[serde(default)]
    pub sprites: BTreeMap<String, String>,
    #[serde(default)]
    pub sounds: BTreeMap<String, u16>,
    #[serde(default)]
    pub banners: BTreeMap<String, u8>,
}

/// BN5's compat tables (content/bn5/compat).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Compat {
    /// chips.toml: by key.
    pub chips: BTreeMap<String, ChipEntry>,
    /// panels.toml: by BN5's panel type number.
    pub panels: BTreeMap<u8, PanelEntry>,
    /// The chips' keys by id.
    pub chip_keys: BTreeMap<u16, String>,
    /// assets.toml: the assets' names.
    pub assets: AssetNames,
    /// rules.toml: the rule definitions' numbers.
    pub rules: RuleNumbers,
}

/// The files of a compat folder.
pub const FILES: [&str; 4] = ["chips.toml", "panels.toml", "assets.toml", "rules.toml"];

/// This repository's compat (content/bn5/compat), built in.
const BN5: [(&str, &str); 4] = [
    ("chips.toml", include_str!("../../../content/bn5/compat/chips.toml")),
    ("panels.toml", include_str!("../../../content/bn5/compat/panels.toml")),
    ("assets.toml", include_str!("../../../content/bn5/compat/assets.toml")),
    ("rules.toml", include_str!("../../../content/bn5/compat/rules.toml")),
];

/// The engine's panel type of an engine panel name as panels.toml writes
/// it.
fn engine_panel(name: &str) -> Option<PanelType> {
    Some(match name {
        "missing" => PanelType::Missing,
        "broken" => PanelType::Broken,
        "normal" => PanelType::Normal,
        "cracked" => PanelType::Cracked,
        "poison" => PanelType::Poison,
        "holy" => PanelType::Holy,
        "grass" => PanelType::Grass,
        "ice" => PanelType::Ice,
        "volcano" => PanelType::Volcano,
        _ => return None,
    })
}

/// A sprite's "cc-ii".
fn parse_sprite(id: &str) -> Option<(u8, u8)> {
    let (c, i) = id.split_once('-')?;
    Some((u8::from_str_radix(c, 16).ok()?, u8::from_str_radix(i, 16).ok()?))
}

impl Compat {
    /// BN5's compat, as this repository's content/bn5/compat has it.
    pub fn bn5() -> &'static Compat {
        static BN5_COMPAT: std::sync::OnceLock<Compat> = std::sync::OnceLock::new();
        BN5_COMPAT.get_or_init(|| {
            Compat::parse(|file| Ok(BN5.iter().find(|(f, _)| *f == file).map(|(_, t)| t.to_string()).unwrap_or_default()))
                .unwrap_or_else(|e| panic!("content/bn5/compat: {e}"))
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
        let panels: PanelsFile = toml::from_str(&text("panels.toml")?).map_err(|e| format!("panels.toml: {e}"))?;
        let mut chip_keys = BTreeMap::new();
        for (k, c) in &chips {
            if let Some(other) = chip_keys.insert(c.id, k.clone()) {
                return Err(format!("chips.toml: {k} and {other} are both chip {:#05x}", c.id));
            }
        }
        let mut by_number = BTreeMap::new();
        for (n, p) in panels.types {
            let n: u8 = n.parse().map_err(|_| format!("panels.toml: type {n:?} isn't a number"))?;
            if let Some(e) = &p.engine {
                engine_panel(e).ok_or_else(|| format!("panels.toml: type {n}'s engine panel {e:?} isn't the engine's"))?;
            }
            by_number.insert(n, p);
        }
        let assets: AssetNames = toml::from_str(&text("assets.toml")?).map_err(|e| format!("assets.toml: {e}"))?;
        for (name, id) in &assets.sprites {
            parse_sprite(id).ok_or_else(|| format!("assets.toml: sprite {name} is {id:?}, not \"cc-ii\""))?;
        }
        let rules: RuleNumbers = toml::from_str(&text("rules.toml")?).map_err(|e| format!("rules.toml: {e}"))?;
        let mut statuses = BTreeMap::new();
        for (k, &n) in &rules.statuses {
            if let Some(other) = statuses.insert(n, k) {
                return Err(format!("rules.toml: statuses {k} and {other} are both {n:#04x}"));
            }
        }
        Ok(Compat { chips, panels: by_number, chip_keys, assets, rules })
    }

    /// A chip's key by its id, as compat writes it (unqualified).
    pub fn chip_key(&self, id: u16) -> Option<&str> {
        self.chip_keys.get(&id).map(String::as_str)
    }

    /// A chip's qualified key (`bn5:cannon`) by its id.
    pub fn chip(&self, id: u16) -> Option<String> {
        self.chip_key(id).map(qualify)
    }

    /// A chip's entry by its qualified key.
    pub fn chip_entry(&self, key: &str) -> Option<&ChipEntry> {
        strip(key).and_then(|k| self.chips.get(k))
    }

    /// A status's qualified key (`bn5:paralyze-90`) by a hit's status byte.
    pub fn status(&self, byte: u8) -> Option<String> {
        self.rules.statuses.iter().find(|&(_, &n)| n == byte).map(|(k, _)| qualify(k))
    }

    /// The sprites' names by (category, index).
    pub fn sprite_names(&self) -> BTreeMap<(u8, u8), String> {
        self.assets.sprites.iter().filter_map(|(name, id)| Some((parse_sprite(id)?, name.clone()))).collect()
    }

    /// The engine's panel type of BN5's panel type `n`: `Ok(None)` for a
    /// type the engine has none of (metal, sea).
    pub fn panel_type(&self, n: u8) -> Result<Option<PanelType>, String> {
        let p = self.panels.get(&n).ok_or_else(|| format!("panels.toml has no type {n}"))?;
        Ok(p.engine.as_deref().and_then(engine_panel))
    }
}
