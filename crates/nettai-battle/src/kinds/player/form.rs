//! What MegaMan's form and a navi's identity decide beyond the weapons and
//! stats of their definitions: the overlay sprites a navi or form wears
//! (the init hooks `off_8010E0C`, `sub_8011268`, `sub_8011420`: an
//! identity's `parts`), how they come off (the death hooks `off_801105C`,
//! `sub_8011384`), and the flags and helper objects a form adds
//! (`sub_8014536`: a form's `status_reset`).

use nettai_content_api::{FormHandle, IdentityHandle, NaviHandle};
use super::{ai, body_hit_modifier, form_of, set_flag1};
use crate::battle::Battle;
use crate::collision::f1;
use crate::content::{FormEffects, Parts};
use crate::kinds::common::{Progress, set_progress};
use crate::kinds::{body_overlay, form_overlay, idle_overlay, target_marker};
use crate::object::{ObjectRef, flags};

/// `sub_8010DF6`: the init hook of an identity's actor record, run on the
/// object `r` (the navi, or an image of it), with its r2 (the overlay's
/// Param3): what the identity wears goes in the object's `related[1]`
/// (and a second body overlay in its `second_overlay`). Most identities'
/// is nothing (`nullsub_42`, `nullsub_43`).
pub(crate) fn put_on_parts(b: &mut Battle, r: ObjectRef, identity: Option<IdentityHandle>, param3: u8) {
    let content = b.content.clone();
    let (Some(h), Some(parts)) = (identity, &content.identity(identity).parts) else { return };
    let body = |b: &mut Battle, second: bool, part: &crate::content::BodyPart| {
        let spec = body_overlay::Vars {
            identity: Some(h),
            second,
            own_palette: part.own_palette,
            always_step: param3 != 0,
            anim_offset: part.anim_offset,
            ..Default::default()
        };
        body_overlay::spawn_with(b, r, spec)
    };
    let overlay = match parts {
        // sub_8010F6A, sub_8010F86, sub_8010F96, sub_8010FAC, sub_8010FC2,
        // sub_8011004: a navi's body overlay; sub_80112E0 .. sub_801133A
        // (sub_8011344): the Crosses'.
        Parts::Body(part) => body(b, false, part),
        // sub_8010FD8: two of them.
        Parts::Bodies(first, second) => {
            let first = body(b, false, first);
            b.objects.get_mut(r).related[1] = first;
            let second = body(b, true, second);
            b.objects.get_mut(r).second_overlay = second;
            return;
        }
        // sub_8010F7A: SpoutMan's idle overlay (its r2 is not used).
        Parts::Idle { sprite } => idle_overlay::spawn(b, r, *sprite),
        // sub_8011366 and sub_8011352 .. sub_8011362 (loc_8011368): a beast
        // head; one without a palette of its own follows the mood (the
        // Falzar beast's and Beast Over's).
        Parts::BeastHead { sprite, palette } => {
            let spec = form_overlay::Vars {
                sprite: Some(*sprite),
                nudged: true,
                stepping: form_overlay::Stepping::from_param(param3),
                palette: if palette.is_some() { form_overlay::Palette::Own } else { form_overlay::Palette::Mood },
                palette_index: palette.unwrap_or(0),
                ..Default::default()
            };
            form_overlay::spawn_with(b, r, spec)
        }
        // A record's own routine (EXE5's 0x0800EE6E, NumberMan's): it keeps
        // what it puts on in the second related slot itself.
        Parts::Own(f) => {
            crate::behavior::call_hook(b, *f, nettai_content_api::HookCall::FormNavi { navi: r });
            return;
        }
    };
    b.objects.get_mut(r).related[1] = overlay;
}

/// `sub_8010DD0` / `sub_8010DDA`: the init hook of `identity`'s actor
/// record, run on the object `r`: most navis' is nothing; a few wear
/// overlays.
pub(crate) fn navi_init_hook(b: &mut Battle, r: ObjectRef, identity: Option<IdentityHandle>) {
    put_on_parts(b, r, identity, 0);
}

