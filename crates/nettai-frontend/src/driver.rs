//! What drives the battle each tick: a golden trace's recorded inputs, or
//! live input from the keyboard.

use crate::folders::{self, Draws, FolderLimits};
use nettai_battle::battle::{mode, top};
use nettai_battle::console::ConsoleSetup;
use nettai_battle::cues::CueAction;
use nettai_battle::content::{ChipCode, Content};
use nettai_battle::custom::{
    self, BattleFolder, CrossList, FolderChip, GameVersion, Phase, PlayerSetup, SavedFolder, SlotKind, SlotState, Unlocks,
};
use nettai_battle::input::keys;
use nettai_battle::link::Link;
use nettai_battle::setup::{BattleSettings, NaviStats, RoundSetup, SetScore, Stage, effects};
use nettai_battle::{Battle, PlayerTick, Rng, TickEvents};
use bn6_compat::trace::{self, Frame, Round};
use bn6_compat::{Compat, codec};
use nettai_content_api::{FormHandle, RecordHandle, StageHandle};
use std::sync::Arc;

/// One tick's inputs.
pub struct Step {
    pub input: [PlayerTick; 2],
    pub events: TickEvents,
    /// The frame number this tick shows: the trace frame it reproduces,
    /// or the tick count in live play.
    pub frame: Option<u32>,
}

pub trait Driver {
    /// A fresh battle at the start.
    fn start(&mut self) -> Battle;
    /// The next tick's inputs (`keys`: the local player's held buttons),
    /// or None when there is nothing more to play.
    fn next(&mut self, b: &Battle, keys: u16) -> Option<Step>;
    /// Compare the battle after a step with the source, if it records
    /// what should have happened.
    fn check(&self, _b: &Battle) -> Vec<String> {
        Vec::new()
    }
    /// A short description of where playback is.
    fn position(&self) -> String;
    /// Something to show the player now, if anything (the custom screen).
    fn prompt(&self, _b: &Battle) -> Option<String> {
        None
    }
    /// The region of the console whose screen this is ("us" or "jp"): what
    /// the original would show of the assets only one region's ROMs have
    /// (`Renderer::console_region`).
    fn console_region(&self) -> &'static str {
        "us"
    }
    /// For a driver that runs the battle itself (netplay's rollback
    /// session), one wall-clock frame with the local player's buttons:
    /// put the frame to show in `shown` and say what happened. None: the
    /// driver gives each tick's inputs instead (`next`).
    fn run_frame(&mut self, _keys: u16, _shown: &mut Battle) -> Option<Result<Ran, String>> {
        None
    }
    /// A line to show all the time (netplay's connection and rollbacks).
    fn status(&self) -> Option<String> {
        None
    }
    /// The battle runs in real time with another player: no pause, no
    /// other speed, no restart.
    fn real_time(&self) -> bool {
        false
    }
}

/// What a frame of a driver that runs the battle itself did
/// (`Driver::run_frame`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ran {
    /// A new frame is in `shown` (none: the frame waited, for clock sync or
    /// the stall guard, and the last one stays).
    pub advanced: bool,
    /// A new round started (the presentation starts over).
    pub new_round: bool,
    /// The sound for this frame: cue actions (a cue played on a prediction
    /// that turned out wrong is cancelled).
    pub sound: Vec<CueAction>,
}

// ---- Trace playback ----------------------------------------------------------

/// Replays one round of a golden trace.
pub struct TracePlayer {
    round: Round,
    /// The content the trace's battle runs on (BN6's).
    content: Arc<Content>,
    /// The original's numbers for it, which the comparison reads.
    compat: &'static Compat,
    /// Indices of the frames the engine simulates.
    frames: Vec<usize>,
    pos: usize,
    pub round_number: usize,
}

impl TracePlayer {
    pub fn new(round: Round, round_number: usize, content: Arc<Content>) -> TracePlayer {
        let start = round.setup.frame;
        let frames = round
            .frames
            .iter()
            .enumerate()
            .filter(|(_, f)| f.frame >= start)
            .take_while(|(_, f)| f.state[0] == 4 || f.state[0] == 8)
            .map(|(i, _)| i)
            .collect();
        TracePlayer { round, content, compat: Compat::bn6(), frames, pos: 0, round_number }
    }

    /// Every round of a trace file, on `content`.
    pub fn load(path: &std::path::Path, content: &Arc<Content>) -> std::io::Result<Vec<TracePlayer>> {
        Ok(trace::rounds(path)?.into_iter().enumerate().map(|(i, r)| TracePlayer::new(r, i + 1, content.clone())).collect())
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// The trace frame numbers this round covers.
    pub fn frame_range(&self) -> Option<(u32, u32)> {
        let f = |i: usize| self.round.frames[self.frames[i]].frame;
        (!self.frames.is_empty()).then(|| (f(0), f(self.frames.len() - 1)))
    }

    fn current(&self) -> Option<&Frame> {
        self.pos.checked_sub(1).and_then(|p| self.frames.get(p)).map(|&i| &self.round.frames[i])
    }
}

impl Driver for TracePlayer {
    fn start(&mut self) -> Battle {
        self.pos = 0;
        self.round.start(self.content.clone(), self.compat)
    }

