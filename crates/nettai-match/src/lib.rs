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

pub mod check;
pub mod draw;
pub mod file;
pub mod folders;
pub mod names;
pub mod stats;
#[cfg(any(test, feature = "testing"))]
pub mod testing;

use nettai_battle::console::ConsoleSetup;
use nettai_battle::content::Content;
use nettai_battle::custom::{BattleFolder, CrossList, GameVersion, PlayerSetup, SavedFolder, Unlocks};
use nettai_battle::link::Link;
use nettai_battle::navicust::NaviCust;
use nettai_battle::patch_cards::{InstalledCard, PatchCards};
use nettai_battle::setup::{BattleSettings, NaviStats, RoundSetup, SetScore, Stage, effects};
use nettai_battle::{Battle, Rng};
use nettai_content_api::{NaviHandle, RulesetHandle, StageHandle};

pub use check::{check_match, check_side};
pub use draw::Draws;
pub use file::{parse, write};

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

/// What a player brings to a match: their rules, their navi and game, the
/// navi's stats (what their save and NaviCust give it), their folder as a
/// save holds it (the round's init shuffles it), the Crosses their Cross
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
    pub folder: SavedFolder,
    /// The Crosses the Cross window offers (none: the game's own five).
    pub crosses: Option<CrossList>,
    pub cards: Vec<InstalledCard>,
    /// A link navi's level (its chip bonus), and the save's bug frags (a
    /// dark chip or a chip weapon spends them).
    pub navi_level: u8,
    pub bug_frags: u32,
    /// The NaviCust, which the side's rules compile into the stats as the
    /// round is set up (none: the stats are what the NaviCust gives, set
    /// directly). With one, the stats are the navi's fresh stats with what
    /// the save keeps (`stats::SAVE_FIELDS`).
    pub navicust: Option<NaviCust>,
}

impl Side {
    /// `navi`'s stats as a fresh save gives them ([`NaviStats::fresh`]),
    /// of `game`: what a side's stats block is written over.
    pub fn base_stats(content: &Content, navi: NaviHandle, game: GameVersion) -> NaviStats {
        let mut s = NaviStats::fresh(navi, content).unwrap_or(NaviStats { navi, ..NaviStats::default() });
        s.version = version_byte(game);
        s
    }

    /// The side's stats, with the version its game says.
    pub fn round_stats(&self) -> NaviStats {
        NaviStats { version: version_byte(self.game), ..self.stats }
    }

    /// The ruleset the side plays by: its own, else the content's stock.
    pub fn ruleset_or_stock(&self, content: &Content) -> Option<RulesetHandle> {
        self.ruleset.or_else(|| content.defs.stock_ruleset())
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
    &content.rules_of(content.defs.ruleset_game(s.ruleset)).navicust
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

/// The background id a place shows.
fn background_id(content: &Content, p: &Place) -> u8 {
    p.background.as_ref().and_then(|b| content.assets.backgrounds.get(b).copied()).unwrap_or(content.stage(p.stage).background)
}

impl Match {
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
            let (folder, tag_pair) = BattleFolder::shuffled_with_tag_pair(&s.folder, 0, &mut rng, content);
            let mut unlocks = Unlocks::everything(s.game);
            unlocks.cross_list = s.crosses;
            PlayerSetup {
                folder: Some(folder),
                unlocks,
                joypad_phase: 0,
                bug_frags: s.bug_frags,
                navi_level: s.navi_level,
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
            }
        };
        RoundSetup {
            content: content.hash(),
            settings,
            navi_stats: [self.sides[0].round_stats(), self.sides[1].round_stats()],
            rng: seed,
            local_side: 0,
            score: SetScore::default(),
            later_stages: self.arena.later.clone().map(|p| Stage { stage: p.stage, background: background_id(content, &p) }),
            low_hp_music_latched: false,
            sp_times: Default::default(),
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
/// they apply (e.g. `canodumb,-shadow`): a key after `-` is installed but
/// switched off (docs/engine/patch-cards.md).
pub fn patch_cards(content: &Content, list: &str) -> Result<Vec<InstalledCard>, String> {
    let mut cards = Vec::new();
    for item in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (key, enabled) = match item.strip_prefix('-') {
            Some(key) => (key, false),
            None => (item, true),
        };
        let card = content.defs.patch_card_by_key(key).ok_or_else(|| {
            let keys: Vec<&str> = content.defs.patch_cards.iter().map(|c| nettai_content_api::keys::local(&c.key)).collect();
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
