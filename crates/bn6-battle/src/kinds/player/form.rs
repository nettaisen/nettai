//! What MegaMan's form and a navi's actor record decide beyond the
//! per-form tables in `data`: the overlay sprites a navi or form wears
//! (the init hooks `off_8010E0C`, `sub_8011268`, `sub_8011420`), how they
//! come off (the death hooks `off_801105C`, `sub_8011384`), and the flags
//! and helper objects a form adds (`sub_8014536`).

use bn6_content_api::IdentityHandle;
use super::{ai, body_hit_modifier, form_of, set_flag1};
use crate::actor::ActorType;
use crate::battle::Battle;
use crate::collision::f1;
use crate::kinds::common::{Progress, set_progress};
use crate::kinds::{body_overlay, form_overlay, idle_overlay, lockon_marker};
use crate::object::{ObjectRef, flags};
use crate::setup::Form;

/// The rows the init and death hook tables have for navis and players
/// (`off_8010EA4`, `off_80110F4`): AI indices 0..=48.
const NAVI_ROWS: u8 = 49;

/// Viruses' rows (`off_8010E18`, `off_8011068`): 35 no-ops, after which
/// the table runs on into the navis'.
const VIRUS_ROWS: u8 = 35;

/// The navi-table row an actor record's hooks come from; a virus's AI
/// index past its own rows reads on into the navis' table.
fn hook_row(actor_type: ActorType, ai_index: u8, table: &str) -> Option<u8> {
    let row = match actor_type {
        ActorType::Virus => ai_index.checked_sub(VIRUS_ROWS)?,
        _ => ai_index,
    };
    if row >= NAVI_ROWS {
        panic!("the {table} for AI index {ai_index:#x} reads past its table");
    }
    Some(row)
}

/// The palette index of the Cross Beasts' beast heads (`sub_8011352` ..
/// `sub_8011362`), by init-hook row 42..=46; 0 for the Falzar beast and
/// Beast Over (`sub_8011366`, whose palette follows the mood).
fn beast_head_palette(row: u8) -> u8 {
    match row {
        42..=46 => 2 * (row - 41),
        _ => 0,
    }
}

/// The body overlay a Cross wears (`sub_80112E0` .. `sub_801133A`), by
/// form 1..=10.
fn cross_overlay(form: u8) -> u8 {
    [0x04, 0x08, 0x0A, 0x0C, 0x11, 0x05, 0x0E, 0x09, 0x0D, 0x12][form as usize - 1]
}

/// One routine of the init-hook table's navi rows, with its r2 (the
/// overlay's Param3): the overlay goes in the navi's `related[1]` (and a
/// second one, for row 14, in its `second_overlay`).
fn init_routine(b: &mut Battle, r: ObjectRef, row: u8, param3: u8) {
    let body = |b: &mut Battle, variant: u8, own_palette: bool, anim_offset: u8| {
        let spec = body_overlay::Vars { variant, own_palette, always_step: param3 != 0, anim_offset, ..Default::default() };
        body_overlay::spawn_with(b, r, spec)
    };
    let overlay = match row {
        // sub_8010F6A, sub_8010F86, sub_8010F96, sub_8010FAC, sub_8010FC2,
        // sub_8011004: a navi's body overlay.
        1 => body(b, 2, false, 0),
        9 => body(b, 0x0B, false, 0),
        13 => body(b, 3, false, 0x0A),
        16 => body(b, 0x0F, true, 9),
        18 => body(b, 0, false, 0x0D),
        19 => body(b, 0x10, false, 0x14),
        // sub_8010FD8: two of them.
        14 => {
            let first = body(b, 6, false, 0x14);
            b.objects.get_mut(r).related[1] = first;
            let second = body(b, 7, false, 0x28);
            b.objects.get_mut(r).second_overlay = second;
            return;
        }
        // sub_8010F7A: SpoutMan's idle overlay (its r2 is not used).
        6 => idle_overlay::spawn(b, r, 0),
        // sub_80112E0 .. sub_801133A (sub_8011344): the Crosses'.
        25..=34 => body(b, cross_overlay(row - 24), true, 0),
        // sub_8011366 and sub_8011352 .. sub_8011362 (loc_8011368): a beast
        // head; the Falzar beast's palette follows the mood.
        24 | 36 | 42..=46 | 48 => {
            let palette = if matches!(row, 42..=46) { form_overlay::Palette::Own } else { form_overlay::Palette::Mood };
            let spec = form_overlay::Vars {
                sprite: Some(form_overlay::BEAST_HEAD),
                nudged: true,
                stepping: form_overlay::Stepping::from_param(param3),
                palette,
                palette_index: beast_head_palette(row),
                ..Default::default()
            };
            form_overlay::spawn_with(b, r, spec)
        }
        // nullsub_42, nullsub_43.
        _ => return,
    };
    b.objects.get_mut(r).related[1] = overlay;
}

