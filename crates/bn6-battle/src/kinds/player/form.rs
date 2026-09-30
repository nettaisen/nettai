//! What MegaMan's form decides beyond the per-form tables in `data`: the
//! overlay sprite a form wears (`sub_8011268` / `sub_8011384`) and the
//! flags and helper objects it adds (`sub_8014536`).

use super::{ai, body_hit_modifier, set_flag1, stats};
use crate::battle::Battle;
use crate::collision::f1;
use crate::kinds::common::{Progress, set_progress};
use crate::kinds::{body_overlay, form_overlay, lockon_marker};
use crate::object::ObjectRef;
use crate::setup::Form;

/// `sub_8011268(form, 0)`: put on the overlay sprite `form` wears (kept
/// in `related[1]`).
pub(super) fn put_on_overlay(b: &mut Battle, r: ObjectRef, form: Form) {
    put_on_overlay_stepping(b, r, form, false);
}

/// `sub_8011268(form, always_step)`: the overlay may step its sprite
/// even while paused (its Param3).
pub(crate) fn put_on_overlay_stepping(b: &mut Battle, r: ObjectRef, form: Form, always_step: bool) {
    if always_step && form.0 == 0x0C {
        panic!("a stepping beast head (sub_8011366 with Param3) is not implemented yet");
    }
    match form.0 {
        // nullsub_42 / nullsub_43: no overlay.
        0 | 0x0B | 0x0D..=0x11 | 0x17 => {}
        // sub_8011366: the Falzar beast head (its palette follows the
        // navi's mood).
        0x0C => {
            let overlay = form_overlay::spawn(b, r, form_overlay::BEAST_HEAD, true);
            b.objects.get_mut(r).related[1] = overlay;
        }
        // sub_80112E0 .. sub_801133A: a Cross's helmet and arm, with its
        // own palette.
        1..=10 => {
            let spec = body_overlay::Vars { variant: cross_overlay(form), own_palette: true, always_step, ..Default::default() };
            let overlay = body_overlay::spawn_with(b, r, spec);
            b.objects.get_mut(r).related[1] = overlay;
        }
        _ => panic!("the form {form:?} overlay (sub_8011268) is not implemented yet"),
    }
}

/// The body overlay a Cross wears (`sub_80112E0` .. `sub_801133A`).
fn cross_overlay(form: Form) -> u8 {
    match form.0 {
        1 => 0x04,
        2 => 0x08,
        3 => 0x0A,
        4 => 0x0C,
        5 => 0x11,
        6 => 0x05,
        7 => 0x0E,
        8 => 0x09,
        9 => 0x0D,
        10 => 0x12,
        _ => unreachable!("form {form:?} is not a Cross"),
    }
}

/// `sub_8011020` / `sub_8011044`: what an object with a navi's NameID
/// takes down when it goes (`off_801105C`, by actor type and AI index):
/// the parts its record added (`navi_parts`). (A navi object keeps no
/// second part: only CircusMan's record adds one, and his navi chip's
/// navi takes his off himself.)
pub(crate) fn navi_death_hook(b: &mut Battle, r: ObjectRef, name_id: u16) {
    let rec = b.content.navi_record(name_id);
    crate::kinds::navi_parts::remove(b, r, rec.actor_type, rec.ai_index, None);
}

/// `sub_8011384(form)`: take off the overlay `form` wore, if that form
/// wears one.
pub(super) fn take_off_overlay(b: &mut Battle, r: ObjectRef, form: Form) {
    // Forms without an overlay leave `related[1]` alone (nullsub_43).
    if matches!(form.0, 0x0B | 0x0D..=0x11 | 0x17) {
        return;
    }
    // sub_80111B8 / sub_80113FC / sub_801140E: destroy it next update.
    if let Some(o) = b.objects.get_mut(r).related[1].take() {
        set_progress(b, o, Progress::DESTROY);
    }
}

/// `sub_8014536`: the flags and helper objects the current form adds
/// (part of the status reset).
pub(super) fn apply_form_flags(b: &mut Battle, r: ObjectRef) {
    let form = stats(b, r).form;
    match form.0 {
        0..=6 | 10 => {}
        // sub_8014606: floating, and the Beast Out lock-on marker.
        0x0C => {
            set_flag1(b, r, f1::AIRSHOE | f1::FLOATSHOE);
            let hm = body_hit_modifier(b);
            b.reset_collision_types(r, 0x10, 2, hm);
            if ai(b, r).lockon_marker.is_none() {
                lockon_marker::spawn(b, r);
            }
        }
        _ => panic!("form {form:?} flags (sub_8014536) are not implemented yet"),
    }
}
