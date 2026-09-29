//! Object behaviors, chosen by pool and index, plus helpers shared by many
//! behaviors (HP changes, damage formulas).

pub mod absorbed_obstacle;
pub mod afterimage;
pub mod attachment;
pub mod charge_glow;
pub mod common;
pub mod effect;
pub mod form_overlay;
pub mod hitbox;
pub mod intro;
pub mod lockon_marker;
pub mod obstacle;
pub mod palette_flash;
pub mod player;
pub mod rock;
pub mod rock_debris;
pub mod spark;
pub mod sun_beam;

use crate::battle::Battle;
use crate::object::{ObjectRef, Pool};

/// Behavior-private state. The game gives every object 0x2C (actors,
/// attacks) or 0x1C (effects) bytes of scratch; here each behavior gets a
/// typed struct, zeroed at spawn like the game's.
#[derive(Clone, Debug, Default)]
pub enum Vars {
    #[default]
    None,
    Intro(intro::Vars),
    ChargeGlow(charge_glow::Vars),
    Effect(effect::Vars),
    Hitbox(hitbox::Vars),
    Rock(rock::Vars),
    AbsorbedObstacle(absorbed_obstacle::Vars),
    FormOverlay(form_overlay::Vars),
    Afterimage(afterimage::Vars),
    LockonMarker(lockon_marker::Vars),
    PaletteFlash(palette_flash::Vars),
    Attachment(attachment::Vars),
    SunBeam(sun_beam::Vars),
    /// A content kind's declared state (see `content`).
    Content(bn6_content_api::ContentState),
}

impl Vars {
    pub fn for_kind(pool: Pool, index: u8) -> Vars {
        match (pool, index) {
            (Pool::Effect, 2) => Vars::Intro(Default::default()),
            (Pool::Effect, 8) => Vars::ChargeGlow(Default::default()),
            (Pool::Effect, 0) => Vars::Effect(Default::default()),
            (Pool::Attack, 3) => Vars::Hitbox(Default::default()),
            (Pool::Actor, form_overlay::INDEX) => Vars::FormOverlay(Default::default()),
            (Pool::Effect, afterimage::INDEX) => Vars::Afterimage(Default::default()),
            (Pool::Effect, lockon_marker::INDEX) => Vars::LockonMarker(Default::default()),
            (Pool::Effect, palette_flash::INDEX) => Vars::PaletteFlash(Default::default()),
            (Pool::Actor, attachment::INDEX) => Vars::Attachment(Default::default()),
            (Pool::Effect, sun_beam::INDEX) => Vars::SunBeam(Default::default()),
            (Pool::Actor, 0) => Vars::None,
            _ => Vars::None,
        }
    }
}

/// Run one object's update.
pub fn update(b: &mut Battle, r: ObjectRef) {
    let index = b.objects.get(r).index;
    if let Some(kind) = b.content.object_kind(r.pool, index) {
        return crate::content::run_object(b, kind, r);
    }
    match (r.pool, index) {
        (Pool::Actor, 0) => player::update(b, r),
        (Pool::Effect, 2) => intro::update(b, r),
        (Pool::Effect, 8) => charge_glow::update(b, r),
        (Pool::Effect, 0) => effect::update(b, r),
        (Pool::Attack, 3) => hitbox::update(b, r),
        (Pool::Effect, 4) => spark::update(b, r),
        (Pool::Attack, rock::INDEX) => rock::update(b, r),
        (Pool::Effect, rock_debris::INDEX) => rock_debris::update(b, r),
        (Pool::Effect, absorbed_obstacle::INDEX) => absorbed_obstacle::update(b, r),
        (Pool::Actor, form_overlay::INDEX) => form_overlay::update(b, r),
        (Pool::Effect, afterimage::INDEX) => afterimage::update(b, r),
        (Pool::Effect, lockon_marker::INDEX) => lockon_marker::update(b, r),
        (Pool::Effect, palette_flash::INDEX) => palette_flash::update(b, r),
        (Pool::Actor, attachment::INDEX) => attachment::update(b, r),
        (Pool::Effect, sun_beam::INDEX) => sun_beam::update(b, r),
        (pool, index) => panic!("object kind {pool:?} {index:#x} is not implemented yet"),
    }
}

/// `object_genericDestroy`: drop reservations, free collision, free.
pub fn generic_destroy(b: &mut Battle, r: ObjectRef) {
    b.release_reservations(r);
    if let Some(c) = b.objects.get(r).collision {
        b.collision.free(c);
    }
    b.objects.free(r);
}

/// `object_subtractHP`.
pub fn subtract_hp(b: &mut Battle, r: ObjectRef, amount: u16) {
    let o = b.objects.get_mut(r);
    o.hp = o.hp.saturating_sub(amount);
}

/// `sub_801A29A`: thaw a frozen object.
pub fn thaw(b: &mut Battle, r: ObjectRef) {
    let Some(c) = b.objects.get(r).collision else { return };
    let s = b.collision.get_mut(c);
    s.f1 &= !crate::collision::f1::FROZEN;
    s.f2 &= !crate::collision::f1::FROZEN;
    s.status_timers[crate::collision::timer::FREEZE] = 0;
}

/// `sub_800AF84`: busting level at the end of a round.
pub fn busting_level(_b: &Battle) -> u8 {
    // Only stored for the results screen; netbattles saw 0x0B.
    0x0B
}

/// `sub_802CEC8`: per-side registry of linked objects; clears an entry
/// when its object's HP reaches 0.
pub fn update_linked_registry(_b: &mut Battle) {}

/// `sub_802CDFE`: age the per-side damage-carry records.
pub fn shift_damage_carry(b: &mut Battle) {
    for side in 0..2 {
        let c = &mut b.damage_carry[side];
        c.previous = c.this_tick;
        c.this_tick = 0;
    }
}

/// Damage formulas for chips whose damage is 1000 or more (`off_80109DC`).
pub fn chip_damage_formula(_b: &Battle, id: u16, _side: u8, formula: u16) -> u16 {
    panic!("damage formula {formula} (chip {id:#x}) is not implemented yet")
}
