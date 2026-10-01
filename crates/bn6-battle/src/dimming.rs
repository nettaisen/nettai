//! The dimming service of dimming chips: the per-side dimming records
//! (`byte_203CF00`, one 0x50-byte record per side) and the phases every
//! dimming controller object goes through: start the dimming (battle
//! flag 4: everything but the allowed objects stands still), dim the
//! screen, show the telop, run the chip's effect (the controller's own),
//! brighten the screen, and end the dimming. A cut-in (the other
//! side's dimming chip used during the telop) makes the controllers wait
//! on each other. See docs/engine/chips.md §3.6.

use crate::battle::{Battle, FadeMode, battle_flags};
use crate::content::{BannerId, ChipId};
use bn6_content_api::ChipHandle;
use crate::hud::BannerStatus;
use crate::kinds::common::{self, Progress};
use crate::object::{ObjectRef, state};

/// Where a side's dimming is (record +1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DimmingState {
    #[default]
    Idle = 0,
    /// A controller was registered.
    Registered = 1,
    /// Its name is (about to be) shown.
    ShowingName = 2,
    /// The telop was shown, but the other side's cut-in runs first.
    Waiting = 3,
    /// The chip's effect runs.
    Running = 4,
    /// Done; the dimming ends once the other side is done too.
    Ending = 5,
}

/// A side's dimming record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DimmingRecord {
    /// +0: the side whose dimming is current (both records are written
    /// together; a cut-in takes it over).
    pub owner: u8,
    pub state: DimmingState,
    /// +2: the chip can't be cut in on (chips 0x170 and up).
    pub no_cut_in: bool,
    /// +3: the side that started the dimming (both records).
    pub initiator: u8,
    /// +8: the side's controller object.
    pub controller: Option<ObjectRef>,
    /// +0xC: the navi that used the chip.
    pub user: Option<ObjectRef>,
}

/// The controllers' screen fades step by 4: from a clear screen the dim
/// takes 16 ticks (from an already dimmed one, after a cut-in, 1), and the
/// undim 17.
const FADE_SPEED: u8 = 4;

/// The telops: the local player's, and the other player's.
pub(crate) const LOCAL_TELOP: BannerId = BannerId(0x4C);
pub(crate) const REMOTE_TELOP: BannerId = BannerId(0x50);

/// Chips from this id on can't be cut in on.
const FIRST_NO_CUT_IN: ChipId = 0x170;

/// What every controller knows about its chip (object +0x30 / +0x32): for
/// the name the HUD shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DimmingChip {
    /// None: the attack's chip field held none (the game's 0).
    pub chip: Option<ChipHandle>,
    /// The Atk+ / cross bonus, shown with the name for damaging chips.
    pub bonus: u16,
}

impl Battle {
    fn dimming(&mut self, side: u8) -> &mut DimmingRecord {
        &mut self.dimming[side as usize]
    }

    /// `side` starts a dimming for `chip` with `controller`, used by `user`
    /// (action 0x15's and 0x1B's registration: `sub_800BF16` with the
    /// chip's cut-in rule).
    pub(crate) fn register_dimming(&mut self, side: u8, chip: Option<ChipHandle>, controller: ObjectRef, user: ObjectRef) {
        let no_cut_in = self.chip_number(chip).is_some_and(|n| n >= FIRST_NO_CUT_IN);
        self.start_dimming(side, no_cut_in, Some(controller), user);
    }

    /// `sub_800BF16`: `side` starts a dimming with `controller` (none if
    /// its spawn failed: the record waits for nothing), used by `user`;
    /// `no_cut_in`: the other side can't cut in on it. Its previous
    /// controller, if any, is told to end.
    pub(crate) fn start_dimming(&mut self, side: u8, no_cut_in: bool, controller: Option<ObjectRef>, user: ObjectRef) {
        for r in &mut self.dimming {
            r.initiator = side;
        }
        self.take_over_dimming(side, no_cut_in, controller, user);
    }

    /// `loc_800BF30(side, 0, controller)`: `side` cuts in on the other
    /// side's dimming (`sub_8017AB4`) with `controller` (none if its spawn
    /// failed), used by `user`: like `start_dimming`, but the initiator
    /// stays the other side, so the dimming ends when theirs does, and
    /// the cut-in chip can itself be cut in on.
    pub(crate) fn cut_in_dimming(&mut self, side: u8, controller: Option<ObjectRef>, user: ObjectRef) {
        self.take_over_dimming(side, false, controller, user);
    }

