//! Match setups: everything a round needs, chosen before the battle (the
//! game, the arena, and each side's facts: what its game's rules take, its
//! navi and folder among them), as a human-readable TOML
//! file of names (docs/frontend.md §6), checked against the content (`check`), and
//! the round it plays ([`Match::round`]). Live play's random pick of one
//! is here too (`pick`), so a random setup can be written out and edited.
//!
//! nettai plays a match file (Play), nettai-tool renders one (`--match`),
//! and netplay's offers are a side of one: the same checks refuse a bad
//! file and a bad offer. nettai's build creator makes sides.
//!
//! A match is of one game (`Match::game`): a
//! game is its rules (it has one rules definition), and both sides play by them
//! with the game's navis, chips, forms and patch cards, every one named in
//! the game's namespace alone (`ids`). There is no mixing of games.

pub mod binary;
pub mod build;
pub mod check;
pub mod pick;
pub mod facts;
pub mod file;
pub mod folders;
#[cfg(test)]
mod games;
pub mod ids;
#[cfg(test)]
mod link_navis;
pub mod names;
mod set;
pub mod sp_times;
mod wrong;
pub mod story;
#[cfg(any(test, feature = "testing"))]
pub mod testing;

use nettai_battle::console::ConsoleSetup;
use nettai_battle::content::{Content, PlayerFact};
use nettai_battle::custom::{BattleFolder, PlayerSetup};
use nettai_battle::setup::{BattleSettings, HostKeys, NaviStats, RoundSetup, SetScore, Stage};
use nettai_battle::Rng;
use nettai_content_api::{NaviHandle, StageHandle};

pub use check::{check_match, check_side};
pub use facts::Facts;
pub use pick::Picks;
pub use file::{parse, write};
pub use folders::Folder;
pub use set::{After, Set};
pub use wrong::{Count, Wrong};

/// A round of a match as the match states it: where it is fought, its
/// stage (one of the game's link battle stages) and the background shown
/// (by its name in the game's pack, or one of the background pool's by its
/// game and its name in that game's pack, `exe4:acdc-area`: `ids::background`),
/// each part none where the match leaves it to be picked from its seed
/// (`Match::places`: as the match's [`BackgroundPick`] says, `pick::places`).
/// The original's battle settings of a round follow from its stage
/// (`RoundSetup::next_settings`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct RoundSettings {
    pub stage: Option<StageHandle>,
    pub background: Option<String>,
}

impl RoundSettings {
    /// A round on `place`, both parts stated (`background` by the name a
    /// match writes: `ids::background_name`).
    pub fn at(content: &Content, place: Stage) -> RoundSettings {
        RoundSettings { stage: Some(place.stage), background: ids::background_name(content, place.background).map(str::to_string) }
    }
}

/// The most round wins a set is first to: ten, the longest set fighting
/// games' players commonly play ("10先"), of 19 rounds at most, whose numbers
/// the round's banner shows in its two digits.
pub const MAX_FIRST_TO: u8 = 10;

/// The original's triple battle: first to 2 round wins, of three rounds at
/// most (best of three): a random match's (`pick::live`), and a new match's
/// (`Match::empty`). A single battle is first to 1.
pub const TRIPLE_BATTLE: u8 = 2;

/// The rounds of a set first to `wins` round wins: `2 × wins − 1`, the most
/// it can take (a single battle's one, a triple battle's three). The set is
/// decided as the original decides its triple battle (`sub_800AF50`, which
/// the engine takes with the set's rounds for its three): once one side can
/// no longer be caught, else after its last round by the wins, drawn if
/// they are level. With no round drawn, that is when a side has `wins`; a
/// drawn round is one of the rounds, as in the original, so a set with one
/// may end on fewer wins (first to 2 after a won round and two drawn: 1–0).
pub fn rounds_for(wins: u8) -> usize {
    2 * wins as usize - 1
}

/// What a set of `rounds` rounds is first to ([`rounds_for`]'s inverse):
/// none for a count no set has (none, an even number, more than
/// [`MAX_FIRST_TO`]'s).
pub fn first_to(rounds: usize) -> Option<u8> {
    (rounds % 2 == 1 && rounds <= rounds_for(MAX_FIRST_TO)).then(|| (rounds / 2 + 1) as u8)
}

