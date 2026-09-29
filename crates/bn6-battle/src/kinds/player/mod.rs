//! The player navi (actor #0 with actor type Player): spawn, init, the
//! per-tick pipeline, and its actions. See docs/engine/objects-and-player.md
//! chapter 12, and field-collision-damage.md chapter 4 for damage intake.
//!
//! The per-tick pipeline (`sub_80EA484`) is: input and charge (`input`),
//! hit collection (`intake`, stage A), damage/status application and the
//! action dispatch (`status`, stage B), then a few per-tick counters and
//! the collision re-registration. Actions below 0x10 live in `entry`
//! (0, 1), `reactions` (2..7) and `idle` (8); 0x10 and up in `actions`.

pub mod actions;
mod chip_use;
mod entry;
mod form;
mod idle;
mod input;
mod intake;
mod reactions;
mod status;

use crate::actor::{ActorData, ActorId, ActorType, request};
use crate::battle::{Battle, battle_flags};
use crate::collision::{CollisionData, CollisionId, f1, timer};
use crate::field::PanelType;
use crate::data::player::{self as pdata, NaviRecord};
use crate::hand::NO_CHIP;
use crate::object::{ObjectRef, PanelPos, Pool, StateWord, Vec3, flags, state};
use crate::setup::{ActorEntry, Form, Navi, NaviStats, effects};

/// Panel center coordinates (`object_getCoordinatesForPanels`).
pub fn panel_coordinates(x: u8, y: u8) -> (i32, i32) {
    ((x as i32 * 40 - 140) << 16, (y as i32 * 24 - 20) << 16)
}

/// The panel under a position (`sub_800E258`; the game divides toward
/// zero).
pub fn coordinates_to_panel(x: i32, y: i32) -> PanelPos {
    PanelPos { x: (((x >> 16) + 0xA0) / 0x28) as u8, y: (((y >> 16) + 0x20) / 0x18) as u8 }
}

/// Spawn a player navi from an actor-list entry (`sub_800753C`).
pub fn spawn(b: &mut Battle, entry: &ActorEntry) -> Option<ObjectRef> {
    let r = b.objects.spawn(Pool::Actor, 0, Vec3::default(), [0; 4])?;
    let (x, y) = panel_coordinates(entry.x, entry.y);
    {
        let o = b.objects.get_mut(r);
        o.alliance = entry.alliance;
        o.panel = PanelPos { x: entry.x, y: entry.y };
        o.future_panel = o.panel;
        o.pos = Vec3 { x, y, z: 0 };
        o.flags |= flags::RUN_WHILE_PAUSED;
    }
    let Some(a) = b.actors.allocate() else {
        b.objects.free(r);
        return None;
    };
    b.objects.get_mut(r).actor = Some(a);
    b.actors.get_mut(a).actor_type = ActorType::Player;
    let navi = b.stats[entry.alliance as usize].navi;
    let name_id = 0x1A0 + navi.0 as u16;
    b.objects.get_mut(r).name_id = name_id;
    // The actor record (`sub_80182B4`); MegaMan's is {0, Player, 0}.
    let rec = pdata::navi_record(name_id);
    let ad = b.actors.get_mut(a);
    ad.actor_type = rec.actor_type;
    ad.ai_index = rec.ai_index;
    Some(r)
}

/// The player's update (`sub_80EA460`): lifecycle state, then the sprite
/// step every tick.
pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => tick(b, r),
        _ => destroy(b, r),
    }
    update_sprite(b, r);
}

// ---- Accessors -------------------------------------------------------------

/// The player's collision data. The game reads it without a null check;
/// players always have one (after deletion, a freed one).
fn coll_id(b: &Battle, r: ObjectRef) -> CollisionId {
    b.objects.get(r).collision.expect("player has collision data")
}

fn coll(b: &Battle, r: ObjectRef) -> &CollisionData {
    b.collision.get(coll_id(b, r))
}

fn coll_mut(b: &mut Battle, r: ObjectRef) -> &mut CollisionData {
    let id = coll_id(b, r);
    b.collision.get_mut(id)
}

fn actor_id(b: &Battle, r: ObjectRef) -> ActorId {
    b.objects.get(r).actor.expect("player has actor data")
}

