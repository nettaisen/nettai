//! An afterimage (effect object #0x28, `sub_80E32B8`): a blinking copy of
//! a navi's sprite left behind when the Beast Out rush warps it. Purely
//! visual, but it holds an effect slot and a place in the update order,
//! wears the navi's form overlay (its own form overlay object), and ends
//! early when the navi's form or action changes. See
//! docs/engine/objects-and-player.md §A.7.
//!
//! A plain afterimage (first parameter not 0xFF) shows a sprite of its own
//! instead, looking as its spawner says (`PlainLook`): content spawns
//! these (`spawn_plain`), such as PitHocky's puck trail. Content also
//! spawns copies with a look of its own (`spawn_copy`), such as the step
//! sword's afterimages of its user.

use nettai_content_api::IdentityHandle;
use crate::battle::Battle;
use crate::content::{Content, SpriteId};
use crate::kinds::common::{Progress, set_progress};
use crate::kinds::player::form;
use crate::object::{ObjectRef, Vec3, flags, state};

/// The color shader `sub_80EAFC2` gives its afterimages.
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
    /// Its look (a plain one's, or a copy's that its spawner chose).
    pub plain: PlainLook,
}

/// How a plain afterimage looks: what its spawner puts in its extra
/// variables (`sub_80E33FA`'s callers).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PlainLook {
    /// ExtraVars+0: the color shader.
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

impl PlainShadow {
    fn shadow(self) -> crate::object::sprite::Shadow {
        use crate::object::sprite::Shadow;
        match self {
            PlainShadow::WithSprite => Shadow::WithSprite,
            PlainShadow::Ground => Shadow::Ground,
            PlainShadow::Hidden => Shadow::Hidden,
        }
    }
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
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::Afterimage, pos, [0xFF, 0, anim, alliance ^ flip])?;
    let o = b.objects.get_mut(r);
    o.related[0] = Some(owner);
    o.alliance = alliance;
    o.flags |= flags::RUN_WHILE_PAUSED;
    // sub_80E341E: tied to the Beast form, or to the attack.
    let tether = if b.form(alliance as usize).kind.is_beast() { Tether::BeastForm } else { Tether::Attack };
    // Less green, with a ground shadow (the spawner's r7 is 0x01010014 - n).
    let look = PlainLook { color_shader: COLOR_SHADER, shadow: PlainShadow::Ground, ..Default::default() };
    *vars(b, r) = Vars { lifetime, tether, anim, plain: look };
    Some(r)
}

/// `sub_80E33FA` with Param1 0xFF: a copy of `owner` (its NameID's sprite
/// and form overlay) at `pos`, holding `anim` flipped by the game's flip
/// value `flip`, for `lifetime` ticks, looking as `look` says (the step
/// sword's). It runs while paused.
pub fn spawn_copy(
    b: &mut Battle,
    owner: ObjectRef,
    pos: Vec3,
    anim: u8,
    flip: u8,
    lifetime: u16,
    tether: Tether,
    look: PlainLook,
) -> Option<ObjectRef> {
    let alliance = b.objects.get(owner).alliance;
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::Afterimage, pos, [0xFF, 0, anim, flip])?;
    let o = b.objects.get_mut(r);
    o.related[0] = Some(owner);
    o.alliance = alliance;
    o.flags |= flags::RUN_WHILE_PAUSED;
    *vars(b, r) = Vars { lifetime, tether, anim, plain: look };
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
    // (The sprite's handle in the first two parameters.)
    let [hi, lo] = sprite.0.to_be_bytes();
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::Afterimage, pos, [hi, lo, anim, flip])?;
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

/// A player navi's battle sprite by its identity (`sub_800F26C`, which
/// reads the same sprites the form and navi tables give): MegaMan's base
/// form's, a link navi's, a form's.
fn player_sprite(content: &Content, identity: Option<IdentityHandle>) -> SpriteId {
    use crate::content::IdentityOwner;
    let id = content.identity(identity);
    match id.owner {
        Some(IdentityOwner::Navi(n)) => content.navi_sprite(n, content.base_form_for(n)),
        Some(IdentityOwner::Form(f)) => content.form(f).sprite,
        // Only players' Beast Out rush leaves afterimages.
        None => unreachable!("an afterimage of identity {:?}, which is not a player's", id.key),
    }
}

/// `sub_80E32D8`: copy the owner's NameID and sprite, put on its form's
/// overlay, and show `anim`.
fn init(b: &mut Battle, r: ObjectRef) {
    if is_plain(b, r) {
        return init_plain(b, r);
    }
    b.objects.get_mut(r).set_visible(true);
    let owner = b.objects.get(r).related[0].expect("afterimage has an owner");
    let identity = b.objects.get(owner).identity;
    b.objects.get_mut(r).identity = identity;
    b.objects.sprite_mut(r).load(player_sprite(&b.content, identity));
    b.objects.get_mut(r).flags &= !flags::NO_SPRITE_UPDATE;
    put_on_layer(b, r, identity);
    let anim = vars(b, r).anim;
    let lifetime = vars(b, r).lifetime;
    let flip = b.objects.get(r).params[3];
    let look = vars(b, r).plain;
    let s = b.objects.sprite_mut(r);
    s.set_animation(anim, &b.content);
    s.update(&b.content);
    // The spawner's shadow and color shader (the Beast rush's: a ground
    // shadow, 0x83E0 less green), and the fourth parameter's flip.
    s.look.shadow = look.shadow.shadow();
    s.look.set_flip(flip);
    s.look.color_shader = look.color_shader;
    let o = b.objects.get_mut(r);
    o.anim = anim;
    o.anim_loaded = anim;
    o.timer2 = lifetime;
    o.timer = 0;
    set_progress(b, r, Progress::UPDATE);
    tick(b, r);
}

/// `sub_80E32D8` for a plain afterimage: its own sprite (the first two
/// parameters, its handle), animation (the third) and flip (the fourth),
/// and the look its spawner gave it.
fn init_plain(b: &mut Battle, r: ObjectRef) {
    b.objects.get_mut(r).set_visible(true);
    let [hi, lo, anim, flip] = b.objects.get(r).params;
    let Vars { lifetime, plain, .. } = *vars(b, r);
    let s = b.objects.sprite_mut(r);
    s.load(SpriteId(u16::from_be_bytes([hi, lo])));
    s.set_animation(anim, &b.content);
    s.update(&b.content);
    s.look.shadow = plain.shadow.shadow();
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
fn put_on_layer(b: &mut Battle, r: ObjectRef, identity: Option<IdentityHandle>) {
    form::navi_init_hook(b, r, identity);
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
        Tether::BeastForm => !b.form(alliance as usize).kind.is_beast(),
        Tether::Attack => !crate::kinds::player::navi_action(b, owner).is_attack(),
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
    o.set_visible(true);
    if !steady && timer & 2 == 0 {
        o.set_visible(false);
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
    let identity = b.objects.get(r).identity;
    form::navi_death_hook(b, r, identity);
    b.objects.free(r);
}
