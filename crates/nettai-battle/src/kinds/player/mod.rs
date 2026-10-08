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
mod navi_action;
pub(crate) mod chip_use;
pub use chip_use::{next_chip_bonus, next_chip_doubles};

/// `sub_8010740`: the opponent's Rush takes `chip` (a weapon's).
pub(crate) fn rush_cancels(b: &mut Battle, r: ObjectRef, chip: nettai_content_api::ChipHandle) -> bool {
    idle::rush_intercepts(b, r, Some(chip))
}

/// `loc_80126EA`: `chip` as the navi's attack (a weapon that fires a
/// chip).
/// `sub_800A772`: the navi's side's chips are enabled and its lockout over.
pub(crate) fn chips_enabled(b: &Battle, r: ObjectRef) -> bool {
    input::chips_enabled(b, r)
}

/// `sub_8010332`: the navi's move lag.
pub(crate) fn move_lag(b: &Battle, r: ObjectRef) -> u16 {
    idle::move_lag(b, r)
}

/// EXE5's 0x081042E6: a navi's status visuals forgotten (`sub_801A5E2`)
/// and its chips off the HUD (`sub_801DC36`, BattleObject +0x1A, +0x2A).
pub(crate) fn drop_links(b: &mut Battle, r: ObjectRef) {
    let c = coll_mut(b, r);
    c.links[crate::collision::link::CONFUSE] = None;
    c.links[crate::collision::link::BLIND] = None;
    let o = b.objects.get_mut(r);
    o.chips_held = 0;
    o.chip = None;
}

/// EXE5's 0x08104306: a navi no player controls leaves: no HP, the side's
/// damage-carry record forgets it (`sub_802CDD0`), its reservation goes,
/// it leaves every slot of the alive lists (0x08006BC2) and its object
/// goes to its destroy state.
pub(crate) fn leave(b: &mut Battle, r: ObjectRef) {
    b.objects.get_mut(r).hp = 0;
    let side = b.objects.get(r).alliance as usize;
    if b.damage_carry[side].target == Some(r) {
        b.damage_carry[side].target = None;
    }
    let fp = b.objects.get(r).future_panel;
    b.unreserve_panel(r, fp.x, fp.y);
    for slot in b.round.alive_actors.iter_mut().flatten() {
        if *slot == Some(r) {
            *slot = None;
        }
    }
    let o = b.objects.get_mut(r);
    o.state = state::DESTROY;
    o.action = 0;
    o.phase = 0;
    o.phase_init = 0;
    // (The word store clears the navi's CurAction too: its first state's.)
    ai_mut(b, r).navi_action = NaviAction::Entry;
}

/// `sub_80117BA`: weapon `weapon`'s setup, and its action started in
/// `set_attack` slot `kind`.
pub(crate) fn start_weapon(b: &mut Battle, r: ObjectRef, weapon: WeaponHandle, kind: u8) {
    let action = idle::weapon_routine(b, r, weapon);
    set_attack(b, r, action, kind);
}

pub(crate) fn load_chip_attack(b: &mut Battle, r: ObjectRef, chip: nettai_content_api::ChipHandle) {
    chip_use::load_attack(b, r, Some(chip));
}
mod ai_navi;
mod entry;
pub(crate) mod form;
pub(crate) mod idle;
mod input;
mod intake;
mod reactions;
mod status;
pub(crate) use status::end_anger;

pub(crate) use reactions::passed;
pub(crate) use intake::strip_body_programs;
pub use intake::take_navi_bug;
pub(crate) use intake::take_status;

use crate::content::MoodHeld;
use nettai_content_api::IdentityHandle;
use crate::actor::{ActorData, ActorId, ActorType, request};
use crate::battle::{Battle, battle_flags};
use crate::collision::{CollisionData, CollisionId, f1, timer};
use crate::field::PanelType;
use crate::content::NaviRecord;
use nettai_content_api::{ChipHandle, WeaponHandle};
use crate::content::Content;

pub use navi_action::{EngineAction, NaviAction, NaviWord};
pub use ai_navi::Controller;
pub(crate) use ai_navi::spawn as spawn_ai_navi;
use crate::object::{ObjectRef, PanelPos, Vec3, flags, state};
use crate::content::ActorEntry;
use crate::content::{FormData, FormTraits, NaviData};
use crate::setup::{NaviStats, effects};

/// Panel center coordinates (`object_getCoordinatesForPanels`, which
/// takes the panel numbers as signed bytes).
pub fn panel_coordinates(x: u8, y: u8) -> (i32, i32) {
    ((x as i8 as i32 * 40 - 140) << 16, (y as i8 as i32 * 24 - 20) << 16)
}

/// The panel under a position (`sub_800E258`; the game divides toward
/// zero).
pub fn coordinates_to_panel(x: i32, y: i32) -> PanelPos {
    PanelPos { x: (((x >> 16) + 0xA0) / 0x28) as u8, y: (((y >> 16) + 0x20) / 0x18) as u8 }
}

/// Spawn a player navi where its stage places it (`sub_800753C`).
pub fn spawn(b: &mut Battle, entry: &ActorEntry) -> Option<ObjectRef> {
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::Player, Vec3::default(), [0; 4])?;
    let (x, y) = panel_coordinates(entry.x, entry.y);
    {
        let o = b.objects.get_mut(r);
        o.alliance = entry.side;
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
    let identity = b.navi(entry.side as usize).identity;
    b.objects.get_mut(r).identity = identity;
    // The actor record (`sub_80182B4`); MegaMan's is {0, Player, 0}.
    let rec = b.content.navi_record(identity);
    let ad = b.actors.get_mut(a);
    ad.actor_type = rec.actor_type;
    ad.ai_index = rec.ai_index;
    ad.identity = identity;
    Some(r)
}

