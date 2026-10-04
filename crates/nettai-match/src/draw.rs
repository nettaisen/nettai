//! Live play's random match (docs/frontend.md §2): a link battle's field,
//! and for each side a legal random folder, five Crosses of both versions
//! and a version, on a 1000-HP MegaMan with no NaviCust programs, all drawn
//! from a seed. That is BN6's (its live navi and Crosses); another game's
//! is a plain match ([`plain`]) of its own. The draw is the frontend's,
//! made before the battle; the battle is then a function of its setup and
//! the buttons, as rollback needs. The same seed gives the same match,
//! which can be written out as a match file (`crate::file`) and played
//! again or edited.

use crate::folders;
use crate::{Arena, Match, Place, Side, ids};
use nettai_battle::Battle;
use std::sync::Arc;
use bn6_compat::{Compat, codec};
use nettai_battle::content::Content;
use bn6_compat::CrossList;
use nettai_battle::custom::{FolderChip, GameVersion, SavedFolder};
use nettai_battle::setup::NaviStats;
use nettai_content_api::{RulesetHandle, StageHandle};

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

/// The backgrounds a BN6 link battle draws from (`sub_81209DC`'s
/// `byte_8120A20`, by name in BN6's pack; some are there twice, so twice
/// as likely).
const BN6_LINK_BACKGROUNDS: [&str; 21] = [
    "honeycomb",
    "statues",
    "statues",
    "seals",
    "clouds",
    "sprouts",
    "calendar-checkers",
    "calendar-mint",
    "calendar-lavender",
    "calendar-navy",
    "calendar-blue",
    "calendar-cyan",
    "trees",
    "calendar-green",
    "calendar-bright-blue",
    "code",
    "globes",
    "code-2",
    "code-2",
    "calendar-purple",
    "calendar-purple",
];

/// The backgrounds a link battle of `game` draws from (BN6's; another
/// game's shows its stage's own).
fn link_backgrounds(game: &str) -> &'static [&'static str] {
    if game == bn6_compat::ROOT { &BN6_LINK_BACKGROUNDS } else { &[] }
}

/// Whether live play draws `game`'s match from its live navi (BN6's).
fn draws_live(game: &str) -> bool {
    game == bn6_compat::ROOT
}

/// A link battle's arena of `game` by `ruleset`, drawn from `draws`: its
/// stage and background, then the later rounds'; `stage` forces the first
/// round's stage.
pub fn arena(content: &Content, game: &str, ruleset: RulesetHandle, draws: &mut Draws, stage: Option<StageHandle>) -> Result<Arena, String> {
    let stages = crate::link_battle_stages(content, game);
    if stages.is_empty() {
        return Err(format!("{game} has no link battle stage"));
    }
    let backgrounds: Vec<&str> = link_backgrounds(game).iter().copied().filter(|b| crate::background(content, game, b).is_some()).collect();
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
    Ok(Arena { game: game.to_string(), ruleset, first, later })
}

/// A version drawn at random, Gregar or Falzar.
fn version(draws: &mut Draws) -> GameVersion {
    if draws.below(2) == 0 { GameVersion::Gregar } else { GameVersion::Falzar }
}

/// Five of `game`'s form-changing navi's Crosses of both versions, drawn at
/// random, listed in the versions' order (Gregar's, then Falzar's).
fn crosses(content: &Content, game: &str, draws: &mut Draws) -> Result<CrossList, String> {
    let all = crate::all_crosses(content, game)?;
    let mut picked: Vec<usize> = (0..all.len()).collect();
    draws.shuffle(&mut picked);
    picked.truncate(bn6_compat::unlocks::CROSSES);
    picked.sort();
    Ok(CrossList::new(&picked.iter().map(|&i| all[i]).collect::<Vec<_>>()))
}

