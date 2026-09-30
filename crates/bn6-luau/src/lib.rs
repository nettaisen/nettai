//! Luau content for the bn6 battle engine (docs/design/scripting.md,
//! docs/design/content-model-v2.md).
//!
//! A content pack's scripts are Luau modules. Loading runs every module of
//! the pack once (the *define phase*): modules make definitions with
//! `define.<registry>(spec)` (a chip, an object kind, an action...) and
//! return tables of their own. The engine plans what it will call from the
//! definitions and from what the pack's data still registers by module
//! (`bn6_content_api::BindPlan`); the runtime binds those functions:
//!
//! ```luau
//! local bomb = define.kind {
//!     id = "bomb",
//!     pool = "attack",
//!     state = { timer = "u16", slot = { "overlay", "related" } },
//!     update = function(me) ... end,
//! }
//! ```
//!
//! `state` declares the kind's or action's typed state, which the engine
//! stores and snapshots; the functions run on demand and keep nothing
//! themselves. The pack's data is the frozen global `data`.
//!
//! Loading enforces that: modules are checked for writes to globals and to
//! module-level locals (`verify`), everything a module returns or captures
//! and every definition is frozen (`sandbox::deep_freeze`), `require` and
//! `define` work only while loading, and the standard library is cut down
//! to what is deterministic (`sandbox`).

mod bind;
mod define;
pub mod sandbox;
pub mod verify;

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use bn6_content_api::{
    BindPlan, ContentError, ContentHost, CoreApi, Data, DataKey, Definitions, FnId, FnSource, HookCall, Manifest,
    ObjectRef, Registry, StateId, Value,
};
use mlua::chunk::ChunkMode;
use mlua::{Function, Lua, Table, Value as LuaValue, VmState};

/// A content pack's scripts: module path (relative to the pack root,
/// without `.luau`) to source text, and bytecode already compiled from
/// them.
#[derive(Clone, Debug, Default)]
pub struct Pack {
    modules: BTreeMap<String, String>,
    compiled: Compiled,
}

/// Modules' bytecode, each with the source it was compiled from: loading a
/// pack again (a runtime per thread, after the define phase) skips the
/// compiler for every module whose source is unchanged. Bytecode is a pure
/// function of the source (the compiler's options are fixed), and it is
/// still verified when it loads.
#[derive(Clone, Debug, Default)]
pub struct Compiled {
    modules: BTreeMap<String, (Arc<str>, Arc<[u8]>)>,
}

impl Compiled {
    /// Module `path`'s bytecode, if it was compiled from `source`.
    fn get(&self, path: &str, source: &str) -> Option<Arc<[u8]>> {
        let (s, b) = self.modules.get(path)?;
        (**s == *source).then(|| b.clone())
    }

    /// Modules compiled.
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}

impl Pack {
    pub fn new(modules: impl IntoIterator<Item = (String, String)>) -> Pack {
        Pack { modules: modules.into_iter().collect(), compiled: Compiled::default() }
    }

