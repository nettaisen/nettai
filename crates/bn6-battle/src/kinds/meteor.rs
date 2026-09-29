//! A meteor (attack object #0x8D, `sub_80D6BD4`): it falls from high
//! behind its panel onto it in 17 ticks; if the panel holds something, it
//! bursts there with a one-tick hit region. ElmntMan's Fire drops them
//! during a time stop (Param1 = 1: it runs in time stop).
//! See docs/engine/chips.md §3.6.7.

use crate::battle::Battle;
use crate::data::SpriteId;
use crate::kinds::common::{self, Progress};
use crate::kinds::{effect, hitbox};
use crate::object::sprite::Shadow;
use crate::object::{ObjectRef, PanelPos, Pool, Vec3, flags, state};

pub const INDEX: u8 = 0x8D;

const SPRITE: SpriteId = SpriteId { category: 0x0C, index: 0x31 };

/// Where it starts: this far behind and above its panel.
const START: i32 = 0xC0_0000;
/// Its speed along X (toward the front) and down, per tick.
const SPEED: i32 = 0xB_0000;
/// Ticks to fall.
const FALL: u16 = 0x11;

/// The meteor's own state.
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    /// Param1: it runs (and steps its sprite) in time stop.
    pub in_time_stop: bool,
}

fn in_time_stop(b: &Battle, r: ObjectRef) -> bool {
    matches!(&b.objects.get(r).vars, crate::kinds::Vars::Meteor(v) if v.in_time_stop)
}

/// `sub_80D6D18`: a Fire meteor from `owner` onto `panel`, running in
/// time stop.
pub fn spawn(b: &mut Battle, owner: ObjectRef, panel: PanelPos, damage: u32) -> Option<ObjectRef> {
    let r = b.objects.spawn(Pool::Attack, INDEX, Vec3::default(), [1, 0, 0, 0])?;
    let (alliance, flip) = {
        let o = b.objects.get(owner);
        (o.alliance, o.flip)
    };
    let o = b.objects.get_mut(r);
    o.panel = panel;
    o.element = 1;
    o.damage = damage as u16;
    o.stamina = (damage >> 16) as u16;
    o.alliance = alliance;
    o.flip = flip;
    o.flags |= flags::RUN_IN_TIME_STOP;
    o.vars = crate::kinds::Vars::Meteor(Vars { in_time_stop: true });
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => tick(b, r),
        _ => {
            // object_genericDestroy
            return crate::kinds::generic_destroy(b, r);
        }
    }
    if in_time_stop(b, r) {
        common::update_sprite_in_time_stop(b, r);
    } else {
        // object_updateSpritePaused
        panic!("meteors outside time stop (object_updateSpritePaused) are not implemented yet");
    }
}

/// `sub_80D6C04`: high above, behind its panel.
fn init(b: &mut Battle, r: ObjectRef) {
    common::set_coordinates_from_panels(b, r);
    let o = b.objects.get_mut(r);
    let front = common::facing(o.alliance, o.flip);
    o.pos.x = o.pos.x.wrapping_add(-front * START);
    o.pos.z = START;
    o.flags |= flags::VISIBLE;
    o.anim = 0;
    o.anim_loaded = 0;
    o.flags &= !flags::NO_SPRITE_UPDATE;
    let flip = o.alliance ^ o.flip;
    let s = b.objects.sprite_mut(r);
    s.load(SPRITE);
    s.look.shadow = Shadow::WithSprite;
    s.look.palette = 0;
    s.look.set_flip(flip);
    s.set_animation(0);
    common::set_progress(b, r, Progress::UPDATE);
}

/// `sub_80D6C56` / `sub_80D6C88`: fall; on landing, burst if the panel
/// holds something; highlight the panel 4 ticks out of 8.
fn tick(b: &mut Battle, r: ObjectRef) {
    if b.is_battle_over() {
        return common::set_progress(b, r, Progress::DESTROY);
    }
    if !in_time_stop(b, r) && b.is_time_stop() {
        return;
    }
    if b.objects.get(r).phase == 0 {
        let o = b.objects.get_mut(r);
        o.phase = 4;
        let front = common::facing(o.alliance, o.flip);
        o.vel.x = front * SPEED;
        o.vel.z = SPEED;
        o.timer = FALL;
        b.play_sound(crate::sound::SoundId(0xC4));
    } else {
        let o = b.objects.get_mut(r);
        o.pos.x = o.pos.x.wrapping_add(o.vel.x);
        o.pos.z = o.pos.z.wrapping_sub(o.vel.z);
        let left = o.timer as i32 - 1;
        o.timer = left as u16;
        if left <= 0 {
            return land(b, r);
        }
    }
    let o = b.objects.get_mut(r);
    o.timer2 = o.timer2.wrapping_add(1);
    if o.timer2 & 4 == 0 {
        let p = o.panel;
        common::highlight_panel(b, p.x, p.y);
    }
}

/// Burst where something is: an explosion and the hit region.
fn land(b: &mut Battle, r: ObjectRef) {
    // byte_80D6D08: what a meteor bursts on, by side.
    const HITS: [u32; 2] = [0x1580_0010, 0x2A80_0010];
    let o = b.objects.get(r);
    let (p, alliance) = (o.panel, o.alliance);
    if b.field.flags(p.x, p.y) & HITS[alliance as usize] != 0 {
        let pos = b.objects.get(r).pos;
        effect::spawn(b, pos, 0, 0, 0, 0);
        // sub_80D6D3E: the region runs in time stop as well
        // (sub_80C53A6).
        let o = b.objects.get(r);
        let spec = hitbox::HitboxSpec {
            panel: p,
            element: o.element,
            z: 0,
            region: 1,
            hit_effect: 1,
            target: 5,
            self_type: 0x0A,
            damage: o.damage,
            stamina: o.stamina,
            hit_mod: 3,
            ..Default::default()
        };
        if let Some(h) = hitbox::spawn(b, r, &spec) {
            b.objects.get_mut(h).flags |= flags::RUN_IN_TIME_STOP;
        }
    }
    common::set_progress(b, r, Progress::DESTROY);
}
