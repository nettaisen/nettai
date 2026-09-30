//! An afterimage (effect object #0x28, `sub_80E32B8`): a blinking copy of
//! a navi's sprite left behind when the Beast Out rush warps it. Purely
//! visual, but it holds an effect slot and a place in the update order,
//! wears the navi's form overlay (its own form overlay object), and ends
//! early when the navi's form or action changes. See
//! docs/engine/objects-and-player.md §A.7.
//!
//! A plain afterimage (first parameter not 0xFF) shows a sprite of its own
//! instead, looking as its spawner says (`PlainLook`): content spawns
//! these (`spawn_plain`), such as PitHocky's puck trail.

use crate::battle::Battle;
use crate::content::{Content, SpriteId};
use crate::kinds::common::{Progress, set_progress};
use crate::kinds::player::form;
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
    /// A plain afterimage's look.
    pub plain: PlainLook,
}

/// How a plain afterimage looks: what its spawner puts in its extra
/// variables (`sub_80E33FA`'s callers).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PlainLook {
    /// ExtraVars+0: the colour shader.
    pub color_shader: u16,
    /// ExtraVars+6 and +7: a shadow at its height (neither set), on the
    /// ground (both), or none (only the first).
    pub shadow: PlainShadow,
    /// ExtraVars+0x10: the palette.
    pub palette: u8,
    /// ExtraVars+8: shown steadily instead of blinking.
    pub steady: bool,
}

/// A plain afterimage's shadow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PlainShadow {
    #[default]
    WithSprite,
    Ground,
    Hidden,
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
    *vars(b, r) = Vars { lifetime, tether, anim, ..Default::default() };
    Some(r)
}

/// `sub_80E33FA` with a sprite of its own: a plain afterimage of `owner`'s
/// side at `pos`, showing `anim` of `sprite` (flipped by the game's flip
/// value `flip`) for `lifetime` ticks. It runs while paused.
#[allow(clippy::too_many_arguments)]
pub fn spawn_plain(
    b: &mut Battle,
    owner: ObjectRef,
    pos: Vec3,
    sprite: SpriteId,
    anim: u8,
    flip: u8,
    lifetime: u16,
    tether: Tether,
    look: PlainLook,
) -> Option<ObjectRef> {
    let alliance = b.objects.get(owner).alliance;
    let r = b.objects.spawn(Pool::Effect, INDEX, pos, [sprite.category, sprite.index, anim, flip])?;
    let o = b.objects.get_mut(r);
    o.related[0] = Some(owner);
    o.alliance = alliance;
    o.flags |= flags::RUN_WHILE_PAUSED;
    *vars(b, r) = Vars { lifetime, tether, anim, plain: look };
    Some(r)
}

/// Whether it is plain (its first parameter isn't 0xFF).
fn is_plain(b: &Battle, r: ObjectRef) -> bool {
    b.objects.get(r).params[0] != 0xFF
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
        // Only players' Beast Out rush leaves afterimages.
        _ => unreachable!("an afterimage of NameID {name_id:#x}, which is not a player's"),
    }
}

/// `sub_80E32D8`: copy the owner's NameID and sprite, put on its form's
/// overlay, and show `anim`.
fn init(b: &mut Battle, r: ObjectRef) {
    if is_plain(b, r) {
        return init_plain(b, r);
    }
    b.objects.get_mut(r).flags |= flags::VISIBLE;
    let owner = b.objects.get(r).related[0].expect("afterimage has an owner");
    let name_id = b.objects.get(owner).name_id;
    b.objects.get_mut(r).name_id = name_id;
    b.objects.sprite_mut(r).load(player_sprite(&b.content, name_id));
    b.objects.get_mut(r).flags &= !flags::NO_SPRITE_UPDATE;
    put_on_layer(b, r, name_id);
    let anim = vars(b, r).anim;
    let lifetime = vars(b, r).lifetime;
    let flip = b.objects.get(r).params[3];
    let s = b.objects.sprite_mut(r);
    s.set_animation(anim, &b.content);
    s.update(&b.content);
    // A ground shadow, the fourth parameter's flip and the spawner's
    // colour shader (0x83E0: less green).
    s.look.shadow = crate::object::sprite::Shadow::Ground;
    s.look.set_flip(flip);
    s.look.color_shader = COLOR_SHADER;
    let o = b.objects.get_mut(r);
    o.anim = anim;
    o.anim_loaded = anim;
    o.timer2 = lifetime;
    o.timer = 0;
    set_progress(b, r, Progress::UPDATE);
    tick(b, r);
}

/// `sub_80E32D8` for a plain afterimage: its own sprite (the first two
/// parameters), animation (the third) and flip (the fourth), and the look
/// its spawner gave it.
fn init_plain(b: &mut Battle, r: ObjectRef) {
    use crate::object::sprite::Shadow;
    b.objects.get_mut(r).flags |= flags::VISIBLE;
    let [category, index, anim, flip] = b.objects.get(r).params;
    let Vars { lifetime, plain, .. } = *vars(b, r);
    let s = b.objects.sprite_mut(r);
    s.load(SpriteId { category, index });
    s.set_animation(anim, &b.content);
    s.update(&b.content);
    s.look.shadow = match plain.shadow {
        PlainShadow::WithSprite => Shadow::WithSprite,
        PlainShadow::Ground => Shadow::Ground,
        PlainShadow::Hidden => Shadow::Hidden,
    };
    s.look.palette = plain.palette;
    s.look.set_flip(flip);
    s.look.color_shader = plain.color_shader;
    let o = b.objects.get_mut(r);
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.anim = anim;
    o.anim_loaded = anim;
    o.timer2 = lifetime;
    o.timer = 0;
    set_progress(b, r, Progress::UPDATE);
    tick(b, r);
}

/// `sub_8010DF6(record, 0)` then `sub_80C4526(layer, 1)`: the overlay the
/// NameID's init hook puts on, on the afterimage, pinned in front.
fn put_on_layer(b: &mut Battle, r: ObjectRef, name_id: u16) {
    form::navi_init_hook(b, r, name_id);
    // (With no overlay the game's store lands in BIOS memory.)
    if let Some(layer) = b.objects.get(r).related[1] {
        form::pin_overlay(b, layer);
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
    let steady = vars(b, r).plain.steady;
    let o = b.objects.get_mut(r);
    o.flags |= flags::VISIBLE;
    if !steady && timer & 2 == 0 {
        o.flags &= !flags::VISIBLE;
    }
}

/// The destroy state, then at once `sub_80E33D2`: the overlay keeps
/// running while paused, the NameID's teardown takes it off, and the
/// afterimage is freed.
fn destroy(b: &mut Battle, r: ObjectRef) {
    set_progress(b, r, Progress::DESTROY);
    if is_plain(b, r) {
        // NameID 0's teardown (sub_8011044) does nothing.
        b.objects.free(r);
        return;
    }
    if let Some(layer) = b.objects.get(r).related[1] {
        b.objects.get_mut(layer).flags |= flags::RUN_WHILE_PAUSED;
    }
    // sub_8011044(record, 1): the NameID's death hook takes the overlay
    // off.
    let name_id = b.objects.get(r).name_id;
    form::navi_death_hook(b, r, name_id);
    b.objects.free(r);
}
