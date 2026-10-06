//! Match setups: everything a round needs, chosen before the battle (the
//! game, the arena, and each side's navi, folder, patch cards, NaviCust and
//! what its game's rules take besides, its facts), as a human-readable TOML
//! file of names (docs/frontend.md §6), checked against the content (`check`), and
//! the round it plays ([`Match::round`]). Live play's random pick of one
//! is here too (`pick`), so a random setup can be written out and edited.
//!
//! nettai-demo plays a match file (`--match`), and netplay's offers are
//! a side of one: the same checks refuse a bad file and a bad offer.
//! nettai-demo-editor edits them.
//!
//! A match is of one game, which its arena chooses (`Arena::game`): a
//! game is its rules (it has one ruleset), and both sides play by them
//! with the game's navis, chips, forms and patch cards, every one named in
//! the game's namespace alone (`ids`). There is no mixing of games.

pub mod check;
pub mod auto_battle;
pub mod pick;
pub mod facts;
pub mod file;
pub mod folders;
#[cfg(test)]
mod games;
pub mod ids;
mod import;
mod import_exe5;
pub mod link_navis;
pub mod names;
mod set;
pub mod sp_times;
pub mod stats;
pub mod story;
#[cfg(any(test, feature = "testing"))]
pub mod testing;

use nettai_battle::console::ConsoleSetup;
use nettai_battle::content::Content;
use nettai_battle::custom::{BattleFolder, PlayerSetup};
use nettai_battle::link::Link;
use nettai_battle::navicust::NaviCust;
use nettai_battle::patch_cards::{InstalledCard, PatchCards};
use nettai_battle::setup::{BattleSettings, NaviStats, RoundSetup, SetScore, SpTimes, Stage, effects};
use nettai_battle::Rng;
use nettai_content_api::{NaviHandle, StageHandle, SystemHandle};

pub use check::{check_match, check_side};
pub use auto_battle::AutoBattle;
pub use facts::Facts;
pub use pick::Picks;
pub use file::{parse, write};
pub use import::save_game;
pub use folders::Folder;
pub use set::{After, Set};

/// Where a round is fought: a stage and the background shown, by its name
/// in the match's game's pack (none: the stage's own).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Place {
    pub stage: StageHandle,
    pub background: Option<String>,
}

/// The arena, which decides everything else: the match's game (`exe6`,
/// `exe5`: everything else a match names is that game's, and its rules are
/// the game's: a game has one ruleset), the first round's place and the
/// set's later rounds' (the original's init exchange carries those), its
/// stages the game's.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Arena {
    pub game: String,
    pub first: Place,
    pub later: [Place; 2],
}

impl Arena {
    /// Every round of a match of `game` on one place.
    pub fn on(game: &str, place: Place) -> Arena {
        Arena { game: game.to_string(), later: [place.clone(), place.clone()], first: place }
    }
}

/// That `content` is `game`'s (a content holds one game), with rules to
/// play by: or why a match of `game` can't be made on it.
pub fn playable(content: &Content, game: &str) -> Result<(), String> {
    if content.game() != game {
        return Err(format!("the content is {}'s, not {game}'s", content.game()));
    }
    if systems(content).is_empty() {
        return Err(format!("{game} has no rules"));
    }
    Ok(())
}

/// The systems of the game's rules (a game has one ruleset): the one
/// place this crate reads the engine's ruleset.
pub fn systems(content: &Content) -> &[SystemHandle] {
    content.defs.ruleset_systems()
}

/// What the send of a side's auto battle data picks from, with the seed
/// and the side.
const AUTO_BATTLE_SALT: u32 = 0x5441_4354;

