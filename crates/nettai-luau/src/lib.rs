//! Luau content for the nettai battle engine (docs/design/scripting.md,
//! docs/design/content-model-v2.md).
//!
//! A content pack's scripts are Luau modules. Loading runs a game's top
//! module, and what it requires (the *define phase*): it returns the game's
//! root, what a match names, by id, in its sections, and its rules. The
//! loader walks from the root, through tables and what functions capture:
//! what it reaches is what the game has. A table only code reaches says
//! what it is by a tag constructor, `new.<registry>(spec)` (an object kind,
//! an action...). The engine plans what it will call from the definitions
//! (`nettai_content_api::BindPlan`); the runtime binds those functions:
//!
//! ```luau
//! local bomb = new.kind {
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
//! `new` work only while loading, and the standard library is cut down to
//! what is deterministic (`sandbox`).

mod bind;
pub mod coverage;
mod define;
pub mod sandbox;
pub mod verify;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod wasm;

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::sync::Arc;

use nettai_content_api::packs::Modules;
use nettai_content_api::{
    AssetKind, AssetNames, BindPlan, ContentError, ContentHost, CoreApi, Definitions, FnId, FnSource, HookCall, Manifest,
    ObjectRef, Registry, StateId, Value, keys,
};
use mlua::chunk::ChunkMode;
use mlua::{Function, Lua, Table, Value as LuaValue, VmState};

/// Content's scripts: module name to source text, and bytecode already
/// compiled from them. A module's name is its pack's and its path in the
/// pack without `.luau` (`exe6:chips/minibomb/init`; docs/design/
/// content-model-v2.md §4.0); the definitions a module makes are keyed
/// local to the game (`minibomb`).
#[derive(Clone)]
pub struct Pack {
    /// Where a module is read from when a load requires it (`packs::find`):
    /// modules in memory, a content directory's packs, or both.
    modules: Arc<dyn Modules>,
    /// The modules the define phase loads, in order (each game's top
    /// module, its init.luau), which require the rest. (A pack a test makes
    /// of modules alone: every one, in name order.)
    entries: Vec<String>,
    /// The packs the modules are of, by name: what a module may require
    /// (`packs::check_require`); none: any module (a test's modules alone).
    packs: BTreeMap<String, nettai_content_api::PackManifest>,
    compiled: Compiled,
}

impl std::fmt::Debug for Pack {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Pack {{ entries: {:?}, packs: {:?}, compiled: {} }}", self.entries, self.packs.keys().collect::<Vec<_>>(), self.compiled.len())
    }
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

    /// The modules a load read, by name, each with the text it had: what
    /// the define phase's content is made of.
    pub fn sources(&self) -> impl Iterator<Item = (&str, &str)> {
        self.modules.iter().map(|(name, (source, _))| (name.as_str(), &**source))
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}

impl Pack {
    /// Modules in memory, by name: their pack and their path in it
    /// (`exe6:chips/minibomb/init`, `keys::module_name`;
    /// docs/design/content-model-v2.md §4.0). The define phase loads every
    /// one, in name order, unless [`Pack::with_entries`] says where it
    /// starts.
    pub fn new(modules: impl IntoIterator<Item = (String, String)>) -> Pack {
        let modules: BTreeMap<String, String> = modules.into_iter().collect();
        for name in modules.keys() {
            assert!(keys::root_of(name).is_some(), "module {name:?} names no pack (`<pack>:<path>`)");
        }
        let entries = modules.keys().cloned().collect();
        Pack { modules: Arc::new(modules), entries, packs: BTreeMap::new(), compiled: Compiled::default() }
    }

    /// Modules read where `modules` has them, as a load requires them (a
    /// content directory's packs, `packs::Dirs`; modules in memory before
    /// them), the define phase starting from `entries` (each game's top
    /// module, by name).
    pub fn of(modules: impl Modules + 'static, entries: Vec<String>) -> Pack {
        Pack { modules: Arc::new(modules), entries, packs: BTreeMap::new(), compiled: Compiled::default() }
    }

