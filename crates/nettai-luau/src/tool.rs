//! Tool Luau: Luau a tool runs that isn't battle content, in a VM of its
//! own (docs/design/extraction-recipes.md §4.4): the games' extraction
//! recipes, which nettai-extract runs. (A recipe's modules load with the
//! content too, as its game's root gives it: there they only load, in an
//! environment of their own, `define::Environments`.)
//!
//! The VM is content's ([`crate::sandbox`]: the deterministic standard
//! library, the compiler, the bytecode check, modules frozen as they load),
//! with none of the battle's libraries (`asset`, `new`, `rules`, `battle`,
//! ...): the caller installs its own ([`open`]'s `install`). A module
//! requires only modules under one folder (`tree`, `extract/`) of its own
//! pack and of the support packs it depends on, found as content finds them
//! (`packs::find`). One budget counts the whole run, loading and calls
//! alike, so a runaway loop fails rather than hangs the caller's thread.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;

use mlua::{Lua, Value as LuaValue, VmState};
use nettai_content_api::packs::{self, Modules, PackManifest};
use nettai_content_api::{ContentError, keys};

use crate::{Compiled, Loader, Pack, load_module, sandbox};

/// How tool Luau runs.
#[derive(Clone, Copy, Debug)]
pub struct ToolOptions {
    /// Interrupt checks (calls, returns, loop back-edges) the whole run may
    /// take, loading included, before it is stopped as runaway.
    pub budget: u64,
    /// Let `print` write to stderr.
    pub debug_print: bool,
}

impl Default for ToolOptions {
    fn default() -> ToolOptions {
        // (Far past what a game's recipe takes, a few million: only a loop
        // that doesn't end meets it.)
        ToolOptions { budget: 2_000_000_000, debug_print: false }
    }
}

/// A tool's VM, its entry module loaded.
pub struct Tool {
    lua: Lua,
    entry: LuaValue,
    left: Rc<Cell<u64>>,
}

impl Tool {
    /// The VM.
    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    /// What the entry module returned.
    pub fn entry(&self) -> &LuaValue {
        &self.entry
    }

    /// Interrupt checks the run has left of its budget.
    pub fn budget_left(&self) -> u64 {
        self.left.get()
    }

    /// Call the function the entry module returned with `args`.
    pub fn call<R: mlua::FromLuaMulti>(&self, args: impl mlua::IntoLuaMulti) -> Result<R, ContentError> {
        let LuaValue::Function(f) = &self.entry else {
            return Err(ContentError::new(format!("the entry module returned {}, not a function", self.entry.type_name())));
        };
        f.call::<R>(args).map_err(|e| ContentError::new(e.to_string()))
    }
}

