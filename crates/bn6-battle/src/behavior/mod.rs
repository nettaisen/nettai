//! Behaviors hosted by a runtime (see docs/design/scripting.md): object kinds
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
use crate::content::Content;
use crate::kinds::Vars;
use crate::object::{ObjectRef, Pool, Vec3};

/// The content a battle runs: a runtime's object kinds and actions, with
/// lookup tables from engine slots to them. The built-in content has none
/// (every kind is the engine's own).
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
}

impl std::fmt::Debug for Behaviors {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Behaviors").field("runtime", &self.runtime()).finish()
    }
}

impl Behaviors {
    /// Only the engine's built-in kinds.
    pub fn builtin() -> Behaviors {
        Behaviors { loaded: None }
    }

    /// The kinds and actions `host` defines, taking over those slots.
    pub fn new(host: impl ContentHost + 'static) -> Result<Behaviors, ContentError> {
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
        Ok(Behaviors { loaded: Some(Rc::new(Loaded { host: Box::new(host), objects, actions })) })
    }

    /// The behaviors this build runs by default on `content`: the Luau
    /// scripts with the `luau` feature, else the Rust content with
    /// `rust-content`, else the built-in kinds. The Luau scripts are loaded
    /// once per thread and content.
    pub fn for_build(content: &Content) -> Behaviors {
        #[cfg(feature = "luau")]
        {
            use std::cell::RefCell;
            thread_local!(static LUAU: RefCell<Option<(crate::content::ContentHash, Behaviors)>> = const { RefCell::new(None) });
            let hash = content.hash();
            LUAU.with(|cell| {
                let mut cell = cell.borrow_mut();
                match &*cell {
                    Some((h, b)) if *h == hash => b.clone(),
                    _ => {
                        let b = Behaviors::luau(content).unwrap_or_else(|e| panic!("{e}"));
                        *cell = Some((hash, b.clone()));
                        b
                    }
                }
            })
        }
        #[cfg(all(feature = "rust-content", not(feature = "luau")))]
        {
            Behaviors::rust(content)
        }
        #[cfg(not(any(feature = "luau", feature = "rust-content")))]
        {
            let _ = content;
            Behaviors::builtin()
        }
    }

    /// The GunDelSol slice written in Rust against the content API, with
    /// its data from `content`.
    #[cfg(feature = "rust-content")]
    pub fn rust(content: &Content) -> Behaviors {
        Behaviors::new(bn6_content_rust::RustContent::new(rust_data(content))).expect("the Rust content is consistent")
    }

    /// The Luau scripts in content/bn6 (compiled into the binary), with
    /// their data from `content`.
    #[cfg(feature = "luau")]
    pub fn luau(content: &Content) -> Result<Behaviors, ContentError> {
        Behaviors::luau_with(content, bn6_luau::Options::default())
    }

