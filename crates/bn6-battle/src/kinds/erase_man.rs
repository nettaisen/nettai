//! EraseMan, the navi of the EraseMan chip (actor object #0x15,
//! `sub_80BB608`), brought by its time-freeze controller (`navi_chip`).
//! He appears and aims a line of marks (`erase_mark`) up-forward,
//! forward or down-forward from his row, switching every Param1 ticks,
//! until his user presses A (or 360 ticks pass); then he slashes along the
//! line (`erase_beam`), and leaves. See docs/engine/chips.md §3.6.10.

use crate::battle::Battle;
use crate::data::SpriteId;
use crate::field;
use crate::input::keys;
use crate::kinds::{common, erase_beam, erase_mark, navi_chip};
use crate::object::sprite::Shadow;
use crate::object::{ObjectRef, PanelPos, Pool, Vec3, flags, state};

pub const INDEX: u8 = 0x15;

/// Its index among the navi chips' navis (`off_802CD5C`).
pub const NAVI: u8 = 5;

const SPRITE: SpriteId = SpriteId { category: 8, index: 4 };

/// The aims a row allows, in the order they cycle (`byte_80BB88C`; 0
/// up-forward, 1 forward, 2 down-forward).
const AIMS_BY_ROW: [&[u8]; 3] = [&[1, 2], &[0, 1, 2, 1], &[0, 1]];

/// Each aim's step (`byte_80BB860`), forward and down.
const AIM_STEPS: [(i32, i32); 3] = [(1, -1), (1, 0), (1, 1)];

/// His own state.
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    /// ExtraVars[0]: where in its row's cycle the aim is.
    pub cycle: u8,
    /// ExtraVars[1]: the aim.
    pub aim: u8,
    /// The controller waiting for him (its pointer sits in his
    /// CollisionDataPtr slot).
    pub controller: Option<ObjectRef>,
    pub damage: u32,
}

fn vars(b: &Battle, r: ObjectRef) -> &Vars {
    match &b.objects.get(r).vars {
        crate::kinds::Vars::EraseMan(v) => v,
        v => panic!("EraseMan with {v:?}"),
    }
}

fn vars_mut(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut b.objects.get_mut(r).vars {
        crate::kinds::Vars::EraseMan(v) => v,
        v => panic!("EraseMan with {v:?}"),
    }
}

/// `sub_80BB7F6`: EraseMan for `user`, on `panel`, reporting to
/// `controller`. Param1 of `params` is the ticks per aim.
pub fn spawn(
    b: &mut Battle,
    user: ObjectRef,
    controller: ObjectRef,
    panel: PanelPos,
    element: u8,
    params: [u8; 4],
    damage: u32,
) -> Option<ObjectRef> {
    let r = b.objects.spawn(Pool::Actor, INDEX, Vec3::default(), params)?;
    let (alliance, flip) = {
        let o = b.objects.get(user);
        (o.alliance, o.flip)
    };
    let o = b.objects.get_mut(r);
    o.panel = panel;
    o.element = element;
    o.related[0] = Some(user);
    o.alliance = alliance;
    o.flip = flip;
    o.damage = damage as u16;
    o.stamina = (damage >> 16) as u16;
    o.vars = crate::kinds::Vars::EraseMan(Vars { controller: Some(controller), damage, ..Vars::default() });
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => match b.objects.get(r).action {
            0 => appear(b, r),
            4 => stand(b, r),
            8 => aim(b, r),
            0xC => raise(b, r),
            0x10 => slash(b, r),
            _ => leave(b, r),
        },
        _ => b.objects.free(r),
    }
    common::update_sprite_in_time_stop(b, r);
}

/// `sub_80BB62C`.
fn init(b: &mut Battle, r: ObjectRef) {
    common::set_coordinates_from_panels(b, r);
    let flip = {
        let o = b.objects.get_mut(r);
        o.pos.z = 0;
        o.alliance ^ o.flip
    };
    let s = b.objects.sprite_mut(r);
    s.load(SPRITE);
    s.look.shadow = Shadow::Ground;
    s.look.palette = 0;
    s.look.set_flip(flip);
    s.set_animation(0);
    let o = b.objects.get_mut(r);
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.anim = 0;
    o.anim_loaded = 0;
    common::set_progress(b, r, common::Progress::UPDATE);
}

/// These actions enter on phase 0 (setting it to 4) and keep Timer2 in
/// the Timer word.
fn enter(b: &mut Battle, r: ObjectRef, anim: u8, ticks: u16) -> bool {
    let o = b.objects.get_mut(r);
    if o.phase != 0 {
        return false;
    }
    o.phase = 4;
    o.anim = anim;
    o.timer = ticks;
    o.timer2 = 0;
    true
}

/// Count the timer down; true at 0 or below.
fn ran_out(b: &mut Battle, r: ObjectRef) -> bool {
    let o = b.objects.get_mut(r);
    let left = o.timer as i32 - 1;
    o.timer = left as u16;
    left <= 0
}