/// `sub_8010DD0` / `sub_8010DDA`: the init hook of NameID `identity`'s
/// actor record, run on the object `r` (the navi, or an image of it):
/// most navis' is nothing; a few wear overlays.
pub(crate) fn navi_init_hook(b: &mut Battle, r: ObjectRef, identity: Option<IdentityHandle>) {
    let rec = b.content.navi_record(identity);
    record_init_hook(b, r, rec.actor_type, rec.ai_index, 0);
}

/// `sub_8010DF6(actor_type, ai_index, param3)`: the init hook of an actor
/// record given outright (a navi chip's navi puts on his own this way,
/// with r2 = 1: his overlay steps even while paused).
pub(crate) fn record_init_hook(b: &mut Battle, r: ObjectRef, actor_type: ActorType, ai_index: u8, param3: u8) {
    if let Some(row) = hook_row(actor_type, ai_index, "init hook (off_8010E0C)") {
        init_routine(b, r, row, param3);
    }
}

/// `sub_8011268(form)`: put on the overlay `form` wears (r2, the
/// overlay's Param3, is always 0 here).
pub(crate) fn put_on_overlay(b: &mut Battle, r: ObjectRef, form: Form) {
    match form.0 {
        // nullsub_42
        0 => {}
        1..=0x18 => init_routine(b, r, 24 + form.0, 0),
        _ => panic!("the overlay of form {form:?} reads past its table (sub_8011268)"),
    }
}

/// `sub_8011420(navi, form, param3)`: the image of a navi puts on its
/// overlay: MegaMan his form's (`sub_8011268`, where the Param3 is lost),
/// a link navi its own (its init hook's row by navi number, with the
/// Param3).
pub(crate) fn put_on_navi_overlay(b: &mut Battle, r: ObjectRef, navi: crate::setup::Navi, form: Form, param3: u8) {
    if navi == crate::setup::Navi::MEGAMAN {
        return put_on_overlay(b, r, form);
    }
    if navi.0 >= NAVI_ROWS {
        panic!("the init hook for navi {:#x} reads past its table (sub_8010DF6)", navi.0);
    }
    init_routine(b, r, navi.0, param3);
}

/// Param3 = 1 and flags |= 0x14 on an overlay (`sub_80C0E24`, and the
/// form changes' first step): it keeps running while paused and dimmed,
/// and steps its sprite through them.
pub(crate) fn keep_overlay_stepping(b: &mut Battle, overlay: ObjectRef) {
    let o = b.objects.get_mut(overlay);
    o.params[2] = 1;
    o.flags |= flags::RUN_WHILE_PAUSED | flags::RUN_WHILE_DIMMED;
    match &mut o.vars {
        crate::kinds::Vars::FormOverlay(v) => v.stepping = form_overlay::Stepping::WhileDimmed,
        crate::kinds::Vars::BodyOverlay(v) => v.always_step = true,
        // The idle overlay doesn't read its Param3.
        _ => {}
    }
}

/// Param3 = 0 on an overlay (Beast Out's emergence): it steps like any
/// object again.
pub(crate) fn normal_overlay_stepping(b: &mut Battle, overlay: ObjectRef) {
    let o = b.objects.get_mut(overlay);
    o.params[2] = 0;
    match &mut o.vars {
        crate::kinds::Vars::FormOverlay(v) => v.stepping = form_overlay::Stepping::Normal,
        crate::kinds::Vars::BodyOverlay(v) => v.always_step = false,
        _ => {}
    }
}