fn ai(b: &Battle, r: ObjectRef) -> &ActorData {
    b.actors.get(actor_id(b, r))
}

fn ai_mut(b: &mut Battle, r: ObjectRef) -> &mut ActorData {
    let id = actor_id(b, r);
    b.actors.get_mut(id)
}

/// The navi stats of the object's side.
fn stats(b: &Battle, r: ObjectRef) -> &NaviStats {
    &b.stats[b.objects.get(r).alliance as usize]
}

fn stats_mut(b: &mut Battle, r: ObjectRef) -> &mut NaviStats {
    let side = b.objects.get(r).alliance as usize;
    &mut b.stats[side]
}

/// `object_getFlag`: status flags (ObjectFlags1).
fn flag1(b: &Battle, r: ObjectRef) -> u32 {
    coll(b, r).f1
}

/// `object_setFlag1`.
fn set_flag1(b: &mut Battle, r: ObjectRef, bits: u32) {
    coll_mut(b, r).f1 |= bits;
}

/// `object_clearFlag`.
fn clear_flag1(b: &mut Battle, r: ObjectRef, bits: u32) {
    coll_mut(b, r).f1 &= !bits;
}

/// `object_getFlag2`: requests (ObjectFlags2).
fn flag2(b: &Battle, r: ObjectRef) -> u32 {
    coll(b, r).f2
}

/// `object_setFlag2`.
fn set_flag2(b: &mut Battle, r: ObjectRef, bits: u32) {
    coll_mut(b, r).f2 |= bits;
}

/// `object_clearFlag2`.
fn clear_flag2(b: &mut Battle, r: ObjectRef, bits: u32) {
    coll_mut(b, r).f2 &= !bits;
}

/// `GetBattleEffects() & 8`: a link battle.
fn is_link(b: &Battle) -> bool {
    b.setup.settings.effects & effects::LINK != 0
}

/// `sub_800A8F8`: battle flag 0x40 (not set in PvP).
fn is_mode_40(b: &Battle) -> bool {
    b.round.flags & battle_flags::MODE_40 != 0
}

/// `GetBattleMode`.
fn battle_mode(b: &Battle) -> u8 {
    b.round.mode_copy
}

/// `sub_80107C0`: the hit modifier bodies inflict (3 in link battles).
fn body_hit_modifier(b: &Battle) -> u8 {
    if is_link(b) { 3 } else { 0 }
}

/// `object_getFlipDirection`: +1 facing right, -1 facing left.
fn flip_direction(alliance: u8, flip: u8) -> i32 {
    if alliance ^ flip == 0 { 1 } else { -1 }
}

/// `sub_80182B4`: the object's actor record.
fn navi_record(b: &Battle, r: ObjectRef) -> NaviRecord {
    pdata::navi_record(b.objects.get(r).name_id)
}

/// `sub_8018810`: the object's sprite attach point `index`, in pixels,
/// facing the object's way.
pub(crate) fn attach_point(b: &Battle, r: ObjectRef, index: usize) -> (i32, i32) {
    let o = b.objects.get(r);
    if (0xCD..=0xFF).contains(&o.name_id) {
        return (0, 7);
    }
    let p = pdata::attach_point(o.name_id, index);
    (p.x as i32 * flip_direction(o.alliance, o.flip), p.y as i32)
}

/// The panel type under (x, y); off the field the game reads BIOS memory,
/// taken here as "no panel".
fn panel_kind(b: &Battle, p: PanelPos) -> PanelType {
    b.field.panel(p.x, p.y).map(|p| p.kind).unwrap_or_default()
}

/// `sub_8010004`: the next chip in the side's hand.
fn next_chip(b: &Battle, r: ObjectRef) -> u16 {
    let hand = &b.hands[b.objects.get(r).alliance as usize];
    hand.ids.get(hand.cursor as usize).copied().unwrap_or(NO_CHIP)
}

/// `sub_8015B54`: a side's emotion (5 worn out, 3 angry, 1 ..., 2 full
/// synchro, 0 normal).
fn emotion(b: &Battle, side: u8) -> u8 {
    let mood = b.stats[side as usize].mood;
    let p = b.player(side).expect("side has a player");
    let a = ai(b, p);
    if a.beast_over_exhausted || mood == 0 {
        5
    } else if a.anger != 0 {
        3
    } else if a.beast_out_spent {
        1
    } else if mood == 0xFF {
        2
    } else {
        0
    }
}

