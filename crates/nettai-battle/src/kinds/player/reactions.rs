//! Hit reactions: deletion (action 2), flinch (3), paralysis (4), drag
//! (5), freeze (6) and bubble (7). See objects-and-player.md §H4-§H6.

use super::{
    ai, ai_mut, cancel_submerged, clear_bubble, clear_flag1, clear_flag2, clear_freeze, clear_invulnerable,
    clear_paralysis, coll, coll_mut, coordinates_to_panel, flag1, panel_coordinates, panel_kind, refresh_form_overlay,
    reset_charge, set_flag1, snap_to_future_panel, NaviAction, set_action,
};
use crate::actor::{request, status as ai_status};
use crate::battle::Battle;
use crate::collision::{f1, timer};
use crate::content::{DragEnding, IceRule, PushSource, ReactionActions, SlideVector};
use crate::field::{self, PanelType};
use crate::object::{DragStep, ObjectRef, PanelPos, state};

// ---- Deletion (action 2) -------------------------------------------------------

/// Action 2, `sub_80173F4`: deletion (§H6). (A navi no player controls
/// is deleted its own way: `ai_navi::deletion`.)
pub(super) fn deletion(b: &mut Battle, r: ObjectRef) {
    if super::ai_navi::is_ai_navi(b, r) {
        return super::ai_navi::deletion(b, r);
    }
    // sprite_forceWhitePalette every tick.
    b.objects.sprite_mut(r).look.white = true;
    match b.objects.get(r).phase {
        0 => begin_deletion(b, r),
        4 => explode(b, r),
        8 => {
            // sub_80174AA
            let o = b.objects.get_mut(r);
            o.timer = o.timer.wrapping_sub(1);
            if o.timer == 0 {
                o.phase = 0xC;
                o.phase_init = 0;
            }
        }
        _ => fade_out(b, r),
    }
}

/// `sub_801741C`: stop colliding, drop chips, reservations, barrier and
/// the charge glow, and leave the side's alive list: the battle is over
/// from here.
fn begin_deletion(b: &mut Battle, r: ObjectRef) {
    let c = coll_mut(b, r);
    c.region = None;
    // sub_801A5E2
    c.links[crate::collision::link::CONFUSE] = None;
    c.links[crate::collision::link::BLIND] = None;
    cancel_submerged(b, r);
    reset_charge(b, r);
    // sub_801DC36: HUD.
    let o = b.objects.get_mut(r);
    o.chips_held = 0;
    o.chip = None;
    let fp = o.future_panel;
    b.unreserve_panel(r, fp.x, fp.y);
    // sub_801A7F4: the barrier goes, and the navi forgets its visual
    // (AIData+0x60).
    coll_mut(b, r).barrier = 0;
    ai_mut(b, r).barrier_visual = None;
    // The charge glow sees its slot cleared and ends itself.
    ai_mut(b, r).charge_glow = None;
    if b.objects.get(r).params[1] < 1 {
        remove_from_alive(b, r);
    }
    // sub_802EF5C: a side tracking the deleted navi tracks another.
    crate::kinds::obstacle::release_tracking(b, r);
    let o = b.objects.get_mut(r);
    o.phase = 4;
    o.phase_init = 0;
}

/// `sub_800A11C`: one fewer alive navi on the side, and out of the alive
/// actor lists.
pub(super) fn remove_from_alive(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance as usize;
    b.round.alive[side] = b.round.alive[side].wrapping_sub(1);
    for slot in b.round.alive_actors.iter_mut().flatten() {
        if *slot == Some(r) {
            *slot = None;
        }
    }
}

/// `sub_801746E`: two explosions (not while dimmed).
fn explode(b: &mut Battle, r: ObjectRef) {
    if b.is_dimmed() {
        return;
    }
    let o = b.objects.get_mut(r);
    o.anim = 2;
    o.related[0] = None;
    o.prevent_anim = 0;
    let pos = o.pos;
    let a = ai_mut(b, r);
    a.full_synchro_aura = None;
    a.overlay = None;
    b.sound(crate::content::SoundRole::Deleted);
    // The second call reuses whatever registers the first left: Z, but
    // list-node addresses from the allocator for X and Y
    // (objects-and-player.md §A.3).
    let look = b.roles().effect(crate::content::EffectRole::Deletion);
    crate::kinds::effect::spawn(b, pos, look, 0, 0, 0);
    crate::kinds::effect::spawn_after_spawn(b, pos.z, look, 0, 0, 0);
    let o = b.objects.get_mut(r);
    o.timer = 0x15;
    o.phase = 8;
    o.phase_init = 0;
}

/// `sub_80174BE`: fade out over 32 ticks, then the destroy state.
fn fade_out(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    o.timer = o.timer.wrapping_add(1);
    if o.timer != 0x20 {
        // Mosaic and alpha follow the timer.
        let t = (o.timer >> 1) as u8;
        let look = &mut b.objects.sprite_mut(r).look;
        look.mosaic = (t != 0).then_some(t);
        look.alpha = Some(16u8.wrapping_sub(t));
        return;
    }
    let look = &mut b.objects.sprite_mut(r).look;
    look.mosaic = None;
    look.alpha = None;
    let o = b.objects.get_mut(r);
    o.set_visible(false);
    // sub_802CDD0: the side's damage-carry record forgets this navi.
    let side = o.alliance as usize;
    if b.damage_carry[side].target == Some(r) {
        b.damage_carry[side].target = None;
    }
    death_hook(b, r);
    let o = b.objects.get_mut(r);
    o.state = state::DESTROY;
    set_action(b, r, NaviAction::Entry);
}