/// What a player brings to a match, all of it the match's game's: their
/// navi, their folder (as a save holds it, once whole: the round's init
/// shuffles it), their patch cards (each switched on or not, in the order
/// they apply), their NaviCust, the rest of what their save says that the
/// engine keeps for every game, and what their game's rules take besides:
/// its facts (among them what the save brings to the navi's stats: EXE6's
/// base HP, Regular memory and sun). A side states no stats: a round starts
/// from the navi's fresh stats, which the game's rules build from all this
/// as the round is set up ([`Match::round`]).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Side {
    pub navi: NaviHandle,
    /// The folder, its entries empty while it is being made (a round is
    /// played with a whole one: the checks refuse a match without).
    pub folder: Folder,
    pub patch_cards: Vec<InstalledCard>,
    /// The level of the navi code the save received (0 to 14: a link
    /// navi's chip bonus and stats, MegaMan's gains over his NaviCust);
    /// none, MegaMan without a code (a link navi always has one:
    /// [`default_navi_level`]).
    pub navi_level: Option<u8>,
    /// How fast the save deleted each SP navi, in frames (the SP navi
    /// chips' damage; 0 the fastest).
    pub sp_times: SpTimes,
    /// The NaviCust, which the side's rules compile into the stats as the
    /// round is set up: its programs and its board. None: no programs, on
    /// the rules' largest board (the round compiles an empty one, for the
    /// navi that changes form where the rules have the navicust system).
    pub navicust: Option<NaviCust>,
    /// EXE5's auto battle data, the player's save's block whole
    /// (`auto_battle`): what a navi in auto battle plays from it, the Dark
    /// MegaMan their failed Chaos Unison brings and their own navi under
    /// DarkInvs. The default: a block nothing has written (a save that
    /// never finished a battle; the navi in auto battle only fires its buster
    /// between rests), which a game without auto battle has. The round's
    /// setup sends it as the console does (`AutoBattleData::sent`).
    pub auto_battle: AutoBattle,
    /// What the side brings that its game's rules take, each a field of a
    /// system's setup by its name there (`facts`: EXE6's `version`, its
    /// `crosses`; EXE5's `karma`, its `souls`): the systems' setup
    /// blocks, as the round's setup carries them. A side that says nothing
    /// has the rules' defaults.
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

/// A side's navi code level when it says none: a link navi's 0 (a link
/// navi exists only through its navi code), MegaMan's none (no code
/// received).
pub fn default_navi_level(content: &Content, navi: NaviHandle) -> Option<u8> {
    (!content.navi(navi).changes_form()).then_some(0)
}

/// Whether the game's rules have a system named `system` (`forms`,
/// `patch-cards`).
pub fn ruleset_has_system(content: &Content, system: &str) -> bool {
    systems(content).iter().any(|&s| ids::local(&content.defs.system(s).key) == system)
}

/// The system that brings the Cross window and the form changes (EXE6's).
pub const FORMS_SYSTEM: &str = "forms";
/// The system that applies patch cards (EXE6's).
pub const PATCH_CARDS_SYSTEM: &str = "patch-cards";
/// The system that compiles the NaviCust (EXE6's).
pub const NAVICUST_SYSTEM: &str = "navicust";

/// A NaviCust with no programs on the rules' largest board, for `navi`
/// where it compiles one: the navi that changes form, in a game whose rules
/// have the navicust system. None for any other.
pub fn empty_navicust(content: &Content, navi: NaviHandle) -> Option<NaviCust> {
    let boards = navicust_rules(content).boards.len();
    (ruleset_has_system(content, NAVICUST_SYSTEM) && content.navi(navi).forms.is_some() && boards > 0)
        .then(|| NaviCust::new(&[], (boards - 1) as u8).ok())
        .flatten()
}

/// The NaviCust board of the content's game (its rule section
/// `navicust`).
pub fn navicust_rules(content: &Content) -> &nettai_battle::content::NaviCustRules {
    &content.rules().navicust
}

/// The SP navis whose deletion times a side's setup carries, by slot
/// (the game's rules' `sp_slots`: EXE6's `sp/heatman` ...).
pub fn sp_slots(content: &Content) -> &[String] {
    &content.rules().sp_slots
}

/// A whole match: the arena and both sides (the left, side 0, then the
/// right), and the seed its setup and battle are picked from, if it names
/// one.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Match {
    pub seed: Option<u32>,
    pub arena: Arena,
    pub sides: [Side; 2],
}

/// The effects a link battle's match type adds to its stage's
/// (`sub_812B768`, from `off_812B7AC`: the match type the recorded matches
/// were, 0x600).
pub const MATCH_EFFECTS: u32 = 0x600;

/// The background a match of `game` names `name`: its handle, if the
/// game's pack has it.
pub fn background(content: &Content, game: &str, name: &str) -> Option<nettai_battle::content::BackgroundId> {
    ids::background(content, game, name)
}

/// What is wrong with a background a match names that its game's pack
/// hasn't.
pub(crate) fn no_background(at: &str, game: &str, name: &str) -> String {
    format!("{at}: no background {name:?} in {game}")
}

