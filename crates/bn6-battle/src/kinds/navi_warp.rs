//! A navi warping out or in (actor object #0x2D, `sub_80C0E04`): while a
//! navi chip's navi acts, its user vanishes behind this 4-tick image of
//! itself (with its form's overlay), and comes back behind another. Purely
//! visual, but it holds an actor slot and a place in the update order, and
//! it hides and shows the user. See docs/engine/chips.md §3.6.7.

use crate::actor::ActorType;
use crate::battle::Battle;
use crate::kinds::common::{self, Progress};
use crate::kinds::player::form;
use crate::object::sprite::Shadow;
use crate::object::{ObjectRef, Vec3, flags, state};
use crate::setup::Navi;
use crate::dimming::{hide_user, show_user};

pub const INDEX: u8 = 0x2D;

/// Which way the navi warps (Param4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Warp {
    /// Back in: the navi shows again at the end.
    #[default]
    In = 0,
    /// Out: the navi vanishes at the start.
    Out = 1,
}

/// The warp's own state.
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    pub warp: Warp,
    /// ExtraVars[0]: the warping object's actor type (only players are
    /// supported).
    pub owner_type: ActorType,
}

fn vars(b: &Battle, r: ObjectRef) -> &Vars {
    match &b.objects.get(r).vars {
        crate::kinds::Vars::NaviWarp(v) => v,
        v => panic!("navi warp with {v:?}"),
    }
}

/// `sub_80C0F52`: `navi` warps `warp`.
pub fn spawn(b: &mut Battle, navi: ObjectRef, warp: Warp) -> Option<ObjectRef> {
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::NaviWarp, Vec3::default(), [0, 0, 0, warp as u8])?;
    let (alliance, flip) = {
        let o = b.objects.get(navi);
        (o.alliance, o.flip)
    };
    let o = b.objects.get_mut(r);
    o.related[0] = Some(navi);
    o.alliance = alliance;
    o.flip = flip;
    o.vars = crate::kinds::Vars::NaviWarp(Vars { warp, owner_type: ActorType::Virus });
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => tick(b, r),
        _ => b.objects.free(r),
    }
}

fn owner(b: &Battle, r: ObjectRef) -> ObjectRef {
    b.objects.get(r).related[0].expect("navi warp has a navi")
}

/// `sub_80C0E24`: a player's image: its sprite and form overlay (which
/// steps even while paused), at its position.
fn init(b: &mut Battle, r: ObjectRef) {
    let navi = owner(b, r);
    let owner_type = b.content.navi_record(b.objects.get(navi).name_id).actor_type;
    if let crate::kinds::Vars::NaviWarp(v) = &mut b.objects.get_mut(r).vars {
        v.owner_type = owner_type;
    }
    if owner_type != ActorType::Player {
        // No image for anything but a player: it just counts its ticks.
        common::set_progress(b, r, Progress::UPDATE);
        return tick(b, r);
    }
    let side = b.objects.get(navi).alliance as usize;
    let (navi_id, form) = (b.stats[side].navi, b.stats[side].form);
    let sprite = if navi_id == Navi::MEGAMAN { b.content.form(form).sprite } else { b.content.navi(navi_id).sprite };
    let (pos, palette) = (b.objects.get(navi).pos, b.objects.sprite(navi).look.palette);
    let flip = {
        let o = b.objects.get(r);
        o.alliance ^ o.flip
    };
    let warp = vars(b, r).warp;
    let s = b.objects.sprite_mut(r);
    s.load(sprite);
    // sprite_hasShadow; the navi's palette (sub_801002C); its facing.
    s.look.shadow = Shadow::Ground;
    s.look.palette = palette;
    s.look.set_flip(flip);
    let o = b.objects.get_mut(r);
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.flags |= flags::VISIBLE;
    o.pos = pos;
    // sub_8011420(navi, form, 1): the form's overlay (whose Param3 is
    // lost on the way), or a link navi's own; then, in a form, the
    // overlay's Param3 = 1 and flags 0x14.
    form::put_on_navi_overlay(b, r, navi_id, form, 1);
    if form != crate::setup::Form::NONE
        && let Some(overlay) = b.objects.get(r).related[1]
    {
        form::keep_overlay_stepping(b, overlay);
    }
    let o = b.objects.get_mut(r);
    o.anim = 3 + warp as u8;
    o.anim_loaded = 0xFF;
    common::set_progress(b, r, Progress::UPDATE);
    tick(b, r);
}

/// `sub_80C0EE2`: 4 ticks; a warp out hides the navi at the start, a warp
/// in shows it at the end. The image then takes its overlay down and
/// goes.
fn tick(b: &mut Battle, r: ObjectRef) {
    let warp = vars(b, r).warp;
    if b.objects.get(r).phase_init == 0 {
        if warp == Warp::Out {
            let navi = owner(b, r);
            hide_user(b, navi);
        }
        let o = b.objects.get_mut(r);
        o.timer = 4;
        o.phase_init = 4;
    }
    let o = b.objects.get_mut(r);
    let left = o.timer as i32 - 1;
    o.timer = left as u16;
    if left <= 0 {
        o.flags &= !flags::VISIBLE;
        let navi = owner(b, r);
        let name_id = b.objects.get(navi).name_id;
        form::navi_death_hook(b, r, name_id);
        if warp == Warp::In {
            show_user(b, navi);
        }
        common::set_progress(b, r, Progress::DESTROY);
    }
    common::update_sprite_while_dimmed(b, r);
}
