//! Time-freeze chips: the per-side freeze records (`byte_203CF00`, one
//! 0x50-byte record per side) and the phases every freeze controller
//! object goes through: stop time, dim the screen, show the chip's name,
//! run the chip's effect (the controller's own), brighten the screen, and
//! start time again. A counter (the other side freezing during the name)
//! makes the controllers wait on each other. See docs/engine/chips.md §3.6.

use crate::battle::{Battle, battle_flags};
use crate::data::{BannerId, ChipId};
use crate::hud::BannerStatus;
use crate::kinds::common::{self, Progress};
use crate::object::{ObjectRef, state};

/// Where a side's freeze is (record +1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FreezeState {
    #[default]
    Idle = 0,
    /// A controller was registered.
    Registered = 1,
    /// Its name is (about to be) shown.
    ShowingName = 2,
    /// The name was shown, but the other side's counter runs first.
    Waiting = 3,
    /// The chip's effect runs.
    Running = 4,
    /// Done; time starts again once the other side is done too.
    Ending = 5,
}

/// A side's freeze record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FreezeRecord {
    /// +0: the side whose freeze is current (both records are written
    /// together; a counter takes it over).
    pub owner: u8,
    pub state: FreezeState,
    /// +2: the chip can't be countered (chips 0x170 and up).
    pub uncounterable: bool,
    /// +3: the side that stopped time (both records).
    pub initiator: u8,
    /// +8: the side's controller object.
    pub controller: Option<ObjectRef>,
    /// +0xC: the navi that used the chip.
    pub user: Option<ObjectRef>,
}

/// Screen fades the controllers use, in ticks.
const DIM_TICKS: u8 = 16;
const UNDIM_TICKS: u8 = 17;

/// The chip-name banners: the local player's, and the other player's.
const LOCAL_NAME_BANNER: BannerId = BannerId(0x4C);
const REMOTE_NAME_BANNER: BannerId = BannerId(0x50);

/// Chips from this id on can't be countered.
const FIRST_UNCOUNTERABLE: ChipId = 0x170;

/// What every controller knows about its chip (object +0x30 / +0x32): for
/// the name the HUD shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FreezeChip {
    pub chip: ChipId,
    /// The Atk+ / cross bonus, shown with the name for damaging chips.
    pub bonus: u16,
}

impl Battle {
    fn freeze(&mut self, side: u8) -> &mut FreezeRecord {
        &mut self.freeze[side as usize]
    }

    /// `sub_800BF16`: `side` stops time with `controller`, used by `user`.
    /// Its previous controller, if any, is told to end.
    pub(crate) fn register_freeze(&mut self, side: u8, chip: ChipId, controller: ObjectRef, user: ObjectRef) {
        for r in &mut self.freeze {
            r.initiator = side;
        }
        // sub_800B8AC
        for r in &mut self.freeze {
            r.owner = side;
        }
        if let Some(old) = self.freeze(side).controller {
            end_controller_now(self, old);
        }
        let rec = self.freeze(side);
        rec.uncounterable = chip >= FIRST_UNCOUNTERABLE;
        rec.controller = Some(controller);
        rec.user = Some(user);
        rec.state = FreezeState::Registered;
    }

    /// `sub_800B89C`: a side's freeze is over.
    fn clear_freeze(&mut self, side: u8) {
        let rec = self.freeze(side);
        rec.state = FreezeState::Idle;
        rec.controller = None;
    }
}

/// Kill a controller: it frees itself at its next update without its end
/// logic (state 8 with the phase initialized).
fn end_controller_now(b: &mut Battle, r: ObjectRef) {
    common::set_progress(b, r, Progress { phase_init: 4, ..Progress::DESTROY });
}

/// Idle, or already done: the other side lets a controller go on.
fn out_of_the_way(state: FreezeState) -> bool {
    matches!(state, FreezeState::Idle | FreezeState::Ending)
}

/// `object_timefreezeBegin` (the controller's init): time stops if its side
/// started the freeze.
pub fn begin(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance;
    // (The local player's HUD hides the custom gauge.)
    if b.freeze[side as usize].initiator == side {
        b.set_flags(battle_flags::TIME_STOP);
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

/// `object_drawChipName`: once the other side isn't mid-freeze, show the
/// chip's name; after it, wait for a counter to finish, then run the
/// effect (the next action), or skip it if the user was deleted.
pub fn show_chip_name(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance;
    let other = side ^ 1;
    if b.objects.get(r).phase_init == 0 {
        b.freeze(side).state = FreezeState::ShowingName;
        if !matches!(b.freeze[other as usize].state, FreezeState::Waiting | FreezeState::Idle) {
            return;
        }
        // (The HUD's other parts hide.) The chip's name, with its damage
        // and bonus for damaging chips.
        let banner = if b.is_remote(side) { REMOTE_NAME_BANNER } else { LOCAL_NAME_BANNER };
        b.banner.start(banner);
        b.play_sound(crate::sound::SoundId(0x173));
        b.objects.get_mut(r).phase_init = 4;
        return;
    }
    if b.banner.status() != BannerStatus::Done {
        return;
    }
    // sub_800B8C2: a counter took the freeze over.
    if b.freeze[side as usize].owner != side && !out_of_the_way(b.freeze[other as usize].state) {
        b.freeze(side).state = FreezeState::Waiting;
        return;
    }
    b.freeze(side).state = FreezeState::Running;
    let rec = b.freeze[side as usize];
    let user_alive = rec.user.is_some_and(|u| b.objects.get(u).hp != 0);
    advance(b, r, if rec.uncounterable || user_alive { 1 } else { 2 });
}

/// `object_undimScreen`: brighten the screen unless the other side's
/// freeze still runs, then end.
pub fn undim_screen(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        let side = b.objects.get(r).alliance;
        if !out_of_the_way(b.freeze[(side ^ 1) as usize].state) {
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
/// is done too, the side that stopped time starts it again (ending the
/// other side's controller), and the controller is freed.
pub fn end(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init != 0 {
        return b.objects.free(r);
    }
    let side = b.objects.get(r).alliance;
    let other = side ^ 1;
    b.freeze(side).state = FreezeState::Ending;
    if !out_of_the_way(b.freeze[other as usize].state) {
        return;
    }
    if b.freeze[side as usize].initiator == side {
        if let Some(c) = b.freeze[other as usize].controller {
            end_controller_now(b, c);
        }
        b.freeze(other).user = None;
        b.clear_freeze(other);
        b.clear_flags(battle_flags::TIME_STOP);
    }
    b.clear_freeze(side);
    b.freeze(side).user = None;
    b.objects.free(r);
}