/// `sub_8011020`: the navi's death hook (its overlays come down).
fn death_hook(b: &mut Battle, r: ObjectRef) {
    let identity = b.objects.get(r).identity;
    super::form::navi_death_hook(b, r, identity);
}

// ---- Common reaction entry ---------------------------------------------------------

/// Whether the arena's reaction actions are EXE4's plain ones (the rule
/// `reaction_actions`).
fn plain(b: &Battle) -> bool {
    b.game_rules().reaction_actions == ReactionActions::Plain
}

/// The flag a reaction action marks the navi with besides its own: the
/// action in use, unless the reactions are plain.
fn in_use(b: &Battle) -> u32 {
    if plain(b) { 0 } else { f1::USING_ACTION }
}

/// The entry the paralysis, freeze and bubble actions share: using an
/// action, not guarding, moving, flinching or dragged, charge and state
/// bits dropped, snapped onto the destination panel (unless sliding),
/// reaction animation. (Plain, EXE4's 0x080109FA: not using an action,
/// snapped sliding or not, at its height.)
fn enter_reaction(b: &mut Battle, r: ObjectRef, anim: u8) {
    set_flag1(b, r, in_use(b));
    clear_flag1(b, r, f1::DRAG | f1::FLINCHING | f1::MOVING | f1::GUARD);
    cancel_submerged(b, r);
    ai_mut(b, r).status &= !0x20_005F;
    reset_charge(b, r);
    if plain(b) {
        snap_to_future_panel(b, r);
    } else if flag1(b, r) & f1::SLIDING == 0 {
        snap_to_future_panel(b, r);
        b.objects.get_mut(r).pos.z = 0;
    }
    let o = b.objects.get_mut(r);
    o.anim = anim;
    o.anim_loaded = 0xFF;
}

/// The end of the reaction entry: links dropped, the reaction counted
/// (`counted`; plain, the overlay link kept).
fn finish_reaction_entry(b: &mut Battle, r: ObjectRef, counted: bool) {
    b.objects.get_mut(r).related[0] = None;
    if !plain(b) {
        ai_mut(b, r).overlay = None;
    }
    if counted {
        let side = b.objects.get(r).alliance;
        b.bump_side_stat(side, 3, 1);
    }
    b.objects.get_mut(r).phase_init = 4;
}

/// Leave a reaction for the idle action.
fn end_reaction(b: &mut Battle, r: ObjectRef) {
    // (EXE6's 0x1000003F, EXE5's 0x1843F: the reactions section's.)
    let clears = b.game_rules().request_clears.paralysis.0;
    ai_mut(b, r).requests &= !(request::ATTACKS | clears);
    clear_flag1(b, r, in_use(b));
    let o = b.objects.get_mut(r);
    o.anim = 0;
    set_action(b, r, NaviAction::Idle);
}

/// `sub_80F06CE`: MegaMan's flinch and drag hook restarts the form
/// overlay.
fn reset_form_overlay(b: &mut Battle, r: ObjectRef) {
    if let Some(overlay) = b.objects.get(r).related[1] {
        super::restart_overlay(b, overlay);
    }
}

/// The hooks a player's actor record has for what it wears (the player
/// rows of the flinch and drag hook tables, `off_80EAB94` and
/// `off_80EABF8`: its identity's `overlay_hooks`), and whether it wears
/// parts of its own; a navi no player controls has its identity's too
/// (EXE5's navi rows, 0x080F256C and 0x080F25C8: Dark MegaMan's are empty);
/// the virus rows (`off_81094D0`, ...) belong to the virus AI.
fn player_hooks(b: &Battle, r: ObjectRef, table: &str) -> (crate::content::OverlayHooks, bool) {
    let a = ai(b, r);
    if a.actor_type == crate::actor::ActorType::Virus {
        panic!("the {table} of {:?} actors belongs to the virus AI", a.actor_type);
    }
    let identity = b.content.identity(a.identity);
    (identity.overlay_hooks, identity.parts.is_some())
}

/// `sub_800F3E8`: the per-form flinch hook (`off_80EAB94`): MegaMan's
/// restarts his form overlay (`sub_80F06CE`), HeatMan's his body overlay
/// (`sub_80F0700`, `sub_80C44D2`).
fn flinch_hook(b: &mut Battle, r: ObjectRef) {
    let (hooks, own_parts) = player_hooks(b, r, "flinch hook (sub_800F3E8)");
    if hooks.lets_go {
        let_go_overlay(b, r);
    }
    if !hooks.flinch {
        return;
    }
    // A navi that wears its own overlay restarts it unchecked: without
    // it, sub_80F0700 jumps into the middle of sub_80F0728 with another
    // stack frame. (EXE5's team navis' hooks test for it: `flinch_checked`.)
    if own_parts && !hooks.flinch_checked && b.objects.get(r).related[1].is_none() {
        panic!("the flinch hook of a navi without the overlay it wears jumps into sub_80F0728 (sub_80F0700)");
    }
    reset_form_overlay(b, r);
}

/// `sub_800F404`: the per-form drag hook (`off_80EABF8`: MegaMan's only).
fn drag_hook(b: &mut Battle, r: ObjectRef) {
    let hooks = player_hooks(b, r, "drag hook (sub_800F404)").0;
    if hooks.lets_go {
        let_go_overlay(b, r);
    }
    if hooks.drag {
        reset_form_overlay(b, r);
    }
}

/// `sub_800F394`: the per-form paralysis hook. EXE6's table has none;
/// EXE5's (0x080EB308) has KnightMan's.
fn paralysis_hook(b: &mut Battle, r: ObjectRef) {
    if player_hooks(b, r, "paralysis hook (sub_800F394)").0.lets_go {
        let_go_overlay(b, r);
    }
}

