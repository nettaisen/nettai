//! Rocks (attack object #0x59): field obstacles with HP that block
//! panels, break into debris when destroyed, and leave after 6000 ticks.
//! Variants 0-2 are rocks; variant 3 is the aqua ice block (what an
//! obstacle encased in ice becomes). Some stages place them when the round
//! starts (actor lists); chips place others. See
//! docs/engine/field-objects.md.

use crate::battle::Battle;
use crate::data::{self, RockKind, SpriteId};
use crate::field::pflags;
use crate::kinds::common;
use crate::kinds::obstacle::{self, Action, Obstacle};
use crate::object::{Object, ObjectRef, PanelPos, Pool, Vec3, flags, state};

/// Object index in the attack pool.
pub const INDEX: u8 = 0x59;

/// Rocks live this many ticks.
const LIFETIME: u16 = 6000;

/// The rock's kind among absorbed obstacles (`data::ABSORBED_SPRITES`).
const ABSORBED_KIND: u8 = 4;

/// Damage a rock deals when thrown.
const START_DAMAGE: u16 = 200;

/// The rock sprite: animation 0 rises out of the panel, 1 and 2 stand.
const SPRITE: SpriteId = SpriteId { category: 0x10, index: 0 };

/// How a rock comes onto the field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entrance {
    /// Rises out of the panel (its appear animation), then stands.
    Rise = 0,
    /// Stands at once.
    Instant = 1,
    /// Falls from above and lands with a shake.
    Fall = 2,
    /// Placed when the round starts: stands at once, and gets its first
    /// update in the paused intro.
    Placed = 3,
}

/// What a rock is spawned with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spec {
    /// Which rock (`data::ROCKS`).
    pub variant: u8,
    /// Which of its side's field-object slots it takes (0: two per side,
    /// 1: one per side).
    pub class: u8,
    pub entrance: Entrance,
}

/// Rock behavior state.
#[derive(Clone, Debug)]
pub struct Vars {
    pub kind: &'static RockKind,
    pub entrance: Entrance,
    pub obstacle: obstacle::State,
}

fn vars(b: &Battle, r: ObjectRef) -> &Vars {
    match &b.objects.get(r).vars {
        crate::kinds::Vars::Rock(v) => v,
        _ => unreachable!("rock without rock state"),
    }
}

fn vars_mut(o: &mut Object) -> &mut Vars {
    match &mut o.vars {
        crate::kinds::Vars::Rock(v) => v,
        _ => unreachable!("rock without rock state"),
    }
}

fn kind(b: &Battle, r: ObjectRef) -> &'static RockKind {
    vars(b, r).kind
}

/// `sub_80074FA`: a rock from a battle's actor list, on the side of its
/// panel.
pub fn spawn_at_start(b: &mut Battle, x: u8, y: u8, variant: u8) -> Option<ObjectRef> {
    let side = b.field.panel(x, y).expect("actor list rock on the field").alliance;
    let spec = Spec { variant, class: 0, entrance: Entrance::Placed };
    spawn(b, PanelPos { x, y }, side, spec, START_DAMAGE, 0)
}

/// `sub_80CFBC4`: place a rock for `side`, registering it in the side's
/// field-object slots (possibly evicting an older one). `damage` and
/// `stamina` are what it hits with when thrown.
pub fn spawn(b: &mut Battle, panel: PanelPos, side: u8, spec: Spec, damage: u16, stamina: u16) -> Option<ObjectRef> {
    let kind = &data::ROCKS[spec.variant as usize];
    let params = [spec.variant, spec.class, spec.entrance as u8, 0];
    // Until init places it, the position is whatever the spawner's
    // registers held; nothing sees it.
    let pos = Vec3 { x: panel.y as i32, y: side as i32, z: 0 };
    let r = b.objects.spawn(Pool::Attack, INDEX, pos, params)?;
    let o = b.objects.get_mut(r);
    o.panel = panel;
    o.alliance = side;
    o.damage = damage;
    o.stamina = stamina;
    o.flags |= if spec.entrance == Entrance::Placed {
        flags::RUN_IN_TIME_STOP | flags::RUN_WHILE_PAUSED
    } else {
        flags::RUN_IN_TIME_STOP
    };
    o.vars = crate::kinds::Vars::Rock(Vars { kind, entrance: spec.entrance, obstacle: Default::default() });
    obstacle::register(b, r, side, spec.class);
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => tick(b, r),
        _ => crate::kinds::generic_destroy(b, r),
    }
}

/// `sub_80CF974`.
fn init(b: &mut Battle, r: ObjectRef) {
    let kind = kind(b, r);
    let entrance = vars(b, r).entrance;
    let anim = if entrance == Entrance::Placed { kind.anim } else { 0 };
    let sprite = b.objects.sprite_mut(r);
    sprite.load(SPRITE);
    sprite.set_animation(anim);
    sprite.update();
    sprite.look.shadow = crate::object::sprite::Shadow::Ground;
    let o = b.objects.get_mut(r);
    o.flags = (o.flags | flags::VISIBLE) & !flags::NO_SPRITE_UPDATE;
    o.anim = anim;
    o.anim_loaded = 0xFF;
    common::set_coordinates_from_panels(b, r);
    let o = b.objects.get_mut(r);
    o.pos.z = 0;
    o.future_panel = o.panel;
    o.element = kind.element as u8;
    o.hp = kind.hp;
    o.max_hp = kind.hp;
    o.name_id = kind.name_id;
    o.timer = LIFETIME;
    let Some(c) = b.create_collision(r) else {
        b.objects.free(r);
        return;
    };
    b.setup_collision(r, 0xE, 0xF, 3);
    let s = b.collision.get_mut(c);
    s.hit_effect = 0xA;
    s.region = 0;
    b.present_collision(c);
    b.objects.get_mut(r).state = state::UPDATE;
    tick(b, r);
}

