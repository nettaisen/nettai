//! The battle's content scripts (see docs/design/scripting.md and
//! docs/design/content-model-v2.md): the Luau runtime loaded from the
//! content's scripts ([`Content::scripts`]), and dispatch from the engine to
//! the functions the content implements ([`Content::defs`]: object kinds'
//! updates, actions' updates, hooks). The engine keeps a content kind's
//! declared state in the object (`Vars::Content`) and an action's in the
//! attack state (`ActionVars::Content`), and serves the scripts' calls
//! through [`CoreApi`] (`core_api`).
//!
//! The runtime is not part of a battle: a battle is plain data (`Send`,
//! snapshotted by cloning). Each thread keeps runtimes in a cache keyed by
//! the content's hash, which the battle's setup carries
//! (`RoundSetup::content`); every runtime made from the same content
//! behaves the same, so which one runs a tick doesn't matter.
//! [`with_runtime`] runs code on a runtime of the caller's choosing (one
//! loaded with particular options, a fresh one).

mod core_api;
mod data;

use std::cell::RefCell;
use std::rc::Rc;

use bn6_content_api::{
    ActionHandle, BindPlan, ContentError, ContentHost, ContentState, CoreApi, FnId, HookCall, KindHandle, Manifest,
    Value,
};

use crate::battle::Battle;
use crate::content::{Content, ContentHash};
use crate::kinds::Vars;
use crate::object::{ObjectRef, Pool, Vec3};

pub use bn6_luau::Options;
pub use data::script_data;

/// A loaded content runtime: shared, immutable code (cloning shares it).
#[derive(Clone, Default)]
pub struct Behaviors {
    loaded: Option<Rc<Loaded>>,
}

struct Loaded {
    host: Box<dyn ContentHost>,
}

impl std::fmt::Debug for Behaviors {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Behaviors").field("runtime", &self.runtime()).finish()
    }
}

thread_local! {
    /// This thread's runtimes, by content (most recently used last).
    static CACHE: RefCell<Vec<(ContentHash, Behaviors)>> = const { RefCell::new(Vec::new()) };
    /// Runtimes callers chose ([`with_runtime`]), innermost last.
    static CHOSEN: RefCell<Vec<Behaviors>> = const { RefCell::new(Vec::new()) };
}

/// Runtimes a thread keeps.
const CACHED: usize = 8;

/// What the runtime binds for `content`.
fn plan(content: &Content) -> BindPlan {
    let d = &content.defs;
    BindPlan {
        functions: d.functions.clone(),
        schemas: d.schemas.iter().map(|s| s.schema.clone()).collect(),
        definitions: d.definitions.clone(),
        handles: d.handles.clone(),
    }
}

impl Behaviors {
    /// No scripts.
    pub fn none() -> Behaviors {
        Behaviors { loaded: None }
    }

    /// The content's scripts, loaded once per thread and content (a VM is
    /// a per-thread cache: every VM made from the same content behaves the
    /// same, and none holds battle state).
    pub fn for_content(content: &Content) -> Result<Behaviors, ContentError> {
        Behaviors::cached(content.hash(), content)
    }

    /// The same, for content whose hash the caller has (a battle's setup
    /// carries it).
    fn cached(hash: ContentHash, content: &Content) -> Result<Behaviors, ContentError> {
        let hit = CACHE.with(|c| {
            let mut c = c.borrow_mut();
            if let Some((h, b)) = c.last()
                && *h == hash
            {
                return Some(b.clone());
            }
            let i = c.iter().position(|(h, _)| *h == hash)?;
            let entry = c.remove(i);
            let b = entry.1.clone();
            c.push(entry);
            Some(b)
        });
        if let Some(b) = hit {
            return Ok(b);
        }
        let b = Behaviors::load(content, Options::default())?;
        CACHE.with(|c| {
            let mut c = c.borrow_mut();
            if c.len() == CACHED {
                c.remove(0);
            }
            c.push((hash, b.clone()));
        });
        Ok(b)
    }

