//! Changing navis mid-battle (a "Cross change": the battle flag 0x40
//! mode's link navi switch). A player's transformation record names the
//! navi to change to (+4); at the turn's start the transformation
//! sequencer asks the navi (`sub_802DCDE`), and the pause handler runs the
//! change as action 0x1C (`sub_802D714`): the navi lands, becomes the other
//! navi (its stats kept or fresh, `Battle::cross_stats`), and after 21
//! ticks goes on as it. A navi changed so falls back to the one it was
//! instead of being deleted (`sub_802DD2A`): the Cross knockout
//! (`sub_802D926`) brings the kept navi back. See docs/engine/battle-flow.md
//! §3.4.
//!
//! Only the battle flag 0x40 mode sends a Cross change; no recording has
//! one, so all of it is unverified.

use super::ActionVars;
use crate::actor::{request, status};
use crate::battle::Battle;
use crate::collision::f1;
use crate::kinds::common;
use crate::kinds::player::{
    ai, ai_mut, clear_flag1, clear_flag2, clear_invulnerable, clear_statuses, coll_mut, exit_attack_state, form,
    is_megaman, load_sprite, post_init_hook, reset_status, reset_status_tail, set_coordinates_from_panel, stats, status as navi_status,
    update_element,
};
use crate::object::ObjectRef;
use crate::content::Content;
use nettai_content_api::NaviHandle;
use crate::setup::{NaviStats, NaviWeapons};

/// The action's own state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Vars {
    /// AIAttackVars+0x10: ticks left in the step.
    pub timer: u16,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    let a = &mut ai_mut(b, r).attack;
    if !matches!(a.action, ActionVars::CrossChange(_)) {
        a.action = ActionVars::CrossChange(Vars::default());
    }
    match &mut a.action {
        ActionVars::CrossChange(v) => v,
        _ => unreachable!(),
    }
}

fn set_step(b: &mut Battle, r: ObjectRef, step: u8) {
    let a = &mut ai_mut(b, r).attack;
    a.step = step;
    a.step_init = 0;
}

/// `sub_802DCDE`: the transformation sequencer asks `navi` to change.
pub(crate) fn request_change(b: &mut Battle, navi: ObjectRef) {
    ai_mut(b, navi).requests |= request::CROSS_CHANGE;
}

/// `sub_802D714`: one paused tick of the change, then the sprite steps
/// (`sub_801BCD0`).
pub(in crate::kinds::player) fn change(b: &mut Battle, r: ObjectRef) {
    match ai(b, r).attack.step {
        // sub_802D738
        0 => land(b, r, false),
        // sub_802D7A0
        4 => become_other_navi(b, r),
        // sub_802D8F0
        8 => {
            if settle(b, r) {
                // (The HUD shows the navi again.)
                let a = ai_mut(b, r);
                a.status |= status::CROSSED;
                a.status &= !(status::TRAP_ARMED | status::CHANGING_CROSS);
                a.requests &= !(request::BODY_GUARD_TRIGGERED | request::ANTI_SWORD_TRIGGERED | request::ANTI_DAMAGE_TRIGGERED);
                exit_attack_state(b, r);
            }
        }
        s => panic!("Cross change step {s:#x} reads past its table (off_802D72C)"),
    }
    common::step_sprite(b, r);
}

/// `sub_802D926` (the pause handler sets the variant to 0 first): one
/// paused tick of the Cross knockout; the sprite steps with variant 0
/// (`sub_801BCD0`).
pub(in crate::kinds::player) fn knock_out(b: &mut Battle, r: ObjectRef) {
    match ai(b, r).attack.step {
        // sub_802D950
        0 => land(b, r, true),
        // sub_802D9B0
        4 => take_back(b, r),
        // sub_802DA78
        8 => {
            if settle(b, r) {
                let a = ai_mut(b, r);
                a.status &= !(status::TRAP_ARMED | status::CROSS_KNOCKOUT | status::CROSSED);
                a.requests &= !(request::CROSS_CHANGE
                    | request::BODY_GUARD_TRIGGERED
                    | request::ANTI_SWORD_TRIGGERED
                    | request::ANTI_DAMAGE_TRIGGERED);
                exit_attack_state(b, r);
            }
        }
        s => panic!("Cross knockout step {s:#x} reads past its table (off_802D944)"),
    }
    // (The original steps the sprite when the attack's variant byte is 0,
    // which its only caller, the pause handler, stores first.)
    common::step_sprite(b, r);
}

