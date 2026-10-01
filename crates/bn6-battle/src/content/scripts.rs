//! The content pack's scripts and what its entities say they implement.
//!
//! A pack's Luau modules live next to the data they implement
//! (`objects/sun-beam/sun_beam.luau`, `chips/00f-gundels1/chip.luau`,
//! `navis/megaman/weapons/absorb/weapon.luau`, `lib/...`).
//! Entities name their script in their data:
//!
//! - an object kind's `[kind]` table (`objects/<name>/object.toml`) gives
//!   the object slot it implements (pool and index) and its script
//!   ([`ObjectKind`]);
//! - a chip's `script` implements the chip's action, or for the ruleset's
//!   generic actions its part of them by the chip's subtype: action 0x15
//!   (dimming chips) its dimming controller, action 0x1B (navi chips) its
//!   navi, action 0x1C (instant chips) its effect
//!   ([`ChipData::script`](super::ChipData::script));
//! - a weapon routine of MegaMan's (`navis/megaman/weapons/NN-name/
//!   weapon.toml`) implements the routine, the action it names and the
//!   instant chip effect it names ([`WeaponData`]).
//!
//! [`Content::define`] turns that, with what the modules define, into
//! what the script runtime binds (`content::defs`). Nothing in the engine says which kind, action or hook is a
//! script: whatever the pack registers runs as content, and the engine's
//! own Rust runs the rest.

use std::collections::BTreeMap;

use bn6_content_api::Pool;
use serde::{Deserialize, Serialize};

use super::Content;

/// The chips' actions the ruleset implements itself and dispatches to a
/// chip's script by its subtype.
pub const DIMMING_CHIP_ACTION: u8 = 0x15;
pub const NAVI_CHIP_ACTION: u8 = 0x1B;
pub const INSTANT_CHIP_ACTION: u8 = 0x1C;

/// The pack's Luau modules.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Scripts {
    /// Source text by module path: the file's path in the pack without
    /// `.luau` (`objects/sun-beam/sun_beam`).
    pub modules: BTreeMap<String, String>,
    /// Their bytecode, as the define phase compiled it (a runtime then
    /// skips the compiler).
    pub compiled: CompiledModules,
}

impl Scripts {
    /// Modules by path, without bytecode.
    pub fn new(modules: BTreeMap<String, String>) -> Scripts {
        Scripts { modules, compiled: CompiledModules::default() }
    }

    /// The modules as a runtime loads them, with the bytecode compiled
    /// from them.
    pub fn pack(&self) -> bn6_luau::Pack {
        let modules = self.modules.iter().map(|(k, v)| (k.clone(), v.clone()));
        bn6_luau::Pack::new(modules).with_compiled(self.compiled.0.clone())
    }
}

/// Bytecode compiled from [`Scripts::modules`]: derived from them (each
/// module's with the source it was compiled from, which loading checks),
/// so content equality and the content hash leave it out.
#[derive(Clone, Default)]
pub struct CompiledModules(pub bn6_luau::Compiled);

impl std::fmt::Debug for CompiledModules {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "CompiledModules({})", self.0.len())
    }
}

impl PartialEq for CompiledModules {
    fn eq(&self, _: &CompiledModules) -> bool {
        true
    }
}

impl Eq for CompiledModules {}

impl std::hash::Hash for CompiledModules {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
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
    /// The instant chip effect (action 0x1C's subtype, `off_80EC3F0`) the
    /// script implements, when the routine names action 0x1C with a
    /// subtype no chip has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instant_chip: Option<u8>,
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
    /// The object kind named `name`.
    pub fn object_kind(&self, name: &str) -> Option<&ObjectKind> {
        self.objects.kinds.iter().find(|k| k.name == name)
    }

    /// The content object kind in a slot, if a script implements it.
    pub fn object_kind_at(&self, pool: Pool, index: u8) -> Option<&ObjectKind> {
        self.objects.kinds.iter().find(|k| (k.pool, k.index) == (pool, index))
    }
}
