//! Match setups: everything a round needs, chosen before the battle (the
//! arena, and each side's ruleset, navi, game, stats, folder, Crosses and
//! patch cards), as a human-readable TOML file of content keys
//! (docs/frontend.md §6), checked against the content (`check`), and the
//! round it plays ([`Match::round`]). Live play's random draw of one is
//! here too (`draw`), so a drawn setup can be written out and edited.
//!
//! nettai-frontend plays a match file (`--match`), and netplay's offers are
//! a side of one: the same checks refuse a bad file and a bad offer.
//! nettai-editor edits them.

/// The game a match plays when it names no ruleset: BN6 (docs/design/rules-in-luau.md,
/// the flat namespace: the default game is a frontend's, by name).
pub const DEFAULT_GAME: &str = "bn6";

#[cfg(test)]
mod bn6_forms;
pub mod check;
pub mod draw;
pub mod file;
pub mod folders;
#[cfg(test)]
mod games;
mod import;
pub mod link_navis;
pub mod names;
pub mod sp_times;
pub mod stats;
#[cfg(any(test, feature = "testing"))]
pub mod testing;

use nettai_battle::console::ConsoleSetup;
use nettai_battle::content::Content;
pub use bn6_compat::CrossList;
pub use bn6_compat::unlocks::CROSSES;
use bn6_compat::Unlocks;
use nettai_battle::custom::{BattleFolder, GameVersion, PlayerSetup, SavedFolder};
use nettai_battle::link::Link;
use nettai_battle::navicust::NaviCust;
use nettai_battle::patch_cards::{InstalledCard, PatchCards};
use nettai_battle::setup::{BattleSettings, NaviStats, RoundSetup, SetScore, SpTimes, Stage, effects};
use nettai_battle::tactics::Tactics;
use nettai_battle::{Battle, Rng};
use nettai_content_api::{NaviHandle, RulesetHandle, StageHandle};

pub use check::{check_match, check_side};
pub use draw::Draws;
pub use file::{parse, write};
pub use folders::Folder;

/// Where a round is fought: a stage and the background shown (none: the
/// stage's own).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Place {
    pub stage: StageHandle,
    pub background: Option<String>,
}

/// The arena (docs/design/rules-in-luau.md §2.3: the stage's game decides
/// the battle's data): the first round's place and the set's later
/// rounds' (the original's init exchange carries those).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Arena {
    pub first: Place,
    pub later: [Place; 2],
}

impl Arena {
    /// Every round on one place.
    pub fn on(place: Place) -> Arena {
        Arena { later: [place.clone(), place.clone()], first: place }
    }
}

/// What a side's tactics' send draws from, with the seed and the side.
const TACTICS_SALT: u32 = 0x5441_4354;

/// What a player brings to a match: their rules, their navi and game, the
/// navi's stats (what their save and NaviCust give it), their folder (as a
/// save holds it, once whole: the round's init shuffles it), the Crosses their Cross
/// window offers, their patch cards (each switched on or not, in the order
/// they apply), and the rest of what their save says.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Side {
    /// The player's ruleset (none: the content's stock ruleset).
    pub ruleset: Option<RulesetHandle>,
    pub navi: NaviHandle,
    /// The player's game: their Beast (Beast Out and Beast Over), their
    /// console's own pictures and Beast Out roar, and the navi's game
    /// (NaviStats+0x20, which MstrCros reads).
    pub game: GameVersion,
    /// The navi's stats as the round starts them (the version is the
    /// game's).
    pub stats: NaviStats,
    /// The save's event flag 0x1720: the emotion window flickers as a
    /// bugged navi's does.
    pub emotion_window_glitch: bool,
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
    /// BN5's computer-navi data, the player's save's block (entries in place
    /// order): what a computer navi across from them plays (BN5's Dark
    /// MegaMan, nettai_battle::tactics). Empty: none to play (he fires his
    /// buster between rests). The round's setup sends them as the console
    /// does (`Tactics::sent`).
    pub tactics: Tactics,
}

impl Side {
    /// `navi`'s stats as a fresh save gives them ([`NaviStats::fresh`]),
    /// of `game`, as a battle starts them (`starting`): what a side's stats
    /// block is written over.
    pub fn base_stats(content: &Content, navi: NaviHandle, game: GameVersion) -> NaviStats {
        let s = NaviStats::fresh(navi, content).unwrap_or(NaviStats { navi, ..NaviStats::default() });
        starting(content, s, game)
    }