    /// The same pack, with bytecode compiled before (see [`Compiled`]).
    pub fn with_compiled(mut self, compiled: Compiled) -> Pack {
        self.compiled = compiled;
        self
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
        Ok(Pack::new(modules))
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

/// What the binding reads while content runs: the state layouts, and which
/// definition a definition table is.
pub(crate) struct Bound {
    pub manifest: Manifest,
    /// A definition table's registry and handle, by the table's address
    /// (definitions are frozen and live as long as the VM).
    pub defs: HashMap<usize, (Registry, u16)>,
}

impl Bound {
    /// The definition `v` is, if it is one.
    pub fn def(&self, v: &LuaValue) -> Option<(Registry, u16)> {
        match v {
            LuaValue::Table(t) => self.defs.get(&(t.to_pointer() as usize)).copied(),
            _ => None,
        }
    }
}

/// Luau content: the VM and the functions the engine calls.
pub struct LuauContent {
    lua: Lua,
    bound: Bound,
    functions: Vec<Function>,
    /// Where each function came from, for messages.
    sources: Vec<FnSource>,
    budget: u32,
    collect_garbage: bool,
}

impl LuauContent {
    /// Load a pack's modules (the define phase), check they define what
    /// `plan` was made from, and bind the functions `plan` names, with the
    /// pack's data as the global `data`.
    pub fn load(pack: &Pack, plan: &BindPlan, data: &Data, options: Options) -> Result<LuauContent, ContentError> {
        let (lua, defined, modules, _) = open(pack, data, options)?;
        if defined.definitions != plan.definitions {
            return Err(ContentError::new(format!(
                "loading Luau content: the scripts define something other than what the content was made from ({})",
                first_difference(&defined.definitions, &plan.definitions)
            )));
        }
        if plan.handles.len() != defined.tables.len() {
            return Err(ContentError::new("loading Luau content: the plan's handles don't match its definitions"));
        }
        let defs = defined
            .tables
            .iter()
            .zip(&defined.definitions.defs)
            .zip(&plan.handles)
            .map(|((t, d), &h)| (t.to_pointer() as usize, (d.registry, h)))
            .collect();
        let mut functions = Vec::with_capacity(plan.functions.len());
        for source in &plan.functions {
            functions.push(
                resolve_function(source, &defined, &modules)
                    .map_err(|e| ContentError::new(format!("loading Luau content: {e}")))?,
            );
        }
        Ok(LuauContent {
            lua,
            bound: Bound { manifest: Manifest { schemas: plan.schemas.clone() }, defs },
            functions,
            sources: plan.functions.clone(),
            budget: options.budget,
            collect_garbage: options.collect_garbage,
        })
    }

    /// The VM (for tests and tools).
    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    fn call<R: mlua::FromLuaMulti>(
        &self,
        f: FnId,
        api: &mut dyn CoreApi,
        args: impl mlua::IntoLuaMulti,
    ) -> Result<R, ContentError> {
        let _enter = bind::Enter::new(api, &self.bound);
        BUDGET.with(|b| b.set(self.budget));
        let result = self.functions[f.0 as usize].call::<R>(args).map_err(|e| ContentError::new(e.to_string()));
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
        &self.bound.manifest
    }

    fn update_object(&self, api: &mut dyn CoreApi, f: FnId, me: ObjectRef) -> Result<(), ContentError> {
        let o = bind::object(&self.lua, me).map_err(|e| ContentError::new(e.to_string()))?;
        self.call(f, api, o)
    }

    fn update_action(&self, api: &mut dyn CoreApi, f: FnId, me: ObjectRef, state: StateId) -> Result<(), ContentError> {
        let o = bind::object(&self.lua, me).map_err(|e| ContentError::new(e.to_string()))?;
        // The state of the action being run, even after the update leaves
        // it (the game's attack variables outlive the action).
        let s = bind::action_state(&self.lua, me, state).map_err(|e| ContentError::new(e.to_string()))?;
        self.call(f, api, (o, s))
    }

    fn call_hook(&self, api: &mut dyn CoreApi, f: FnId, call: HookCall) -> Result<Value, ContentError> {
        let args = bind::hook_args(&self.lua, call).map_err(|e| ContentError::new(e.to_string()))?;
        let v: LuaValue = self.call(f, api, args)?;
        bind::hook_result(v, call, &self.bound).map_err(|e| ContentError::new(format!("{}: {e}", self.describe(f))))
    }
}

impl LuauContent {
    /// What function `f` is, for messages.
    fn describe(&self, f: FnId) -> String {
        self.sources.get(f.0 as usize).map_or_else(|| format!("function {}", f.0), |s| s.to_string())
    }
}

/// Read a pack's definitions: the define phase, in a VM of its own (which
/// is dropped). The engine makes its content from what this returns, and
/// keeps the modules' bytecode for its runtimes (`Pack::with_compiled`).
pub fn define(pack: &Pack, data: &Data, options: Options) -> Result<(Definitions, Compiled), ContentError> {
    open(pack, data, options).map(|(_, defined, _, compiled)| (defined.definitions, compiled))
}

/// The first place two readings of the definitions differ, for messages.
fn first_difference(a: &Definitions, b: &Definitions) -> String {
    for (x, y) in a.defs.iter().zip(&b.defs) {
        if (x.registry, &x.key) != (y.registry, &y.key) {
            return format!("{} {} against {} {}", x.registry, x.key, y.registry, y.key);
        }
        if x != y {
            return format!("{} {} differs", x.registry, x.key);
        }
    }
    if a.defs.len() != b.defs.len() {
        return format!("{} definitions against {}", a.defs.len(), b.defs.len());
    }
    "the modules' exports differ".to_string()
}

/// Module loading state (only while loading).
struct Loader {
    pack: Pack,
    /// Module results by path.
    loaded: BTreeMap<String, LuaValue>,
    /// What was compiled (or taken from the pack's bytecode) by path.
    compiled: Compiled,
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
    let cached = loader.borrow().pack.compiled.get(path, &source);
    let bytecode: Arc<[u8]> = match cached {
        Some(b) => b,
        None => sandbox::compiler().compile(&source)?.into(),
    };
    verify::check(path, &bytecode).map_err(|v| mlua::Error::runtime(v.to_string()))?;
    loader.borrow_mut().compiled.modules.insert(path.to_string(), (source.as_str().into(), bytecode.clone()));
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
        Data::Nil | Data::Function => LuaValue::Nil,
        Data::Bool(b) => LuaValue::Boolean(*b),
        Data::Int(i) => LuaValue::Number(*i as f64),
        Data::Str(s) => LuaValue::String(lua.create_string(s)?),
        Data::Ref(r, k) => LuaValue::String(lua.create_string(format!("{r}:{k}"))?),
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

/// A VM with the content API, `data`, `define` and `require`, every module
/// of the pack loaded, and the define phase finished.
type Opened = (Lua, define::Defined, BTreeMap<String, LuaValue>, Compiled);

fn open(pack: &Pack, data: &Data, options: Options) -> Result<Opened, ContentError> {
    let err = |e: mlua::Error| ContentError::new(format!("loading Luau content: {e}"));
    let lua = sandbox::new_vm(options.debug_print).map_err(err)?;
    #[cfg(feature = "jit")]
    lua.enable_jit(options.native_code);
    #[cfg(not(feature = "jit"))]
    if options.native_code {
        return Err(ContentError::new("native code needs bn6-luau's `jit` feature"));
    }
    bind::install(&lua).map_err(err)?;
    let data = data_value(&lua, data).map_err(err)?;
    sandbox::deep_freeze(&lua, &data).map_err(err)?;
    lua.globals().set("data", data).map_err(err)?;
    let loader = Rc::new(RefCell::new(Loader {
        pack: pack.clone(),
        loaded: BTreeMap::new(),
        compiled: Compiled::default(),
        stack: Vec::new(),
    }));
    let collector = Rc::new(RefCell::new(define::Collector::open()));
    let module = {
        let loader = Rc::downgrade(&loader);
        Rc::new(move || loader.upgrade().and_then(|l| l.borrow().stack.last().cloned()))
    };
    define::install(&lua, &collector, module).map_err(err)?;
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
        })
        .map_err(err)?
    };
    lua.globals().set("require", require).map_err(err)?;
    // Libraries and globals become read-only; the budget stops runaway
    // loops (also while loading).
    lua.sandbox(true).map_err(err)?;
    lua.set_interrupt(|_| {
        BUDGET.with(|b| match b.get() {
            0 => Err(mlua::Error::runtime("content ran past its budget (a runaway loop?)")),
            n => {
                b.set(n - 1);
                Ok(VmState::Continue)
            }
        })
    });
    // Every module, in path order, as if required from the pack's root.
    let paths: Vec<String> = pack.modules.keys().cloned().collect();
    for path in &paths {
        BUDGET.with(|b| b.set(options.budget.saturating_mul(16)));
        let v = load_module(&lua, &loader, path);
        loader.borrow_mut().stack.clear();
        v.map_err(err)?;
    }
    let modules = std::mem::take(&mut loader.borrow_mut().loaded);
    let compiled = std::mem::take(&mut loader.borrow_mut().compiled);
    drop(loader);
    let defined = define::finish(&lua, &collector, &modules)
        .map_err(|e| ContentError::new(format!("loading Luau content: {e}")))?;
    // Nothing a script can reach may change after loading.
    lua.globals().set_readonly(true);
    Ok((lua, defined, modules, compiled))
}

