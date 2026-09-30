//! ElmntMan, the navi of the ElmntMan chip (actor object #0x10,
//! `sub_80BAA8C`), brought by its time-freeze controller (`navi_chip`).
//! He appears, cycles through the four elements until his user presses A
//! (or a while passes, then one at random), and attacks with it: Fire
//! drops a meteor on every enemy navi, Aqua, Elec and Wood are not
//! ported yet. Then he leaves and tells the controller.
//! See docs/engine/chips.md §3.6.7.

use crate::battle::Battle;
use crate::content::SpriteId;
use crate::input::keys;
use crate::kinds::{body_overlay, common, meteor, navi_chip};
use crate::kinds::player::form;
use crate::object::sprite::Shadow;
use crate::object::{ObjectRef, PanelPos, Pool, Vec3, flags, state};

pub const INDEX: u8 = 0x10;

/// Its index among the navi chips' navis (`off_802CD5C`, the chip's
/// subtype).
pub const NAVI: u8 = 0x10;

const SPRITE: SpriteId = SpriteId { category: 8, index: 0x10 };

/// ElmntMan's NameID (the death hook takes down his overlay).
const NAME_ID: u16 = 0x1A0 + 0x10;

/// The elements, in the order he cycles through them, by palette
/// (`dword_80BAC4C`).
const ELEMENT_PALETTES: [u8; 4] = [2, 4, 8, 6];

/// His own state.
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    /// ExtraVars[0]: the element shown (0 Fire, 1 Aqua, 2 Elec, 3 Wood).
    pub element: u8,
    /// Object +0x64: the panels of the enemy navis (Fire).
    pub targets: Vec<PanelPos>,
    /// Param3: attacks made.
    pub shots: u8,
    /// The controller waiting for him (its pointer sits in his
    /// CollisionDataPtr slot).
    pub controller: Option<ObjectRef>,
    /// The user's side's damage word.
    pub damage: u32,
}

fn vars(b: &Battle, r: ObjectRef) -> &Vars {
    match &b.objects.get(r).vars {
        crate::kinds::Vars::ElmntMan(v) => v,
        v => panic!("ElmntMan with {v:?}"),
    }
}

fn vars_mut(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut b.objects.get_mut(r).vars {
        crate::kinds::Vars::ElmntMan(v) => v,
        v => panic!("ElmntMan with {v:?}"),
    }
}

/// `sub_80BAE16`: ElmntMan for `user`, on `panel`, reporting to
/// `controller`. Param1 of `params` is the ticks per element.
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
    o.vars = crate::kinds::Vars::ElmntMan(Vars { controller: Some(controller), damage, ..Vars::default() });
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => match b.objects.get(r).action {
            0 => appear(b, r),
            4 => stand(b, r),
            8 => cycle(b, r),
            0xC => wind_up(b, r),
            0x10 => attack(b, r),
            0x14 => wind_down(b, r),
            _ => leave(b, r),
        },
        _ => b.objects.free(r),
    }
    common::update_sprite_in_time_stop(b, r);
}

/// `sub_80BAAB0`: on his panel, with his overlay (actor #0x56 variant
/// 0xF, animation offset 9, stepping even while paused).
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
    s.set_animation(0, &b.content);
    let o = b.objects.get_mut(r);
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.anim = 0;
    o.anim_loaded = 0;
    // sub_8010DF6(2, 0x10, 1) → sub_8011004.
    let overlay = body_overlay::Vars {
        variant: 0x0F,
        own_palette: true,
        always_step: true,
        anim_offset: 9,
        ..Default::default()
    };
    let o = body_overlay::spawn_with(b, r, overlay);
    b.objects.get_mut(r).related[1] = o;
    common::set_progress(b, r, common::Progress::UPDATE);
}

/// Enter the phase and set the timer, or count it down; true once it is
/// 0 or below.
fn countdown(b: &mut Battle, r: ObjectRef, ticks: u16) -> bool {
    let o = b.objects.get_mut(r);
    if o.phase_init == 0 {
        o.phase_init = 4;
        o.timer = ticks;
        return false;
    }
    let left = o.timer as i32 - 1;
    o.timer = left as u16;
    left <= 0
}

/// `sub_80BAB30`: he appears; 2 ticks later, on to standing, or straight
/// to leaving if his panel can't hold him.
fn appear(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        let o = b.objects.get_mut(r);
        o.anim = 3;
        o.flags |= flags::VISIBLE;
        b.play_sound(crate::sound::SoundId(0x94));
    }
    if !countdown(b, r, 2) {
        return;
    }
    let p = b.objects.get(r).panel;
    let next = if b.field.flags(p.x, p.y) & 0x1_0010 == 0x1_0010 { 4 } else { 0x18 };
    common::set_action(b, r, next);
}

/// `sub_80BAB84`: 10 ticks standing.
fn stand(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        b.objects.get_mut(r).anim = 0;
    }
    if countdown(b, r, 10) {
        common::set_action(b, r, 8);
    }
}

fn show_element(b: &mut Battle, r: ObjectRef) {
    let e = vars(b, r).element;
    b.objects.sprite_mut(r).look.palette = ELEMENT_PALETTES[e as usize];
}