/// A VM for tool Luau: the content standard library, `require` of the
/// modules under folder `tree` of the packs `packs` (each module's own
/// pack's and its support packs', read where `modules` has them, by name:
/// `exe6:extract/hud`), the libraries `install` adds, and module `entry`
/// (by name, under `tree`) loaded. Libraries and globals are read-only
/// once it is open.
pub fn open(
    modules: impl Modules + 'static,
    packs: impl IntoIterator<Item = PackManifest>,
    entry: &str,
    tree: &str,
    options: ToolOptions,
    install: impl FnOnce(&Lua) -> mlua::Result<()>,
) -> Result<Tool, ContentError> {
    let err = |e: mlua::Error| ContentError::new(e.to_string());
    let pack = Pack::of(modules, vec![entry.to_string()]).with_packs(packs);
    let under = |name: &str| name.split_once(keys::SEPARATOR).is_some_and(|(_, path)| path == tree || path.starts_with(&format!("{tree}/")));
    let entry = packs::module(&*pack.modules, entry)
        .filter(|m| under(m))
        .ok_or_else(|| ContentError::new(format!("no module {}.luau under {tree}/", keys::module_path(entry))))?;
    let lua = sandbox::new_vm(options.debug_print).map_err(err)?;
    install(&lua).map_err(err)?;
    let loader = Rc::new(RefCell::new(Loader {
        pack: pack.clone(),
        loaded: BTreeMap::new(),
        compiled: Compiled::default(),
        stack: Vec::new(),
        envs: None,
        support: HashSet::new(),
    }));
    let require = {
        let loader = Rc::downgrade(&loader);
        let packs = pack.packs.clone();
        let tree = tree.to_string();
        lua.create_function(move |lua, path: String| {
            let loader = loader.upgrade().ok_or_else(|| mlua::Error::runtime("require is only available while the modules load"))?;
            let from = loader.borrow().stack.last().cloned();
            let from = from.ok_or_else(|| mlua::Error::runtime("require is only available at the top of a module"))?;
            let target = packs::find(&*loader.borrow().pack.modules, &packs, &from, &path).map_err(mlua::Error::runtime)?;
            let inside = target.split_once(keys::SEPARATOR).is_some_and(|(_, p)| p.starts_with(&format!("{tree}/")));
            if !inside {
                return Err(mlua::Error::runtime(format!(
                    "{}.luau: require({path:?}): {}.luau isn't under {tree}/ (a module here requires only modules under {tree}/)",
                    keys::module_path(&from),
                    keys::module_path(&target)
                )));
            }
            load_module(lua, &loader, &target)
        })
        .map_err(err)?
    };
    lua.globals().set("require", require).map_err(err)?;
    // Libraries and globals become read-only; the budget stops a runaway
    // loop, loading or running.
    lua.sandbox(true).map_err(err)?;
    let left = Rc::new(Cell::new(options.budget));
    {
        let left = left.clone();
        lua.set_interrupt(move |_| match left.get() {
            0 => Err(mlua::Error::runtime("the Luau ran past its budget (a runaway loop?)")),
            n => {
                left.set(n - 1);
                Ok(VmState::Continue)
            }
        });
    }
    let value = load_module(&lua, &loader, &entry).map_err(err)?;
    drop(loader);
    lua.globals().set_readonly(true);
    Ok(Tool { lua, entry: value, left })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_content_api::PackKind;

    fn manifest(id: &str, kind: PackKind, depends: &[&str]) -> PackManifest {
        PackManifest { id: id.into(), kind, depends: depends.iter().map(|u| u.to_string()).collect() }
    }

    fn modules(list: &[(&str, &str)]) -> BTreeMap<String, String> {
        list.iter().map(|(n, s)| (n.to_string(), s.to_string())).collect()
    }

    fn packs() -> Vec<PackManifest> {
        vec![manifest("lib", PackKind::Support, &[]), manifest("g", PackKind::Game, &["lib"])]
    }

    fn install(lua: &Lua) -> mlua::Result<()> {
        lua.globals().set("twice", lua.create_function(|_, n: i64| Ok(2 * n))?)
    }

    #[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
    fn a_tool_runs_its_tree_with_its_own_libraries() {
        let m = modules(&[
            ("g:extract/init", "local place = require(\"@lib/extract/place\")\nlocal own = require(\"@self/own\")\nreturn function(n) return place.at(n) + own + twice(n) end"),
            ("g:extract/own", "return 100"),
            ("lib:extract/place", "return { at = function(n) return n * 10 end }"),
        ]);
        let tool = open(m, packs(), "g:extract", "extract", ToolOptions::default(), install).unwrap();
        assert_eq!(tool.call::<i64>(3).unwrap(), 30 + 100 + 6);
        // None of the battle's libraries.
        let none: Vec<bool> = ["asset", "new", "rules", "battle", "define"].iter().map(|g| tool.lua().globals().get::<LuaValue>(*g).unwrap().is_nil()).collect();
        assert!(none.iter().all(|&n| n), "{none:?}");
    }

    #[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
    fn a_tool_requires_nothing_outside_its_tree() {
        let m = modules(&[("g:extract/init", "return require(\"@lib/swords\")"), ("lib:swords", "return 1")]);
        let e = open(m, packs(), "g:extract", "extract", ToolOptions::default(), install).err().unwrap().to_string();
        assert!(e.contains("lib/swords.luau isn't under extract/"), "{e}");
        let m = modules(&[("g:extract/init", "return 1"), ("g:rules", "return 1")]);
        assert!(open(m, packs(), "g:rules", "extract", ToolOptions::default(), install).is_err(), "an entry outside the tree");
        // A support pack the game doesn't depend on, as content's requires.
        let m = modules(&[("g:extract/init", "return require(\"@other/extract/x\")"), ("other:extract/x", "return 1")]);
        let mut p = packs();
        p.push(manifest("other", PackKind::Support, &[]));
        let e = open(m, p, "g:extract", "extract", ToolOptions::default(), install).err().unwrap().to_string();
        assert!(e.contains("doesn't depend on other"), "{e}");
    }

    #[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
    fn a_runaway_tool_stops_and_its_modules_are_frozen() {
        let m = modules(&[("g:extract/init", "return function() local n = 0 while true do n += 1 end end")]);
        let tool = open(m, packs(), "g:extract", "extract", ToolOptions { budget: 10_000, ..Default::default() }, install).unwrap();
        let e = tool.call::<()>(()).unwrap_err().to_string();
        assert!(e.contains("past its budget"), "{e}");
        let m = modules(&[("g:extract/init", "local t = { 1 }\nreturn function() t[1] = 2 end")]);
        let tool = open(m, packs(), "g:extract", "extract", ToolOptions::default(), install).unwrap();
        assert!(tool.call::<()>(()).unwrap_err().to_string().contains("readonly"));
        // No global is assigned, no module-level local written.
        let m = modules(&[("g:extract/init", "local n = 0\nreturn function() n = 1 end")]);
        assert!(open(m, packs(), "g:extract", "extract", ToolOptions::default(), install).is_err());
    }
}