    fn next(&mut self, _b: &Battle, _keys: u16) -> Option<Step> {
        let &i = self.frames.get(self.pos)?;
        let f = &self.round.frames[i];
        // The frames around it: the link's events are read from the frame
        // before (the session closing) and the one after (a recorded
        // custom screen's result).
        let mut window = Vec::with_capacity(3);
        if let Some(&h) = self.pos.checked_sub(1).and_then(|p| self.frames.get(p)) {
            window.push(&self.round.frames[h]);
        }
        let at = window.len();
        window.push(f);
        if let Some(&j) = self.frames.get(self.pos + 1) {
            window.push(&self.round.frames[j]);
        }
        let ids = codec::Ids::new(&self.content, self.compat);
        let (input, events) = self.round.tick_inputs(at, &window, &ids);
        self.pos += 1;
        Some(Step { input, events, frame: Some(f.frame) })
    }

    fn check(&self, b: &Battle) -> Vec<String> {
        self.current().map(|f| trace::compare(b, f, self.compat)).unwrap_or_default()
    }

    fn console_region(&self) -> &'static str {
        match self.round.console_game() {
            bn6_compat::Game::JpFalzar | bn6_compat::Game::JpGregar => "jp",
            bn6_compat::Game::Falzar | bn6_compat::Game::Gregar => "us",
        }
    }

    fn position(&self) -> String {
        match self.current() {
            Some(f) => format!("round {} frame {}", self.round_number, f.frame),
            None => format!("round {} start", self.round_number),
        }
    }
}

// ---- Live play -------------------------------------------------------------------

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
    codec::navi_stats(&unhex(LIVE_NAVI).try_into().unwrap(), &codec::Ids::new(content, Compat::bn6()))
}

/// The effects a link battle's match type adds to its stage's
/// (`sub_812B768`, from `off_812B7AC`: the match type the recorded matches
/// were, 0x600).
const MATCH_EFFECTS: u32 = 0x600;

/// The backgrounds a link battle draws from (`sub_81209DC`'s
/// `byte_8120A20`, by name; some are there twice, so twice as likely).
const LINK_BACKGROUNDS: [&str; 21] = [
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

/// The stages a link battle draws from: the content's link battle stages
/// that aren't the random battle's (`sub_81209DC` draws a link battle's
/// from the settings records 0 to 0x5F; those from 0x60 on are the random
/// battle's, `effects::RANDOM`).
pub fn link_battle_stages(content: &Content) -> Vec<StageHandle> {
    (0..content.defs.stages.len() as u16)
        .map(StageHandle)
        .filter(|&s| {
            let e = content.stage(s).effects;
            e & effects::LINK != 0 && e & effects::RANDOM == 0
        })
        .collect()
}

/// What a player brings to a live round: their folder as a save holds it
/// (the round's init shuffles it), their game (their Beast, Beast Out and
/// Beast Over, and their console's own pictures and Beast Out roar), the
/// Crosses their Cross window offers, and their patch cards (each
/// switched on or not, in the order they apply). In netplay each player
/// brings their own (`crate::netplay`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loadout {
    pub folder: SavedFolder,
    pub game: GameVersion,
    pub crosses: CrossList,
    pub cards: Vec<(RecordHandle, bool)>,
}

impl Loadout {
    /// A player's loadout drawn from `draws` as live play draws one: a
    /// legal random folder, five Crosses of both games, a game; no patch
    /// cards.
    pub fn drawn(content: &Content, draws: &mut Draws) -> Result<Loadout, String> {
        let folder = folders::random_folder(content, FolderLimits::of(&live_navi(content)), draws);
        let crosses = random_crosses(content, draws)?;
        let game = random_game(draws);
        Ok(Loadout { folder, game, crosses, cards: Vec::new() })
    }
}

/// What live play drew for a round (`bn6_live_setup`): the frontend's
/// choice of setup, made from the seed before the battle. The battle is
/// then a function of its setup and the players' buttons.
#[derive(Clone, Debug)]
pub struct LiveChoices {
    pub seed: u32,
    pub stage: StageHandle,
    /// The background's name (none: the stage's own).
    pub background: Option<String>,
    /// Each player's folder as a save holds it (the round's init shuffles
    /// it).
    pub folders: [SavedFolder; 2],
    /// The Crosses each player's Cross window offers.
    pub crosses: [CrossList; 2],
    /// Each player's game: their Beast (Beast Out and Beast Over), and
    /// their console's own pictures and Beast Out roar.
    pub games: [GameVersion; 2],
}