/// EXE5's 0x080FBA20, KnightMan's entry in the flinch, drag and paralysis
/// tables alike (the identity's `lets_go`): what the navi wears is let go
/// where an attack held it shown or hidden
/// (`sub_80C4510`: the overlay follows its wearer's visibility again), and
/// nothing without it.
fn let_go_overlay(b: &mut Battle, r: ObjectRef) {
    if let Some(overlay) = b.objects.get(r).related[1] {
        b.objects.get_mut(overlay).phase_init = 0;
    }
}

// ---- Flinch (action 3) --------------------------------------------------------------

/// Action 3, `sub_80174FE`: 24 ticks of flinching; ends paralysis,
/// freeze and bubble.
pub(super) fn flinch(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        flinch_hook(b, r);
        set_flag1(b, r, in_use(b) | f1::FLINCHING);
        clear_paralysis(b, r);
        clear_freeze(b, r);
        clear_bubble(b, r);
        cancel_submerged(b, r);
        ai_mut(b, r).status &= !0x20_005F;
        clear_flag1(b, r, f1::DRAG | f1::MOVING | f1::GUARD);
        reset_charge(b, r);
        if flag1(b, r) & f1::SLIDING == 0 {
            snap_to_future_panel(b, r);
            // (Plain, EXE4's 0x08010960: at its height.)
            if !plain(b) {
                b.objects.get_mut(r).pos.z = 0;
            }
        }
        let o = b.objects.get_mut(r);
        o.anim = 1;
        o.anim_loaded = 0xFF;
        refresh_form_overlay(b, r);
        finish_reaction_entry(b, r, true);
        b.objects.get_mut(r).timer = 0x17;
    }
    let o = b.objects.get_mut(r);
    let t = o.timer as i32 - 1;
    o.timer = t as u16;
    if t >= 0 {
        return;
    }
    clear_flag1(b, r, in_use(b) | f1::FLINCHING);
    // (EXE6's 0x1000043F, EXE5's 0x1843F: the reactions section's.)
    let clears = b.game_rules().request_clears.flinch.0;
    ai_mut(b, r).requests &= !(request::ATTACKS | clears);
    let o = b.objects.get_mut(r);
    o.anim = 0;
    set_action(b, r, NaviAction::Idle);
}

// ---- Paralysis, freeze, bubble (actions 4, 6, 7) ------------------------------------

/// Mashing buttons takes an extra tick off a status timer; returns true
/// once the status flag is gone.
fn mash(b: &mut Battle, r: ObjectRef, t: usize, flag: u32) -> bool {
    if ai(b, r).pad.pressed != 0 {
        let c = coll_mut(b, r);
        let v = c.status_timers[t] as i16 as i32 - 1;
        c.status_timers[t] = if v > 0 { v as u16 } else { 0 };
        if v <= 0 {
            clear_flag1(b, r, flag);
        }
    }
    flag1(b, r) & flag == 0
}

/// Action 4, `sub_80175B8`: paralyzed until the timer (or mashing) ends
/// it.
pub(super) fn paralysis(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        paralysis_hook(b, r);
        enter_reaction(b, r, 2);
        finish_reaction_entry(b, r, !plain(b));
    }
    if mash(b, r, timer::PARALYZE, f1::PARALYZED) {
        end_reaction(b, r);
    }
}

/// Action 6, `sub_8017688`: frozen; loses the flash and invulnerability.
pub(super) fn freeze(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        // sub_800F3B0: no per-form hook.
        coll_mut(b, r).status_timers[timer::FLASH] = 0;
        clear_invulnerable(b, r);
        b.sound(crate::content::SoundRole::Freeze);
        enter_reaction(b, r, 2);
        finish_reaction_entry(b, r, !plain(b));
    }
    if mash(b, r, timer::FREEZE, f1::FROZEN) {
        end_reaction(b, r);
    }
}

/// Action 7, `sub_8017768`: bubbled, bobbing up and down.
pub(super) fn bubble(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        // sub_800F3CC: no per-form hook.
        clear_invulnerable(b, r);
        b.sound(crate::content::SoundRole::Bubble);
        enter_reaction(b, r, 2);
        finish_reaction_entry(b, r, !plain(b));
    }
    let popped = mash(b, r, timer::BUBBLE, f1::BUBBLED);
    let t = coll(b, r).status_timers[timer::BUBBLE] as i16 as i32;
    b.objects.get_mut(r).pos.z = (b.game_rules().bubble_bob[((t >> 2) & 0x1F) as usize] as i32) << 16;
    if popped {
        b.objects.get_mut(r).pos.z = 0;
        b.sound(crate::content::SoundRole::BubblePop);
        end_reaction(b, r);
    }
}

// ---- Drag (action 5) --------------------------------------------------------------

/// Action 5, `sub_80178B6`: knocked back along the push vector, then a
/// short recovery (§H4.3).
pub(super) fn drag(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).drag_step {
        DragStep::Start => start_drag(b, r),
        DragStep::Slide => step_drag(b, r),
        DragStep::Recover => recover_from_drag(b, r),
    }
}

