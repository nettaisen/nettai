//! The content pack's scripts: its Luau modules, which live next to what
//! they define (`chips/minibomb/chip.luau`, `chips/rockcube/rock.luau`,
//! `navis/megaman/weapons/absorb/weapon.luau`, `lib/...`).
//!
//! [`Content::define`](super::Content::define) turns what the modules
//! define into what the script runtime binds (`content::defs`). Nothing in
//! the engine says which kind, action or hook is a script: whatever the
//! pack defines runs as content, and the engine's own Rust runs the rest.

use std::collections::BTreeMap;

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
