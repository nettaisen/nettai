//! Content hosted by a runtime (see docs/design/scripting.md): object kinds
//! and navi actions whose behavior comes from a [`ContentHost`] instead of
//! `kinds`. The engine dispatches to it by (pool, index) and by action
//! number, keeps the content's declared state in the objects
//! (`Vars::Content`) and the attack state (`ActionVars::Content`), and
//! serves its calls through [`CoreApi`] (`core_api`).
//!
//! The content handle is shared, immutable code: cloning a `Battle` (a
//! snapshot) copies the handle, not the runtime.

mod core_api;

use std::rc::Rc;

use bn6_content_api::{ContentError, ContentHost, ContentState, CoreApi, KindId, Manifest, Value};

use crate::battle::Battle;
use crate::kinds::Vars;
use crate::object::{ObjectRef, Pool, Vec3};

/// The content a battle runs: a runtime's object kinds and actions, with
/// lookup tables from engine slots to them. The built-in content has none
/// (every kind is the engine's own).
#[derive(Clone, Default)]
pub struct Content {
    loaded: Option<Rc<Loaded>>,
}

struct Loaded {
    host: Box<dyn ContentHost>,
    /// Object kind by pool and index.
    objects: [[Option<KindId>; 256]; 3],
    /// Action by number.
    actions: [Option<KindId>; 256],
}

impl std::fmt::Debug for Content {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Content").field("runtime", &self.runtime()).finish()
    }
}

impl Content {
    /// Only the engine's built-in kinds.
    pub fn builtin() -> Content {
        Content { loaded: None }
    }

    /// The kinds and actions `host` defines, taking over those slots.
    pub fn new(host: impl ContentHost + 'static) -> Result<Content, ContentError> {
        let m = host.manifest();
        m.validate()?;
        let mut objects = [[None; 256]; 3];
        for (i, k) in m.objects.iter().enumerate() {
            objects[k.pool as usize][k.index as usize] = Some(KindId(i as u16));
        }
        let mut actions = [None; 256];
        for (i, a) in m.actions.iter().enumerate() {
            actions[a.action as usize] = Some(KindId(i as u16));
        }
        Ok(Content { loaded: Some(Rc::new(Loaded { host: Box::new(host), objects, actions })) })
    }

    /// The content this build runs by default: the Luau content pack with
    /// the `luau` feature, else the Rust content with `rust-content`, else
    /// the built-in kinds. Loaded once per thread.
    pub fn for_build() -> Content {
        #[cfg(feature = "luau")]
        {
            thread_local!(static LUAU: Content = Content::luau().unwrap_or_else(|e| panic!("{e}")));
            LUAU.with(Content::clone)
        }
        #[cfg(all(feature = "rust-content", not(feature = "luau")))]
        {
            Content::rust()
        }
        #[cfg(not(any(feature = "luau", feature = "rust-content")))]
        {
            Content::builtin()
        }
    }

    /// The GunDelSol slice written in Rust against the content API.
    #[cfg(feature = "rust-content")]
    pub fn rust() -> Content {
        Content::new(bn6_content_rust::RustContent::new()).expect("the Rust content is consistent")
    }

    /// The Luau content pack in content/bn6 (compiled into the binary).
    #[cfg(feature = "luau")]
    pub fn luau() -> Result<Content, ContentError> {
        Content::luau_with(bn6_luau::Options::default())
    }

    /// The Luau content pack, with runtime options (e.g. native code).
    #[cfg(feature = "luau")]
    pub fn luau_with(options: bn6_luau::Options) -> Result<Content, ContentError> {
        Content::new(bn6_luau::LuauContent::load(&luau_pack(), options)?)
    }

    /// Which runtime runs the content ("builtin" for none).
    pub fn runtime(&self) -> &str {
        self.loaded.as_ref().map_or("builtin", |l| l.host.runtime())
    }

    pub fn manifest(&self) -> Option<&Manifest> {
        self.loaded.as_ref().map(|l| l.host.manifest())
    }

    /// The content kind in an object slot, if content defines it.
    pub fn object_kind(&self, pool: Pool, index: u8) -> Option<KindId> {
        self.loaded.as_ref()?.objects[pool as usize][index as usize]
    }

    /// The content action with this number, if content defines it.
    pub fn action(&self, action: u8) -> Option<KindId> {
        self.loaded.as_ref()?.actions[action as usize]
    }
}

/// The content pack under content/bn6, compiled in.
#[cfg(feature = "luau")]
pub fn luau_pack() -> bn6_luau::Pack {
    macro_rules! pack {
        ($($path:literal),* $(,)?) => {
            bn6_luau::Pack::new(vec![
                $(($path.to_string(), include_str!(concat!("../../../../content/bn6/", $path, ".luau")).to_string()),)*
            ])
        };
    }
    pack![
        "pack",
        "data/attacks",
        "lib/slot",
        "objects/attachment",
        "objects/sun_beam",
        "objects/hitbox",
        "chips/gun_del_sol"
    ]
}

/// Run content object `kind` for `r`.
pub(crate) fn run_object(b: &mut Battle, kind: KindId, r: ObjectRef) {
    let content = b.content.clone();
    let loaded = content.loaded.as_ref().expect("content kinds come from loaded content");
    if let Err(e) = loaded.host.update_object(b as &mut dyn CoreApi, kind, r) {
        let name = &loaded.host.manifest().objects[kind.0 as usize].name;
        panic!("{} content error in {name} ({r:?}): {e}", loaded.host.runtime());
    }
}

/// Run content action `kind` for the navi `r`.
pub(crate) fn run_action(b: &mut Battle, kind: KindId, r: ObjectRef) {
    let content = b.content.clone();
    let loaded = content.loaded.as_ref().expect("content actions come from loaded content");
    if let Err(e) = loaded.host.update_action(b as &mut dyn CoreApi, kind, r) {
        let name = &loaded.host.manifest().actions[kind.0 as usize].name;
        panic!("{} content error in {name} ({r:?}): {e}", loaded.host.runtime());
    }
}

/// Spawn an object; a content kind starts with its zeroed state.
pub fn spawn_object(b: &mut Battle, pool: Pool, index: u8, pos: Vec3, params: [u8; 4]) -> Option<ObjectRef> {
    let r = b.objects.spawn(pool, index, pos, params)?;
    if let Some(kind) = b.content.object_kind(pool, index) {
        let m = b.content.manifest().expect("content kinds come from loaded content");
        let id = m.object_state(kind);
        b.objects.get_mut(r).vars = Vars::Content(ContentState::new(id, m.schema(id)));
    }
    Some(r)
}

/// Set an enum state field of a content object by variant name.
pub fn set_state_variant(b: &mut Battle, r: ObjectRef, name: &str, variant: &str) {
    let content = b.content.clone();
    let m = content.manifest().expect("content state belongs to loaded content");
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
    let content = b.content.clone();
    let m = content.manifest().expect("content state belongs to loaded content");
    let Vars::Content(state) = &mut b.objects.get_mut(r).vars else {
        panic!("{r:?} is not a content object");
    };
    let schema = m.schema(state.id());
    let i = schema.index_of(name).unwrap_or_else(|| panic!("content state has no field `{name}`"));
    state.set(schema, i, v).unwrap_or_else(|e| panic!("field `{name}`: {e}"));
}

#[cfg(test)]
mod tests;
