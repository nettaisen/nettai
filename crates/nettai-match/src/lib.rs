//! Match setups: everything a round needs, chosen before the battle (the
//! game, the arena, and each side's navi, version, stats,
//! folder, Crosses and patch cards), as a human-readable TOML file of
//! names (docs/frontend.md §6), checked against the content (`check`), and
//! the round it plays ([`Match::round`]). Live play's random draw of one
//! is here too (`draw`), so a drawn setup can be written out and edited.
//!
//! nettai-frontend plays a match file (`--match`), and netplay's offers are
//! a side of one: the same checks refuse a bad file and a bad offer.
//! nettai-editor edits them.
//!
//! A match is of one game, which its arena chooses (`Arena::game`): a
//! game is its rules (it has one ruleset), and both sides play by them
//! with the game's navis, chips, souls and patch cards, every one named in
//! the game's namespace alone (`ids`). There is no mixing of games.

#[cfg(test)]
mod exe6_forms;
pub mod check;
pub mod computer_navi;
pub mod draw;
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
pub mod sp_times;
pub mod stats;
#[cfg(any(test, feature = "testing"))]
pub mod testing;

use nettai_battle::console::ConsoleSetup;
use nettai_battle::content::Content;
pub use exe6_compat::CrossList;
pub use exe6_compat::unlocks::CROSSES;
use exe6_compat::Unlocks;
use nettai_battle::custom::{BattleFolder, GameVersion, PlayerSetup, SavedFolder};
use nettai_battle::link::Link;
use nettai_battle::navicust::NaviCust;
use nettai_battle::patch_cards::{InstalledCard, PatchCards};
use nettai_battle::setup::{BattleSettings, NaviStats, RoundSetup, SetScore, SpTimes, Stage, effects};
use nettai_battle::{Battle, Rng};
use nettai_content_api::{NaviHandle, StageHandle, SystemHandle};

pub use check::{check_match, check_side};
pub use computer_navi::ComputerNavi;
pub use draw::Draws;
pub use file::{parse, write};
pub use import::save_game;
pub use folders::Folder;

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

/// What the send of a side's computer-navi data draws from, with the seed
/// and the side.
const TACTICS_SALT: u32 = 0x5441_4354;

/// What a player brings to a match, all of it the match's game's: their
/// navi and version, the navi's stats (what their save and NaviCust give
/// it), their folder (as a save holds it, once whole: the round's init
/// shuffles it), the Crosses their Cross window offers, their patch cards
/// (each switched on or not, in the order they apply), and the rest of
/// what their save says.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Side {
    pub navi: NaviHandle,
    /// The player's version of the game, where the game's rules take one
    /// (`Side::takes_version`: EXE6's Gregar or Falzar): their Beast (Beast
    /// Out and Beast Over), their console's own pictures and Beast Out roar,
    /// and the navi's version (NaviStats+0x20, which MstrCros reads). None
    /// is no default of either: a side of such a game states its own (the
    /// checks refuse a match until it does), and a side of a game whose
    /// versions play alike (EXE5) has none.
    pub version: Option<GameVersion>,
    /// The navi's stats as the round starts them (the version is the
    /// side's).
    pub stats: NaviStats,
    /// The folder, its entries empty while it is being made (a round is
    /// played with a whole one: the checks refuse a match without).
    pub folder: Folder,
    /// The Crosses the Cross window offers (none: the game's own five), and
    /// Beast Out unlocked (the save's event flag 0xE0).
    pub crosses: Option<CrossList>,
    pub beast_out: bool,
    pub cards: Vec<InstalledCard>,
    /// The level of the navi code the save received (0 to 14: a link
    /// navi's chip bonus and stats, MegaMan's gains over his NaviCust);
    /// none, MegaMan without a code (a link navi always has one:
    /// [`default_navi_level`]). The save's bug frags (a dark chip or a chip
    /// weapon spends them).
    pub navi_level: Option<u8>,
    pub bug_frags: u32,
    /// How fast the save deleted each SP navi, in frames (the SP navi
    /// chips' damage; 0 the fastest).
    pub sp_times: SpTimes,
    /// The NaviCust, which the side's rules compile into the stats as the
    /// round is set up (none: the stats are what the NaviCust gives, set
    /// directly). With one, the stats are the navi's fresh stats with what
    /// the save keeps (`stats::SAVE_FIELDS`).
    pub navicust: Option<NaviCust>,
    /// EXE5's computer-navi data, the player's save's block whole
    /// (`computer_navi`): what a computer navi plays from it, the Dark
    /// MegaMan their failed Chaos Unison brings and their own navi under
    /// DarkInvs. The default: a block nothing has written (a save that
    /// never finished a battle; the computer navi only fires its buster
    /// between rests), which a game without computer navis has. The round's
    /// setup sends it as the console does (`Tactics::sent`).
    pub computer_navi: ComputerNavi,
    /// EXE5's karma, the save's light/dark value (0 to 1000; a fresh
    /// save's 500), and the souls the side has (EXE5's Soul Unison: none
    /// listed, every soul the content has): facts its systems take by name
    /// (`facts`).
    pub karma: u16,
    pub souls: Option<Vec<nettai_content_api::FormHandle>>,
    /// EXE5's Soul Unison and Chaos Unison (the save's event flags 0 and
    /// 0x236): the custom screen's soul button, and a dark chip's Chaos
    /// Unison. A finished save has both (the default); facts its souls
    /// system takes by name (`facts`).
    pub soul_unison: bool,
    pub chaos_unison: bool,
}