    /// Load the content's scripts afresh, with runtime options (native
    /// code, a garbage collection after every call...).
    pub fn load(content: &Content, options: Options) -> Result<Behaviors, ContentError> {
        if !content.defs.defined {
            return Err(ContentError::new("the content isn't defined (Content::define)"));
        }
        if content.defs.functions.is_empty() && content.defs.definitions.is_empty() {
            return Ok(Behaviors::none());
        }
        let pack = bn6_luau::Pack::new(content.scripts.modules.iter().map(|(k, v)| (k.clone(), v.clone())));
        let host = bn6_luau::LuauContent::load(&pack, &plan(content), &script_data(content), options)?;
        Ok(Behaviors { loaded: Some(Rc::new(Loaded { host: Box::new(host) })) })
    }

    /// Which runtime runs the content ("none" without scripts).
    pub fn runtime(&self) -> &str {
        self.loaded.as_ref().map_or("none", |l| l.host.runtime())
    }

    pub fn manifest(&self) -> Option<&Manifest> {
        self.loaded.as_ref().map(|l| l.host.manifest())
    }
}

/// Run `f` with `runtime` running the content of the battles it steps (for
/// runtimes loaded with particular options, or to show a fresh VM changes
/// nothing). The runtime must be of the battles' content.
pub fn with_runtime<R>(runtime: &Behaviors, f: impl FnOnce() -> R) -> R {
    struct Pop;
    impl Drop for Pop {
        fn drop(&mut self) {
            CHOSEN.with(|c| c.borrow_mut().pop());
        }
    }
    CHOSEN.with(|c| c.borrow_mut().push(runtime.clone()));
    let _pop = Pop;
    f()
}

/// The runtime running `b`'s content on this thread.
fn loaded(b: &Battle) -> Rc<Loaded> {
    // A runtime the caller chose runs whatever it was loaded from (tests
    // run edited scripts on a battle this way).
    if let Some(chosen) = CHOSEN.with(|c| c.borrow().last().cloned()) {
        return chosen.loaded.expect("content kinds, actions and hooks come from loaded scripts");
    }
    Behaviors::cached(b.setup.content, &b.content)
        .unwrap_or_else(|e| panic!("{e}"))
        .loaded
        .expect("content kinds, actions and hooks come from loaded scripts")
}

/// Stop the battle on a content error, the same way on every machine.
fn content_error(runtime: &str, what: impl std::fmt::Display, r: impl std::fmt::Debug, e: ContentError) -> ! {
    panic!("{runtime} content error in {what} ({r:?}): {e}")
}

/// Run content object kind `kind` (its update, `f`) for `r`.
pub(crate) fn run_object(b: &mut Battle, kind: KindHandle, f: FnId, r: ObjectRef) {
    let l = loaded(b);
    if let Err(e) = l.host.update_object(b as &mut dyn CoreApi, f, r) {
        let key = b.content.defs.kind(kind).key.clone();
        let source = &b.content.defs.functions[f.0 as usize];
        content_error(l.host.runtime(), format!("kind {key} ({source})"), r, e);
    }
}

/// Run content action `action` for the navi `r`.
pub(crate) fn run_action(b: &mut Battle, action: ActionHandle, r: ObjectRef) {
    let l = loaded(b);
    let a = b.content.defs.action(action);
    let (f, state) = (a.update, a.schema);
    if let Err(e) = l.host.update_action(b as &mut dyn CoreApi, f, r, state) {
        let key = b.content.defs.action(action).key.clone();
        let source = &b.content.defs.functions[f.0 as usize];
        content_error(l.host.runtime(), format!("action {key} ({source})"), r, e);
    }
}

/// Call the content function `f` for a hook.
pub(crate) fn call_hook(b: &mut Battle, f: FnId, call: HookCall) -> Value {
    let l = loaded(b);
    match l.host.call_hook(b as &mut dyn CoreApi, f, call) {
        Ok(v) => v,
        Err(e) => {
            let source = b.content.defs.functions[f.0 as usize].clone();
            content_error(l.host.runtime(), source, call, e)
        }
    }
}

