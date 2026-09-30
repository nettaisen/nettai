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
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    /// Ticks the flash lasts (Param2).
    pub duration: u8,
    /// Keeps flashing while dimmed (Param3 bit 0).
    pub while_dimmed: bool,
    /// Keeps flashing while the battle is paused (Param3 bit 1).
    pub while_paused: bool,
    /// Param1 1 (`sub_80E114C`): the palettes held white rather than
    /// blinking, and no tick count; it doesn't keep on while dimmed (the
    /// routine tests the pause bit twice).
    pub steady: bool,
    /// Param4: the blinking colour (0 white, 1 red; drawn only).
    pub color: u8,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut b.objects.get_mut(r).vars {
        crate::kinds::Vars::PaletteFlash(v) => v,
        v => panic!("palette flash with {v:?}"),
    }
}

/// `sub_80E11E0`: a white flash (variant 0) for `duration` ticks.
pub fn spawn(b: &mut Battle, duration: u8, while_dimmed: bool, while_paused: bool) -> Option<ObjectRef> {
    spawn_with(b, Vars { duration, while_dimmed, while_paused, steady: false, color: 0 })
}

/// `sub_80E11E0` with all its parameters.
pub fn spawn_with(b: &mut Battle, v: Vars) -> Option<ObjectRef> {
    let mode = v.while_dimmed as u8 | (v.while_paused as u8) << 1;
    let r = b.objects.spawn(Pool::Effect, INDEX, Vec3::default(), [v.steady as u8, v.duration, mode, v.color])?;
    b.objects.get_mut(r).flags |= flags::RUN_WHILE_PAUSED | flags::RUN_WHILE_DIMMED;
    *vars(b, r) = v;
    Some(r)
}

/// `sub_80E10C0` (`sub_80E114C` steady): count the flash down and free it
/// when done. While the battle is paused or dimmed (unless it keeps
/// flashing then), it only holds the palette.
pub fn update(b: &mut Battle, r: ObjectRef) {
    let Vars { duration, while_dimmed, while_paused, steady, .. } = vars(b, r).clone();
    let while_dimmed = while_dimmed && !steady;
    let held = !while_paused && (b.paused || (!while_dimmed && b.is_dimmed()));
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
    if !steady {
        o.timer = o.timer.wrapping_add(1);
    }
}