/// What a round that leaves its background is drawn on, picked from the
/// match's seed (`pick::places`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BackgroundPick {
    /// As the game picks a link battle's: one of its rules' `link_pick`
    /// `backgrounds` (EXE6's `byte_8120A20`), a background there twice
    /// twice as likely; where they list none, the stage's own. The
    /// original's, and a match's unless it says otherwise.
    #[default]
    Game,
    /// One of the background pool's, every game's, each as likely
    /// (`Content::pool_backgrounds`; docs/app.md §1, "The battle's
    /// background"): nettai's own matches, Practice's and Play's.
    Pool,
}

impl BackgroundPick {
    /// The pick's name in a match file (`backgrounds = "pool"`).
    pub fn name(self) -> &'static str {
        match self {
            BackgroundPick::Game => "game",
            BackgroundPick::Pool => "pool",
        }
    }

    /// The pick a match file names.
    pub fn named(name: &str) -> Option<BackgroundPick> {
        [BackgroundPick::Game, BackgroundPick::Pool].into_iter().find(|p| p.name() == name)
    }
}

/// That `content` is `game`'s (a content holds one game), with rules to
/// play by: or why a match of `game` can't be made on it.
pub fn playable(content: &Content, game: &str) -> Result<(), String> {
    if content.game() != game {
        return Err(format!("the content is {}'s, not {game}'s", content.game()));
    }
    if content.defs.rules().is_none() {
        return Err(format!("{game} has no rules"));
    }
    // (A side's navi and folder are facts of the rules': the round's stats
    // are its navi's, and its battle folder is dealt from its folder.)
    for role in [PlayerFact::Navi, PlayerFact::Folder] {
        if content.defs.fact_field(role).is_none() {
            return Err(format!("{game}'s rules take no {} fact", role.name()));
        }
    }
    Ok(())
}

/// What a player brings to a match, all of it the match's game's: what
/// their game's rules take, its facts (their navi and their folder, which
/// the engine knows by role: [`Side::navi`], [`Side::folder`], as a save
/// holds it, once whole: the round's init shuffles it; EXE6's version and
/// Crosses, patch cards, NaviCust, what the save brings to the navi's
/// stats, EXE5's auto battle data...). A side states no stats: a round
/// starts from the navi's fresh stats, which the game's rules build from
/// all this as the round is set up ([`Match::round`]).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Side {
    /// What the side brings that its game's rules take, each a field of
    /// the rules' setup by its name there (`facts`: the navi, the folder;
    /// EXE6's `version`, its `crosses`; EXE5's `karma`, its `souls`): the
    /// rules' setup block, as the round's setup carries it. A side that
    /// says nothing has the rules' defaults (but its navi, which it
    /// states).
    pub facts: Facts,
}

impl Side {
    /// The stats a round of this side's starts from, before its rules build
    /// on them: its navi's fresh stats ([`NaviStats::fresh`]: what a new
    /// save gives the navi, by its game's `fresh_stats` rules and its own
    /// definition).
    pub fn fresh_stats(content: &Content, navi: NaviHandle) -> NaviStats {
        NaviStats::fresh(navi, content).unwrap_or(NaviStats { navi, ..NaviStats::default() })
    }
}

/// Whether a side of `navi` has a level: a navi that takes one (its
/// definition says what a level gives it: `levels`, `story`) and doesn't
/// change form (a link navi exists only through its navi code, an EXE5 team
/// navi is at its story's progress). MegaMan has none (no navi code
/// received).
pub fn level_required(content: &Content, navi: NaviHandle) -> bool {
    let n = content.navi(navi);
    !n.changes_form() && n.last_level().is_some()
}

/// The level a side of `navi` has where it states none: the highest (a
/// save at its best, as the rules' defaults are): a navi that has a level
/// ([`level_required`]) at its last (a link navi's navi code's, a team
/// navi's story's), any other at none.
pub fn play_level(content: &Content, navi: NaviHandle) -> Option<u8> {
    if !level_required(content, navi) {
        return None;
    }
    content.navi(navi).last_level()
}