/// The background a place of a match of `game` shows.
fn background_id(content: &Content, game: &str, p: &Place) -> nettai_battle::content::BackgroundId {
    p.background.as_ref().and_then(|b| background(content, game, b)).unwrap_or(content.stage(p.stage).background)
}

impl Match {
    /// A new match of `game`, nothing chosen yet: the game's first link
    /// battle stage (with its own background), and on each
    /// side its navi (MegaMan, the navi that changes form, else the first
    /// with fresh stats) at its fresh stats; an empty folder, no
    /// Regular or tag chips, no patch cards, a
    /// NaviCust with no programs where the rules have one, where they
    /// have auto battle the auto battle data the game's battle end
    /// writes of a player it has learned nothing of
    /// (`AutoBattle::nothing_learned`), and of the facts its rules take what
    /// a side that says nothing has (their defaults; one a round can't
    /// start without, EXE6's version, is each side's to choose). No seed
    /// (the battle's is picked when it is played). Its folders are none the
    /// checks accept until they are made, nor is a side without a fact its
    /// rules require.
    pub fn empty(content: &Content, game: &str) -> Result<Match, String> {
        playable(content, game)?;
        let stage = *link_battle_stages(content, game).first().ok_or_else(|| format!("{game} has no link battle stage"))?;
        let arena = Arena::on(game, Place { stage, background: None });
        let side = Side::fresh(content, &arena)?;
        Ok(Match { seed: None, arena, sides: [side.clone(), side] })
    }

    /// The match's game.
    pub fn game(&self) -> &str {
        &self.arena.game
    }
}

impl Side {
    /// A side of a match on `arena`, nothing chosen yet (`Match::empty`).
    pub fn fresh(content: &Content, arena: &Arena) -> Result<Side, String> {
        let game = &arena.game;
        let navi = first_navi(content, game).ok_or_else(|| format!("{game} has no navi with fresh stats"))?;
        // (Its facts the rules' defaults: where they require one that has
        // none, a version, the side is given its own, or the checks say
        // it states none.)
        let facts = Facts::defaults(content);
        let mut side = Side {
            navi,
            folder: Folder::EMPTY,
            patch_cards: Vec::new(),
            navi_level: default_navi_level(content, navi),
            sp_times: SpTimes::default(),
            navicust: None,
            // (What the game's battle end writes of a player it has
            // learned nothing of, where the game has auto battle.)
            auto_battle: if auto_battle::has(content) { AutoBattle::nothing_learned() } else { AutoBattle::default() },
            facts,
        };
        side.navicust = empty_navicust(content, navi);
        Ok(side)
    }
}

impl Match {
    /// The round this match starts with, its battle's RNG and each console's
    /// from `seed`: the arena's first place, each side's player on their
    /// side (their folder shuffled by their console's RNG), the set's later
    /// places.
    pub fn round(&self, content: &Content, seed: u32) -> RoundSetup {
        let first = &self.arena.first;
        let game = &self.arena.game;
        let settings = BattleSettings {
            stage: first.stage,
            background: background_id(content, game, first),
            effects: content.stage(first.stage).effects | MATCH_EFFECTS,
        };
        let player = |side: usize| {
            let s = &self.sides[side];
            // Each console shuffles its folder with its own RNG (RNG1),
            // which goes on from there.
            let mut rng = Rng::new(seed ^ (side as u32).wrapping_mul(0x9E37_79B9));
            let saved = s.folder.saved().expect("a whole folder (the match's checks refuse one being made; `check::start` fills one in)");
            let (folder, tag_pair) = BattleFolder::shuffled_with_tag_pair(&saved, 0, &mut rng, content);
            let player = PlayerSetup {
                folder,
                joypad_phase: 0,
                navi_level: s.navi_level,
                sp_times: s.sp_times,
                console: ConsoleSetup { rng: rng.state, tag_pair, ..ConsoleSetup::default() },
                // What the side brings that its rules' systems take: their
                // setup blocks, as the side holds them.
                rules: s.facts.blocks().to_vec(),
                patch_cards: PatchCards::new(&s.patch_cards).unwrap_or_default(),
                // (The navi that changes form compiles a NaviCust where the
                // rules have one: an empty one on the largest board when the
                // side places none.)
                navicust: s.navicust.or_else(|| empty_navicust(content, s.navi)),
                // The block the console sends (0x0802C7BE), its RNG2 a
                // stream of the side's own from the seed (the original's
                // is the console's at the link's start, which nothing
                // here runs).
                auto_battle: s.auto_battle.data().sent(&mut Rng::new(seed ^ AUTO_BATTLE_SALT ^ (side as u32).wrapping_mul(0x9E37_79B9))),
            };
            player
        };
        RoundSetup {
            content: content.hash(),
            settings,
            // (Each navi's fresh stats, which its rules build on: the
            // version byte, what the save brings, a link navi's level, the
            // NaviCust, the patch cards.)
            navi_stats: self.sides.each_ref().map(|s| Side::fresh_stats(content, s.navi)),
            rng: seed,
            local_side: 0,
            score: SetScore::default(),
            later_stages: self.arena.later.clone().map(|p| Stage { stage: p.stage, background: background_id(content, game, &p) }),
            low_hp_music_latched: false,
            players: [player(0), player(1)],
            link_delay: Link::RECORDED_DELAY,
        }
    }
}