/// The function a plan names.
fn resolve_function(
    source: &FnSource,
    defined: &define::Defined,
    modules: &BTreeMap<String, LuaValue>,
) -> Result<Function, String> {
    let (table, what, path): (Table, String, &str) = match source {
        FnSource::Export { module, name } => {
            let t = match modules.get(module) {
                Some(LuaValue::Table(t)) => t.clone(),
                Some(v) => return Err(format!("{module}.luau returned {}, not a table", v.type_name())),
                None => return Err(format!("no module {module}.luau in the pack")),
            };
            (t, format!("{module}.luau"), name)
        }
        FnSource::Slot { registry, key, path } => {
            let defs = &defined.definitions.defs;
            let i = defs
                .iter()
                .position(|d| d.registry == *registry && d.key == *key)
                .ok_or_else(|| format!("no {registry} is defined as {key:?}"))?;
            (defined.tables[i].clone(), format!("{registry} {key}"), path)
        }
    };
    let mut at = LuaValue::Table(table);
    for segment in path.split('.') {
        let LuaValue::Table(t) = &at else {
            return Err(format!("{what}: `{path}` is not a function slot"));
        };
        let key = match segment.parse::<i64>() {
            Ok(i) => t.raw_get::<LuaValue>(i),
            Err(_) => t.raw_get::<LuaValue>(segment),
        };
        at = key.map_err(|e| e.to_string())?;
    }
    match at {
        LuaValue::Function(f) => Ok(f),
        v => Err(format!("{what}: `{path}` is {}, not a function", v.type_name())),
    }
}

#[cfg(test)]
mod tests;
