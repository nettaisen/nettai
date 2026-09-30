//! An afterimage (effect object #0x28, `sub_80E32B8`): a blinking copy of
//! a navi's sprite left behind when the Beast Out rush warps it (Param1
//! 0xFF), or of a given sprite (a navi chip's shot's trail). Purely
//! visual, but it holds an effect slot and a place in the update order;
//! a navi's wears the navi's form overlay (its own form overlay object),
//! and ends early when the navi's form or action changes. See
//! docs/engine/objects-and-player.md §A.7.

use crate::battle::Battle;
use crate::content::{Content, SpriteId};
use crate::kinds::common::{Progress, set_progress};
use crate::kinds::form_overlay;
use crate::object::sprite::Shadow;
use crate::object::{ObjectRef, Pool, Vec3, flags, state};
use crate::setup::Form;

pub const INDEX: u8 = 0x28;

/// The colour shader `sub_80EAFC2` gives its afterimages.
const COLOR_SHADER: u16 = 0x83E0;

/// What ends an afterimage before its time is up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Tether {
    /// Nothing.
    #[default]
    None,
    /// The navi's side leaving its Beast forms.
    BeastForm,
    /// The navi going back to a non-attack action.
    Attack,
}

/// Afterimage-private state.
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    /// Ticks it lasts.
    pub lifetime: u16,
    pub tether: Tether,
    /// The animation it holds.
    pub anim: u8,
    /// Its own sprite (Param1/Param2), or none: a copy of its owner, a
    /// navi (Param1 0xFF).
    pub sprite: Option<SpriteId>,
    /// ExtraVars[0]: its colour shader.
    pub color_shader: u16,
    /// Its shadow (ExtraVars[1]'s upper bytes).
    pub shadow: Shadow,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut b.objects.get_mut(r).vars {
        crate::kinds::Vars::Afterimage(v) => v,
        v => panic!("afterimage with {v:?}"),
    }
}

/// `sub_80EAFC2` (via `sub_80E33FA`): an afterimage of `owner` at `pos`
/// lasting `lifetime` ticks, holding `anim`.
pub fn spawn(b: &mut Battle, owner: ObjectRef, pos: Vec3, anim: u8, lifetime: u16) -> Option<ObjectRef> {
    let (alliance, flip) = {
        let o = b.objects.get(owner);
        (o.alliance, o.flip)
    };
    // Param1 0xFF: copy the owner's sprite; Param4 its facing
    // (`object_getFlip`).
    let r = b.objects.spawn(Pool::Effect, INDEX, pos, [0xFF, 0, anim, alliance ^ flip])?;
    let o = b.objects.get_mut(r);
    o.related[0] = Some(owner);
    o.alliance = alliance;
    o.flags |= flags::RUN_WHILE_PAUSED;
    // sub_80E341E: tied to the Beast form, or to the attack.
    let tether = if b.stats[alliance as usize].form.is_beast() { Tether::BeastForm } else { Tether::Attack };
    *vars(b, r) = Vars { lifetime, tether, anim, sprite: None, color_shader: COLOR_SHADER, shadow: Shadow::Ground };
    Some(r)
}

/// `sub_80E33FA` with a sprite of its own: an afterimage of `sprite` at
/// `pos` for `owner`, holding `anim` with `flip`, lasting `lifetime` ticks
/// (ExtraVars[1]'s low half) with `color_shader` (ExtraVars[0]) and
/// `shadow`; it blinks, and nothing ends it early.
#[allow(clippy::too_many_arguments)]
pub fn spawn_sprite(
    b: &mut Battle,
    owner: ObjectRef,
    pos: Vec3,
    sprite: SpriteId,
    anim: u8,
    flip: u8,
    color_shader: u16,
    lifetime: u16,
    shadow: Shadow,
) -> Option<ObjectRef> {
    let alliance = b.objects.get(owner).alliance;
    let r = b.objects.spawn(Pool::Effect, INDEX, pos, [sprite.category, sprite.index, anim, flip])?;
    let o = b.objects.get_mut(r);
    o.related[0] = Some(owner);
    o.alliance = alliance;
    o.flags |= flags::RUN_WHILE_PAUSED;
    *vars(b, r) = Vars { lifetime, tether: Tether::None, anim, sprite: Some(sprite), color_shader, shadow };
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => tick(b, r),
        _ => b.objects.free(r),
    }
}

/// A player navi's battle sprite by NameID (`sub_800F26C`, which reads the
/// same sprites the form and navi tables give).
fn player_sprite(content: &Content, name_id: u16) -> SpriteId {
    match name_id {
        0x1A0 => content.form(Form::NONE).sprite,
        0x1A1..=0x1AB => content.navi(crate::setup::Navi((name_id - 0x1A0) as u8)).sprite,
        0x1AC..=0x1C3 => content.form(Form((name_id - 0x1AB) as u8)).sprite,
        _ => panic!("afterimages of NameID {name_id:#x} are not implemented yet"),
    }
}