    /// The same pack, the define phase starting from these modules (each
    /// game's top module, by name) and loading what they require.
    pub fn with_entries(mut self, entries: Vec<String>) -> Pack {
        self.entries = entries;
        self
    }

    /// The same pack, its modules' packs these: a require reaches only its
    /// own pack and the support packs it depends on (`packs::check_require`).
    pub fn with_packs(mut self, packs: impl IntoIterator<Item = nettai_content_api::PackManifest>) -> Pack {
        self.packs = packs.into_iter().map(|p| (p.id.clone(), p)).collect();
        self
    }

    /// One folder's modules, by path in the folder (`chips/minibomb/init`).
    pub fn root(name: &str, modules: impl IntoIterator<Item = (String, String)>) -> Pack {
        Pack::new(modules.into_iter().map(|(path, source)| (format!("{name}{}{path}", keys::SEPARATOR), source)))
    }

    /// The same pack, with bytecode compiled before (see [`Compiled`]).
    pub fn with_compiled(mut self, compiled: Compiled) -> Pack {
        self.compiled = compiled;
        self
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
        rules: Option<bind::RulesCtx>,
        args: impl mlua::IntoLuaMulti,
    ) -> Result<R, ContentError> {
        let _enter = bind::Enter::new(api, &self.bound, rules);
        BUDGET.with(|b| b.set(self.budget));
        if coverage::recording() {
            coverage::called(f);
        }
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
        rules: Option<u8>,
    ) -> Result<(), ContentError> {
        let o = bind::object(&self.lua, me).map_err(|e| ContentError::new(e.to_string()))?;
        // The state of the action being run, even after the update leaves
        // it (the game's attack variables outlive the action).
        let s = bind::action_state(&self.lua, me, state).map_err(|e| ContentError::new(e.to_string()))?;
        self.call(f, api, rules.map(|side| bind::RulesCtx { side }), (o, s))
    }