/// `sub_80178D4` (plain, EXE4's 0x08010ABC: not using an action, the
/// flinch's pose, the overlay link kept, at its height, counted not).
fn start_drag(b: &mut Battle, r: ObjectRef) {
    drag_hook(b, r);
    let plain = plain(b);
    set_flag1(b, r, in_use(b) | f1::DRAG);
    b.objects.get_mut(r).related[0] = None;
    if !plain {
        ai_mut(b, r).overlay = None;
    }
    // (The game's poses: EXE6's paralyzed one, EXE5's SuperArmor one.)
    let anim = b.game_rules().drag.poses.pose(flag1(b, r));
    let o = b.objects.get_mut(r);
    o.anim = anim;
    o.anim_loaded = 0xFF;
    refresh_form_overlay(b, r);
    reset_charge(b, r);
    let o = b.objects.get_mut(r);
    o.panel = o.future_panel;
    super::set_coordinates_from_panel(b, r);
    b.update_collision_panels(r);
    if !plain {
        b.objects.get_mut(r).pos.z &= !0xFFFF;
    }
    clear_flag1(b, r, f1::SLIDING | f1::FLINCHING | f1::MOVING | f1::GUARD);
    cancel_submerged(b, r);
    let fp = b.objects.get(r).future_panel;
    b.unreserve_panel(r, fp.x, fp.y);
    if !plain {
        let side = b.objects.get(r).alliance;
        b.bump_side_stat(side, 3, 1);
    }
    let v = slide_vector(b, r);
    let o = b.objects.get_mut(r);
    o.slide_dx = v.dx as u8;
    o.slide_dy = v.dy as u8;
    o.timer2 = v.tiles as u16;
    if v.tiles != 0 {
        let p = o.panel;
        let target = PanelPos { x: (p.x as i8 + v.dx) as u8, y: (p.y as i8 + v.dy) as u8 };
        if can_slide_to(b, r, target) {
            // (The arena's speed: EXE5's goes 8 pixels a tick in depth.)
            let speed = b.game_rules().slide_speed;
            let o = b.objects.get_mut(r);
            o.vel.x = v.dx as i32 * speed.x;
            o.vel.y = v.dy as i32 * speed.y;
            o.future_panel = target;
            b.reserve_panel(r, target.x, target.y);
            b.objects.get_mut(r).drag_step = DragStep::Slide;
            return;
        }
    }
    let o = b.objects.get_mut(r);
    o.timer = 0x18;
    o.drag_step = DragStep::Recover;
}

/// `sub_8017992`: move toward the destination; on arrival continue (one
/// more tile on ice) or stop.
fn step_drag(b: &mut Battle, r: ObjectRef) {
    let fp = b.objects.get(r).future_panel;
    let (tx, ty) = panel_coordinates(fp.x, fp.y);
    let o = b.objects.get_mut(r);
    let old_x = o.pos.x;
    o.pos.x = o.pos.x.wrapping_add(o.vel.x);
    let mut arrived = passed(o.pos.x, old_x, tx);
    if !arrived {
        let old_y = o.pos.y;
        o.pos.y = o.pos.y.wrapping_add(o.vel.y);
        arrived = passed(o.pos.y, old_y, ty);
    }
    if !arrived {
        let o = b.objects.get_mut(r);
        o.panel = coordinates_to_panel(o.pos.x, o.pos.y);
        b.update_collision_panels(r);
        return;
    }
    b.unreserve_panel(r, fp.x, fp.y);
    let kind = panel_kind(b, fp);
    if kind == PanelType::Ice && coll(b, r).element != 2 {
        let o = b.objects.get_mut(r);
        o.timer2 = o.timer2.wrapping_add(1);
    } else if b.game_rules().panels.types[kind as usize].stops_slides && flag1(b, r) & f1::FLOATSHOE == 0 {
        // EXE4's pitfall stops a drag (0x08010B54).
        b.objects.get_mut(r).timer2 = 0;
    }
    let o = b.objects.get_mut(r);
    let left = o.timer2 as i32 - 1;
    o.timer2 = left as u16;
    if left > 0 {
        let (dx, dy) = (o.slide_dx as i8, o.slide_dy as i8);
        let next = PanelPos { x: (fp.x as i8 + dx) as u8, y: (fp.y as i8 + dy) as u8 };
        if can_slide_to(b, r, next) {
            b.objects.get_mut(r).future_panel = next;
            b.reserve_panel(r, next.x, next.y);
            return;
        }
    }
    let o = b.objects.get_mut(r);
    o.panel = o.future_panel;
    super::set_coordinates_from_panel(b, r);
    b.update_collision_panels(r);
    let o = b.objects.get_mut(r);
    o.timer = 0x14;
    o.drag_step = DragStep::Recover;
}

/// `sub_800E6E8`: whether a step from `old` to `new` reached `target`
/// (moving left, exactly reaching it does not count).
pub(crate) fn passed(new: i32, old: i32, target: i32) -> bool {
    if new > old { target > old && target <= new } else { target > new && target <= old }
}

/// `sub_8017A38`: wait, then back to idle (or to paralysis), by the game's
/// `drag.ending`.
fn recover_from_drag(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    let t = o.timer as i32 - 1;
    o.timer = t as u16;
    if t >= 0 {
        return;
    }
    let ending = b.game_rules().drag.ending;
    if ending != DragEnding::ResumesParalysis {
        // EXE5's (0x080144CE), EXE4's (0x08010C16): the drag and its use
        // cleared, its requests, idle (EXE5's standing, EXE4's in its pose).
        clear_flag1(b, r, in_use(b) | f1::DRAG);
        let clears = b.game_rules().request_clears.drag.0;
        ai_mut(b, r).requests &= !(request::ATTACKS | clears);
        if ending == DragEnding::Stands {
            let o = b.objects.get_mut(r);
            o.anim = 0;
            o.anim_loaded = 0xFF;
        }
        return set_action(b, r, NaviAction::Idle);
    }
    if flag1(b, r) & f1::PARALYZED != 0 {
        clear_flag1(b, r, f1::DRAG);
        set_action(b, r, NaviAction::Paralysis);
        return;
    }
    clear_flag1(b, r, f1::USING_ACTION | f1::DRAG | f1::SLIDING | f1::PARALYZED);
    // (EXE6's 0x1000043F, EXE5's 0x1843F: the reactions section's.)
    let clears = b.game_rules().request_clears.drag.0;
    let a = ai_mut(b, r);
    a.requests &= !(request::ATTACKS | clears);
    a.status &= !ai_status::HEAT_TRAP;
    clear_flag2(b, r, 0x10);
    let o = b.objects.get_mut(r);
    o.slide_state = 0;
    o.anim = 0;
    o.anim_loaded = 0xFF;
    refresh_form_overlay(b, r);
    set_action(b, r, NaviAction::Idle);
}

