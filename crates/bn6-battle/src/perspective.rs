//! What each player is shown when two netplay peers present one
//! simulation.
//!
//! Both peers simulate from one perspective, `RoundSetup::local_side`
//! (shared setup), and each presents the result for its own player, the
//! *viewer*. Where the original keeps per-console state, the engine either
//! keeps it for both sides or derives it per viewer here:
//!
//! - sound: `Battle::sound_cues_for` (the engine records what each side
//!   hears: its charge sounds, its hit sound, its pinch music, its
//!   victory or defeat music);
//! - banners: [`Battle::banner_for`] (the telops say whose chip
//!   it is, the result banner shows the viewer's navi winning or losing)
//!   and [`Battle::telop_for`] (a hidden chip's telop names it only to its
//!   user, if to anyone) and [`Battle::used_chip_for`] (the other player's
//!   chip, named for a second);
//! - the result: [`Battle::round_end_for`].
//!
//! Still shown only as the local side sees it (docs/design/rollback.md):
//! which navi appears at once at the intro and which fades in, what a
//! blinded player can't see, and the visibility of the Beast Out lock-on
//! marker and of the A-button charge glow (the objects' `VISIBLE` flag).

use crate::battle::{Battle, BattleResult, RoundEnd, fight};
use crate::content::BannerId;
use crate::setup::SetScore;
use crate::dimming::telop_banner;
use crate::hud::TelopHidden;
use bn6_content_api::ChipHandle;

/// A telop as one player's console shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShownTelop {
    /// It is the other player's chip (the banner on the right).
    pub remote: bool,
    pub name: TelopName,
    /// The damage after the name (0: none).
    pub damage: u16,
    /// "+N" after the damage (0: none).
    pub bonus: u16,
    /// "x2" after the numbers.
    pub doubled: bool,
}

/// The name a telop shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TelopName {
    Chip(ChipHandle),
    /// "????": a hidden chip (the game shows chip 0x171's name).
    Hidden,
    /// The engine wasn't told which chip the telop names.
    Unknown,
}

impl BattleResult {
    /// The same result for the other side.
    pub fn for_other_side(self) -> BattleResult {
        match self {
            BattleResult::Won => BattleResult::Lost,
            BattleResult::Lost => BattleResult::Won,
            // The rest end the battle for both alike.
            r => r,
        }
    }
}

impl SetScore {
    /// The same score for the other side.
    pub fn for_other_side(self) -> SetScore {
        SetScore { wins: self.losses, losses: self.wins, ..self }
    }
}

impl Battle {
    /// `sub_800EB6C`: the local player sees `alliance`'s objects unless
    /// they are the other side's and the local navi (`sub_80103BC`) is
    /// blind. A navi whose init hasn't run yet has no collision data: the
    /// game reads its status word through the null pointer, from the BIOS,
    /// which gives the opcode it last fetched (`f1::NULL_READ`), and that
    /// has the blind bit. It happens on a round's first tick when the
    /// other side's navi inits first and its FirstBarrier's visual asks.
    pub fn viewer_sees(&self, alliance: u8) -> bool {
        use crate::collision::f1;
        if !self.is_remote(alliance) {
            return true;
        }
        let Some(viewer) = self.player(alliance ^ 1) else { return true };
        let status = self.objects.get(viewer).collision.map_or(f1::NULL_READ, |c| self.collision.get(c).f1);
        status & f1::BLIND == 0
    }

    /// The banner on screen as `viewer`'s console shows it.
    pub fn banner_for(&self, viewer: u8) -> Option<BannerId> {
        let id = self.banner.id.filter(|_| self.banner.active)?;
        let local = self.round.local_side;
        if viewer == local {
            return Some(id);
        }
        let (telop, remote_telop) = (telop_banner(self, false), telop_banner(self, true));
        if id == telop {
            return Some(remote_telop);
        }
        if id == remote_telop {
            return Some(telop);
        }
        let navi = |side: u8| self.content.navi(self.stats[side as usize].navi);
        let result_banner = navi(local).win_banner == id || navi(local).lose_banner == id;
        if result_banner && matches!(self.fight.state, fight::WIN | fight::LOSE) {
            let won = self.round.winner == viewer;
            return Some(if won { navi(viewer).win_banner } else { navi(viewer).lose_banner });
        }
        Some(id)
    }

    /// The telop on screen as `viewer`'s console shows it.
    pub fn telop_for(&self, viewer: u8) -> Option<ShownTelop> {
        let t = self.banner.telop.filter(|_| self.banner.active)?;
        let remote = t.side != viewer;
        let hidden = match t.hidden {
            TelopHidden::No => false,
            TelopHidden::FromOpponent => remote,
            TelopHidden::FromBoth => true,
        };
        if hidden {
            return Some(ShownTelop { remote, name: TelopName::Hidden, damage: 0, bonus: 0, doubled: false });
        }
        let name = t.chip.map_or(TelopName::Unknown, TelopName::Chip);
        Some(ShownTelop { remote, name, damage: t.damage, bonus: t.bonus, doubled: t.doubled })
    }

    /// What `viewer`'s console shows of its navi's chips.
    pub fn chip_hud_for(&self, viewer: u8) -> crate::hud::ChipHud {
        self.chip_hud[viewer as usize & 1]
    }

    /// The chip the other player just used, while `viewer`'s console names
    /// it.
    pub fn used_chip_for(&self, viewer: u8) -> Option<crate::hud::UsedChip> {
        self.used_chips[(viewer ^ 1) as usize & 1]
    }

    /// How the round ended, for `viewer` (the result and the score are
    /// the local side's in `round_end`).
    pub fn round_end_for(&self, viewer: u8) -> Option<RoundEnd> {
        let end = self.round_end()?.clone();
        if viewer == self.round.local_side {
            return Some(end);
        }
        Some(match end {
            RoundEnd::Over(r) => RoundEnd::Over(r.for_other_side()),
            RoundEnd::NextRound { settings, score } => RoundEnd::NextRound { settings, score: score.for_other_side() },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::testing;

    fn battle() -> Battle {
        Battle::new(testing::round_setup(testing::LINK_BATTLE, testing::stats(100)), testing::content())
    }

    #[test]
    fn each_viewer_sees_its_own_name_and_result_banners() {
        let mut b = battle();
        let (telop, remote_telop) = (telop_banner(&b, false), telop_banner(&b, true));
        b.start_banner(telop);
        assert_eq!((b.banner_for(0), b.banner_for(1)), (Some(telop), Some(remote_telop)));
        b.banner = Default::default();
        b.fight.state = fight::WIN;
        b.round.winner = 0;
        let navi = b.content.navi_data(crate::setup::Navi::MEGAMAN).clone();
        b.start_banner(navi.win_banner);
        assert_eq!((b.banner_for(0), b.banner_for(1)), (Some(navi.win_banner), Some(navi.lose_banner)));
    }

    #[test]
    fn the_other_side_lost_what_the_local_side_won() {
        assert_eq!(BattleResult::Won.for_other_side(), BattleResult::Lost);
        let s = SetScore { wins: 2, losses: 1, round: 3, max_combo: 4 };
        assert_eq!(s.for_other_side(), SetScore { wins: 1, losses: 2, round: 3, max_combo: 4 });
    }
}
