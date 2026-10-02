//! A player's buster charge glow (effect #8). Purely visual, but it holds a
//! pool slot and a place in the update order for the whole round, and its
//! lifetime follows the player's. See docs/engine/objects-and-player.md §A.5.

use crate::battle::Battle;
use crate::content::{SoundRole, SpriteRole};
use crate::object::{ObjectRef, Vec3, flags, state};

#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    /// Other effects can hide the glow.
    pub enabled: bool,
    /// The glow sprite loaded (none yet at first).
    pub sprite: Option<SpriteRole>,
    /// The owner's charge level this tick and last tick.
    pub level: u8,
    pub previous_level: u8,
    /// Its link slot is a BIOS address (FirstBarrier's register clobber):
    /// the navi never learns of it, and its link test (the slot nonzero)
    /// always passes, so it never ends itself.
    pub bios_link: bool,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut b.objects.get_mut(r).vars {
        crate::kinds::Vars::ChargeGlow(v) => v,
        v => panic!("charge glow with {v:?}"),
    }
}

/// `sub_80E0F22` / `sub_80E0F28`: hide it with its navi, or let it show
/// again (EV+4).
pub fn set_enabled(b: &mut Battle, r: ObjectRef, on: bool) {
    vars(b, r).enabled = on;
}

/// Spawn the glow for `owner` (`sub_80E0F02`). Its initial X/Y/Z are
/// leftover registers in the game (§A.5); they are never read.
pub fn spawn(b: &mut Battle, owner: ObjectRef) -> Option<ObjectRef> {
    let actor = b.objects.get(owner).actor?;
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::ChargeGlow, Vec3::default(), [0; 4])?;
    let o = b.objects.get_mut(r);
    o.related[0] = Some(owner);
    o.flags |= flags::RUN_WHILE_PAUSED;
    b.actors.get_mut(actor).charge_glow = Some(r);
    Some(r)
}

/// `sub_80E0F02` with its link slot in the BIOS (FirstBarrier): the glow
/// for `owner`, which the navi's AIData doesn't record.
pub fn spawn_unlinked(b: &mut Battle, owner: ObjectRef) -> Option<ObjectRef> {
    b.objects.get(owner).actor?;
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::ChargeGlow, Vec3::default(), [0; 4])?;
    let o = b.objects.get_mut(r);
    o.related[0] = Some(owner);
    o.flags |= flags::RUN_WHILE_PAUSED;
    vars(b, r).bios_link = true;
    Some(r)
}

/// `sub_80E0DF0`.
pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => {
            // sub_80E0E10
            vars(b, r).enabled = true;
            b.objects.get_mut(r).state = state::UPDATE;
            tick(b, r);
        }
        state::UPDATE => tick(b, r),
        _ => b.objects.free(r),
    }
}

/// `sub_80E0E20`: follow the owner's charge: level as animation, shown
/// only while charging (and on the owner's console only unless the B
/// button charges), at the owner's attach point 0.
fn tick(b: &mut Battle, r: ObjectRef) {
    let owner = b.objects.get(r).related[0].expect("charge glow has an owner");
    let actor = b.objects.get(owner).actor.expect("charge glow owner has actor data");
    if b.paused {
        if b.actors.get(actor).charge_source == 0 {
            b.objects.get_mut(r).set_visible(false);
        }
        return;
    }
    if b.actors.get(actor).charge_glow.is_none() && !vars(b, r).bios_link {
        let o = b.objects.get_mut(r);
        o.state = state::DESTROY;
        o.action = 0;
        o.phase = 0;
        o.phase_init = 0;
        return;
    }
    let alliance = b.objects.get(owner).alliance;
    let panel = b.objects.get(owner).panel;
    let visible = vars(b, r).enabled && crate::field::is_valid(panel.x, panel.y);
    let source = b.actors.get(actor).charge_source;
    // Seen by a viewer who sees the owner's side (`sub_800EB6C`), and on
    // its owner's console only unless the B button charges (source 2).
    let shown_to = |b: &Battle, viewer: u8| match source {
        0 => false,
        2 => b.sees(viewer, alliance),
        _ => viewer == alliance & 1 && b.sees(viewer, alliance),
    };
    let shown = [0u8, 1].map(|viewer| visible && shown_to(b, viewer));
    b.set_visible_by_viewer(r, shown);
    select_sprite(b, r, source);
    let level = b.actors.get(actor).charge_level;
    let v = vars(b, r);
    v.previous_level = v.level;
    v.level = level;
    b.objects.get_mut(r).anim = level;
    if level == 0 {
        b.objects.get_mut(r).set_visible(false);
    }
    charge_sound(b, r, alliance, source);
    let (dx, dz) = crate::kinds::player::attach_point(b, owner, 0);
    let p = b.objects.get(owner).pos;
    b.objects.get_mut(r).pos = Vec3 { x: p.x.wrapping_add(dx << 16), y: p.y, z: p.z.wrapping_add(dz << 16) };
    crate::kinds::player::update_sprite(b, r);
}

/// `sub_80E0F5E`: the charge sounds, as the charge starts and as it
/// completes; only the charging navi's player hears a charge unless it
/// comes from source 2.
fn charge_sound(b: &mut Battle, r: ObjectRef, alliance: u8, source: u8) {
    let v = vars(b, r);
    let id = match (v.level, v.previous_level) {
        (1, 0) => SoundRole::BusterCharge,
        (2, 1) => SoundRole::BusterCharged,
        _ => return,
    };
    if source == 2 {
        b.sound(id);
    } else {
        b.sound_for(alliance, id);
    }
}

/// `sub_80E0F2E`: the A charge glows differently from the B charge.
fn select_sprite(b: &mut Battle, r: ObjectRef, source: u8) {
    let wanted = if source == 1 { SpriteRole::ChargeGlowA } else { SpriteRole::ChargeGlow };
    if vars(b, r).sprite == Some(wanted) {
        return;
    }
    vars(b, r).sprite = Some(wanted);
    let sprite = b.roles_for(r).sprite(wanted);
    b.objects.sprite_mut(r).load(sprite);
    b.objects.sprite_mut(r).look.shadow = crate::object::sprite::Shadow::WithSprite;
    let o = b.objects.get_mut(r);
    o.anim_loaded = 0xFF;
    o.flags &= !flags::NO_SPRITE_UPDATE;
}
