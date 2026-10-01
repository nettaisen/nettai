//! Compatibility with the original game: its numbers for BN6's content
//! (content/bn6/compat, docs/design/content-model-v2.md §6), and what reads
//! them.
//!
//! - [`Compat`]: the tables, content key to the original's numbers (chip
//!   ids, action numbers, object slots, weapon routines, NameIDs, asset
//!   numbers...). [`Compat::bn6`] is this repository's, built in.
//! - [`codec`]: the game's setup records (navi stats, battle folders, chip
//!   hands, transformation requests, battle settings, the set's stages, SP
//!   deletion times) to the engine's types and back, for traces, real saves
//!   and link data.
//! - `trace` (feature `trace`): golden traces recorded from the original,
//!   replayed through the engine and compared with it.
//!
//! The engine never reads any of it: this crate depends on `bn6-battle`,
//! never the other way (a test guards it), and content (Luau) can't load
//! compat's TOML.

pub mod codec;
#[cfg(feature = "trace")]
pub mod trace;

use bn6_battle::Battle;
use bn6_battle::kinds::player::{NaviAction, navi_action};
use bn6_battle::object::{ObjectRef, Pool};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// A chip: its id, and the action and subtype its record names (the
/// latter two documentation).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChipEntry {
    pub id: u16,
    pub action: u8,
    pub subtype: u8,
}

/// A navi: its number (NaviStats+0x29) and NameID.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaviEntry {
    pub navi: u8,
    pub name_id: u16,
}

/// One of MegaMan's forms: its number (NaviStats+0x2C) and NameID (the
/// base form has none of its own: MegaMan's).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormEntry {
    pub form: u8,
    #[serde(default)]
    pub name_id: Option<u16>,
}

/// An object kind: the object slot it fills (what the traces compare),
/// and the positions the trace comparison skips.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KindEntry {
    /// "actor", "attack" or "effect".
    pub pool: String,
    pub index: u8,
    /// Its position is register garbage until its init places it.
    #[serde(default)]
    pub scratch_position: bool,
    /// Its position is garbage while it has no sprite (header flag
    /// NO_SPRITE_UPDATE): the charge glow before its first update.
    #[serde(default)]
    pub scratch_position_without_sprite: bool,
    /// The fraction of its Z is register garbage.
    #[serde(default)]
    pub scratch_z_fraction: bool,
    /// The actor-list entry type that places it (`off_80073A0`).
    #[serde(default)]
    pub actor_list_entry: Option<u8>,
}

/// A stage: the battle settings records (`BattleSettingsList1`) it is, its
/// panel layout's number and the address its actor list goes by.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageEntry {
    pub settings: Vec<u8>,
    pub layout: u8,
    pub actor_list: u32,
}

/// Records a setup names by byte.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Records {
    /// The save's SP navi deletion-time slots (`byte_203EB00`), by index.
    #[serde(default)]
    pub sp_slots: BTreeMap<String, u8>,
    /// The rocks a stage places, by the argument of the actor list's
    /// entry (`byte_80CF934`'s row).
    #[serde(default)]
    pub rock_variants: BTreeMap<String, u8>,
    /// The projectile's variants a navi's stats name as a shot program
    /// (NaviStats+0x4D, +0x4F), by the row of `off_80C4C78`.
    #[serde(default)]
    pub projectile_variants: BTreeMap<String, u8>,
}

/// The original's numbers of rule definitions (rules.toml): only
/// `gen-content check` reads them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleNumbers {
    /// Beast Out lock-on modes (`jt_8026584`), by key.
    #[serde(default)]
    pub lockon: BTreeMap<String, u8>,
    /// Statuses (`off_80209EC`): a hit's status byte, by key.
    #[serde(default)]
    pub statuses: BTreeMap<String, u8>,
    /// The field objects' identities: the NameID of each, by key (a
    /// navi's and a form's is in navis.toml and forms.toml).
    #[serde(default)]
    pub identities: BTreeMap<String, u16>,
}

