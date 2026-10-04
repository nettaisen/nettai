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
//! - the result: [`Battle::round_end_for`];
//! - what each player sees of the objects: [`Battle::visible_to`] (a blind
//!   player doesn't see the other side's objects, the Beast Out lock-on
//!   marker and the A-button charge glow show only to their owner). Where
//!   the original decides an object's `VISIBLE` flag by its console's
//!   rule, the engine decides it for both viewers ([`Battle::hide_from_blind`],
//!   [`Battle::hide_from_other_side`]) and keeps the local side's in the
//!   flag, which the recordings check.
//!
//! Still the local side's (docs/design/rollback.md §2.3): which navi
//! appears at once at the intro and which fades in (simulation state), and
//! a few looks (the counter blink, the parts a barrier visual hides unless
//! its navi is the local MegaMan, the trap mark's facing).

use crate::battle::{Battle, BattleResult, RoundEnd, fight};
use crate::object::{ObjectRef, Sight, flags};
use crate::content::BannerId;
use crate::setup::SetScore;
use crate::dimming::telop_banner;
use crate::hud::TelopHidden;
use nettai_content_api::ChipHandle;

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
    /// `sub_800EB6C` on `viewer`'s console: its player sees `alliance`'s
    /// objects unless they are the other side's and its navi
    /// (`sub_80103BC`) is blind. A navi whose init hasn't run yet has no
    /// collision data: the game reads its status word through the null
    /// pointer, from the BIOS, which gives the opcode it last fetched (the
    /// game's `missing_collision_status`: BN6's has the blind bit,
    /// BN5's hasn't). It happens on a round's first tick when the other
    /// side's navi inits first and its FirstBarrier's visual asks.
    pub fn sees(&self, viewer: u8, alliance: u8) -> bool {
        use crate::collision::f1;
        if alliance & 1 == viewer & 1 {
            return true;
        }
        let Some(navi) = self.player(viewer & 1) else { return true };
        let missing = self.game_rules().missing_collision_status.0;
        let status = self.objects.get(navi).collision.map_or(missing, |c| self.collision.get(c).f1);
        status & f1::BLIND == 0
    }

    /// Whether `viewer`'s player sees object `r`: its `VISIBLE` flag, or
    /// what a rule decided for that viewer ([`Object::sight`]).
    ///
    /// [`Object::sight`]: crate::object::Object::sight
    pub fn visible_to(&self, r: ObjectRef, viewer: u8) -> bool {
        let o = self.objects.get(r);
        match o.sight {
            Sight::Shared => o.flags & flags::VISIBLE != 0,
            Sight::ByViewer(shown) => shown[viewer as usize & 1],
        }
    }

    /// `r`'s header flags as `viewer`'s console has them: its `VISIBLE`
    /// flag the viewer's (the others are the same for every viewer).
    pub fn flags_for(&self, r: ObjectRef, viewer: u8) -> u8 {
        let shown = if self.visible_to(r, viewer) { flags::VISIBLE } else { 0 };
        self.objects.get(r).flags & !flags::VISIBLE | shown
    }

    /// `r` shown to the viewers `shown` says (by side). Its `VISIBLE` flag
    /// is the local side's.
    pub fn set_visible_by_viewer(&mut self, r: ObjectRef, shown: [bool; 2]) {
        let local = self.round.local_side as usize & 1;
        let o = self.objects.get_mut(r);
        o.set_visible(shown[local]);
        if shown[0] != shown[1] {
            o.sight = Sight::ByViewer(shown);
        }
    }

    /// `r` hidden from the viewers `hidden` names (by side), on top of what
    /// each sees of it now: where a console of the original clears
    /// `VISIBLE` by a rule of its own.
    pub fn hide_from(&mut self, r: ObjectRef, hidden: [bool; 2]) {
        let shown = [0u8, 1].map(|v| self.visible_to(r, v) && !hidden[v as usize]);
        self.set_visible_by_viewer(r, shown);
    }

    /// `r` hidden from a viewer blind to its side ([`Battle::sees`] on
    /// each console).
    pub fn hide_from_blind(&mut self, r: ObjectRef) {
        let alliance = self.objects.get(r).alliance;
        let hidden = [0u8, 1].map(|v| !self.sees(v, alliance));
        self.hide_from(r, hidden);
    }

    /// `r` shown on its own side's console only (`!is_remote` in the
    /// original: a Beast Out lock-on marker, an A-button charge glow).
    pub fn hide_from_other_side(&mut self, r: ObjectRef) {
        let alliance = self.objects.get(r).alliance & 1;
        self.hide_from(r, [0u8, 1].map(|v| v != alliance));
    }

    /// `to` seen by whoever sees `from`: an overlay or attachment that takes
    /// its owner's visibility.
    pub fn copy_visibility(&mut self, from: ObjectRef, to: ObjectRef) {
        let shown = [0u8, 1].map(|v| self.visible_to(from, v));
        self.set_visible_by_viewer(to, shown);
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
            RoundEnd::Error(message) => RoundEnd::Error(message),
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
        let navi = b.content.navi(b.content.navi_by_key(testing::MEGAMAN)).clone();
        b.start_banner(navi.win_banner);
        assert_eq!((b.banner_for(0), b.banner_for(1)), (Some(navi.win_banner), Some(navi.lose_banner)));
    }

    /// Each viewer sees the objects as its own console would: a blind
    /// player doesn't see the other side's, whoever the local side is, and
    /// a marker shows on its owner's console only. The `VISIBLE` flag stays
    /// the local side's.
    #[test]
    fn each_viewer_sees_what_its_console_would() {
        use crate::collision::f1;
        let mut b = battle();
        let no_buttons = [crate::input::PlayerTick::default(); 2];
        for _ in 0..4 {
            b.tick(&no_buttons, Default::default());
        }
        let navi = |b: &Battle, side: u8| b.player(side).expect("both navis are in");
        let (n0, n1) = (navi(&b, 0), navi(&b, 1));
        let blind = |b: &mut Battle, side: u8, on: bool| {
            let c = b.objects.get(navi(b, side)).collision.unwrap();
            let f = &mut b.collision.get_mut(c).f1;
            *f = if on { *f | f1::BLIND } else { *f & !f1::BLIND };
        };
        for r in [n0, n1] {
            b.objects.get_mut(r).set_visible(true);
        }
        // Side 1's player is blind: on its console, side 0's navi is gone.
        blind(&mut b, 1, true);
        assert!(!b.sees(1, 0) && b.sees(1, 1) && b.sees(0, 1));
        for r in [n0, n1] {
            b.hide_from_blind(r);
        }
        let seen = |b: &Battle| [n0, n1].map(|r| [0, 1].map(|viewer| b.visible_to(r, viewer)));
        assert_eq!(seen(&b), [[true, false], [true, true]]);
        // The flag is the local side's (side 0, who sees both).
        assert!(b.objects.get(n0).flags & flags::VISIBLE != 0);
        assert_eq!(b.flags_for(n0, 1) & flags::VISIBLE, 0);
        // Showing it again shows it to everyone.
        b.objects.get_mut(n0).set_visible(true);
        blind(&mut b, 1, false);
        blind(&mut b, 0, true);
        for r in [n0, n1] {
            b.hide_from_blind(r);
        }
        assert_eq!(seen(&b), [[true, true], [false, true]]);
        assert_eq!(b.objects.get(n1).flags & flags::VISIBLE, 0);
        // Something following side 1's navi is seen by whoever sees it.
        let follower = n0;
        b.copy_visibility(n1, follower);
        assert_eq!(seen(&b)[0], [false, true]);
        // A marker of side 1's shows on side 1's console only.
        b.objects.get_mut(follower).set_visible(true);
        b.objects.get_mut(follower).alliance = 1;
        b.hide_from_other_side(follower);
        assert_eq!(seen(&b)[0], [false, true]);
    }

    #[test]
    fn the_other_side_lost_what_the_local_side_won() {
        assert_eq!(BattleResult::Won.for_other_side(), BattleResult::Lost);
        let s = SetScore { wins: 2, losses: 1, round: 3, max_combo: 4 };
        assert_eq!(s.for_other_side(), SetScore { wins: 1, losses: 2, round: 3, max_combo: 4 });
    }
}
