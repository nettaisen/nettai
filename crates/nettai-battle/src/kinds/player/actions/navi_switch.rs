//! Changing navis mid-battle: the navi switch.
//!
//! **The framework's switch** (the battle flag 0x40 mode's link navi
//! switch, the GBA's): a player's transformation record names the navi to
//! change to (+4); at the turn's start the transformation sequencer asks
//! the navi (`sub_802DCDE`), and the pause handler runs the change as action
//! 0x1C (`sub_802D714`): the navi lands, becomes the other navi (its stats
//! kept or fresh: the party's member 0, `Battle::party`), and after 21
//! ticks goes on as it. A navi changed so falls back to the one it was
//! instead of being deleted (`sub_802DD2A`): the switch knockout
//! (`sub_802D926`) brings the kept navi back. See
//! docs/engine/battle-flow.md §3.4. Only the own-gauges mode sends one; no
//! recording has one, so all of it is unverified.
//!
//! **A party's switch** (a side with partners: the DS's party battle,
//! docs/design/exe5ds-map.md §4.3–§4.7): the same requests and steps,
//! rewritten. A switch goes to a member of the side's party, whose stats
//! block it takes whole (its HP among them), and leaves the navi's in the
//! outgoing member's; it is taken at a turn's start as the framework's is
//! (the custom screen names the navi: `change`), or in battle at the
//! player's request, with a dimming of its own (action 0x59, `in_battle`).
//! A switched-in partner at 0 HP falls back to member 0, the side's own
//! navi (`take_back`). Each settles into the side's rules
//! (`navi_switched`), where the DS's Double Attacks start. No DS recording
//! exists yet: all of it is read from the code, unverified.

use super::ActionVars;
use crate::actor::{request, status};
use crate::battle::{Battle, battle_flags};
use crate::collision::{f1, timer};
use crate::kinds::common;
use crate::kinds::player::{
    ai, ai_mut, clear_flag1, clear_flag2, clear_invulnerable, clear_statuses, coll_mut, exit_attack_state, form,
    is_megaman, load_sprite, post_init_hook, reset_status, reset_status_tail, set_coordinates_from_panel, stats, status as navi_status,
    update_element,
};
use crate::object::ObjectRef;
use crate::content::Content;
use crate::setup::{NaviStats, effects};
use nettai_content_api::NaviHandle;

/// The action's own state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Vars {
    /// AIAttackVars+0x10: ticks left in the step.
    pub timer: u16,
    /// A party's switch: the member it leaves (the DS keeps the navi, +0x30)
    /// and that navi's mood as it left, before its anger ended (+0x38).
    pub from: u8,
    pub mood: u8,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    let a = &mut ai_mut(b, r).attack;
    if !matches!(a.action, ActionVars::NaviSwitch(_)) {
        a.action = ActionVars::NaviSwitch(Vars::default());
    }
    match &mut a.action {
        ActionVars::NaviSwitch(v) => v,
        _ => unreachable!(),
    }
}

fn set_step(b: &mut Battle, r: ObjectRef, step: u8) {
    let a = &mut ai_mut(b, r).attack;
    a.step = step;
    a.step_init = 0;
}

fn side_of(b: &Battle, r: ObjectRef) -> usize {
    b.objects.get(r).alliance as usize & 1
}

fn has_partners(b: &Battle, r: ObjectRef) -> bool {
    b.party[side_of(b, r)].has_partners()
}

/// `sub_802DCDE`: the transformation sequencer asks `navi` to change.
pub(crate) fn request_change(b: &mut Battle, navi: ObjectRef) {
    ai_mut(b, navi).requests |= request::NAVI_SWITCH;
}

