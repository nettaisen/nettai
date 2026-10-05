//! Live play's random match (docs/frontend.md §2): a link battle's field,
//! and for each side a legal random folder, five Crosses of both versions
//! and a version, on MegaMan at his fresh stats (100 HP, as a new match's:
//! `Side::fresh`) with no NaviCust programs, all drawn from a seed. That is
//! EXE6's (its Crosses and versions); another game's is a plain match
//! ([`plain`]) of its own. The draw is the frontend's,
//! made before the battle; the battle is then a function of its setup and
//! the buttons, as rollback needs. The same seed gives the same match,
//! which can be written out as a match file (`crate::file`) and played
//! again or edited.

use crate::folders;
use crate::{Arena, Match, Place, Side, ids};
use nettai_battle::Battle;
use std::sync::Arc;
use nettai_battle::content::Content;
use exe6_compat::CrossList;
use nettai_battle::custom::{FolderChip, GameVersion, SavedFolder};
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

/// The backgrounds an EXE6 link battle draws from (`sub_81209DC`'s
/// `byte_8120A20`, by name in EXE6's pack; some are there twice, so twice
/// as likely).
const EXE6_LINK_BACKGROUNDS: [&str; 21] = [
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

/// The backgrounds a link battle of `game` draws from (EXE6's; another
/// game's shows its stage's own).
fn link_backgrounds(game: &str) -> &'static [&'static str] {
    if game == exe6_compat::ROOT { &EXE6_LINK_BACKGROUNDS } else { &[] }
}

/// Whether live play draws `game`'s match with Crosses and a version
/// (EXE6's).
fn draws_live(game: &str) -> bool {
    game == exe6_compat::ROOT
}

/// A link battle's arena of `game`, drawn from `draws`: its stage and
/// background, then the later rounds'; `stage` forces the first round's
/// stage.
pub fn arena(content: &Content, game: &str, draws: &mut Draws, stage: Option<StageHandle>) -> Result<Arena, String> {
    crate::playable(content, game)?;
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
    Ok(Arena { game: game.to_string(), first, later })
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
    picked.truncate(exe6_compat::unlocks::CROSSES);
    picked.sort();
    Ok(CrossList::new(&picked.iter().map(|&i| all[i]).collect::<Vec<_>>()))
}

impl Side {
    /// A live player of EXE6 on `arena`: a new match's side (`Side::fresh`:
    /// MegaMan at his fresh stats, a NaviCust with no programs) of
    /// `version`, with this folder and Cross list.
    pub fn live(content: &Content, arena: &Arena, folder: SavedFolder, crosses: CrossList, version: GameVersion) -> Result<Side, String> {
        let mut side = Side::fresh(content, arena)?;
        side.game = version;
        side.stats = Side::base_stats(content, side.navi, version);
        side.folder = folder.into();
        side.crosses = Some(crosses);
        Ok(side)
    }

    /// A player of a match of `game` drawn from
    /// `draws` as netplay draws one: EXE6's a random folder the rules accept,
    /// five Crosses of both versions, a version, no patch cards; another
    /// game's a plain side's ([`plain`]).
    pub fn drawn(content: &Arc<Content>, game: &str, draws: &mut Draws) -> Result<Side, String> {
        let stage = *crate::link_battle_stages(content, game).first().ok_or_else(|| format!("{game} has no link battle stage"))?;
        crate::playable(content, game)?;
        let arena = Arena::on(game, Place { stage, background: None });
        if !draws_live(game) {
            return plain_side(content, &arena, draws);
        }
        let mut rules = rules_battle(content, &arena)?;
        let folder = folders::random_folder(content, game, &mut rules, 0, draws);
        let crosses = crosses(content, game, draws)?;
        let version = version(draws);
        Side::live(content, &arena, folder, crosses, version)
    }
}