impl LiveChoices {
    /// What was drawn, for the terminal: the seed, the field, the Crosses,
    /// and with `folders` the folders; `you` is the side the player plays.
    pub fn describe(&self, content: &Content, folders: bool, you: usize) -> String {
        let stage = &content.defs.stage(self.stage).key;
        let mut out = format!("live play: seed {}, stage {stage}", self.seed);
        if let Some(b) = &self.background {
            out.push_str(&format!(", background {b}"));
        }
        for side in 0..2 {
            let who = match (side == you, side) {
                (true, _) => "you",
                (false, 0) => "the left navi",
                (false, _) => "the right navi",
            };
            let names: Vec<&str> = self.crosses[side].forms().map(|f| crate::strings::own_form_name(content, f)).collect();
            let game = match self.games[side] {
                GameVersion::Gregar => "Gregar",
                GameVersion::Falzar => "Falzar",
            };
            out.push_str(&format!("\n  {game} ({who}), Crosses: {}", names.join(", ")));
            if folders {
                out.push_str(&format!("\n  folder ({who}): {}", folders::describe(content, &self.folders[side])));
            }
        }
        out
    }
}

/// A link battle's field: the round's stage and background (none: the
/// stage's own), and the set's later rounds' (the original's init exchange
/// carries those).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub stage: StageHandle,
    pub background: Option<String>,
    pub later: [(StageHandle, Option<String>); 2],
}

/// The link battle stage with this key.
pub fn link_stage(content: &Content, key: &str) -> Result<StageHandle, String> {
    let stages = link_battle_stages(content);
    content.defs.stage_by_key(key).filter(|s| stages.contains(s)).ok_or_else(|| {
        let keys: Vec<&str> = stages.iter().map(|&s| content.defs.stage(s).key.as_str()).collect();
        format!("no link battle stage {key:?}; the content's are {}", keys.join(", "))
    })
}

/// A link battle's field drawn from `draws`: its stage and background, then
/// the later rounds'; `stage` forces the round's stage.
pub fn draw_field(content: &Content, draws: &mut Draws, stage: Option<StageHandle>) -> Result<Field, String> {
    let stages = link_battle_stages(content);
    if stages.is_empty() {
        return Err("the content has no link battle stage".into());
    }
    let backgrounds: Vec<&str> = LINK_BACKGROUNDS.iter().copied().filter(|b| content.assets.backgrounds.contains_key(*b)).collect();
    let field = |draws: &mut Draws| {
        let stage = stages[draws.below(stages.len())];
        let background = (!backgrounds.is_empty()).then(|| backgrounds[draws.below(backgrounds.len())].to_string());
        (stage, background)
    };
    let (first, background) = field(draws);
    let later = [field(draws), field(draws)];
    Ok(Field { stage: stage.unwrap_or(first), background, later })
}

/// The live round on BN6's content, drawn from `seed`: a link battle's
/// stage (`stage`, a stage's key, forces one) and background, a legal
/// random folder for each player (`crate::folders`), and five of MegaMan's
/// ten Crosses, of both games, for each Cross window
/// (`Unlocks::cross_list`, docs/engine/custom-screen.md §4.1). Both
/// players are 1000-HP MegaMen (`live_navi`), each of a game drawn at
/// random, Falzar or Gregar: their Beast Out is that game's Beast.
pub fn bn6_live_setup(content: &Content, seed: u32, stage: Option<&str>) -> Result<(RoundSetup, LiveChoices), String> {
    let stage = stage.map(|key| link_stage(content, key)).transpose()?;
    let mut draws = Draws::new(seed);
    let field = draw_field(content, &mut draws, stage)?;
    let limits = FolderLimits::of(&live_navi(content));
    let folders = [folders::random_folder(content, limits, &mut draws), folders::random_folder(content, limits, &mut draws)];
    let crosses = [random_crosses(content, &mut draws)?, random_crosses(content, &mut draws)?];
    let games = [random_game(&mut draws), random_game(&mut draws)];
    let players = [0, 1].map(|side| Loadout { folder: folders[side], game: games[side], crosses: crosses[side], cards: Vec::new() });
    live_round(content, seed, &field, &players)
}

/// The live round of `players` (side 0's, then side 1's) on `field`: two
/// 1000-HP MegaMen (`live_navi`) of the players' games, each folder
/// shuffled by its console's RNG, the battle's RNG and the consoles' from
/// `seed`.
pub fn live_round(content: &Content, seed: u32, field: &Field, players: &[Loadout; 2]) -> Result<(RoundSetup, LiveChoices), String> {
    let background_id =
        |s: StageHandle, b: &Option<String>| b.as_ref().map_or(content.stage(s).background, |b| content.assets.backgrounds[b]);
    let settings = BattleSettings {
        stage: field.stage,
        background: background_id(field.stage, &field.background),
        effects: content.stage(field.stage).effects | MATCH_EFFECTS,
    };
    let folders = [players[0].folder, players[1].folder];
    let mut setup = live_setup(content, settings, folders, seed);
    for (side, player) in players.iter().enumerate() {
        let p = &mut setup.players[side];
        p.unlocks.cross_list = Some(player.crosses);
        // The player's game: the save's (their Beast Out and Beast Over,
        // `Unlocks::version`) and the navi's (NaviStats+0x20, 0 Gregar, 1
        // Falzar, which MstrCros reads).
        p.unlocks.version = player.game;
        setup.navi_stats[side].version = match player.game {
            GameVersion::Gregar => 0,
            GameVersion::Falzar => 1,
        };
        codec::install_patch_cards(content, p, &player.cards)?;
    }
    setup.later_stages = field.later.clone().map(|(s, b)| Stage { stage: s, background: background_id(s, &b) });
    let choices = LiveChoices {
        seed,
        stage: field.stage,
        background: field.background.clone(),
        folders,
        crosses: [players[0].crosses, players[1].crosses],
        games: [players[0].game, players[1].game],
    };
    Ok((setup, choices))
}