    /// The Luau scripts, with runtime options (e.g. native code).
    #[cfg(feature = "luau")]
    pub fn luau_with(content: &Content, options: bn6_luau::Options) -> Result<Behaviors, ContentError> {
        Behaviors::new(bn6_luau::LuauContent::load(&luau_pack(content), options)?)
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

/// The Luau scripts under content/bn6 (compiled in), with their data
/// module, `data/pack`, built from `content` (the repository only has its
/// type stub).
#[cfg(feature = "luau")]
pub fn luau_pack(content: &Content) -> bn6_luau::Pack {
    macro_rules! pack {
        ($($path:literal),* $(,)?) => {
            bn6_luau::Pack::new(vec![
                ("data/pack".to_string(), luau_data(content)),
                $(($path.to_string(), include_str!(concat!("../../../../content/bn6/", $path, ".luau")).to_string()),)*
            ])
        };
    }
    pack![
        "pack",
        "lib/slot",
        "objects/attachment",
        "objects/sun_beam",
        "objects/hitbox",
        "chips/gun_del_sol"
    ]
}

/// The scripts' data module (`data/pack`, typed by its stub in
/// content/bn6): each chip's own data, the attachment kinds and the sun
/// beam's looks, from `content`.
#[cfg(feature = "luau")]
pub fn luau_data(content: &Content) -> String {
    use crate::content::{AttachmentKind, SunBeamLook};
    let sprite = |s: crate::content::SpriteId| format!("{{ category = {:#04x}, index = {:#04x} }}", s.category, s.index);
    let attachment = |k: &AttachmentKind| {
        let point = k.attach_point.map_or("nil".to_string(), |p| p.to_string());
        format!(
            "{{ id = {:#04x}, sprite = {}, palette = {}, lift = {}, attach_point = {point} }}",
            k.id,
            sprite(k.sprite),
            k.palette,
            k.lift
        )
    };
    let look = |l: SunBeamLook| format!("{{ look = {}, palette = {} }}", l.look, l.palette);
    let mut s = String::from("--!strict\n-- Built from the content pack by the engine; see the stub in content/bn6/data/pack.luau.\n\nreturn {\n    chips = {\n");
    for c in content.chips.iter().filter(|c| c.gun_del_sol.is_some()) {
        let g = c.gun_del_sol.as_ref().expect("filtered");
        s += &format!(
            "        [{:#05x}] = {{ gun_del_sol = {{ firing_ticks = {}, beam = {}, beam_in_sun = {}, gun = {} }} }},\n",
            c.id,
            g.firing_ticks,
            look(g.beam),
            look(g.beam_in_sun),
            attachment(&g.gun)
        );
    }
    s += "    } :: { [number]: ChipData },\n    attachments = {\n";
    for k in &content.objects.attachments {
        s += &format!("        [{:#04x}] = {},\n", k.id, attachment(k));
    }
    s += "    } :: { [number]: AttachmentKind },\n    sun_beam_looks = {\n";
    for (i, &id) in content.objects.sun_beam_looks.iter().enumerate() {
        s += &format!("        [{i}] = {},\n", sprite(id));
    }
    s += "    } :: { [number]: SpriteId },\n}\n";
    s
}

/// The Rust content's data, from `content`.
#[cfg(feature = "rust-content")]
fn rust_data(content: &Content) -> bn6_content_rust::data::Data {
    use bn6_content_rust::data as rc;
    let attachment = |k: &crate::content::AttachmentKind| rc::AttachmentKind {
        id: k.id,
        sprite: k.sprite,
        palette: k.palette,
        lift: k.lift,
        attach_point: k.attach_point,
    };
    let look = |l: crate::content::SunBeamLook| rc::SunBeamLook { look: l.look, palette: l.palette };
    rc::Data {
        gun_del_sol: content
            .chips
            .iter()
            .filter_map(|c| {
                let g = c.gun_del_sol.as_ref()?;
                let d = rc::GunDelSol {
                    firing_ticks: g.firing_ticks,
                    beam: look(g.beam),
                    beam_in_sun: look(g.beam_in_sun),
                    gun: attachment(&g.gun),
                };
                Some((c.id, d))
            })
            .collect(),
        attachments: content.objects.attachments.iter().map(attachment).collect(),
        sun_beam_looks: content.objects.sun_beam_looks.clone(),
    }
}

/// Run content object `kind` for `r`.
pub(crate) fn run_object(b: &mut Battle, kind: KindId, r: ObjectRef) {
    let content = b.behaviors.clone();
    let loaded = content.loaded.as_ref().expect("content kinds come from loaded content");
    if let Err(e) = loaded.host.update_object(b as &mut dyn CoreApi, kind, r) {
        let name = &loaded.host.manifest().objects[kind.0 as usize].name;
        panic!("{} content error in {name} ({r:?}): {e}", loaded.host.runtime());
    }
}

/// Run content action `kind` for the navi `r`.
pub(crate) fn run_action(b: &mut Battle, kind: KindId, r: ObjectRef) {
    let content = b.behaviors.clone();
    let loaded = content.loaded.as_ref().expect("content actions come from loaded content");
    if let Err(e) = loaded.host.update_action(b as &mut dyn CoreApi, kind, r) {
        let name = &loaded.host.manifest().actions[kind.0 as usize].name;
        panic!("{} content error in {name} ({r:?}): {e}", loaded.host.runtime());
    }
}

/// Spawn an object; a content kind starts with its zeroed state.
pub fn spawn_object(b: &mut Battle, pool: Pool, index: u8, pos: Vec3, params: [u8; 4]) -> Option<ObjectRef> {
    let r = b.objects.spawn(pool, index, pos, params)?;
    if let Some(kind) = b.behaviors.object_kind(pool, index) {
        let m = b.behaviors.manifest().expect("content kinds come from loaded content");
        let id = m.object_state(kind);
        b.objects.get_mut(r).vars = Vars::Content(ContentState::new(id, m.schema(id)));
    }
    Some(r)
}

/// Set an enum state field of a content object by variant name.
pub fn set_state_variant(b: &mut Battle, r: ObjectRef, name: &str, variant: &str) {
    let content = b.behaviors.clone();
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
    let content = b.behaviors.clone();
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