/// `sub_8015BEC`: set a side's mood, unless its navi is in a special
/// emotion state.
fn set_mood(b: &mut Battle, side: u8, mood: u8) {
    let Some(p) = b.player(side) else { return };
    let a = ai(b, p);
    if a.beast_out_spent || a.beast_over_exhausted {
        return;
    }
    b.stats[side as usize].mood = mood;
}

/// Save the object's lifecycle position (`obj+0x5C`) unless one is saved.
fn save_state_word(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    if o.saved_state.is_none() {
        o.saved_state = Some(StateWord { state: o.state, action: o.action, phase: o.phase, phase_init: o.phase_init });
    }
}

/// `sub_802DD2A`: a Cross navi that falls back to base form instead of
/// dying.
fn cross_protected(b: &Battle, r: ObjectRef) -> bool {
    stats(b, r).navi != Navi::MEGAMAN && ai(b, r).status & crate::actor::status::CROSSED != 0
}

/// Switch to `action` at phase 0 (the game's direct CurAction stores).
fn set_action(b: &mut Battle, r: ObjectRef, action: u8) {
    let o = b.objects.get_mut(r);
    o.action = action;
    o.phase = 0;
    o.phase_init = 0;
}

/// `object_setAttack0..5`: start `action`, recording which helper started
/// it in the attack variables (§M2.2).
fn set_attack(b: &mut Battle, r: ObjectRef, action: u8, kind: u8) {
    set_action(b, r, action);
    let a = &mut ai_mut(b, r).attack;
    a.step = 0;
    a.step_init = 0;
    a.kind = kind;
    reset_attack_links(b, r);
}

/// `sub_801011A`: clear the attack's link bytes and unfreeze the Beast
/// Out lock-on marker (`sub_80E1662`; a no-op without one).
fn reset_attack_links(b: &mut Battle, r: ObjectRef) {
    let a = ai_mut(b, r);
    a.attack.beast_lockon = 0;
    if let Some(marker) = a.lockon_marker {
        crate::kinds::lockon_marker::unfreeze(b, marker);
    }
}

/// `object_exitAttackState`: back to the idle action with animation 0.
fn exit_attack_state(b: &mut Battle, r: ObjectRef) {
    b.objects.get_mut(r).anim = 0;
    end_attack(b, r);
}

/// `sub_801171C`: leave the current attack for the idle action. A move
/// (kind 4) keeps pending requests and the charge.
fn end_attack(b: &mut Battle, r: ObjectRef) {
    let a = ai_mut(b, r);
    a.attack.special_source = 0;
    let kind = a.attack.kind;
    if kind != 4 {
        match kind {
            2 => a.lockout = a.attack.lockout,
            3 => a.back_special_cooldown = a.attack.lockout,
            _ => {}
        }
        a.buffered_move = 0;
        a.requests &= !(request::ATTACKS | request::MODE9_A);
        reset_charge(b, r);
        clear_flag1(b, r, f1::USING_ACTION);
    }
    b.objects.get_mut(r).action = 8;
    let a = ai_mut(b, r);
    a.attack.step = 0;
    a.attack.step_init = 0;
}

/// `sub_8012EA8`: drop the buster charge and the hold flags.
fn reset_charge(b: &mut Battle, r: ObjectRef) {
    reset_charge_counters(b, r);
    ai_mut(b, r).requests &= !request::HOLDS;
}

/// The charge counter, level and source back to zero.
fn reset_charge_counters(b: &mut Battle, r: ObjectRef) {
    let a = ai_mut(b, r);
    a.charge_level = 0;
    a.charge_counter = 0;
    a.charge_source = 0;
}

/// `sub_8019F8C`: set the object's element and its collision elements.
fn set_element(b: &mut Battle, r: ObjectRef, element: u8) {
    b.objects.get_mut(r).element = element;
    let c = coll_mut(b, r);
    c.element = element & 0xF;
    c.secondary_element = element & 0xF0;
}

