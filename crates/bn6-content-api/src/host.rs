//! [`ContentHost`]: a runtime holding content (Luau). The engine plans what
//! the runtime binds from its content ([`BindPlan`]: the functions it will
//! call, found by module export or by definition slot, and the state
//! layouts); the runtime loads the pack's modules, checks it reads the same
//! definitions the content was made from, and the engine calls the
//! functions by [`FnId`].

use std::fmt;

use crate::api::CoreApi;
use crate::definitions::Definitions;
use crate::registry::Registry;
use crate::assets::AssetNames;
use crate::state::{Schema, StateId, Value};
use crate::types::{ObjectRef, PanelPos};

/// Where a function content implements is.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FnSource {
    /// A function a module's table exports: registration by module
    /// (content-pack.md §1.3), until the content model v2 migration ends.
    Export { module: String, name: String },
    /// A function slot of a definition: `update` of kind `bomb`, `dimming`
    /// of chip `areagrab` (docs/design/content-model-v2.md §3.10). `path`
    /// is the field's place in the spec, dot-separated.
    Slot { registry: Registry, key: String, path: String },
}

impl FnSource {
    /// A definition's slot.
    pub fn slot(registry: Registry, key: &str, path: &str) -> FnSource {
        FnSource::Slot { registry, key: key.to_string(), path: path.to_string() }
    }

    /// A module's export.
    pub fn export(module: &str, name: &str) -> FnSource {
        FnSource::Export { module: module.to_string(), name: name.to_string() }
    }
}

impl fmt::Display for FnSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FnSource::Export { module, name } => write!(f, "{module}.luau's {name}"),
            FnSource::Slot { registry, key, path } => write!(f, "{registry} {key}'s {path}"),
        }
    }
}

/// An index into [`BindPlan::functions`]: a function the engine calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FnId(pub u32);

/// What a runtime loads, as the engine planned it from its content: the
/// functions it will call, the state layouts, and the definitions the
/// runtime's define phase must read back. Nothing in the engine or the
/// runtime names a particular kind, action or module.
#[derive(Clone, Debug, Default)]
pub struct BindPlan {
    /// The functions the engine calls, by [`FnId`].
    pub functions: Vec<FnSource>,
    /// Content state layouts, by [`StateId`].
    pub schemas: Vec<Schema>,
    /// What the define phase read when the content was made; a runtime must
    /// read the same.
    pub definitions: Definitions,
    /// The handle of each definition, in `definitions.defs`' order (a
    /// registry's handles also number the engine's own entries, so they are
    /// not the definitions' positions).
    pub handles: Vec<u16>,
    /// The registries' entries that are no definition (the engine's own,
    /// and what registration by number makes), by registry, handle and key:
    /// a script reaches them as stand-in values (`me.kind` of a v1 object).
    pub entries: Vec<(Registry, u16, String)>,
    /// The assets content can name (`asset.sprite("bomb")`).
    pub assets: AssetNames,
}

/// What a runtime loaded that the binding reads: the state layouts.
#[derive(Clone, Debug, Default)]
pub struct Manifest {
    pub schemas: Vec<Schema>,
}

impl Manifest {
    pub fn schema(&self, id: StateId) -> &Schema {
        &self.schemas[id.0 as usize]
    }
}

/// What a dimming chip's controller is spawned with (the user's attack:
/// the registers `sub_80EBD9C` passes to `off_802CCB4[subtype]`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DimmingChipSpec {
    /// The attack's element byte (primary | secondary bits).
    pub element: u8,
    /// The chip's parameters.
    pub params: [u8; 4],
    /// The damage word: damage | hit parameter << 16.
    pub damage: u32,
    /// The chip, and the Atk+ / cross bonus the telop shows with it.
    pub chip: u16,
    pub bonus: u16,
}

/// What a navi chip's navi is spawned with (the registers
/// `sub_80E1880` passes to `off_802CD5C[subtype]`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NaviChipSpec {
    /// Where the navi appears (the user's panel when the chip was used).
    pub panel: PanelPos,
    pub element: u8,
    pub params: [u8; 4],
    /// The damage word with the bonus added.
    pub damage: u32,
}

/// What an instant chip's effect runs with (the registers `sub_80EC39C`
/// passes to `off_80EC3F0[subtype]`: the user's panel and Z, and its
/// attack).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InstantChipSpec {
    /// The user's panel.
    pub panel: PanelPos,
    /// The attack's element byte.
    pub element: u8,
    /// The user's Z, 16.16.
    pub z: i32,
    /// The chip's parameters.
    pub params: [u8; 4],
    /// The damage word (damage | hit parameter << 16) plus the Atk+ /
    /// cross bonus's low byte.
    pub damage: u32,
}

/// An actor list's entry, as its spawner (`off_80073A0[type]`) reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActorListEntrySpec {
    pub panel: PanelPos,
    /// The entry's side (the one navis take; field objects take their
    /// panel's).
    pub side: u8,
    /// The entry's argument (a rock's variant).
    pub variant: u8,
}

/// A call of a hook, with its arguments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HookCall {
    /// `setup(navi)`: returns the action, a number (registration by number)
    /// or an action definition.
    Weapon { navi: ObjectRef },
    /// `dimming_chip(user, spec)`: returns the controller, or nil.
    DimmingChip { user: ObjectRef, spec: DimmingChipSpec },
    /// `navi_chip(user, controller, spec)`: returns the navi, or nil. The
    /// navi calls `navi_chip.navi_left(controller)` when it is done.
    NaviChip { user: ObjectRef, controller: ObjectRef, spec: NaviChipSpec },
    /// `instant_chip(user, spec)`: its result is unused.
    InstantChip { user: ObjectRef, spec: InstantChipSpec },
    /// `actor_list_entry(spec)`: returns what it placed, or nil.
    ActorListEntry { spec: ActorListEntrySpec },
}

/// A content error: a bug in the content, or a script breaking the
/// runtime's rules. The engine stops the battle on one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentError {
    pub message: String,
}

impl ContentError {
    pub fn new(message: impl Into<String>) -> ContentError {
        ContentError { message: message.into() }
    }
}

impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ContentError {}

/// A content runtime. Its functions must be deterministic functions of the
/// engine state they read through `api`: the host keeps nothing between
/// calls that affects the battle (so a battle snapshot doesn't include
/// it).
pub trait ContentHost {
    /// A short name for messages ("luau").
    fn runtime(&self) -> &str;
    fn manifest(&self) -> &Manifest;
    /// One tick of object `me`: function `f` is its kind's `update`.
    fn update_object(&self, api: &mut dyn CoreApi, f: FnId, me: ObjectRef) -> Result<(), ContentError>;
    /// One tick of an action for the navi `me`: function `f` is the
    /// action's `update`, and its second argument is the attack state as a
    /// state of layout `state`.
    fn update_action(&self, api: &mut dyn CoreApi, f: FnId, me: ObjectRef, state: StateId)
    -> Result<(), ContentError>;
    /// Call function `f` for a hook.
    fn call_hook(&self, api: &mut dyn CoreApi, f: FnId, call: HookCall) -> Result<Value, ContentError>;
}