/// A plain side on `arena`: the game's first navi with fresh stats, its
/// fresh stats, and a folder of the rules' pool drawn from `draws` (else
/// the game's first chip with a code, thirty times); where the game has
/// computer navis (EXE5's), the computer-navi data the game would have
/// learned from a player who used each chip of that folder once
/// (`ComputerNavi::of_folder`), so that a drawn match states what a
/// computer navi plays.
fn plain_side(content: &Arc<Content>, arena: &Arena, draws: &mut Draws) -> Result<Side, String> {
    let game = &arena.game;
    let navi = *crate::navis(content, game).first().ok_or_else(|| format!("{game} has no navi with fresh stats"))?;
    let chip = (0..content.defs.chips.len() as u16)
        .map(nettai_content_api::ChipHandle)
        .find(|&c| !content.chip(c).codes.is_empty() && ids::in_game(content, game, &content.defs.chip(c).key))
        .ok_or_else(|| format!("{game} has no chip with a code"))?;
    let folder = SavedFolder { chips: [FolderChip::new(chip, content.chip(chip).codes[0]); 30], regular: None, tags: None };
    let version = GameVersion::Falzar;
    let mut side = Side {
        navi,
        game: version,
        stats: Side::base_stats(content, navi, version),
        folder: folder.into(),
        crosses: None,
        beast_out: true,
        cards: Vec::new(),
        navi_level: crate::default_navi_level(content, navi),
        bug_frags: 0,
        sp_times: Default::default(),
        navicust: None,
        computer_navi: Default::default(),
        karma: crate::facts::DEFAULT_KARMA,
        souls: None,
        soul_unison: true,
        chaos_unison: true,
    };
    let m = Match { seed: None, arena: arena.clone(), sides: [side.clone(), side.clone()] };
    if let Ok(mut b) = crate::check::start(content, &m)
        && !folders::pool(content, game, &mut b, 0).is_empty()
    {
        side.folder = folders::random_folder(content, game, &mut b, 0, draws).into();
    }
    // What a computer navi plays from the side's save, where the game has
    // computer navis: what the game would have learned from this folder.
    if crate::computer_navi::has(content) {
        side.computer_navi = crate::ComputerNavi::of_folder(content, &side.folder);
    }
    Ok(side)
}

/// A plain match of `game`, for a game live play draws none of (EXE5's): an arena drawn from `seed` (`stage` forces the first
/// round's stage), and on both sides a plain side (`plain_side`), each its
/// own folder.
pub fn plain(content: &Arc<Content>, game: &str, seed: u32, stage: Option<StageHandle>) -> Result<Match, String> {
    let mut draws = Draws::new(seed);
    let arena = arena(content, game, &mut draws, stage)?;
    let sides = [plain_side(content, &arena, &mut draws)?, plain_side(content, &arena, &mut draws)?];
    Ok(Match { seed: Some(seed), arena, sides })
}

/// The battle a live player's folder is drawn against: two live players
/// on `arena` (their folders anything: the rules read the stats).
fn rules_battle(content: &Arc<Content>, arena: &Arena) -> Result<Battle, String> {
    let anything = SavedFolder { chips: [FolderChip::new(Default::default(), nettai_battle::content::ChipCode(0)); 30], regular: None, tags: None };
    let side = Side::live(content, arena, anything, CrossList::default(), GameVersion::Falzar)?;
    crate::check::start(content, &Match { seed: None, arena: arena.clone(), sides: [side.clone(), side] })
}