/// A set's next round after `ended`, a round of `first`'s set: the
/// settings and score the round's end hands over, the players' folders
/// shuffled again by their consoles' RNG where the round left it (the
/// original's carries on through the next init's shuffle), the battle's
/// RNG drawn from the first round's and the round's number. Both peers of
/// a netplay match build the same from their settled states.
pub fn next_round_setup(
    content: &Content,
    first: &RoundSetup,
    players: &[Loadout; 2],
    ended: &Battle,
    settings: BattleSettings,
    score: SetScore,
) -> RoundSetup {
    let mut next = first.clone();
    next.settings = settings;
    next.score = score;
    next.rng = Draws::new(first.rng ^ (score.round as u32) << 24).next() as u32;
    for (side, player) in players.iter().enumerate() {
        let console = &ended.consoles[side];
        let mut rng = console.rng;
        let (folder, tag_pair) = BattleFolder::shuffled_with_tag_pair(&player.folder, 0, &mut rng, content);
        let p = &mut next.players[side];
        p.folder = Some(folder);
        p.console = ConsoleSetup { rng: rng.state, tag_pair, frames: console.frames, ..ConsoleSetup::default() };
    }
    next
}

/// Install a player's patch cards (BN6's patch-cards system's setup) from
/// a list of card names (`patch_cards`).
pub fn install_patch_cards(content: &Content, player: &mut PlayerSetup, list: &str) -> Result<(), String> {
    codec::install_patch_cards(content, player, &patch_cards(content, list)?)
}

/// Patch cards from a list of card names, comma-separated, in the order
/// they apply (e.g. `canodumb,-shadow`): a name after `-` is installed but
/// switched off (docs/engine/patch-cards.md).
pub fn patch_cards(content: &Content, list: &str) -> Result<Vec<(RecordHandle, bool)>, String> {
    let mut cards = Vec::new();
    for item in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (name, on) = match item.strip_prefix('-') {
            Some(name) => (name, false),
            None => (item, true),
        };
        let card = content.defs.record(&format!("patch-card/{name}")).ok_or_else(|| {
            let names: Vec<&str> =
                content.defs.records.iter().filter_map(|r| r.key.strip_prefix("patch-card/")).collect();
            format!("no patch card {name:?}; the content's are {}", names.join(", "))
        })?;
        cards.push((card, on));
    }
    Ok(cards)
}

/// A game drawn at random, Gregar or Falzar.
fn random_game(draws: &mut Draws) -> GameVersion {
    if draws.below(2) == 0 { GameVersion::Gregar } else { GameVersion::Falzar }
}

/// The form-changing navi's Crosses of both games, in the games' order
/// (Gregar's, then Falzar's).
pub fn all_crosses(content: &Content) -> Result<Vec<FormHandle>, String> {
    let navi = content.form_changing_navi().ok_or("the content has no navi that changes form")?;
    let forms = content.navi(navi).forms.as_ref().ok_or("the navi that changes form has no forms")?;
    Ok([GameVersion::Gregar, GameVersion::Falzar].iter().flat_map(|&g| forms.of(g).crosses.iter().copied()).collect())
}

/// Five of the form-changing navi's Crosses of both games, drawn at
/// random, listed in the games' order (Gregar's, then Falzar's).
fn random_crosses(content: &Content, draws: &mut Draws) -> Result<CrossList, String> {
    let all = all_crosses(content)?;
    let mut picked: Vec<usize> = (0..all.len()).collect();
    draws.shuffle(&mut picked);
    picked.truncate(custom::screen::CROSSES);
    picked.sort();
    Ok(CrossList::new(&picked.iter().map(|&i| all[i]).collect::<Vec<_>>()))
}

