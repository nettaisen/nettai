//! Luau content for the bn6 battle engine (docs/design/scripting.md).
//!
//! A content pack's scripts are Luau modules, colocated with the data they
//! implement (`objects/sun-beam/sun_beam.luau`, `chips/00f-gundels1/chip.luau`,
//! `lib/slot.luau`...). The pack's data registers them (see
//! `bn6_content_api::Registrations`): which module implements which object
//! kind, navi action or hook. A module returns a table:
//!
//! ```luau
//! return {
//!     state = { timer = "u16", slot = { "overlay", "related" } },
//!     update = function(me, s) ... end,   -- kinds: update(me); actions: update(me, s)
//!     setup = function(navi) ... end,     -- a weapon routine
//! }
//! ```
//!
//! `state` declares the kind's or action's typed state, which the engine
//! stores and snapshots; the functions run on demand and keep nothing
//! themselves. The pack's data is the frozen global `data`.
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
    ActionDef, ContentError, ContentHost, CoreApi, Data, DataKey, FieldDef, FieldType, HookCall, HookDef, HookId,
    KindId, Manifest, ObjectKindDef, ObjectRef, Registrations, Schema, Value,
};
use mlua::chunk::ChunkMode;
use mlua::{Function, Lua, Table, Value as LuaValue, VmState};

/// A content pack's scripts: module path (relative to the pack root,
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
    /// Interrupt checks (calls, returns, loop back-edges) one call may take
    /// before it is stopped as runaway. Deterministic: the same bytecode
    /// reaches the same count on every machine.
    pub budget: u32,
    /// Let `print` write to stderr.
    pub debug_print: bool,
    /// Run a full garbage collection after every call (tests use it to
    /// show GC timing doesn't reach the battle).
    pub collect_garbage: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options { native_code: false, budget: 1_000_000, debug_print: false, collect_garbage: false }
    }
}

/// Whether this build and machine can compile Luau to native code.
pub fn native_code_supported() -> bool {
    #[cfg(feature = "jit")]
    // SAFETY: a query with no arguments.
    return unsafe { mlua::ffi::luau_codegen_supported() != 0 };
    #[cfg(not(feature = "jit"))]
    false
}

