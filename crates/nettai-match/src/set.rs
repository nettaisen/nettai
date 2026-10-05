//! A set: the rounds of a match, one after another. [`Set`] holds what
//! every round of one is made from and knows how it goes on when a round
//! ends; whoever runs a set asks it (offline play of its battle, a netplay
//! peer of its settled state, so both peers go on alike), and none of them
//! decides it by itself.

use crate::{Match, Picks};
use nettai_battle::console::ConsoleSetup;
use nettai_battle::content::Content;
use nettai_battle::custom::{BattleFolder, SavedFolder};
use nettai_battle::setup::{BattleSettings, RoundSetup, SetScore};
use nettai_battle::{Battle, BattleResult, RoundEnd};
use std::sync::Arc;

/// A set being played: its first round, and the players' folders as they
/// saved them (by side), which every later round shuffles again.
#[derive(Clone)]
pub struct Set {
    content: Arc<Content>,
    first: RoundSetup,
    folders: [SavedFolder; 2],
}

/// How a set goes on after a round ([`Set::after`]).
#[derive(Clone, Debug)]
pub enum After {
    /// With another round: its battle, at its start.
    Round(Box<Battle>),
    /// It doesn't, the set is over: the result, for the side that asked.
    Over(BattleResult),
    /// It can't: the engine stopped the battle (`RoundEnd::Error`), and
    /// this is why.
    Stopped(String),
}

impl Set {
    /// The set that starts with the round `first`, between these players'
    /// folders (by side).
    pub fn new(content: Arc<Content>, first: RoundSetup, folders: [SavedFolder; 2]) -> Set {
        Set { content, first, folders }
    }

    /// The set a match plays from `seed` (`Match::round`). The match is a
    /// checked one: both folders are whole.
    pub fn of(content: &Arc<Content>, m: &Match, seed: u32) -> Set {
        let folders = [0, 1].map(|side| m.sides[side].folder.saved().expect("a whole folder (the match's checks refuse one being made)"));
        Set::new(content.clone(), m.round(content, seed), folders)
    }

    /// The content the set is played on.
    pub fn content(&self) -> &Arc<Content> {
        &self.content
    }

    /// The set's first round.
    pub fn first(&self) -> &RoundSetup {
        &self.first
    }

    /// The first round's battle, at its start.
    pub fn start(&self) -> Battle {
        Battle::new(self.first.clone(), self.content.clone())
    }

    /// How the set goes on after `battle`, a round of it, as `viewer`'s
    /// side sees it; none while the round is still being played. The next
    /// round is the simulation's (the same whoever asks: two netplay peers
    /// ask of the same settled state and start the same battle); the result
    /// of a set that is over is `viewer`'s.
    pub fn after(&self, battle: &Battle, viewer: u8) -> Option<After> {
        Some(match battle.round_end()? {
            &RoundEnd::NextRound { settings, score } => {
                After::Round(Box::new(Battle::new(next_round(&self.content, &self.first, &self.folders, battle, settings, score), self.content.clone())))
            }
            RoundEnd::Over(_) => match battle.round_end_for(viewer) {
                Some(RoundEnd::Over(result)) => After::Over(result),
                _ => unreachable!("a round's end is the same kind for either side"),
            },
            RoundEnd::Error(why) => After::Stopped(why.clone()),
        })
    }
}

/// A set's next round after `ended`, a round of `first`'s set: the
/// settings and score the round's end hands over, the players' folders
/// shuffled again by their consoles' RNG where the round left it (the
/// original's carries on through the next init's shuffle), the battle's
/// RNG picked from the first round's and the round's number.
fn next_round(content: &Content, first: &RoundSetup, folders: &[SavedFolder; 2], ended: &Battle, settings: BattleSettings, score: SetScore) -> RoundSetup {
    let mut next = first.clone();
    next.settings = settings;
    next.score = score;
    next.rng = Picks::new(first.rng ^ (score.round as u32) << 24).next() as u32;
    for (side, folder) in folders.iter().enumerate() {
        let console = &ended.consoles[side];
        let mut rng = console.rng;
        let (folder, tag_pair) = BattleFolder::shuffled_with_tag_pair(folder, 0, &mut rng, content);
        let p = &mut next.players[side];
        p.folder = folder;
        p.console = ConsoleSetup { rng: rng.state, tag_pair, frames: console.frames };
    }
    next
}