/// `sub_802D738` / `sub_802D950`: onto the destination panel on the
/// ground, facing the default way, out of any slide, paralysis, flinch,
/// move or guard (the requests' clear takes the same bits: the game passes
/// the flags' mask again), the animation 4, links cut; the change keeps its
/// collision region on, the knockout ends the Full Synchro aura's link.
/// Four ticks.
fn land(b: &mut Battle, r: ObjectRef, knockout: bool) {
    if ai(b, r).attack.step_init == 0 {
        let o = b.objects.get_mut(r);
        o.panel = o.future_panel;
        let p = o.panel;
        b.unreserve_panel(r, p.x, p.y);
        set_coordinates_from_panel(b, r);
        b.objects.get_mut(r).pos.z = 0;
        b.update_collision_panels(r);
        super::transform::face_default(b, r);
        const LANDING: u32 = f1::SLIDING | f1::PARALYZED | f1::FLINCHING | f1::MOVING | f1::GUARD;
        clear_flag1(b, r, LANDING);
        clear_flag2(b, r, LANDING);
        common::set_animation(b, r, 4);
        b.objects.get_mut(r).related[0] = None;
        ai_mut(b, r).overlay = None;
        if knockout {
            ai_mut(b, r).full_synchro_aura = None;
        } else {
            coll_mut(b, r).region = b.anchor_region();
        }
        vars(b, r).timer = 4;
        ai_mut(b, r).attack.step_init = 4;
    }
    let v = vars(b, r);
    let t = v.timer as i32 - 1;
    v.timer = t as u16;
    if t <= 0 {
        set_step(b, r, 4);
    }
}

/// `sub_802D8F0` / `sub_802DA78`: 21 ticks; true when they are over.
fn settle(b: &mut Battle, r: ObjectRef) -> bool {
    if ai(b, r).attack.step_init == 0 {
        vars(b, r).timer = 0x14;
        ai_mut(b, r).attack.step_init = 4;
    }
    let v = vars(b, r);
    let t = v.timer as i32 - 1;
    v.timer = t as u16;
    t < 0
}

/// The navi `r` becomes the one its stats name: its actor record, sprite
/// (animation 3) and overlays; the statuses and anger end.
fn take_identity(b: &mut Battle, r: ObjectRef) {
    // (The original stores the navi's number as the AI index, and
    // 0x1A0 plus it as the NameID: the navi's identity and its record's
    // index.)
    let identity = b.content.navi(stats(b, r).navi).identity;
    let ai_index = b.content.navi_record(identity).ai_index;
    let a = ai_mut(b, r);
    a.ai_index = ai_index;
    a.identity = identity;
    b.objects.get_mut(r).identity = identity;
    load_sprite(b, r);
    common::set_animation(b, r, 3);
}

/// `sub_802D7A0`: the change itself (one tick).
fn become_other_navi(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance as usize & 1;
    let old_form = stats(b, r).form;
    form::take_off_overlay(b, r, old_form);
    let identity = b.objects.get(r).identity;
    form::navi_death_hook(b, r, identity);
    navi_status::end_anger(b, r);
    // The navi it leaves is kept when it is the kept one (with its HP).
    if b.cross_stats[side].navi == b.stats[side].navi {
        b.stats[side].hp = b.objects.get(r).hp;
        b.cross_stats[side] = b.stats[side];
    }
    // sub_802DCCC: the record's navi.
    let Some(target) = b.turn_transforms[side].cross_change else {
        panic!("a Cross change without a navi reads 0xFF as one (sub_802DCCC)");
    };
    let kept = b.cross_stats[side].navi == target;
    b.stats[side] = if kept { b.cross_stats[side] } else { fresh_stats(target, &b.content) };
    super::super::refresh_navicust_state(b, r);
    take_identity(b, r);
    let s = *stats(b, r);
    let megaman = is_megaman(b, r);
    if megaman {
        form::put_on_overlay(b, r, s.form);
    } else {
        form::navi_init_hook(b, r, b.objects.get(r).identity);
    }
    post_init_hook(b, r);
    clear_statuses(b, r);
    navi_status::end_anger(b, r);
    let (hp, max_hp) = if megaman { (s.hp, s.max_hp) } else { changed_hp(b, s.navi, side as u8) };
    let o = b.objects.get_mut(r);
    o.hp = hp;
    o.max_hp = max_hp;
    finish_change(b, r);
    set_step(b, r, 8);
}

