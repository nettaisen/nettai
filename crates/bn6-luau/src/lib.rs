//! Luau content for the bn6 battle engine (docs/design/scripting.md).
//!
//! A content pack is a set of Luau modules; its `pack` module returns the
//! object kinds and actions it defines:
//!
//! ```luau
//! return {
//!     objects = { require("./objects/sun_beam"), ... },
//!     actions = { require("./chips/gun_del_sol") },
//! }
//! ```
//!
//! An object kind is a table `{ pool, index, state, update }`; an action is
//! `{ action, state, update }`. `state` declares the kind's typed state
//! (`{ timer = "u16", slot = { "overlay", "related" } }`), which the engine
//! stores and snapshots; `update(me)` / `update(me, state)` run once per
//! tick and keep nothing themselves.
//!
//! Loading enforces that: modules are checked for writes to globals and to
//! module-level locals (`verify`), everything a module returns or captures
//! is frozen (`sandbox::deep_freeze`), `require` works only while loading,
//! and the standard library is cut down to what is deterministic
//! (`sandbox`).

mod bind;
pub mod sandbox;
pub mod verify;

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::rc::Rc;

use bn6_content_api::{
    ActionDef, ContentError, ContentHost, CoreApi, FieldDef, FieldType, KindId, Manifest, ObjectKindDef, ObjectRef,
    Pool, Schema,
};
use mlua::chunk::{ChunkMode, Compiler};
use mlua::{Function, Lua, Table, Value as LuaValue, VmState};

/// A content pack's sources: module path (relative to the pack root,
/// without `.luau`) to source text.
#[derive(Clone, Debug, Default)]
pub struct Pack {
    modules: BTreeMap<String, String>,
}

impl Pack {
    pub fn new(modules: impl IntoIterator<Item = (String, String)>) -> Pack {
        Pack { modules: modules.into_iter().collect() }
    }

    /// Every `.luau` file under `dir` (definition files, `.d.luau`,
    /// excluded).
    pub fn from_dir(dir: &Path) -> std::io::Result<Pack> {
        fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) -> std::io::Result<()> {
            for entry in std::fs::read_dir(dir)? {
                let path = entry?.path();
                if path.is_dir() {
                    walk(root, &path, out)?;
                    continue;
                }
                let Some(name) = path.to_str().and_then(|s| s.strip_suffix(".luau")) else { continue };
                if name.ends_with(".d") {
                    continue;
                }
                let rel = Path::new(name).strip_prefix(root).expect("walked under the root");
                let key = rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
                out.insert(key, std::fs::read_to_string(&path)?);
            }
            Ok(())
        }
        let mut modules = BTreeMap::new();
        walk(dir, dir, &mut modules)?;
        Ok(Pack { modules })
    }

    pub fn modules(&self) -> impl Iterator<Item = (&str, &str)> {
        self.modules.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

/// How content runs.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Compile to native code where Luau supports it (the `jit` feature).
    pub native_code: bool,
    /// Interrupt checks (calls, returns, loop back-edges) one update call
    /// may take before it is stopped as runaway. Deterministic: the same
    /// bytecode reaches the same count on every machine.
    pub budget: u32,
    /// Let `print` write to stderr.
    pub debug_print: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options { native_code: false, budget: 1_000_000, debug_print: false }
    }
}

