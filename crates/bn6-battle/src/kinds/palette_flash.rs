//! A screen palette flash (effect object #0x0A, `sub_80E10A4`), such as
//! the one when MegaMan's Beast Out lands. It only writes palette
//! transforms, but it holds an effect slot and a place in the update order
//! for its duration. It has no position of its own: its spawner leaves
//! whatever was in the registers. See docs/engine/objects-and-player.md
//! §A.7.

use crate::battle::Battle;
use crate::object::{ObjectRef, Pool, Vec3, flags, state};

pub const INDEX: u8 = 0x0A;

/// Flash-private state (the spawn parameters).
#[derive(Clone, Debug, Default)]
pub struct Vars {
    /// Ticks the flash lasts.
    pub duration: u8,
    /// Keeps flashing in time stop.
    pub in_time_stop: bool,
    /// Keeps flashing while the battle is paused.
    pub while_paused: bool,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut b.objects.get_mut(r).vars {
        crate::kinds::Vars::PaletteFlash(v) => v,
        v => panic!("palette flash with {v:?}"),
    }
}

/// `sub_80E11E0`: a white flash (variant 0) for `duration` ticks.
pub fn spawn(b: &mut Battle, duration: u8, in_time_stop: bool, while_paused: bool) -> Option<ObjectRef> {
    let mode = in_time_stop as u8 | (while_paused as u8) << 1;
    let r = b.objects.spawn(Pool::Effect, INDEX, Vec3::default(), [0, duration, mode, 0])?;
    b.objects.get_mut(r).flags |= flags::RUN_WHILE_PAUSED | flags::RUN_IN_TIME_STOP;
    *vars(b, r) = Vars { duration, in_time_stop, while_paused };
    Some(r)
}

/// `sub_80E10C0`: count the flash down and free it when done. While the
/// battle is paused or time is stopped (unless it keeps flashing then), it
/// only holds the palette.
pub fn update(b: &mut Battle, r: ObjectRef) {
    let Vars { duration, in_time_stop, while_paused } = vars(b, r).clone();
    let held = !while_paused && (b.paused || (!in_time_stop && b.is_time_stop()));
    if held {
        return;
    }
    let o = b.objects.get_mut(r);
    if o.state == state::INIT {
        o.timer2 = duration as u16;
        o.timer = 0;
        o.state = state::UPDATE;
    }
    if o.timer2 == 0 {
        b.objects.free(r);
        return;
    }
    o.timer2 -= 1;
    o.timer = o.timer.wrapping_add(1);
}