/// `sub_802D9B0`: the knockout's change back to the kept navi (one tick).
fn take_back(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance as usize & 1;
    let identity = b.objects.get(r).identity;
    form::navi_death_hook(b, r, identity);
    b.stats[side] = b.cross_stats[side];
    take_identity(b, r);
    form::navi_init_hook(b, r, b.objects.get(r).identity);
    let s = *stats(b, r);
    form::put_on_overlay(b, r, s.form);
    clear_statuses(b, r);
    reset_status(b, r);
    // sub_80143B4
    clear_flag1(b, r, f1::ANGER);
    clear_flag2(b, r, 0x200);
    let a = ai_mut(b, r);
    a.anger = 0;
    a.stun_ticks = 0;
    let o = b.objects.get_mut(r);
    o.hp = s.hp;
    o.max_hp = s.max_hp;
    let content = b.content.clone();
    b.hands[side].drop_link_navi_chips(&content);
    finish_change(b, r);
    set_step(b, r, 8);
}

/// What both changes end with: the hit in progress dropped
/// (`sub_800EA0E`), poison affecting it again, its element, the hand's
/// charge bonuses cleared (`sub_8014216`, and for MegaMan his status reset
/// without its NaviCust part or weapon bytes, `sub_80144CA`), no
/// invulnerability.
fn finish_change(b: &mut Battle, r: ObjectRef) {
    clear_flag2(b, r, 0x3_01FE);
    let acc = &mut coll_mut(b, r).acc;
    acc.final_damage = 0;
    acc.element_damage = [0; 6];
    clear_flag1(b, r, f1::UNTOUCHABLE);
    update_element(b, r);
    // sub_8014216
    ai_mut(b, r).status &= !0x20;
    let side = b.objects.get(r).alliance as usize & 1;
    b.hands[side].charge_bonus = [0; 6];
    // `off_801426C`, by the navi: MegaMan's (the navi that changes form)
    // is his status reset; the link navis' are nothing.
    if is_megaman(b, r) {
        reset_status_tail(b, r, false);
    }
    clear_invulnerable(b, r);
}

/// `init_8013B64`: `navi`'s stats, fresh: the defaults
/// (`initNaviStats_WithDefaultStatsMaybe_8013438`) with what the navi comes
/// with (`byte_80210DD`'s row: the navi's `fresh` and `weapons`).
fn fresh_stats(navi: NaviHandle, content: &Content) -> NaviStats {
    let data = content.navi(navi);
    let Some(fresh) = data.fresh else {
        panic!("navi {:?}'s fresh stats read past their table (init_8013B64)", content.defs.navi(navi).key);
    };
    let defaults = NaviStats::default();
    let base = content.base_form();
    NaviStats {
        version: 1,
        reg_up: 4,
        custom_level: 5,
        support: Some(Default::default()),
        mood: 0x99,
        beast_out_counter: 3,
        form: base,
        starting_form: base,
        folder: 0,
        folder_reg: [0xFF; 2],
        folder_tags: [[0xFF; 2]; 2],
        navi,
        max_base_hp: fresh.hp,
        hp: fresh.hp,
        max_hp: fresh.hp,
        super_armor: fresh.super_armor,
        float_shoes: fresh.float_shoes,
        air_shoes: fresh.air_shoes,
        undershirt: fresh.undershirt,
        first_barrier: fresh.first_barrier,
        mega_level: fresh.mega_level,
        giga_level: fresh.giga_level,
        weapons: NaviWeapons {
            buster: data.weapons.buster,
            charge_shot: data.weapons.charge_shot,
            back_special: data.weapons.back_special,
            a_charge: data.weapons.a_charge,
            back_special_damage: fresh.back_special_damage,
            ..defaults.weapons
        },
        bugs: crate::setup::NaviCustBugs { panel_trail_kind: 0xFF, ..defaults.bugs },
        ..defaults
    }
}

/// `sub_802DD70(navi, side)`: a link navi's HP (and max) after a change
/// (`byte_802DD88`: the navi's `cross_hp`; the game passes the side where
/// the table's column is the navi's level).
fn changed_hp(b: &Battle, navi: NaviHandle, side: u8) -> (u16, u16) {
    let Some(row) = b.content.navi(navi).cross_hp else {
        panic!("navi {:?}'s HP after a Cross change reads past its table (sub_802DD70)", b.content.defs.navi(navi).key);
    };
    let hp = row[side as usize & 1];
    (hp, hp)
}
