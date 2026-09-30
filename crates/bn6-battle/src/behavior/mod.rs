//! The battle's content scripts (see docs/design/scripting.md): the Luau
//! runtime loaded from the content pack's scripts ([`Content::scripts`]),
//! and dispatch from the engine to what the pack registers
//! ([`Content::registrations`]): object kinds by (pool, index), navi
//! actions by number, and hooks (weapon routines, dimming chips' dimming
//! controllers, navi chips' navis) by number. The engine keeps a content
//! kind's declared state in the object (`Vars::Content`) and an action's
//! in the attack state (`ActionVars::Content`), and serves the scripts'
//! calls through [`CoreApi`] (`core_api`).
//!
//! Whatever the pack doesn't register runs as the engine's own Rust.
//!
//! The handle is shared, immutable code: cloning a `Battle` (a snapshot)
//! copies the handle, not the runtime.

mod core_api;
mod data;

use std::rc::Rc;

use bn6_content_api::{ContentError, ContentHost, ContentState, CoreApi, Hook, HookCall, HookId, KindId, Manifest, Value};

use crate::battle::Battle;
use crate::content::{Content, ContentHash};
use crate::kinds::Vars;
use crate::object::{ObjectRef, Pool, Vec3};

pub use bn6_luau::Options;

/// The content scripts a battle runs, with lookup tables from the
/// engine's numbering to them. Without scripts (content that registers
/// none), every kind and action is the engine's own.
#[derive(Clone, Default)]
pub struct Behaviors {
    loaded: Option<Rc<Loaded>>,
}

struct Loaded {
    host: Box<dyn ContentHost>,
    /// Object kind by pool and index.
    objects: [[Option<KindId>; 256]; 3],
    /// Action by number.
    actions: [Option<KindId>; 256],
    /// Hooks by table and number: weapon routines, dimming chips'
    /// controllers, navi chips' navis.
    weapons: [Option<HookId>; 256],
    dimming_chips: [Option<HookId>; 256],
    navi_chips: [Option<HookId>; 256],
    actor_list_entries: [Option<HookId>; 256],
}

impl std::fmt::Debug for Behaviors {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Behaviors").field("runtime", &self.runtime()).finish()
    }
}

impl Behaviors {
    /// No scripts: every kind and action is the engine's own.
    pub fn none() -> Behaviors {
        Behaviors { loaded: None }
    }

    /// The content's scripts, loaded once per thread and content (a VM is
    /// a per-thread cache: every VM made from the same content behaves the
    /// same, and none holds battle state).
    pub fn for_content(content: &Content) -> Result<Behaviors, ContentError> {
        use std::cell::RefCell;
        thread_local!(static CACHE: RefCell<Option<(ContentHash, Behaviors)>> = const { RefCell::new(None) });
        let hash = content.hash();
        CACHE.with(|cell| {
            if let Some((h, b)) = &*cell.borrow()
                && *h == hash
            {
                return Ok(b.clone());
            }
            let b = Behaviors::load(content, Options::default())?;
            *cell.borrow_mut() = Some((hash, b.clone()));
            Ok(b)
        })
    }

    /// Load the content's scripts afresh, with runtime options (native
    /// code, a garbage collection after every call...).
    pub fn load(content: &Content, options: Options) -> Result<Behaviors, ContentError> {
        let registrations = content.registrations().map_err(ContentError::new)?;
        if registrations == Default::default() {
            return Ok(Behaviors::none());
        }
        let pack = bn6_luau::Pack::new(content.scripts.modules.iter().map(|(k, v)| (k.clone(), v.clone())));
        let host = bn6_luau::LuauContent::load(&pack, &registrations, &data::script_data(content), options)?;
        Ok(Behaviors::new(Box::new(host)))
    }

    /// Dispatch to what `host` loaded.
    fn new(host: Box<dyn ContentHost>) -> Behaviors {
        let m = host.manifest();
        let mut objects = [[None; 256]; 3];
        for (i, k) in m.objects.iter().enumerate() {
            objects[k.pool as usize][k.index as usize] = Some(KindId(i as u16));
        }
        let mut actions = [None; 256];
        for (i, a) in m.actions.iter().enumerate() {
            actions[a.action as usize] = Some(KindId(i as u16));
        }
        let (mut weapons, mut dimming_chips, mut navi_chips) = ([None; 256], [None; 256], [None; 256]);
        let mut actor_list_entries = [None; 256];
        for (i, h) in m.hooks.iter().enumerate() {
            let id = Some(HookId(i as u16));
            match h.hook {
                Hook::Weapon(n) => weapons[n as usize] = id,
                Hook::DimmingChip(n) => dimming_chips[n as usize] = id,
                Hook::NaviChip(n) => navi_chips[n as usize] = id,
                Hook::ActorListEntry(n) => actor_list_entries[n as usize] = id,
            }
        }
        let loaded = Loaded { host, objects, actions, weapons, dimming_chips, navi_chips, actor_list_entries };
        Behaviors { loaded: Some(Rc::new(loaded)) }
    }