impl Side {
    /// A live player of BN6: the live navi (`live_navi`) of `game`, with
    /// this folder and Cross list, no patch cards.
    pub fn live(content: &Content, folder: SavedFolder, crosses: CrossList, game: GameVersion) -> Side {
        let stats = crate::starting(content, live_navi(content), game);
        Side {
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

    /// A player of a match of `game` by its stock rules, drawn from
    /// `draws` as netplay draws one: BN6's a random folder the rules accept,
    /// five Crosses of both versions, a version, no patch cards; another
    /// game's a plain side's ([`plain`]).
    pub fn drawn(content: &Arc<Content>, game: &str, draws: &mut Draws) -> Result<Side, String> {
        let stage = *crate::link_battle_stages(content, game).first().ok_or_else(|| format!("{game} has no link battle stage"))?;
        let arena = Arena::on(game, crate::stock_ruleset(content, game)?, Place { stage, background: None });
        if !draws_live(game) {
            return plain_side(content, &arena, draws);
        }
        let mut rules = rules_battle(content, &arena)?;
        let folder = folders::random_folder(content, game, &mut rules, 0, draws);
        let crosses = crosses(content, game, draws)?;
        let version = version(draws);
        Ok(Side::live(content, folder, crosses, version))
    }
}

/// A plain side on `arena`: the game's first navi with fresh stats, its
/// fresh stats, and a folder of the rules' pool drawn from `draws` (else
/// the game's first chip with a code, thirty times).
fn plain_side(content: &Arc<Content>, arena: &Arena, draws: &mut Draws) -> Result<Side, String> {
    let game = &arena.game;
    let navi = *crate::navis(content, game).first().ok_or_else(|| format!("{game} has no navi with fresh stats"))?;
    let chip = (0..content.defs.chips.len() as u16)
        .map(nettai_content_api::ChipHandle)
        .find(|&c| !content.chip(c).codes.is_empty() && ids::in_game(game, &content.defs.chip(c).key))
        .ok_or_else(|| format!("{game} has no chip with a code"))?;
    let folder = SavedFolder { chips: [FolderChip::new(chip, content.chip(chip).codes[0]); 30], regular: None, tags: None };
    let version = GameVersion::Falzar;
    let mut side = Side {
        navi,
        game: version,
        stats: Side::base_stats(content, navi, version),
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
    let m = Match { seed: None, arena: arena.clone(), sides: [side.clone(), side.clone()] };
    if let Ok(mut b) = crate::check::start(content, &m)
        && !folders::pool(content, game, &mut b, 0).is_empty()
    {
        side.folder = folders::random_folder(content, game, &mut b, 0, draws).into();
    }
    Ok(side)
}

/// A plain match of `game`, for a game live play draws none of (BN5's): its
/// stock rules, an arena drawn from `seed` (`stage` forces the first
/// round's stage), and on both sides a plain side (`plain_side`), each its
/// own folder.
pub fn plain(content: &Arc<Content>, game: &str, seed: u32, stage: Option<StageHandle>) -> Result<Match, String> {
    let mut draws = Draws::new(seed);
    let arena = arena(content, game, crate::stock_ruleset(content, game)?, &mut draws, stage)?;
    let sides = [plain_side(content, &arena, &mut draws)?, plain_side(content, &arena, &mut draws)?];
    Ok(Match { seed: Some(seed), arena, sides })
}

/// The battle a live player's folder is drawn against: two live navis on
/// `arena` (their folders anything: the rules read the stats).
fn rules_battle(content: &Arc<Content>, arena: &Arena) -> Result<Battle, String> {
    let anything = SavedFolder { chips: [FolderChip::new(Default::default(), nettai_battle::content::ChipCode(0)); 30], regular: None, tags: None };
    let side = Side::live(content, anything, CrossList::default(), GameVersion::Falzar);
    crate::check::start(content, &Match { seed: None, arena: arena.clone(), sides: [side.clone(), side] })
}

/// Live play's match of `game`, by its stock rules, drawn from `seed`.
/// BN6's: a link battle's stage (`stage` forces one) and background, a
/// random folder each player's rules accept (`crate::folders`), five of
/// MegaMan's ten Crosses, of both versions, for each Cross window
/// (`Unlocks::cross_list`, docs/engine/custom-screen.md §4.1), and each
/// player's version, Falzar or Gregar; both players are 1000-HP MegaMen
/// (`live_navi`). Another game's is [`plain`].
pub fn live(content: &Arc<Content>, game: &str, seed: u32, stage: Option<StageHandle>) -> Result<Match, String> {
    if !draws_live(game) {
        return plain(content, game, seed, stage);
    }
    let mut draws = Draws::new(seed);
    let arena = arena(content, game, crate::stock_ruleset(content, game)?, &mut draws, stage)?;
    let mut rules = rules_battle(content, &arena)?;
    let folders =
        [folders::random_folder(content, game, &mut rules, 0, &mut draws), folders::random_folder(content, game, &mut rules, 1, &mut draws)];
    let crosses = [crosses(content, game, &mut draws)?, crosses(content, game, &mut draws)?];
    let versions = [version(&mut draws), version(&mut draws)];
    let sides = [0, 1].map(|side| Side::live(content, folders[side], crosses[side], versions[side]));
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
        let stages = crate::link_battle_stages(&content, "bn6");
        assert_eq!(stages.len(), 96);
        let mut seen = std::collections::BTreeSet::new();
        let navi = content.form_changing_navi().unwrap();
        for seed in 0..12 {
            let m = live(&content, "bn6", seed, None).unwrap();
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
                    let crosses = |g| bn6_compat::forms::set(&content, navi, g).unwrap().crosses;
                    assert!(crosses(GameVersion::Gregar).contains(&f) || crosses(GameVersion::Falzar).contains(&f));
                }
                assert_eq!(unlocks.version, m.sides[side].game);
                assert_eq!(setup.navi_stats[side].version, crate::version_byte(m.sides[side].game));
            }
            assert_eq!(live(&content, "bn6", seed, None).unwrap(), m);
            assert!(crate::check_match(&content, &m).is_empty(), "{:?}", crate::check_match(&content, &m));
        }
        assert!(seen.len() > 6, "{seen:?}");
        // Both games come up, and some seed offers both games' Crosses.
        let games: std::collections::BTreeSet<String> =
            (0..12).flat_map(|seed| live(&content, "bn6", seed, None).unwrap().sides.map(|s| format!("{:?}", s.game))).collect();
        assert_eq!(games.len(), 2);
        let mixed = (0..12).any(|seed| {
            let list = live(&content, "bn6", seed, None).unwrap().sides[0].crosses.unwrap();
            let gregar = list.forms().filter(|&f| bn6_compat::forms::game(&content, f) == Some(GameVersion::Gregar)).count();
            gregar > 0 && gregar < 5
        });
        assert!(mixed);
        let forced = live(&content, "bn6", 3, Some(crate::link_stage(&content, "bn6", "netbattle-43").unwrap())).unwrap();
        assert_eq!(ids::local(&content.defs.stage(forced.arena.first.stage).key), "netbattle-43");
        assert_eq!(forced.sides, live(&content, "bn6", 3, None).unwrap().sides);
        assert!(crate::link_stage(&content, "bn6", "netbattle-100").is_err());
        // Another game's name is none of this game's.
        assert!(crate::link_stage(&content, "bn6", "bn6:netbattle-43").is_err()); // (written in full)
    }

    /// A plain match is one the checks accept (its folder the rules' draw),
    /// and live play of BN5 is one, of BN5's alone.
    #[test]
    fn a_plain_match_is_legal() {
        let content = crate::testing::bn6_content();
        let m = plain(&content, "bn6", 4, None).unwrap();
        assert_eq!(crate::check_match(&content, &m), Vec::<String>::new());
        assert_ne!(m.sides[0].folder.chips[0], m.sides[0].folder.chips[1], "a drawn folder");
        let content = crate::testing::bn5_content();
        let m = live(&content, "bn5", 4, None).unwrap();
        assert_eq!(crate::check_match(&content, &m), Vec::<String>::new());
        assert_eq!(m.arena.game, "bn5");
        assert!(m.sides.iter().flat_map(|s| s.folder.chips()).all(|c| ids::in_game("bn5", &content.defs.chip(c.id).key)));
    }
}
