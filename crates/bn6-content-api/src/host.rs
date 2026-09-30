//! [`ContentHost`]: a runtime holding content (Luau). The engine tells it
//! what the content pack registers ([`Registrations`]: which module
//! implements which object kind, navi action or hook, from the pack's
//! data), the runtime loads those modules into a [`Manifest`], and the
//! engine calls their functions.

use std::fmt;

use crate::api::CoreApi;
use crate::state::{Schema, StateId, Value};
use crate::types::{ObjectRef, PanelPos, Pool};

/// A ruleset table that content fills by number, instead of an object
/// kind or a navi action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Hook {
    /// Weapon routine `n` (`off_80117D4`): sets up the attack from the
    /// navi's weapon and names its action. The module's `setup(navi)`.
    Weapon(u8),
    /// The dimming chips' dimming controllers by chip subtype
    /// (`off_802CCB4`, chips with action 0x15). The module's
    /// `dimming_chip(user, spec)`.
    DimmingChip(u8),
    /// The navi chips' navis by chip subtype (`off_802CD5C`, chips with
    /// action 0x1B). The module's `navi_chip(user, controller, spec)`.
    NaviChip(u8),
}

impl Hook {
    /// The function the module exports for the hook.
    pub fn function(self) -> &'static str {
        match self {
            Hook::Weapon(_) => "setup",
            Hook::DimmingChip(_) => "dimming_chip",
            Hook::NaviChip(_) => "navi_chip",
        }
    }
}

impl fmt::Display for Hook {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Hook::Weapon(n) => write!(f, "weapon routine {n:#04x}"),
            Hook::DimmingChip(n) => write!(f, "dimming chip subtype {n}"),
            Hook::NaviChip(n) => write!(f, "navi chip subtype {n}"),
        }
    }
}

/// What a content pack registers: which module (a path in the pack,
/// without `.luau`) implements each object kind, navi action and hook.
/// Built by the engine from the pack's data; nothing in the engine or the
/// runtime names a particular kind, action or module.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Registrations {
    pub kinds: Vec<KindReg>,
    pub actions: Vec<ActionReg>,
    pub hooks: Vec<HookReg>,
}

/// An object kind: the module exports `state` (its schema) and
/// `update(me)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KindReg {
    /// The kind's name in the pack (its folder under `objects/`).
    pub name: String,
    pub pool: Pool,
    pub index: u8,
    pub module: String,
}

/// A navi action: the module exports `state` and `update(me, state)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionReg {
    /// The action number (0x10 and up).
    pub action: u8,
    pub module: String,
}

/// A hook: the module exports the hook's function ([`Hook::function`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HookReg {
    pub hook: Hook,
    pub module: String,
}

/// An object kind the content implements, loaded.
#[derive(Clone, Debug)]
pub struct ObjectKindDef {
    /// The kind's name in the pack.
    pub name: String,
    pub pool: Pool,
    pub index: u8,
    pub module: String,
    pub schema: Schema,
}

/// A navi action the content implements, loaded.
#[derive(Clone, Debug)]
pub struct ActionDef {
    pub action: u8,
    pub module: String,
    pub schema: Schema,
}

/// A hook the content implements, loaded.
#[derive(Clone, Debug)]
pub struct HookDef {
    pub hook: Hook,
    pub module: String,
}

/// Everything a content set defines, as loaded.
#[derive(Clone, Debug, Default)]
pub struct Manifest {
    pub objects: Vec<ObjectKindDef>,
    pub actions: Vec<ActionDef>,
    pub hooks: Vec<HookDef>,
}

/// An index into [`Manifest::objects`] or [`Manifest::actions`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KindId(pub u16);

/// An index into [`Manifest::hooks`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HookId(pub u16);

impl Manifest {
    /// The state id of object kind `k`.
    pub fn object_state(&self, k: KindId) -> StateId {
        StateId(k.0)
    }

    /// The state id of action `a`.
    pub fn action_state(&self, a: KindId) -> StateId {
        StateId(self.objects.len() as u16 + a.0)
    }

    pub fn schema(&self, id: StateId) -> &Schema {
        let i = id.0 as usize;
        match self.objects.get(i) {
            Some(k) => &k.schema,
            None => &self.actions[i - self.objects.len()].schema,
        }
    }

    /// The object kind named `name`.
    pub fn kind_by_name(&self, name: &str) -> Option<KindId> {
        self.objects.iter().position(|k| k.name == name).map(|i| KindId(i as u16))
    }
}

impl Registrations {
    /// Check the registrations don't overlap: each kind slot, kind name,
    /// action number and hook is implemented once.
    pub fn validate(&self) -> Result<(), ContentError> {
        for (i, k) in self.kinds.iter().enumerate() {
            for o in &self.kinds[..i] {
                if (o.pool, o.index) == (k.pool, k.index) {
                    return Err(ContentError::new(format!(
                        "objects/{} and objects/{} both implement {} object {:#x}",
                        o.name,
                        k.name,
                        k.pool.name(),
                        k.index
                    )));
                }
                if o.name == k.name {
                    return Err(ContentError::new(format!("two object kinds are named {}", k.name)));
                }
            }
        }
        for (i, a) in self.actions.iter().enumerate() {
            if a.action < 0x10 {
                return Err(ContentError::new(format!("{}: actions below 0x10 are the engine's", a.module)));
            }
            if let Some(o) = self.actions[..i].iter().find(|o| o.action == a.action && o.module != a.module) {
                return Err(ContentError::new(format!(
                    "{} and {} both implement action {:#x}",
                    o.module, a.module, a.action
                )));
            }
        }
        for (i, h) in self.hooks.iter().enumerate() {
            if let Some(o) = self.hooks[..i].iter().find(|o| o.hook == h.hook && o.module != h.module) {
                return Err(ContentError::new(format!("{} and {} both implement {}", o.module, h.module, h.hook)));
            }
        }
        Ok(())
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

/// A call of a hook, with its arguments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HookCall {
    /// `setup(navi)`: returns the action number.
    Weapon { navi: ObjectRef },
    /// `dimming_chip(user, spec)`: returns the controller, or nil.
    DimmingChip { user: ObjectRef, spec: DimmingChipSpec },
    /// `navi_chip(user, controller, spec)`: returns the navi, or nil. The
    /// navi calls `navi_chip.navi_left(controller)` when it is done.
    NaviChip { user: ObjectRef, controller: ObjectRef, spec: NaviChipSpec },
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
    /// One tick of object `me`, of kind `kind`.
    fn update_object(&self, api: &mut dyn CoreApi, kind: KindId, me: ObjectRef) -> Result<(), ContentError>;
    /// One tick of action `action` for the navi `me`.
    fn update_action(&self, api: &mut dyn CoreApi, action: KindId, me: ObjectRef) -> Result<(), ContentError>;
    /// Call hook `hook`.
    fn call_hook(&self, api: &mut dyn CoreApi, hook: HookId, call: HookCall) -> Result<Value, ContentError>;
}