/// `sub_800EB08`: end invulnerability.
fn clear_invulnerable(b: &mut Battle, r: ObjectRef) {
    coll_mut(b, r).status_timers[timer::INVULNERABLE] = 0;
    clear_flag1(b, r, f1::INVULNERABLE);
}

/// `sub_80101C4`: end the timed semi-intangible state.
fn cancel_semi_intangible(b: &mut Battle, r: ObjectRef) {
    coll_mut(b, r).status_timers[timer::SEMI_INTANGIBLE] = 0;
    clear_flag1(b, r, f1::SEMI_INTANGIBLE);
}

/// `sub_801A284`: end paralysis.
fn clear_paralysis(b: &mut Battle, r: ObjectRef) {
    clear_flag1(b, r, f1::PARALYZED);
    clear_flag2(b, r, 0x8);
    coll_mut(b, r).status_timers[timer::PARALYZE] = 0;
}

/// `sub_801A29A`: thaw.
fn clear_freeze(b: &mut Battle, r: ObjectRef) {
    crate::kinds::thaw(b, r);
}

/// `sub_801A2B0`: pop the bubble and restore the navi's resting height.
fn clear_bubble(b: &mut Battle, r: ObjectRef) {
    clear_flag1(b, r, f1::BUBBLED);
    clear_flag2(b, r, 0x2_0000);
    coll_mut(b, r).status_timers[timer::BUBBLE] = 0;
    let z16 = ai(b, r).bubble_base_z;
    let o = b.objects.get_mut(r);
    o.pos.z = (o.pos.z & 0xFFFF) | ((z16 as i32) << 16);
}

/// Snap a moving navi onto its destination panel (the reaction actions'
/// common entry, unless sliding).
fn snap_to_future_panel(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    o.panel = o.future_panel;
    let p = o.panel;
    b.unreserve_panel(r, p.x, p.y);
    set_coordinates_from_panel(b, r);
    b.update_collision_panels(r);
}

/// `object_setCoordinatesFromPanels`.
fn set_coordinates_from_panel(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    let (x, y) = panel_coordinates(o.panel.x, o.panel.y);
    o.pos.x = x;
    o.pos.y = y;
}

/// `sub_8011450`: restart the form overlay (`related[1]`) with the navi
/// after an animation change.
fn refresh_form_overlay(b: &mut Battle, r: ObjectRef) {
    let a = ai(b, r);
    if a.actor_type == ActorType::Virus {
        return;
    }
    let Some(overlay) = b.objects.get(r).related[1] else { return };
    match a.ai_index {
        0 | 1 | 9 | 13 | 16 | 18 | 19 => crate::kinds::form_overlay::restart(b, overlay),
        14 | 24 | 25.. => panic!("form overlay refresh for AI index {} is not implemented yet", a.ai_index),
        _ => {}
    }
}

// ---- The transformation sequencer's checks -------------------------------------

/// `sub_80159C6` + `sub_8015994`, once per turn start: the check runs
/// only while `beast_out_check_delay` is 0 (and sets it to 2), and a
/// Beast Out whose counter ran out asks to revert (request 0x40, handled
/// while paused).
pub fn check_beast_out_end(b: &mut Battle, r: ObjectRef) {
    let s = *stats(b, r);
    let revert = if battle_mode(b) == 1 {
        s.form.is_beast()
    } else {
        let a = ai_mut(b, r);
        if a.beast_out_check_delay != 0 {
            return;
        }
        a.beast_out_check_delay = 2;
        if s.beast_out_counter != 0 {
            return;
        }
        a.beast_out_spent = true;
        s.form.is_beast()
    };
    if revert {
        ai_mut(b, r).requests |= request::REVERT_FORM;
    }
}

/// `sub_80159A2`: a form reversion is pending or running.
pub fn reverting_form(b: &Battle, r: ObjectRef) -> bool {
    ai(b, r).status & crate::actor::status::REVERTING_FORM != 0 || ai(b, r).requests & request::REVERT_FORM != 0
}

/// `sub_801596E`: ask the navi to change form (it does so in the pause
/// handler, as action 0x1C).
pub fn request_form_change(b: &mut Battle, r: ObjectRef) {
    ai_mut(b, r).requests |= request::FORM_CHANGE;
}

