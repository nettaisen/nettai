//! What the define phase reads from a content pack
//! (docs/design/content-model-v2.md §7.3): every definition the pack's
//! modules make, as plain data (the canonical tree), and what each module
//! exports. A runtime produces it; the engine turns it into typed content
//! and checks that every runtime made from the same content reads the same.

use crate::data::Data;
use crate::registry::Registry;

/// One definition.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Definition {
    pub registry: Registry,
    /// Its key: an explicit `id`, or derived (docs/design/content-model-v2.md
    /// §2.2).
    pub key: String,
    /// The module that made it (its path in the pack, without `.luau`).
    pub module: String,
    /// A record's type (`define.record(type, spec)`); None for the others.
    pub record_type: Option<String>,
    /// Its spec: fields as data, other definitions as `Data::Ref`, the
    /// tables schemas come from as `Data::Ref(Registry::Schema, ..)`,
    /// functions as `Data::Function`.
    pub spec: Data,
}

/// Everything the define phase read, in a canonical order: definitions by
/// registry then key.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Definitions {
    pub defs: Vec<Definition>,
}

/// What a game has (docs/design/content-model-v2.md §4.0): the groups of a
/// game pack's top module (`<game>/init.luau`), by name, each with the
/// registries whose definitions it holds. Every definition of these
/// registries is made by a module its game's init.luau requires itself
/// (`packs::required_by_init`).
pub const GAME_LISTS: &[(&str, &[Registry])] = &[
    ("rules", &[Registry::Ruleset]),
    ("chips", &[Registry::Chip]),
    ("navis", &[Registry::Navi]),
    ("forms", &[Registry::Form]),
    ("stages", &[Registry::Stage]),
    ("patch_cards", &[Registry::PatchCard]),
    ("navicust", &[Registry::NaviCustProgram]),
];

impl Definitions {
    /// The definitions of one registry, in key order (their handles).
    pub fn of(&self, registry: Registry) -> &[Definition] {
        let start = self.defs.partition_point(|d| d.registry < registry);
        let end = self.defs.partition_point(|d| d.registry <= registry);
        &self.defs[start..end]
    }

    /// A definition by registry and key.
    pub fn get(&self, registry: Registry, key: &str) -> Option<&Definition> {
        let defs = self.of(registry);
        defs.binary_search_by(|d| d.key.as_str().cmp(key)).ok().map(|i| &defs[i])
    }

    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }
}