/// Spawn an object; a content kind starts with its zeroed state.
pub fn spawn_object(b: &mut Battle, pool: Pool, index: u8, pos: Vec3, params: [u8; 4]) -> Option<ObjectRef> {
    let r = b.objects.spawn(pool, index, pos, params)?;
    Some(init_state(b, r))
}

/// Spawn an object at the head of the update list (`sub_80033E4`); a
/// content kind starts with its zeroed state.
pub fn spawn_object_first(b: &mut Battle, pool: Pool, index: u8, pos: Vec3, params: [u8; 4]) -> Option<ObjectRef> {
    let r = b.objects.spawn_at_front(pool, index, pos, params)?;
    Some(init_state(b, r))
}

/// `sub_8003374` (attacks) and `sub_800333C` (actors): spawn an object at
/// the end of the update list rather than right after its spawner; a
/// content kind starts with its zeroed state.
pub fn spawn_object_at_end(b: &mut Battle, pool: Pool, index: u8, pos: Vec3, params: [u8; 4]) -> Option<ObjectRef> {
    let r = b.objects.spawn_at_end(pool, index, pos, params)?;
    Some(init_state(b, r))
}

/// A new content object's zeroed state.
fn init_state(b: &mut Battle, r: ObjectRef) -> ObjectRef {
    use crate::content::KindImpl;
    let index = b.objects.get(r).index;
    if let Some(kind) = b.content.defs.kind_at(r.pool, index) {
        let k = b.content.defs.kind(kind);
        if matches!(k.implementation, KindImpl::Script { .. }) {
            let id = k.schema;
            b.objects.get_mut(r).vars = Vars::Content(ContentState::new(id));
        }
    }
    r
}

/// Spawn the content object kind `key` (a v1 kind's folder name): how
/// engine code spawns a kind a script implements. None if the pool is full;
/// panics if no kind has the key, or it fills no object slot.
pub fn spawn_kind(b: &mut Battle, key: &str, pos: Vec3, params: [u8; 4]) -> Option<ObjectRef> {
    let (pool, index) = kind_slot(&b.content, key).unwrap_or_else(|e| panic!("{e}"));
    spawn_object(b, pool, index, pos, params)
}

/// The object slot of kind `key`.
pub(crate) fn kind_slot(content: &Content, key: &str) -> Result<(Pool, u8), String> {
    let h = content.defs.kind_by_key(key).ok_or_else(|| format!("no object kind is named {key:?}"))?;
    content.defs.kind(h).slot.ok_or_else(|| format!("object kind {key} fills no object slot yet (compat)"))
}

/// Set an enum state field of a content object by variant name.
pub fn set_state_variant(b: &mut Battle, r: ObjectRef, name: &str, variant: &str) {
    let content = b.content.clone();
    let Vars::Content(state) = &b.objects.get(r).vars else {
        panic!("{r:?} is not a content object");
    };
    let schema = content.defs.schema(state.id());
    let i = schema.index_of(name).unwrap_or_else(|| panic!("content state has no field `{name}`"));
    let bn6_content_api::FieldType::Enum(names) = &schema.field(i).ty else {
        panic!("content state field `{name}` is not an enum");
    };
    let v = names.iter().position(|n| n == variant).unwrap_or_else(|| panic!("`{name}` has no variant `{variant}`"));
    set_state_field(b, r, name, Value::Int(v as i64));
}

/// Set a state field of a content object by name: how engine code that
/// spawns a content kind passes it arguments.
pub fn set_state_field(b: &mut Battle, r: ObjectRef, name: &str, v: Value) {
    let content = b.content.clone();
    let Vars::Content(state) = &mut b.objects.get_mut(r).vars else {
        panic!("{r:?} is not a content object");
    };
    let schema = content.defs.schema(state.id());
    let i = schema.index_of(name).unwrap_or_else(|| panic!("content state has no field `{name}`"));
    state.set(schema, i, v).unwrap_or_else(|e| panic!("field `{name}`: {e}"));
}

#[cfg(test)]
mod tests;
