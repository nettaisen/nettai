//! An afterimage (effect object #0x28, `sub_80E32B8`): a blinking copy of
//! a sprite. The Beast Out rush leaves a navi's (Param1 0xFF: the owner's
//! sprite and form overlay) when it warps it; attacks leave their own
//! sprite's (Param1 and Param2 the sprite). Purely visual, but it holds an
//! effect slot and a place in the update order, and a navi's ends early
//! when the navi's form or action changes. See
//! docs/engine/objects-and-player.md §A.7.

use crate::battle::Battle;
use crate::content::{Content, SpriteId};
use crate::kinds::common::{Progress, set_progress};
use crate::kinds::form_overlay;
use crate::object::sprite::Shadow;
use crate::object::{ObjectRef, Pool, Vec3, flags, state};
use crate::setup::Form;

pub const INDEX: u8 = 0x28;

/// Param1 of a navi's afterimage: the owner's sprite.
const OWNERS_SPRITE: u8 = 0xFF;

/// The colour shader `sub_80EAFC2` gives its afterimages.
const COLOR_SHADER: u16 = 0x83E0;

/// What ends an afterimage before its time is up (ExtraVars+0xC,
/// `sub_80E341E`).
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

/// Afterimage-private state (the spawner's words, ExtraVars+0..+0x10).
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    /// +0: its colour shader.
    pub shader: u16,
    /// +4: ticks it lasts.
    pub lifetime: u16,
    /// +6, +7: drawn with a shadow, and that shadow kept on the ground.
    pub shadow: bool,
    pub ground_shadow: bool,
    /// +8: it doesn't blink.
    pub steady: bool,
    /// +0xC.
    pub tether: Tether,
    /// +0x10.
    pub palette: u8,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut b.objects.get_mut(r).vars {
        crate::kinds::Vars::Afterimage(v) => v,
        v => panic!("afterimage with {v:?}"),
    }
}

/// `sub_80E33FA`: an afterimage for `owner`'s side at `pos`, with the
/// spawn parameters `params` (the sprite, or 0xFF for the owner's; the
/// animation; the flip), colour shader `shader`, and `lifetime` (the
/// word's low half: its ticks; byte 2: a shadow; byte 3: kept on the
/// ground). It runs while paused.
pub fn spawn_with(b: &mut Battle, owner: ObjectRef, pos: Vec3, params: [u8; 4], shader: u32, lifetime: u32) -> Option<ObjectRef> {
    let alliance = b.objects.get(owner).alliance;
    let r = b.objects.spawn(Pool::Effect, INDEX, pos, params)?;
    let o = b.objects.get_mut(r);
    o.related[0] = Some(owner);
    o.alliance = alliance;
    o.flags |= flags::RUN_WHILE_PAUSED;
    *vars(b, r) = Vars {
        shader: shader as u16,
        lifetime: lifetime as u16,
        shadow: (lifetime >> 16) as u8 != 0,
        ground_shadow: (lifetime >> 24) as u8 != 0,
        ..Default::default()
    };
    Some(r)
}

/// `sub_80EAFC2`: an afterimage of the navi `owner` at `pos` lasting
/// `lifetime` ticks, holding `anim`, tied to its Beast form or to its
/// attack (`sub_80E341E`).
pub fn spawn(b: &mut Battle, owner: ObjectRef, pos: Vec3, anim: u8, lifetime: u16) -> Option<ObjectRef> {
    let (alliance, flip) = {
        let o = b.objects.get(owner);
        (o.alliance, o.flip)
    };
    // Param4 the owner's facing (`object_getFlip`); a shadow on the
    // ground.
    let params = [OWNERS_SPRITE, 0, anim, alliance ^ flip];
    let r = spawn_with(b, owner, pos, params, COLOR_SHADER as u32, 0x0101_0000 | lifetime as u32)?;
    let tether = if b.stats[alliance as usize].form.is_beast() { Tether::BeastForm } else { Tether::Attack };
    vars(b, r).tether = tether;
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

/// `sub_80E32D8`: its sprite (the owner's, with its NameID and form
/// overlay, or Param1-Param2), animation Param3, the spawner's look and
/// lifetime, flip Param4.
fn init(b: &mut Battle, r: ObjectRef) {
    b.objects.get_mut(r).flags |= flags::VISIBLE;
    let [sprite_category, sprite_index, anim, flip] = b.objects.get(r).params;
    if sprite_category == OWNERS_SPRITE {
        let owner = b.objects.get(r).related[0].expect("afterimage has an owner");
        let name_id = b.objects.get(owner).name_id;
        b.objects.get_mut(r).name_id = name_id;
        b.objects.sprite_mut(r).load(player_sprite(&b.content, name_id));
        b.objects.get_mut(r).flags &= !flags::NO_SPRITE_UPDATE;
        put_on_layer(b, r, name_id);
    } else {
        b.objects.sprite_mut(r).load(SpriteId { category: sprite_category, index: sprite_index });
        b.objects.get_mut(r).flags &= !flags::NO_SPRITE_UPDATE;
    }
    let v = vars(b, r).clone();
    let s = b.objects.sprite_mut(r);
    s.set_animation(anim, &b.content);
    s.update(&b.content);
    // sprite_noShadow, or sprite_hasShadow (then sprite_removeShadow
    // unless kept on the ground).
    s.look.shadow = match (v.shadow, v.ground_shadow) {
        (false, _) => Shadow::WithSprite,
        (true, true) => Shadow::Ground,
        (true, false) => Shadow::Hidden,
    };
    s.look.palette = v.palette;
    s.look.set_flip(flip);
    s.look.color_shader = v.shader;
    let o = b.objects.get_mut(r);
    o.anim = anim;
    o.anim_loaded = anim;
    o.timer2 = v.lifetime;
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
    let steady = vars(b, r).steady;
    let o = b.objects.get_mut(r);
    o.flags |= flags::VISIBLE;
    if !steady && timer & 2 == 0 {
        o.flags &= !flags::VISIBLE;
    }
}

/// The destroy state, then at once `sub_80E33D2`: a navi's overlay keeps
/// running while paused, the NameID's teardown takes it off, and the
/// afterimage is freed.
fn destroy(b: &mut Battle, r: ObjectRef) {
    set_progress(b, r, Progress::DESTROY);
    if b.objects.get(r).params[0] == OWNERS_SPRITE
        && let Some(layer) = b.objects.get(r).related[1]
    {
        b.objects.get_mut(layer).flags |= flags::RUN_WHILE_PAUSED;
    }
    // sub_8011044(record, 1): the Falzar beast head comes off (sub_801140E).
    match b.objects.get(r).name_id {
        // A sprite's afterimage keeps NameID 0, whose record is a virus's
        // (nullsub_43).
        0 => {}
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