/// The stages a link battle of `game` picks from: the game's link battle
/// stages that aren't the random battle's (`sub_81209DC` picks a link
/// battle's from the settings records 0 to 0x5F; those from 0x60 on are
/// the random battle's, `effects::RANDOM`). A match is fought on one of
/// them.
pub fn link_battle_stages(content: &Content, game: &str) -> Vec<StageHandle> {
    (0..content.defs.stages.len() as u16)
        .map(StageHandle)
        .filter(|&s| {
            let e = content.stage(s).effects;
            e & effects::LINK != 0 && e & effects::RANDOM == 0 && ids::in_game(content, game, &content.defs.stage(s).key)
        })
        .collect()
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

/// Patch cards of `game` from a list of their names, comma-separated, in
/// the order they apply (e.g. `canodumb,-shadow`): a name after `-` is
/// installed but switched off (docs/engine/patch-cards.md).
pub fn patch_cards(content: &Content, game: &str, list: &str) -> Result<Vec<InstalledCard>, String> {
    let mut cards = Vec::new();
    for item in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (name, enabled) = match item.strip_prefix('-') {
            Some(name) => (name, false),
            None => (item, true),
        };
        let card = ids::patch_card(content, game, name).ok_or_else(|| {
            let names: Vec<&str> =
                content.defs.patch_cards.iter().map(|c| c.key.as_str()).filter(|k| ids::in_game(content, game, k)).map(ids::local).collect();
            format!("no patch card {name:?} in {game} ({game}'s are {})", names.join(", "))
        })?;
        cards.push(InstalledCard { card, enabled });
    }
    Ok(cards)
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

/// What a match is, for the terminal: its game, the seed, the field, each
/// side's navi and the facts it states (those that aren't its rules'
/// defaults, each as its field's name and its value: `version: falzar`),
/// with `folders` the folders, and where the game has auto battle what a
/// navi in it plays from the side's save; `you` is the side the player
/// plays.
pub fn describe(content: &Content, m: &Match, seed: u32, folders: bool, you: usize) -> String {
    let place = |p: &Place| {
        let stage = ids::local(&content.defs.stage(p.stage).key);
        match &p.background {
            Some(b) => format!("stage {stage}, background {b}"),
            None => format!("stage {stage}"),
        }
    };
    let mut out = format!("match of {}: seed {seed}, {}", m.arena.game, place(&m.arena.first));
    for (side, s) in m.sides.iter().enumerate() {
        let who = match (side == you, side) {
            (true, _) => "you",
            (false, 0) => "the left navi",
            (false, _) => "the right navi",
        };
        let navi = names::navi(content, s.navi);
        out.push_str(&format!("\n  {navi} ({who})"));
        for f in facts::fields(content) {
            if let Some(value) = s.facts.get(content, f.name).filter(|_| !s.facts.is_default(content, f.name)) {
                out.push_str(&format!("; {}: {}", f.name, facts::shown(content, &value)));
            }
        }
        if !s.patch_cards.is_empty() {
            let cards: Vec<String> =
                s.patch_cards.iter().map(|c| format!("{}{}", if c.enabled { "" } else { "-" }, names::patch_card(content, c.card))).collect();
            out.push_str(&format!("; patch cards: {}", cards.join(", ")));
        }
        if folders {
            out.push_str(&format!("\n  folder ({who}): {}", folders::describe(content, &s.folder)));
        }
        // What a navi in auto battle plays from the side's save, where the game
        // has auto battle.
        if auto_battle::has(content) {
            out.push_str(&format!("\n  auto battle plays ({who}): {}", s.auto_battle.describe(content)));
        }
    }
    out
}