/// The player's update (`sub_80EA460`): lifecycle state, then the sprite
/// step every tick.
pub fn update(b: &mut Battle, r: ObjectRef) {
    // A navi no player controls: the navi type's update (EXE5's 0x080F2228).
    if ai_navi::is_ai_navi(b, r) {
        return ai_navi::update(b, r);
    }
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

/// The form of the object's side.
pub(crate) fn form_of(b: &Battle, r: ObjectRef) -> &FormData {
    b.content.form(stats(b, r).form)
}

/// The object's side is in the base form (a link navi always is).
pub(crate) fn in_base_form(b: &Battle, r: ObjectRef) -> bool {
    form_of(b, r).base
}

/// The object's side's navi changes form: where the original asks whether
/// the navi is MegaMan.
pub(crate) fn is_megaman(b: &Battle, r: ObjectRef) -> bool {
    navi_of(b, r).changes_form()
}

/// The navi of the object's side.
pub(crate) fn navi_of(b: &Battle, r: ObjectRef) -> &NaviData {
    b.content.navi(stats(b, r).navi)
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
fn own_gauges(b: &Battle) -> bool {
    b.round.flags & battle_flags::OWN_GAUGES != 0
}

/// `GetBattleMode`.
pub(crate) fn battle_mode(b: &Battle) -> u8 {
    b.round.mode_copy
}

/// `sub_80107C0`: the hit modifier bodies inflict (3 in link battles).
fn body_hit_modifier(b: &Battle) -> u8 {
    if is_link(b) { 3 } else { 0 }
}

/// A navi's body's collision types: what it is (floating or not) and what
/// it reacts to.
fn body_types(b: &Battle, floating: bool) -> (nettai_content_api::CollisionHandle, nettai_content_api::CollisionHandle) {
    use crate::content::CollisionRole;
    let roles = b.roles();
    let body = if floating { CollisionRole::FloatingNavi } else { CollisionRole::Navi };
    (roles.collision(body), roles.collision(CollisionRole::NaviTarget))
}

/// `sub_801A082` for a navi's body: it becomes the floating body or the
/// plain one again.
fn reset_body_types(b: &mut Battle, r: ObjectRef, floating: bool, hit_mod: u8) {
    let (body, target) = body_types(b, floating);
    b.reset_collision_types(r, body, target, hit_mod);
}

/// `sub_800F2FC`: turn to face `target` (its panel column), unless it
/// stands in the navi's column; the sprite follows (`sub_800F2C6`).
pub(crate) fn face_toward(b: &mut Battle, r: ObjectRef, target: ObjectRef) {
    let tx = b.objects.get(target).panel.x as i32;
    let o = b.objects.get(r);
    let dx = tx - o.panel.x as i32;
    if dx == 0 {
        return;
    }
    let flip = (dx < 0) as u8 ^ o.alliance;
    b.objects.get_mut(r).flip = flip;
    let facing = b.objects.get(r).alliance ^ flip;
    b.objects.sprite_mut(r).look.set_flip(facing);
}

/// `object_getFlipDirection`: +1 facing right, -1 facing left.
fn flip_direction(alliance: u8, flip: u8) -> i32 {
    if alliance ^ flip == 0 { 1 } else { -1 }
}

/// `sub_80182B4`: the object's actor record.
fn navi_record(b: &Battle, r: ObjectRef) -> NaviRecord {
    b.content.navi_record(b.objects.get(r).identity)
}

/// `sub_8018810`: the object's sprite attach point `index`, in pixels,
/// facing the object's way.
pub(crate) fn attach_point(b: &Battle, r: ObjectRef, index: usize) -> (i32, i32) {
    let o = b.objects.get(r);
    name_attach_point(b, o.identity, index, o.alliance, o.flip)
}

/// `sub_8018810` as the game calls it: an identity's attach point
/// `index`, in pixels, facing the way `alliance` and `flip` say. Every
/// point of a field object is (0, 7).
pub(crate) fn name_attach_point(b: &Battle, identity: Option<IdentityHandle>, index: usize, alliance: u8, flip: u8) -> (i32, i32) {
    if b.content.identity(identity).class == crate::content::IdentityClass::FieldObject {
        return (0, 7);
    }
    let p = b.content.attach_point(identity, index);
    (p.x as i32 * flip_direction(alliance, flip), p.y as i32)
}

/// The panel type under (x, y); off the field the game reads BIOS memory,
/// taken here as "no panel".
fn panel_kind(b: &Battle, p: PanelPos) -> PanelType {
    b.field.panel(p.x, p.y).map(|p| p.kind).unwrap_or_default()
}

/// `sub_8010004`: the next chip in the side's hand (none: the game's
/// 0xFFFF).
pub(crate) fn next_chip(b: &Battle, r: ObjectRef) -> Option<ChipHandle> {
    let hand = &b.hands[b.objects.get(r).alliance as usize];
    hand.ids.get(hand.cursor as usize).copied().flatten()
}

/// Whether hand chip `id` is of the game's non-elemental family (the
/// rules' `elements.non_elemental`). The game looks an empty hand's chip
/// (0xFFFF) up in the chip table too, reading the record past its end
/// (`Rules::empty_hand`).
fn null_family(b: &Battle, id: Option<ChipHandle>) -> bool {
    let Some(id) = id else { return b.game_rules().empty_hand.null_family };
    b.content.chip(id).family == b.game_rules().chip_families.non_elemental
}

pub use crate::content::{Emotion, EmotionRole};

/// `sub_8015B54` (EXE5's 0x0801270C): a side's emotion, the first of its
/// game's order (the status section's `emotion`) that holds of its navi:
/// its mood, its anger, its held tired and exhausted states, whether it is
/// out of its base form, and the battle's mode.
pub fn emotion(b: &Battle, side: u8) -> Emotion {
    let p = b.player(side).expect("side has a player");
    let a = ai(b, p);
    let facts = crate::content::EmotionFacts {
        mood: b.stats[side as usize].mood,
        angry: a.anger != 0,
        tired: a.tired,
        exhausted: a.exhausted,
        in_form: !form_of(b, p).base,
        battle_mode: battle_mode(b),
    };
    b.game_rules().emotion.of(&facts)
}

/// What side `side`'s emotion is to the framework.
pub fn emotion_role(b: &Battle, side: u8) -> Option<EmotionRole> {
    b.game_rules().emotion.role(emotion(b, side))
}

/// Side `side`'s emotion's name (its game's).
pub fn emotion_name(b: &Battle, side: u8) -> &str {
    b.game_rules().emotion.name(emotion(b, side))
}

/// A loss of HP brought `r` to 0 (EXE5's `object_subtractHP` calls
/// 0x0802C16C): its side's rules are asked (`hp_emptied`: EXE5's last
/// stand, which may hold it at 1 HP), an object with actor data's; whether
/// the register r1 the callers read next is left non-zero (EXE5's
/// `applyDamageToPlayer` takes it for HP left, and shows the hit). An
/// object without actor data is no navi: not asked, r1 left 0.
pub(super) fn hp_emptied(b: &mut Battle, r: ObjectRef) -> bool {
    if b.objects.get(r).actor.is_none() {
        return false;
    }
    let side = b.objects.get(r).alliance;
    b.rules_hp_emptied(side, r)
}

/// EXE5's 0x0800C734: what a player's loss drains of its side's gauge in
/// the own-gauges mode, by its size.
fn gauge_loss(amount: u16) -> u32 {
    match amount {
        0..=9 => 0,
        10..=90 => 0x555,
        91..=299 => 0xAAA,
        _ => 0x2000,
    }
}

/// EXE5's `object_subtractHP` (0x0800C6E0): a player's loss first drains
/// its side's gauge (0x0802D4C0: by the loss ×128, in the own-gauges mode
/// by `gauge_loss`), then the HP goes down, to 0, where the side's rules
/// are asked (`hp_emptied`). Whether r1 is left non-zero
/// (`kinds::subtract_hp`).
pub(crate) fn lose_hp_and_gauge(b: &mut Battle, r: ObjectRef, amount: u16) -> bool {
    let player = b.objects.get(r).actor.is_some_and(|id| b.actors.get(id).actor_type == ActorType::Player);
    if player {
        let drain = if own_gauges(b) { gauge_loss(amount) } else { (amount as u32) << 7 };
        let side = b.objects.get(r).alliance as usize & 1;
        let s = &mut b.sides[side];
        s.gauge = (s.gauge as u32).saturating_sub(drain) as u16;
    }
    let o = b.objects.get_mut(r);
    o.hp = o.hp.saturating_sub(amount);
    if o.hp != 0 {
        return true;
    }
    hp_emptied(b, r)
}

/// Presentation: whether side `side`'s emotion window shows its form's
/// second set of faces, as the side's rules ask: while its navi's rules' B
/// charge is armed, whatever its form (`face_charged`), or in the base form
/// (`base_face_variant`).
pub fn shows_face_variant(b: &Battle, side: u8) -> bool {
    face_charged(b, side) || base_face_variant(b, side)
}

/// Presentation: the second set for the side's base form when its rules
/// ask (`SideLooks::face_variant`: EXE5's Hub Style, 0x0801AF8E's picture 11
/// on, which a soul's face doesn't take). Part of the picture as it is
/// picked.
pub fn base_face_variant(b: &Battle, side: u8) -> bool {
    let Some(p) = b.player(side) else { return false };
    b.looks[side as usize & 1].face_variant && form_of(b, p).base
}

/// Presentation: the second set while the side's navi's rules' B charge is
/// armed, where its rules ask (`SideLooks::face_variant_charged`: EXE5's
/// 0x080125F6, AIData +0x12, the face's palette 11 on, a soul's Chaos
/// Unison look), whichever picture the window shows: the original reads it
/// as it draws (0x08019704).
pub fn face_charged(b: &Battle, side: u8) -> bool {
    b.looks[side as usize & 1].face_variant_charged && b.player(side).is_some_and(|p| ai(b, p).b_charge_time.is_some())
}

/// Whether a navi's mood is held (`sub_8015BEC`'s test): held tired or
/// exhausted. Another side's rules read it (`sub_801A200`'s counter).
pub(crate) fn mood_held(b: &Battle, r: ObjectRef) -> bool {
    let a = ai(b, r);
    a.tired || a.exhausted
}

/// Whether a side's mood is held against the setter, by the game's rule
/// (the status section's `emotion.mood_held`): its navi held tired or
/// exhausted (`sub_8015BEC`'s test), or a mood of 0 (EXE5's 0x080127D6).
pub(crate) fn mood_is_held(b: &Battle, side: u8) -> bool {
    match b.game_rules().emotion.mood_held {
        MoodHeld::TiredOrExhausted => b.player(side).is_some_and(|p| mood_held(b, p)),
        MoodHeld::AtZero => b.stats[side as usize & 1].mood == 0,
    }
}

/// `sub_8015BEC` (EXE5's 0x080127D6): set a side's mood, unless it is held
/// ([`mood_is_held`]).
pub(crate) fn set_mood(b: &mut Battle, side: u8, mood: u8) {
    if b.player(side).is_none() || mood_is_held(b, side) {
        return;
    }
    b.stats[side as usize].mood = mood;
}

/// EXE5's 0x08012802: a side's mood rises by `n`, to 254 at most; a mood
/// of 0 or 0xFF (Full Synchro) stays.
pub(crate) fn gain_mood(b: &mut Battle, side: u8, n: u16) {
    let s = &mut b.stats[side as usize & 1];
    if s.mood != 0 && s.mood != 0xFF {
        s.mood = (s.mood as u32 + n as u32).min(254) as u8;
    }
}

/// EXE4's 0x0800F4FA: a side's mood rises by `n`, to 0xFF (Full Synchro)
/// at most; a mood of 0 stays.
pub(crate) fn raise_mood(b: &mut Battle, side: u8, n: u16) {
    let s = &mut b.stats[side as usize & 1];
    if s.mood != 0 {
        s.mood = (s.mood as u32 + n as u32).min(0xFF) as u8;
    }
}

/// `sub_8015C12` (EXE5's 0x08012820): a side's mood falls by `n`, to 1 at
/// least; a mood of 0 stays.
pub(crate) fn lose_mood(b: &mut Battle, side: u8, n: u16) {
    let s = &mut b.stats[side as usize & 1];
    if s.mood != 0 {
        s.mood = (s.mood as i32 - n as i32).max(1) as u8;
    }
}

/// Save the navi's lifecycle position (`obj+0x5C`) unless one is saved.
fn save_state_word(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).saved_word.is_some() {
        return;
    }
    let o = b.objects.get(r);
    let word = NaviWord { state: o.state, action: ai(b, r).navi_action, phase: o.phase, phase_init: o.phase_init };
    ai_mut(b, r).saved_word = Some(word);
}

/// What navi `r` runs.
pub fn navi_action(b: &Battle, r: ObjectRef) -> NaviAction {
    ai(b, r).navi_action
}

/// Set what navi `r` runs, its phase untouched (the game's byte stores).
pub fn set_navi_action(b: &mut Battle, r: ObjectRef, action: NaviAction) {
    ai_mut(b, r).navi_action = action;
}

/// The action of `role` (a role content hasn't filled is a panic naming
/// it).
pub(crate) fn role_action(b: &Battle, role: crate::content::ActionRole) -> NaviAction {
    NaviAction::Content(b.roles().action(role))
}

/// Whether navi `r` runs the action of `role`.
pub(crate) fn runs_role(b: &Battle, r: ObjectRef, role: crate::content::ActionRole) -> bool {
    matches!(navi_action(b, r), NaviAction::Content(h) if b.roles().is_action(role, h))
}

/// `sub_802DD2A`: a switched-in navi (the navi switch) that falls back
/// instead of dying.
fn switch_protected(b: &Battle, r: ObjectRef) -> bool {
    !is_megaman(b, r) && ai(b, r).status & crate::actor::status::SWITCHED != 0
}

/// Switch to `action` at phase 0 (the game's direct CurAction stores).
pub(crate) fn set_action(b: &mut Battle, r: ObjectRef, action: NaviAction) {
    set_navi_action(b, r, action);
    let o = b.objects.get_mut(r);
    o.phase = 0;
    o.phase_init = 0;
}

/// `object_setAttack0..5`: start `action`, recording which helper started
/// it in the attack variables (§M2.2).
pub(crate) fn set_attack(b: &mut Battle, r: ObjectRef, action: impl Into<NaviAction>, kind: u8) {
    set_action(b, r, action.into());
    let a = &mut ai_mut(b, r).attack;
    a.step = 0;
    a.step_init = 0;
    a.kind = kind;
    reset_attack_links(b, r);
}


/// The content action the navi `r` is running. None for an object that
/// isn't an actor.
pub fn running_content_action(b: &Battle, r: ObjectRef) -> Option<nettai_content_api::ActionHandle> {
    let id = b.objects.get(r).actor?;
    match b.actors.get(id).navi_action {
        NaviAction::Content(h) => Some(h),
        _ => None,
    }
}

impl From<EngineAction> for NaviAction {
    fn from(e: EngineAction) -> NaviAction {
        NaviAction::Engine(e)
    }
}

/// `sub_801011A`: clear the attack's link bytes and unfreeze the Beast
/// Out lock-on marker (`sub_80E1662`; a no-op without one).
pub(crate) fn reset_attack_links(b: &mut Battle, r: ObjectRef) {
    let a = ai_mut(b, r);
    a.attack.wrapped = 0;
    a.attack.wrapper_fresh = true;
    if let Some(marker) = a.target_marker {
        crate::kinds::target_marker::unfreeze(b, marker);
    }
}

/// `object_exitAttackState`: back to the idle action with animation 0.
pub(crate) fn exit_attack_state(b: &mut Battle, r: ObjectRef) {
    b.objects.get_mut(r).anim = 0;
    end_attack(b, r);
}

/// The same, keeping the lockouts as they are (EXE4's 0x0800CA28, the
/// buster's and the charged shot's recovery's end).
pub(crate) fn exit_attack_state_keeping_lockout(b: &mut Battle, r: ObjectRef) {
    b.objects.get_mut(r).anim = 0;
    end_attack_with(b, r, false);
}

/// `sub_801171C`: leave the current attack for the idle action. A move
/// (kind 4) keeps pending requests and the charge.
pub(crate) fn end_attack(b: &mut Battle, r: ObjectRef) {
    end_attack_with(b, r, true);
}

/// [`end_attack`], its lockout handed on by the rules (`hands_on`) or not.
fn end_attack_with(b: &mut Battle, r: ObjectRef, hands_on: bool) {
    use crate::content::AttackEndLockout;
    // What else the end clears of the requests is its game's (EXE6's
    // 0x1000003F, EXE5's 0x1803F: the reactions section's).
    let clears = b.game_rules().request_clears.attack.0;
    let rule = b.game_rules().attack_end_lockout;
    let a = ai_mut(b, r);
    a.attack.special_source = 0;
    let kind = a.attack.kind;
    if kind != 4 {
        match (rule, kind) {
            (_, _) if !hands_on => {}
            (AttackEndLockout::ChipLockout, _) | (AttackEndLockout::ByKind, 2) => a.lockout = a.attack.lockout,
            (AttackEndLockout::ByKind, 3) => a.back_special_cooldown = a.attack.lockout,
            // (EXE5's 0x0800F2D0 drops its Chaos Unison charge at the end
            // of its failure, slot 6: the failure's revert dropped it with
            // its status reset already, rules/souls/chaos.)
            _ => {}
        }
        a.buffered_move = 0;
        a.requests &= !(request::ATTACKS | clears);
        reset_charge(b, r);
        clear_flag1(b, r, f1::USING_ACTION);
    }
    set_navi_action(b, r, NaviAction::Idle);
    let a = ai_mut(b, r);
    a.attack.step = 0;
    a.attack.step_init = 0;
}

/// `sub_8012EA8`: drop the buster charge and the hold flags.
pub(crate) fn reset_charge(b: &mut Battle, r: ObjectRef) {
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

/// `object_setInvulnerableTime`: invulnerable for `ticks` (0xFFFF: for
/// good).
fn set_invulnerable(b: &mut Battle, r: ObjectRef, ticks: u16) {
    coll_mut(b, r).status_timers[timer::INVULNERABLE] = ticks;
    set_flag1(b, r, f1::INVULNERABLE);
}

/// `sub_801A264`: the statuses end: their flags, requests and timers, the
/// game's (its reactions' `status_end`: EXE4's 0x08013218 has no freeze or
/// bubble).
pub(crate) fn clear_statuses(b: &mut Battle, r: ObjectRef) {
    let end = b.game_rules().status_end;
    clear_flag1(b, r, end.flags.0);
    clear_flag2(b, r, end.requests);
    let c = coll_mut(b, r);
    for (t, timer) in c.status_timers.iter_mut().enumerate() {
        if end.timers.0 & (1 << t) != 0 {
            *timer = 0;
        }
    }
}

/// `sub_800EB08`: end invulnerability.
pub(crate) fn clear_invulnerable(b: &mut Battle, r: ObjectRef) {
    coll_mut(b, r).status_timers[timer::INVULNERABLE] = 0;
    clear_flag1(b, r, f1::INVULNERABLE);
}

/// `sub_80101C4`: end the timed submerged state.
fn cancel_submerged(b: &mut Battle, r: ObjectRef) {
    coll_mut(b, r).status_timers[timer::SUBMERGED] = 0;
    clear_flag1(b, r, f1::SUBMERGED);
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
pub(crate) fn snap_to_future_panel(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    o.panel = o.future_panel;
    let p = o.panel;
    b.unreserve_panel(r, p.x, p.y);
    set_coordinates_from_panel(b, r);
    b.update_collision_panels(r);
}

/// `object_setCoordinatesFromPanels`.
pub(crate) fn set_coordinates_from_panel(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    let (x, y) = panel_coordinates(o.panel.x, o.panel.y);
    o.pos.x = x;
    o.pos.y = y;
}

/// `sub_8011450`: restart the form overlay (`related[1]`) with the navi
/// after an animation change.
pub(crate) fn refresh_form_overlay(b: &mut Battle, r: ObjectRef) {
    let a = ai(b, r);
    if a.actor_type == ActorType::Virus {
        return;
    }
    // `off_8011470` by the actor's record (a navi's: MegaMan's stays his
    // own in every form): what its identity's `refresh` hook does, by
    // what the identity wears.
    let identity = b.content.identity(a.identity);
    if !identity.overlay_hooks.refresh {
        return;
    }
    let overlay = b.objects.get(r).related[1];
    match identity.parts {
        // sub_80FF668: both overlays.
        Some(crate::content::Parts::Bodies(..)) => {
            let second = b.objects.get(r).second_overlay;
            for o in [overlay, second].into_iter().flatten() {
                crate::kinds::form_overlay::restart(b, o);
            }
        }
        // sub_80C46B6: the animation reloads at its next step.
        Some(crate::content::Parts::BeastHead { .. }) => {
            if let Some(o) = overlay {
                b.objects.get_mut(o).anim_loaded = 0xFF;
            }
        }
        // sub_80C44D2 (MegaMan's restarts what his form wears).
        _ => {
            if let Some(o) = overlay {
                restart_overlay(b, o);
            }
        }
    }
}

/// `sub_80C44D2`: restart `overlay`, what the navi `r` wears, by its game's
/// rules: EXE6's steps it at once, EXE5's (0x080C374E) has it reload its
/// animation at its next step.
pub(crate) fn restart_overlay(b: &mut Battle, overlay: ObjectRef) {
    match b.game_rules().overlay_restart {
        crate::content::OverlayRestart::Step => crate::kinds::form_overlay::restart(b, overlay),
        crate::content::OverlayRestart::Reload => b.objects.get_mut(overlay).anim_loaded = 0xFF,
    }
}

/// `sub_80127C0(0)`: fill the attack variables for the next chip (for
/// weapon routines that use the chip, such as SlashCross's A-charge, which
/// take its action from `attack_chip`).
pub(crate) fn prepare_chip(b: &mut Battle, r: ObjectRef) {
    chip_use::prepare(b, r, 0);
}

// ---- The transformation sequencer's checks -------------------------------------

// (`sub_80159C6`, the turn-start check that a Beast Out whose counter ran
// out reverts, is EXE6's rules/beast's `turn_check`: content/exe6/rules/
// beast/init.luau.)

/// `sub_80159A2`: a form reversion is pending or running.
pub fn reverting_form(b: &Battle, r: ObjectRef) -> bool {
    ai(b, r).status & crate::actor::status::REVERTING_FORM != 0 || ai(b, r).requests & request::REVERT_FORM != 0
}

/// `sub_801596E`: ask the navi to change form (it does so in the pause
/// handler, as action 0x1C).
pub fn request_form_change(b: &mut Battle, r: ObjectRef) {
    ai_mut(b, r).requests |= request::FORM_CHANGE;
}

/// The form `r`'s side asked to change into at this turn's start (none:
/// none, or the base form).
pub(crate) fn form_change_target(b: &Battle, r: ObjectRef) -> Option<nettai_content_api::FormHandle> {
    let side = b.objects.get(r).alliance as usize;
    b.turn_transforms[side].form.filter(|&f| !b.content.form(f).base)
}

/// `sub_801597C`: a form change is running.
pub fn changing_form(b: &Battle, r: ObjectRef) -> bool {
    ai(b, r).status & crate::actor::status::FORM_CHANGE != 0
}

/// `sub_802DCEC`: a navi switch is pending or running.
pub fn switching_navi(b: &Battle, r: ObjectRef) -> bool {
    ai(b, r).status & crate::actor::status::SWITCHING_NAVI != 0 || ai(b, r).requests & request::NAVI_SWITCH != 0
}

// ---- Init --------------------------------------------------------------------

/// `sub_80172F0`: set up a freshly spawned player (§12.2).
fn init(b: &mut Battle, r: ObjectRef) {
    // sub_800F35C: the per-form init hook is a no-op for every form.
    let form = stats(b, r).starting_form;
    stats_mut(b, r).form = form;
    load_sprite(b, r);
    // sub_801002C, sprite_setPalette.
    let palette = palette_pick(b, r);
    b.objects.sprite_mut(r).look.palette = palette;
    // sub_80142B0: bodies deal 10 in link battles (the rules'
    // `link_body_damage`: EXE6's and EXE5's; EXE4 has none).
    if is_link(b)
        && let Some(d) = b.game_rules().link_body_damage
    {
        b.objects.get_mut(r).damage = d.damage;
    }
    if b.create_collision(r).is_none() {
        b.objects.free(r);
        return;
    }
    let hm = body_hit_modifier(b);
    let (body, target) = body_types(b, false);
    b.setup_collision(r, body, target, hm);
    init_hp(b, r);
    init_round_state(b, r);
    update_element(b, r);
    let s = *stats(b, r);
    if is_megaman(b, r) {
        // sub_8015B22
        b.objects.get_mut(r).identity = b.content.form_identity(s.navi, s.form);
    }
    // sub_8011268: the starting form's overlay. A base form's is its navi's
    // init hook's, below (EXE6's base form wears nothing; EXE5's init has no
    // call here, its base form's routine being MegaMan's record's hook).
    if !b.content.form(s.form).base {
        form::put_on_overlay(b, r, s.form);
    }
    reset_status(b, r);
    style_hook(b, r);
    // sub_801DB84, sub_8018856, sub_801DC06, sub_801DC36: the HP number
    // HUD table.
    enable_turning(b, r);
    if b.content.rules().effects.charge_glow == crate::content::ChargeGlow::WithCharge {
        // (EXE4's init spawns none: a charge brings its own.)
    } else if stats(b, r).first_barrier.is_some() {
        // sub_8013892's `pop {r4}` left the barrier type in r4, so the glow's
        // link slot (r4 + 0x58) is a BIOS address (docs/engine/dimming-
        // chips.md §3.4).
        crate::kinds::charge_glow::spawn_unlinked(b, r);
    } else {
        crate::kinds::charge_glow::spawn(b, r);
    }
    post_init_hook(b, r);
    if in_base_form(b, r) {
        let identity = b.objects.get(r).identity;
        form::base_init_hook(b, r, identity);
    }
    reset_side_state(b, r);
    apply_starting_hp_bug(b, r);
    b.objects.get_mut(r).state = state::UPDATE;
    set_action(b, r, NaviAction::Entry);
}

/// `sub_800F378`: the post-init hook by actor type and AI index. For
/// players (`off_80EAA04`; EXE5's 0x080EB2A8) it is the navi's
/// `post_init`: every entry is empty but EXE6's DustMan's (`sub_80F22F8`),
/// EXE5's MegaMan's (0x080F04EE: in a soul, the status reset again) and
/// EXE5's ToadMan's (0x080F199C). Viruses' and AI navis' hooks
/// (`off_81092D0`, `off_80F2668`) belong to their AI.
fn post_init_hook(b: &mut Battle, r: ObjectRef) {
    match ai(b, r).actor_type {
        ActorType::Player => {}
        t => panic!("the post-init hooks of {t:?} actors (sub_800F378) belong to the virus and navi AI"),
    }
    let navi = stats(b, r).navi;
    if let Some(f) = b.content.defs.navi(navi).post_init {
        crate::behavior::call_hook(b, f, nettai_content_api::HookCall::FormNavi { navi: r });
    }
}

/// `sub_80F22F8`'s spawns (EXE6's DustMan's post-init hook in battle mode
/// 9): two objects the navi keeps, the roles' `mode9_attack` (attack
/// #0xD2, on the same side, running while dimmed) and `mode9_actor` (actor
/// #0x28).
pub(crate) fn spawn_mode9_objects(b: &mut Battle, r: ObjectRef) {
    // sub_80DFD74 (at 0, 0, 0) and sub_80C02A6 (at the registers the
    // first spawn left: garbage nothing is known to read).
    let (alliance, flip) = {
        let o = b.objects.get(r);
        (o.alliance, o.flip)
    };
    let kind = b.roles().kind(crate::content::KindRole::Mode9Attack);
    let junk = crate::kinds::spawn(b, kind, nettai_content_api::SpawnAt::AfterCurrent, Vec3::default(), [0; 4]);
    if let Some(j) = junk {
        let o = b.objects.get_mut(j);
        o.related[0] = Some(r);
        o.alliance = alliance;
        o.flip = flip;
        o.element = 0;
        o.flags |= flags::RUN_WHILE_DIMMED;
    }
    let kind = b.roles().kind(crate::content::KindRole::Mode9Actor);
    let second = crate::kinds::spawn(b, kind, nettai_content_api::SpawnAt::AfterCurrent, Vec3::default(), [0; 4]);
    if let Some(s) = second {
        let o = b.objects.get_mut(s);
        o.related[0] = Some(r);
        o.alliance = alliance;
        o.flip = flip;
    }
    ai_mut(b, r).mode9_objects = [junk, second];
}

/// `sub_800FC9E`: a side's navi's battle sprite by its stats (MegaMan's by
/// his form, another navi's his own).
pub(crate) fn stats_sprite(b: &Battle, side: u8) -> crate::content::SpriteId {
    let s = &b.stats[side as usize & 1];
    b.content.navi_sprite(s.navi, s.form)
}

/// `sub_800FC9E` + `sprite_load`: load the navi's battle sprite.
fn load_sprite(b: &mut Battle, r: ObjectRef) {
    let id = stats_sprite(b, b.objects.get(r).alliance);
    let flip = b.objects.get(r).alliance ^ b.objects.get(r).flip;
    let sprite = b.objects.sprite_mut(r);
    sprite.load(id);
    sprite.set_animation(0, &b.content);
    // sprite_hasShadow; sprite_setFlip(object_getFlip()).
    sprite.look.shadow = crate::object::sprite::Shadow::Ground;
    sprite.look.set_flip(flip);
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
fn init_round_state(b: &mut Battle, r: ObjectRef) {
    b.objects.get_mut(r).stamina = 10;
    let eff = b.setup.settings.effects;
    if eff & effects::LINK != 0 || eff & 0x1_0000 != 0 || stats(b, r).mood != 0xFF {
        // sub_8015C2C: the starting mood (the side's rules may say
        // another: EXE5's rules/light_dark's, by the light/dark value,
        // 0x0801283A).
        let side = b.objects.get(r).alliance;
        let mood = b.rules_starting_mood(side).unwrap_or(0x80);
        stats_mut(b, r).mood = mood;
    }
    if stats(b, r).first_barrier.is_some() {
        // sub_801A7CC(stat 6) and the barrier's visual (sub_80E0D98): the
        // content's FirstBarrier (the role hooks.first_barrier). The game's
        // FirstBarrier program sets the stat to 1, the Barrier chip's
        // barrier; the role raises that one. (The `pop {r4}` after it
        // clobbers the AIData pointer the charge glow's spawn uses: `init`
        // spawns the glow unlinked.)
        let hook = b.roles().hook(crate::content::HookRole::FirstBarrier);
        crate::behavior::call_hook(b, hook, nettai_content_api::HookCall::RoleNavi { navi: r });
    }
    // The move bug's confusion at the start (EXE4's 0x0800D8B0:
    // `effects.steps.bug`).
    if let Some(bug) = &b.game_rules().effects.steps.bug
        && idle::move_bug(b, r) == 0xFF
    {
        let ticks = bug.confused;
        coll_mut(b, r).status_timers[timer::CONFUSE] = ticks;
    }
    // (EXE6's rules/emotion holds a navi whose Beast Out counter is spent
    // tired from the round's start.)
    reset_abilities(b, r);
}

/// `sub_801390C`: weapon bytes, invulnerability and the NaviCust flags.
fn reset_abilities(b: &mut Battle, r: ObjectRef) {
    let w = stats(b, r).weapons;
    let a = ai_mut(b, r);
    a.charge_shot = w.charge_shot;
    a.back_special = w.back_special;
    // off_8013CA8 is 0x08000001: a guard left up (an action the custom
    // screen's form change cut short) comes down with it.
    clear_flag1(b, r, f1::UNTOUCHABLE | f1::GUARD);
    // What the navi's stats raise again (EXE4's All Guard: the role hook
    // `abilities_reset`).
    if let Some(hook) = b.roles().try_hook(crate::content::HookRole::AbilitiesReset) {
        crate::behavior::call_hook(b, hook, nettai_content_api::HookCall::RoleNavi { navi: r });
    }
    clear_invulnerable(b, r);
    // sub_80E5410: the linked object's state word becomes 8 (it frees
    // itself at its next update) and its first extra variable 0, and the
    // link goes. (EXE4's WindSoul's wind links itself here; its first
    // extra variable, its gusts' slots, its own end empties as it runs, the
    // first of what reads them: so that variable has no field.)
    if let Some(o) = ai_mut(b, r).reset_linked_object.take() {
        crate::kinds::common::set_progress(b, o, crate::kinds::common::Progress::DESTROY);
    }
    apply_ability_flags(b, r);
}

/// `sub_801393A`: refresh the weapon bytes (base form) and the NaviCust
/// flags after a NaviCust change.
fn refresh_abilities(b: &mut Battle, r: ObjectRef) {
    let s = *stats(b, r);
    if in_base_form(b, r) {
        let a = ai_mut(b, r);
        a.charge_shot = s.weapons.charge_shot;
        a.back_special = s.weapons.back_special;
    }
    apply_ability_flags(b, r);
}

/// `loc_8013956`: FloatShoe (also changes what the body is), AirShoe,
/// ice, Undershirt and SuperArmor from the navi stats.
fn apply_ability_flags(b: &mut Battle, r: ObjectRef) {
    let s = *stats(b, r);
    let hm = body_hit_modifier(b);
    if s.float_shoes {
        set_flag1(b, r, f1::FLOATSHOE);
        reset_body_types(b, r, true, hm);
    } else {
        clear_flag1(b, r, f1::FLOATSHOE);
        reset_body_types(b, r, false, hm);
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
    let base = in_base_form(b, r);
    let element = if !is_megaman(b, r) {
        b.content.navi(s.navi).element as u8
    } else if !base {
        b.content.form(s.form).element as u8
    } else {
        s.element
    };
    set_element(b, r, element);
    let weakness = if !base { b.content.form(s.form).weakness } else { b.content.navi(s.navi).weakness };
    coll_mut(b, r).secondary_weakness = weakness.0;
}

/// `sub_80144C0`: the full status reset (NaviCust state, hand bonuses,
/// hit modifier, region, charge, weapon bytes, element, body damage).
pub(crate) fn reset_status(b: &mut Battle, r: ObjectRef) {
    reset_abilities(b, r);
    reset_status_tail(b, r, true);
}

/// `sub_80144CA` (`sub_80144C0` past its NaviCust reset): hand bonuses,
/// hit modifier, region, charge, weapon bytes (only from `sub_80144C0`),
/// form flags, element, body damage.
fn reset_status_tail(b: &mut Battle, r: ObjectRef, reload_weapons: bool) {
    let side = b.objects.get(r).alliance as usize;
    b.hands[side].charge_bonus = [0; 6];
    // EXE5's (0x08011B3C, 0x080CAC30) disarms the side's ColonelSoul army
    // (`obstacle::Conversion`; the form's reset below arms it again): EXE6
    // has none to disarm.
    b.obstacle_conversion[side & 1].armed = false;
    ai_mut(b, r).status &= !0x20;
    // (Netbattle, local player: removes the opponent's HUD entry: its chip
    // icons, which EXE5's SearchSoul's reset below puts back.)
    b.chip_hud[side & 1].opponent = false;
    // EXE5's 0x08011B74: a form's priming is spent (EXE6 never primes).
    ai_mut(b, r).primed = false;
    let hm = body_hit_modifier(b);
    let anchor = b.anchor_region();
    let c = coll_mut(b, r);
    c.hit_mod_base = hm;
    c.region = anchor;
    reset_charge(b, r);
    if reload_weapons {
        load_weapons(b, r);
    }
    form::apply_form_flags(b, r);
    update_element(b, r);
    // sub_80142C2: the body's damage in a link battle again (the rules'
    // `link_body_damage`, EXE6's).
    if is_link(b)
        && let Some(d) = b.game_rules().link_body_damage
        && d.again_at_reset
    {
        coll_mut(b, r).self_damage = d.damage;
    }
}

/// `sub_800FEEC`: the navi's weapons, from the navi stats (base form) or
/// the form's own.
fn load_weapons(b: &mut Battle, r: ObjectRef) {
    let s = *stats(b, r);
    let mode9 = battle_mode(b) == 9;
    let content = b.content.clone();
    let base = in_base_form(b, r);
    let a = ai_mut(b, r);
    if base {
        let w = s.weapons;
        a.mode9_a = if mode9 { w.mode9_a } else { None };
        a.buster = w.buster;
        set_charge_shot_routine(a, w.charge_shot, &content);
        a.a_charge = w.a_charge;
        a.back_special = w.back_special;
        a.alt_a_charge = None;
    } else {
        let w = content.form(s.form).weapons;
        a.mode9_a = w.mode9_a;
        a.a_charge = w.a_charge;
        a.buster = w.buster;
        set_charge_shot_routine(a, w.charge_shot, &content);
        a.back_special = w.back_special;
        a.alt_a_charge = w.alt_a_charge;
    }
    // The load drops the rules' own B charge (EXE5's 0x0800DCD8 disarms its
    // Chaos Unison charge).
    (a.b_charge_time, a.b_charge_glow, a.b_charge_anim) = (None, None, None);
}

/// `sub_800FF5E`: reload the base form's weapon bytes (after a NaviCust
/// change).
fn reload_base_weapons(b: &mut Battle, r: ObjectRef) {
    if in_base_form(b, r) {
        load_weapons(b, r);
    }
}

/// `sub_800FFAA`: set the charged shot. A sticky one (a chip's weapon)
/// stays unless the new one is sticky too, and while the charged shot is
/// sticky a buster with a plain counterpart (the Beast busters, the Beast
/// form's throw) gives way to it.
fn set_charge_shot_routine(a: &mut ActorData, v: Option<WeaponHandle>, content: &Content) {
    let sticky = |w: Option<WeaponHandle>| w.is_some_and(|w| content.weapon(w).sticky);
    if sticky(v) || !sticky(a.charge_shot) {
        a.charge_shot = v;
    }
    if sticky(a.charge_shot)
        && let Some(plain) = a.buster.and_then(|w| content.weapon(w).plain)
    {
        a.buster = Some(plain);
    }
}

/// `sub_8013E58`: the NaviCust battle-start bug (stat 0x1A,
/// `off_8013E9C`): for 300 ticks (variants 1..=4) or 600 (5..=8) the navi
/// starts invisible, invulnerable, blind or confused; 9 and 10 roll one of
/// the four.
fn style_hook(b: &mut Battle, r: ObjectRef) {
    let s = stats(b, r).bugs.battle_start;
    let variant = match s {
        9 => (b.rng.next() & 3) as u8 + 1,
        10 => (b.rng.next() & 3) as u8 + 5,
        _ => s,
    };
    let ticks: u16 = match variant {
        0 => return,
        1..=4 => 300,
        5..=8 => 600,
        _ => panic!("NaviCust battle-start bug {variant} reads past its table (off_8013E9C)"),
    };
    match (variant - 1) % 4 {
        // sub_8010474: invisible, flashing, with its sound.
        0 => {
            coll_mut(b, r).status_timers[timer::FLASH] = ticks;
            set_flag1(b, r, f1::INVISIBLE);
            b.sound(crate::content::SoundRole::Invisible);
        }
        1 => set_invulnerable(b, r, ticks),
        2 => {
            coll_mut(b, r).status_timers[timer::BLIND] = ticks;
            set_flag2(b, r, 0x20);
        }
        _ => coll_mut(b, r).status_timers[timer::CONFUSE] = ticks,
    }
}

/// `sub_80141F4`: L/R turning, except with the standard column patterns.
fn enable_turning(b: &mut Battle, r: ObjectRef) {
    const DUSTMAN_MINI_GAME: u8 = 0xB;
    if matches!(b.panel_pattern(), 0x38 | 0x30 | 0x3C) || battle_mode(b) == DUSTMAN_MINI_GAME {
        return;
    }
    ai_mut(b, r).status |= crate::actor::status::CAN_TURN;
}

/// `sub_802DFC8`: reset the side's extra state (set up only in the battle
/// the own-gauges mode).
fn reset_side_state(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get(r);
    let (side, panel_x) = (o.alliance as usize, o.panel.x);
    let own = own_gauges(b);
    let s = &mut b.sides[side];
    *s = Default::default();
    if own {
        // The game also sets bytes nothing ported reads (see
        // docs/engine/field-names.md, SideState).
        s.active = 1;
        s.panel_x = panel_x;
        reset_select_special(s);
    }
}

/// `sub_802E07C`: the SELECT special is over and its hold rearmed. (It
/// also clears side bytes nothing ported reads: +3, +0x2A, +0x18..+0x23.)
pub(crate) fn reset_select_special(s: &mut crate::battle::SideState) {
    s.select_special = 0;
    s.select_ticks = 0xB4;
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
    // The side's rules' tick for the navi, if one asked (EXE6's NaviCust
    // emotion-swing bug, `sub_8013DA0`), not while paused.
    if !b.paused && ai(b, r).ticked {
        let side = b.objects.get(r).alliance;
        b.rules_navi_tick(side, r);
    }
    intake::collect_hits(b, r);
    status::update(b, r);
    per_form_tick(b, r);
    tick_cooldowns(b, r);
    full_synchro_effect(b, r);
    navi_palette(b, r);
    if !b.paused {
        b.present_collision(coll_id(b, r));
    }
}

/// `sub_80100EC` (presentation only): a navi in a form with a glow
/// glows (`sub_8016A38`, Beast Over's: a color shader by the battle time);
/// any other takes its sprite palette (`sub_801002C`):
///
/// - MegaMan while he can't charge (status 0x200): 1, plus the element
///   style's;
/// - MegaMan in base form or a plain Beast Out: 4 in Full Synchro, else 0,
///   plus the element style's in base form;
/// - MegaMan in a Cross (a Cross Beast's is 0): the Cross's
///   (`byte_80203EA`);
/// - a link navi (`sub_800FD0A`): 4 in Full Synchro, 1 while it can't
///   charge, else 0 (the navi's version 0's of `byte_800FD5C`), times the
///   navi's palette step (`byte_80212BB`: every navi's is 1 in EXE6; EXE5's
///   0x0801D737).
fn navi_palette(b: &mut Battle, r: ObjectRef) {
    if let Some(glow) = &form_of(b, r).glow {
        let shader = glow[(b.round.battle_time % glow.len() as u32) as usize];
        b.objects.sprite_mut(r).look.color_shader = shader;
        return;
    }
    let palette = palette_pick(b, r);
    b.objects.sprite_mut(r).look.palette = palette;
}

/// `sub_801002C`: the navi's sprite palette (`navi_palette`'s), which its
/// init sets too, as its sprite loads.
fn palette_pick(b: &mut Battle, r: ObjectRef) -> u8 {
    let s = *stats(b, r);
    // The side's rules may pick it (EXE5's 0x0800DD94: its light and dark
    // part's; EXE4's 0x0800BFFE).
    let side = b.objects.get(r).alliance;
    if let Some(palette) = b.rules_navi_palette(side, r) {
        return palette;
    }
    let (base, mood_palette, form_palette) = {
        let form = form_of(b, r);
        (form.base, form.traits.has(FormTraits::MOOD_PALETTE), form.palette)
    };
    let no_charge = ai(b, r).status & crate::actor::status::NO_CHARGE != 0;
    let full_synchro = emotion_role(b, b.objects.get(r).alliance) == Some(EmotionRole::FullSynchro);
    let style = if s.element != 0 { s.element.wrapping_mul(5).wrapping_add(0x12) } else { 0 };
    let palette = if !is_megaman(b, r) {
        let by_state: u8 = match (full_synchro, no_charge) {
            (true, _) => 4,
            (false, true) => 1,
            (false, false) => 0,
        };
        by_state.wrapping_mul(navi_of(b, r).palette_step)
    } else if no_charge {
        1u8.wrapping_add(style)
    } else {
        let by_mood = if s.mood == 0xFF { 4u8 } else { 0 };
        if base {
            by_mood.wrapping_add(style)
        } else if mood_palette {
            // (EXE6's Beast Out.)
            by_mood
        } else {
            // `byte_80203EA`: a Cross's palette (the bytes after the
            // Crosses', a Cross in Beast Out's, are 0).
            form_palette
        }
    };
    palette
}

/// `off_80EA93C[AIIndex]`: the per-form tick hook, which only MegaMan's
/// and ChargeMan's actor records have (`sub_80F0608`): the Fire chip's
/// charge for a navi or a form that has one (ChargeMan's limit by his
/// level, `byte_802136D`: the navi's `fire_charge`; ChargeCross's 100:
/// the form's), and the height clamp of a navi that changes form.
fn per_form_tick(b: &mut Battle, r: ObjectRef) {
    // The form's own part (EXE5's MegaMan's routine, 0x080F04CE: by soul),
    // or the navi's own, for one that doesn't change form (EXE5's
    // GyroMan's, 0x080F09EC: the table's entry for his AI index).
    let (navi, form) = (stats(b, r).navi, stats(b, r).form);
    let own = if is_megaman(b, r) { b.content.defs.form(form).tick } else { b.content.defs.navi(navi).tick };
    if let Some(f) = own {
        crate::behavior::call_hook(b, f, nettai_content_api::HookCall::FormNavi { navi: r });
    }
    // (EXE5's runs none of the rest: the status rules' `form_tick`.)
    if !b.game_rules().form_tick {
        return;
    }
    let content = b.content.clone();
    let s = stats(b, r);
    let (navi, form) = (content.navi(s.navi), content.form(s.form));
    if !b.paused {
        if content.defs.navi(s.navi).given.fire_charge.is_some() {
            // ChargeMan charges his Fire chips up to the limit the content
            // gave his side for the round (`crate::given`: by his level,
            // none without one). (The game first compares the level with
            // the word at the start of `byte_8021300`, MegaMan's row of
            // zeros: never less.)
            if let Some(limit) = b.given.navis[b.objects.get(r).alliance as usize & 1].fire_charge {
                charge_fire_chip(b, r, limit);
            }
        } else if navi.changes_form()
            && let Some(limit) = form.fire_charge
        {
            charge_fire_chip(b, r, limit);
        }
    }
    if navi.changes_form() {
        if form.hover != 0 {
            b.objects.get_mut(r).pos.z = (form.hover as i32) << 16;
        } else if !runs_role(b, r, crate::content::ActionRole::Ungrounded) && flag1(b, r) & f1::BUBBLED == 0 {
            b.objects.get_mut(r).pos.z = 0;
        }
    }
}

/// `sub_80F0608`, ChargeCross (and ChargeCross in Beast Out): while the navi charges
/// with A, a damaging Fire chip up next gains a point of damage each time
/// the charge counter reaches 15 (which sets it back to 10), up to
/// `limit`; without the A charge the bonus is lost.
fn charge_fire_chip(b: &mut Battle, r: ObjectRef, limit: u16) {
    let side = b.objects.get(r).alliance as usize;
    let hand = &b.hands[side];
    let i = hand.cursor as usize;
    let Some(&chip) = hand.ids.get(i) else { return };
    // With no chip left, the game looks up chip 0xFFFF, far past the
    // table (`Rules::empty_hand`).
    let (flags, fire) = match chip {
        None => {
            let e = b.game_rules().empty_hand;
            (e.flags, e.fire)
        }
        Some(chip) => {
            let cd = b.content.chip(chip);
            (cd.flags, cd.element == crate::content::Element::Fire)
        }
    };
    if !flags.has(crate::content::ChipFlags::HAS_DAMAGE) || !fire {
        return;
    }
    if b.hands[side].charge_bonus[i] >= limit {
        return;
    }
    let a = ai_mut(b, r);
    if a.charge_source != 1 {
        b.hands[side].charge_bonus[i] = 0;
        return;
    }
    if a.charge_counter < 0xF {
        return;
    }
    a.charge_counter = 0xA;
    b.hands[side].charge_bonus[i] += 1;
}

/// `sub_80107D4`: chip lockout and special cooldowns (not while dimmed).
fn tick_cooldowns(b: &mut Battle, r: ObjectRef) {
    if b.is_dimmed() {
        return;
    }
    let a = ai_mut(b, r);
    a.lockout = a.lockout.saturating_sub(1);
    a.back_special_cooldown = a.back_special_cooldown.saturating_sub(1);
    // The per-side gauge timers count down here too (+0x2E, the per-player
    // gauges' mode's, isn't ported: nothing reads it).
    let side = &mut b.sides[b.objects.get(r).alliance as usize];
    side.fast_gauge_ticks = side.fast_gauge_ticks.saturating_sub(1);
    side.slow_gauge_ticks = side.slow_gauge_ticks.saturating_sub(1);
}

/// `sub_80139C4`: the Full Synchro aura, spawned while the emotion is 2.
fn full_synchro_effect(b: &mut Battle, r: ObjectRef) {
    // (The original tests the actor record: a player's, with an AI index
    // up to 0xB: the navis, each of which has an aura of its own.)
    let a = ai(b, r);
    if b.objects.get(r).hp == 0 || a.actor_type != ActorType::Player || b.content.identity(a.identity).aura_anim.is_none() {
        return;
    }
    if emotion_role(b, b.objects.get(r).alliance) == Some(EmotionRole::FullSynchro) && a.full_synchro_aura.is_none() {
        crate::kinds::full_synchro_aura::spawn(b, r);
    }
}

// ---- Destroy -------------------------------------------------------------------

/// The destroy state, as the rules' `dead_player` says: `sub_8016C4E`,
/// once (players, `not_counted == 0`, stay allocated and linked until the
/// end of the round), or EXE4's 0x0801052C, freed at once.
fn destroy(b: &mut Battle, r: ObjectRef) {
    if b.game_rules().dead_player == crate::content::DeadPlayer::Freed {
        let c = coll_id(b, r);
        b.collision.free(c);
        // (Its param 2 set, an owner's count goes down in its place: no
        // player the engine spawns has one.)
        let side = b.objects.get(r).alliance as usize;
        b.round.actor_count[side] = b.round.actor_count[side].wrapping_sub(1);
        let a = actor_id(b, r);
        b.actors.free(a);
        b.objects.free(r);
        return;
    }
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
/// step the sprite (not while paused, while dimmed without flag 0x10, or
/// with `PreventAnim`).
pub(crate) fn update_sprite(b: &mut Battle, r: ObjectRef) {
    if b.paused {
        return;
    }
    let o = b.objects.get(r);
    if o.flags & flags::ACTIVE == 0 || o.flags & flags::NO_SPRITE_UPDATE != 0 {
        return;
    }
    if o.flags & flags::RUN_WHILE_DIMMED == 0 && b.is_dimmed() {
        return;
    }
    if o.collision.is_some() && o.prevent_anim != 0 {
        return;
    }
    let (anim, loaded) = (o.anim, o.anim_loaded);
    if anim != loaded {
        b.objects.sprite_mut(r).set_animation(anim, &b.content);
        b.objects.get_mut(r).anim_loaded = anim;
    }
    b.objects.sprite_mut(r).update(&b.content);
}