/// A round to play live on `content` with these battle settings: two
/// MegaMen with 1000 HP (`live_navi`), each bringing their folder,
/// shuffled from the seed, with every Cross and Beast Out of Falzar.
pub fn live_setup(content: &Content, settings: BattleSettings, folders: [SavedFolder; 2], seed: u32) -> RoundSetup {
    let stats = live_navi(content);
    let player = |side: u32| {
        // Each console shuffles its folder with its own RNG (RNG1), which
        // goes on from there.
        let mut rng = Rng::new(seed ^ side.wrapping_mul(0x9E37_79B9));
        let (folder, tag_pair) = BattleFolder::shuffled_with_tag_pair(&folders[side as usize], 0, &mut rng, content);
        PlayerSetup {
            folder: Some(folder),
            unlocks: Unlocks::everything(GameVersion::Falzar),
            joypad_phase: 0,
            bug_frags: 0,
            navi_level: 0,
            console: ConsoleSetup { rng: rng.state, tag_pair, ..ConsoleSetup::default() },
            ruleset: None,
            rules: Vec::new(),
        }
    };
    RoundSetup {
        content: content.hash(),
        settings,
        navi_stats: [stats, stats],
        rng: seed,
        local_side: 0,
        score: SetScore::default(),
        later_stages: Default::default(),
        low_hp_music_latched: false,
        sp_times: Default::default(),
        players: [player(0), player(1)],
        link_delay: Link::RECORDED_DELAY,
    }
}

/// A folder of these chips (the content's, by key, with codes) repeated to
/// 30, with no Regular or tag chips.
pub fn folder_of(content: &Content, chips: &[(&str, u8)]) -> SavedFolder {
    let chip = |key: &str| content.defs.chip_by_key(key).unwrap_or_else(|| panic!("the content defines no chip {key:?}"));
    SavedFolder {
        chips: std::array::from_fn(|i| {
            let (key, code) = chips[i % chips.len()];
            FolderChip::new(chip(key), ChipCode(code))
        }),
        regular: None,
        tags: None,
    }
}


/// Plays a round from the keyboard: the local player is the left navi,
/// with their own custom screen; the right navi stands still, and its
/// custom screen picks the first chip it can and presses OK.
pub struct LivePlayer {
    pub setup: RoundSetup,
    content: Arc<Content>,
    ticks: u32,
}

impl LivePlayer {
    pub fn new(setup: RoundSetup, content: Arc<Content>) -> LivePlayer {
        LivePlayer { setup, content, ticks: 0 }
    }
}

/// The right navi's buttons: on its custom screen, A on the chip under the
/// cursor (the first one) if it can be picked, then START and A, a press
/// every other tick.
fn bot_buttons(b: &Battle, side: usize, tick: u32) -> u16 {
    let s = &b.custom.sides[side];
    let Some(screen) = s.screen.as_ref().filter(|_| b.round.mode == mode::CUSTOM && s.in_custom) else { return 0 };
    if screen.phase != Phase::Choosing || tick % 2 == 0 {
        return 0;
    }
    let here = &screen.slots[screen.cursor as usize];
    if screen.selected == 0 && matches!(here.kind, SlotKind::Chip { .. }) && here.state == SlotState::Selectable {
        keys::A
    } else if screen.cursor != custom::screen::OK_SLOT {
        keys::START
    } else {
        keys::A
    }
}

impl Driver for LivePlayer {
    fn start(&mut self) -> Battle {
        self.ticks = 0;
        Battle::new(self.setup.clone(), self.content.clone())
    }

    fn next(&mut self, b: &Battle, keys: u16) -> Option<Step> {
        self.ticks += 1;
        let local = b.setup.local_side as usize;
        let held = |side: usize| if side == local { keys & 0x3FF } else { bot_buttons(b, side, self.ticks) };
        let input = [PlayerTick { held: held(0) }, PlayerTick { held: held(1) }];
        // The end state asks the link session to close; it closes at once.
        let r = &b.round;
        let events = TickEvents { link_closed: r.top == top::END && r.mode == 0 && r.sub == 4 && r.init == 4, ..TickEvents::default() };
        Some(Step { input, events, frame: Some(self.ticks) })
    }

    fn position(&self) -> String {
        format!("live tick {}", self.ticks)
    }

}

