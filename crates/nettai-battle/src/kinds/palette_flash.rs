//! A screen palette flash (effect object #0x0A, `sub_80E10A4`), such as
//! the one when MegaMan's Beast Out lands. It only writes palette
//! transforms, but it holds an effect slot and a place in the update order
//! for its duration. It has no position of its own: its spawner leaves
//! whatever was in the registers. See docs/engine/objects-and-player.md
//! §A.7.

use crate::battle::Battle;
use crate::object::{ObjectRef, Vec3, flags, state};

/// Flash-private state (the spawn parameters).
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    /// Which flash (Param1): 0 white or red over one palette layer
    /// (`sub_80E10C0`), 1 white over two (`sub_80E114C`).
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
    spawn_variant(b, 0, duration, while_dimmed, while_paused)
}

/// `sub_80E11E0` with the flash's variant (Param1).
pub fn spawn_variant(b: &mut Battle, variant: u8, duration: u8, while_dimmed: bool, while_paused: bool) -> Option<ObjectRef> {
    let mode = while_dimmed as u8 | (while_paused as u8) << 1;
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::PaletteFlash, Vec3::default(), [variant, duration, mode, 0])?;
    b.objects.get_mut(r).flags |= flags::RUN_WHILE_PAUSED | flags::RUN_WHILE_DIMMED;
    *vars(b, r) = Vars { variant, duration, while_dimmed, while_paused };
    Some(r)
}

/// `sub_80E10A4`: count the flash down and free it when done. While the
/// battle is paused or dimmed (unless it keeps flashing then), it only
/// holds the palette. Variant 1 (`sub_80E114C`) tests the pause bit where
/// variant 0 tests the dimming bit, so it holds while dimmed whatever its
/// parameters say, and it doesn't count its ticks up. BN5's (the arena's
/// `effects.palette_flash`, 0x080E1030) holds either variant while paused,
/// and while dimmed only with no mode bit set.
pub fn update(b: &mut Battle, r: ObjectRef) {
    let Vars { variant, duration, while_dimmed, while_paused } = vars(b, r).clone();
    let keeps_dimming = match variant {
        0 => while_dimmed,
        1 => while_paused,
        v => panic!("palette flash variant {v} reads past off_80E10B8"),
    };
    let held = match b.game_rules().effects.palette_flash {
        crate::content::PaletteFlashRule::Bn6 => !while_paused && (b.paused || (!keeps_dimming && b.is_dimmed())),
        crate::content::PaletteFlashRule::Bn5 => b.paused || (!while_dimmed && !while_paused && b.is_dimmed()),
    };
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