/// Asset names and the ROM's numbers.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assets {
    /// "cc-ii": the sprite category's byte offset and the index, in hex.
    #[serde(default)]
    pub sprites: BTreeMap<String, String>,
    /// Song-table entries (music and sound effects share it).
    #[serde(default)]
    pub sounds: BTreeMap<String, u16>,
    #[serde(default)]
    pub backgrounds: BTreeMap<String, u8>,
    /// Banner ids (a multiple of 4).
    #[serde(default)]
    pub banners: BTreeMap<String, u8>,
    #[serde(default)]
    pub mugshots: BTreeMap<String, u8>,
}

/// The game's text encoding: what each byte below `first_control` draws.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Text {
    pub glyphs: Vec<String>,
    pub first_control: u8,
}

/// content/bn6/compat: the original's numbers by content key. Every map
/// is key to numbers; many-to-one maps are allowed (weapon aliases, the
/// chips of one action handler).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Compat {
    pub chips: BTreeMap<String, ChipEntry>,
    pub actions: BTreeMap<String, u8>,
    pub navis: BTreeMap<String, NaviEntry>,
    pub forms: BTreeMap<String, FormEntry>,
    pub weapons: BTreeMap<String, Vec<u8>>,
    pub kinds: BTreeMap<String, KindEntry>,
    pub stages: BTreeMap<String, StageEntry>,
    pub records: Records,
    pub rules: RuleNumbers,
    pub assets: Assets,
    pub text: Text,
    /// The kinds by the slot they fill.
    slots: BTreeMap<(Pool, u8), String>,
}

/// The files, in the order they are read.
pub const FILES: [&str; 11] = [
    "chips.toml",
    "actions.toml",
    "navis.toml",
    "forms.toml",
    "weapons.toml",
    "kinds.toml",
    "stages.toml",
    "records.toml",
    "rules.toml",
    "assets.toml",
    "text.toml",
];

/// This repository's compat (content/bn6/compat), built in.
const BN6: [(&str, &str); 11] = [
    ("chips.toml", include_str!("../../../content/bn6/compat/chips.toml")),
    ("actions.toml", include_str!("../../../content/bn6/compat/actions.toml")),
    ("navis.toml", include_str!("../../../content/bn6/compat/navis.toml")),
    ("forms.toml", include_str!("../../../content/bn6/compat/forms.toml")),
    ("weapons.toml", include_str!("../../../content/bn6/compat/weapons.toml")),
    ("kinds.toml", include_str!("../../../content/bn6/compat/kinds.toml")),
    ("stages.toml", include_str!("../../../content/bn6/compat/stages.toml")),
    ("records.toml", include_str!("../../../content/bn6/compat/records.toml")),
    ("rules.toml", include_str!("../../../content/bn6/compat/rules.toml")),
    ("assets.toml", include_str!("../../../content/bn6/compat/assets.toml")),
    ("text.toml", include_str!("../../../content/bn6/compat/text.toml")),
];