/// `sub_80CFA18`.
fn tick(b: &mut Battle, r: ObjectRef) {
    b.objects.get_mut(r).flags |= flags::VISIBLE;
    obstacle::take_hits(b, r);
    crate::kinds::spark::spawn_collision_effect(b, r);
    obstacle::tick_lifetime(b, r);
    obstacle::react::<Rock>(b, r);
    common::update_sprite(b, r);
    let c = b.objects.get(r).collision.expect("rock with collision data");
    b.present_collision(c);
}

/// The rock's own actions.
pub struct Rock;

impl Obstacle for Rock {
    fn state(o: &mut Object) -> &mut obstacle::State {
        &mut vars_mut(o).obstacle
    }

    /// `sub_80CFA6C`: take the panel once the entrance is over; if the
    /// panel isn't free solid ground, break at once.
    fn appear(b: &mut Battle, r: ObjectRef) {
        match vars(b, r).entrance {
            Entrance::Fall => return fall(b, r),
            Entrance::Rise if !b.objects.sprite(r).finished() => return,
            _ => {}
        }
        let p = b.objects.get(r).panel;
        if !b.field.check(p.x, p.y, pflags::SOLID, pflags::OCCUPIED) {
            return Action::Destroyed.start(b, r);
        }
        stand(b, r);
    }

    /// `sub_80CFB2C`: leave the field, breaking into debris unless a chip
    /// removed it.
    fn destroyed(b: &mut Battle, r: ObjectRef) {
        obstacle::set_region(b, r, 0);
        let f = b.objects.get(r).future_panel;
        b.unreserve_panel(r, f.x, f.y);
        obstacle::release_tracking(b, r);
        b.field.objects.unregister(r);
        let f2 = obstacle::f2_of(b, r);
        if f2 & obstacle::f2::REMOVED != 0 {
            if f2 & obstacle::f2::ABSORBED != 0 {
                // Rocks never set a sprite palette.
                obstacle::fly_to_absorber(b, r, ABSORBED_KIND, 0);
            } else {
                match obstacle::blink_out(b, r, f2) {
                    obstacle::BlinkOut::Blinking => return,
                    obstacle::BlinkOut::Done => {}
                    obstacle::BlinkOut::No => {
                        let o = b.objects.get(r);
                        let pos = Vec3 { z: o.pos.z.wrapping_add(0xC << 16), ..o.pos };
                        crate::kinds::effect::spawn(b, pos, 0x14, 0, 0, 0);
                    }
                }
            }
        } else {
            let pos = b.objects.get(r).pos;
            let palette = kind(b, r).debris_palette;
            for _ in 0..2 {
                let at = crate::kinds::spark::jitter(b, 0xF, pos);
                crate::kinds::rock_debris::spawn(b, at, palette);
            }
            let above = Vec3 { z: pos.z.wrapping_add(0x10 << 16), ..pos };
            crate::kinds::effect::spawn(b, above, 2, 0, 0, 0);
            // (Plays the kind's break sound.)
        }
        obstacle::finish(b, r);
    }

    /// `sub_80CFB28`.
    fn idle(_b: &mut Battle, _r: ObjectRef) {}
}

/// Stand on the panel: standing animation, collision on the panel,
/// normal pause behavior.
fn stand(b: &mut Battle, r: ObjectRef) {
    let anim = kind(b, r).anim;
    b.objects.get_mut(r).anim = anim;
    obstacle::set_region(b, r, 1);
    b.objects.get_mut(r).flags &= !flags::RUN_WHILE_PAUSED;
    Action::Idle.start(b, r);
}

/// `sub_80CFAC0`: fall from 160 pixels up; break if the panel below is
/// gone.
fn fall(b: &mut Battle, r: ObjectRef) {
    let anim = kind(b, r).anim;
    let o = b.objects.get_mut(r);
    if o.phase_init == 0 {
        o.anim = anim;
        o.vel.z = 0;
        o.pos.z = 0xA0 << 16;
        o.phase_init = 4;
    }
    o.vel.z = o.vel.z.wrapping_sub(0x4000);
    o.pos.z = o.pos.z.wrapping_add(o.vel.z);
    if o.pos.z <= 0 {
        let p = o.panel;
        if !b.field.is_solid(p.x, p.y) {
            Action::Destroyed.start(b, r);
        } else {
            // (The landing shakes the camera, which draws the local RNG,
            // and plays a sound.)
            b.objects.get_mut(r).pos.z = 0;
            obstacle::set_region(b, r, 1);
            Action::Idle.start(b, r);
        }
    }
    let p = b.objects.get(r).panel;
    common::highlight_panel(b, p.x, p.y);
}

#[cfg(test)]
#[path = "rock_tests.rs"]
mod tests;
