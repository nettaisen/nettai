//! The dimming service of dimming chips: the per-side dimming records
//! (`byte_203CF00`, one 0x50-byte record per side) and the phases every
//! dimming controller object goes through: start the dimming (battle
//! flag 4: everything but the allowed objects stands still), dim the
//! screen, show the telop, run the chip's effect (the controller's own),
//! brighten the screen, and end the dimming. A cut-in (the other
//! side's dimming chip used during the telop) makes the controllers wait
//! on each other. See docs/engine/chips.md §3.6.

use crate::battle::{Battle, battle_flags};
use crate::content::{BannerId, ChipId};
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

/// Screen fades the controllers use, in ticks.
const DIM_TICKS: u8 = 16;
const UNDIM_TICKS: u8 = 17;

/// The telops: the local player's, and the other player's.
pub(crate) const LOCAL_TELOP: BannerId = BannerId(0x4C);
pub(crate) const REMOTE_TELOP: BannerId = BannerId(0x50);

/// Chips from this id on can't be cut in on.
const FIRST_NO_CUT_IN: ChipId = 0x170;

/// What every controller knows about its chip (object +0x30 / +0x32): for
/// the name the HUD shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DimmingChip {
    pub chip: ChipId,
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
    pub(crate) fn register_dimming(&mut self, side: u8, chip: ChipId, controller: ObjectRef, user: ObjectRef) {
        self.start_dimming(side, chip >= FIRST_NO_CUT_IN, Some(controller), user);
    }

    /// `sub_800BF16`: `side` starts a dimming with `controller` (none if
    /// its spawn failed: the record waits for nothing), used by `user`;
    /// `no_cut_in`: the other side can't cut in on it. Its previous
    /// controller, if any, is told to end.
    pub(crate) fn start_dimming(&mut self, side: u8, no_cut_in: bool, controller: Option<ObjectRef>, user: ObjectRef) {
        for r in &mut self.dimming {
            r.initiator = side;
        }
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
        b.fade.start();
        b.fade.remaining = DIM_TICKS;
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
fn is_navi_chip(chip: ChipId) -> bool {
    (0xDD..=0x118).contains(&chip)
}

/// AntiNavi (chip 0xBA) is the other side's defensive chip.
fn anti_navi_waits(b: &Battle, side: u8) -> bool {
    b.linked[(side ^ 1) as usize].chip == 0xBA
}

/// `sub_800BDB2` (a navi chip's action after the dim): on to the name,
/// unless the other side's AntiNavi turns the navi back.
pub fn check_anti_navi(b: &mut Battle, r: ObjectRef, chip: ChipId) {
    let side = b.objects.get(r).alliance;
    if b.objects.get(r).phase != 0 || (is_navi_chip(chip) && anti_navi_waits(b, side)) {
        panic!("AntiNavi (sub_800BDB2) is not implemented yet");
    }
    advance(b, r, 1);
}

/// `sub_800BA8A`: a navi chip's name, like `show_telop`, but the
/// effect runs whether or not the chip can be cut in on, and is skipped
/// only if the user was deleted.
pub fn show_navi_telop(b: &mut Battle, r: ObjectRef, chip: ChipId) {
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
        b.play_sound(crate::sound::SoundId(0x173));
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
    let user_alive = b.dimming[side as usize].user.is_some_and(|u| b.objects.get(u).hp != 0);
    if !user_alive {
        // (dword_200F3B8[side] = 1: never read.)
        return advance(b, r, 2);
    }
    if is_navi_chip(chip) && anti_navi_waits(b, side) {
        panic!("AntiNavi (sub_800BA8A) is not implemented yet");
    }
    advance(b, r, 1);
}

/// `sub_80E1352(user, 0)`: the user vanishes while its navi chip's navi
/// acts (its status visuals and the HUD with it).
pub fn hide_user(b: &mut Battle, user: ObjectRef) {
    b.objects.get_mut(user).flags &= !crate::object::flags::VISIBLE;
    set_vanished(b, user, true);
    set_links_visible(b, user, false);
    if b.objects.get(user).actor.is_some_and(|a| b.actors.get(a).full_synchro_aura.is_some()) {
        panic!("hiding the Full Synchro aura (sub_80C4C46) is not implemented yet");
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
    set_links_visible(b, user, true);
    if b.objects.get(user).actor.is_some_and(|a| b.actors.get(a).full_synchro_aura.is_some()) {
        panic!("showing the Full Synchro aura (sub_80C4C4C) is not implemented yet");
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
        b.fade.start();
        b.fade.remaining = UNDIM_TICKS;
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
