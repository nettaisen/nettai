//! Luau content for the nettai battle engine (docs/design/scripting.md,
//! docs/design/content-model-v2.md).
//!
//! A content pack's scripts are Luau modules. Loading runs every module of
//! the pack once (the *define phase*): modules make definitions with
//! `define.<registry>(spec)` (a chip, an object kind, an action...) and
//! return tables of their own. The engine plans what it will call from the
//! definitions (`nettai_content_api::BindPlan`); the runtime binds those
//! functions:
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
//! themselves. What modules share they `require`.
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

use nettai_content_api::{
    AssetKind, AssetNames, BindPlan, ContentError, ContentHost, CoreApi, Definitions, FnId, FnSource, HookCall, Manifest,
    ObjectRef, Registry, StateId, Value, keys,
};
use mlua::chunk::ChunkMode;
use mlua::{Function, Lua, Table, Value as LuaValue, VmState};

/// Content's scripts: module name to source text, and bytecode already
/// compiled from them. A module's name is its root's and its path in the
/// root without `.luau` (`bn6:chips/minibomb/chip`, docs/design/
/// rules-in-luau.md §7.2); the definitions a module makes are its root's
/// (`bn6:minibomb`).
#[derive(Clone, Debug, Default)]
pub struct Pack {
    modules: BTreeMap<String, String>,
    /// Each root's `requires`: the roots its modules may `require` from
    /// (`require("@bn6/rules/beast/system")`).
    roots: BTreeMap<String, Vec<String>>,
    /// Each root's assets pack (its game; by default the root's name): what
    /// an unqualified asset name of its modules is.
    assets: BTreeMap<String, String>,
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
    /// Modules by name (`bn6:chips/minibomb/chip`). Each module's root is
    /// one of the pack's, which requires no other until
    /// [`Pack::with_requires`] says so.
    pub fn new(modules: impl IntoIterator<Item = (String, String)>) -> Pack {
        let modules: BTreeMap<String, String> = modules.into_iter().collect();
        let mut roots = BTreeMap::new();
        for name in modules.keys() {
            let root = keys::root_of(name).unwrap_or_else(|| panic!("module {name:?} names no root (`<root>:<path>`)"));
            roots.entry(root.to_string()).or_insert_with(Vec::new);
        }
        Pack { modules, roots, assets: BTreeMap::new(), compiled: Compiled::default() }
    }

    /// One root's modules, by path in the root (`chips/minibomb/chip`).
    pub fn root(name: &str, modules: impl IntoIterator<Item = (String, String)>) -> Pack {
        let mut p = Pack::new(modules.into_iter().map(|(path, source)| (format!("{name}{}{path}", keys::SEPARATOR), source)));
        p.roots.entry(name.to_string()).or_default();
        p
    }

    /// Root `root` may require from `requires` (its manifest's).
    pub fn with_requires(mut self, root: &str, requires: Vec<String>) -> Pack {
        self.roots.insert(root.to_string(), requires);
        self
    }

    /// Root `root`'s asset names resolve in pack `game` (its manifest's
    /// `assets`).
    pub fn with_assets(mut self, root: &str, game: &str) -> Pack {
        self.assets.insert(root.to_string(), game.to_string());
        self
    }

    /// The pack an asset name of root `root`'s modules resolves in.
    fn assets_of(&self, root: &str) -> String {
        self.assets.get(root).cloned().unwrap_or_else(|| root.to_string())
    }

    /// Asset name `name` as a module of root `root` means it: an
    /// unqualified name its own pack's; a qualified one (`bn6:bomb`) only of
    /// its own pack or a pack of a root it requires.
    pub fn asset_name(&self, root: &str, name: &str) -> Result<String, String> {
        let own = self.assets_of(root);
        match keys::root_of(name) {
            None => Ok(keys::qualify(&own, name)),
            Some(game) => {
                let allowed = game == own
                    || self.roots.get(root).is_some_and(|r| r.iter().any(|q| self.assets_of(q) == game));
                if allowed {
                    Ok(name.to_string())
                } else {
                    Err(format!("{name:?}: root {root} names only its own pack's assets ({own}) and those of the roots it requires"))
                }
            }
        }
    }

    /// The same pack, with bytecode compiled before (see [`Compiled`]).
    pub fn with_compiled(mut self, compiled: Compiled) -> Pack {
        self.compiled = compiled;
        self
    }