/// A whole match: what is set for the match (its game: `exe6`, `exe5`;
/// everything else a match names is that game's, and its rules are the
/// game's: a game has one rules definition), each round its set may take
/// (`rounds`: a set first to `n` round wins takes `2n − 1` at most,
/// [`rounds_for`]; the original's single battle one, its triple battle
/// three), both sides (the left, side 0, then the right), the seed its
/// setup and battle are picked from, if it names one, and what a round that
/// leaves its background is drawn on (`backgrounds`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Match {
    pub game: String,
    pub seed: Option<u32>,
    pub rounds: Vec<RoundSettings>,
    pub sides: [Side; 2],
    pub backgrounds: BackgroundPick,
}

/// The effects a link battle's match type adds to its stage's
/// (`sub_812B768`, from `off_812B7AC`: the match type the recorded matches
/// were, 0x600).
pub const MATCH_EFFECTS: u32 = 0x600;

/// The keys a match's host answers in the battle's place: both. A match is
/// played in nettai, whose own menu START opens where the game's pause
/// was, and which nobody runs from (the user, 2026-10-12: "in battle pause
/// should be replaced with nettai's pause menu", "disable L to run in
/// battle"). Every match has them, so both players of one and its replay do:
/// no setting, and nothing to agree.
pub const HOST_KEYS: HostKeys = HostKeys { pause: true, run: true };

/// The background a match of `game` names `name`: its handle, if the
/// game's pack has it, or the background pool (`exe4:acdc-area`:
/// `ids::background`).
pub fn background(content: &Content, game: &str, name: &str) -> Option<nettai_battle::content::BackgroundId> {
    ids::background(content, game, name)
}

/// What is wrong with a background a match names that neither its game's
/// pack nor the background pool has.
pub(crate) fn no_background(at: &str, game: &str, name: &str) -> String {
    match name.contains(':') {
        true => format!("{at}: no background {name:?} in the background pool"),
        false => format!("{at}: no background {name:?} in {game}"),
    }
}

impl Match {
    /// A new match of `game`, nothing chosen yet: a triple battle (first to
    /// 2), its three rounds each picked from the seed, and on each
    /// side its navi (MegaMan, the navi that changes form, else the first
    /// with fresh stats) at its fresh stats; of the facts its rules take
    /// what a side that says nothing has (their defaults: an empty folder,
    /// EXE6's no patch cards and an empty NaviCust, EXE5's auto battle data
    /// the game's battle end writes of a player it has learned nothing
    /// of...; one a round can't start without, EXE6's version, is each
    /// side's to choose). No seed
    /// (the battle's is picked when it is played). Its folders are none the
    /// checks accept until they are made, nor is a side without a fact its
    /// rules require.
    pub fn empty(content: &Content, game: &str) -> Result<Match, String> {
        playable(content, game)?;
        link_battle_stages(content, game).first().ok_or_else(|| format!("{game} has no link battle stage"))?;
        let side = Side::fresh(content, game)?;
        Ok(Match {
            game: game.to_string(),
            seed: None,
            rounds: vec![RoundSettings::default(); rounds_for(TRIPLE_BATTLE)],
            sides: [side.clone(), side],
            backgrounds: BackgroundPick::Game,
        })
    }

    /// The match's game.
    pub fn game(&self) -> &str {
        &self.game
    }

    /// The round wins its set is first to (a checked match's: [`first_to`]
    /// of its rounds).
    pub fn first_to(&self) -> u8 {
        first_to(self.rounds.len()).expect("a checked match's rounds are a set's")
    }

    /// Every round's place, in order, picked from `seed` where the match
    /// leaves a part of one unstated (`pick::places`, a background as
    /// `backgrounds` says). Refused: a match that leaves a stage to a game
    /// whose rules pick none, or names a background neither its game's pack
    /// nor the background pool has (the checks refuse both).
    pub fn places(&self, content: &Content, seed: u32) -> Result<Vec<Stage>, String> {
        pick::places_from(content, &self.game, &self.rounds, self.backgrounds, &mut Picks::new(seed))
    }

