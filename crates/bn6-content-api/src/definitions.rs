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

/// What a module returns that registration by module still uses (the v1
/// registration of content-pack.md §1.3, until the migration ends): the
/// names of the functions its table exports, its `state` table's schema,
/// and the action definition it exports as `action`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModuleExports {
    pub path: String,
    /// Function fields of the module's table, sorted.
    pub functions: Vec<String>,
    /// The key of the schema its `state` field declares.
    pub state: Option<String>,
    /// The key of the action definition its `action` field holds: a chip
    /// record that names the module runs it (docs/design/content-model-v2.md
    /// §12, "A record's action by its module").
    pub action: Option<String>,
}

/// Everything the define phase read, in a canonical order: definitions by
/// registry then key, modules by path.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Definitions {
    pub defs: Vec<Definition>,
    pub modules: Vec<ModuleExports>,
}

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

    /// A module's exports.
    pub fn module(&self, path: &str) -> Option<&ModuleExports> {
        self.modules.binary_search_by(|m| m.path.as_str().cmp(path)).ok().map(|i| &self.modules[i])
    }

    pub fn is_empty(&self) -> bool {
        self.defs.is_empty() && self.modules.is_empty()
    }
}