thread_local! {
    /// Interrupt checks left for the running call.
    static BUDGET: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Luau content: the VM and the functions the pack registered.
pub struct LuauContent {
    lua: Lua,
    manifest: Manifest,
    objects: Vec<Function>,
    actions: Vec<Function>,
    hooks: Vec<Function>,
    budget: u32,
    collect_garbage: bool,
}

impl LuauContent {
    /// Load a pack's registered modules (and what they require), with the
    /// pack's data as the global `data`.
    pub fn load(pack: &Pack, registrations: &Registrations, data: &Data, options: Options) -> Result<LuauContent, ContentError> {
        registrations.validate()?;
        load(pack, registrations, data, options).map_err(|e| ContentError::new(format!("loading Luau content: {e}")))
    }

    /// The VM (for tests and tools).
    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    fn call<R: mlua::FromLuaMulti>(
        &self,
        f: &Function,
        api: &mut dyn CoreApi,
        args: impl mlua::IntoLuaMulti,
    ) -> Result<R, ContentError> {
        let _enter = bind::Enter::new(api, &self.manifest);
        BUDGET.with(|b| b.set(self.budget));
        let result = f.call::<R>(args).map_err(|e| ContentError::new(e.to_string()));
        if self.collect_garbage {
            self.lua.gc_collect().map_err(|e| ContentError::new(e.to_string()))?;
        }
        result
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
        // The state of the action being run, even after the update leaves
        // it (the game's attack variables outlive the action).
        let number = self.manifest.actions[action.0 as usize].action;
        let s = bind::action_state(&self.lua, me, number).map_err(|e| ContentError::new(e.to_string()))?;
        self.call(&self.actions[action.0 as usize], api, (o, s))
    }

    fn call_hook(&self, api: &mut dyn CoreApi, hook: HookId, call: HookCall) -> Result<Value, ContentError> {
        let f = &self.hooks[hook.0 as usize];
        let args = bind::hook_args(&self.lua, call).map_err(|e| ContentError::new(e.to_string()))?;
        let name = self.manifest.hooks[hook.0 as usize].hook;
        let v: LuaValue = self.call(f, api, args)?;
        bind::hook_result(v, call).map_err(|e| ContentError::new(format!("{name}: {e}")))
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
        l.pack
            .modules
            .get(path)
            .cloned()
            .ok_or_else(|| mlua::Error::runtime(format!("no module {path}.luau in the pack")))?
    };
    let bytecode = sandbox::compiler().compile(&source)?;
    verify::check(path, &bytecode).map_err(|v| mlua::Error::runtime(v.to_string()))?;
    let chunk =
        lua.load(&bytecode[..]).set_name(format!("@{path}.luau")).set_mode(ChunkMode::Binary).into_function()?;
    loader.borrow_mut().stack.push(path.to_string());
    let result = chunk.call::<LuaValue>(());
    loader.borrow_mut().stack.pop();
    let value = result?;
    sandbox::deep_freeze(lua, &value)?;
    loader.borrow_mut().loaded.insert(path.to_string(), value.clone());
    Ok(value)
}

/// The pack's data as Luau values.
fn data_value(lua: &Lua, d: &Data) -> mlua::Result<LuaValue> {
    Ok(match d {
        Data::Nil => LuaValue::Nil,
        Data::Bool(b) => LuaValue::Boolean(*b),
        Data::Int(i) => LuaValue::Number(*i as f64),
        Data::Str(s) => LuaValue::String(lua.create_string(s)?),
        Data::List(items) => {
            let t = lua.create_table_with_capacity(items.len(), 0)?;
            for (i, v) in items.iter().enumerate() {
                t.raw_set(i + 1, data_value(lua, v)?)?;
            }
            LuaValue::Table(t)
        }
        Data::Map(entries) => {
            let t = lua.create_table_with_capacity(0, entries.len())?;
            for (k, v) in entries {
                let v = data_value(lua, v)?;
                match k {
                    DataKey::Int(i) => t.raw_set(*i as f64, v)?,
                    DataKey::Str(s) => t.raw_set(s.as_str(), v)?,
                }
            }
            LuaValue::Table(t)
        }
    })
}

fn load(pack: &Pack, registrations: &Registrations, data: &Data, options: Options) -> mlua::Result<LuauContent> {
    let lua = sandbox::new_vm(options.debug_print)?;
    #[cfg(feature = "jit")]
    lua.enable_jit(options.native_code);
    #[cfg(not(feature = "jit"))]
    if options.native_code {
        return Err(mlua::Error::runtime("native code needs bn6-luau's `jit` feature"));
    }
    bind::install(&lua)?;
    let data = data_value(&lua, data)?;
    sandbox::deep_freeze(&lua, &data)?;
    lua.globals().set("data", data)?;
    let loader = Rc::new(RefCell::new(Loader { pack: pack.clone(), loaded: HashMap::new(), stack: Vec::new() }));
    let require = {
        let loader = Rc::downgrade(&loader);
        lua.create_function(move |lua, path: String| {
            let loader = loader
                .upgrade()
                .ok_or_else(|| mlua::Error::runtime("require is only available while content loads"))?;
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

    // Each registered module, loaded as if required from the pack's root.
    let entry = |path: &str| -> mlua::Result<Table> {
        loader.borrow_mut().stack.push("(pack)".into());
        let v = load_module(&lua, &loader, path);
        loader.borrow_mut().stack.clear();
        match v? {
            LuaValue::Table(t) => Ok(t),
            v => Err(mlua::Error::runtime(format!("{path}.luau returned {}, not a table", v.type_name()))),
        }
    };
    let function = |t: &Table, path: &str, name: &str| -> mlua::Result<Function> {
        match t.get::<LuaValue>(name)? {
            LuaValue::Function(f) => Ok(f),
            v => Err(mlua::Error::runtime(format!("{path}.luau: `{name}` is {}, not a function", v.type_name()))),
        }
    };
    let mut manifest = Manifest::default();
    let (mut objects, mut actions, mut hooks) = (Vec::new(), Vec::new(), Vec::new());
    for k in &registrations.kinds {
        let t = entry(&k.module)?;
        objects.push(function(&t, &k.module, "update")?);
        let schema = read_schema(t.get("state")?, &k.module)?;
        manifest.objects.push(ObjectKindDef {
            name: k.name.clone(),
            pool: k.pool,
            index: k.index,
            module: k.module.clone(),
            schema,
        });
    }
    for a in &registrations.actions {
        if manifest.actions.iter().any(|d| d.action == a.action) {
            continue;
        }
        let t = entry(&a.module)?;
        actions.push(function(&t, &a.module, "update")?);
        let schema = read_schema(t.get("state")?, &a.module)?;
        manifest.actions.push(ActionDef { action: a.action, module: a.module.clone(), schema });
    }
    for h in &registrations.hooks {
        if manifest.hooks.iter().any(|d| d.hook == h.hook) {
            continue;
        }
        let t = entry(&h.module)?;
        hooks.push(function(&t, &h.module, h.hook.function())?);
        manifest.hooks.push(HookDef { hook: h.hook, module: h.module.clone() });
    }
    // Nothing a script can reach may change after loading.
    lua.globals().set_readonly(true);
    drop(loader);
    Ok(LuauContent {
        lua,
        manifest,
        objects,
        actions,
        hooks,
        budget: options.budget,
        collect_garbage: options.collect_garbage,
    })
}

/// A module's `state` table: field name to type name (`"u16"`,
/// `"u8[18]"`), or to a list of variant names (an enum). Fields are stored
/// in name order.
fn read_schema(t: Option<Table>, what: &str) -> mlua::Result<Schema> {
    let mut fields = Vec::new();
    if let Some(t) = t {
        for pair in t.pairs::<String, LuaValue>() {
            let (name, ty) = pair?;
            let ty = match ty {
                LuaValue::String(s) => {
                    let s = s.to_str()?;
                    FieldType::scalar(&s).ok_or_else(|| {
                        mlua::Error::runtime(format!("{what}.luau: state field `{name}` has unknown type {:?}", &*s))
                    })?
                }
                LuaValue::Table(variants) => {
                    FieldType::Enum(variants.sequence_values::<String>().collect::<mlua::Result<Vec<_>>>()?)
                }
                v => {
                    return Err(mlua::Error::runtime(format!(
                        "{what}.luau: state field `{name}` is a {}",
                        v.type_name()
                    )));
                }
            };
            fields.push(FieldDef { name, ty });
        }
    }
    fields.sort_by(|a, b| a.name.cmp(&b.name));
    Schema::new(fields).map_err(|e| mlua::Error::runtime(format!("{what}.luau: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_resolve_within_the_pack() {
        assert_eq!(resolve("chips/00f-gundels1/chip", "../../objects/sun-beam/sun_beam").unwrap(), "objects/sun-beam/sun_beam");
        assert_eq!(resolve("(pack)", "./lib/slot").unwrap(), "lib/slot");
        assert!(resolve("lib/slot", "../../x").is_err());
        assert!(resolve("lib/slot", "objects/x").is_err());
    }
}