    /// The match with every round's place stated: as `seed` picks those it
    /// leaves (`places`), as a replay keeps it.
    pub fn stated(&self, content: &Content, seed: u32) -> Result<Match, String> {
        let rounds = self.places(content, seed)?.into_iter().map(|p| RoundSettings::at(content, p)).collect();
        Ok(Match { rounds, ..self.clone() })
    }
}

impl Side {
    /// A side of a match of `game`, nothing chosen yet (`Match::empty`).
    pub fn fresh(content: &Content, game: &str) -> Result<Side, String> {
        let navi = first_navi(content, game).ok_or_else(|| format!("{game} has no navi with fresh stats"))?;
        // (Its facts the rules' defaults: where they require one that has
        // none, a version, the side is given its own, or the checks say
        // it states none.)
        let facts = Facts::defaults(content);
        let mut side = Side { facts };
        side.set_navi(content, navi)?;
        // (A navi that must have a level states one: 0.)
        if level_required(content, navi) {
            side.set_level(content, Some(0))?;
        }
        Ok(side)
    }
}

impl Match {
    /// The round this match starts with, its battle's RNG and each console's
    /// from `seed`: the first round's place, each side's player on their
    /// side (their folder shuffled by their console's RNG), the set's later
    /// rounds' places (every place picked from `seed` where the match
    /// leaves it: `places`). The match is a checked one.
    pub fn round(&self, content: &Content, seed: u32) -> RoundSetup {
        let mut places = self.places(content, seed).expect("a checked match's places").into_iter();
        let first = places.next().expect("a checked match has a round");
        let settings = BattleSettings { stage: first.stage, background: first.background, effects: content.stage(first.stage).effects | MATCH_EFFECTS };
        let player = |side: usize| {
            let s = &self.sides[side];
            // Each console shuffles its folder with its own RNG (RNG1),
            // which goes on from there.
            let mut rng = Rng::new(seed ^ (side as u32).wrapping_mul(0x9E37_79B9));
            let saved = s.folder(content).saved().expect("a whole folder (the match's checks refuse one being made; `check::start` fills one in)");
            let (folder, tag_pair) = BattleFolder::shuffled_with_tag_pair(&saved, 0, &mut rng, content);
            let player = PlayerSetup {
                folder,
                joypad_phase: 0,
                console: ConsoleSetup { rng: rng.state, tag_pair, ..ConsoleSetup::default() },
                // What the side brings that its rules take: their setup
                // block, as the side holds it.
                rules: s.facts.block().cloned(),
            };
            player
        };
        RoundSetup {
            content: content.hash(),
            settings,
            // (Each navi's fresh stats, which its rules build on: the
            // version byte, what the save brings, a link navi's level, the
            // NaviCust, the patch cards.)
            navi_stats: self.sides.each_ref().map(|s| Side::fresh_stats(content, s.navi(content))),
            rng: seed,
            local_side: 0,
            score: SetScore::default(),
            later_stages: places.collect(),
            low_hp_music_latched: false,
            host_keys: HOST_KEYS,
            players: [player(0), player(1)],
        }
    }
}

/// The stages a match of `game` may name, in handle order: its rules'
/// (`link_pick.match_stages`: EXE6's and EXE5's link battle stages, each a
/// settings record with the link effect that isn't the random battle's). A
/// match is fought on one of them.
pub fn link_battle_stages(content: &Content, game: &str) -> Vec<StageHandle> {
    content.rules().link_pick.match_stages.iter().copied().filter(|&s| ids::in_game(content, game, &content.defs.stage(s).key)).collect()
}

/// `game`'s link battle stage named `name`.
pub fn link_stage(content: &Content, game: &str, name: &str) -> Result<StageHandle, String> {
    let stages = link_battle_stages(content, game);
    ids::stage(content, game, name).filter(|s| stages.contains(s)).ok_or_else(|| {
        let names: Vec<&str> = stages.iter().map(|&s| ids::local(&content.defs.stage(s).key)).collect();
        format!("no link battle stage {name:?} in {game} ({game}'s are {})", names.join(", "))
    })
}