/// `sub_800E468`: the slide vector for the slide type, none when the
/// first panel is not open.
pub(super) fn slide_vector(b: &Battle, r: ObjectRef) -> SlideVector {
    let o = b.objects.get(r);
    let front = if o.alliance == 0 { 1 } else { -1 };
    let facing = |v: SlideVector| SlideVector { dx: v.dx * front, ..v };
    let v = match o.slide_type {
        0 => SlideVector::NONE,
        1 => {
            let reading = &b.game_rules().push_reading;
            let none = reading.bits as usize;
            match reading.reads {
                // sub_800E548 (EXE4's 0x0800ACAA): the first of the final
                // modifier's bits from bit 2, toward the front; a shift bit
                // (EXE6's 0x80) picks rows further on.
                PushSource::Final => {
                    let hm = coll(b, r).hit_mod_final;
                    let off = match reading.shift {
                        Some(s) if hm & s.bit != 0 => s.rows as usize,
                        _ => 0,
                    };
                    facing(b.game_rules().push_vectors[reading.first(hm) + off])
                }
                // EXE5's 0x0800C9D8: the first of the unflipped hitters'
                // modifier's bits, else of the flipped ones' with the
                // direction reversed (none past them).
                PushSource::ByHitterFlip => {
                    let [from0, from1] = coll(b, r).hit_mod_by_side;
                    let (i, sign) = match reading.first(from0) {
                        i if i == none => (reading.first(from1), -1),
                        i => (i, 1),
                    };
                    let v = b.game_rules().push_vectors[i];
                    SlideVector { dx: v.dx * front * sign, ..v }
                }
            }
        }
        2 => match b.game_rules().ice {
            IceRule::Slide(rows) => facing(*rows.get(coll(b, r).direction as usize).expect("ice slide direction")),
            // (Ice that pushes starts no ice slide: its push is slide type 1.)
            IceRule::Push(_) => SlideVector::NONE,
        },
        3 => {
            let kind = panel_kind(b, o.panel);
            // EXE5's metal (0x0800C8A8): the steps its slide tries by the
            // direction of the move, the first the navi can slide to.
            if let Some(slide) = b.game_rules().panels.types[kind as usize].slide {
                let tries = slide.tries.get(coll(b, r).direction as usize).copied().unwrap_or_default();
                return tries
                    .into_iter()
                    .flatten()
                    .map(|(dx, dy)| SlideVector { dx: dx * front, dy, tiles: 1 })
                    .find(|v| can_slide_to(b, r, PanelPos { x: (o.panel.x as i8 + v.dx) as u8, y: (o.panel.y as i8 + v.dy) as u8 }))
                    .unwrap_or(SlideVector::NONE);
            }
            b.game_rules().panels.road_slide(kind).unwrap_or(SlideVector::NONE)
        }
        t => panic!("slide type {t} reads past its table"),
    };
    let p = o.panel;
    let first = PanelPos { x: (p.x as i8 + v.dx) as u8, y: (p.y as i8 + v.dy) as u8 };
    if can_slide_to(b, r, first) { v } else { SlideVector::NONE }
}