/// `sub_801597C`: a form change is running.
pub fn changing_form(b: &Battle, r: ObjectRef) -> bool {
    ai(b, r).status & crate::actor::status::FORM_CHANGE != 0
}

/// `sub_802DCEC`: a Cross change is pending or running.
pub fn changing_cross(b: &Battle, r: ObjectRef) -> bool {
    ai(b, r).status & crate::actor::status::CHANGING_CROSS != 0 || ai(b, r).requests & request::CROSS_CHANGE != 0
}

// ---- Init --------------------------------------------------------------------

/// `sub_80172F0`: set up a freshly spawned player (§12.2).
fn init(b: &mut Battle, r: ObjectRef) {
    // sub_800F35C: the per-form init hook is a no-op for every form.
    let form = stats(b, r).starting_form;
    stats_mut(b, r).form = form;
    load_sprite(b, r);
    // sub_80142B0: bodies deal 10 in link battles.
    if is_link(b) {
        b.objects.get_mut(r).damage = 10;
    }
    if b.create_collision(r).is_none() {
        b.objects.free(r);
        return;
    }
    let hm = body_hit_modifier(b);
    b.setup_collision(r, 1, 2, hm);
    init_hp(b, r);
    init_navicust(b, r);
    update_element(b, r);
    if stats(b, r).navi == Navi::MEGAMAN {
        // sub_8015B22
        let form = stats(b, r).form;
        b.objects.get_mut(r).name_id = if form == Form::NONE { 0x1A0 } else { 0x1AB + form.0 as u16 };
    }
    let form = stats(b, r).form;
    if form != Form::NONE {
        panic!("form {form:?} set-up (sub_8011268) is not implemented yet");
    }
    reset_status(b, r);
    style_hook(b, r);
    // sub_801DB84, sub_8018856, sub_801DC06, sub_801DC36: the HP number
    // HUD table.
    enable_turning(b, r);
    crate::kinds::charge_glow::spawn(b, r);
    // sub_800F378: per-form post-init hook.
    if ai(b, r).ai_index == 10 {
        panic!("post-init hook sub_80F22F8 is not implemented yet");
    }
    if stats(b, r).form == Form::NONE {
        // sub_8010DD0: per-navi hook (`off_8010E0C`); none for MegaMan.
        let rec = navi_record(b, r);
        if matches!(rec.ai_index, 1 | 6 | 9 | 13 | 14 | 16 | 18 | 19 | 24 | 25..) {
            panic!("navi init hook for AI index {} is not implemented yet", rec.ai_index);
        }
    }
    reset_side_state(b, r);
    apply_starting_hp_bug(b, r);
    let o = b.objects.get_mut(r);
    o.state = state::UPDATE;
    o.action = 0;
    o.phase = 0;
    o.phase_init = 0;
}

/// `sub_800FC9E` + `sprite_load`: load the navi's battle sprite.
fn load_sprite(b: &mut Battle, r: ObjectRef) {
    let (navi, form) = (stats(b, r).navi, stats(b, r).form);
    let id = if navi == Navi::MEGAMAN { pdata::form_sprite(form) } else { pdata::navi_sprite(navi) };
    let sprite = b.objects.sprite_mut(r);
    sprite.load(id);
    sprite.set_animation(0);
    let o = b.objects.get_mut(r);
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.anim = 0;
}

/// `sub_80141C8`: HP from the navi stats (full HP unless the battle keeps
/// HP).
fn init_hp(b: &mut Battle, r: ObjectRef) {
    let s = *stats(b, r);
    let o = b.objects.get_mut(r);
    o.hp = s.max_hp;
    o.max_hp = s.max_hp;
    if b.setup.settings.effects & 4 == 0 {
        b.objects.get_mut(r).hp = s.hp;
    }
}

/// `sub_8013892`: counter strength, mood, first barrier and the
/// NaviCust-driven flags.
fn init_navicust(b: &mut Battle, r: ObjectRef) {
    b.objects.get_mut(r).stamina = 10;
    let eff = b.setup.settings.effects;
    if eff & effects::LINK != 0 || eff & 0x1_0000 != 0 || stats(b, r).mood != 0xFF {
        // sub_8015C2C: the starting mood.
        stats_mut(b, r).mood = 0x80;
    }
    if stats(b, r).first_barrier != 0 {
        // Also clobbers the AIData pointer the charge-glow spawn uses
        // (objects-and-player.md §15 item 5).
        panic!("FirstBarrier (sub_801A7CC) is not implemented yet");
    }
    if stats(b, r).beast_out_counter == 0 {
        ai_mut(b, r).beast_out_spent = true;
    }
    reset_navicust_state(b, r);
}