/// `game`'s navis a side can play (those with fresh stats), in handle
/// order.
pub fn navis(content: &Content, game: &str) -> Vec<NaviHandle> {
    (0..content.defs.navis.len() as u16)
        .map(NaviHandle)
        .filter(|&n| content.navi(n).fresh.is_some() && ids::in_game(content, game, &content.defs.navi(n).key))
        .collect()
}

/// The navi a new side of `game` operates: its navi that changes form
/// (MegaMan), else the first of its navis a side can play.
pub fn first_navi(content: &Content, game: &str) -> Option<NaviHandle> {
    let navis = navis(content, game);
    navis.iter().copied().find(|&n| content.navi(n).forms.is_some()).or_else(|| navis.first().copied())
}

/// The forms a form list of `game` may hold: those of its navi that
/// changes form's own lists, of every version, in the versions' order
/// (EXE6's Crosses: Gregar's, then Falzar's).
pub fn listed_forms(content: &Content, game: &str) -> Result<Vec<nettai_content_api::FormHandle>, String> {
    let navi = navis(content, game).into_iter().find(|&n| navi_forms(content, n).is_some_and(|c| !c.is_empty()));
    let navi = navi.ok_or_else(|| format!("{game} has no navi with form lists"))?;
    navi_forms(content, navi).ok_or_else(|| "the navi that changes form has no forms".into())
}

/// The forms `navi`'s form list offers, of every version (EXE6's Crosses of
/// both), in the versions' order as the rules declare them, each version's
/// in its list's (`NaviForms::by_version`); none: it doesn't change form.
pub fn navi_forms(content: &Content, navi: NaviHandle) -> Option<Vec<nettai_content_api::FormHandle>> {
    let forms = content.navi(navi).forms.as_ref()?;
    Some(facts::versions(content).iter().flat_map(|version| forms.listed(version).iter().copied()).collect())
}

/// What a match is, for the terminal: its game, the seed, the round wins
/// its set is first to, its rounds' places (as the seed picks those it
/// leaves), each
/// side's navi and the facts it states (those that aren't its rules'
/// defaults, each as its field's name and its value: `version: falzar`),
/// with `folders` the folders; `you` is the side the player plays.
pub fn describe(content: &Content, m: &Match, seed: u32, folders: bool, you: usize) -> String {
    let place = |p: &Stage| {
        let stage = ids::local(&content.defs.stage(p.stage).key);
        match ids::background_name(content, p.background) {
            Some(b) => format!("stage {stage}, background {b}"),
            None => format!("stage {stage}"),
        }
    };
    let places = match m.places(content, seed) {
        Ok(places) => places.iter().map(place).collect::<Vec<_>>().join("; "),
        Err(e) => e,
    };
    let rounds = m.rounds.len();
    let set = match first_to(rounds) {
        Some(1) => "a single battle".to_string(),
        Some(wins) => format!("first to {wins}, {rounds} rounds at most"),
        None => format!("{rounds} rounds"),
    };
    let mut out = format!("match of {}: seed {seed}, {set}: {places}", m.game);
    for (side, s) in m.sides.iter().enumerate() {
        let who = match (side == you, side) {
            (true, _) => "you",
            (false, 0) => "the left navi",
            (false, _) => "the right navi",
        };
        let navi = names::navi(content, s.navi(content));
        out.push_str(&format!("\n  {navi} ({who})"));
        // (The navi and the folder are said apart.)
        let side_parts = [PlayerFact::Navi, PlayerFact::Folder, PlayerFact::RegularChip, PlayerFact::TagChips].map(PlayerFact::name);
        for f in facts::fields(content).into_iter().filter(|f| !side_parts.contains(&f.name)) {
            if let Some(value) = s.facts.get(content, f.name).filter(|_| !s.facts.is_default(content, f.name)) {
                out.push_str(&format!("; {}: {}", f.name, facts::shown(content, &value)));
            }
        }
        if folders {
            out.push_str(&format!("\n  folder ({who}): {}", folders::describe(content, &s.folder(content))));
        }
    }
    out
}