impl Side {
    /// `navi`'s stats as a fresh save gives them ([`NaviStats::fresh`]),
    /// of `version` (none: a side without one), as a battle starts them
    /// (`starting`): what a side's stats block is written over.
    pub fn base_stats(content: &Content, navi: NaviHandle, version: Option<GameVersion>) -> NaviStats {
        let s = NaviStats::fresh(navi, content).unwrap_or(NaviStats { navi, ..NaviStats::default() });
        starting(content, s, version)
    }

    /// The side's stats as the battle starts them (`starting`).
    pub fn round_stats(&self, content: &Content) -> NaviStats {
        starting(content, self.stats, self.version)
    }

    /// The side's stats block: what differs from the navi's stats as a save
    /// gives them (a link navi's at its level: `Side::save_base`), but what
    /// the battle's start sets (MegaMan's variant).
    pub fn stats_block(&self, content: &Content) -> std::collections::BTreeMap<String, toml::Value> {
        let base = Side::save_base(content, self.navi, self.version, self.navi_level);
        let mut block = stats::diff(content, &base, &self.round_stats(content));
        if content.navi(self.navi).forms.is_some() {
            block.remove("navi_variant");
        }
        block
    }
}

/// A side's navi code level when it says none: a link navi's 0 (a link
/// navi exists only through its navi code), MegaMan's none (no code
/// received).
pub fn default_navi_level(content: &Content, navi: NaviHandle) -> Option<u8> {
    (!content.navi(navi).changes_form()).then_some(0)
}

/// What the console sets in the stats as a battle starts, whatever the
/// save says: the navi's version (NaviStats+0x20) is the player's, and
/// MegaMan's variant (+0x2B, which picks his move lag) is his base HP in
/// hundreds (`sub_800A2F8`).
pub fn starting(content: &Content, mut s: NaviStats, version: Option<GameVersion>) -> NaviStats {
    s.version = version_byte(version);
    if content.navi(s.navi).forms.is_some() {
        s.navi_variant = (s.max_base_hp / 100) as u8;
    }
    s
}

