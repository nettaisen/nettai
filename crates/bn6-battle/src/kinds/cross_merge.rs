//! A Cross navi's image merging with MegaMan (actor object #0x1B,
//! `sub_80BC650`), spawned by a Cross change: it appears above and in
//! front of MegaMan, then swings from one side of him to the other, each
//! swing narrower, and when it reaches him it bursts into an effect and is
//! gone. Purely visual, but it holds an actor slot and a place in the
//! update order, and runs while the battle is paused.
//! See docs/engine/objects-and-player.md §12.10.

use crate::battle::Battle;
use crate::kinds::common;
use crate::kinds::effect;
use crate::object::sprite::Shadow;
use crate::object::{ObjectRef, PanelPos, Vec3, flags, state};
use bn6_content_api::NaviHandle;

/// The image's own state.
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    /// Which navi it shows (Param1).
    pub navi: Option<NaviHandle>,
    /// +0x62: swings left; each is `swing_step` narrower than the last.
    pub swings_left: u16,
    /// ExtraVars+4: how much narrower each swing gets.
    pub swing_step: i32,
    /// +0x68: the side of the next swing (+1 ahead of MegaMan, -1
    /// behind).
    pub swing_side: i32,
    /// ExtraVars+0xC: how far above the panel it floats (added to both Y
    /// and Z, so it is drawn at the panel's height, nearer the viewer).
    pub lift: i32,
    /// ExtraVars+0x10: the navi's extra height (`NaviData::merge_height`).
    pub extra_height: i32,
    /// ExtraVars+0x14: the swing sound has played.
    pub sounded: bool,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut b.objects.get_mut(r).vars {
        crate::kinds::Vars::CrossMerge(v) => v,
        v => panic!("cross merge with {v:?}"),
    }
}

/// `sub_80BC844`: `navi`'s image over MegaMan (`owner`), `swings` swings
/// from his panel.
pub fn spawn(b: &mut Battle, owner: ObjectRef, navi: NaviHandle, swings: u16) -> Option<ObjectRef> {
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::CrossMerge, Vec3::default(), [0; 4])?;
    let (panel, alliance, flip) = {
        let o = b.objects.get(owner);
        (o.panel, o.alliance, o.flip)
    };
    let o = b.objects.get_mut(r);
    o.panel = PanelPos { x: panel.x, y: panel.y };
    o.alliance = alliance;
    o.flip = flip;
    o.related[0] = Some(owner);
    o.timer = 6;
    o.flags |= flags::RUN_WHILE_PAUSED | flags::RUN_WHILE_DIMMED;
    let v = vars(b, r);
    v.navi = Some(navi);
    v.swings_left = swings;
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => tick(b, r),
        _ => b.objects.free(r),
    }
}