/// `sub_8010DD0` in the base form, at a player's init: the base form's
/// put-on routine (its `put_on`), which EXE5's MegaMan's actor record names
/// as his init hook (0x0800ED90's row 11 and 0x0800F038's row 0 are one
/// routine, 0x0800EE1C: Hub Style's shade); else its navi's init hook. (A
/// navi that doesn't change form is in the game's base form too, MegaMan's:
/// its own actor record's hook is its navi's, what its identity wears.)
pub(crate) fn base_init_hook(b: &mut Battle, r: ObjectRef, identity: Option<IdentityHandle>) {
    let form = super::stats(b, r).form;
    if super::is_megaman(b, r)
        && let Some(f) = b.content.defs.form(form).put_on
    {
        crate::behavior::call_hook(b, f, nettai_content_api::HookCall::FormNavi { navi: r });
        return;
    }
    navi_init_hook(b, r, identity);
}

/// `sub_8011268(form)`: the form's put-on routine (EXE6's by form, EXE5's
/// 0x0800F024 by soul): its `put_on`, else the default (`put_on_form_parts`).
pub(crate) fn put_on_overlay(b: &mut Battle, r: ObjectRef, form: FormHandle) {
    if let Some(f) = b.content.defs.form(form).put_on {
        crate::behavior::call_hook(b, f, nettai_content_api::HookCall::FormNavi { navi: r });
        return;
    }
    put_on_form_parts(b, r, form);
}

/// The default put-on routine: the overlay `form`'s identity wears, its
/// parts (r2, the overlay's Param3, is always 0 here). A base form, which
/// has no identity of its own, puts on nothing.
pub(crate) fn put_on_form_parts(b: &mut Battle, r: ObjectRef, form: FormHandle) {
    let identity = b.content.form(form).identity;
    put_on_parts(b, r, identity, 0);
}

/// A form's own hook on its navi (`reset`).
fn form_hook(b: &mut Battle, r: ObjectRef, hook: Option<nettai_content_api::FnId>) {
    if let Some(f) = hook {
        crate::behavior::call_hook(b, f, nettai_content_api::HookCall::FormNavi { navi: r });
    }
}

