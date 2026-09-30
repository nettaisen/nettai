//! A navi's parts: the extra sprites an actor record adds to whatever
//! wears its look (`sub_8010DF6`, the table `off_8010E0C` by actor type
//! and AI index), and takes off again (`sub_8011044`, `off_801105C`). The
//! navi framework's: a navi chip's navi adds his own when he appears, and
//! MegaMan's forms theirs (for them the table continues as the form table
//! `off_801127C`, AI index 24 + form).
//!
//! The parts are body overlays (`body_overlay`), SpoutMan's layer
//! (`navi_layer`) and the beast heads (`form::put_on_overlay_stepping`).
//! The first part goes in the wearer's second related slot; CircusMan's
//! record adds a second one, which the routine keeps in the wearer's
//! ExtraVars[0] (returned here, for the wearer to keep).

use crate::actor::ActorType;
use crate::battle::Battle;
use crate::kinds::common::{Progress, set_progress};
use crate::kinds::player::form;
use crate::kinds::{body_overlay, navi_layer};
use crate::object::ObjectRef;
use crate::setup::Form;

/// `sub_80C44A8` with Param1 `variant`, Param2 `own_palette`, Param3 `arg`
/// (the sprite steps even while paused) and Param4 `anim_offset`.
fn overlay(b: &mut Battle, r: ObjectRef, variant: u8, own_palette: bool, arg: u8, anim_offset: u8) -> Option<ObjectRef> {
    let spec = body_overlay::Vars { variant, own_palette, always_step: arg != 0, anim_offset, ..Default::default() };
    body_overlay::spawn_with(b, r, spec)
}

/// `sub_8010DF6(actor_type, ai_index, arg)`: put on the record's parts.
/// Returns the second part, for the records that add one.
pub fn add(b: &mut Battle, r: ObjectRef, actor_type: ActorType, ai_index: u8, arg: u8) -> Option<ObjectRef> {
    // off_8010E18: a virus record adds nothing.
    if actor_type == ActorType::Virus {
        return None;
    }
    // off_8010EA4.
    let first = match ai_index {
        // sub_8010F6A
        1 => overlay(b, r, 2, false, arg, 0),
        // sub_8010F7A: SpoutMan's layer.
        6 => navi_layer::spawn(b, r),
        // sub_8010F86
        9 => overlay(b, r, 0x0B, false, arg, 0),
        // sub_8010F96
        13 => overlay(b, r, 3, false, arg, 0x0A),
        // sub_8010FD8: two overlays; the second is kept in ExtraVars[0].
        14 => {
            let first = overlay(b, r, 6, false, arg, 0x14);
            b.objects.get_mut(r).related[1] = first;
            return overlay(b, r, 7, false, arg, 0x28);
        }
        // sub_8011004
        16 => overlay(b, r, 0x0F, true, arg, 9),
        // sub_8010FAC
        18 => overlay(b, r, 0, false, arg, 0x0D),
        // sub_8010FC2
        19 => overlay(b, r, 0x10, false, arg, 0x14),
        // The form table (sub_8011268's): a Cross's helmet and arm, the
        // beast heads.
        24.. => {
            form::put_on_overlay_stepping(b, r, Form(ai_index - 24), arg != 0);
            return None;
        }
        // nullsub_42 / nullsub_43: nothing (and the second related slot is
        // left alone).
        _ => return None,
    };
    b.objects.get_mut(r).related[1] = first;
    None
}

/// `sub_8011044(actor_type, ai_index)`: take the record's parts off (at
/// their next update). `extra` is the second part (CircusMan's ExtraVars
/// part), which the caller then forgets.
pub fn remove(b: &mut Battle, r: ObjectRef, actor_type: ActorType, ai_index: u8, extra: Option<ObjectRef>) {
    // off_8011068: a virus record has nothing to take off.
    if actor_type == ActorType::Virus {
        return;
    }
    // off_80110F4.
    match ai_index {
        // sub_80111B8, sub_80111CA, sub_80111EE, sub_8011200, sub_8011256,
        // sub_8011212, sub_8011224: a body overlay (sub_80C44C8).
        // sub_801140E, sub_80113FC: a form's overlay or beast head
        // (sub_80C46B0, sub_80C44C8).
        0 | 1 | 9 | 13 | 16 | 18 | 19 | 24..=34 | 36 | 42..=46 | 48 => take_off(b, r),
        // sub_80111DC: SpoutMan's layer (sub_80C4204).
        6 => {
            if let Some(o) = b.objects.get_mut(r).related[1].take() {
                navi_layer::remove(b, o);
            }
        }
        // sub_8011236: both of CircusMan's overlays.
        14 => {
            take_off(b, r);
            if let Some(o) = extra {
                set_progress(b, o, Progress::DESTROY);
            }
        }
        _ => {}
    }
}

/// Destroy the part in the second related slot (a word store of the
/// destroy state), and forget it.
fn take_off(b: &mut Battle, r: ObjectRef) {
    if let Some(o) = b.objects.get_mut(r).related[1].take() {
        set_progress(b, o, Progress::DESTROY);
    }
}