/// The navi's version as NaviStats+0x20 has it (EXE6's: 0 Gregar, 1
/// Falzar); a side without a version has 0 there (EXE5's games write
/// nothing a battle reads to it: the recordings' are 0).
pub fn version_byte(version: Option<GameVersion>) -> u8 {
    match version {
        None | Some(GameVersion::Gregar) => 0,
        Some(GameVersion::Falzar) => 1,
    }
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
/// right), and the seed its setup and battle are drawn from, if it names
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
    /// with fresh stats) at its fresh stats, of no version (where the game's
    /// rules take one, each side's is to be chosen); an empty folder, no
    /// Regular or tag chips, the game's own Crosses, no patch cards, a
    /// NaviCust with no programs where the rules have one, and where they
    /// have computer navis the computer-navi data the game's battle end
    /// writes of a player it has learned nothing of
    /// (`ComputerNavi::nothing_learned`). No seed (the
    /// battle's is drawn when it is played). Its folders are none the
    /// checks accept until they are made, nor is a side without the version
    /// its game takes.
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
        let navis: Vec<NaviHandle> = navis(content, game);
        let navi = navis
            .iter()
            .copied()
            .find(|&n| content.navi(n).forms.is_some())
            .or_else(|| navis.first().copied())
            .ok_or_else(|| format!("{game} has no navi with fresh stats"))?;
        // (No version: a side of a game that takes one is given its own,
        // or the checks say it has none.)
        let mut side = Side {
            navi,
            version: None,
            stats: Side::base_stats(content, navi, None),
            folder: Folder::EMPTY,
            crosses: None,
            beast_out: true,
            cards: Vec::new(),
            navi_level: default_navi_level(content, navi),
            bug_frags: 0,
            sp_times: SpTimes::default(),
            navicust: None,
            // (What the game's battle end writes of a player it has
            // learned nothing of, where the game has computer navis.)
            computer_navi: if computer_navi::has(content) { ComputerNavi::nothing_learned() } else { ComputerNavi::default() },
            karma: facts::DEFAULT_KARMA,
            souls: None,
            soul_unison: true,
            chaos_unison: true,
        };
        let boards = navicust_rules(content).boards.len();
        if ruleset_has_system(content, NAVICUST_SYSTEM) && content.navi(navi).forms.is_some() && boards > 0 {
            side.navicust = NaviCust::new(&[], (boards - 1) as u8).ok();
        }
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
            let mut player = PlayerSetup {
                folder,
                joypad_phase: 0,
                navi_level: s.navi_level,
                sp_times: s.sp_times,
                console: ConsoleSetup { rng: rng.state, tag_pair, ..ConsoleSetup::default() },
                rules: Vec::new(),
                patch_cards: PatchCards::new(&s.cards).unwrap_or_default(),
                navicust: s.navicust,
                // The block the console sends (0x0802C7BE), its RNG2 a
                // stream of the side's own from the seed (the original's
                // is the console's at the link's start, which nothing
                // here runs).
                tactics: s.computer_navi.tactics().sent(&mut Rng::new(seed ^ TACTICS_SALT ^ (side as u32).wrapping_mul(0x9E37_79B9))),
            };
            // What the save unlocks, into its EXE6 systems' setup: its
            // version, every Cross of it (or the side's list) and Beast Out
            // as the side says. A side without a version brings none of it
            // (a game whose rules take none; one whose rules do is refused
            // by the match's checks until its sides state theirs).
            if let Some(version) = s.version {
                let unlocks = Unlocks { beast_out: s.beast_out, cross_list: s.crosses, ..Unlocks::everything(version) };
                unlocks.write(content, &mut player).expect("the game's rules take EXE6's setup as their systems declare it");
            }
            // Its karma and souls, into the systems that take them.
            facts::write(content, &self.arena, s, &mut player).expect("a side's karma and souls fit its rules (the match's checks)");
            player
        };
        RoundSetup {
            content: content.hash(),
            settings,
            // (The game's rules.)
            navi_stats: [self.sides[0].round_stats(content), self.sides[1].round_stats(content)],
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

/// A set's next round after `ended`, a round of `first`'s set: the
/// settings and score the round's end hands over, the players' folders
/// shuffled again by their consoles' RNG where the round left it (the
/// original's carries on through the next init's shuffle), the battle's
/// RNG drawn from the first round's and the round's number. Both peers of
/// a netplay match build the same from their settled states.
pub fn next_round(content: &Content, first: &RoundSetup, folders: &[SavedFolder; 2], ended: &Battle, settings: BattleSettings, score: SetScore) -> RoundSetup {
    let mut next = first.clone();
    next.settings = settings;
    next.score = score;
    next.rng = Draws::new(first.rng ^ (score.round as u32) << 24).next() as u32;
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

/// The stages a link battle of `game` draws from: the game's link battle
/// stages that aren't the random battle's (`sub_81209DC` draws a link
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

/// The Crosses of `game`'s navi that changes form, of both versions, in
/// the versions' order (Gregar's, then Falzar's).
pub fn all_crosses(content: &Content, game: &str) -> Result<Vec<nettai_content_api::FormHandle>, String> {
    let navi = navis(content, game).into_iter().find(|&n| navi_crosses(content, n).is_some_and(|c| !c.is_empty()));
    let navi = navi.ok_or_else(|| format!("{game} has no navi with Crosses"))?;
    navi_crosses(content, navi).ok_or_else(|| "the navi that changes form has no forms".into())
}

/// `navi`'s Crosses of both games (none: it doesn't change form).
pub fn navi_crosses(content: &Content, navi: NaviHandle) -> Option<Vec<nettai_content_api::FormHandle>> {
    content.navi(navi).forms.as_ref()?;
    let crosses = |g| exe6_compat::forms::set(content, navi, g).map(|s| s.crosses).unwrap_or_default();
    Some([GameVersion::Gregar, GameVersion::Falzar].into_iter().flat_map(crosses).collect())
}

/// What a match is, for the terminal: its game and rules, the seed, the
/// field, each side's navi, version (where the rules take one) and
/// Crosses, with `folders` the folders, and where the game has computer
/// navis what one plays from the side's save; `you` is the side the player
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
        let version = match s.version {
            Some(GameVersion::Gregar) => " of Gregar",
            Some(GameVersion::Falzar) => " of Falzar",
            None => "",
        };
        let navi = names::navi(content, s.navi);
        out.push_str(&format!("\n  {navi}{version} ({who})"));
        if let Some(list) = &s.crosses {
            let crosses: Vec<&str> = list.forms().map(|f| names::form(content, f)).collect();
            out.push_str(&format!(", Crosses: {}", crosses.join(", ")));
        }
        if !s.cards.is_empty() {
            let cards: Vec<String> =
                s.cards.iter().map(|c| format!("{}{}", if c.enabled { "" } else { "-" }, names::patch_card(content, c.card))).collect();
            out.push_str(&format!(", patch cards: {}", cards.join(", ")));
        }
        if folders {
            out.push_str(&format!("\n  folder ({who}): {}", folders::describe(content, &s.folder)));
        }
        // What a computer navi plays from the side's save, where the game
        // has computer navis.
        if computer_navi::has(content) {
            out.push_str(&format!("\n  a computer navi's plays ({who}): {}", s.computer_navi.describe(content)));
        }
    }
    out
}