/// `sub_80BC670`: the navi's sprite, ahead of MegaMan by a full swing.
fn init(b: &mut Battle, r: ObjectRef) {
    let navi = vars(b, r).navi.expect("a cross merge shows a navi");
    let identity = b.content.navi(navi).identity;
    b.objects.get_mut(r).identity = identity;
    // sub_800FC9E(navi, no form)
    let sprite = b.content.navi_sprite(navi, b.content.base_form());
    let flip = {
        let o = b.objects.get(r);
        o.alliance ^ o.flip
    };
    let s = b.objects.sprite_mut(r);
    s.load(sprite);
    s.set_animation(0, &b.content);
    // sprite_removeShadow, palette 4, the object's facing, and white
    // (sprite_forceWhitePalette at the end of the init).
    s.look.shadow = Shadow::Hidden;
    s.look.palette = 4;
    s.look.set_flip(flip);
    s.look.white = true;
    let o = b.objects.get_mut(r);
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.flags |= flags::VISIBLE;
    o.anim = 0;
    o.anim_loaded = 0;
    let (panel_y, facing) = (o.panel.y, common::facing(o.alliance, o.flip));
    let swings = vars(b, r).swings_left;
    // The game's svc Div, with a zero divisor never passed.
    let step = 0x28_0000 / swings as i32;
    let lift = (4 - panel_y as i32) * 0x18_0000;
    let extra = (b.content.navi(navi).merge_height as i32) << 16;
    let v = vars(b, r);
    v.swing_side = -1;
    v.swing_step = step;
    v.lift = lift;
    v.extra_height = extra;
    v.sounded = false;
    common::set_coordinates_from_panels(b, r);
    let o = b.objects.get_mut(r);
    o.pos.x = o.pos.x.wrapping_add(step.wrapping_mul(swings as i32).wrapping_mul(facing));
    o.pos.y = o.pos.y.wrapping_add(lift);
    o.pos.z = lift.wrapping_add(extra);
    // sub_8010DD0: the navi's own init hook (some wear an overlay).
    crate::kinds::player::form::navi_init_hook(b, r, identity);
    // SpoutMan's image keeps its idle overlay at its own height (the
    // hook's result, ExtraVars[0] = 1; the original tests his navi
    // number: the one navi that wears an idle overlay).
    if matches!(b.content.identity(identity).parts, Some(crate::content::Parts::Idle { .. }))
        && let Some(o) = b.objects.get(r).related[1]
    {
        crate::kinds::player::form::pin_overlay(b, o);
    }
    let o = b.objects.get_mut(r);
    o.state = state::UPDATE;
    o.action = 0;
    o.phase = 0;
    o.phase_init = 0;
}

/// `sub_80BC78C`: hold, then swing every other tick until it reaches
/// MegaMan.
fn tick(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).action == 0 {
        // Held in place, white, until the timer runs out.
        b.objects.sprite_mut(r).look.white = true;
        let o = b.objects.get_mut(r);
        o.timer = o.timer.wrapping_sub(1);
        if o.timer != 0 {
            return common::update_sprite(b, r);
        }
        // sprite_clearFinalPalette
        b.objects.sprite_mut(r).look.white = false;
        let o = b.objects.get_mut(r);
        o.timer = 10;
        o.action = 1;
    }
    let o = b.objects.get_mut(r);
    o.timer = o.timer.wrapping_sub(1);
    if o.timer != 0 {
        return common::update_sprite(b, r);
    }
    if !vars(b, r).sounded {
        vars(b, r).sounded = true;
        b.sound(crate::content::SoundRole::CrossMerge);
    }
    let v = vars(b, r);
    v.swings_left = v.swings_left.wrapping_sub(1);
    if v.swings_left as i16 <= 0 {
        return burst(b, r);
    }
    let Vars { swings_left, swing_step, swing_side, lift, extra_height, .. } = *v;
    b.objects.get_mut(r).timer = 2;
    common::set_coordinates_from_panels(b, r);
    let o = b.objects.get_mut(r);
    let facing = common::facing(o.alliance, o.flip);
    let dx = swing_side.wrapping_mul(swing_step).wrapping_mul(swings_left as i32).wrapping_mul(facing);
    o.pos.x = o.pos.x.wrapping_add(dx);
    o.pos.y = o.pos.y.wrapping_add(lift);
    o.pos.z = lift.wrapping_add(extra_height);
    vars(b, r).swing_side = -swing_side;
    common::update_sprite(b, r);
}

/// It reached MegaMan: its NameID's death hook (`sub_8011020`: its
/// overlay, if its init hook put one on, comes off), an effect on the
/// panel, and it is freed at once.
fn burst(b: &mut Battle, r: ObjectRef) {
    let identity = b.objects.get(r).identity;
    crate::kinds::player::form::navi_death_hook(b, r, identity);
    let o = b.objects.get(r);
    let (x, y) = crate::kinds::player::panel_coordinates(o.panel.x, o.panel.y);
    let look = b.content.defs.roles.effect(crate::content::EffectRole::Deletion);
    if let Some(e) = effect::spawn(b, Vec3 { x, y, z: 0x10_0000 }, look, 0, 0, 0) {
        b.objects.get_mut(e).flags |= flags::RUN_WHILE_PAUSED;
    }
    b.objects.free(r);
}