    /// `loc_800BF32`: `side` owns the dimming now; its record takes the
    /// controller.
    fn take_over_dimming(&mut self, side: u8, no_cut_in: bool, controller: Option<ObjectRef>, user: ObjectRef) {
        // sub_800B8AC
        for r in &mut self.dimming {
            r.owner = side;
        }
        if let Some(old) = self.dimming(side).controller {
            end_controller_now(self, old);
        }
        let rec = self.dimming(side);
        rec.no_cut_in = no_cut_in;
        rec.controller = controller;
        rec.user = Some(user);
        rec.state = DimmingState::Registered;
    }

    /// `sub_800B89C`: a side's dimming is over.
    fn clear_dimming(&mut self, side: u8) {
        let rec = self.dimming(side);
        rec.state = DimmingState::Idle;
        rec.controller = None;
    }
}

/// The cut-in flash: effect #0 look 0x1E, 120 pixels up, below the middle
/// of the other side's area.
const CUT_IN_FLASH: u8 = 0x1E;
const CUT_IN_FLASH_Z: i32 = 0x78 << 16;
const CUT_IN_SOUND: crate::sound::SoundId = crate::sound::SoundId(0xA5);

/// `sub_800B8EE(side)`: `side` cut in: a flash at panel (2 + 3 · the other
/// side, 4) and its sound.
pub(crate) fn cut_in_flash(b: &mut Battle, side: u8) {
    let (x, y) = crate::kinds::player::panel_coordinates((side ^ 1) * 3 + 2, 4);
    crate::kinds::effect::spawn(b, crate::object::Vec3 { x, y, z: CUT_IN_FLASH_Z }, CUT_IN_FLASH, 0, 0, 0);
    b.play_sound(CUT_IN_SOUND);
}

/// Kill a controller: it frees itself at its next update without its end
/// logic (state 8 with the phase initialized).
fn end_controller_now(b: &mut Battle, r: ObjectRef) {
    common::set_progress(b, r, Progress { phase_init: 4, ..Progress::DESTROY });
}

/// Idle, or already done: the other side lets a controller go on.
fn out_of_the_way(state: DimmingState) -> bool {
    matches!(state, DimmingState::Idle | DimmingState::Ending)
}

/// `object_timefreezeBegin` (the controller's init) starts the dimming if
/// its side started it.
pub fn begin(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance;
    // (The local player's HUD hides the custom gauge.)
    if b.dimming[side as usize].initiator == side {
        b.set_flags(battle_flags::DIMMED);
    }
    // (dword_200F3B8[side] = 0: written, never read.)
    b.objects.get_mut(r).state = state::UPDATE;
}

/// Step to the controller's next action (actions are 4 apart).
fn advance(b: &mut Battle, r: ObjectRef, actions: u8) {
    let a = b.objects.get(r).action;
    common::set_action(b, r, a + 4 * actions);
}

/// `object_dimScreen`: dim the screen; on to the next action when done.
/// The timer counts the ticks.
pub fn dim_screen(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        b.fade.start(FadeMode::Dim, FADE_SPEED);
        let o = b.objects.get_mut(r);
        o.timer = 0;
        o.phase_init = 4;
    }
    let o = b.objects.get_mut(r);
    o.timer = o.timer.wrapping_add(1);
    if !b.fade.active() {
        advance(b, r, 1);
    }
}

/// `object_drawChipName`: once the other side isn't mid-dimming, show the
/// telop; after it, wait for a cut-in to finish, then run the
/// effect (the next action), or skip it if the user was deleted.
pub fn show_telop(b: &mut Battle, r: ObjectRef) {
    telop(b, r, true);
}

/// `sub_800BBA8`: a hidden chip's telop (the trap chips: the other player
/// sees "???", and the user too for some); unlike `show_telop`, the
/// effect is skipped whenever the user was deleted.
pub fn show_hidden_telop(b: &mut Battle, r: ObjectRef) {
    telop(b, r, false);
}