/// `sub_801390C`: weapon bytes, invulnerability and the NaviCust flags.
fn reset_navicust_state(b: &mut Battle, r: ObjectRef) {
    let w = stats(b, r).weapons;
    let a = ai_mut(b, r);
    a.charge_shot = w.charge_shot;
    a.back_special = w.back_special;
    clear_flag1(b, r, 0x0800_0000);
    clear_invulnerable(b, r);
    if ai(b, r).reset_linked_object.is_some() {
        panic!("ending the status reset's linked object (sub_80E5410) is not implemented yet");
    }
    apply_navicust_flags(b, r);
}

/// `sub_801393A`: refresh the weapon bytes (base form) and the NaviCust
/// flags after a NaviCust change.
fn refresh_navicust_state(b: &mut Battle, r: ObjectRef) {
    let s = *stats(b, r);
    if s.form == Form::NONE {
        let a = ai_mut(b, r);
        a.charge_shot = s.weapons.charge_shot;
        a.back_special = s.weapons.back_special;
    }
    apply_navicust_flags(b, r);
}

/// `loc_8013956`: FloatShoe (also changes what the body is), AirShoe,
/// ice, Undershirt and SuperArmor from the navi stats.
fn apply_navicust_flags(b: &mut Battle, r: ObjectRef) {
    let s = *stats(b, r);
    let hm = body_hit_modifier(b);
    if s.float_shoes {
        set_flag1(b, r, f1::FLOATSHOE);
        b.reset_collision_types(r, 0x10, 2, hm);
    } else {
        clear_flag1(b, r, f1::FLOATSHOE);
        b.reset_collision_types(r, 1, 2, hm);
    }
    let set = |b: &mut Battle, bit: u32, on: bool| {
        if on { set_flag1(b, r, bit) } else { clear_flag1(b, r, bit) }
    };
    set(b, f1::AIRSHOE, s.air_shoes);
    set_flag1(b, r, f1::AFFECTED_BY_ICE);
    set(b, f1::UNDERSHIRT, s.undershirt);
    set(b, f1::SUPERARMOR, s.super_armor);
}

/// `sub_801086C`: element and secondary weakness from the navi and form.
fn update_element(b: &mut Battle, r: ObjectRef) {
    let s = *stats(b, r);
    let element = if s.navi != Navi::MEGAMAN {
        pdata::navi_element(s.navi) as u8
    } else if s.form != Form::NONE {
        pdata::form_element(s.form) as u8
    } else {
        s.element
    };
    set_element(b, r, element);
    let weakness = if s.form != Form::NONE { pdata::form_weakness(s.form) } else { pdata::navi_weakness(s.navi) };
    coll_mut(b, r).secondary_weakness = weakness.0;
}

/// `sub_80144C0`: the full status reset (NaviCust state, hand bonuses,
/// hit modifier, region, charge, weapon bytes, element, body damage).
fn reset_status(b: &mut Battle, r: ObjectRef) {
    reset_navicust_state(b, r);
    let side = b.objects.get(r).alliance as usize;
    b.hands[side].charge_bonus = [0; 6];
    ai_mut(b, r).status &= !0x20;
    // (Netbattle, local player: removes the opponent's HUD entry.)
    let hm = body_hit_modifier(b);
    let c = coll_mut(b, r);
    c.hit_mod_base = hm;
    c.region = 1;
    reset_charge(b, r);
    load_weapons(b, r);
    form::apply_form_flags(b, r);
    update_element(b, r);
    // sub_80142C2
    if is_link(b) {
        coll_mut(b, r).self_damage = 10;
    }
}