/// `sub_8011420(navi, form, param3)`: the image of a navi puts on its
/// overlay: a navi that changes form its form's (`sub_8011268`, where the
/// Param3 is lost), a link navi its own (its init hook, with the Param3).
pub(crate) fn put_on_navi_overlay(b: &mut Battle, r: ObjectRef, navi: NaviHandle, form: FormHandle, param3: u8) {
    if b.content.navi(navi).changes_form() {
        return put_on_overlay(b, r, form);
    }
    let identity = b.content.navi(navi).identity;
    put_on_parts(b, r, identity, param3);
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

/// `sub_8011020` / `sub_8011044`: what an object with an identity takes
/// down when it goes (`off_801105C`, by its actor record): the overlay in
/// its `related[1]` (and of two body overlays, the second), if its death
/// hook does (an identity that wears something; MegaMan's, for what his
/// form wears).
pub(crate) fn navi_death_hook(b: &mut Battle, r: ObjectRef, identity: Option<IdentityHandle>) {
    let content = b.content.clone();
    let identity = content.identity(identity);
    if !identity.overlay_hooks.death {
        return;
    }
    let first = b.objects.get_mut(r).related[1].take();
    take_down(b, first);
    if matches!(identity.parts, Some(Parts::Bodies(..))) {
        // sub_8011236
        let second = b.objects.get_mut(r).second_overlay.take();
        take_down(b, second);
    }
}

/// `sub_8011384(form)`: the form's take-off routine (EXE5's 0x0800F088 by
/// soul): its `take_off`, else the default (`take_off_form_parts`).
pub(crate) fn take_off_overlay(b: &mut Battle, r: ObjectRef, form: FormHandle) {
    if let Some(f) = b.content.defs.form(form).take_off {
        crate::behavior::call_hook(b, f, nettai_content_api::HookCall::FormNavi { navi: r });
        return;
    }
    take_off_form_parts(b, r, form);
}

/// The default take-off routine: the overlay `form` wore, if that form
/// wears one: its identity's death hook (`sub_80113FC`, `sub_801140E`; a
/// form that wears nothing leaves `related[1]` alone, `nullsub_43`). The
/// base form, which has no identity of its own, takes off whatever is
/// there (`sub_80111B8`; EXE5's 0x0800EFCC).
pub(crate) fn take_off_form_parts(b: &mut Battle, r: ObjectRef, form: FormHandle) {
    let takes = match b.content.form(form).identity {
        None => true,
        identity => b.content.identity(identity).overlay_hooks.death,
    };
    if takes {
        let o = b.objects.get_mut(r).related[1].take();
        take_down(b, o);
    }
}

/// `sub_8014536`: the flags and helper objects the current form adds
/// (part of the status reset): its `status_reset`.
pub(super) fn apply_form_flags(b: &mut Battle, r: ObjectRef) {
    let effects = form_of(b, r).status_reset;
    apply_effects(b, r, effects);
    // The form's own (`reset`: EXE5's souls' routines, 0x08011B92).
    let form = super::stats(b, r).form;
    form_hook(b, r, b.content.defs.form(form).reset);
}

/// `sub_801469C`: after a NaviCust edit (a bug code, an uninstall) the
/// form's flags come back (`off_80146B8`): its `navicust_refresh`, or what
/// `apply_form_flags` gave without the lock-on marker.
pub(super) fn refresh_form_flags(b: &mut Battle, r: ObjectRef) {
    let effects = form_of(b, r).refresh_effects();
    apply_effects(b, r, effects);
}

/// What a form's routine of `sub_8014536` or `sub_801469C` does, in the
/// order the original's routines do it: the statuses end
/// (`sub_80145C2`); the flags (`SetObjectAirshoeFlag`,
/// `SetObjectSuperArmorFlag`, AirShoe and FloatShoe of `sub_8014606`, the
/// untouchable flag 0x08000000 with the shoes of `sub_8014674`); the floating body; the lock-on marker;
/// invulnerable for good and the controllers' state cleared
/// (`sub_8014650`: `controller_fresh`, which EXE6's systems read).
fn apply_effects(b: &mut Battle, r: ObjectRef, effects: FormEffects) {
    if effects.has(FormEffects::CLEAR_STATUSES) {
        clear_statuses_unless_mode1(b, r);
    }
    let mut flags1 = 0;
    for (effect, flag) in [
        (FormEffects::SUPER_ARMOR, f1::SUPERARMOR),
        (FormEffects::AIR_SHOES, f1::AIRSHOE),
        (FormEffects::FLOAT_SHOES, f1::FLOATSHOE),
        (FormEffects::UNTOUCHABLE, f1::UNTOUCHABLE),
    ] {
        if effects.has(effect) {
            flags1 |= flag;
        }
    }
    if flags1 != 0 {
        set_flag1(b, r, flags1);
    }
    if effects.has(FormEffects::FLOATING_BODY) {
        let hm = body_hit_modifier(b);
        super::reset_body_types(b, r, true, hm);
    }
    if effects.has(FormEffects::TARGET_MARKER) {
        spawn_target_marker(b, r);
    }
    if effects.has(FormEffects::INVULNERABLE) {
        super::set_invulnerable(b, r, 0xFFFF);
    }
    if effects.has(FormEffects::BERSERK) {
        super::ai_mut(b, r).controller_fresh = true;
    }
}

/// `sub_80E1620` unless AIData+0x40 already holds a marker.
fn spawn_target_marker(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).target_marker.is_none() {
        target_marker::spawn(b, r);
    }
}

/// `sub_801A264` unless the battle mode is 1.
fn clear_statuses_unless_mode1(b: &mut Battle, r: ObjectRef) {
    if super::battle_mode(b) != 1 {
        super::clear_statuses(b, r);
    }
}