/// `sub_80BABAC`: cycle the elements every Param1 ticks until the user
/// presses A in the time stop; after 20 cycles' worth, one at random (a
/// simulation RNG draw).
fn cycle(b: &mut Battle, r: ObjectRef) {
    let period = b.objects.get(r).params[0] as u16;
    if b.objects.get(r).phase_init == 0 {
        vars_mut(b, r).element = 0;
        show_element(b, r);
        b.play_sound(crate::sound::SoundId(0x134));
        let o = b.objects.get_mut(r);
        o.phase_init = 4;
        o.timer = period;
        o.timer2 = period.wrapping_mul(20);
        return;
    }
    let user = b.objects.get(r).related[0].expect("ElmntMan has a user");
    let a = b.objects.get(user).actor.expect("the user has actor data");
    if b.actors.get(a).timestop_pad.pressed & keys::A != 0 {
        b.play_sound(crate::sound::SoundId(0x182));
        return common::set_action(b, r, 0xC);
    }
    let o = b.objects.get_mut(r);
    let left = o.timer as i32 - 1;
    o.timer = left as u16;
    if left <= 0 {
        let v = vars_mut(b, r);
        v.element = (v.element + 1) & 3;
        show_element(b, r);
        b.play_sound(crate::sound::SoundId(0x134));
        b.objects.get_mut(r).timer = period;
    }
    let o = b.objects.get_mut(r);
    let left = o.timer2 as i32 - 1;
    o.timer2 = left as u16;
    if left > 0 {
        return;
    }
    vars_mut(b, r).element = (b.rng.next_positive() & 3) as u8;
    show_element(b, r);
    common::set_action(b, r, 0xC);
}

/// `sub_80BAC50`: 35 ticks of wind-up (Wood first turns the field to
/// grass).
fn wind_up(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        let o = b.objects.get_mut(r);
        o.phase_init = 4;
        o.anim = 5;
        o.timer = 0x23;
        if vars(b, r).element == 3 {
            panic!("ElmntMan's Wood (sub_80BAF06) is not implemented yet");
        }
        return;
    }
    let o = b.objects.get_mut(r);
    let left = o.timer as i32 - 1;
    o.timer = left as u16;
    if left == 0x10 {
        o.anim = 7;
        return;
    }
    if left > 0 {
        return;
    }
    let element = vars(b, r).element;
    common::set_action(b, r, 0x10);
    b.objects.get_mut(r).phase = element * 4;
}

/// `sub_80BAC98`: the element's attack.
fn attack(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).phase {
        0 => fire(b, r),
        4 => panic!("ElmntMan's Aqua (sub_80BAD06) is not implemented yet"),
        8 => panic!("ElmntMan's Elec (sub_80BAD76) is not implemented yet"),
        _ => panic!("ElmntMan's Wood (sub_80BAD34) is not implemented yet"),
    }
}

/// `sub_80BACBC`: a meteor on each enemy navi's panel, 12 ticks apart.
fn fire(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        b.objects.get_mut(r).phase_init = 4;
        let targets = enemy_panels(b, r);
        if targets.is_empty() {
            return common::set_action(b, r, 0x14);
        }
        let v = vars_mut(b, r);
        v.targets = targets;
        v.shots = 0;
        b.objects.get_mut(r).timer = 0;
        return;
    }
    let o = b.objects.get_mut(r);
    let left = o.timer as i32 - 1;
    o.timer = left as u16;
    if left > 0 {
        return;
    }
    let (target, damage) = {
        let v = vars(b, r);
        (v.targets[v.shots as usize], v.damage)
    };
    meteor::spawn(b, r, target, damage);
    let v = vars_mut(b, r);
    v.shots += 1;
    if v.shots as usize >= v.targets.len() {
        return common::set_action(b, r, 0x14);
    }
    b.objects.get_mut(r).timer = 0xC;
}

/// `sub_80BAE3C` (`object_getPanelsExceptCurrentFiltered`): the panels
/// holding an enemy navi's body, from the back row and column.
fn enemy_panels(b: &Battle, r: ObjectRef) -> Vec<PanelPos> {
    use crate::field::pflags;
    let o = b.objects.get(r);
    let enemy = if o.alliance == 0 { pflags::BODY_SIDE1 } else { pflags::BODY_SIDE0 };
    let mut out = Vec::new();
    for y in (1..=3u8).rev() {
        for x in (1..=6u8).rev() {
            if b.field.flags(x, y) & enemy == enemy && (x, y) != (o.panel.x, o.panel.y) {
                out.push(PanelPos { x, y });
            }
        }
    }
    out
}

/// `sub_80BADB4`: 20 ticks, back to standing on the 15th.
fn wind_down(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        let o = b.objects.get_mut(r);
        o.phase_init = 4;
        o.timer = 0x14;
        return;
    }
    let o = b.objects.get_mut(r);
    let left = o.timer as i32 - 1;
    o.timer = left as u16;
    if left == 5 {
        o.anim = 8;
        return;
    }
    if left <= 0 {
        common::set_action(b, r, 0x18);
    }
}

/// `sub_80BADE4`: 2 ticks, then he goes (with his overlay) and the
/// controller moves on.
fn leave(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        b.objects.get_mut(r).anim = 4;
    }
    if !countdown(b, r, 2) {
        return;
    }
    // sub_8011044(2, 0x10)
    form::navi_death_hook(b, r, NAME_ID);
    if let Some(c) = vars(b, r).controller {
        navi_chip::navi_left(b, c);
    }
    common::set_progress(b, r, common::Progress::DESTROY);
}