fn telop(b: &mut Battle, r: ObjectRef, no_cut_in_runs: bool) {
    let side = b.objects.get(r).alliance;
    let other = side ^ 1;
    if b.objects.get(r).phase_init == 0 {
        b.dimming(side).state = DimmingState::ShowingName;
        if !matches!(b.dimming[other as usize].state, DimmingState::Waiting | DimmingState::Idle) {
            return;
        }
        // (The HUD's other parts hide.) The telop: the chip's name, with its
        // damage and bonus for damaging chips.
        let banner = if b.is_remote(side) { REMOTE_TELOP } else { LOCAL_TELOP };
        b.start_banner(banner);
        b.play_sound(crate::sound::SoundId(0x173));
        b.objects.get_mut(r).phase_init = 4;
        return;
    }
    if b.banner.status() != BannerStatus::Done {
        return;
    }
    // sub_800B8C2: a cut-in took the dimming over.
    if b.dimming[side as usize].owner != side && !out_of_the_way(b.dimming[other as usize].state) {
        b.dimming(side).state = DimmingState::Waiting;
        return;
    }
    b.dimming(side).state = DimmingState::Running;
    let rec = b.dimming[side as usize];
    let user_alive = rec.user.is_some_and(|u| b.objects.get(u).hp != 0);
    advance(b, r, if (no_cut_in_runs && rec.no_cut_in) || user_alive { 1 } else { 2 });
}

/// Navi chips (0xDD..=0x118) that AntiNavi turns back.
fn is_navi_chip(b: &Battle, chip: Option<ChipHandle>) -> bool {
    b.chip_number(chip).is_some_and(|n| (0xDD..=0x118).contains(&n))
}

/// AntiNavi, the defensive chip that turns a navi chip around.
const ANTI_NAVI: ChipId = 0xBA;

/// AntiNavi (chip 0xBA) is the other side's defensive chip.
fn anti_navi_waits(b: &Battle, side: u8) -> bool {
    b.chip_number(b.linked[(side ^ 1) as usize].chip) == Some(ANTI_NAVI)
}

/// How long the sparkle shows before AntiNavi's telop, in ticks after the
/// first.
const ANTI_NAVI_WAIT: u16 = 0x1E;
/// `sub_800ABC6`'s sparkle: effect #0 look 0x46, 16 pixels down the field
/// and 32 up from the panel's center, with its sound.
const SPARKLE: u8 = 0x46;
const SPARKLE_DY: i32 = 0x10_0000;
const SPARKLE_Z: i32 = 0x20_0000;
const SPARKLE_SOUND: crate::sound::SoundId = crate::sound::SoundId(0xA5);
/// The telop's sound.
const TELOP_SOUND: crate::sound::SoundId = crate::sound::SoundId(0x173);

/// `sub_800BDB2` (a navi chip's action after the dim; `off_800BDC4` by
/// phase): on to the name, unless the other side's AntiNavi turns the
/// chip around: a sparkle on the controller's panel, 31 ticks, AntiNavi's
/// telop, then the controller changes sides and runs for AntiNavi's user.
/// See docs/engine/dimming-chips.md §2.
pub fn check_anti_navi(b: &mut Battle, r: ObjectRef, chip: Option<ChipHandle>) {
    match b.objects.get(r).phase {
        0 => anti_navi_check(b, r, chip),
        4 => anti_navi_wait(b, r),
        8 => anti_navi_turn(b, r),
        p => panic!("navi chip controller phase {p:#x} reads past sub_800BDB2's table (off_800BDC4)"),
    }
}

/// Set the controller's phase, not yet entered.
fn set_phase(b: &mut Battle, r: ObjectRef, phase: u8) {
    let o = b.objects.get_mut(r);
    o.phase = phase;
    o.phase_init = 0;
}

/// `sub_800BDD0`: the other side's AntiNavi springs on a navi chip (the
/// side's dimming runs its effect from now on; a sparkle on the
/// controller's panel), or the name comes next.
fn anti_navi_check(b: &mut Battle, r: ObjectRef, chip: Option<ChipHandle>) {
    let side = b.objects.get(r).alliance;
    if !(is_navi_chip(b, chip) && anti_navi_waits(b, side)) {
        return advance(b, r, 1);
    }
    b.dimming(side).state = DimmingState::Running;
    let p = b.objects.get(r).panel;
    // sub_800ABC6: facing the local side's way (presentation).
    let (x, y) = crate::kinds::player::panel_coordinates(p.x, p.y);
    let local = b.round.local_side;
    crate::kinds::effect::spawn(b, crate::object::Vec3 { x, y: y + SPARKLE_DY, z: SPARKLE_Z }, SPARKLE, local, 0, 0);
    b.play_sound(SPARKLE_SOUND);
    set_phase(b, r, 4);
}