/// The end of the navi's input (the DS's input handler, 0x0203C5F6, on a
/// tick that asks no chip): the player's switch request
/// (`ActorData::switch_input`) asks the switch, when the side's rules take
/// it on (`switch_asked`: the DS's battle mode 6 calls a partner instead),
/// no switch or knockout is asked for or runs (`sub_802DE5C`, 0x0200E064),
/// the member isn't the one on the field, and isn't busy (`member_busy`).
/// The live request handler then starts the switch (`in_battle`). The
/// request isn't cleared: it is the input's, which a held press repeats.
pub(crate) fn request_in_battle(b: &mut Battle, r: ObjectRef) {
    let side = side_of(b, r);
    let Some(member) = ai(b, r).switch_input else { return };
    if !b.party[side].has_partners() || !b.rules_switch_asked(side as u8, r, member) {
        return;
    }
    // sub_802DE5C
    let a = ai(b, r);
    if a.requests & (request::NAVI_SWITCH | request::SWITCH_KNOCKOUT) != 0
        || a.status & (status::SWITCHING_NAVI | status::SWITCH_KNOCKOUT) != 0
    {
        return;
    }
    if member == b.party[side].active || b.rules_member_busy(side as u8, member) {
        return;
    }
    let a = ai_mut(b, r);
    a.switch_member = Some(member);
    a.requests |= request::NAVI_SWITCH;
}

/// `sub_802D714`: one paused tick of the change, then the sprite steps
/// (`sub_801BCD0`). A party's (0x0202BEA4) lands for 20 ticks, takes the
/// member's block and settles into its rules.
pub(in crate::kinds::player) fn change(b: &mut Battle, r: ObjectRef) {
    let party = has_partners(b, r);
    match ai(b, r).attack.step {
        // sub_802D738, 0x0202BEB8
        0 if party => party_land(b, r, Land::Paused),
        0 => land(b, r, false),
        // sub_802D7A0, 0x0202BF98
        4 if party => {
            let side = side_of(b, r);
            let navi = b.turn_transforms[side]
                .navi_switch
                .unwrap_or_else(|| panic!("a navi switch without a navi reads 0xFF as one (0x0200DEC8)"));
            let Some(member) = member_of(b, side, navi) else {
                panic!("side {side}'s party has no member of navi {:?}: the DS loops forever (0x0200DEE8)", b.content.defs.navi(navi).key)
            };
            party_change(b, r, member, false);
        }
        4 => become_other_navi(b, r),
        // sub_802D8F0, 0x0202C130
        8 if party => {
            if settle(b, r) {
                let a = ai_mut(b, r);
                a.status |= status::SWITCHED;
                a.status &= !(status::TRAP_ARMED | status::SWITCHING_NAVI | status::HOVERING);
                a.requests &= !(request::ANTI_SWORD_TRIGGERED | request::ANTI_DAMAGE_TRIGGERED);
                clear_flag2(b, r, 0x4);
                settled(b, r);
                exit_attack_state(b, r);
            }
        }
        8 => {
            if settle(b, r) {
                // (The HUD shows the navi again.)
                let a = ai_mut(b, r);
                a.status |= status::SWITCHED;
                a.status &= !(status::TRAP_ARMED | status::SWITCHING_NAVI);
                a.requests &= !(request::BODY_GUARD_TRIGGERED | request::ANTI_SWORD_TRIGGERED | request::ANTI_DAMAGE_TRIGGERED);
                exit_attack_state(b, r);
            }
        }
        s => panic!("navi switch step {s:#x} reads past its table (off_802D72C)"),
    }
    common::step_sprite(b, r);
}

/// The DS's action 0x59 (0x0202C67C, its table 0x020D9B44): a party's
/// switch in battle, to the member the navi's request named
/// (`ActorData::switch_member`). It dims the battle as it lands (battle flag
/// 4), counts the switch, and undims as it settles; then the sprite steps.
pub(crate) fn in_battle(b: &mut Battle, r: ObjectRef) {
    match ai(b, r).attack.step {
        // 0x0202C690
        0 => party_land(b, r, Land::InBattle),
        // 0x0202C760
        4 => {
            let Some(member) = ai(b, r).switch_member else {
                panic!("an in-battle switch to no member (AIData +0x72)")
            };
            party_change(b, r, member, true);
        }
        // 0x0202C8F8
        8 => {
            if settle(b, r) {
                let a = ai_mut(b, r);
                a.status &= !(status::TRAP_ARMED | status::SWITCHING_NAVI | status::HOVERING);
                a.requests &= !(request::ANTI_SWORD_TRIGGERED | request::ANTI_DAMAGE_TRIGGERED);
                clear_flag2(b, r, 0x104);
                b.clear_flags(battle_flags::DIMMED);
                settled(b, r);
                exit_attack_state(b, r);
            }
        }
        s => panic!("in-battle switch step {s:#x} reads past its table (0x020D9B44)"),
    }
    common::step_sprite(b, r);
}

