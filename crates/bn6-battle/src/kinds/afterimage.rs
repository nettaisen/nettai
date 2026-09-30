//! An afterimage (effect object #0x28, `sub_80E32B8`): a blinking copy of
//! a navi's sprite left behind when the Beast Out rush warps it, or when a
//! step sword steps (and of the sword's blade, a sprite of its own).
//! Purely visual, but it holds an effect slot and a place in the update
//! order, wears the navi's form overlay (its own form overlay object), and
//! may end early when the navi's form or action changes. See
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

/// Afterimage-private state: what its spawner (`sub_80E33FA`) and the
/// setters after it leave in its ExtraVars.
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    /// Ticks it lasts (ExtraVars+4, a halfword).
    pub lifetime: u16,
    /// ExtraVars+0xC (`sub_80E341E`).
    pub tether: Tether,
    /// The animation it holds.
    pub anim: u8,
    /// ExtraVars+0: its sprite's colour shader.
    pub color_shader: u16,
    /// ExtraVars+6 and +7: its shadow.
    pub shadow: ShadowFlag,
}

/// How an afterimage's shadow is drawn (presentation only):
/// `sprite_noShadow` when ExtraVars+6 is 0, else `sprite_hasShadow`, then
/// `sprite_removeShadow` when ExtraVars+7 is 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ShadowFlag {
    #[default]
    WithSprite,
    Ground,
    Hidden,
}

impl ShadowFlag {
    fn shadow(self) -> Shadow {
        match self {
            ShadowFlag::WithSprite => Shadow::WithSprite,
            ShadowFlag::Ground => Shadow::Ground,
            ShadowFlag::Hidden => Shadow::Hidden,
        }
    }
}

/// What an afterimage shows and for how long (`sub_80E33FA`'s
/// parameters).
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    /// A sprite of its own (Param1 and Param2), or None: a copy of the
    /// owner's (Param1 0xFF), with the owner's NameID and form overlay.
    pub sprite: Option<SpriteId>,
    pub anim: u8,
    /// Param4: the game's flip value.
    pub flip: u8,
    pub color_shader: u16,
    pub lifetime: u16,
    pub shadow: ShadowFlag,
    pub tether: Tether,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut b.objects.get_mut(r).vars {
        crate::kinds::Vars::Afterimage(v) => v,
        v => panic!("afterimage with {v:?}"),
    }
}

/// `sub_80EAFC2` (via `sub_80E33FA`): the Beast Out rush's afterimage of
/// `owner` at `pos` lasting `lifetime` ticks, holding `anim`.
pub fn spawn(b: &mut Battle, owner: ObjectRef, pos: Vec3, anim: u8, lifetime: u16) -> Option<ObjectRef> {
    let (alliance, flip) = {
        let o = b.objects.get(owner);
        (o.alliance, o.flip)
    };
    // sub_80E341E: tied to the Beast form, or to the attack.
    let tether = if b.stats[alliance as usize].form.is_beast() { Tether::BeastForm } else { Tether::Attack };
    // A copy of the owner facing its way (`object_getFlip`), less green,
    // with a ground shadow (the spawner's r7 is 0x01010014 - n).
    let spec = Spec {
        sprite: None,
        anim,
        flip: alliance ^ flip,
        color_shader: COLOR_SHADER,
        lifetime,
        shadow: ShadowFlag::Ground,
        tether,
    };
    spawn_with(b, owner, pos, &spec)
}