/// `sub_80C4526(overlay, 1)`: an overlay copied onto an afterimage sits in
/// front (and an idle overlay keeps its owner's height).
pub(crate) fn pin_overlay(b: &mut Battle, overlay: ObjectRef) {
    match &mut b.objects.get_mut(overlay).vars {
        crate::kinds::Vars::FormOverlay(v) => v.nudged = true,
        crate::kinds::Vars::BodyOverlay(v) => v.forced_front = true,
        crate::kinds::Vars::IdleOverlay(v) => v.pinned = true,
        _ => {}
    }
}

/// Take down an overlay (`sub_80C44C8`, `sub_80C46B0`, `sub_80C4204`: its
/// state word becomes 8; it frees itself at its next update).
fn take_down(b: &mut Battle, overlay: Option<ObjectRef>) {
    if let Some(o) = overlay {
        set_progress(b, o, Progress::DESTROY);
    }
}

/// The death-hook table's navi rows that take the init hook's overlays
/// down.
fn has_death_routine(row: u8) -> bool {
    matches!(row, 0 | 1 | 6 | 9 | 13 | 14 | 16 | 18 | 19 | 24..=34 | 36 | 42..=46 | 48)
}

/// `sub_8011020` / `sub_8011044`: what an object with a navi's NameID
/// takes down when it goes (`off_801105C`, by actor type and AI index):
/// the overlay in its `related[1]` (and row 14's second one).
pub(crate) fn navi_death_hook(b: &mut Battle, r: ObjectRef, identity: Option<IdentityHandle>) {
    let rec = b.content.navi_record(identity);
    record_death_hook(b, r, rec.actor_type, rec.ai_index);
}

/// `sub_8011044(actor_type, ai_index)`: the death hook of an actor record
/// given outright (a navi chip's navi takes his own off this way).
pub(crate) fn record_death_hook(b: &mut Battle, r: ObjectRef, actor_type: ActorType, ai_index: u8) {
    let Some(row) = hook_row(actor_type, ai_index, "death hook (off_801105C)") else { return };
    if !has_death_routine(row) {
        return;
    }
    let first = b.objects.get_mut(r).related[1].take();
    take_down(b, first);
    if row == 14 {
        // sub_8011236
        let second = b.objects.get_mut(r).second_overlay.take();
        take_down(b, second);
    }
}

/// `sub_8011384(form)`: take off the overlay `form` wore, if that form
/// wears one.
pub(super) fn take_off_overlay(b: &mut Battle, r: ObjectRef, form: Form) {
    match form.0 {
        // nullsub_43: forms without an overlay leave `related[1]` alone.
        0x0B | 0x0D..=0x11 | 0x17 => {}
        // sub_80111B8 / sub_80113FC / sub_801140E.
        0..=0x18 => {
            let o = b.objects.get_mut(r).related[1].take();
            take_down(b, o);
        }
        _ => panic!("taking off the overlay of form {form:?} reads past its table (sub_8011384)"),
    }
}

/// `sub_8014536`: the flags and helper objects the current form adds
/// (part of the status reset).
pub(super) fn apply_form_flags(b: &mut Battle, r: ObjectRef) {
    let form = form_of(b, r);
    match form.0 {
        // nullsub_50 .. nullsub_56, nullsub_4.
        0..=6 | 10 => {}
        // sub_80145C2, TomahawkCross: statuses end (not in battle mode 1).
        7 => clear_statuses_unless_mode1(b, r),
        // SetObjectAirshoeFlag, TenguCross.
        8 => set_flag1(b, r, f1::AIRSHOE),
        // SetObjectSuperArmorFlag, GroundCross.
        9 => set_flag1(b, r, f1::SUPERARMOR),
        // sub_80145EC, the Gregar beast: SuperArmor and the lock-on
        // marker.
        0x0B => {
            set_flag1(b, r, f1::SUPERARMOR);
            spawn_lockon_marker(b, r);
        }
        // sub_8014606: the Falzar beast and SpoutCross, TenguCross and
        // DustCross Beasts: floating and the lock-on marker.
        0x0C | 0x12 | 0x14 | 0x16 => floating_beast(b, r),
        // sub_801462A, TomahawkCross Beast.
        0x13 => {
            clear_statuses_unless_mode1(b, r);
            floating_beast(b, r);
        }
        // sub_8014640, GroundCross Beast.
        0x15 => {
            set_flag1(b, r, f1::SUPERARMOR);
            floating_beast(b, r);
        }
        // sub_8014650: the Gregar Cross Beasts and Gregar Beast Over:
        // SuperArmor, the lock-on marker, invulnerable for good, and the
        // berserk controller's state cleared.
        0x0D..=0x11 | 0x17 => {
            set_flag1(b, r, f1::SUPERARMOR);
            spawn_lockon_marker(b, r);
            super::set_invulnerable(b, r, 0xFFFF);
            super::berserk::reset(b, r);
        }
        // sub_8014674, Falzar Beast Over: flag 0x08000000 (not the
        // floating ones), the floating body, the lock-on marker, the
        // berserk controller's state cleared.
        0x18 => {
            set_flag1(b, r, f1::UNAFFECTED_BY_POISON);
            let hm = body_hit_modifier(b);
            super::reset_body_types(b, r, true, hm);
            spawn_lockon_marker(b, r);
            super::berserk::reset(b, r);
        }
        _ => panic!("the flags of form {form:?} read past their table (sub_8014536)"),
    }
}

