//! [`ContentHost`]: a runtime holding content (native Rust, Luau, ...). The
//! engine asks it what the content defines, then calls its update
//! functions.

use std::fmt;

use crate::api::CoreApi;
use crate::state::{Schema, StateId};
use crate::types::{ObjectRef, Pool};

/// An object kind the content implements: the (pool, index) slot of the
/// behavior table it fills, and its state.
#[derive(Clone, Debug)]
pub struct ObjectKindDef {
    /// A name for messages (the module that defines it).
    pub name: String,
    pub pool: Pool,
    pub index: u8,
    pub schema: Schema,
}

/// A navi action the content implements.
#[derive(Clone, Debug)]
pub struct ActionDef {
    pub name: String,
    /// The action number (0x10 and up; chips name theirs in their data).
    pub action: u8,
    pub schema: Schema,
}

/// Everything a content set defines.
#[derive(Clone, Debug, Default)]
pub struct Manifest {
    pub objects: Vec<ObjectKindDef>,
    pub actions: Vec<ActionDef>,
}

/// An index into [`Manifest::objects`] or [`Manifest::actions`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KindId(pub u16);

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

    /// Check the definitions don't overlap.
    pub fn validate(&self) -> Result<(), ContentError> {
        for (i, k) in self.objects.iter().enumerate() {
            if let Some(o) = self.objects[..i].iter().find(|o| (o.pool, o.index) == (k.pool, k.index)) {
                return Err(ContentError::new(format!(
                    "{} and {} both define {} object {:#x}",
                    o.name,
                    k.name,
                    k.pool.name(),
                    k.index
                )));
            }
        }
        for (i, a) in self.actions.iter().enumerate() {
            if a.action < 0x10 {
                return Err(ContentError::new(format!("{}: actions below 0x10 are the engine's", a.name)));
            }
            if let Some(o) = self.actions[..i].iter().find(|o| o.action == a.action) {
                return Err(ContentError::new(format!("{} and {} both define action {:#x}", o.name, a.name, a.action)));
            }
        }
        Ok(())
    }
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

/// A content runtime. Its update functions must be deterministic functions
/// of the engine state they read through `api`: the host keeps nothing
/// between calls that affects the battle (so a battle snapshot doesn't
/// include it).
pub trait ContentHost {
    /// A short name for messages ("luau", "rust").
    fn runtime(&self) -> &str;
    fn manifest(&self) -> &Manifest;
    /// One tick of object `me`, of kind `kind`.
    fn update_object(&self, api: &mut dyn CoreApi, kind: KindId, me: ObjectRef) -> Result<(), ContentError>;
    /// One tick of action `action` for the navi `me`.
    fn update_action(&self, api: &mut dyn CoreApi, action: KindId, me: ObjectRef) -> Result<(), ContentError>;
}