/// `sub_800FEEC`: the weapon routine bytes, from the navi stats (base
/// form) or the form table.
fn load_weapons(b: &mut Battle, r: ObjectRef) {
    let s = *stats(b, r);
    let mode9 = battle_mode(b) == 9;
    let a = ai_mut(b, r);
    if s.form == Form::NONE {
        let w = s.weapons;
        a.mode9_a = if mode9 { w.mode9_a } else { 0xFF };
        a.buster = w.buster;
        set_charge_shot_routine(a, w.charge_shot);
        a.a_charge = w.a_charge;
        a.back_special = w.back_special;
        a.alt_a_charge = 0xFF;
    } else {
        let w = pdata::form_weapons(s.form);
        a.mode9_a = w.mode9_a;
        a.a_charge = w.a_charge;
        a.buster = w.buster;
        set_charge_shot_routine(a, w.charge_shot);
        a.back_special = w.back_special;
        a.alt_a_charge = w.alt_a_charge;
    }
}

/// `sub_800FF5E`: reload the base form's weapon bytes (after a NaviCust
/// change).
fn reload_base_weapons(b: &mut Battle, r: ObjectRef) {
    if stats(b, r).form == Form::NONE {
        load_weapons(b, r);
    }
}

/// `sub_800FFAA`: set the charge-shot routine; the special routines
/// 0x21..=0x26 stick, and change some buster routines.
fn set_charge_shot_routine(a: &mut ActorData, v: u8) {
    let special = |x: u8| (0x21..=0x26).contains(&x);
    if special(v) || !special(a.charge_shot) {
        a.charge_shot = v;
    }
    if special(a.charge_shot) {
        match a.buster {
            3 | 4 => a.buster = 0,
            0x2C => a.buster = 0x2B,
            _ => {}
        }
    }
}

/// `sub_8013E58`: the NaviCust battle-start hook (9 and 10 pick a random
/// variant).
fn style_hook(b: &mut Battle, r: ObjectRef) {
    let s = stats(b, r).bugs.battle_start;
    let variant = match s {
        9 => (b.rng.next() & 3) as u8 + 1,
        10 => (b.rng.next() & 3) as u8 + 5,
        _ => s,
    };
    if variant != 0 {
        panic!("NaviCust style hook {variant} (off_8013E9C) is not implemented yet");
    }
}

/// `sub_80141F4`: L/R turning, except with the standard column patterns.
fn enable_turning(b: &mut Battle, r: ObjectRef) {
    const DUSTMAN_MINI_GAME: u8 = 0xB;
    if matches!(b.setup.settings.panel_pattern, 0x38 | 0x30 | 0x3C) || battle_mode(b) == DUSTMAN_MINI_GAME {
        return;
    }
    ai_mut(b, r).status |= crate::actor::status::CAN_TURN;
}

/// `sub_802DFC8`: reset the side's extra state (set up only in the battle
/// flag 0x40 mode).
fn reset_side_state(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get(r);
    let (side, panel_x) = (o.alliance as usize, o.panel.x);
    let mode_40 = is_mode_40(b);
    let s = &mut b.sides[side];
    *s = Default::default();
    if mode_40 {
        // The game also sets bytes nothing ported reads (see
        // docs/engine/field-names.md, SideState).
        s.active = 1;
        s.panel_x = panel_x;
        // sub_802E07C
        s.select_special = 0;
    }
}

/// `sub_8013FF8`: the NaviCust starting-HP bug (stat 0x3D), which never
/// kills.
fn apply_starting_hp_bug(b: &mut Battle, r: ObjectRef) {
    let n = stats(b, r).bugs.starting_damage as u16;
    let hp = b.objects.get(r).hp;
    if n == 0 || hp == 1 {
        return;
    }
    let d = if hp > n - 1 { n } else { hp - 1 };
    crate::kinds::subtract_hp(b, r, d);
}

// ---- Update ------------------------------------------------------------------

/// `sub_80EA484`: the per-tick pipeline (§12.M M1).
fn tick(b: &mut Battle, r: ObjectRef) {
    input::update(b, r);
    emotion_timer(b, r);
    intake::collect_hits(b, r);
    status::update(b, r);
    per_form_tick(b, r);
    tick_cooldowns(b, r);
    full_synchro_effect(b, r);
    // sub_80100EC: palette (presentation).
    if !b.paused {
        b.present_collision(coll_id(b, r));
    }
}