/// `sub_800E5AC`: a slide may enter (x, y): like a step, but the floor
/// rule depends only on AirShoes.
pub(super) fn can_slide_to(b: &Battle, r: ObjectRef, p: PanelPos) -> bool {
    if !field::is_valid(p.x, p.y) {
        return false;
    }
    let airshoes = flag1(b, r) & f1::AIRSHOE != 0;
    b.field.meets(p.x, p.y, b.game_rules().panels.step.get(airshoes, b.objects.get(r).alliance))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::battle_flags;
    use crate::content::{Content, testing};
    use std::sync::Arc;

    /// A fight on the test content whose arena reads pushes from `reads`
    /// (its push rules else the test content's, EXE6's): the battle and
    /// its two navis (side 0 at (2, 2), side 1 at (5, 2)).
    fn fight(reads: PushSource) -> (Battle, [ObjectRef; 2]) {
        fight_with(|r| r.push_reading.reads = reads)
    }

    /// The same, the arena's rules changed by `change`.
    fn fight_with(change: impl FnOnce(&mut crate::content::Rules)) -> (Battle, [ObjectRef; 2]) {
        let mut c: Content = testing::build();
        c.define().unwrap_or_else(|e| panic!("{e}"));
        change(c.rules_mut());
        let c = Arc::new(c);
        let mut setup = testing::round_setup(testing::LINK_BATTLE, testing::megaman_on(&c));
        crate::content::testing::on(&mut setup, &c);
        let mut b = Battle::new(setup, c);
        b.spawn_actors();
        b.run_objects();
        b.round.flags |= battle_flags::FIGHTING;
        let players = [b.player(0).unwrap(), b.player(1).unwrap()];
        (b, players)
    }

    /// The push of side 1's navi after hits from unflipped hitters with
    /// modifier `from0` and from flipped ones with `from1` (EXE6 reads them
    /// together).
    fn push(reads: PushSource, from0: u8, from1: u8) -> SlideVector {
        let (mut b, [_, r]) = fight(reads);
        let c = coll_mut(&mut b, r);
        c.hit_mod_final = from0 | from1;
        c.hit_mod_by_side = [from0, from1];
        b.objects.get_mut(r).slide_type = 1;
        slide_vector(&b, r)
    }

    /// docs/design/exe5-map.md §15.3 item 2: EXE5's push reads the unflipped
    /// hitters' modifier toward the navi's front, else the flipped ones'
    /// the other way; EXE6's reads them together, toward the front.
    #[test]
    fn exe5_pushes_by_the_hitters_flip() {
        // Bit 2: row 0, six panels along +x times the navi's front (side
        // 1's is -1).
        let left = SlideVector { dx: -1, dy: 0, tiles: 6 };
        let right = SlideVector { dx: 1, dy: 0, tiles: 6 };
        assert_eq!(push(PushSource::Final, 0x04, 0), left);
        assert_eq!(push(PushSource::Final, 0, 0x04), left);
        assert_eq!(push(PushSource::ByHitterFlip, 0x04, 0), left);
        assert_eq!(push(PushSource::ByHitterFlip, 0, 0x04), right, "a flipped hitter's hit pushes it the other way");
        // The unflipped hitters' hits come first.
        assert_eq!(push(PushSource::ByHitterFlip, 0x04, 0x08), left);
        // EXE6's 0x80 picks the vertical rows; EXE5 has no such bit.
        let v = push(PushSource::Final, 0x84, 0);
        assert_eq!((v.dx, v.dy), (0, -1));
    }

    /// EXE4's push (0x0800ACAA) reads six bits of the final modifier, from
    /// bit 2, and no shift: 0x40 and 0x80 are its pushes up and down.
    #[test]
    fn a_push_reads_as_many_bits_as_the_rules_say() {
        let rows = [(1, 0, 6), (-1, 0, 6), (1, 0, 1), (-1, 0, 1), (0, -1, 1), (0, 1, 1), (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0)];
        let (mut b, [_, r]) = fight_with(|rules| {
            rules.push_reading.bits = 6;
            rules.push_reading.shift = None;
            rules.push_vectors = rows.map(|(dx, dy, tiles)| SlideVector { dx, dy, tiles });
        });
        b.objects.get_mut(r).slide_type = 1;
        let mut at = |hm: u8| {
            coll_mut(&mut b, r).hit_mod_final = hm;
            let v = slide_vector(&b, r);
            (v.dx, v.dy, v.tiles)
        };
        // (Side 1's front is -1.)
        assert_eq!(at(0x04), (-1, 0, 6));
        assert_eq!(at(0x40), (0, -1, 1));
        assert_eq!(at(0x80), (0, 1, 1));
        assert_eq!(at(0x01), (0, 0, 0), "no bit set: the row past them");
    }

    /// docs/design/exe4-map.md §18 item 33: plain reaction actions (EXE4's,
    /// with its drag: the flinch's pose, kept to its end) mark no action in
    /// use, count no reaction for a paralysis or a drag, keep a dragged
    /// navi's paralysis to idle in the flinch's pose; marked ones (the test
    /// content's, EXE6's) do each.
    #[test]
    fn plain_reaction_actions_mark_and_count_nothing_more() {
        let run = |plain: bool| {
            let (mut b, [_, r]) = fight_with(|rules| {
                rules.reaction_actions = if plain { ReactionActions::Plain } else { ReactionActions::Marked };
                if plain {
                    rules.drag = crate::content::DragRule {
                        poses: crate::content::DragPoses { paralyzed: None, super_armor: None, otherwise: 1 },
                        ending: DragEnding::KeepsPose,
                    };
                }
            });
            let side = b.objects.get(r).alliance;
            let counted = |b: &Battle| b.side_stats[side as usize][3];
            let before = counted(&b);
            // A paralyzed navi dragged no panel away.
            set_flag1(&mut b, r, f1::PARALYZED);
            coll_mut(&mut b, r).hit_mod_final = 0;
            b.objects.get_mut(r).slide_type = 1;
            start_drag(&mut b, r);
            let drag = (b.objects.get(r).anim, flag1(&b, r) & f1::USING_ACTION != 0, counted(&b) - before);
            // Its end.
            b.objects.get_mut(r).timer = 0;
            recover_from_drag(&mut b, r);
            let end = (super::super::navi_action(&b, r), flag1(&b, r) & f1::PARALYZED != 0);
            (drag, end)
        };
        assert_eq!(run(false), ((2, true, 1), (NaviAction::Paralysis, true)));
        assert_eq!(run(true), ((1, false, 0), (NaviAction::Idle, true)));
    }

    /// A drag's pose and end by the game's `drag` (the status section's):
    /// EXE6's paralyzed pose, then SuperArmor's, and a paralysis that
    /// outlasts the drag goes on; EXE5's SuperArmor pose alone, and its end
    /// stands the navi up whatever the paralysis, which it keeps.
    #[test]
    fn a_drags_pose_and_end_are_the_games() {
        use crate::content::{DragPoses, DragRule};
        let exe5 = DragRule { poses: DragPoses { paralyzed: None, super_armor: Some(0), otherwise: 1 }, ending: DragEnding::Stands };
        let run = |rule: Option<DragRule>, status: u32| {
            let (mut b, [_, r]) = fight_with(|rules| {
                if let Some(rule) = rule {
                    rules.drag = rule;
                }
            });
            set_flag1(&mut b, r, status);
            coll_mut(&mut b, r).hit_mod_final = 0;
            b.objects.get_mut(r).slide_type = 1;
            start_drag(&mut b, r);
            let pose = b.objects.get(r).anim;
            b.objects.get_mut(r).anim = 7;
            b.objects.get_mut(r).timer = 0;
            recover_from_drag(&mut b, r);
            let o = b.objects.get(r);
            (pose, super::super::navi_action(&b, r), o.anim, flag1(&b, r) & (f1::PARALYZED | f1::DRAG | f1::USING_ACTION))
        };
        let (p, sa) = (f1::PARALYZED, f1::SUPERARMOR);
        // The test content's, EXE6's.
        assert_eq!(run(None, p | sa), (2, NaviAction::Paralysis, 7, p | f1::USING_ACTION), "the paralysis goes on, in use");
        assert_eq!(run(None, sa).0, 0);
        assert_eq!(run(None, 0), (1, NaviAction::Idle, 0, 0));
        // EXE5's.
        assert_eq!(run(Some(exe5), p | sa), (0, NaviAction::Idle, 0, p));
        assert_eq!(run(Some(exe5), p), (1, NaviAction::Idle, 0, p));
        assert_eq!(run(Some(exe5), 0), (1, NaviAction::Idle, 0, 0));
    }

    /// docs/design/exe5-map.md §15.2: lava (the test content's burns for 50,
    /// as EXE5's does) burns a grounded navi not of fire in fire, turning
    /// normal, with its burn's spark; a navi of fire it leaves be.
    #[test]
    fn lava_burns_a_grounded_navi() {
        let (mut b, [_, r]) = fight(PushSource::Final);
        b.set_panel_type(5, 2, PanelType::Lava);
        let sparks = |b: &Battle| b.objects.in_order().filter(|&o| b.local_kind_key(o).contains("spark")).count();
        let before = sparks(&b);
        crate::kinds::common::panel_burn(&mut b, r);
        assert_eq!(coll(&b, r).acc.element_damage[1], 50);
        assert_eq!(coll(&b, r).hit_mod_final & 3, 3, "a hit");
        assert_eq!(b.field.panels[2][5].kind, PanelType::Normal);
        assert_eq!(sparks(&b), before + 1, "its burn shows");
        // A navi of fire stands on it.
        b.set_panel_type(5, 2, PanelType::Lava);
        coll_mut(&mut b, r).element = 1;
        crate::kinds::common::panel_burn(&mut b, r);
        assert_eq!(b.field.panels[2][5].kind, PanelType::Lava);
    }

    /// EXE5's metal (0x0800C8A8, the tables at 0x0800C920 and 0x0800C9C0):
    /// its slide tries steps by the direction of the move, the first the
    /// navi can slide to: after a move up, forward (side 1's front is -x),
    /// then up; after a move forward, down first.
    #[test]
    fn metal_slides_by_the_direction_of_the_move() {
        let (mut b, [_, r]) = fight(PushSource::Final);
        b.set_panel_type(5, 2, PanelType::Metal);
        b.objects.get_mut(r).slide_type = 3;
        coll_mut(&mut b, r).direction = 1;
        assert_eq!(slide_vector(&b, r), SlideVector { dx: -1, dy: 0, tiles: 1 });
        // Forward is the other side's: up is next.
        b.set_panel_alliance(4, 2, 0);
        assert_eq!(slide_vector(&b, r), SlideVector { dx: 0, dy: -1, tiles: 1 });
        coll_mut(&mut b, r).direction = 4;
        assert_eq!(slide_vector(&b, r), SlideVector { dx: 0, dy: 1, tiles: 1 });
    }

    /// docs/design/exe4-map.md §18 item 12: a drag that reaches a panel
    /// whose type stops slides (EXE4's pitfall, 0x08010B54) stops there,
    /// unless the navi floats.
    #[test]
    fn a_pitfall_stops_a_drag_unless_the_navi_floats() {
        let run = |floats: bool| {
            let (mut b, [_, r]) = fight_with(|r| r.panels.types[PanelType::Pitfall as usize].stops_slides = true);
            b.set_panel_type(5, 2, PanelType::Pitfall);
            if floats {
                set_flag1(&mut b, r, f1::FLOATSHOE);
            }
            let (from, to) = (panel_coordinates(4, 2), panel_coordinates(5, 2));
            let o = b.objects.get_mut(r);
            o.panel = PanelPos { x: 4, y: 2 };
            o.future_panel = PanelPos { x: 5, y: 2 };
            (o.pos.x, o.pos.y) = from;
            (o.vel.x, o.vel.y) = (to.0 - from.0, 0);
            (o.slide_dx, o.slide_dy) = (1, 0);
            o.timer2 = 3;
            o.drag_step = DragStep::Slide;
            step_drag(&mut b, r);
            let o = b.objects.get(r);
            (o.drag_step, o.future_panel.x)
        };
        assert_eq!(run(false), (DragStep::Recover, 5), "stopped on the pitfall");
        assert_eq!(run(true), (DragStep::Slide, 6), "floating over it");
    }

    /// docs/design/exe5-map.md §15.3 item 17: a drag goes at the arena's
    /// speed (EXE6's 6 pixels a tick in depth, EXE5's 8).
    #[test]
    fn a_drag_goes_at_the_arenas_speed() {
        let speed = |y: i32| {
            let mut c: Content = testing::build();
            c.define().unwrap_or_else(|e| panic!("{e}"));
            {
            let rules = c.rules_mut();
                rules.slide_speed.y = y;
            }
            let c = Arc::new(c);
            let mut setup = testing::round_setup(testing::LINK_BATTLE, testing::megaman_on(&c));
            crate::content::testing::on(&mut setup, &c);
            let mut b = Battle::new(setup, c);
            b.spawn_actors();
            b.run_objects();
            b.round.flags |= battle_flags::FIGHTING;
            let r = b.player(1).unwrap();
            // A push up a panel (EXE6's 0x80 rows: bit 2, row 5).
            coll_mut(&mut b, r).hit_mod_final = 0x84;
            b.objects.get_mut(r).slide_type = 1;
            start_drag(&mut b, r);
            b.objects.get(r).vel.y
        };
        assert_eq!(speed(0x6_0000), -0x6_0000);
        assert_eq!(speed(0x8_0000), -0x8_0000);
    }

    /// docs/design/exe5-map.md §15.3 item 6: the mood rises to 254 at most,
    /// falls to 1 at least, and 0 (and, rising, 0xFF) stays.
    #[test]
    fn the_mood_rises_and_falls_within_its_bounds() {
        let (mut b, _) = fight(PushSource::Final);
        let mood = |b: &Battle| b.stats[0].mood;
        b.stats[0].mood = 250;
        super::super::gain_mood(&mut b, 0, 10);
        assert_eq!(mood(&b), 254);
        super::super::lose_mood(&mut b, 0, 300);
        assert_eq!(mood(&b), 1);
        b.stats[0].mood = 0xFF;
        super::super::gain_mood(&mut b, 0, 1);
        assert_eq!(mood(&b), 0xFF);
        b.stats[0].mood = 0;
        super::super::gain_mood(&mut b, 0, 1);
        super::super::lose_mood(&mut b, 0, 1);
        assert_eq!(mood(&b), 0);
    }

    /// docs/design/exe5-map.md §15.10: EXE5's emotions go by its order (a mood
    /// under 65 worried, anger before a mood of 0), and its setter leaves a
    /// mood of 0; EXE6's by its own. Each is a set of the status section's
    /// `emotion` rules, as the game's content states them.
    #[test]
    fn each_games_emotions_go_by_its_rules() {
        use super::super::{emotion_name, set_mood};
        use crate::content::{AngerEnd, EmotionRole, EmotionRules, EmotionWhen, MoodHeld};
        let when = |w: EmotionWhen| vec![w];
        let case = |name: &str, when: Vec<EmotionWhen>| (name.to_string(), when);
        let roles = || {
            [("full_synchro", EmotionRole::FullSynchro), ("angry", EmotionRole::Angry), ("worn_out", EmotionRole::WornOut)]
                .map(|(n, r)| (n.to_string(), r))
                .to_vec()
        };
        let mut exe6_roles = roles();
        exe6_roles.push(("tired".to_string(), EmotionRole::Tired));
        let exe6 = EmotionRules::new(
            MoodHeld::TiredOrExhausted,
            AngerEnd::ResetsMood,
            vec![
                case("worn_out", vec![EmotionWhen { mood: Some(0), ..Default::default() }, EmotionWhen { exhausted: Some(true), ..Default::default() }]),
                case("angry", when(EmotionWhen { angry: Some(true), ..Default::default() })),
                case("tired", when(EmotionWhen { tired: Some(true), ..Default::default() })),
                case("full_synchro", when(EmotionWhen { mood: Some(0xFF), ..Default::default() })),
                case("normal", vec![]),
            ],
            exe6_roles,
        )
        .unwrap();
        let exe5 = EmotionRules::new(
            MoodHeld::AtZero,
            AngerEnd::ThroughSetter,
            vec![
                case("full_synchro", when(EmotionWhen { battle_mode: Some(1), mood: Some(0xFF), ..Default::default() })),
                case("normal", vec![EmotionWhen { battle_mode: Some(1), ..Default::default() }, EmotionWhen { in_form: Some(true), ..Default::default() }]),
                case("angry", when(EmotionWhen { angry: Some(true), ..Default::default() })),
                case("worn_out", when(EmotionWhen { mood: Some(0), ..Default::default() })),
                case("full_synchro", when(EmotionWhen { mood: Some(0xFF), ..Default::default() })),
                case("worried", when(EmotionWhen { mood_below: Some(65), ..Default::default() })),
                case("normal", vec![]),
            ],
            roles(),
        )
        .unwrap();
        assert_eq!(testing::rules().emotion, exe6, "the test content plays as EXE6 does");
        let emotions = |which: EmotionRules| {
            let mut c: Content = testing::build();
            c.define().unwrap_or_else(|e| panic!("{e}"));
            {
                let rules = c.rules_mut();
                rules.emotion = which;
            }
            let c = Arc::new(c);
            let mut setup = testing::round_setup(testing::LINK_BATTLE, testing::megaman_on(&c));
            crate::content::testing::on(&mut setup, &c);
            let mut b = Battle::new(setup, c);
            b.spawn_actors();
            b.run_objects();
            let r = b.player(0).unwrap();
            let mut seen = Vec::new();
            for mood in [0x80, 40, 0xFF, 0] {
                b.stats[0].mood = mood;
                seen.push(emotion_name(&b, 0).to_string());
            }
            // Angry with a mood of 0.
            ai_mut(&mut b, r).anger = 600;
            seen.push(emotion_name(&b, 0).to_string());
            // Anger's end, from a mood of 0.
            super::super::status::end_anger(&mut b, r);
            let calmed = b.stats[0].mood;
            b.stats[0].mood = 0;
            // The setter, from a mood of 0; then with the navi held tired
            // (a mood of 0x80), and its emotion.
            set_mood(&mut b, 0, 0xFF);
            let set = b.stats[0].mood;
            b.stats[0].mood = 0x80;
            ai_mut(&mut b, r).tired = true;
            set_mood(&mut b, 0, 0xFF);
            seen.push(emotion_name(&b, 0).to_string());
            (seen, calmed, set, b.stats[0].mood)
        };
        let names = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(emotions(exe6), (names(&["normal", "normal", "full_synchro", "worn_out", "worn_out", "tired"]), 0x80, 0xFF, 0x80));
        assert_eq!(emotions(exe5), (names(&["normal", "worried", "full_synchro", "worn_out", "angry", "full_synchro"]), 0, 0, 0xFF));
    }
}
