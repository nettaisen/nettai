//! Live play's random match (docs/frontend.md §2): a link battle's field,
//! and for each side a legal random folder, five Crosses of both games and
//! a game, on a 1000-HP MegaMan with no NaviCust programs, all drawn from a
//! seed. The draw is the frontend's, made before the battle; the battle is
//! then a function of its setup and the buttons, as rollback needs. The
//! same seed gives the same match, which can be written out as a match file
//! (`crate::file`) and played again or edited.

use crate::folders;
use crate::{Arena, Match, Place, Side};
use nettai_battle::Battle;
use std::sync::Arc;
use bn6_compat::{Compat, codec};
use nettai_battle::content::Content;
use bn6_compat::CrossList;
use nettai_battle::custom::{FolderChip, GameVersion, SavedFolder};
use nettai_battle::setup::NaviStats;
use nettai_content_api::StageHandle;

/// The frontend's own random draws for a setup (splitmix64): not the
/// game's RNG, which the battle keeps.
#[derive(Clone, Debug)]
pub struct Draws(u64);

impl Draws {
    pub fn new(seed: u32) -> Draws {
        Draws(seed as u64 ^ 0x6E65_7474_6169_0000)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number below `n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    /// `items` in a random order.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            items.swap(i, self.below(i + 1));
        }
    }
}

/// The live navi's NaviStats record: a MegaMan of Falzar with 1000 HP,
/// custom level 5, Mega level 5, Giga level 1, Regular memory 50, three
/// Beast Outs, the sun out (+0x22, as in the recorded matches: the sun
/// chips hit harder), and no NaviCust programs: no FloatShoe or AirShoe
/// (+0x1B, +0x1C), so road panels carry him and holes stop him.
const LIVE_NAVI: &str = "08000000000100ff00320505010080000000ff00000000000000000000000001010301000000001f0000000a0000ffffff0000000000000000ff00000000e803e803e8030000010000000a0000000000000000000000ffffffffffff0000000000000000";

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

/// The live navi's stats on `content`.
pub fn live_navi(content: &Content) -> NaviStats {
    codec::navi_stats(&unhex(LIVE_NAVI).try_into().unwrap(), &codec::Ids::new(content, Compat::bn6_for(content)))
}

/// The backgrounds a link battle draws from (`sub_81209DC`'s
/// `byte_8120A20`, by name; some are there twice, so twice as likely).
/// BN6's: a link battle is.
const LINK_BACKGROUNDS: [&str; 21] = [
    "bn6:honeycomb",
    "bn6:statues",
    "bn6:statues",
    "bn6:seals",
    "bn6:clouds",
    "bn6:sprouts",
    "bn6:calendar-checkers",
    "bn6:calendar-mint",
    "bn6:calendar-lavender",
    "bn6:calendar-navy",
    "bn6:calendar-blue",
    "bn6:calendar-cyan",
    "bn6:trees",
    "bn6:calendar-green",
    "bn6:calendar-bright-blue",
    "bn6:code",
    "bn6:globes",
    "bn6:code-2",
    "bn6:code-2",
    "bn6:calendar-purple",
    "bn6:calendar-purple",
];

/// A link battle's arena drawn from `draws`: its stage and background, then
/// the later rounds'; `stage` forces the first round's stage.
pub fn arena(content: &Content, draws: &mut Draws, stage: Option<StageHandle>) -> Result<Arena, String> {
    let stages = crate::link_battle_stages(content);
    if stages.is_empty() {
        return Err("the content has no link battle stage".into());
    }
    let backgrounds: Vec<&str> = LINK_BACKGROUNDS.iter().copied().filter(|b| crate::background(content, b).is_some()).collect();
    let place = |draws: &mut Draws| {
        let stage = stages[draws.below(stages.len())];
        let background = (!backgrounds.is_empty()).then(|| backgrounds[draws.below(backgrounds.len())].to_string());
        Place { stage, background }
    };
    let mut first = place(draws);
    let later = [place(draws), place(draws)];
    if let Some(s) = stage {
        first.stage = s;
    }
    Ok(Arena { first, later })
}

/// A game drawn at random, Gregar or Falzar.
fn game(draws: &mut Draws) -> GameVersion {
    if draws.below(2) == 0 { GameVersion::Gregar } else { GameVersion::Falzar }
}

/// Five of the form-changing navi's Crosses of both games, drawn at
/// random, listed in the games' order (Gregar's, then Falzar's).
fn crosses(content: &Content, draws: &mut Draws) -> Result<CrossList, String> {
    let all = crate::all_crosses(content)?;
    let mut picked: Vec<usize> = (0..all.len()).collect();
    draws.shuffle(&mut picked);
    picked.truncate(bn6_compat::unlocks::CROSSES);
    picked.sort();
    Ok(CrossList::new(&picked.iter().map(|&i| all[i]).collect::<Vec<_>>()))
}

impl Side {
    /// A live player: the live navi (`live_navi`) of `game`, with this
    /// folder and Cross list, on the content's stock rules, no patch cards.
    pub fn live(content: &Content, folder: SavedFolder, crosses: CrossList, game: GameVersion) -> Side {
        let stats = crate::starting(content, live_navi(content), game);
        Side {
            ruleset: content.defs.stock_ruleset_of(crate::DEFAULT_GAME),
            navi: stats.navi,
            game,
            stats,
            emotion_window_glitch: false,
            folder: folder.into(),
            crosses: Some(crosses),
            beast_out: true,
            cards: Vec::new(),
            navi_level: crate::default_navi_level(content, stats.navi),
            bug_frags: 0,
            sp_times: Default::default(),
            navicust: None,
            tactics: Default::default(),
            karma: crate::facts::DEFAULT_KARMA,
            souls: None,
        }
    }

    /// A player drawn from `draws` as netplay draws one: a random folder
    /// the rules accept, five Crosses of both games, a game; no patch cards.
    pub fn drawn(content: &Arc<Content>, draws: &mut Draws) -> Result<Side, String> {
        let stage = *crate::link_battle_stages(content).first().ok_or("the content has no link battle stage")?;
        let mut rules = rules_battle(content, &Arena::on(Place { stage, background: None }))?;
        let folder = folders::random_folder(content, &mut rules, 0, draws);
        let crosses = crosses(content, draws)?;
        let game = game(draws);
        Ok(Side::live(content, folder, crosses, game))
    }
}

/// A plain match for content live play can't draw one from (no navi that
/// changes form): the first link battle stage, on both sides the content's
/// own root's first navi with its fresh stats and a folder of its rules'
/// pool drawn from `seed` (else its first chip with a code, thirty times),
/// on the content's stock rules.
pub fn plain(content: &Arc<Content>, seed: u32) -> Result<Match, String> {
    let stage = *crate::link_battle_stages(content).first().ok_or("the content has no link battle stage")?;
    let arena = Arena::on(Place { stage, background: None });
    let home = content.scripts.games().first().cloned().unwrap_or_default();
    let navi = (0..content.defs.navis.len() as u16)
        .map(nettai_content_api::NaviHandle)
        .filter(|&n| content.navi(n).fresh.is_some())
        .min_by_key(|&n| nettai_content_api::keys::root_of(&content.defs.navi(n).key) != Some(home.as_str()))
        .ok_or("the content has no navi with fresh stats")?;
    let chip = (0..content.defs.chips.len() as u16)
        .map(nettai_content_api::ChipHandle)
        .find(|&c| !content.chip(c).codes.is_empty())
        .ok_or("the content has no chip with a code")?;
    let folder = SavedFolder { chips: [FolderChip::new(chip, content.chip(chip).codes[0]); 30], regular: None, tags: None };
    let game = GameVersion::Falzar;
    let side = Side {
        ruleset: content.defs.stock_ruleset_of(crate::DEFAULT_GAME),
        navi,
        game,
        stats: Side::base_stats(content, navi, game),
        emotion_window_glitch: false,
        folder: folder.into(),
        crosses: None,
        beast_out: true,
        cards: Vec::new(),
        navi_level: crate::default_navi_level(content, navi),
        bug_frags: 0,
        sp_times: Default::default(),
        navicust: None,
        tactics: Default::default(),
        karma: crate::facts::DEFAULT_KARMA,
        souls: None,
    };
    let mut m = Match { seed: Some(seed), arena, sides: [side.clone(), side] };
    if let Ok(mut b) = crate::check::start(content, &m) {
        let mut draws = Draws::new(seed);
        for s in 0..2u8 {
            if !folders::pool(content, &mut b, s).is_empty() {
                m.sides[s as usize].folder = folders::random_folder(content, &mut b, s, &mut draws).into();
            }
        }
    }
    Ok(m)
}

/// The battle a live player's folder is drawn against: two live navis on
/// `arena` (their folders anything: the rules read the stats).
fn rules_battle(content: &Arc<Content>, arena: &Arena) -> Result<Battle, String> {
    let anything = SavedFolder { chips: [FolderChip::new(Default::default(), nettai_battle::content::ChipCode(0)); 30], regular: None, tags: None };
    let side = Side::live(content, anything, CrossList::default(), GameVersion::Falzar);
    crate::check::start(content, &Match { seed: None, arena: arena.clone(), sides: [side.clone(), side] })
}

/// Live play's match on BN6's content, drawn from `seed`: a link battle's
/// stage (`stage` forces one) and background, a random folder each
/// player's rules accept (`crate::folders`), five of MegaMan's ten
/// Crosses, of both games, for each Cross window (`Unlocks::cross_list`,
/// docs/engine/custom-screen.md §4.1), and each player's game, Falzar or
/// Gregar. Both players are 1000-HP MegaMen (`live_navi`).
pub fn live(content: &Arc<Content>, seed: u32, stage: Option<StageHandle>) -> Result<Match, String> {
    let mut draws = Draws::new(seed);
    let arena = arena(content, &mut draws, stage)?;
    let mut rules = rules_battle(content, &arena)?;
    let folders = [folders::random_folder(content, &mut rules, 0, &mut draws), folders::random_folder(content, &mut rules, 1, &mut draws)];
    let crosses = [crosses(content, &mut draws)?, crosses(content, &mut draws)?];
    let games = [game(&mut draws), game(&mut draws)];
    let sides = [0, 1].map(|side| Side::live(content, folders[side], crosses[side], games[side]));
    Ok(Match { seed: Some(seed), arena, sides })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_battle::setup::effects;

    /// Live play's match from a seed: a link battle's stage (the 96 the
    /// original draws from), legal folders, five Crosses of both games per
    /// window; the same seed, the same match; a forced stage.
    #[test]
    fn the_live_match_is_drawn_from_the_seed() {
        let content = crate::testing::bn6_content();
        // The navi: no NaviCust programs (road panels carry him).
        let s = live_navi(&content);
        assert_eq!((s.hp, s.mega_level, s.giga_level, s.reg_up), (1000, 5, 1, 50));
        assert!(!s.float_shoes && !s.air_shoes && !s.undershirt && !s.super_armor && !s.chip_shuffle && !s.number_open);
        let stages = crate::link_battle_stages(&content);
        assert_eq!(stages.len(), 96);
        let mut seen = std::collections::BTreeSet::new();
        let navi = content.form_changing_navi().unwrap();
        for seed in 0..12 {
            let m = live(&content, seed, None).unwrap();
            let setup = m.round(&content, seed);
            assert!(stages.contains(&setup.settings.stage));
            assert_eq!(setup.settings.effects & effects::RANDOM, 0);
            seen.insert(setup.settings.stage);
            let mut b = crate::check::start(&content, &m).unwrap();
            for side in 0..2 {
                assert!(folders::problems(&mut b, side as u8, &m.sides[side].folder).is_empty());
                let unlocks = bn6_compat::Unlocks::of(&b, side as u8).unwrap();
                let list = unlocks.cross_list.unwrap();
                assert_eq!(list.forms().count(), 5);
                for f in list.forms() {
                    let forms = content.navi(navi).forms.as_ref().unwrap();
                    assert!(forms.gregar.crosses.contains(&f) || forms.falzar.crosses.contains(&f));
                }
                assert_eq!(unlocks.version, m.sides[side].game);
                assert_eq!(setup.navi_stats[side].version, crate::version_byte(m.sides[side].game));
            }
            assert_eq!(live(&content, seed, None).unwrap(), m);
            assert!(crate::check_match(&content, &m).is_empty(), "{:?}", crate::check_match(&content, &m));
        }
        assert!(seen.len() > 6, "{seen:?}");
        // Both games come up, and some seed offers both games' Crosses.
        let games: std::collections::BTreeSet<String> =
            (0..12).flat_map(|seed| live(&content, seed, None).unwrap().sides.map(|s| format!("{:?}", s.game))).collect();
        assert_eq!(games.len(), 2);
        let mixed = (0..12).any(|seed| {
            let list = live(&content, seed, None).unwrap().sides[0].crosses.unwrap();
            let gregar = list.forms().filter(|&f| bn6_compat::forms::game(&content, f) == Some(GameVersion::Gregar)).count();
            gregar > 0 && gregar < 5
        });
        assert!(mixed);
        let forced = live(&content, 3, Some(crate::link_stage(&content, "bn6:netbattle-43").unwrap())).unwrap();
        assert_eq!(content.defs.stage(forced.arena.first.stage).key, "bn6:netbattle-43");
        assert_eq!(forced.sides, live(&content, 3, None).unwrap().sides);
        assert!(crate::link_stage(&content, "bn6:netbattle-100").is_err());
    }

    /// A plain match is one the checks accept (its folder the rules' draw).
    #[test]
    fn a_plain_match_is_legal() {
        let content = crate::testing::bn6_content();
        let m = plain(&content, 4).unwrap();
        assert_eq!(crate::check_match(&content, &m), Vec::<String>::new());
        assert_ne!(m.sides[0].folder.chips[0], m.sides[0].folder.chips[1], "a drawn folder");
    }
}
