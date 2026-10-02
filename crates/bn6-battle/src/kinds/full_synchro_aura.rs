//! The Full Synchro aura (actor object #0x5E, `sub_80C4B18`): the glow
//! around a navi in Full Synchro. It follows its navi, keeps the navi's
//! actor data pointing at it, and goes once the navi's emotion is no
//! longer Full Synchro. Purely visual, but it holds an actor slot and a
//! place in the update order.

use crate::battle::Battle;
use crate::collision::f1;
use crate::kinds::common::{self, Progress};
use crate::kinds::player::{Emotion, emotion};
use crate::object::sprite::Shadow;
use crate::object::{ObjectRef, Vec3, flags, state};


/// Aura-private state.
#[derive(Clone, Copy, Debug, Default, Hash)]
pub struct Vars {
    /// +0x0C: shown (dimming chips hide it while they run: `sub_80C4C46`
    /// and `sub_80C4C4C`).
    pub shown: bool,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut b.objects.get_mut(r).vars {
        crate::kinds::Vars::FullSynchroAura(v) => v,
        v => panic!("Full Synchro aura with {v:?}"),
    }
}

/// `sub_80C4C12`: an aura on `navi`, linked from its actor data
/// (AIData+0x5C). It runs while paused.
pub fn spawn(b: &mut Battle, navi: ObjectRef) -> Option<ObjectRef> {
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::FullSynchroAura, Vec3::default(), [0; 4])?;
    let (alliance, flip) = {
        let o = b.objects.get(navi);
        (o.alliance, o.flip)
    };
    let o = b.objects.get_mut(r);
    o.alliance = alliance;
    o.flip = flip;
    o.related[0] = Some(navi);
    o.flags |= flags::RUN_WHILE_PAUSED;
    o.vars = crate::kinds::Vars::FullSynchroAura(Vars { shown: true });
    set_link(b, navi, Some(r));
    Some(r)
}

/// The navi's actor-data link to its aura.
fn set_link(b: &mut Battle, navi: ObjectRef, aura: Option<ObjectRef>) {
    if let Some(a) = b.objects.get(navi).actor {
        b.actors.get_mut(a).full_synchro_aura = aura;
    }
}

/// `sub_80C4C3A`: end the aura (its state word becomes 8) and unlink it.
pub fn end(b: &mut Battle, r: ObjectRef) {
    common::set_progress(b, r, Progress::DESTROY);
    let navi = owner(b, r);
    set_link(b, navi, None);
}

/// `sub_80C4C46`: hide the aura.
pub fn hide(b: &mut Battle, r: ObjectRef) {
    vars(b, r).shown = false;
}

/// `sub_80C4C4C`: show it again.
pub fn show(b: &mut Battle, r: ObjectRef) {
    vars(b, r).shown = true;
}

fn owner(b: &Battle, r: ObjectRef) -> ObjectRef {
    b.objects.get(r).related[0].expect("Full Synchro aura has a navi")
}

/// `sub_80C4B18`: the state's routine, then the sprite unless dimmed or
/// paused.
pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => tick(b, r),
        _ => return b.objects.free(r),
    }
    if b.objects.is_allocated(r) && !b.is_dimmed() && !b.paused {
        common::update_sprite(b, r);
    }
}

/// `sub_80C4C52`: the aura's animation by its navi's identity (the
/// original's by the actor record's AI index: the index itself below
/// 0x23, a navi's or a Cross's own; 0xC for the Gregar beasts, 0xD for the
/// Falzar ones): the identity's `aura_anim`. (Every player's identity has
/// one; an object with no identity reads as the record of zeros.)
fn animation(b: &Battle, r: ObjectRef) -> u8 {
    let identity = b.objects.get(owner(b, r)).identity;
    b.content.identity(identity).aura_anim.unwrap_or(0)
}

/// `sub_80C4B48`: the sprite, visible, on its animation.
fn init(b: &mut Battle, r: ObjectRef) {
    let anim = animation(b, r);
    let sprite = b.content.defs.roles.sprite(crate::content::SpriteRole::FullSynchroAura);
    let s = b.objects.sprite_mut(r);
    s.load(sprite);
    s.look.shadow = Shadow::WithSprite;
    s.set_animation(anim, &b.content);
    s.update(&b.content);
    let o = b.objects.get_mut(r);
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.flags |= flags::VISIBLE;
    o.anim = anim;
    o.anim_loaded = anim;
    common::set_progress(b, r, Progress::UPDATE);
    tick(b, r);
}

/// `sub_80C4B84`: follow the navi (hidden when hidden, when the navi is
/// submerged, or when the navi is the other side's and the local navi is
/// blind); go when unlinked or no longer in Full Synchro.
fn tick(b: &mut Battle, r: ObjectRef) {
    let anim = animation(b, r);
    let navi = owner(b, r);
    b.objects.get_mut(r).anim = anim;
    let alliance = b.objects.get(r).alliance;
    // sub_800EB6C: hidden from a blind local navi.
    let viewer_blind = !b.viewer_sees(alliance);
    let shown = !viewer_blind && vars(b, r).shown;
    let submerged = b.objects.get(navi).collision.is_some_and(|c| b.collision.get(c).f1 & f1::SUBMERGED != 0);
    let (pos, flip) = {
        let n = b.objects.get(navi);
        (n.pos, n.flip)
    };
    let o = b.objects.get_mut(r);
    o.flags &= !flags::VISIBLE;
    if shown && !submerged {
        o.flags |= flags::VISIBLE;
    }
    o.pos = pos;
    o.flip = flip;
    let facing = o.alliance ^ flip;
    b.objects.sprite_mut(r).look.set_flip(facing);
    let linked = b.objects.get(navi).actor.is_some_and(|a| b.actors.get(a).full_synchro_aura.is_some());
    if linked {
        if emotion(b, alliance) == Emotion::FullSynchro {
            return;
        }
        set_link(b, navi, None);
    }
    b.objects.free(r);
}
