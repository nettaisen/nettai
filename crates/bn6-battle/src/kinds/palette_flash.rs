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
    /// Param1: which flash (0: the screen, `sub_80E10C0`; 1: the screen
    /// and the objects, `sub_80E114C`).
    pub variant: u8,
    /// Ticks the flash lasts.
    pub duration: u8,
    /// Keeps flashing while dimmed.
    pub while_dimmed: bool,
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
pub fn spawn(b: &mut Battle, duration: u8, while_dimmed: bool, while_paused: bool) -> Option<ObjectRef> {
    let mode = while_dimmed as u8 | (while_paused as u8) << 1;
    spawn_with(b, [0, duration, mode, 0])
}

/// `sub_80E11E0` with its parameters: the variant, the ticks, whether it
/// keeps flashing while dimmed (bit 0) and while paused (bit 1), and the
/// colour (Param4). (The spawner also leaves a register in ExtraVars,
/// which neither variant reads.)
pub fn spawn_with(b: &mut Battle, params: [u8; 4]) -> Option<ObjectRef> {
    let [variant, duration, mode, _] = params;
    let r = b.objects.spawn(Pool::Effect, INDEX, Vec3::default(), params)?;
    b.objects.get_mut(r).flags |= flags::RUN_WHILE_PAUSED | flags::RUN_WHILE_DIMMED;
    *vars(b, r) = Vars { variant, duration, while_dimmed: mode & 1 != 0, while_paused: mode & 2 != 0 };
    Some(r)
}

/// `sub_80E10A4`: by variant, `sub_80E10C0` or `sub_80E114C`: count the
/// flash down and free it when done. While the battle is paused or dimmed
/// (unless it keeps flashing then), it only holds the palette. (Variant
/// 1 tests the paused bit twice, so it holds while dimmed either way; it
/// leaves its timer alone.)
pub fn update(b: &mut Battle, r: ObjectRef) {
    let Vars { variant, duration, while_dimmed, while_paused } = vars(b, r).clone();
    let while_dimmed = match variant {
        0 => while_dimmed,
        1 => false,
        v => panic!("palette flash variant {v} reads past off_80E10B8"),
    };
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
    if variant == 0 {
        o.timer = o.timer.wrapping_add(1);
    }
}
