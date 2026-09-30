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
//! - banners: [`Battle::banner_for`] (the chip-name banners say whose chip
//!   it is, the result banner shows the viewer's navi winning or losing);
//! - the result: [`Battle::round_end_for`].
//!
//! Still shown only as the local side sees it (docs/design/rollback.md):
//! which navi appears at once at the intro and which fades in, what a
//! blinded player can't see, and the visibility of the Beast Out lock-on
//! marker and of the A-button charge glow (the objects' `VISIBLE` flag).

use crate::battle::{Battle, BattleResult, RoundEnd, fight};
use crate::data::{BannerId, LOSE_BANNERS, WIN_BANNERS};
use crate::setup::SetScore;
use crate::time_freeze::{LOCAL_NAME_BANNER, REMOTE_NAME_BANNER};

impl BattleResult {
    /// The same result for the other side.
    pub fn for_other_side(self) -> BattleResult {
        match self {
            BattleResult::Won => BattleResult::Lost,
            BattleResult::Lost => BattleResult::Won,
            BattleResult::Drawn => BattleResult::Drawn,
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
    /// The banner on screen as `viewer`'s console shows it.
    pub fn banner_for(&self, viewer: u8) -> Option<BannerId> {
        let id = self.banner.id.filter(|_| self.banner.active)?;
        let local = self.round.local_side;
        if viewer == local {
            return Some(id);
        }
        if id == LOCAL_NAME_BANNER {
            return Some(REMOTE_NAME_BANNER);
        }
        if id == REMOTE_NAME_BANNER {
            return Some(LOCAL_NAME_BANNER);
        }
        let navi = |side: u8| self.stats[side as usize].navi.index();
        let result_banner = WIN_BANNERS[navi(local)] == id || LOSE_BANNERS[navi(local)] == id;
        if result_banner && matches!(self.fight.state, fight::WIN | fight::LOSE) {
            let won = self.round.winner == viewer;
            return Some(if won { WIN_BANNERS[navi(viewer)] } else { LOSE_BANNERS[navi(viewer)] });
        }
        Some(id)
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
    use crate::setup::{BattleSettings, NaviStats, RoundSetup};

    fn battle() -> Battle {
        let stats = NaviStats { hp: 100, max_hp: 100, max_base_hp: 100, ..NaviStats::default() };
        Battle::new(RoundSetup {
            settings: BattleSettings { effects: crate::setup::effects::LINK, ..crate::data::BATTLE_SETTINGS[0] },
            navi_stats: [stats; 2],
            rng: 1,
            local_side: 0,
            score: SetScore::default(),
            later_stages: Default::default(),
            low_hp_music_latched: false,
            sp_times: Default::default(),
            players: Default::default(),
            link_delay: 0,
        })
    }

    #[test]
    fn each_viewer_sees_its_own_name_and_result_banners() {
        let mut b = battle();
        b.banner.start(LOCAL_NAME_BANNER);
        assert_eq!((b.banner_for(0), b.banner_for(1)), (Some(LOCAL_NAME_BANNER), Some(REMOTE_NAME_BANNER)));
        b.banner = Default::default();
        b.fight.state = fight::WIN;
        b.round.winner = 0;
        b.banner.start(WIN_BANNERS[0]);
        assert_eq!((b.banner_for(0), b.banner_for(1)), (Some(WIN_BANNERS[0]), Some(LOSE_BANNERS[0])));
    }

    #[test]
    fn the_other_side_lost_what_the_local_side_won() {
        assert_eq!(BattleResult::Won.for_other_side(), BattleResult::Lost);
        let s = SetScore { wins: 2, losses: 1, round: 3, max_combo: 4 };
        assert_eq!(s.for_other_side(), SetScore { wins: 1, losses: 2, round: 3, max_combo: 4 });
    }
}