    /// Which runtime runs the content ("none" without scripts).
    pub fn runtime(&self) -> &str {
        self.loaded.as_ref().map_or("none", |l| l.host.runtime())
    }

    pub fn manifest(&self) -> Option<&Manifest> {
        self.loaded.as_ref().map(|l| l.host.manifest())
    }

    /// The content kind in an object slot, if a script implements it.
    pub fn object_kind(&self, pool: Pool, index: u8) -> Option<KindId> {
        self.loaded.as_ref()?.objects[pool as usize][index as usize]
    }

    /// The content action with this number, if a script implements it.
    pub fn action(&self, action: u8) -> Option<KindId> {
        self.loaded.as_ref()?.actions[action as usize]
    }

    /// The content hook for `hook`, if a script implements it.
    pub fn hook(&self, hook: Hook) -> Option<HookId> {
        let l = self.loaded.as_ref()?;
        match hook {
            Hook::Weapon(n) => l.weapons[n as usize],
            Hook::DimmingChip(n) => l.dimming_chips[n as usize],
            Hook::NaviChip(n) => l.navi_chips[n as usize],
            Hook::ActorListEntry(n) => l.actor_list_entries[n as usize],
        }
    }
}

fn loaded(b: &Battle) -> Rc<Loaded> {
    b.behaviors.loaded.clone().expect("content kinds, actions and hooks come from loaded scripts")
}

/// Stop the battle on a content error, the same way on every machine.
fn content_error(runtime: &str, what: impl std::fmt::Display, r: impl std::fmt::Debug, e: ContentError) -> ! {
    panic!("{runtime} content error in {what} ({r:?}): {e}")
}

/// Run content object `kind` for `r`.
pub(crate) fn run_object(b: &mut Battle, kind: KindId, r: ObjectRef) {
    let l = loaded(b);
    if let Err(e) = l.host.update_object(b as &mut dyn CoreApi, kind, r) {
        let def = &l.host.manifest().objects[kind.0 as usize];
        content_error(l.host.runtime(), format!("objects/{} ({})", def.name, def.module), r, e);
    }
}

/// Run content action `kind` for the navi `r`.
pub(crate) fn run_action(b: &mut Battle, kind: KindId, r: ObjectRef) {
    let l = loaded(b);
    if let Err(e) = l.host.update_action(b as &mut dyn CoreApi, kind, r) {
        let def = &l.host.manifest().actions[kind.0 as usize];
        content_error(l.host.runtime(), format!("action {:#x} ({})", def.action, def.module), r, e);
    }
}

/// Call content hook `hook`.
pub(crate) fn call_hook(b: &mut Battle, hook: HookId, call: HookCall) -> Value {
    let l = loaded(b);
    match l.host.call_hook(b as &mut dyn CoreApi, hook, call) {
        Ok(v) => v,
        Err(e) => {
            let def = &l.host.manifest().hooks[hook.0 as usize];
            content_error(l.host.runtime(), format!("{} ({})", def.hook, def.module), call, e)
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

/// A new content object's zeroed state.
fn init_state(b: &mut Battle, r: ObjectRef) -> ObjectRef {
    let (pool, index) = (r.pool, b.objects.get(r).index);
    if let Some(kind) = b.behaviors.object_kind(pool, index) {
        let m = b.behaviors.manifest().expect("content kinds come from loaded scripts");
        b.objects.get_mut(r).vars = Vars::Content(ContentState::new(m.object_state(kind)));
    }
    r
}

/// Spawn the content object kind named `name` (its folder in the pack):
/// how engine code spawns a kind a script implements. None if the pool is
/// full; panics if no script implements the kind.
pub fn spawn_kind(b: &mut Battle, name: &str, pos: Vec3, params: [u8; 4]) -> Option<ObjectRef> {
    let k = b.content.object_kind(name).unwrap_or_else(|| panic!("no object kind is named {name:?}"));
    let (pool, index) = (k.pool, k.index);
    spawn_object(b, pool, index, pos, params)
}

/// Set an enum state field of a content object by variant name.
pub fn set_state_variant(b: &mut Battle, r: ObjectRef, name: &str, variant: &str) {
    let content = b.behaviors.clone();
    let m = content.manifest().expect("content state belongs to loaded scripts");
    let Vars::Content(state) = &b.objects.get(r).vars else {
        panic!("{r:?} is not a content object");
    };
    let schema = m.schema(state.id());
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
    let content = b.behaviors.clone();
    let m = content.manifest().expect("content state belongs to loaded scripts");
    let Vars::Content(state) = &mut b.objects.get_mut(r).vars else {
        panic!("{r:?} is not a content object");
    };
    let schema = m.schema(state.id());
    let i = schema.index_of(name).unwrap_or_else(|| panic!("content state has no field `{name}`"));
    state.set(schema, i, v).unwrap_or_else(|e| panic!("field `{name}`: {e}"));
}

#[cfg(test)]
mod tests;