    /// Every `.luau` file under `dir` (definition files, `.d.luau`,
    /// excluded), as root `name`.
    pub fn from_dir(name: &str, dir: &Path) -> std::io::Result<Pack> {
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
        Ok(Pack::root(name, modules))
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

/// What the binding reads while content runs: the state layouts, which
/// definition a definition table is and the other way round, and the asset
/// values (docs/design/content-model-v2.md §2.4, §6.3).
pub(crate) struct Bound {
    pub manifest: Manifest,
    /// A definition table's registry and handle, by the table's address
    /// (definitions are frozen and live as long as the VM).
    defs: HashMap<usize, (Registry, u16)>,
    /// Each definition's table by registry and handle; for an entry that is
    /// no definition (an engine kind), a stand-in `{ id = key }`.
    tables: HashMap<(Registry, u16), Table>,
    /// Records' types, by handle.
    record_types: HashMap<u16, String>,
    assets: RefCell<define::AssetTables>,
}

impl Bound {
    /// The definition `v` is, if it is one.
    pub fn def(&self, v: &LuaValue) -> Option<(Registry, u16)> {
        match v {
            LuaValue::Table(t) => self.defs.get(&(t.to_pointer() as usize)).copied(),
            _ => None,
        }
    }

    /// Definition `h` of `registry` as a script value.
    pub fn def_value(&self, registry: Registry, h: u16) -> mlua::Result<Table> {
        self.tables.get(&(registry, h)).cloned().ok_or_else(|| mlua::Error::runtime(format!("no {registry} has handle {h}")))
    }

    /// Record `h`'s type.
    pub fn record_type(&self, h: u16) -> Option<&str> {
        self.record_types.get(&h).map(String::as_str)
    }

    /// The asset `v` is, if it is one.
    pub fn asset(&self, v: &LuaValue) -> Option<(AssetKind, u16)> {
        self.assets.borrow().asset(v)
    }

    /// Asset `h` of `kind` as a script value.
    pub fn asset_value(&self, lua: &Lua, kind: AssetKind, h: u16) -> mlua::Result<Table> {
        self.assets.borrow_mut().value(lua, kind, h)
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
    /// `plan` was made from, and bind the functions `plan` names.
    pub fn load(pack: &Pack, plan: &BindPlan, options: Options) -> Result<LuauContent, ContentError> {
        let (lua, defined, _, _, assets) = open(pack, &plan.assets, options)?;
        if defined.definitions != plan.definitions {
            return Err(ContentError::new(format!(
                "loading Luau content: the scripts define something other than what the content was made from ({})",
                first_difference(&defined.definitions, &plan.definitions)
            )));
        }
        if plan.handles.len() != defined.tables.len() {
            return Err(ContentError::new("loading Luau content: the plan's handles don't match its definitions"));
        }
        let err = |e: mlua::Error| ContentError::new(format!("loading Luau content: {e}"));
        let mut defs = HashMap::new();
        let mut tables = HashMap::new();
        let mut record_types = HashMap::new();
        for ((t, d), &h) in defined.tables.iter().zip(&defined.definitions.defs).zip(&plan.handles) {
            defs.insert(t.to_pointer() as usize, (d.registry, h));
            tables.insert((d.registry, h), t.clone());
            if let Some(ty) = &d.record_type {
                record_types.insert(h, ty.clone());
            }
        }
        // Stand-ins for the entries that are no definition.
        for (registry, h, key) in &plan.entries {
            let t = lua.create_table().map_err(err)?;
            t.raw_set("id", key.as_str()).map_err(err)?;
            let meta = lua.create_table().map_err(err)?;
            meta.raw_set("__metatable", format!("{registry} definition")).map_err(err)?;
            meta.set_readonly(true);
            t.set_metatable(Some(meta)).map_err(err)?;
            t.set_readonly(true);
            defs.insert(t.to_pointer() as usize, (*registry, *h));
            tables.insert((*registry, *h), t);
        }
        let mut functions = Vec::with_capacity(plan.functions.len());
        for source in &plan.functions {
            functions.push(
                resolve_function(source, &defined)
                    .map_err(|e| ContentError::new(format!("loading Luau content: {e}")))?,
            );
        }
        Ok(LuauContent {
            lua,
            bound: Bound {
                manifest: Manifest { schemas: plan.schemas.clone() },
                defs,
                tables,
                record_types,
                assets: RefCell::new(assets),
            },
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
        system: Option<bind::SystemCtx>,
        args: impl mlua::IntoLuaMulti,
    ) -> Result<R, ContentError> {
        let _enter = bind::Enter::new(api, &self.bound, system);
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
        self.call(f, api, None, o)
    }

    fn update_action(
        &self,
        api: &mut dyn CoreApi,
        f: FnId,
        me: ObjectRef,
        state: StateId,
        system: Option<(u8, u8)>,
    ) -> Result<(), ContentError> {
        let o = bind::object(&self.lua, me).map_err(|e| ContentError::new(e.to_string()))?;
        // The state of the action being run, even after the update leaves
        // it (the game's attack variables outlive the action).
        let s = bind::action_state(&self.lua, me, state).map_err(|e| ContentError::new(e.to_string()))?;
        let system = system.map(|(side, slot)| bind::SystemCtx { side, slot });
        self.call(f, api, system, (o, s))
    }

    fn call_hook(&self, api: &mut dyn CoreApi, f: FnId, call: HookCall) -> Result<Value, ContentError> {
        let args = bind::hook_args(&self.lua, call, &self.bound).map_err(|e| ContentError::new(e.to_string()))?;
        let system = match call {
            HookCall::System { side, slot, .. } => Some(bind::SystemCtx { side, slot }),
            _ => None,
        };
        let v: LuaValue = self.call(f, api, system, args)?;
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
pub fn define(pack: &Pack, assets: &AssetNames, options: Options) -> Result<(Definitions, Compiled), ContentError> {
    open(pack, assets, options).map(|(_, defined, _, compiled, _)| (defined.definitions, compiled))
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
    "nothing".to_string()
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

/// Resolve a `require` path from module `from` (`bn6:rules/beast/system`):
/// relative to its directory within its root (`./rush`, `../lib/slot`), or
/// in a root it requires, or its own, from that root's top
/// (`@bn6/rules/cross/system`). `roots` gives each root's `requires`.
fn resolve(from: &str, path: &str, roots: &BTreeMap<String, Vec<String>>) -> Result<String, String> {
    let (root, from_path) = from.split_once(keys::SEPARATOR).ok_or_else(|| format!("module {from:?} names no root"))?;
    if let Some(rest) = path.strip_prefix('@') {
        let (target, p) = rest.split_once('/').ok_or_else(|| format!("require({path:?}): a root's module is `@<root>/<path>`"))?;
        let p = p.trim_end_matches(".luau");
        if p.split('/').any(|s| s.is_empty() || s == "." || s == "..") {
            return Err(format!("require({path:?}): a path from a root's top names its directories"));
        }
        if target != root && !roots.get(root).is_some_and(|r| r.iter().any(|x| x == target)) {
            return Err(format!(
                "require({path:?}) from {from}: root {root} doesn't require root {target} (its manifest's `requires`)"
            ));
        }
        return Ok(format!("{target}{}{p}", keys::SEPARATOR));
    }
    if !(path.starts_with("./") || path.starts_with("../")) {
        return Err(format!("require({path:?}): content paths start with ./, ../ or @<root>/"));
    }
    let mut parts: Vec<&str> = from_path.split('/').collect();
    parts.pop();
    for seg in path.trim_end_matches(".luau").split('/') {
        match seg {
            "." | "" => {}
            ".." => {
                parts.pop().ok_or_else(|| format!("require({path:?}) from {from} leaves root {root}"))?;
            }
            s => parts.push(s),
        }
    }
    Ok(format!("{root}{}{}", keys::SEPARATOR, parts.join("/")))
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
            .ok_or_else(|| mlua::Error::runtime(format!("no module {path}.luau in the content")))?
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

/// A VM with the content API, `define` and `require`, every module of the
/// pack loaded, and the define phase finished.
type Opened = (Lua, define::Defined, BTreeMap<String, LuaValue>, Compiled, define::AssetTables);

fn open(pack: &Pack, assets: &AssetNames, options: Options) -> Result<Opened, ContentError> {
    let err = |e: mlua::Error| ContentError::new(format!("loading Luau content: {e}"));
    let lua = sandbox::new_vm(options.debug_print).map_err(err)?;
    #[cfg(feature = "jit")]
    lua.enable_jit(options.native_code);
    #[cfg(not(feature = "jit"))]
    if options.native_code {
        return Err(ContentError::new("native code needs nettai-luau's `jit` feature"));
    }
    bind::install(&lua).map_err(err)?;
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
    define::install(&lua, &collector, module.clone()).map_err(err)?;
    let assets = Rc::new(RefCell::new(define::AssetTables::new(&lua, assets.clone()).map_err(err)?));
    define::install_assets(&lua, &assets, module, Rc::new(pack.clone())).map_err(err)?;
    let require = {
        let loader = Rc::downgrade(&loader);
        lua.create_function(move |lua, path: String| {
            let loader = loader
                .upgrade()
                .ok_or_else(|| mlua::Error::runtime("require is only available while content loads"))?;
            let from = loader.borrow().stack.last().cloned();
            let from = from.ok_or_else(|| mlua::Error::runtime("require is only available at the top of a module"))?;
            let target = resolve(&from, &path, &loader.borrow().pack.roots).map_err(mlua::Error::runtime)?;
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
    let assets = Rc::try_unwrap(assets).ok().expect("the resolvers hold the asset tables weakly").into_inner();
    let defined = define::finish(&lua, &collector, &assets)
        .map_err(|e| ContentError::new(format!("loading Luau content: {e}")))?;
    // Nothing a script can reach may change after loading.
    lua.globals().set_readonly(true);
    Ok((lua, defined, modules, compiled, assets))
}

/// The function a plan names.
fn resolve_function(
    source: &FnSource,
    defined: &define::Defined,
) -> Result<Function, String> {
    let FnSource { registry, key, path } = source;
    let defs = &defined.definitions.defs;
    let i = defs
        .iter()
        .position(|d| d.registry == *registry && d.key == *key)
        .ok_or_else(|| format!("no {registry} is defined as {key:?}"))?;
    let what = format!("{registry} {key}");
    let mut at = LuaValue::Table(defined.tables[i].clone());
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