    /// The side's stats as the battle starts them (`starting`).
    pub fn round_stats(&self, content: &Content) -> NaviStats {
        starting(content, self.stats, self.game)
    }

    /// The side's stats block: what differs from the navi's stats as a save
    /// gives them (a link navi's at its level: `Side::save_base`), but what
    /// the battle's start sets (MegaMan's variant).
    pub fn stats_block(&self, content: &Content) -> std::collections::BTreeMap<String, toml::Value> {
        let base = Side::save_base(content, self.navi, self.game, self.navi_level);
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
/// save says: the navi's game (NaviStats+0x20) is the player's, and
/// MegaMan's variant (+0x2B, which picks his move lag) is his base HP in
/// hundreds (`sub_800A2F8`).
pub fn starting(content: &Content, mut s: NaviStats, game: GameVersion) -> NaviStats {
    s.version = version_byte(game);
    if content.navi(s.navi).forms.is_some() {
        s.navi_variant = (s.max_base_hp / 100) as u8;
    }
    s
}

impl Side {

    /// The ruleset the side plays by: its own, else the content's stock.
    pub fn ruleset_or_stock(&self, content: &Content) -> Option<RulesetHandle> {
        self.ruleset.or_else(|| content.defs.stock_ruleset_of(crate::DEFAULT_GAME))
    }

    /// Whether the side's ruleset has the system with this key (unqualified,
    /// in any root: `forms`, `patch-cards`).
    pub fn has_system(&self, content: &Content, system: &str) -> bool {
        ruleset_has_system(content, self.ruleset_or_stock(content), system)
    }
}

/// The navi's game as NaviStats+0x20 has it (0 Gregar, 1 Falzar).
pub fn version_byte(game: GameVersion) -> u8 {
    match game {
        GameVersion::Gregar => 0,
        GameVersion::Falzar => 1,
    }
}

/// Whether `ruleset` (none: none at all) lists a system whose key is
/// `system` in its root (`bn6:forms` for `forms`).
pub fn ruleset_has_system(content: &Content, ruleset: Option<RulesetHandle>, system: &str) -> bool {
    let Some(r) = ruleset else { return false };
    content.defs.ruleset(r).systems.iter().any(|&s| nettai_content_api::keys::local(&content.defs.system(s).key) == system)
}

/// The system that brings the Cross window and the form changes (BN6's).
pub const FORMS_SYSTEM: &str = "forms";
/// The system that applies patch cards (BN6's).
pub const PATCH_CARDS_SYSTEM: &str = "patch-cards";
/// The system that compiles the NaviCust (BN6's).
pub const NAVICUST_SYSTEM: &str = "navicust";

/// The NaviCust board of the side's game (its ruleset's game's rule
/// section `navicust`).
pub fn navicust_rules<'c>(content: &'c Content, s: &Side) -> &'c nettai_battle::content::NaviCustRules {
    &content.side_rules(s.ruleset, ruleset_game(content, s.ruleset)).navicust
}

/// The game of a side playing by `ruleset` (none: BN6's stock rules,
/// [`DEFAULT_GAME`]).
/// The SP navis whose deletion times a side's setup carries, by slot
/// (its ruleset's rules' `sp_slots`: BN6's `sp/heatman` ...).
pub fn sp_slots(content: &Content, ruleset: Option<RulesetHandle>) -> &[String] {
    &content.side_rules(ruleset, ruleset_game(content, ruleset)).sp_slots
}