/// `sub_80BB694`: he appears; 3 ticks; on to standing, or straight to
/// leaving if his panel can't hold him.
fn appear(b: &mut Battle, r: ObjectRef) {
    if enter(b, r, 3, 3) {
        b.play_sound(crate::sound::SoundId(0x94));
        b.objects.get_mut(r).flags |= flags::VISIBLE;
        return;
    }
    if !ran_out(b, r) {
        return;
    }
    let p = b.objects.get(r).panel;
    let next = if b.field.flags(p.x, p.y) & 0x1_0010 == 0x1_0010 { 4 } else { 0x14 };
    common::set_action(b, r, next);
}

/// `sub_80BB6E8`: 30 ticks standing.
fn stand(b: &mut Battle, r: ObjectRef) {
    if enter(b, r, 0, 0x1E) {
        return;
    }
    if ran_out(b, r) {
        common::set_action(b, r, 8);
    }
}

/// `sub_80BB866`: the next aim of his row's cycle.
fn next_aim(b: &mut Battle, r: ObjectRef) {
    let row = AIMS_BY_ROW[b.objects.get(r).panel.y as usize - 1];
    let v = vars_mut(b, r);
    let i = v.cycle as usize + 1;
    let (aim, cycle) = match row.get(i) {
        Some(&a) => (a, i as u8),
        None => (row[0], 0),
    };
    v.aim = aim;
    v.cycle = cycle;
}

/// The panels of his aim, up to five, while on the field.
fn line(b: &Battle, r: ObjectRef) -> Vec<PanelPos> {
    let o = b.objects.get(r);
    let front = common::facing(o.alliance, o.flip);
    let (dx, dy) = AIM_STEPS[vars(b, r).aim as usize];
    (1..=5)
        .map(|i| (o.panel.x as i32 + dx * front * i, o.panel.y as i32 + dy * i))
        .take_while(|&(x, y)| field::is_valid(x as u8, y as u8))
        .map(|(x, y)| PanelPos { x: x as u8, y: y as u8 })
        .collect()
}

/// `sub_80BB81C`: mark the aim's panels.
fn mark(b: &mut Battle, r: ObjectRef) {
    let ticks = b.objects.get(r).params[0];
    for p in line(b, r) {
        erase_mark::spawn(b, r, p, ticks);
    }
    b.play_sound(crate::sound::SoundId(0x10E));
}

/// `sub_80BB710`: aim, switching every Param1 ticks, until the user
/// presses A in the time stop, or for 360 ticks.
fn aim(b: &mut Battle, r: ObjectRef) {
    let period = b.objects.get(r).params[0] as u16;
    if b.objects.get(r).phase == 0 {
        b.objects.get_mut(r).phase = 4;
        vars_mut(b, r).cycle = 3;
        next_aim(b, r);
        mark(b, r);
        let o = b.objects.get_mut(r);
        o.timer = 0x168;
        o.timer2 = period;
        return;
    }
    let user = b.objects.get(r).related[0].expect("EraseMan has a user");
    let a = b.objects.get(user).actor.expect("the user has actor data");
    if b.actors.get(a).timestop_pad.pressed & keys::A != 0 || ran_out(b, r) {
        return common::set_action(b, r, 0xC);
    }
    let o = b.objects.get_mut(r);
    let left = o.timer2 as i32 - 1;
    o.timer2 = left as u16;
    if left > 0 {
        return;
    }
    next_aim(b, r);
    mark(b, r);
    b.objects.get_mut(r).timer2 = period;
}

/// `sub_80BB772`: 10 ticks with the scythe raised.
fn raise(b: &mut Battle, r: ObjectRef) {
    if enter(b, r, 0x11, 0xA) {
        return;
    }
    if ran_out(b, r) {
        common::set_action(b, r, 0x10);
    }
}

/// `sub_80BB79A`: the slash along the aim (a beam on each panel, lasting
/// 60 ticks); 60 ticks.
fn slash(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase == 0 {
        b.objects.get_mut(r).phase = 4;
        b.objects.get_mut(r).anim = 0x12;
        // sub_80BB8B0
        let (aim, damage, element) = {
            let o = b.objects.get(r);
            (vars(b, r).aim, vars(b, r).damage, o.element)
        };
        for p in line(b, r) {
            erase_beam::spawn(b, r, p, element, aim, 0x3C, damage);
        }
        b.play_sound(crate::sound::SoundId(0xBA));
        b.objects.get_mut(r).timer = 0x3C;
        return;
    }
    if ran_out(b, r) {
        common::set_action(b, r, 0x14);
    }
}

/// `sub_80BB7CC`: 3 ticks, then he goes and the controller moves on.
fn leave(b: &mut Battle, r: ObjectRef) {
    if enter(b, r, 4, 3) {
        return;
    }
    let o = b.objects.get_mut(r);
    let left = o.timer as i32 - 1;
    o.timer = left as u16;
    if left >= 0 {
        return;
    }
    if let Some(c) = vars(b, r).controller {
        navi_chip::navi_left(b, c);
    }
    common::set_progress(b, r, common::Progress::DESTROY);
}