thread_local! {
    /// Interrupt checks left for the running call.
    static BUDGET: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Luau content: the VM and the functions the pack defined.
pub struct LuauContent {
    lua: Lua,
    manifest: Manifest,
    objects: Vec<Function>,
    actions: Vec<Function>,
    budget: u32,
}

impl LuauContent {
    /// Load and check a pack.
    pub fn load(pack: &Pack, options: Options) -> Result<LuauContent, ContentError> {
        load(pack, options).map_err(|e| ContentError::new(format!("loading Luau content: {e}")))
    }

    /// The VM (for tests and tools).
    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    fn call(&self, f: &Function, api: &mut dyn CoreApi, args: impl mlua::IntoLuaMulti) -> Result<(), ContentError> {
        let _enter = bind::Enter::new(api, &self.manifest);
        BUDGET.with(|b| b.set(self.budget));
        f.call::<()>(args).map_err(|e| ContentError::new(e.to_string()))
    }
}

impl ContentHost for LuauContent {
    fn runtime(&self) -> &str {
        "luau"
    }

    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn update_object(&self, api: &mut dyn CoreApi, kind: KindId, me: ObjectRef) -> Result<(), ContentError> {
        let o = bind::object(&self.lua, me).map_err(|e| ContentError::new(e.to_string()))?;
        self.call(&self.objects[kind.0 as usize], api, o)
    }

    fn update_action(&self, api: &mut dyn CoreApi, action: KindId, me: ObjectRef) -> Result<(), ContentError> {
        let o = bind::object(&self.lua, me).map_err(|e| ContentError::new(e.to_string()))?;
        let s = bind::action_state(&self.lua, me).map_err(|e| ContentError::new(e.to_string()))?;
        self.call(&self.actions[action.0 as usize], api, (o, s))
    }
}

/// Module loading state (only while `load` runs).
struct Loader {
    pack: Pack,
    /// Module results by path.
    loaded: HashMap<String, LuaValue>,
    /// Modules being loaded, innermost last (for relative paths and
    /// cycles).
    stack: Vec<String>,
}

/// Resolve a `require` path against the requiring module's directory.
fn resolve(from: &str, path: &str) -> Result<String, String> {
    if !(path.starts_with("./") || path.starts_with("../")) {
        return Err(format!("require({path:?}): content paths start with ./ or ../"));
    }
    let mut parts: Vec<&str> = from.split('/').collect();
    parts.pop();
    for seg in path.trim_end_matches(".luau").split('/') {
        match seg {
            "." | "" => {}
            ".." => {
                parts.pop().ok_or_else(|| format!("require({path:?}) from {from} leaves the pack"))?;
            }
            s => parts.push(s),
        }
    }
    Ok(parts.join("/"))
}

fn load_module(lua: &Lua, loader: &Rc<RefCell<Loader>>, path: &str) -> mlua::Result<LuaValue> {
    let source = {
        let l = loader.borrow();
        if let Some(v) = l.loaded.get(path) {
            return Ok(v.clone());
        }
        if l.stack.iter().any(|p| p == path) {
            return Err(mlua::Error::runtime(format!("require cycle: {} -> {path}", l.stack.join(" -> "))));
        }
        l.pack.modules.get(path).cloned().ok_or_else(|| mlua::Error::runtime(format!("no module {path}.luau in the pack")))?
    };
    let bytecode = Compiler::new().set_optimization_level(1).set_debug_level(2).compile(&source)?;
    verify::check(path, &bytecode).map_err(|v| mlua::Error::runtime(v.to_string()))?;
    let chunk = lua.load(&bytecode[..]).set_name(format!("@{path}.luau")).set_mode(ChunkMode::Binary).into_function()?;
    loader.borrow_mut().stack.push(path.to_string());
    let result = chunk.call::<LuaValue>(());
    loader.borrow_mut().stack.pop();
    let value = result?;
    sandbox::deep_freeze(lua, &value)?;
    loader.borrow_mut().loaded.insert(path.to_string(), value.clone());
    Ok(value)
}

fn load(pack: &Pack, options: Options) -> mlua::Result<LuauContent> {
    let lua = sandbox::new_vm(options.debug_print)?;
    #[cfg(feature = "jit")]
    lua.enable_jit(options.native_code);
    #[cfg(not(feature = "jit"))]
    if options.native_code {
        return Err(mlua::Error::runtime("native code needs bn6-luau's `jit` feature"));
    }
    bind::install(&lua)?;
    let loader = Rc::new(RefCell::new(Loader { pack: pack.clone(), loaded: HashMap::new(), stack: Vec::new() }));
    let require = {
        let loader = Rc::downgrade(&loader);
        lua.create_function(move |lua, path: String| {
            let loader = loader.upgrade().ok_or_else(|| mlua::Error::runtime("require is only available while content loads"))?;
            let from = loader.borrow().stack.last().cloned();
            let from = from.ok_or_else(|| mlua::Error::runtime("require is only available at the top of a module"))?;
            let target = resolve(&from, &path).map_err(mlua::Error::runtime)?;
            load_module(lua, &loader, &target)
        })?
    };
    lua.globals().set("require", require)?;
    // Libraries and globals become read-only; the budget stops runaway
    // loops (also while loading).
    lua.sandbox(true)?;
    BUDGET.with(|b| b.set(options.budget.saturating_mul(16)));
    lua.set_interrupt(|_| {
        BUDGET.with(|b| match b.get() {
            0 => Err(mlua::Error::runtime("content ran past its budget (a runaway loop?)")),
            n => {
                b.set(n - 1);
                Ok(VmState::Continue)
            }
        })
    });

    // The pack's entry module, loaded as if required from the root.
    loader.borrow_mut().stack.push("(pack)".into());
    let root = load_module(&lua, &loader, "pack");
    loader.borrow_mut().stack.clear();
    let root = match root? {
        LuaValue::Table(t) => t,
        v => return Err(mlua::Error::runtime(format!("pack.luau returned {}, not a table", v.type_name()))),
    };
    let (manifest, objects, actions) = read_manifest(&root)?;
    manifest.validate().map_err(|e| mlua::Error::runtime(e.to_string()))?;
    // Nothing a script can reach may change after loading.
    lua.globals().set_readonly(true);
    drop(loader);
    Ok(LuauContent { lua, manifest, objects, actions, budget: options.budget })
}

/// A kind's `state` table: field name to type name (`"u16"`), or to a list
/// of variant names (an enum). Fields are stored in name order.
fn read_schema(t: Option<Table>, what: &str) -> mlua::Result<Schema> {
    let mut fields = Vec::new();
    if let Some(t) = t {
        for pair in t.pairs::<String, LuaValue>() {
            let (name, ty) = pair?;
            let ty = match ty {
                LuaValue::String(s) => {
                    let s = s.to_str()?;
                    FieldType::scalar(&s)
                        .ok_or_else(|| mlua::Error::runtime(format!("{what}: state field `{name}` has unknown type {:?}", &*s)))?
                }
                LuaValue::Table(variants) => {
                    FieldType::Enum(variants.sequence_values::<String>().collect::<mlua::Result<Vec<_>>>()?)
                }
                v => return Err(mlua::Error::runtime(format!("{what}: state field `{name}` is a {}", v.type_name()))),
            };
            fields.push(FieldDef { name, ty });
        }
    }
    fields.sort_by(|a, b| a.name.cmp(&b.name));
    Schema::new(fields).map_err(|e| mlua::Error::runtime(format!("{what}: {e}")))
}

type Kinds = (Manifest, Vec<Function>, Vec<Function>);

fn read_manifest(root: &Table) -> mlua::Result<Kinds> {
    let mut manifest = Manifest::default();
    let (mut objects, mut actions) = (Vec::new(), Vec::new());
    if let Some(list) = root.get::<Option<Table>>("objects")? {
        for (i, def) in list.sequence_values::<Table>().enumerate() {
            let def = def?;
            let what = format!("objects[{}]", i + 1);
            let pool: String = def.get("pool")?;
            let pool = Pool::from_name(&pool).ok_or_else(|| mlua::Error::runtime(format!("{what}: {pool:?} is not a pool")))?;
            let index: u8 = def.get("index")?;
            let name = def.get::<Option<String>>("name")?.unwrap_or_else(|| format!("{} {index:#x}", pool.name()));
            let schema = read_schema(def.get("state")?, &name)?;
            objects.push(def.get::<Function>("update")?);
            manifest.objects.push(ObjectKindDef { name, pool, index, schema });
        }
    }
    if let Some(list) = root.get::<Option<Table>>("actions")? {
        for (i, def) in list.sequence_values::<Table>().enumerate() {
            let def = def?;
            let action: u8 = def.get("action")?;
            let name = def.get::<Option<String>>("name")?.unwrap_or_else(|| format!("actions[{}] {action:#x}", i + 1));
            let schema = read_schema(def.get("state")?, &name)?;
            actions.push(def.get::<Function>("update")?);
            manifest.actions.push(ActionDef { name, action, schema });
        }
    }
    Ok((manifest, objects, actions))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_resolve_within_the_pack() {
        assert_eq!(resolve("chips/gun_del_sol", "../objects/hitbox").unwrap(), "objects/hitbox");
        assert_eq!(resolve("pack", "./chips/gun_del_sol").unwrap(), "chips/gun_del_sol");
        assert!(resolve("pack", "../x").is_err());
        assert!(resolve("pack", "objects/x").is_err());
    }
}