/// `sub_800BE0C`: 31 ticks after the first.
fn anti_navi_wait(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    if o.phase_init == 0 {
        o.timer = ANTI_NAVI_WAIT;
        o.phase_init = 4;
        return;
    }
    let left = o.timer as i32 - 1;
    o.timer = left as u16;
    if left < 0 {
        set_phase(b, r, 8);
    }
}

/// `sub_800BE2C`: AntiNavi's telop (as its user's side sees it, without
/// damage), then the chip changes sides: the side's dimming is over, the
/// controller takes the other side, which now owns and started the
/// dimming (its own controller, if any, is told to end), with AntiNavi's
/// user as the navi chip's user (on its panel), and AntiNavi is spent.
/// The name follows, now the other side's.
fn anti_navi_turn(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance;
    if b.objects.get(r).phase_init == 0 {
        let banner = if b.is_remote(side ^ 1) { REMOTE_TELOP } else { LOCAL_TELOP };
        b.start_banner(banner);
        b.play_sound(TELOP_SOUND);
        b.objects.get_mut(r).phase_init = 4;
        return;
    }
    if b.banner.status() != BannerStatus::Done {
        return;
    }
    // sub_800B89C: the side's dimming is over (its user stays recorded).
    b.clear_dimming(side);
    let taker = side ^ 1;
    b.objects.get_mut(r).alliance = taker;
    // sub_800B8AC
    for rec in &mut b.dimming {
        rec.owner = taker;
    }
    b.dimming(taker).state = DimmingState::Running;
    if let Some(c) = b.dimming[taker as usize].controller {
        end_controller_now(b, c);
    }
    for rec in &mut b.dimming {
        rec.initiator = taker;
    }
    // sub_802CE78: AntiNavi's record's user.
    if let Some(owner) = b.linked[taker as usize].owner {
        let panel = b.objects.get(owner).panel;
        let o = b.objects.get_mut(r);
        o.panel = panel;
        o.related[0] = Some(owner);
    }
    // sub_802CEA6
    b.clear_linked(taker);
    advance(b, r, 1);
}

/// `sub_800BA8A`: a navi chip's name, like `show_telop`, but the
/// effect runs whether or not the chip can be cut in on, and is skipped
/// only if the user was deleted.
pub fn show_navi_telop(b: &mut Battle, r: ObjectRef, chip: Option<ChipHandle>) {
    let side = b.objects.get(r).alliance;
    let other = side ^ 1;
    if b.objects.get(r).phase_init == 0 {
        if b.dimming[side as usize].state != DimmingState::Running {
            b.dimming(side).state = DimmingState::ShowingName;
            if !matches!(b.dimming[other as usize].state, DimmingState::Waiting | DimmingState::Idle) {
                return;
            }
        }
        let banner = if b.is_remote(side) { REMOTE_TELOP } else { LOCAL_TELOP };
        b.start_banner(banner);
        b.play_sound(TELOP_SOUND);
        b.objects.get_mut(r).phase_init = 4;
        return;
    }
    if b.banner.status() != BannerStatus::Done {
        return;
    }
    if b.dimming[side as usize].owner != side && !out_of_the_way(b.dimming[other as usize].state) {
        b.dimming(side).state = DimmingState::Waiting;
        return;
    }
    b.dimming(side).state = DimmingState::Running;
    // After AntiNavi turned the chip around, the side's record has no user:
    // the game reads the HP through the null pointer, from the BIOS, whose
    // open-bus value is never 0 (so the navi comes).
    let user_alive = b.dimming[side as usize].user.is_none_or(|u| b.objects.get(u).hp != 0);
    if !user_alive {
        // (dword_200F3B8[side] = 1: never read.)
        return advance(b, r, 2);
    }
    if is_navi_chip(b, chip) && anti_navi_waits(b, side) {
        // The other side's AntiNavi turns it around again: back to
        // `sub_800BDB2` from its first phase.
        let a = b.objects.get(r).action;
        return common::set_action(b, r, a - 4);
    }
    advance(b, r, 1);
}