/// Live play's match of `game`, drawn from `seed`.
/// EXE6's: a link battle's stage (`stage` forces one) and background, a
/// random folder each player's rules accept (`crate::folders`), five of
/// MegaMan's ten Crosses, of both versions, for each Cross window
/// (`Unlocks::cross_list`, docs/engine/custom-screen.md §4.1), and each
/// player's version, Falzar or Gregar; both players are MegaMen at their
/// fresh stats (100 HP). Another game's is [`plain`].
pub fn live(content: &Arc<Content>, game: &str, seed: u32, stage: Option<StageHandle>) -> Result<Match, String> {
    if !draws_live(game) {
        return plain(content, game, seed, stage);
    }
    let mut draws = Draws::new(seed);
    let arena = arena(content, game, &mut draws, stage)?;
    let mut rules = rules_battle(content, &arena)?;
    let folders =
        [folders::random_folder(content, game, &mut rules, 0, &mut draws), folders::random_folder(content, game, &mut rules, 1, &mut draws)];
    let crosses = [crosses(content, game, &mut draws)?, crosses(content, game, &mut draws)?];
    let versions = [version(&mut draws), version(&mut draws)];
    let side = |side: usize| Side::live(content, &arena, folders[side], crosses[side], versions[side]);
    let sides = [side(0)?, side(1)?];
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
        let content = crate::testing::exe6_content();
        // The navi: MegaMan at his fresh stats, as a new match's, with no
        // NaviCust programs (road panels carry him).
        let m = live(&content, "exe6", 0, None).unwrap();
        let fresh = &crate::Match::empty(&content, "exe6").unwrap().sides[0];
        for side in &m.sides {
            let s = &side.stats;
            assert_eq!((side.navi, side.navicust, side.navi_level), (fresh.navi, fresh.navicust, fresh.navi_level));
            assert_eq!(*s, Side::base_stats(&content, side.navi, side.game));
            assert_eq!((s.hp, s.max_hp), (100, 100));
            assert!(!s.float_shoes && !s.air_shoes && !s.undershirt && !s.super_armor && !s.chip_shuffle && !s.number_open);
        }
        let stages = crate::link_battle_stages(&content, "exe6");
        assert_eq!(stages.len(), 96);
        let mut seen = std::collections::BTreeSet::new();
        let navi = content.form_changing_navi().unwrap();
        for seed in 0..12 {
            let m = live(&content, "exe6", seed, None).unwrap();
            let setup = m.round(&content, seed);
            assert!(stages.contains(&setup.settings.stage));
            assert_eq!(setup.settings.effects & effects::RANDOM, 0);
            seen.insert(setup.settings.stage);
            let mut b = crate::check::start(&content, &m).unwrap();
            for side in 0..2 {
                assert!(folders::problems(&mut b, side as u8, &m.sides[side].folder).is_empty());
                let unlocks = exe6_compat::Unlocks::of(&b, side as u8).unwrap();
                let list = unlocks.cross_list.unwrap();
                assert_eq!(list.forms().count(), 5);
                for f in list.forms() {
                    let crosses = |g| exe6_compat::forms::set(&content, navi, g).unwrap().crosses;
                    assert!(crosses(GameVersion::Gregar).contains(&f) || crosses(GameVersion::Falzar).contains(&f));
                }
                assert_eq!(unlocks.version, m.sides[side].game);
                assert_eq!(setup.navi_stats[side].version, crate::version_byte(m.sides[side].game));
            }
            assert_eq!(live(&content, "exe6", seed, None).unwrap(), m);
            assert!(crate::check_match(&content, &m).is_empty(), "{:?}", crate::check_match(&content, &m));
        }
        assert!(seen.len() > 6, "{seen:?}");
        // Both games come up, and some seed offers both games' Crosses.
        let games: std::collections::BTreeSet<String> =
            (0..12).flat_map(|seed| live(&content, "exe6", seed, None).unwrap().sides.map(|s| format!("{:?}", s.game))).collect();
        assert_eq!(games.len(), 2);
        let mixed = (0..12).any(|seed| {
            let list = live(&content, "exe6", seed, None).unwrap().sides[0].crosses.unwrap();
            let gregar = list.forms().filter(|&f| exe6_compat::forms::game(&content, f) == Some(GameVersion::Gregar)).count();
            gregar > 0 && gregar < 5
        });
        assert!(mixed);
        let forced = live(&content, "exe6", 3, Some(crate::link_stage(&content, "exe6", "netbattle-43").unwrap())).unwrap();
        assert_eq!(ids::local(&content.defs.stage(forced.arena.first.stage).key), "netbattle-43");
        assert_eq!(forced.sides, live(&content, "exe6", 3, None).unwrap().sides);
        assert!(crate::link_stage(&content, "exe6", "netbattle-100").is_err());
        // Another game's name is none of this game's.
        assert!(crate::link_stage(&content, "exe6", "exe6:netbattle-43").is_err()); // (written in full)
    }

    /// A plain match is one the checks accept (its folder the rules' draw),
    /// and live play of EXE5 is one, of EXE5's alone.
    #[test]
    fn a_plain_match_is_legal() {
        let content = crate::testing::exe6_content();
        let m = plain(&content, "exe6", 4, None).unwrap();
        assert_eq!(crate::check_match(&content, &m), Vec::<String>::new());
        assert_ne!(m.sides[0].folder.chips[0], m.sides[0].folder.chips[1], "a drawn folder");
        let content = crate::testing::exe5_content();
        let m = live(&content, "exe5", 4, None).unwrap();
        assert_eq!(crate::check_match(&content, &m), Vec::<String>::new());
        assert_eq!(m.arena.game, "exe5");
        assert!(m.sides.iter().flat_map(|s| s.folder.chips()).all(|c| ids::in_game(&content, "exe5", &content.defs.chip(c.id).key)));
        // An EXE5 match states what a computer navi plays: each side's
        // folder's chips, as the game would have learned them (a folder's
        // own chips alone, none in the first three places and no
        // patterns), written in its file.
        use crate::computer_navi::{Entry, LISTS, PATTERNS, Record};
        for s in &m.sides {
            assert!(s.computer_navi.entries() > 0);
            assert!(s.computer_navi.list(&LISTS[0]).iter().chain(s.computer_navi.list(&PATTERNS)).all(|e| *e == Entry::Empty));
            for e in s.computer_navi.places.iter().filter(|e| **e != Entry::Empty) {
                let Entry::Chip(c) = e else { panic!("a drawn side has chips alone: {e:?}") };
                assert!(s.folder.chips().any(|f| f.id == *c));
            }
            assert_eq!(s.computer_navi.records, [Record::ZERO; 8]);
            assert_eq!(s.computer_navi, crate::ComputerNavi::of_folder(&content, &s.folder));
        }
        assert_ne!(m.sides[0].computer_navi, m.sides[1].computer_navi);
        let text = crate::write(&content, &m);
        assert!(text.contains("[left.computer_navi]\nfirst = [{}, {}, {}]\nstandard = [\n") && text.contains("[right.computer_navi]"), "{text}");
        assert_eq!(crate::parse(&content, &text).unwrap(), m);
        // EXE6 has no computer navis: a drawn match of it states none.
        let six = crate::testing::exe6_content();
        assert!(live(&six, "exe6", 4, None).unwrap().sides.iter().all(|s| s.computer_navi.is_blank()));
    }
}
