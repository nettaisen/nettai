//! DustCross's charged shot (attack object #0xB0, `sub_80DB6A4`): a ball
//! of junk that rolls forward from in front of the navi, 6 pixels a tick,
//! to the far edge; where it meets an enemy body or object it stops, and
//! 5 ticks later hits its panel (cracking it). Param1 0: it stands still
//! in time stop.

use crate::battle::Battle;
use crate::data::SpriteId;
use crate::kinds::common::{self, Progress};
use crate::kinds::hitbox;
use crate::kinds::player::panel_coordinates;
use crate::object::sprite::Shadow;
use crate::object::{ObjectRef, PanelPos, Pool, Vec3, flags, state};

pub const INDEX: u8 = 0xB0;

const SPRITE: SpriteId = SpriteId { category: 8, index: 0x0A };

/// Its speed along X.
const SPEED: i32 = 0x6_0000;

/// `sub_80DB800`: a ball from `owner` starting on `panel`.
pub fn spawn(b: &mut Battle, owner: ObjectRef, panel: PanelPos, element: u8, damage: u32) -> Option<ObjectRef> {
    // The spawn's Z is register garbage; the init keeps only its fraction.
    let r = b.objects.spawn(Pool::Attack, INDEX, Vec3::default(), [0; 4])?;
    let (alliance, flip) = {
        let o = b.objects.get(owner);
        (o.alliance, o.flip)
    };
    let o = b.objects.get_mut(r);
    o.panel = panel;
    o.element = element;
    o.related[0] = Some(owner);
    o.damage = damage as u16;
    o.stamina = (damage >> 16) as u16;
    o.alliance = alliance;
    o.flip = flip;
    o.flags |= flags::RUN_IN_TIME_STOP;
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => tick(b, r),
        _ => return crate::kinds::generic_destroy(b, r),
    }
    // object_updateSpritePaused: not in time stop.
    if !b.is_time_stop() && b.objects.get(r).flags & flags::ACTIVE != 0 {
        common::step_sprite(b, r);
    }
}

/// `sub_80DB6D4`: 18 pixels in front of its panel's centre.
fn init(b: &mut Battle, r: ObjectRef) {
    common::set_coordinates_from_panels(b, r);
    let o = b.objects.get_mut(r);
    o.pos.z &= 0xFFFF;
    let front = common::facing(o.alliance, o.flip);
    o.pos.x = o.pos.x.wrapping_add(front * 0x12_0000);
    o.flags |= flags::VISIBLE;
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.anim = 0x19;
    o.anim_loaded = 0x19;
    let flip = o.alliance ^ o.flip;
    let s = b.objects.sprite_mut(r);
    s.load(SPRITE);
    s.look.shadow = Shadow::Ground;
    s.look.palette = 0;
    s.look.set_flip(flip);
    s.set_animation(0x19);
    common::set_progress(b, r, Progress::UPDATE);
}

fn tick(b: &mut Battle, r: ObjectRef) {
    if b.is_battle_over() {
        b.objects.get_mut(r).state = state::DESTROY;
        return;
    }
    if b.is_time_stop() {
        return;
    }
    match b.objects.get(r).action {
        0 => roll(b, r),
        _ => burst(b, r),
    }
}

/// `sub_80DB864`: an enemy body or object on its panel.
fn blocked(b: &Battle, r: ObjectRef) -> bool {
    // byte_80DB888
    const STOPS: [u32; 2] = [0x0580_0000, 0x0A80_0000];
    let o = b.objects.get(r);
    b.field.flags(o.panel.x, o.panel.y) & STOPS[o.alliance as usize] != 0
}

fn to_burst(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    o.action = 4;
    o.phase = 0;
}

/// `sub_80DB75C`: roll to the far column, checking for a stop each time
/// it passes a panel's centre.
fn roll(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase == 0 {
        b.objects.get_mut(r).phase = 4;
        // sub_80DB828: the ticks to the far column.
        let o = b.objects.get_mut(r);
        let front = common::facing(o.alliance, o.flip);
        o.vel.x = front * SPEED;
        let far = if o.alliance == 0 { 6 } else { 1 };
        if o.panel.x == far {
            o.vel.x = 0;
            o.timer = 0;
        } else {
            let (x, _) = panel_coordinates(far, 1);
            o.timer = ((x - o.pos.x).abs() / SPEED) as u16;
        }
        if blocked(b, r) {
            return to_burst(b, r);
        }
    }
    let o = b.objects.get_mut(r);
    let left = o.timer as i32 - 1;
    o.timer = left as u16;
    if left <= 0 {
        return to_burst(b, r);
    }
    let (centre, _) = panel_coordinates(o.panel.x, o.panel.y);
    let old = o.pos.x;
    o.pos.x = o.pos.x.wrapping_add(o.vel.x);
    let new = o.pos.x;
    // sub_800E708: it reached or passed the centre.
    if centre != old && crossed(new, old, centre) && blocked(b, r) {
        return to_burst(b, r);
    }
    common::set_panels_from_coordinates(b, r);
}

/// `sub_800E708`: `c` lies between `a` and `b` (inclusive).
fn crossed(a: i32, b: i32, c: i32) -> bool {
    if a == c || b == c {
        return true;
    }
    if a <= b { c > a && c <= b } else { c >= b && c <= a }
}

/// `sub_80DB7B2`: 30 ticks of bursting; on the 5th the hit, and a solid
/// panel cracks.
fn burst(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        let o = b.objects.get_mut(r);
        o.phase_init = 4;
        o.anim = 0x1A;
        o.timer = 0x1E;
        return;
    }
    let o = b.objects.get_mut(r);
    let left = o.timer as i32 - 1;
    o.timer = left as u16;
    if left == 0x19 {
        // sub_80DB890
        let o = b.objects.get(r);
        let (p, element) = (o.panel, o.element);
        let spec = hitbox::HitboxSpec {
            panel: p,
            element,
            region: 1,
            hit_effect: 0x0A,
            target: 5,
            self_type: 0x15,
            damage: o.damage,
            stamina: o.stamina,
            hit_mod: 3,
            ..Default::default()
        };
        hitbox::spawn(b, r, &spec);
        if b.field.is_solid(p.x, p.y) {
            // (A camera shake: presentation, with RNG1.)
            b.play_sound(crate::sound::SoundId(0xC0));
            b.crack_panel(p.x, p.y);
        }
        return;
    }
    if left <= 0 {
        common::set_progress(b, r, Progress::DESTROY);
    }
}