/// `sub_802D926` (the pause handler sets the variant to 0 first): one
/// paused tick of the switch knockout; the sprite steps with variant 0
/// (`sub_801BCD0`). A party's (0x0202C1A0) brings back member 0.
pub(in crate::kinds::player) fn knock_out(b: &mut Battle, r: ObjectRef) {
    let party = has_partners(b, r);
    match ai(b, r).attack.step {
        // sub_802D950
        0 => land(b, r, true),
        // sub_802D9B0, 0x0202C24C
        4 if party => {
            take_back(b, r);
            set_step(b, r, 8);
        }
        4 => take_back_reserve(b, r),
        // sub_802DA78, 0x0202C380
        8 if party => {
            if settle(b, r) {
                let a = ai_mut(b, r);
                a.status &= !(status::TRAP_ARMED | status::SWITCH_KNOCKOUT | status::SWITCHED | status::HOVERING);
                a.requests &= !(request::NAVI_SWITCH | request::ANTI_SWORD_TRIGGERED | request::ANTI_DAMAGE_TRIGGERED);
                clear_flag2(b, r, 0x4);
                exit_attack_state(b, r);
            }
        }
        8 => {
            if settle(b, r) {
                let a = ai_mut(b, r);
                a.status &= !(status::TRAP_ARMED | status::SWITCH_KNOCKOUT | status::SWITCHED);
                a.requests &= !(request::NAVI_SWITCH
                    | request::BODY_GUARD_TRIGGERED
                    | request::ANTI_SWORD_TRIGGERED
                    | request::ANTI_DAMAGE_TRIGGERED);
                exit_attack_state(b, r);
            }
        }
        s => panic!("switch knockout step {s:#x} reads past its table (off_802D944)"),
    }
    // (The original steps the sprite when the attack's variant byte is 0,
    // which its only caller, the pause handler, stores first.)
    common::step_sprite(b, r);
}

/// What flags the landings clear (the GBA's `sub_802D738` passes the
/// flags' mask to both clears; a party's, the DS's 0x101C41 and 0x110).
const LANDING: u32 = f1::SLIDING | f1::PARALYZED | f1::FLINCHING | f1::MOVING | f1::GUARD;

