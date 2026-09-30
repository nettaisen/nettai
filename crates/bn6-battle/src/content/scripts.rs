//! The content pack's scripts and what its entities say they implement.
//!
//! A pack's Luau modules live next to the data they implement
//! (`objects/sun-beam/sun_beam.luau`, `chips/00f-gundels1/chip.luau`,
//! `navis/00-megaman/weapons/02-blank-shot/blank_shot.luau`, `lib/...`).
//! Entities name their script in their data:
//!
//! - an object kind's `[kind]` table (`objects/<name>/object.toml`) gives
//!   the object slot it implements (pool and index) and its script
//!   ([`ObjectKind`]);
//! - a chip's `script` implements the chip's action, or for the ruleset's
//!   generic actions its part of them by the chip's subtype: action 0x15
//!   (dimming chips) its dimming controller, action 0x1B (navi chips) its
//!   navi ([`ChipData::script`](super::ChipData::script));
//! - a weapon routine of MegaMan's (`navis/00-megaman/weapons/NN-name/
//!   weapon.toml`) implements the routine and the action it names
//!   ([`WeaponData`]).
//!
//! [`Content::registrations`] turns that into what the script runtime
//! loads. Nothing in the engine says which kind, action or hook is a
//! script: whatever the pack registers runs as content, and the engine's
//! own Rust runs the rest.

use std::collections::BTreeMap;

use bn6_content_api::{ActionReg, Hook, HookReg, KindReg, Pool, Registrations};
use serde::{Deserialize, Serialize};

use super::Content;

/// The chips' actions the ruleset implements itself and dispatches to a
/// chip's script by its subtype.
pub const DIMMING_CHIP_ACTION: u8 = 0x15;
pub const NAVI_CHIP_ACTION: u8 = 0x1B;

/// The pack's Luau modules.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Scripts {
    /// Source text by module path: the file's path in the pack without
    /// `.luau` (`objects/sun-beam/sun_beam`).
    pub modules: BTreeMap<String, String>,
}

/// An object kind a script implements (an object folder's `[kind]`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectKind {
    /// The kind's name: its folder under `objects/` (how scripts and
    /// engine code spawn it).
    #[serde(skip)]
    pub name: String,
    /// The object slot it fills: its pool and index, the original's
    /// identity the traces compare.
    #[serde(with = "pool_name")]
    pub pool: Pool,
    pub index: u8,
    /// The module (see [`Scripts::modules`]); in the file, a path relative
    /// to the folder.
    pub script: String,
    /// Its position is whatever its spawner's registers held until its
    /// init places it (the trace comparison skips it).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub scratch_position: bool,
}

/// A weapon routine of MegaMan's that a script implements
/// (`off_80117D4`: what a button's weapon does).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponData {
    /// The routine number (a form's or the navi stats' weapon byte).
    pub id: u8,
    pub name: String,
    /// The action the script implements besides the routine (the one its
    /// `setup` names), if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<u8>,
    /// The module; in the file, a path relative to the folder.
    pub script: String,
}

mod pool_name {
    use bn6_content_api::Pool;

    pub fn serialize<S: serde::Serializer>(p: &Pool, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(p.name())
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Pool, D::Error> {
        let s = <String as serde::Deserialize>::deserialize(d)?;
        Pool::from_name(&s).ok_or_else(|| serde::de::Error::custom(format!("{s:?} is not a pool (actor, attack, effect)")))
    }
}

impl Content {
    /// What the pack's scripts implement, from its entities' data. Errors
    /// name the entities that conflict (two scripts for one action, a
    /// script that isn't in the pack).
    pub fn registrations(&self) -> Result<Registrations, String> {
        let mut r = Registrations::default();
        let exists = |module: &str, whose: &str| {
            if self.scripts.modules.contains_key(module) {
                Ok(())
            } else {
                Err(format!("{whose} names the script {module}.luau, which isn't in the pack"))
            }
        };
        for k in &self.objects.kinds {
            exists(&k.script, &format!("object kind {}", k.name))?;
            r.kinds.push(KindReg { name: k.name.clone(), pool: k.pool, index: k.index, module: k.script.clone() });
        }
        let mut actions: BTreeMap<u8, (String, String)> = BTreeMap::new();
        let mut hooks: BTreeMap<Hook, (String, String)> = BTreeMap::new();
        let mut add_action = |action: u8, module: &str, whose: String| -> Result<(), String> {
            match actions.get(&action) {
                Some((m, first)) if m != module => {
                    Err(format!("{whose} implements action {action:#x} with {module}.luau, but {first} with {m}.luau"))
                }
                Some(_) => Ok(()),
                None => {
                    actions.insert(action, (module.to_string(), whose));
                    Ok(())
                }
            }
        };
        let mut add_hook = |hook: Hook, module: &str, whose: String| -> Result<(), String> {
            match hooks.get(&hook) {
                Some((m, first)) if m != module => {
                    Err(format!("{whose} implements {hook} with {module}.luau, but {first} with {m}.luau"))
                }
                Some(_) => Ok(()),
                None => {
                    hooks.insert(hook, (module.to_string(), whose));
                    Ok(())
                }
            }
        };
        for c in &self.chips {
            let Some(module) = &c.script else { continue };
            let whose = format!("chip {:#05x} ({})", c.id, c.name);
            exists(module, &whose)?;
            match c.action {
                DIMMING_CHIP_ACTION => add_hook(Hook::DimmingChip(c.subtype), module, whose)?,
                NAVI_CHIP_ACTION => add_hook(Hook::NaviChip(c.subtype), module, whose)?,
                action if action < 0x10 => return Err(format!("{whose}: actions below 0x10 are the engine's")),
                action => add_action(action, module, whose)?,
            }
        }
        for w in &self.weapons {
            let whose = format!("weapon routine {:#04x} ({})", w.id, w.name);
            exists(&w.script, &whose)?;
            add_hook(Hook::Weapon(w.id), &w.script, whose.clone())?;
            if let Some(action) = w.action {
                add_action(action, &w.script, whose)?;
            }
        }
        r.actions = actions.into_iter().map(|(action, (module, _))| ActionReg { action, module }).collect();
        r.hooks = hooks.into_iter().map(|(hook, (module, _))| HookReg { hook, module }).collect();
        r.validate().map_err(|e| e.message)?;
        Ok(r)
    }

    /// The object kind named `name`.
    pub fn object_kind(&self, name: &str) -> Option<&ObjectKind> {
        self.objects.kinds.iter().find(|k| k.name == name)
    }

    /// The content object kind in a slot, if a script implements it.
    pub fn object_kind_at(&self, pool: Pool, index: u8) -> Option<&ObjectKind> {
        self.objects.kinds.iter().find(|k| (k.pool, k.index) == (pool, index))
    }
}
