//! An afterimage (effect object #0x28, `sub_80E32B8`): a blinking copy of
//! a navi's sprite (or of a given sprite) left behind when the Beast Out
//! rush warps the navi, or when a Cross dashes to slash. Purely visual,
//! but it holds an effect slot and a place in the update order, wears the
//! navi's form overlay (its own overlay object), and can end early when
//! the navi's form or action changes. See docs/engine/objects-and-player.md
//! §A.7.

use crate::battle::Battle;
use crate::content::{Content, SpriteId};
use crate::kinds::common::{Progress, set_progress};
use crate::kinds::{body_overlay, form_overlay};
use crate::object::sprite::Shadow;
use crate::object::{ObjectRef, Pool, Vec3, flags, state};
use crate::setup::Form;

pub const INDEX: u8 = 0x28;

/// The colour shader `sub_80EAFC2` gives its afterimages.
const COLOR_SHADER: u16 = 0x83E0;

/// Param1: copy the owner's sprite (by its NameID) rather than a given
/// one.
const OWNER_SPRITE: u8 = 0xFF;

/// What ends an afterimage before its time is up (`sub_80E341E`,
/// ExtraVars+0xC).
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

/// What `sub_80E33FA` spawns an afterimage with: its parameters and
/// ExtraVars+0 and +4.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Spec {
    /// Param1: 0xFF copies the owner's sprite, else the sprite's category;
    /// Param2 the sprite's index; Param3 the animation; Param4 the flip.
    pub params: [u8; 4],
    pub color_shader: u16,
    /// Ticks it lasts.
    pub lifetime: u16,
    /// Its shadow, from ExtraVars+6 and +7: with the sprite (0), on the
    /// ground (1, 1) or none (1, 0).
    pub shadow: Shadow,
}

/// Afterimage-private state.
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    pub spec: Spec,
    pub tether: Tether,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut b.objects.get_mut(r).vars {
        crate::kinds::Vars::Afterimage(v) => v,
        v => panic!("afterimage with {v:?}"),
    }
}

/// `sub_80EAFC2` (via `sub_80E33FA`): an afterimage of `owner` at `pos`
/// lasting `lifetime` ticks, holding `anim`, tied to the Beast form or the
/// attack.
pub fn spawn(b: &mut Battle, owner: ObjectRef, pos: Vec3, anim: u8, lifetime: u16) -> Option<ObjectRef> {
    let (alliance, flip) = {
        let o = b.objects.get(owner);
        (o.alliance, o.flip)
    };
    // Param1 0xFF: copy the owner's sprite; Param4 its facing
    // (`object_getFlip`).
    let spec = Spec {
        params: [OWNER_SPRITE, 0, anim, alliance ^ flip],
        color_shader: COLOR_SHADER,
        lifetime,
        shadow: Shadow::Ground,
    };
    let r = spawn_with(b, owner, pos, spec)?;
    // sub_80E341E: tied to the Beast form, or to the attack.
    let tether = if b.stats[alliance as usize].form.is_beast() { Tether::BeastForm } else { Tether::Attack };
    vars(b, r).tether = tether;
    Some(r)
}

/// `sub_80E33FA`: an afterimage of `owner` at `pos`. It keeps running while
/// paused.
pub fn spawn_with(b: &mut Battle, owner: ObjectRef, pos: Vec3, spec: Spec) -> Option<ObjectRef> {
    let alliance = b.objects.get(owner).alliance;
    let r = b.objects.spawn(Pool::Effect, INDEX, pos, spec.params)?;
    let o = b.objects.get_mut(r);
    o.related[0] = Some(owner);
    o.alliance = alliance;
    o.flags |= flags::RUN_WHILE_PAUSED;
    *vars(b, r) = Vars { spec, tether: Tether::None };
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

/// `sub_80E32D8`: copy the owner's NameID, sprite and overlay (Param1
/// 0xFF), or load the given sprite; show the animation.
fn init(b: &mut Battle, r: ObjectRef) {
    b.objects.get_mut(r).flags |= flags::VISIBLE;
    let [source, index, anim, flip] = b.objects.get(r).params;
    if source == OWNER_SPRITE {
        let owner = b.objects.get(r).related[0].expect("afterimage has an owner");
        let name_id = b.objects.get(owner).name_id;
        b.objects.get_mut(r).name_id = name_id;
        b.objects.sprite_mut(r).load(player_sprite(&b.content, name_id));
        put_on_layer(b, r, name_id);
    } else {
        b.objects.sprite_mut(r).load(SpriteId { category: source, index });
    }
    b.objects.get_mut(r).flags &= !flags::NO_SPRITE_UPDATE;
    let spec = vars(b, r).spec;
    let s = b.objects.sprite_mut(r);
    s.set_animation(anim, &b.content);
    s.update(&b.content);
    s.look.shadow = spec.shadow;
    // (ExtraVars+0x10's palette is 0: nothing ported sets it.)
    s.look.palette = 0;
    s.look.set_flip(flip);
    s.look.color_shader = spec.color_shader;
    let o = b.objects.get_mut(r);
    o.anim = anim;
    o.anim_loaded = anim;
    o.timer2 = spec.lifetime;
    o.timer = 0;
    set_progress(b, r, Progress::UPDATE);
    tick(b, r);
}

/// `sub_8010DF6(record, 0)` then `sub_80C4526(layer, 1)`: the overlay the
/// NameID's form wears, on the afterimage, drawn a pixel in front.
fn put_on_layer(b: &mut Battle, r: ObjectRef, name_id: u16) {
    match name_id {
        // Base MegaMan and the forms without an overlay (nullsub).
        0x1A0 | 0x1B6 | 0x1B8..=0x1BC | 0x1C2 => {}
        // The Falzar beast head (sub_8011366).
        0x1B7 => {
            let layer = form_overlay::spawn(b, r, form_overlay::BEAST_HEAD, true);
            b.objects.get_mut(r).related[1] = layer;
        }
        // The Crosses' helmets and arms (sub_80112E0 .. sub_801133A).
        0x1AC..=0x1B5 => {
            crate::kinds::player::form::put_on_overlay_stepping(b, r, Form((name_id - 0x1AB) as u8), false);
            if let Some(layer) = b.objects.get(r).related[1] {
                body_overlay::set_forced_front(b, layer);
            }
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

/// The destroy state, then at once `sub_80E33D2`: the overlay (of an
/// owner's copy) keeps running while paused, the NameID's teardown
/// (`sub_8011044(record, 1)`) takes it off, and the afterimage is freed.
fn destroy(b: &mut Battle, r: ObjectRef) {
    set_progress(b, r, Progress::DESTROY);
    if b.objects.get(r).params[0] == OWNER_SPRITE
        && let Some(layer) = b.objects.get(r).related[1]
    {
        b.objects.get_mut(layer).flags |= flags::RUN_WHILE_PAUSED;
    }
    let name_id = b.objects.get(r).name_id;
    match name_id {
        // A given sprite's afterimage keeps NameID 0, whose record is a
        // virus's (a nullsub).
        0 | 0x1A0 | 0x1B6 | 0x1B8..=0x1BC | 0x1C2 => {}
        // The Falzar beast head (sub_801140E) and the Crosses' overlays
        // come off.
        0x1B7 | 0x1AC..=0x1B5 => {
            if let Some(layer) = b.objects.get_mut(r).related[1].take() {
                set_progress(b, layer, Progress::DESTROY);
            }
        }
        n => panic!("afterimage teardown for NameID {n:#x} (sub_8011044) is not implemented yet"),
    }
    b.objects.free(r);
}