/// `sub_802D738` / `sub_802D950`: onto the destination panel on the
/// ground, facing the default way, out of any slide, paralysis, flinch,
/// move or guard (the requests' clear takes the same bits: the game passes
/// the flags' mask again), the animation 4, links cut; the change keeps its
/// collision region on, the knockout ends the Full Synchro aura's link.
/// Four ticks. A party's knockout (0x0202C1BC) clears the DS's masks.
fn land(b: &mut Battle, r: ObjectRef, knockout: bool) {
    if ai(b, r).attack.step_init == 0 {
        to_destination(b, r);
        if has_partners(b, r) {
            clear_flag1(b, r, LANDING | f1::DRAG);
            clear_flag2(b, r, 0x110);
            b.objects.get_mut(r).slide_state = 0;
        } else {
            clear_flag1(b, r, LANDING);
            clear_flag2(b, r, LANDING);
        }
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

/// The landings' start: onto the destination panel on the ground, its
/// reservation gone, facing the default way.
fn to_destination(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    o.panel = o.future_panel;
    let p = o.panel;
    b.unreserve_panel(r, p.x, p.y);
    set_coordinates_from_panel(b, r);
    b.objects.get_mut(r).pos.z = 0;
    b.update_collision_panels(r);
    super::transform::face_default(b, r);
}

/// Which of a party's switches lands.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Land {
    /// At a turn's start, while paused (0x0202BEB8).
    Paused,
    /// In battle (0x0202C690): it dims the battle.
    InBattle,
}

/// A party's landing (0x0202BEB8, 0x0202C690): the navi shows white, lands
/// on its destination, a flash (the roles' `deletion` effect, 16 pixels
/// up; the paused one's runs while paused), the switch's sound, the DS's
/// flags cleared, animation 0, the overlay told (its third parameter, 1),
/// links cut, its collision region on. The voice the DS plays for the
/// incoming navi is left out (the user, 2026-10-20), but the draw of the
/// side's console RNG that picks it stays (0x02098ED0): on the local
/// side, without effect `NO_SWITCH_VOICE`, and (the paused one) not to
/// member 0 in the first turns (the DS's MegaMan, ProtoMan and Colonel in
/// slot 0 on turn 1 at most: a party's member 0 is its own navi, MegaMan).
/// 20 ticks.
fn party_land(b: &mut Battle, r: ObjectRef, how: Land) {
    b.objects.sprite_mut(r).look.white = true;
    if ai(b, r).attack.step_init == 0 {
        if how == Land::InBattle {
            b.set_flags(battle_flags::DIMMED);
        }
        to_destination(b, r);
        let pos = b.objects.get(r).pos;
        let look = b.roles().effect(crate::content::EffectRole::Deletion);
        let flash = crate::kinds::effect::spawn(b, crate::object::Vec3 { z: pos.z.wrapping_add(0x10_0000), ..pos }, look, 0, 0, 0);
        if how == Land::Paused
            && let Some(f) = flash
        {
            b.objects.get_mut(f).flags |= crate::object::flags::RUN_WHILE_PAUSED;
        }
        let side = side_of(b, r);
        let target = match how {
            Land::InBattle => ai(b, r).switch_member,
            Land::Paused => b.turn_transforms[side].navi_switch.and_then(|n| member_of(b, side, n)),
        };
        let quiet = how == Land::Paused && target == Some(0) && b.round.turn <= 1;
        if !quiet && side as u8 == b.round.local_side && b.setup.settings.effects & effects::NO_SWITCH_VOICE == 0 {
            b.consoles[side].rng.next();
        }
        b.sound(crate::content::SoundRole::NaviSwitch);
        clear_flag1(b, r, LANDING | f1::DRAG);
        clear_flag2(b, r, 0x110);
        b.objects.get_mut(r).slide_state = 0;
        common::set_animation(b, r, 0);
        if let Some(overlay) = b.objects.get(r).related[1] {
            form::tell_overlay(b, overlay);
        }
        b.objects.get_mut(r).related[0] = None;
        ai_mut(b, r).overlay = None;
        coll_mut(b, r).region = b.anchor_region();
        vars(b, r).timer = 0x14;
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

/// A party's switch settled: its rules hear of it (`navi_switched`: the DS's
/// Double Attack launcher, 0x02042DC0, in battle; its trigger, 0x02042E74,
/// while paused), before the action ends.
fn settled(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance;
    let Vars { from, mood, .. } = *vars(b, r);
    b.rules_navi_switched(side, r, from, mood);
}

/// The member of `side`'s party whose block is of `navi` (the DS's
/// 0x0200DEE8: none offline for side 1).
fn member_of(b: &Battle, side: usize, navi: NaviHandle) -> Option<u8> {
    if b.setup.settings.effects & effects::LINK == 0 && side != 0 {
        return None;
    }
    b.party[side].member_of(navi)
}

/// The navi `r` becomes the one its stats name: its actor record, sprite
/// (animation `anim`) and overlays; the statuses and anger end.
fn take_identity(b: &mut Battle, r: ObjectRef, anim: u8) {
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
    common::set_animation(b, r, anim);
}

/// `sub_802D7A0`: the change itself (one tick).
fn become_other_navi(b: &mut Battle, r: ObjectRef) {
    let side = side_of(b, r);
    let old_form = stats(b, r).form;
    form::take_off_overlay(b, r, old_form);
    let identity = b.objects.get(r).identity;
    form::navi_death_hook(b, r, identity);
    navi_status::end_anger(b, r);
    // The navi it leaves is kept when it is the kept one (with its HP).
    if b.party[side].members[0].navi == b.stats[side].navi {
        b.stats[side].hp = b.objects.get(r).hp;
        b.party[side].members[0] = b.stats[side];
    }
    // sub_802DCCC: the record's navi.
    let Some(target) = b.turn_transforms[side].navi_switch else {
        panic!("a navi switch without a navi reads 0xFF as one (sub_802DCCC)");
    };
    let kept = b.party[side].members[0].navi == target;
    b.stats[side] = if kept { b.party[side].members[0] } else { fresh_stats(target, &b.content) };
    super::super::refresh_abilities(b, r);
    take_identity(b, r, 3);
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

/// A party's change (0x0202BF98 while paused, 0x0202C760 in battle; one
/// tick): the form comes off and the navi is in its base form again (Soul
/// Unison ends), its barrier gone, it came in by a switch; the member it
/// leaves and its mood kept for the settle; its anger over; its stats,
/// with the HP it has, back into the outgoing member's block; the
/// incoming member's block taken whole, its HP and maximum the block's; a
/// navi that doesn't change form at the normal mood (0x80); then its
/// identity (animation 0), form or init hook, its statuses and anger
/// ended, and the framework's tail. In battle it also counts the switch
/// and drops the link navis' chips from the hand (`sub_80108FC`).
fn party_change(b: &mut Battle, r: ObjectRef, member: u8, in_battle: bool) {
    let side = side_of(b, r);
    let old_form = stats(b, r).form;
    form::take_off_overlay(b, r, old_form);
    let identity = b.objects.get(r).identity;
    form::navi_death_hook(b, r, identity);
    let navi = b.stats[side].navi;
    b.stats[side].form = b.content.base_form_for(navi);
    // sub_801A7F4
    coll_mut(b, r).barrier = 0;
    ai_mut(b, r).barrier_visual = None;
    ai_mut(b, r).status |= status::SWITCHED;
    let (from, mood) = (b.party[side].active, b.stats[side].mood);
    let v = vars(b, r);
    (v.from, v.mood) = (from, mood);
    navi_status::end_anger(b, r);
    b.stats[side].hp = b.objects.get(r).hp;
    let outgoing = b.stats[side];
    let party = &mut b.party[side];
    *party.member_mut(from).expect("the member on the field is the party's") = outgoing;
    b.stats[side] = *party.member(member).unwrap_or_else(|| panic!("side {side}'s party has no member {member}"));
    party.active = member;
    if in_battle {
        // 0x02036E48
        party.switches = party.switches.wrapping_add(1);
    }
    if !b.content.navi(b.stats[side].navi).changes_form() {
        b.stats[side].mood = 0x80;
    }
    super::super::refresh_abilities(b, r);
    take_identity(b, r, 0);
    let s = *stats(b, r);
    if is_megaman(b, r) {
        form::put_on_overlay(b, r, s.form);
    } else {
        form::navi_init_hook(b, r, b.objects.get(r).identity);
    }
    post_init_hook(b, r);
    clear_statuses(b, r);
    navi_status::end_anger(b, r);
    if in_battle {
        let content = b.content.clone();
        b.hands[side].drop_link_navi_chips(&content);
    }
    let o = b.objects.get_mut(r);
    o.hp = s.hp;
    o.max_hp = s.max_hp;
    finish_party_change(b, r);
    // 0x02098804
    coll_mut(b, r).status_timers[timer::FLASH] = 0;
    clear_flag1(b, r, f1::INVISIBLE);
    b.sound(crate::content::SoundRole::Invisible);
    set_step(b, r, 8);
}

/// `sub_802D9B0`: the knockout's change back to the kept navi (one tick).
fn take_back_reserve(b: &mut Battle, r: ObjectRef) {
    let side = side_of(b, r);
    let identity = b.objects.get(r).identity;
    form::navi_death_hook(b, r, identity);
    b.stats[side] = b.party[side].members[0];
    take_identity(b, r, 3);
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

/// A party's knockout's take-back (0x0202C24C while paused, the in-battle
/// knockout's 0x0202C4CC: EXE5's action 0x4C, its rules' `switch_knockout`,
/// asks it through the API; one tick): the fallen member's block keeps its
/// stats at the HP it has (0), and member 0, the side's own navi, comes
/// back with its block: its identity (animation 3), init hook and form,
/// its statuses, anger and barrier gone, its HP and maximum the block's,
/// the link navis' chips dropped from the hand, and the framework's tail.
pub(crate) fn take_back(b: &mut Battle, r: ObjectRef) {
    let side = side_of(b, r);
    let identity = b.objects.get(r).identity;
    form::navi_death_hook(b, r, identity);
    b.stats[side].hp = b.objects.get(r).hp;
    let fallen = b.stats[side];
    let party = &mut b.party[side];
    if let Some(m) = party.member_mut(party.active) {
        *m = fallen;
    }
    b.stats[side] = party.members[0];
    party.active = 0;
    take_identity(b, r, 3);
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
    // sub_801A7F4
    coll_mut(b, r).barrier = 0;
    ai_mut(b, r).barrier_visual = None;
    let o = b.objects.get_mut(r);
    o.hp = s.hp;
    o.max_hp = s.max_hp;
    let content = b.content.clone();
    b.hands[side].drop_link_navi_chips(&content);
    finish_party_change(b, r);
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
    // EXE5's (0x08011918) disarms the side's ColonelSoul army
    // (`obstacle::Conversion`): EXE6 has none to disarm.
    b.obstacle_conversion[side].armed = false;
    // `off_801426C`, by the navi: MegaMan's (the navi that changes form)
    // is his status reset; the others' are their `reset` (EXE6's link
    // navis' are nothing; EXE5's ToadMan dives again, 0x080119A0).
    if is_megaman(b, r) {
        reset_status_tail(b, r, false);
    } else {
        let navi = stats(b, r).navi;
        if let Some(f) = b.content.defs.navi(navi).reset {
            crate::behavior::call_hook(b, f, nettai_content_api::HookCall::FormNavi { navi: r });
        }
    }
    clear_invulnerable(b, r);
}

/// A party's changes' and take-back's tail: the framework's, with
/// `exe5_802C126` before the element (the no-charge drive over, the auto
/// battle AI's drive off). (A change then ends the navi's invisibility,
/// 0x02098804; a take-back doesn't.)
fn finish_party_change(b: &mut Battle, r: ObjectRef) {
    // exe5_802C126
    let a = ai_mut(b, r);
    a.no_charge_timer = 0;
    a.in_auto_battle = false;
    a.status &= !status::NO_CHARGE;
    finish_change(b, r);
}

/// `init_8013B64`: `navi`'s stats, fresh ([`NaviStats::fresh`]).
fn fresh_stats(navi: NaviHandle, content: &Content) -> NaviStats {
    NaviStats::fresh(navi, content)
        .unwrap_or_else(|| panic!("navi {:?}'s fresh stats read past their table (init_8013B64)", content.defs.navi(navi).key))
}

/// `sub_802DD70(navi, side)`: a link navi's HP (and max) after a change
/// (`byte_802DD88`: the navi's `switch_hp`; the game passes the side where
/// the table's column is the navi's level).
fn changed_hp(b: &Battle, navi: NaviHandle, side: u8) -> (u16, u16) {
    let Some(row) = b.content.navi(navi).switch_hp else {
        panic!("navi {:?}'s HP after a navi switch reads past its table (sub_802DD70)", b.content.defs.navi(navi).key);
    };
    let hp = row[side as usize & 1];
    (hp, hp)
}