pub fn ruleset_game(content: &Content, ruleset: Option<RulesetHandle>) -> nettai_battle::content::RootId {
    content.defs.ruleset_game(ruleset, content.defs.root_id(DEFAULT_GAME).unwrap_or_default())
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

/// The background a match names, `name`, written in full (`bn6:clouds`):
/// its handle, if the packs have it (docs/design/rules-in-luau.md, the flat
/// namespace).
pub fn background(content: &Content, name: &str) -> Option<nettai_battle::content::BackgroundId> {
    content.assets.handle(nettai_content_api::AssetKind::Background, name).map(nettai_battle::content::BackgroundId)
}

/// What is wrong with a background a match names that the packs haven't.
pub(crate) fn no_background(at: &str, name: &str) -> String {
    if nettai_content_api::keys::is_qualified(name) {
        format!("{at}: no background {name:?}")
    } else {
        format!("{at}: background {name:?} names no pack: write it in full (\"bn6:{name}\")")
    }
}

/// The background a place shows.
fn background_id(content: &Content, p: &Place) -> nettai_battle::content::BackgroundId {
    p.background.as_ref().and_then(|b| background(content, b)).unwrap_or(content.stage(p.stage).background)
}

impl Match {
    /// A new match, nothing chosen yet: the first link battle stage (with
    /// its own background), and each side on the content's stock rules with
    /// BN6's navi ([`DEFAULT_GAME`]'s: MegaMan, the navi that changes form,
    /// else the first with fresh stats; any game's when it has none) at its
    /// fresh stats, of Falzar; an
    /// empty folder, no Regular or tag chips, the game's own Crosses, no
    /// patch cards, and a NaviCust with no programs where the rules have
    /// one. No seed (the battle's is drawn when it is played). Its folders
    /// are none the checks accept until they are made.
    pub fn empty(content: &Content) -> Result<Match, String> {
        let stage = *link_battle_stages(content).first().ok_or("the content has no link battle stage")?;
        let fresh: Vec<NaviHandle> = (0..content.defs.navis.len() as u16).map(NaviHandle).filter(|&n| content.navi(n).fresh.is_some()).collect();
        let own: Vec<NaviHandle> =
            fresh.iter().copied().filter(|&n| nettai_content_api::keys::root_of(&content.defs.navi(n).key) == Some(DEFAULT_GAME)).collect();
        let navis = if own.is_empty() { fresh } else { own };
        let navi = navis
            .iter()
            .copied()
            .find(|&n| content.navi(n).forms.is_some())
            .or_else(|| navis.first().copied())
            .ok_or("the content has no navi with fresh stats")?;
        let game = GameVersion::Falzar;
        let mut side = Side {
            ruleset: content.defs.stock_ruleset_of(DEFAULT_GAME),
            navi,
            game,
            stats: Side::base_stats(content, navi, game),
            emotion_window_glitch: false,
            folder: Folder::EMPTY,
            crosses: None,
            beast_out: true,
            cards: Vec::new(),
            navi_level: default_navi_level(content, navi),
            bug_frags: 0,
            sp_times: SpTimes::default(),
            navicust: None,
            tactics: Tactics::default(),
        };
        let boards = navicust_rules(content, &side).boards.len();
        if side.has_system(content, NAVICUST_SYSTEM) && content.navi(navi).forms.is_some() && boards > 0 {
            side.navicust = NaviCust::new(&[], (boards - 1) as u8).ok();
        }
        Ok(Match { seed: None, arena: Arena::on(Place { stage, background: None }), sides: [side.clone(), side] })
    }

    /// The round this match starts with, its battle's RNG and each console's
    /// from `seed`: the arena's first place, each side's player on their
    /// side (their folder shuffled by their console's RNG), the set's later
    /// places.
    pub fn round(&self, content: &Content, seed: u32) -> RoundSetup {
        let first = &self.arena.first;
        let settings = BattleSettings {
            stage: first.stage,
            background: background_id(content, first),
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
                folder: Some(folder),
                souls: Default::default(),
                joypad_phase: 0,
                bug_frags: s.bug_frags,
                navi_level: s.navi_level,
                sp_times: s.sp_times,
                console: ConsoleSetup {
                    rng: rng.state,
                    tag_pair,
                    emotion_window_glitch: s.emotion_window_glitch,
                    ..ConsoleSetup::default()
                },
                ruleset: s.ruleset,
                rules: Vec::new(),
                patch_cards: PatchCards::new(&s.cards).unwrap_or_default(),
                navicust: s.navicust,
                // The block the console sends (0x0802C7BE), its RNG2 a
                // stream of the side's own from the seed (the original's
                // is the console's at the link's start, which nothing
                // here runs).
                tactics: s.tactics.sent(&mut Rng::new(seed ^ TACTICS_SALT ^ (side as u32).wrapping_mul(0x9E37_79B9))),
            };
            // What the save unlocks, into its BN6 systems' setup: every
            // Cross of the game (or the side's list) and Beast Out as the
            // side says.
            let unlocks = Unlocks { beast_out: s.beast_out, cross_list: s.crosses, ..Unlocks::everything(s.game) };
            unlocks.write(content, &mut player).expect("a side's ruleset takes BN6's setup as its systems declare it");
            player
        };
        RoundSetup {
            content: content.hash(),
            settings,
            navi_stats: [self.sides[0].round_stats(content), self.sides[1].round_stats(content)],
            rng: seed,
            local_side: 0,
            score: SetScore::default(),
            later_stages: self.arena.later.clone().map(|p| Stage { stage: p.stage, background: background_id(content, &p) }),
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
        p.folder = Some(folder);
        // (The save's glitch flag stays.)
        p.console = ConsoleSetup { rng: rng.state, tag_pair, frames: console.frames, emotion_window_glitch: p.console.emotion_window_glitch };
    }
    next
}

/// The stages a link battle draws from: the content's link battle stages
/// that aren't the random battle's (`sub_81209DC` draws a link battle's
/// from the settings records 0 to 0x5F; those from 0x60 on are the random
/// battle's, `effects::RANDOM`). A match is fought on one of them.
pub fn link_battle_stages(content: &Content) -> Vec<StageHandle> {
    (0..content.defs.stages.len() as u16)
        .map(StageHandle)
        .filter(|&s| {
            let e = content.stage(s).effects;
            e & effects::LINK != 0 && e & effects::RANDOM == 0
        })
        .collect()
}

/// The link battle stage with this key.
pub fn link_stage(content: &Content, key: &str) -> Result<StageHandle, String> {
    let stages = link_battle_stages(content);
    content.defs.stage_by_key(key).filter(|s| stages.contains(s)).ok_or_else(|| {
        let keys: Vec<&str> = stages.iter().map(|&s| content.defs.stage(s).key.as_str()).collect();
        format!("no link battle stage {key:?}; the content's are {}", keys.join(", "))
    })
}

/// Patch cards from a list of card keys, comma-separated, in the order
/// they apply (e.g. `bn6:canodumb,-bn6:shadow`): a key after `-` is installed but
/// switched off (docs/engine/patch-cards.md).
pub fn patch_cards(content: &Content, list: &str) -> Result<Vec<InstalledCard>, String> {
    let mut cards = Vec::new();
    for item in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (key, enabled) = match item.strip_prefix('-') {
            Some(key) => (key, false),
            None => (item, true),
        };
        let card = content.defs.patch_card_by_key(key).ok_or_else(|| {
            let keys: Vec<&str> = content.defs.patch_cards.iter().map(|c| c.key.as_str()).collect();
            format!("no patch card {key:?}; the content's are {}", keys.join(", "))
        })?;
        cards.push(InstalledCard { card, enabled });
    }
    Ok(cards)
}