/// `sub_80E1352(user, 0)`: the user vanishes while its navi chip's navi
/// acts (its barrier visual, status visuals, charge glow, Full Synchro
/// aura and the HUD with it).
pub fn hide_user(b: &mut Battle, user: ObjectRef) {
    b.objects.get_mut(user).flags &= !crate::object::flags::VISIBLE;
    set_vanished(b, user, true);
    set_barrier_visual_shown(b, user, false);
    set_links_visible(b, user, false);
    set_charge_glow(b, user, false);
    if let Some(aura) = b.objects.get(user).actor.and_then(|a| b.actors.get(a).full_synchro_aura) {
        crate::kinds::full_synchro_aura::hide(b, aura);
    }
}

/// `sub_80E1352(user, 0xF)`: the user vanishes, but its barrier visual,
/// its confusion and blindness visuals and the HUD stay (BugFix's glow).
pub fn hide_user_sparing(b: &mut Battle, user: ObjectRef) {
    b.objects.get_mut(user).flags &= !crate::object::flags::VISIBLE;
    set_vanished(b, user, true);
    set_charge_glow(b, user, false);
    if let Some(aura) = b.objects.get(user).actor.and_then(|a| b.actors.get(a).full_synchro_aura) {
        crate::kinds::full_synchro_aura::hide(b, aura);
    }
}

/// `sub_80E13DC`: the user is back: visible unless submerged or
/// hidden by the viewer's blindness.
pub fn show_user(b: &mut Battle, user: ObjectRef) {
    let o = b.objects.get(user);
    let f1 = o.collision.map(|c| b.collision.get(c).f1).unwrap_or(0);
    // sub_800EB6C: the other side's navi is hidden from a blind viewer.
    let viewer_blind = b.is_remote(o.alliance)
        && b.player(o.alliance ^ 1).and_then(|p| b.objects.get(p).collision).is_some_and(|c| {
            b.collision.get(c).f1 & crate::collision::f1::BLIND != 0
        });
    if f1 & crate::collision::f1::SUBMERGED == 0 && !viewer_blind {
        b.objects.get_mut(user).flags |= crate::object::flags::VISIBLE;
    }
    set_vanished(b, user, false);
    set_barrier_visual_shown(b, user, true);
    set_links_visible(b, user, true);
    set_charge_glow(b, user, true);
    if let Some(aura) = b.objects.get(user).actor.and_then(|a| b.actors.get(a).full_synchro_aura) {
        crate::kinds::full_synchro_aura::show(b, aura);
    }
}

/// `sub_8010312` / `sub_801031C` with state bit 0x100000: the user is
/// marked gone (what a Reflector's shield, for one, hides by).
fn set_vanished(b: &mut Battle, user: ObjectRef, on: bool) {
    let Some(a) = b.objects.get(user).actor else { return };
    let status = &mut b.actors.get_mut(a).status;
    if on {
        *status |= crate::actor::status::VANISHED;
    } else {
        *status &= !crate::actor::status::VANISHED;
    }
}

/// The barrier's visual (AIData+0x60, a content kind) is hidden and shown
/// with its navi (`sub_80E0DCA`, `sub_80E0DD0`: its `shown` byte).
fn set_barrier_visual_shown(b: &mut Battle, user: ObjectRef, on: bool) {
    let Some(v) = b.objects.get(user).actor.and_then(|a| b.actors.get(a).barrier_visual) else { return };
    // (The game writes the byte of whatever object the link names; a link
    // names a visual while the visual lives.)
    if crate::behavior::has_state_field(b, v, "shown") {
        crate::behavior::set_state_field(b, v, "shown", bn6_content_api::Value::Bool(on));
    }
}

/// The charge glow (AIData+0x58) is hidden and shown with its navi
/// (`sub_80E0F22`, `sub_80E0F28`).
fn set_charge_glow(b: &mut Battle, user: ObjectRef, on: bool) {
    if let Some(glow) = b.objects.get(user).actor.and_then(|a| b.actors.get(a).charge_glow) {
        crate::kinds::charge_glow::set_enabled(b, glow, on);
    }
}

