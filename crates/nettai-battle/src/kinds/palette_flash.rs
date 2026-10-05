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

/// Whether the battle holds the flash `r`: paused or dimmed (unless it
/// keeps flashing then), it turns its palette transforms off and counts
/// nothing. Variant 1 (`sub_80E114C`) tests the pause bit where variant 0
/// tests the dimming bit, so it holds while dimmed whatever its parameters
/// say. EXE5's (the arena's `effects.palette_flash`, 0x080E1030) holds
/// either variant while paused, and while dimmed only with no mode bit
/// set. A frontend asks this for whether the flash shows.
pub fn held(b: &Battle, r: ObjectRef) -> bool {
    let crate::kinds::Vars::PaletteFlash(Vars { variant, while_dimmed, while_paused, .. }) = b.objects.get(r).vars else {
        panic!("palette flash with {:?}", b.objects.get(r).vars)
    };
    let keeps_dimming = match variant {
        0 => while_dimmed,
        1 => while_paused,
        v => panic!("palette flash variant {v} reads past off_80E10B8"),
    };
    match b.game_rules().effects.palette_flash {
        crate::content::PaletteFlashRule::ModeRunsThroughPause => !while_paused && (b.paused || (!keeps_dimming && b.is_dimmed())),
        crate::content::PaletteFlashRule::PauseHolds => b.paused || (!while_dimmed && !while_paused && b.is_dimmed()),
    }
}

/// `sub_80E10A4`: count the flash down and free it when done; while the
/// battle holds it (`held`), nothing. Variant 1 doesn't count its ticks
/// up.
pub fn update(b: &mut Battle, r: ObjectRef) {
    if held(b, r) {
        return;
    }
    let Vars { variant, duration, .. } = vars(b, r).clone();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::battle_flags;
    use crate::content::{PaletteFlashRule, testing};

    /// A battle on `rule`, with a flash of `variant` and the mode's two
    /// bits, dimmed or paused.
    fn held_on(rule: PaletteFlashRule, variant: u8, while_dimmed: bool, while_paused: bool, dimmed: bool, paused: bool) -> bool {
        let mut c: crate::content::Content = testing::build();
        c.define().unwrap_or_else(|e| panic!("{e}"));
        c.rules_mut().effects.palette_flash = rule;
        let c = std::sync::Arc::new(c);
        let mut setup = testing::round_setup(testing::LINK_BATTLE, testing::megaman_on(&c));
        testing::on(&mut setup, &c);
        let mut b = Battle::new(setup, c);
        let r = spawn_variant(&mut b, variant, 4, while_dimmed, while_paused).expect("an effect slot");
        if dimmed {
            b.set_flags(battle_flags::DIMMED);
        }
        b.paused = paused;
        held(&b, r)
    }

    /// What holds a flash is its game's rule, and one answer for its
    /// update and for whoever draws it: EXE6's two-layer flash tests the
    /// pause bit against a dimming, so one that only keeps flashing while
    /// dimmed is held by it; EXE5's flashes through a dimming with any
    /// mode bit, and a pause holds either whatever the mode (ChaosLrd's
    /// strike, `battle.palette_flash(1, 4, true, false)`).
    #[test]
    fn a_flash_is_held_by_its_games_rule() {
        use PaletteFlashRule::{ModeRunsThroughPause, PauseHolds};
        // (rule, variant, while dimmed, while paused, dimmed, paused, held)
        let cases = [
            (ModeRunsThroughPause, 1, true, false, true, false, true),
            (PauseHolds, 1, true, false, true, false, false),
            (ModeRunsThroughPause, 0, true, false, true, false, false),
            (PauseHolds, 0, true, false, true, false, false),
            (ModeRunsThroughPause, 0, false, false, true, false, true),
            (PauseHolds, 0, false, false, true, false, true),
            (ModeRunsThroughPause, 0, false, true, false, true, false),
            (PauseHolds, 0, false, true, false, true, true),
            (PauseHolds, 1, true, true, false, true, true),
        ];
        for (rule, variant, while_dimmed, while_paused, dimmed, paused, want) in cases {
            let got = held_on(rule, variant, while_dimmed, while_paused, dimmed, paused);
            assert_eq!(got, want, "{rule:?} variant {variant} dimmed-mode {while_dimmed} paused-mode {while_paused}, dimmed {dimmed} paused {paused}");
        }
    }
}