    fn call_hook(&self, api: &mut dyn CoreApi, f: FnId, call: HookCall) -> Result<Value, ContentError> {
        let args = bind::hook_args(&self.lua, call, &self.bound).map_err(|e| ContentError::new(e.to_string()))?;
        let rules = match call {
            HookCall::Rules { side, .. } => Some(bind::RulesCtx { side }),
            _ => None,
        };
        let v: LuaValue = self.call(f, api, rules, args)?;
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
    /// The modules' environments (none until the globals are installed).
    envs: Option<define::Environments>,
    /// The support packs: their modules run in the support environment.
    support: std::collections::HashSet<String>,
}

/// Load module `path` (by name, as `packs::find` or `packs::module` found
/// it): read it where the pack's modules are, compile it and run it, once.
fn load_module(lua: &Lua, loader: &Rc<RefCell<Loader>>, path: &str) -> mlua::Result<LuaValue> {
    let source = {
        let l = loader.borrow();
        if let Some(v) = l.loaded.get(path) {
            return Ok(v.clone());
        }
        if l.stack.iter().any(|p| p == path) {
            let chain: Vec<String> = l.stack.iter().map(|p| format!("{}.luau", keys::module_path(p))).collect();
            return Err(mlua::Error::runtime(format!("require cycle: {} -> {}.luau", chain.join(" -> "), keys::module_path(path))));
        }
        l.pack.modules.read(path).ok_or_else(|| mlua::Error::runtime(format!("no module {}.luau in the content", keys::module_path(path))))?
    };
    let cached = loader.borrow().pack.compiled.get(path, &source);
    let bytecode: Arc<[u8]> = match cached {
        Some(b) => b,
        None => sandbox::compiler().compile(&source)?.into(),
    };
    verify::check(path, &bytecode).map_err(|v| mlua::Error::runtime(v.to_string()))?;
    loader.borrow_mut().compiled.modules.insert(path.to_string(), (source.as_str().into(), bytecode.clone()));
    // (Its pack's environment: a support pack's lacks the game's context.)
    let env = {
        let l = loader.borrow();
        let support = keys::root_of(path).is_some_and(|p| l.support.contains(p));
        l.envs.as_ref().map(|e| if support { e.support.clone() } else { e.game.clone() })
    };
    let mut chunk = lua.load(&bytecode[..]).set_name(format!("@{path}.luau")).set_mode(ChunkMode::Binary);
    if let Some(env) = env {
        chunk = chunk.set_environment(env);
    }
    let chunk = chunk.into_function()?;
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
        envs: None,
        support: pack
            .packs
            .values()
            .filter(|p| p.kind == nettai_content_api::PackKind::Support)
            .map(|p| p.id.clone())
            .collect(),
    }));
    let tags = Rc::new(RefCell::new(define::Tags::open()));
    let module = {
        let loader = Rc::downgrade(&loader);
        Rc::new(move || loader.upgrade().and_then(|l| l.borrow().stack.last().cloned()))
    };
    define::install(&lua, &tags, module.clone()).map_err(err)?;
    let assets = Rc::new(RefCell::new(define::AssetTables::new(&lua, assets.clone()).map_err(err)?));
    define::install_assets(&lua, &assets, module).map_err(err)?;
    let require = {
        let loader = Rc::downgrade(&loader);
        let packs = pack.packs.clone();
        lua.create_function(move |lua, path: String| {
            let loader = loader
                .upgrade()
                .ok_or_else(|| mlua::Error::runtime("require is only available while content loads"))?;
            let from = loader.borrow().stack.last().cloned();
            let from = from.ok_or_else(|| mlua::Error::runtime("require is only available at the top of a module"))?;
            // (Found as it is reached, and read then: `packs::find`.)
            let target = nettai_content_api::packs::find(&*loader.borrow().pack.modules, &packs, &from, &path).map_err(mlua::Error::runtime)?;
            load_module(lua, &loader, &target)
        })
        .map_err(err)?
    };
    lua.globals().set("require", require).map_err(err)?;
    // Each pack's environment (a support pack's without the game's
    // context), every global in place.
    let envs = define::environments(&lua).map_err(err)?;
    loader.borrow_mut().envs = Some(envs);
    // Libraries and globals become read-only; the budget stops runaway
    // loops (also while loading).
    lua.sandbox(true).map_err(err)?;
    lua.set_interrupt(|lua| {
        if coverage::recording() {
            coverage::interrupt(lua);
        }
        BUDGET.with(|b| match b.get() {
            0 => Err(mlua::Error::runtime("content ran past its budget (a runaway loop?)")),
            n => {
                b.set(n - 1);
                Ok(VmState::Continue)
            }
        })
    });
    // Each game's top module, and what it requires (in the order it
    // requires them, which no key depends on); a test's modules alone,
    // every one in name order. What each returns is what the walk starts
    // from: a game's root.
    let mut results = Vec::with_capacity(pack.entries.len());
    for path in &pack.entries {
        BUDGET.with(|b| b.set(options.budget.saturating_mul(16)));
        let v = match nettai_content_api::packs::module(&*pack.modules, path) {
            Some(module) => load_module(&lua, &loader, &module).map(|v| (module, v)),
            None => Err(mlua::Error::runtime(format!("no module {}.luau in the content", keys::module_path(path)))),
        };
        loader.borrow_mut().stack.clear();
        results.push(v.map_err(err)?);
    }
    let modules = std::mem::take(&mut loader.borrow_mut().loaded);
    let compiled = std::mem::take(&mut loader.borrow_mut().compiled);
    drop(loader);
    let assets = Rc::try_unwrap(assets).ok().expect("the resolvers hold the asset tables weakly").into_inner();
    let games: std::collections::HashSet<String> =
        pack.packs.values().filter(|p| p.kind == nettai_content_api::PackKind::Game).map(|p| p.id.clone()).collect();
    let defined = define::finish(&lua, &tags, &assets, &games, &results, &modules)
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