/// `sub_8013DA0`: the NaviCust emotion timer (stats 0x24 and 0x21).
fn emotion_timer(b: &mut Battle, r: ObjectRef) {
    if b.paused {
        return;
    }
    let s = stats(b, r);
    if s.bugs.emotion != 0 && s.beast_out_counter != 0 {
        panic!("NaviCust emotion timer (sub_8013DA0) is not implemented yet");
    }
}

/// `off_80EA93C[AIIndex]`: the per-form tick hook (`sub_80F0608` for
/// MegaMan): per-chip charge counters for some forms, and the height
/// clamp.
fn per_form_tick(b: &mut Battle, r: ObjectRef) {
    if !matches!(ai(b, r).ai_index, 0 | 5) {
        return;
    }
    let s = *stats(b, r);
    if !b.paused && (s.navi == Navi(5) || matches!(s.form.0, 5 | 0x11)) {
        panic!("per-chip charge counters (sub_80F0608) are not implemented yet");
    }
    if s.navi == Navi::MEGAMAN {
        if s.form == Form::FALZAR_BEAST_OVER {
            b.objects.get_mut(r).pos.z = 0x14_0000;
        } else if b.objects.get(r).action != 0x50 && flag1(b, r) & f1::BUBBLED == 0 {
            b.objects.get_mut(r).pos.z = 0;
        }
    }
}

/// `sub_80107D4`: chip lockout and special cooldowns (not in time stop).
fn tick_cooldowns(b: &mut Battle, r: ObjectRef) {
    if b.is_time_stop() {
        return;
    }
    let a = ai_mut(b, r);
    a.lockout = a.lockout.saturating_sub(1);
    a.back_special_cooldown = a.back_special_cooldown.saturating_sub(1);
    // The battle flag 0x40 mode's per-side timers (`sub_802E070` +0x2E,
    // +0x3A, +0x3C) count down here too; nothing ported reads them.
}

/// `sub_80139C4`: the Full Synchro aura, spawned while the emotion is 2.
fn full_synchro_effect(b: &mut Battle, r: ObjectRef) {
    let a = ai(b, r);
    if b.objects.get(r).hp == 0 || a.actor_type != ActorType::Player || a.ai_index > 0xB {
        return;
    }
    if emotion(b, b.objects.get(r).alliance) == 2 && a.full_synchro_aura.is_none() {
        panic!("Full Synchro aura (sub_80C4C12) is not implemented yet");
    }
}

// ---- Destroy -------------------------------------------------------------------

/// `sub_8016C4E`: runs once. Players (`not_counted == 0`) stay allocated and
/// linked until the end of the round.
fn destroy(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init != 0 {
        return;
    }
    b.release_reservations(r);
    let c = coll_id(b, r);
    b.collision.free(c);
    // sub_800A104: the side has one fewer combatant.
    let side = b.objects.get(r).alliance as usize;
    let not_counted = ai(b, r).not_counted;
    if not_counted == 0 {
        b.round.actor_count[side] = b.round.actor_count[side].wrapping_sub(1);
    }
    b.objects.get_mut(r).phase_init = 4;
    if not_counted != 0 {
        let a = actor_id(b, r);
        b.actors.free(a);
        b.objects.free(r);
    }
}

/// `sub_801BCF4` / `object_updateSprite`: apply a requested animation and
/// step the sprite (not while paused, in time stop without flag 0x10, or
/// with `PreventAnim`).
pub(crate) fn update_sprite(b: &mut Battle, r: ObjectRef) {
    if b.paused {
        return;
    }
    let o = b.objects.get(r);
    if o.flags & flags::ACTIVE == 0 || o.flags & flags::NO_SPRITE_UPDATE != 0 {
        return;
    }
    if o.flags & flags::RUN_IN_TIME_STOP == 0 && b.is_time_stop() {
        return;
    }
    if o.collision.is_some() && o.prevent_anim != 0 {
        return;
    }
    let (anim, loaded) = (o.anim, o.anim_loaded);
    if anim != loaded {
        b.objects.sprite_mut(r).set_animation(anim);
        b.objects.get_mut(r).anim_loaded = anim;
    }
    b.objects.sprite_mut(r).update();
}