/// `sub_80E33FA`: an afterimage of `owner` at `pos`.
pub fn spawn_with(b: &mut Battle, owner: ObjectRef, pos: Vec3, spec: &Spec) -> Option<ObjectRef> {
    let alliance = b.objects.get(owner).alliance;
    let (p1, p2) = spec.sprite.map_or((0xFF, 0), |s| (s.category, s.index));
    let r = b.objects.spawn(Pool::Effect, INDEX, pos, [p1, p2, spec.anim, spec.flip])?;
    let o = b.objects.get_mut(r);
    o.related[0] = Some(owner);
    o.alliance = alliance;
    o.flags |= flags::RUN_WHILE_PAUSED;
    *vars(b, r) = Vars {
        lifetime: spec.lifetime,
        tether: spec.tether,
        anim: spec.anim,
        color_shader: spec.color_shader,
        shadow: spec.shadow,
    };
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

/// `sub_80E32D8`: copy the owner's NameID and sprite and put on its form's
/// overlay (Param1 0xFF), or load a sprite of its own; show `anim`.
fn init(b: &mut Battle, r: ObjectRef) {
    b.objects.get_mut(r).flags |= flags::VISIBLE;
    let params = b.objects.get(r).params;
    if params[0] == 0xFF {
        let owner = b.objects.get(r).related[0].expect("afterimage has an owner");
        let name_id = b.objects.get(owner).name_id;
        b.objects.get_mut(r).name_id = name_id;
        b.objects.sprite_mut(r).load(player_sprite(&b.content, name_id));
        b.objects.get_mut(r).flags &= !flags::NO_SPRITE_UPDATE;
        put_on_layer(b, r, name_id);
    } else {
        b.objects.sprite_mut(r).load(SpriteId { category: params[0], index: params[1] });
        b.objects.get_mut(r).flags &= !flags::NO_SPRITE_UPDATE;
    }
    let Vars { anim, lifetime, color_shader, shadow, .. } = vars(b, r).clone();
    let flip = params[3];
    let s = b.objects.sprite_mut(r);
    s.set_animation(anim, &b.content);
    s.update(&b.content);
    // Its shadow, the fourth parameter's flip and the spawner's colour
    // shader (the rush's 0x83E0: less green). (Its palette, ExtraVars+0x10,
    // is 0 from every spawner.)
    s.look.shadow = shadow.shadow();
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
/// NameID's navi or form wears (by its record's AI index: `off_8010EA4`
/// and `off_8010F08` after it), on the afterimage, drawn in front.
fn put_on_layer(b: &mut Battle, r: ObjectRef, name_id: u16) {
    use crate::kinds::body_overlay;
    let rec = b.content.navi_record(name_id);
    // A virus record's table is all nullsubs.
    if rec.actor_type == crate::actor::ActorType::Virus {
        return;
    }
    let body = |variant: u8, own_palette: bool, anim_offset: u8| body_overlay::Vars {
        variant,
        own_palette,
        anim_offset,
        ..Default::default()
    };
    let layer = match rec.ai_index {
        // nullsub_42 / nullsub_43.
        0 | 2..=5 | 7 | 8 | 10..=12 | 15 | 17 | 20..=23 | 35 | 37..=41 | 47 => return,
        // sub_8010F6A, sub_8010F86, sub_8010F96, sub_8011004, sub_8010FAC,
        // sub_8010FC2: a body overlay (Param3, stepping while paused, 0).
        1 => body_overlay::spawn_with(b, r, body(2, false, 0)),
        9 => body_overlay::spawn_with(b, r, body(0x0B, false, 0)),
        13 => body_overlay::spawn_with(b, r, body(3, false, 0x0A)),
        16 => body_overlay::spawn_with(b, r, body(0x0F, true, 9)),
        18 => body_overlay::spawn_with(b, r, body(0, false, 0x0D)),
        19 => body_overlay::spawn_with(b, r, body(0x10, false, 0x14)),
        // sub_80112E0 .. sub_801133A (via sub_8011344): a Cross's helmet
        // and arm, with its own palette; AI index 24 + the form.
        25..=34 => {
            let variant = crate::kinds::player::form::cross_overlay(Form(rec.ai_index - 24));
            body_overlay::spawn_with(b, r, body(variant, true, 0))
        }
        // sub_8011366 and sub_8011352 .. sub_8011362: the Falzar beast head
        // (for 24, 36 and 48 its palette follows the navi's mood: drawn
        // only).
        24 | 36 | 42..=46 | 48 => {
            let layer = form_overlay::spawn(b, r, form_overlay::BEAST_HEAD, true);
            b.objects.get_mut(r).related[1] = layer;
            return;
        }
        // sub_8010F7A: SpoutMan's overlay, actor object #0x55, which the
        // engine doesn't have yet (the navi init hook for AI index 6).
        6 => panic!("afterimage overlays for AI index 6 (sub_8010F7A, actor object #0x55) are not implemented yet"),
        // sub_8010FD8: two overlays, the second stored over the
        // afterimage's colour shader; AI index 14 is a navi AI's, which no
        // player NameID has.
        14 => panic!("afterimages of NameID {name_id:#x} (a navi AI's record, sub_8010FD8) are not implemented yet"),
        n => panic!("NameID {name_id:#x}'s AI index {n} reads past sub_8010DF6's overlay table"),
    };
    b.objects.get_mut(r).related[1] = layer;
    if let Some(l) = layer {
        body_overlay::force_front(b, l);
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
    // sub_8011044(record, 1): the overlay comes off (for the Falzar beast
    // head, sub_801140E). One with a sprite of its own keeps NameID 0, a
    // virus's record, whose hook is a nullsub.
    let name_id = b.objects.get(r).name_id;
    if name_id != 0 {
        crate::kinds::player::form::navi_death_hook(b, r, name_id);
    }
    b.objects.free(r);
}
