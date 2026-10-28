//! The danger map (training-ai.md §3.2): where the attacks already under
//! way will land, how soon and how hard, if nobody presses anything. A copy
//! of the battle is played on for [`HORIZON`] undimmed ticks (at most
//! [`CAP`] in all: a dimming plays out whole, so a navi chip lands in its
//! own), both sides pressing nothing, its random numbers drawn anew (it
//! can't foresee the battle's luck); each tick's attacked panels
//! (`Battle::attacked_panels`, `Battle::attack_damage`) are kept, by the
//! side whose attack it is: the first tick an attack lands on the panel and
//! the most damage one carried there.
//!
//! It reads the battle as it is, as the lookahead does: what is on the
//! field and how it moves, not the other player's next press (bending §3's
//! screen-only rule the same way).

use crate::env::{fight_runs, step_battle};
use nettai_battle::Battle;

/// Undimmed ticks a copy is played on, and the ticks at most in all.
pub const HORIZON: u32 = 40;
pub const CAP: u32 = 480;

/// What lands on a panel within the horizon: the undimmed ticks till the
/// first attack does (at least 1; 0: none does), and the most damage one
/// carried there.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Landing {
    pub first: u16,
    pub damage: u16,
}

/// By the side whose attacks they are and panel bit (`y * 8 + x`, the
/// field's 32 first: every playable panel), what lands there.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Danger {
    pub panels: [[Landing; 32]; 2],
}

impl Danger {
    /// What side `side`'s attacks bring to panel bit `panel`.
    pub fn at(&self, side: u8, panel: usize) -> Landing {
        self.panels[side as usize & 1].get(panel).copied().unwrap_or_default()
    }
}

/// The danger map of `b`, its copy's luck `luck`; none where the fight
/// doesn't run.
pub fn foresee(b: &Battle, luck: u64) -> Danger {
    let mut out = Danger::default();
    if !fight_runs(b) {
        return out;
    }
    let mut c = b.clone();
    crate::search::own_luck(&mut c, luck);
    let (mut undimmed, mut t) = (0u32, 0u32);
    while undimmed < HORIZON && t < CAP {
        if !step_battle(&mut c, [0, 0]) {
            break;
        }
        t += 1;
        undimmed += !c.is_dimmed() as u32;
        for side in 0..2u8 {
            let mut bits = c.attacked_panels(side) & 0xFFFF_FFFF;
            while bits != 0 {
                let p = bits.trailing_zeros();
                bits &= bits - 1;
                let l = &mut out.panels[side as usize][p as usize];
                if l.first == 0 {
                    l.first = undimmed.max(1) as u16;
                }
                l.damage = l.damage.max(c.attack_damage(side, p));
            }
        }
        if c.round_end().is_some() || !fight_runs(&c) {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests;