impl Compat {
    /// BN6's compat, as this repository's content/bn6/compat has it.
    pub fn bn6() -> &'static Compat {
        static BN6_COMPAT: std::sync::OnceLock<Compat> = std::sync::OnceLock::new();
        BN6_COMPAT.get_or_init(|| {
            Compat::parse(|file| Ok(BN6.iter().find(|(f, _)| *f == file).map(|(_, t)| t.to_string()).unwrap_or_default()))
                .unwrap_or_else(|e| panic!("content/bn6/compat: {e}"))
        })
    }

    /// Read a compat folder.
    pub fn read(dir: &Path) -> Result<Compat, String> {
        Compat::parse(|file| {
            let path = dir.join(file);
            std::fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))
        })
    }

    /// Parse the files `text` gives by name.
    fn parse(text: impl Fn(&str) -> Result<String, String>) -> Result<Compat, String> {
        fn get<T: serde::de::DeserializeOwned>(text: &impl Fn(&str) -> Result<String, String>, file: &str) -> Result<T, String> {
            toml::from_str(&text(file)?).map_err(|e| format!("{file}: {e}"))
        }
        let mut c = Compat {
            chips: get(&text, "chips.toml")?,
            actions: get(&text, "actions.toml")?,
            navis: get(&text, "navis.toml")?,
            forms: get(&text, "forms.toml")?,
            weapons: get(&text, "weapons.toml")?,
            kinds: get(&text, "kinds.toml")?,
            stages: get(&text, "stages.toml")?,
            records: get(&text, "records.toml")?,
            rules: get(&text, "rules.toml")?,
            assets: get(&text, "assets.toml")?,
            text: get(&text, "text.toml")?,
            slots: BTreeMap::new(),
        };
        for (k, e) in &c.kinds {
            let pool = Pool::from_name(&e.pool)
                .ok_or_else(|| format!("kinds.toml: {k}'s pool {:?} is not actor, attack or effect", e.pool))?;
            if let Some(other) = c.slots.insert((pool, e.index), k.clone()) {
                return Err(format!("kinds.toml: {k} and {other} both fill {} #{:#04X}", e.pool, e.index));
            }
        }
        Ok(c)
    }

    /// The kind that fills an object slot, and its entry.
    pub fn kind_at(&self, pool: Pool, index: u8) -> Option<(&str, &KindEntry)> {
        let key = self.slots.get(&(pool, index))?;
        Some((key.as_str(), &self.kinds[key]))
    }

    /// The key of the chip with this id.
    pub fn chip_key(&self, id: u16) -> Option<&str> {
        self.chips.iter().find(|(_, c)| c.id == id).map(|(k, _)| k.as_str())
    }

    /// The key of the weapon a routine number names.
    pub fn weapon_key(&self, routine: u8) -> Option<&str> {
        self.weapons.iter().find(|(_, n)| n.contains(&routine)).map(|(k, _)| k.as_str())
    }

    /// The key of the navi with this number.
    pub fn navi_key(&self, navi: u8) -> Option<&str> {
        self.navis.iter().find(|(_, n)| n.navi == navi).map(|(k, _)| k.as_str())
    }

    /// The key of MegaMan's form with this number.
    pub fn form_key(&self, form: u8) -> Option<&str> {
        self.forms.iter().find(|(_, f)| f.form == form).map(|(k, _)| k.as_str())
    }

    /// The key of the stage a battle settings record (by its index in
    /// `BattleSettingsList1`) is.
    pub fn stage_key(&self, settings: u8) -> Option<&str> {
        self.stages.iter().find(|(_, st)| st.settings.contains(&settings)).map(|(k, _)| k.as_str())
    }

    /// The original's object slot for object `r`'s kind. The engine never
    /// learns it for a kind content defines: the object records the kind's
    /// handle, and compat has the slot by key. The engine's own kinds and
    /// the kinds the pack registers by number (keyed by their folders)
    /// carry the slot their registration gives.
    pub fn object_slot(&self, b: &Battle, r: ObjectRef) -> Result<(Pool, u8), String> {
        let kind = b.content.defs.kind(b.objects.get(r).kind);
        if let Some(slot) = kind.slot {
            return Ok(slot);
        }
        let e = self.kinds.get(&kind.key).ok_or_else(|| format!("kinds.toml has no {:?}", kind.key))?;
        match Pool::from_name(&e.pool) {
            Some(pool) if pool == kind.pool => Ok((pool, e.index)),
            _ => Err(format!("kinds.toml puts {} in the {} pool; it is defined in the {}", kind.key, e.pool, kind.pool.name())),
        }
    }

    /// The original's action number for object `r`'s CurAction: a navi's
    /// NaviAction (the framework's states as themselves, the ruleset's
    /// actions and content's by key; a v1 registration by its number), any
    /// other object's its own byte.
    pub fn navi_action(&self, b: &Battle, r: ObjectRef) -> Result<u8, String> {
        if b.objects.get(r).actor.is_none() {
            return Ok(b.objects.get(r).action);
        }
        let action = navi_action(b, r);
        if let Some(n) = action.state_number() {
            return Ok(n);
        }
        let key = match action {
            NaviAction::Engine(e) => e.key(),
            NaviAction::Content(h) => {
                let d = b.content.defs.action(h);
                if let Some(n) = d.number {
                    return Ok(n);
                }
                &d.key
            }
            NaviAction::Unported(n) => return Ok(n),
            state => unreachable!("{state:?} is a state"),
        };
        self.actions.get(key).copied().ok_or_else(|| format!("actions.toml has no {key:?}"))
    }
}
