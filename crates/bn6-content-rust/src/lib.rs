//! The GunDelSol slice as native Rust content: the chip's action (0x37),
//! the gun attachment (actor #5), the sun beam (effect #0x48) and the
//! one-tick hitbox (attack #3), written only against the content API.
//!
//! It is the comparison point for the scripted versions in content/bn6
//! (docs/design/scripting.md): the same code shape, compiled in, with no
//! sandbox. The engine's own implementations live in bn6-battle's `kinds`.

pub mod attachment;
pub mod data;
pub mod gun_del_sol;
pub mod hitbox;
pub mod sun_beam;

use bn6_content_api::state::TypedState;
use bn6_content_api::{
    ActionDef, ContentError, ContentHost, CoreApi, KindId, Manifest, ObjectKindDef, ObjectRef, Pool,
};

/// Where an owner keeps an attached object; the object lives while the
/// slot holds something (not necessarily the object itself).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Slot {
    /// The owner's actor overlay slot.
    #[default]
    Overlay,
    /// The owner's first related object.
    Related,
}

impl Slot {
    pub fn is_occupied(self, api: &dyn CoreApi, owner: ObjectRef) -> bool {
        use bn6_content_api::api::{ActorFields, ObjectFields};
        match self {
            Slot::Overlay => api.overlay(owner).is_some(),
            Slot::Related => api.related1(owner).is_some(),
        }
    }

    pub fn set(self, api: &mut dyn CoreApi, owner: ObjectRef, v: Option<ObjectRef>) {
        use bn6_content_api::api::{ActorFields, ObjectFields};
        match self {
            Slot::Overlay => api.set_overlay(owner, v),
            Slot::Related => api.set_related1(owner, v),
        }
    }
}

impl bn6_content_api::state::StateField for Slot {
    fn field_type() -> bn6_content_api::FieldType {
        bn6_content_api::FieldType::Enum(vec!["overlay".into(), "related".into()])
    }
    fn from_field(v: bn6_content_api::FieldValue) -> Slot {
        match v {
            bn6_content_api::FieldValue::Enum(0) => Slot::Overlay,
            bn6_content_api::FieldValue::Enum(1) => Slot::Related,
            v => panic!("expected a slot, got {v:?}"),
        }
    }
    fn to_field(&self) -> bn6_content_api::FieldValue {
        bn6_content_api::FieldValue::Enum(*self as u8)
    }
}

type Update = fn(&mut dyn CoreApi, ObjectRef);

/// The slice as a content host.
pub struct RustContent {
    manifest: Manifest,
    objects: Vec<Update>,
    actions: Vec<Update>,
}

impl RustContent {
    pub fn new() -> RustContent {
        let kind = |name: &str, pool, index, schema| ObjectKindDef { name: name.into(), pool, index, schema };
        let manifest = Manifest {
            objects: vec![
                kind("attachment", Pool::Actor, attachment::INDEX, attachment::State::schema()),
                kind("sun_beam", Pool::Effect, sun_beam::INDEX, sun_beam::State::schema()),
                kind("hitbox", Pool::Attack, hitbox::INDEX, hitbox::State::schema()),
            ],
            actions: vec![ActionDef {
                name: "gun_del_sol".into(),
                action: gun_del_sol::ACTION,
                schema: gun_del_sol::State::schema(),
            }],
        };
        RustContent {
            manifest,
            objects: vec![attachment::update, sun_beam::update, hitbox::update],
            actions: vec![gun_del_sol::update],
        }
    }
}

impl Default for RustContent {
    fn default() -> RustContent {
        RustContent::new()
    }
}

impl ContentHost for RustContent {
    fn runtime(&self) -> &str {
        "rust"
    }

    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn update_object(&self, api: &mut dyn CoreApi, kind: KindId, me: ObjectRef) -> Result<(), ContentError> {
        (self.objects[kind.0 as usize])(api, me);
        Ok(())
    }

    fn update_action(&self, api: &mut dyn CoreApi, action: KindId, me: ObjectRef) -> Result<(), ContentError> {
        (self.actions[action.0 as usize])(api, me);
        Ok(())
    }
}

/// An object's content state, typed.
fn state<S: TypedState>(api: &dyn CoreApi, o: ObjectRef) -> S {
    S::load(api.state(o).expect("a content object has its state"))
}

fn set_state<S: TypedState>(api: &mut dyn CoreApi, o: ObjectRef, s: &S) {
    s.store(api.state_mut(o).expect("a content object has its state"));
}
