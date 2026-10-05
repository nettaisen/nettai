//! Live play's random match (docs/frontend.md §2): a link battle's field,
//! and for each side a legal random folder, a form list of its navi's forms
//! of every version, and one variant of each fact its rules require (a
//! version), on MegaMan at his fresh stats (100 HP, as a new match's:
//! `Side::fresh`) with no NaviCust programs, all picked from a seed. That is
//! a game whose rules take a form list (EXE6's Cross window); another
//! game's is a plain match ([`plain`]) of its own. The pick is the frontend's,
//! made before the battle; the battle is then a function of its setup and
//! the buttons, as rollback needs. The same seed gives the same match,
//! which can be written out as a match file (`crate::file`) and played
//! again or edited.

use crate::folders;
use crate::{Arena, Match, Place, Side, ids};
use nettai_battle::Battle;
use std::sync::Arc;
use nettai_battle::content::{Content, PlayerFact};
use nettai_battle::custom::{FolderChip, SavedFolder};
use nettai_battle::rules::Fact;
use nettai_content_api::{FormHandle, Registry, StageHandle, Value};

/// The frontend's own random picks for a setup (splitmix64): not the
/// game's RNG, which the battle keeps.
#[derive(Clone, Debug)]
pub struct Picks(u64);

impl Picks {
    pub fn new(seed: u32) -> Picks {
        Picks(seed as u64 ^ 0x6E65_7474_6169_0000)
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


/// The backgrounds a link battle of the game picks from at random, by
/// name: its rules' (`link_pick.backgrounds`: EXE6's table, some there
/// twice and so twice as likely). None: a link battle shows its stage's
/// own.
fn link_backgrounds(content: &Content) -> Vec<&str> {
    content.rules().link_pick.backgrounds.iter().filter_map(|&b| ids::background_name(content, b)).collect()
}

/// Whether live play picks the game's match with a form list each side:
/// its rules take one (a system's setup declares the engine's
/// `PlayerFact::CrossList`: EXE6's cross system). A game whose rules take
/// none gets a plain match ([`plain`]).
fn picks_live(content: &Content) -> bool {
    content.defs.fact_field(PlayerFact::CrossList).is_some()
}

/// A link battle's arena of `game`, picked at random from `picks` as the
/// game picks one: its stage and background, then the later rounds';
/// `stage` forces the first round's stage. The stage is one of the game's
/// rules' `link_pick.stages`, the stage for each index of the original's
/// pick: a stage there twice is twice as likely, and a link battle stage
/// that isn't there is never picked (EXE5's records 76 to 87; a match may
/// still name one).
pub fn arena(content: &Content, game: &str, picks: &mut Picks, stage: Option<StageHandle>) -> Result<Arena, String> {
    crate::playable(content, game)?;
    let stages = &content.rules().link_pick.stages;
    if stages.is_empty() {
        return Err(format!("{game}'s rules state no stage a link battle picks (link_pick.stages)"));
    }
    let backgrounds = link_backgrounds(content);
    let place = |picks: &mut Picks| {
        let stage = stages[picks.below(stages.len())];
        let background = (!backgrounds.is_empty()).then(|| backgrounds[picks.below(backgrounds.len())].to_string());
        Place { stage, background }
    };
    let mut first = place(picks);
    let later = [place(picks), place(picks)];
    if let Some(s) = stage {
        first.stage = s;
    }
    Ok(Arena { game: game.to_string(), first, later })
}

/// What a random side states of the facts its rules require (the enums no
/// default states: EXE6's version, gregar or falzar): one variant of each,
/// picked at random, in the facts' order.
fn required(content: &Content, picks: &mut Picks) -> Vec<(String, String)> {
    crate::facts::required(content).into_iter().map(|(name, variants)| (name.to_string(), variants[picks.below(variants.len())].clone())).collect()
}

/// What live play's players' required facts are picked from, with the
/// seed: a stream of their own, picked before the folders (the rules a
/// folder is picked by are its player's).
const VERSION_SALT: u32 = 0x5645_5253;

/// A form list of `game`: as many as a list holds (EXE6's five) of its
/// form-changing navi's listed forms of every version, picked at random, in
/// the versions' order (Gregar's, then Falzar's).
fn form_list(content: &Content, game: &str, picks: &mut Picks) -> Result<Vec<FormHandle>, String> {
    let all = crate::listed_forms(content, game)?;
    let mut picked: Vec<usize> = (0..all.len()).collect();
    picks.shuffle(&mut picked);
    picked.truncate(crate::facts::form_list_capacity(content));
    picked.sort();
    Ok(picked.iter().map(|&i| all[i]).collect())
}

impl Side {
    /// A live player on `arena`: a new match's side (`Side::fresh`:
    /// MegaMan at his fresh stats, a NaviCust with no programs) that states
    /// `stated` (its rules' required facts, each a variant's name), with
    /// this folder and form list.
    pub fn live(content: &Content, arena: &Arena, folder: SavedFolder, forms: &[FormHandle], stated: &[(String, String)]) -> Result<Side, String> {
        let mut side = Side::fresh(content, arena)?;
        for (field, variant) in stated {
            side.set_fact(content, field, &[Fact::Name(variant)])?;
        }
        side.stats = Side::base_stats(content, side.navi, side.version(content));
        side.folder = folder.into();
        let list: Vec<Fact> = forms.iter().map(|f| Fact::Value(Value::Def(Registry::Form, f.0))).collect();
        side.set_fact(content, PlayerFact::CrossList.name(), &list)?;
        Ok(side)
    }

    /// A player of a match of `game` picked from `picks` as netplay picks
    /// one. Where the game's rules take a form list (EXE6's): what they
    /// require (a version), a random folder the rules accept, a form list,
    /// no patch cards. Else a plain side ([`plain_side`]).
    pub fn picked(content: &Arc<Content>, game: &str, picks: &mut Picks) -> Result<Side, String> {
        let stage = *crate::link_battle_stages(content, game).first().ok_or_else(|| format!("{game} has no link battle stage"))?;
        crate::playable(content, game)?;
        let arena = Arena::on(game, Place { stage, background: None });
        if !picks_live(content) {
            return plain_side(content, &arena, picks);
        }
        let stated = required(content, picks);
        let mut rules = rules_battle(content, &arena, [&stated, &stated])?;
        let folder = folders::random_folder(content, game, &mut rules, 0, picks);
        let forms = form_list(content, game, picks)?;
        Side::live(content, &arena, folder, &forms, &stated)
    }
}

/// A plain side on `arena`: the navi a new side operates (`first_navi`:
/// MegaMan), its fresh stats, what its rules require picked from `picks`
/// (a version, where they take one), the rest of its facts their defaults,
/// and a folder of the rules' pool picked from `picks` (else
/// the game's first chip with a code, thirty times); where the game has
/// navis in auto battle (EXE5's), the auto battle data the game would have
/// learned from a player who used each chip of that folder once
/// (`AutoBattle::of_folder`), so that a random match states what a
/// navi in auto battle plays.
fn plain_side(content: &Arc<Content>, arena: &Arena, picks: &mut Picks) -> Result<Side, String> {
    let game = &arena.game;
    let navi = crate::first_navi(content, game).ok_or_else(|| format!("{game} has no navi with fresh stats"))?;
    let chip = (0..content.defs.chips.len() as u16)
        .map(nettai_content_api::ChipHandle)
        .find(|&c| !content.chip(c).codes.is_empty() && ids::in_game(content, game, &content.defs.chip(c).key))
        .ok_or_else(|| format!("{game} has no chip with a code"))?;
    let folder = SavedFolder { chips: [FolderChip::new(chip, content.chip(chip).codes[0]); 30], regular: None, tags: None };
    let mut facts = crate::Facts::defaults(content);
    for (field, variant) in required(content, picks) {
        facts.set(content, &field, &[Fact::Name(&variant)])?;
    }
    let mut side = Side {
        navi,
        stats: Side::base_stats(content, navi, facts.version(content)),
        folder: folder.into(),
        cards: Vec::new(),
        navi_level: crate::default_navi_level(content, navi),
        sp_times: Default::default(),
        navicust: None,
        auto_battle: Default::default(),
        facts,
    };
    let m = Match { seed: None, arena: arena.clone(), sides: [side.clone(), side.clone()] };
    if let Ok(mut b) = crate::check::start(content, &m)
        && !folders::pool(content, game, &mut b, 0).is_empty()
    {
        side.folder = folders::random_folder(content, game, &mut b, 0, picks).into();
    }
    // What a navi in auto battle plays from the side's save, where the game has
    // navis in auto battle: what the game would have learned from this folder.
    if crate::auto_battle::has(content) {
        side.auto_battle = crate::AutoBattle::of_folder(content, &side.folder);
    }
    Ok(side)
}

/// A plain match of `game`, for a game live play picks none of (EXE5's): an arena picked from `seed` (`stage` forces the first
/// round's stage), and on both sides a plain side (`plain_side`), each its
/// own folder.
pub fn plain(content: &Arc<Content>, game: &str, seed: u32, stage: Option<StageHandle>) -> Result<Match, String> {
    let mut picks = Picks::new(seed);
    let arena = arena(content, game, &mut picks, stage)?;
    let sides = [plain_side(content, &arena, &mut picks)?, plain_side(content, &arena, &mut picks)?];
    Ok(Match { seed: Some(seed), arena, sides })
}

/// The battle live players' folders are picked against: two live players
/// on `arena` who state these (their folders anything: the rules read the
/// stats).
fn rules_battle(content: &Arc<Content>, arena: &Arena, stated: [&[(String, String)]; 2]) -> Result<Battle, String> {
    let anything = SavedFolder { chips: [FolderChip::new(Default::default(), nettai_battle::content::ChipCode(0)); 30], regular: None, tags: None };
    let side = |stated| Side::live(content, arena, anything, &[], stated);
    crate::check::start(content, &Match { seed: None, arena: arena.clone(), sides: [side(stated[0])?, side(stated[1])?] })
}

/// Live play's match of `game`, picked from `seed`.
/// Where the game's rules take a form list (EXE6's): a link battle's
/// stage (`stage` forces one) and background, a random folder each
/// player's rules accept (`crate::folders`), as many of the listed forms of
/// the navi that changes form as a list holds (five of MegaMan's ten
/// Crosses, of both versions) for each side's form list
/// (docs/engine/custom-screen.md §4.1), and what each player's rules
/// require picked too (a version, from a stream of its own from the seed:
/// none is assumed); both players are that navi at its fresh stats
/// (100 HP). Else it is [`plain`].
pub fn live(content: &Arc<Content>, game: &str, seed: u32, stage: Option<StageHandle>) -> Result<Match, String> {
    if !picks_live(content) {
        return plain(content, game, seed, stage);
    }
    let mut picks = Picks::new(seed);
    let arena = arena(content, game, &mut picks, stage)?;
    let mut own = Picks::new(seed ^ VERSION_SALT);
    let stated = [required(content, &mut own), required(content, &mut own)];
    let mut rules = rules_battle(content, &arena, [&stated[0], &stated[1]])?;
    let folders =
        [folders::random_folder(content, game, &mut rules, 0, &mut picks), folders::random_folder(content, game, &mut rules, 1, &mut picks)];
    let forms = [form_list(content, game, &mut picks)?, form_list(content, game, &mut picks)?];
    let side = |side: usize| Side::live(content, &arena, folders[side], &forms[side], &stated[side]);
    let sides = [side(0)?, side(1)?];
    Ok(Match { seed: Some(seed), arena, sides })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_battle::setup::effects;

    /// A random match's arena is picked as the game picks a link battle's
    /// (the rules' `link_pick`). EXE6's: its 96 link battle stages each
    /// once, in the original's record order, and 21 backgrounds, three of
    /// them twice. EXE5's: 96 indices over 82 stages, the first twelve
    /// records' twice (the original takes 76 off an index from 76 to 87)
    /// and the twelve those indices would have reached never, and 27
    /// backgrounds each once. A stage the pick never reaches is still a
    /// link battle stage a match may name.
    #[test]
    fn the_arena_is_picked_as_the_game_picks_it() {
        let name = |c: &Content, s: StageHandle| ids::local(&c.defs.stage(s).key).to_string();
        let six = crate::testing::exe6_content();
        let pick = &six.rules().link_pick;
        let names: Vec<String> = pick.stages.iter().map(|&s| name(&six, s)).collect();
        assert_eq!(names, (1..=96).map(|n| format!("netbattle-{n}")).collect::<Vec<_>>());
        let mut pool = crate::link_battle_stages(&six, "exe6");
        let mut listed = pick.stages.clone();
        (pool.sort(), listed.sort());
        assert_eq!(pool, listed, "every link battle stage, once");
        let backgrounds = link_backgrounds(&six);
        assert_eq!((backgrounds.len(), backgrounds[0], backgrounds[1], backgrounds[2]), (21, "lans-hp", "acdc-hp", "acdc-hp"));
        assert_eq!(backgrounds.iter().collect::<std::collections::BTreeSet<_>>().len(), 18);

        let five = crate::testing::exe5_content();
        let pick = &five.rules().link_pick;
        let names: Vec<String> = pick.stages.iter().map(|&s| name(&five, s)).collect();
        assert_eq!(names.len(), 96);
        assert_eq!(names[76..88], names[..12], "an index from 76 to 87 is the record 76 before it");
        assert_eq!((names[0].as_str(), names[75].as_str(), names[88].as_str(), names[95].as_str()), ("netbattle-1", "netbattle-74", "netbattle-87", "netbattle-94"));
        let reached: std::collections::BTreeSet<&String> = names.iter().collect();
        assert_eq!(reached.len(), 82);
        let never: Vec<String> = crate::link_battle_stages(&five, "exe5").into_iter().map(|s| name(&five, s)).filter(|n| !reached.contains(n)).collect();
        assert_eq!(never, (75..=86).map(|n| format!("netbattle-{n}")).collect::<Vec<_>>(), "records 76 to 87");
        assert!(crate::link_stage(&five, "exe5", "netbattle-80").is_ok(), "a match may name one all the same");
        let backgrounds = link_backgrounds(&five);
        assert_eq!((backgrounds.len(), backgrounds.iter().collect::<std::collections::BTreeSet<_>>().len()), (27, 27));
        // A random arena's rounds are of the pick's stages and backgrounds.
        for (content, game) in [(&six, "exe6"), (&five, "exe5")] {
            let pick = &content.rules().link_pick;
            let backgrounds = link_backgrounds(content);
            for seed in 0..40 {
                let a = arena(content, game, &mut Picks::new(seed), None).unwrap();
                for place in [&a.first, &a.later[0], &a.later[1]] {
                    assert!(pick.stages.contains(&place.stage), "{game} seed {seed}");
                    assert!(place.background.as_deref().is_some_and(|b| backgrounds.contains(&b)), "{game} seed {seed}: {:?}", place.background);
                }
            }
        }
    }

    /// Live play's match from a seed: a link battle's stage (the 96 the
    /// original picks from), legal folders, a form list of five Crosses of
    /// both games per side; the same seed, the same match; a forced stage.
    #[test]
    fn the_live_match_is_picked_from_the_seed() {
        let content = crate::testing::exe6_content();
        // The navi: MegaMan at his fresh stats, as a new match's, with no
        // NaviCust programs (road panels carry him).
        let m = live(&content, "exe6", 0, None).unwrap();
        let fresh = &crate::Match::empty(&content, "exe6").unwrap().sides[0];
        for side in &m.sides {
            let s = &side.stats;
            assert_eq!((side.navi, side.navicust, side.navi_level), (fresh.navi, fresh.navicust, fresh.navi_level));
            assert_eq!(*s, Side::base_stats(&content, side.navi, side.version(&content)));
            assert!(side.version(&content).is_some(), "a picked EXE6 side states its version");
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
                // (What the round's player brought, as the engine reads
                // it back: the side's list and version.)
                use nettai_battle::content::PlayerFact;
                let brought = b.fact(side as u8, PlayerFact::CrossList).unwrap();
                let list: Vec<_> = (0..crate::facts::form_list_capacity(&content)).filter_map(|k| brought.form(k)).collect();
                assert_eq!(list, m.sides[side].facts.form_list(&content));
                assert_eq!(list.len(), 5);
                let forms = content.navi(navi).forms.as_ref().unwrap();
                for f in &list {
                    assert!(forms.listed("gregar").contains(f) || forms.listed("falzar").contains(f));
                }
                assert_eq!(b.fact(side as u8, PlayerFact::Version).and_then(|f| f.name()), m.sides[side].version(&content));
                assert_eq!(b.fact(side as u8, PlayerFact::BeastOut).and_then(|f| f.flag()), Some(true));
                assert_eq!(setup.navi_stats[side].version, crate::version_byte(&content, m.sides[side].version(&content)));
                // (A picked side states its version and its form list, and
                // nothing else: the rest is its rules' defaults.)
                let stated: Vec<&str> =
                    crate::facts::fields(&content).iter().map(|f| f.name).filter(|n| !m.sides[side].facts.is_default(&content, n)).collect();
                assert_eq!(stated, ["cross_list", "version"]);
            }
            assert_eq!(live(&content, "exe6", seed, None).unwrap(), m);
            assert!(crate::check_match(&content, &m).is_empty(), "{:?}", crate::check_match(&content, &m));
        }
        assert!(seen.len() > 6, "{seen:?}");
        // Both games come up, and some seed offers both games' Crosses.
        let games: std::collections::BTreeSet<String> =
            (0..12).flat_map(|seed| live(&content, "exe6", seed, None).unwrap().sides.map(|s| format!("{:?}", s.version(&content)))).collect();
        assert_eq!(games.len(), 2);
        let mixed = (0..12).any(|seed| {
            let list = live(&content, "exe6", seed, None).unwrap().sides[0].facts.form_list(&content);
            let gregar = list.iter().filter(|&&f| content.form(f).version.as_deref() == Some("gregar")).count();
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

    /// A plain match is one the checks accept (its folder the rules' pick),
    /// and live play of EXE5 is one, of EXE5's alone.
    #[test]
    fn a_plain_match_is_legal() {
        let content = crate::testing::exe6_content();
        let m = plain(&content, "exe6", 4, None).unwrap();
        assert_eq!(crate::check_match(&content, &m), Vec::<String>::new());
        assert_ne!(m.sides[0].folder.chips[0], m.sides[0].folder.chips[1], "a picked folder");
        // (Its sides' versions picked: EXE6's rules take one.)
        assert!(m.sides.iter().all(|s| s.version(&content).is_some()));
        let content = crate::testing::exe5_content();
        let m = live(&content, "exe5", 4, None).unwrap();
        assert_eq!(crate::check_match(&content, &m), Vec::<String>::new());
        assert_eq!(m.arena.game, "exe5");
        // An EXE5 match has no version: its sides state none, their stats'
        // version byte is 0, and its file has no such key.
        assert!(m.sides.iter().all(|s| s.version(&content).is_none() && s.stats.version == 0));
        // (Nor anything else: EXE5's rules require no fact, and a random
        // side's are their defaults.)
        assert!(m.sides.iter().all(|s| s.facts == crate::Facts::defaults(&content)));
        assert!(!crate::write(&content, &m).contains("version"));
        assert!(m.sides.iter().flat_map(|s| s.folder.chips()).all(|c| ids::in_game(&content, "exe5", &content.defs.chip(c.id).key)));
        // An EXE5 match states what a navi in auto battle plays: each side's
        // folder's chips, as the game would have learned them (a folder's
        // own chips alone, none in the first three places and no
        // patterns), written in its file.
        use crate::auto_battle::{Entry, LISTS, PATTERNS, Record};
        for s in &m.sides {
            assert!(s.auto_battle.entries() > 0);
            assert!(s.auto_battle.list(&LISTS[0]).iter().chain(s.auto_battle.list(&PATTERNS)).all(|e| *e == Entry::Empty));
            for e in s.auto_battle.places.iter().filter(|e| **e != Entry::Empty) {
                let Entry::Chip(c) = e else { panic!("a picked side has chips alone: {e:?}") };
                assert!(s.folder.chips().any(|f| f.id == *c));
            }
            assert_eq!(s.auto_battle.records, [Record::ZERO; 8]);
            assert_eq!(s.auto_battle, crate::AutoBattle::of_folder(&content, &s.folder));
        }
        assert_ne!(m.sides[0].auto_battle, m.sides[1].auto_battle);
        let text = crate::write(&content, &m);
        assert!(text.contains("[left.auto_battle]\nfirst = [{}, {}, {}]\nstandard = [\n") && text.contains("[right.auto_battle]"), "{text}");
        assert_eq!(crate::parse(&content, &text).unwrap(), m);
        // EXE6 has no auto battle: a random match of it states none.
        let six = crate::testing::exe6_content();
        assert!(live(&six, "exe6", 4, None).unwrap().sides.iter().all(|s| s.auto_battle.is_blank()));
    }
}