/// `sub_80E32D8`: copy the owner's NameID and sprite, put on its form's
/// overlay, and show `anim`.
fn init(b: &mut Battle, r: ObjectRef) {
    b.objects.get_mut(r).flags |= flags::VISIBLE;
    match vars(b, r).sprite {
        Some(sprite) => b.objects.sprite_mut(r).load(sprite),
        None => {
            let owner = b.objects.get(r).related[0].expect("afterimage has an owner");
            let name_id = b.objects.get(owner).name_id;
            b.objects.get_mut(r).name_id = name_id;
            b.objects.sprite_mut(r).load(player_sprite(&b.content, name_id));
        }
    }
    b.objects.get_mut(r).flags &= !flags::NO_SPRITE_UPDATE;
    if vars(b, r).sprite.is_none() {
        let name_id = b.objects.get(r).name_id;
        put_on_layer(b, r, name_id);
    }
    let anim = vars(b, r).anim;
    let lifetime = vars(b, r).lifetime;
    let (color_shader, shadow) = (vars(b, r).color_shader, vars(b, r).shadow);
    let flip = b.objects.get(r).params[3];
    let s = b.objects.sprite_mut(r);
    s.set_animation(anim, &b.content);
    s.update(&b.content);
    // Its shadow, the fourth parameter's flip and the spawner's colour
    // shader (the rush's 0x83E0: less green).
    s.look.shadow = shadow;
    s.look.set_flip(flip);
    s.look.color_shader = color_shader;
    let o = b.objects.get_mut(r);
    o.anim = anim;
    o.anim_loaded = anim;
    o.timer2 = lifetime;
    o.timer = 0;
    set_progress(b, r, Progress::UPDATE);
    tick(b, r);
}

/// `sub_8010DF6(record, 0)` then `sub_80C4526(layer, 1)`: the overlay the
/// NameID's form wears, on the afterimage.
fn put_on_layer(b: &mut Battle, r: ObjectRef, name_id: u16) {
    match name_id {
        // Base MegaMan and the forms without an overlay (nullsub).
        0x1A0 | 0x1B6 | 0x1B8..=0x1BC | 0x1C2 => {}
        // The Falzar beast head (sub_8011366).
        0x1B7 => {
            let layer = form_overlay::spawn(b, r, form_overlay::BEAST_HEAD, true);
            b.objects.get_mut(r).related[1] = layer;
        }
        _ => panic!("afterimage overlays for NameID {name_id:#x} (sub_8010DF6) are not implemented yet"),
    }
}

/// `sub_80E336E`: blink for the lifetime, unless the tether ends it first.
fn tick(b: &mut Battle, r: ObjectRef) {
    let owner = b.objects.get(r).related[0].expect("afterimage has an owner");
    let tether = vars(b, r).tether;
    let alliance = b.objects.get(r).alliance;
    let cut = match tether {
        Tether::BeastForm => !b.stats[alliance as usize].form.is_beast(),
        Tether::Attack => b.objects.get(owner).action < 0x10,
        Tether::None => false,
    };
    if cut {
        return destroy(b, r);
    }
    if b.paused {
        return;
    }
    let o = b.objects.get(r);
    let timer = o.timer.wrapping_add(1);
    if timer >= o.timer2 {
        return destroy(b, r);
    }
    b.objects.get_mut(r).timer = timer;
    b.objects.sprite_mut(r).update(&b.content);
    let o = b.objects.get_mut(r);
    o.flags |= flags::VISIBLE;
    if timer & 2 == 0 {
        o.flags &= !flags::VISIBLE;
    }
}

/// The destroy state, then at once `sub_80E33D2`: the overlay keeps
/// running while paused, the NameID's teardown takes it off, and the
/// afterimage is freed.
fn destroy(b: &mut Battle, r: ObjectRef) {
    set_progress(b, r, Progress::DESTROY);
    if let Some(layer) = b.objects.get(r).related[1] {
        b.objects.get_mut(layer).flags |= flags::RUN_WHILE_PAUSED;
    }
    // sub_8011044(record, 1): the Falzar beast head comes off (sub_801140E).
    // (One with its own sprite keeps NameID 0, a virus's, with no parts.)
    if vars(b, r).sprite.is_some() {
        b.objects.free(r);
        return;
    }
    match b.objects.get(r).name_id {
        0x1A0 | 0x1B6 | 0x1B8..=0x1BC | 0x1C2 => {}
        0x1B7 => {
            if let Some(layer) = b.objects.get_mut(r).related[1].take() {
                set_progress(b, layer, Progress::DESTROY);
            }
        }
        n => panic!("afterimage teardown for NameID {n:#x} (sub_8011044) is not implemented yet"),
    }
    b.objects.free(r);
}