/// `sub_801469C`: after a NaviCust edit (a bug code, an uninstall) the
/// form's flags come back, as `apply_form_flags` gave them but without the
/// lock-on markers (`off_80146B8`).
pub(super) fn refresh_form_flags(b: &mut Battle, r: ObjectRef) {
    let form = form_of(b, r);
    // sub_8014760: floating (AirShoe and FloatShoe), the floating body.
    let floating = |b: &mut Battle| {
        set_flag1(b, r, f1::AIRSHOE | f1::FLOATSHOE);
        let hm = body_hit_modifier(b);
        super::reset_body_types(b, r, true, hm);
    };
    match form.0 {
        // nullsub_801471C .. nullsub_8014728, nullsub_5.
        0..=6 | 10 => {}
        // sub_801472A, TomahawkCross.
        7 => clear_statuses_unless_mode1(b, r),
        // sub_801473C, TenguCross.
        8 => set_flag1(b, r, f1::AIRSHOE),
        // sub_8014746, GroundCross; sub_8014754, the Gregar beast and its
        // Crosses.
        9 | 0x0B | 0x0D..=0x11 => set_flag1(b, r, f1::SUPERARMOR),
        // sub_8014760: the Falzar beast, SpoutCross, TenguCross and
        // DustCross Beasts.
        0x0C | 0x12 | 0x14 | 0x16 => floating(b),
        // sub_8014776, TomahawkCross Beast.
        0x13 => {
            clear_statuses_unless_mode1(b, r);
            floating(b);
        }
        // sub_801478C, GroundCross Beast.
        0x15 => {
            set_flag1(b, r, f1::SUPERARMOR);
            floating(b);
        }
        // sub_801479C, Gregar Beast Over.
        0x17 => {
            set_flag1(b, r, f1::SUPERARMOR);
            super::set_invulnerable(b, r, 0xFFFF);
            super::berserk::reset(b, r);
        }
        // sub_80147B2, Falzar Beast Over.
        0x18 => {
            floating(b);
            set_flag1(b, r, f1::UNAFFECTED_BY_POISON);
            super::set_invulnerable(b, r, 0xFFFF);
            super::berserk::reset(b, r);
        }
        _ => panic!("the NaviCust refresh of form {form:?} reads past its table (sub_801469C)"),
    }
}

/// `sub_80E1620` unless AIData+0x40 already holds a marker.
fn spawn_lockon_marker(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).lockon_marker.is_none() {
        lockon_marker::spawn(b, r);
    }
}

/// `sub_8014606`: AirShoe and FloatShoe, the floating body (collision
/// type 0x10), and the lock-on marker.
fn floating_beast(b: &mut Battle, r: ObjectRef) {
    set_flag1(b, r, f1::AIRSHOE | f1::FLOATSHOE);
    let hm = body_hit_modifier(b);
    super::reset_body_types(b, r, true, hm);
    spawn_lockon_marker(b, r);
}

/// `sub_801A264` unless the battle mode is 1.
fn clear_statuses_unless_mode1(b: &mut Battle, r: ObjectRef) {
    if super::battle_mode(b) != 1 {
        super::clear_statuses(b, r);
    }
}