/// The form-changing navi's Crosses of both games, in the games' order
/// (Gregar's, then Falzar's).
pub fn all_crosses(content: &Content) -> Result<Vec<nettai_content_api::FormHandle>, String> {
    let navi = content.form_changing_navi().ok_or("the content has no navi that changes form")?;
    navi_crosses(content, navi).ok_or_else(|| "the navi that changes form has no forms".into())
}

/// `navi`'s Crosses of both games (none: it doesn't change form).
pub fn navi_crosses(content: &Content, navi: NaviHandle) -> Option<Vec<nettai_content_api::FormHandle>> {
    let forms = content.navi(navi).forms.as_ref()?;
    Some([GameVersion::Gregar, GameVersion::Falzar].iter().flat_map(|&g| forms.of(g).crosses.iter().copied()).collect())
}

/// What a match is, for the terminal: the seed, the field, each side's
/// game and Crosses, and with `folders` the folders; `you` is the side the
/// player plays.
pub fn describe(content: &Content, m: &Match, seed: u32, folders: bool, you: usize) -> String {
    let place = |p: &Place| {
        let stage = &content.defs.stage(p.stage).key;
        match &p.background {
            Some(b) => format!("stage {stage}, background {b}"),
            None => format!("stage {stage}"),
        }
    };
    let mut out = format!("match: seed {seed}, {}", place(&m.arena.first));
    for (side, s) in m.sides.iter().enumerate() {
        let who = match (side == you, side) {
            (true, _) => "you",
            (false, 0) => "the left navi",
            (false, _) => "the right navi",
        };
        let game = match s.game {
            GameVersion::Gregar => "Gregar",
            GameVersion::Falzar => "Falzar",
        };
        let navi = names::navi(content, s.navi);
        out.push_str(&format!("\n  {navi} of {game} ({who})"));
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
    }
    out
}