/// A plain-text custom screen for a player: the dealt chips in the grid's
/// order with the cursor, the picks, OK and Beast Out, and the Cross
/// window when it's open.
pub fn custom_screen_text(b: &Battle, side: usize) -> Option<String> {
    let s = &b.custom.sides[side];
    let (screen, folder) = (s.screen.as_ref()?, s.folder.as_ref()?);
    if b.round.mode != mode::CUSTOM {
        return None;
    }
    let mut out = String::new();
    if !s.in_custom {
        out.push_str(if s.sent.is_some() { "CUSTOM: WAITING FOR THE OTHER PLAYER" } else { "CUSTOM: SENDING" });
        return Some(out);
    }
    let title = match screen.phase {
        Phase::Opening { .. } => "CUSTOM",
        Phase::Choosing => "CUSTOM: A PICK, B UNDO, START OK, UP CROSS, R INFO",
        Phase::Hidden { .. } => "CUSTOM (HIDDEN: ANY KEY)",
        Phase::Description { .. } => "CUSTOM: CHIP INFO (ANY KEY)",
        Phase::RunMessage { .. } => "CUSTOM: NO TIME TO RUN (A)",
        Phase::CrossWindow { .. } | Phase::CrossWindowOpening { .. } | Phase::CrossWindowClosing { .. } => {
            "CUSTOM: CROSS (UP/DOWN, A CHOOSE, B BACK)"
        }
        Phase::CrossChosen { .. } => "CUSTOM: CROSS!",
        Phase::BeastOutChosen { .. } => "CUSTOM: BEAST OUT!",
        _ => "CUSTOM",
    };
    out.push_str(title);
    let names = |slot: u8| -> String {
        let x = &screen.slots[slot as usize];
        let label = match x.kind {
            SlotKind::Ok => "OK".to_string(),
            SlotKind::BeastOut => "BEAST OUT".to_string(),
            SlotKind::Scrap { right_half: false } => "SCRAP".to_string(),
            SlotKind::Redeal { right_half: false } => "REDEAL".to_string(),
            SlotKind::Empty | SlotKind::Hidden | SlotKind::Scrap { .. } | SlotKind::Redeal { .. } => return String::new(),
            _ => screen.chip_in(slot, folder).map(|c| format!("{} {}", crate::strings::own_chip_name(&b.content, c.id), c.code.letter())).unwrap_or_default(),
        };
        let mark = match x.state {
            SlotState::Selected => "+",
            SlotState::Unavailable => "-",
            _ => " ",
        };
        let cursor = if screen.cursor == slot && matches!(screen.phase, Phase::Choosing) { ">" } else { " " };
        format!("{cursor}{mark}{label}")
    };
    for row in [[0u8, 1, 2, 3, 4, 10], [5, 6, 7, 8, 9, 11]] {
        let cells: Vec<String> = row.iter().map(|&s| names(s)).filter(|c| !c.is_empty()).collect();
        if !cells.is_empty() {
            out.push('\n');
            out.push_str(&cells.join(" "));
        }
    }
    let picks: Vec<String> = screen
        .selection()
        .iter()
        .map(|&s| match screen.chip_in(s, folder) {
            Some(c) => format!("{} {}", crate::strings::own_chip_name(&b.content, c.id), c.code.letter()),
            None => "BEAST OUT".to_string(),
        })
        .collect();
    if !picks.is_empty() {
        out.push_str(&format!("\nPICKED: {}", picks.join(", ")));
    }
    let w = &screen.crosses;
    let cross_name = |place: u8| match s.unlocks.cross_at(&*b.content, b.stats[side].navi, place) {
        Some(f) => crate::strings::own_form_name(&b.content, f).to_uppercase(),
        None => format!("CROSS {}", place + 1),
    };
    if matches!(screen.phase, Phase::CrossWindow { .. }) {
        let entries: Vec<String> = (0..w.count)
            .map(|i| {
                let (cursor, marked) = (if w.cursor == i { ">" } else { " " }, if w.marked[i as usize] { "+" } else { "" });
                format!("{cursor}{marked}{}", cross_name(w.offered[i as usize]))
            })
            .collect();
        out.push_str(&format!("\n{}", entries.join(" ")));
    }
    if let Some(c) = w.chosen {
        out.push_str(&format!("\n{} CHOSEN", cross_name(c)));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Live play through the first custom screen: the local player picks
    /// the first chip and presses OK, the right navi's screen does the
    /// same, and the fight starts with both hands.
    #[test]
    fn live_custom_screen() {
        let content = nettai_battle::content::testing::content();
        let stage = content.stage_by_key(nettai_battle::content::testing::LINK_BATTLE);
        let settings = BattleSettings::on(&content, stage);
        // GunDelS3 N, which the test content has.
        let folder = folder_of(&content, &[("gundels3", 13)]);
        let mut live = LivePlayer::new(live_setup(&content, settings, [folder, folder], 7), content.clone());
        let mut b = live.start();
        let mut shown = false;
        for tick in 0..3000u32 {
            let s = &b.custom.sides[0];
            let choosing = s.in_custom && s.screen.as_ref().is_some_and(|x| x.phase == Phase::Choosing);
            shown |= choosing && custom_screen_text(&b, 0).is_some_and(|t| t.contains("OK"));
            let picked = s.screen.as_ref().is_some_and(|x| x.selected > 0);
            let on_ok = s.screen.as_ref().is_some_and(|x| x.cursor == custom::screen::OK_SLOT);
            // A press every other tick: A on the first chip, START, A on OK.
            let held = match (choosing && tick % 2 == 1, picked, on_ok) {
                (false, _, _) => 0,
                (true, false, _) | (true, true, true) => keys::A,
                (true, true, false) => keys::START,
            };
            let step = live.next(&b, held).unwrap();
            b.tick(&step.input, step.events);
            if b.round.turn == 1 && b.round.mode == mode::FIGHTING {
                break;
            }
        }
        assert!(shown);
        assert_eq!((b.round.turn, b.round.mode), (1, mode::FIGHTING));
        for side in 0..2 {
            assert_eq!(b.hands[side].remaining(), 1, "side {side}");
        }
    }

    /// Live play's setup from a seed: a link battle's stage (the 96 the
    /// original draws from), legal folders, five Crosses of both games per
    /// window; the same seed, the same setup; `--stage` forces the stage.
    #[test]
    fn the_live_setup_is_drawn_from_the_seed() {
        let content = crate::folders::bn6_test_content();
        // The navi: no NaviCust programs (road panels carry him).
        let s = live_navi(&content);
        assert_eq!((s.hp, s.mega_level, s.giga_level, s.reg_up), (1000, 5, 1, 50));
        assert!(!s.float_shoes && !s.air_shoes && !s.undershirt && !s.super_armor && !s.chip_shuffle && !s.number_open);
        let stages = link_battle_stages(&content);
        assert_eq!(stages.len(), 96);
        let mut seen = std::collections::BTreeSet::new();
        let navi = content.form_changing_navi().unwrap();
        for seed in 0..12 {
            let (setup, choices) = bn6_live_setup(&content, seed, None).unwrap();
            assert!(stages.contains(&setup.settings.stage));
            assert_eq!(setup.settings.effects & effects::RANDOM, 0);
            seen.insert(setup.settings.stage);
            let limits = FolderLimits::of(&setup.navi_stats[0]);
            for side in 0..2 {
                assert!(folders::violations(&content, &choices.folders[side], limits).is_empty());
                let list = setup.players[side].unlocks.cross_list.unwrap();
                let games: Vec<_> = list.forms().map(|f| content.form(f).game.unwrap()).collect();
                assert_eq!(games.len(), 5);
                for f in list.forms() {
                    let forms = content.navi(navi).forms.as_ref().unwrap();
                    assert!(forms.gregar.crosses.contains(&f) || forms.falzar.crosses.contains(&f));
                }
            }
            assert_eq!(format!("{:?}", bn6_live_setup(&content, seed, None).unwrap().0), format!("{setup:?}"));
        }
        assert!(seen.len() > 6, "{seen:?}");
        // Each player's game is drawn too: both games come up, and the
        // navi's game is the save's.
        let mut games = std::collections::BTreeSet::new();
        for seed in 0..12 {
            let (setup, choices) = bn6_live_setup(&content, seed, None).unwrap();
            for side in 0..2 {
                assert_eq!(setup.players[side].unlocks.version, choices.games[side]);
                assert_eq!(setup.navi_stats[side].version, (choices.games[side] == GameVersion::Falzar) as u8);
                games.insert(format!("{:?}", choices.games[side]));
            }
        }
        assert_eq!(games.len(), 2);
        // Some seed offers both games' Crosses.
        let mixed = (0..12).any(|seed| {
            let list = bn6_live_setup(&content, seed, None).unwrap().0.players[0].unlocks.cross_list.unwrap();
            let gregar = list.forms().filter(|&f| content.form(f).game == Some(GameVersion::Gregar)).count();
            gregar > 0 && gregar < 5
        });
        assert!(mixed);
        let (forced, _) = bn6_live_setup(&content, 3, Some("netbattle-43")).unwrap();
        assert_eq!(content.defs.stage(forced.settings.stage).key, "netbattle-43");
        assert_eq!(forced.players, bn6_live_setup(&content, 3, None).unwrap().0.players);
        assert!(bn6_live_setup(&content, 3, Some("netbattle-100")).is_err());
    }

    /// Run `live` until `done`, with the local player's buttons from
    /// `keys` (tick, battle); panics past `limit` ticks.
    fn play_until(
        live: &mut LivePlayer,
        b: &mut Battle,
        limit: u32,
        mut keys: impl FnMut(u32, &Battle) -> u16,
        mut done: impl FnMut(&Battle) -> bool,
    ) {
        for tick in 0..limit {
            if done(b) {
                return;
            }
            let held = keys(tick, b);
            let step = live.next(b, held).unwrap();
            b.tick(&step.input, step.events);
        }
        panic!("not done in {limit} ticks: mode {:#x}, turn {}", b.round.mode, b.round.turn);
    }

    /// The local player's screen, while it takes keys.
    fn choosing(b: &Battle) -> Option<&custom::Screen> {
        let s = &b.custom.sides[0];
        s.screen.as_ref().filter(|x| s.in_custom && b.round.mode == mode::CUSTOM && x.phase == Phase::Choosing)
    }

    /// nettai's Cross list on BN6's content: a Falzar player offered
    /// HeatCross, Gregar's, chooses it on the custom screen and fights in
    /// it (its form, element, buster and charged shot: HeatCross's flame);
    /// on the next screen Beast Out from it is HeatCross's Beast form, a
    /// Gregar Beast, with its weapons.
    #[test]
    fn a_falzar_player_plays_a_gregar_cross() {
        use nettai_battle::battle::battle_flags;
        use nettai_battle::content::Element;
        use nettai_battle::kinds::player::{NaviAction, navi_action};
        let content = crate::folders::bn6_test_content();
        let heat = content.defs.form_by_key("heatcross").unwrap();
        let heat_beast = content.defs.form_by_key("heatcross-beast").unwrap();
        let stage = link_battle_stages(&content)[0];
        let settings = BattleSettings { stage, background: 0, effects: content.stage(stage).effects | MATCH_EFFECTS };
        let folder = folder_of(&content, &[("cannon", 0)]);
        let mut setup = live_setup(&content, settings, [folder, folder], 5);
        setup.players[0].unlocks.cross_list = Some(CrossList::new(&[heat]));
        let mut live = LivePlayer::new(setup, content.clone());
        let mut b = live.start();
        // The first screen: UP opens the Cross window (a hold acts on its
        // second tick), A chooses HeatCross, START and A press OK.
        play_until(
            &mut live,
            &mut b,
            3000,
            |tick, b| {
                let s = &b.custom.sides[0];
                let Some(screen) = s.screen.as_ref().filter(|_| s.in_custom && b.round.mode == mode::CUSTOM) else { return 0 };
                let w = &screen.crosses;
                match screen.phase {
                    Phase::Choosing if w.chosen.is_none() => [keys::UP, keys::UP, 0][tick as usize % 3],
                    Phase::CrossWindow { entered: true } if w.chosen.is_none() => {
                        assert_eq!((w.count, custom_screen_text(b, 0).unwrap().contains(">HEATCROSS")), (1, true));
                        if tick % 2 == 1 { keys::A } else { 0 }
                    }
                    Phase::Choosing if tick % 2 == 1 => {
                        if screen.cursor == custom::screen::OK_SLOT { keys::A } else { keys::START }
                    }
                    _ => 0,
                }
            },
            |b| b.custom.sides[0].sent.is_some(),
        );
        assert_eq!(b.custom.sides[0].sent.as_ref().unwrap().result.transform.form, Some(heat));
        // The fight resumes and the navi changes into HeatCross.
        let p0 = b.player(0).unwrap();
        play_until(&mut live, &mut b, 1000, |_, _| 0, |b| b.stats[0].form == heat && navi_action(b, p0) == NaviAction::Idle);
        let actor = b.objects.get(p0).actor.unwrap();
        let weapons = content.form(heat).weapons;
        assert_eq!((b.actors.get(actor).buster, b.actors.get(actor).charge_shot), (weapons.buster, weapons.charge_shot));
        assert_eq!(b.objects.get(p0).element & 0xF, Element::Fire as u8);
        // B held charges the buster; let go, HeatCross's flame.
        let flame = content.defs.action_by_key("heatcross/charge/action").unwrap();
        let mut charged = false;
        play_until(
            &mut live,
            &mut b,
            600,
            |tick, _| if tick < 200 { keys::B } else { 0 },
            |b| {
                charged |= navi_action(b, p0) == NaviAction::Content(flame);
                charged
            },
        );
        // The next screen, opened with L once the gauge is full: Beast Out
        // (START, DOWN, A) from HeatCross is HeatCross's Beast form, of
        // Gregar's Beast: the Beast Out button and pictures are Gregar's.
        assert_eq!(b.custom.sides[0].unlocks.beast_game(&*content, heat), GameVersion::Gregar);
        let mut beast = false;
        play_until(
            &mut live,
            &mut b,
            6000,
            |tick, b| {
                if b.round.mode == mode::FIGHTING {
                    return if b.round.flags & battle_flags::GAUGE_FULL != 0 && tick % 2 == 1 { keys::L } else { 0 };
                }
                let Some(screen) = choosing(b) else { return 0 };
                if b.round.turn < 2 {
                    return 0;
                }
                beast |= screen.beast_out;
                // Each key held two ticks (a direction acts on a hold's
                // second), then let go.
                let key = match (beast, screen.cursor) {
                    (false, custom::screen::OK_SLOT) => keys::DOWN,
                    (false, custom::screen::SPECIAL_SLOT) => keys::A,
                    (false, _) => keys::START,
                    (true, custom::screen::SPECIAL_SLOT) => keys::UP,
                    (true, _) => keys::A,
                };
                [key, key, 0][tick as usize % 3]
            },
            |b| b.round.turn >= 2 && b.custom.sides[0].sent.is_some(),
        );
        assert_eq!(b.custom.sides[0].sent.as_ref().unwrap().result.transform.form, Some(heat_beast));
        play_until(&mut live, &mut b, 1000, |_, _| 0, |b| b.stats[0].form == heat_beast && navi_action(b, p0) == NaviAction::Idle);
        let weapons = content.form(heat_beast).weapons;
        assert_eq!((b.actors.get(actor).buster, b.actors.get(actor).charge_shot), (weapons.buster, weapons.charge_shot));
        assert_eq!(content.form(heat_beast).game, Some(GameVersion::Gregar));
    }
}