/// The confusion and blindness visuals (CollisionData+0x48, +0x4C) follow
/// their navi's visibility.
fn set_links_visible(b: &mut Battle, user: ObjectRef, visible: bool) {
    let Some(c) = b.objects.get(user).collision else { return };
    let links = b.collision.get(c).links;
    for o in [links[crate::collision::link::CONFUSE], links[crate::collision::link::BLIND]].into_iter().flatten() {
        let f = &mut b.objects.get_mut(o).flags;
        if visible {
            *f |= crate::object::flags::VISIBLE;
        } else {
            *f &= !crate::object::flags::VISIBLE;
        }
    }
}

/// `object_undimScreen`: brighten the screen unless the other side's
/// dimming still runs, then end.
pub fn undim_screen(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        let side = b.objects.get(r).alliance;
        if !out_of_the_way(b.dimming[(side ^ 1) as usize].state) {
            return common::set_progress(b, r, Progress::DESTROY);
        }
        b.fade.start(FadeMode::Undim, FADE_SPEED);
        b.objects.get_mut(r).phase_init = 4;
    }
    if !b.fade.active() {
        common::set_progress(b, r, Progress::DESTROY);
    }
}

/// `object_timefreezeEnd` (the controller's state 8): once the other side
/// is done too, the side that started the dimming ends it (ending the
/// other side's controller), and the controller is freed.
pub fn end(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init != 0 {
        return b.objects.free(r);
    }
    let side = b.objects.get(r).alliance;
    let other = side ^ 1;
    b.dimming(side).state = DimmingState::Ending;
    if !out_of_the_way(b.dimming[other as usize].state) {
        return;
    }
    if b.dimming[side as usize].initiator == side {
        if let Some(c) = b.dimming[other as usize].controller {
            end_controller_now(b, c);
        }
        b.dimming(other).user = None;
        b.clear_dimming(other);
        b.clear_flags(battle_flags::DIMMED);
    }
    b.clear_dimming(side);
    b.dimming(side).user = None;
    b.objects.free(r);
}

#[cfg(test)]
mod tests {
    use crate::battle::{Battle, LinkedRecord};
    use crate::content::testing;
    use crate::scenario;

    /// The navi chip controller, and the test navi chip's navi, by kind
    /// (the scene has kinds content defines, which have no slot number).
    const CONTROLLER: &str = "engine/navi-chip";
    const HEAT_NAVI: &str = "heat-man";

    /// Play a duel with the test navi chip; once a side uses it, the other
    /// side gets AntiNavi (with `bounce`, the user's side too, so the chip
    /// comes back). Returns the side that used it first, the sides its navi
    /// came for in order (later uses included), and the battle.
    fn anti_navi_duel(bounce: bool) -> (u8, Vec<u8>, Battle) {
        let setup = || scenario::setup_with(&[testing::HEAT]);
        let tape = scenario::record_on(setup(), 1500, 11);
        let mut b = Battle::new(setup(), scenario::content());
        let mut user = None;
        let mut came_for = Vec::new();
        let arm = |b: &mut Battle, s: u8| {
            let owner = b.player(s);
            let chip = b.content.chip_numbered(super::ANTI_NAVI);
            b.linked[s as usize] = LinkedRecord { chip, owner, ..LinkedRecord::default() };
        };
        for t in &tape {
            b.tick(&t.input, t.events.clone());
            let seen: Vec<_> = b.objects.in_order().map(|r| (b.kind_key(r).to_string(), b.objects.get(r).alliance)).collect();
            for (kind, alliance) in seen {
                if kind == CONTROLLER && user.is_none() {
                    user = Some(alliance);
                    arm(&mut b, alliance ^ 1);
                    if bounce {
                        arm(&mut b, alliance);
                    }
                }
                if kind == HEAT_NAVI && !came_for.contains(&alliance) {
                    came_for.push(alliance);
                }
            }
        }
        (user.expect("a side used the navi chip"), came_for, b)
    }

    #[test]
    fn anti_navi_turns_a_navi_chip_around() {
        let (user, came_for, b) = anti_navi_duel(false);
        assert_eq!(came_for.first(), Some(&(user ^ 1)), "the navi comes for AntiNavi's side");
        assert_eq!(b.linked[(user ^ 1) as usize].chip, None, "AntiNavi is spent");
    }

    #[test]
    fn two_anti_navis_send_the_chip_back() {
        let (user, came_for, b) = anti_navi_duel(true);
        assert_eq!(came_for.first(), Some(&user), "the navi comes for its own side after all");
        assert!(b.linked.iter().all(|l| l.chip.is_none()), "both AntiNavis are spent");
    }
}
