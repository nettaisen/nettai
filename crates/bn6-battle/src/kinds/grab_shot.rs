//! A grab shot (attack object #0xF, `sub_80C6414`): AreaGrab's and
//! PanelGrab's. It drops onto its panel from high up in 32 ticks, hits
//! whatever stands there, and takes the panel for its side if it is the
//! other side's, empty, and not the last of that side's full columns
//! (the panel comes back later with the stolen-area return). Spawned
//! during a time stop, it runs in it. See docs/engine/chips.md §3.6.8.

use crate::battle::Battle;
use crate::content::SpriteId;
use crate::field::{self, pflags};
use crate::kinds::common;
use crate::kinds::hitbox;
use crate::object::sprite::Shadow;
use crate::object::{ObjectRef, PanelPos, Pool, Vec3, flags, state};

pub const INDEX: u8 = 0xF;

const SPRITE: SpriteId = SpriteId { category: 0x0C, index: 0x13 };

/// How high it starts, and how fast it falls.
const HEIGHT: i32 = 0x100_0000;
const FALL_SPEED: i32 = 0x8_0000;

/// A stolen panel stays the thief's for this many ticks
/// (`object_setPanelAllianceTimerLong`).
const STOLEN_TICKS: u16 = 0x708;

/// `sub_80C6548`: a grab shot for `owner`'s side onto `panel`.
pub fn spawn(b: &mut Battle, owner: ObjectRef, panel: PanelPos, element: u8, damage: u32) -> Option<ObjectRef> {
    let alliance = b.objects.get(owner).alliance;
    let r = b.objects.spawn(Pool::Attack, INDEX, Vec3::default(), [alliance, 0, 0, 0])?;
    let o = b.objects.get_mut(r);
    o.panel = panel;
    o.element = element;
    o.alliance = alliance;
    o.damage = damage as u16;
    o.stamina = (damage >> 16) as u16;
    o.flags |= flags::RUN_IN_TIME_STOP;
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => tick(b, r),
        _ => return crate::kinds::generic_destroy(b, r),
    }
    common::update_sprite_in_time_stop(b, r);
}

/// `sub_80C6438`: high above its panel.
fn init(b: &mut Battle, r: ObjectRef) {
    let s = b.objects.sprite_mut(r);
    s.load(SPRITE);
    s.look.shadow = Shadow::WithSprite;
    let o = b.objects.get_mut(r);
    o.flags |= flags::VISIBLE;
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.anim = 0;
    o.anim_loaded = 0xFF;
    o.vel.z = FALL_SPEED;
    let (x, y) = crate::kinds::player::panel_coordinates(o.panel.x, o.panel.y);
    o.pos = Vec3 { x, y, z: HEIGHT };
    b.play_sound(crate::sound::SoundId(0xA1));
    b.objects.get_mut(r).state = state::UPDATE;
    tick(b, r);
}

/// `sub_80C647C`: fall, land (hit and grab), then play out the landing.
fn tick(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).action {
        0 => fall(b, r),
        4 => {
            // sub_80C6524
            b.objects.get_mut(r).anim = 1;
            b.play_sound(crate::sound::SoundId(0xA2));
            b.objects.get_mut(r).action = 8;
        }
        _ => {
            // sub_80C6536
            if b.objects.sprite(r).frame_parameters() & crate::object::sprite::FRAME_LAST != 0 {
                b.objects.get_mut(r).state = state::DESTROY;
            }
        }
    }
    common::update_sprite(b, r);
}

/// `sub_80C64A0`.
fn fall(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    o.pos.z = o.pos.z.wrapping_sub(o.vel.z);
    if o.pos.z > 0 {
        return;
    }
    o.pos.z = 0;
    let (p, alliance, element) = (o.panel, o.alliance, o.element);
    let spec = hitbox::HitboxSpec {
        panel: p,
        element,
        z: 0,
        region: 1,
        hit_effect: 0xFF,
        target: 5,
        self_type: 0x0A,
        damage: o.damage,
        stamina: o.stamina,
        hit_mod: 1,
        ..Default::default()
    };
    if let Some(h) = hitbox::spawn(b, r, &spec) {
        b.objects.get_mut(h).flags |= flags::RUN_IN_TIME_STOP;
    }
    // byte_80C6514: the other side's panel, with nothing on it.
    let (require, forbid) = if alliance == 0 {
        (pflags::ALLIANCE_1, pflags::OCCUPIED)
    } else {
        (0, pflags::OCCUPIED | pflags::ALLIANCE_1)
    };
    if b.field.check(p.x, p.y, require, forbid) && can_take(b, p, alliance ^ 1) {
        b.set_panel_alliance(p.x, p.y, alliance);
        b.field.columns[p.x as usize].timer = STOLEN_TICKS;
    }
    b.objects.get_mut(r).action = 4;
}

/// `sub_800D618`: a panel of `victim`'s home that someone else holds.
fn taken_from(b: &Battle, x: u8, y: u8, victim: u8) -> bool {
    b.field.panel(x, y).is_some_and(|p| p.home != p.alliance && p.home == victim)
}

/// `sub_800D6CC`: column `x` is `victim`'s home and all its.
fn full_home_column(b: &Battle, x: u8, victim: u8) -> bool {
    b.field.columns[x as usize].home == victim
        && (1..=3u8).all(|y| b.field.panel(x, y).is_some_and(|p| p.alliance == victim))
}

/// `sub_800D668`: `victim` may lose panel `p`: its column isn't
/// `victim`'s home, or `victim` keeps another full home column on either
/// side before a panel it already lost.
fn can_take(b: &Battle, p: PanelPos, victim: u8) -> bool {
    if b.field.columns[p.x as usize].home != victim {
        return true;
    }
    for dir in [1i32, -1] {
        let mut x = p.x as i32 + dir;
        while field::is_valid(x as u8, p.y) && !taken_from(b, x as u8, p.y, victim) {
            if full_home_column(b, x as u8, victim) {
                return true;
            }
            x += dir;
        }
    }
    false
}

