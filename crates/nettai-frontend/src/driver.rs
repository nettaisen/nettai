//! What drives the battle each tick: a golden trace's recorded inputs, or
//! live input from the keyboard.

use crate::folders::{self, Draws, FolderLimits};
use nettai_battle::battle::{mode, top};
use nettai_battle::console::ConsoleSetup;
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
use nettai_content_api::{FormHandle, StageHandle};
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

    fn position(&self) -> String {
        match self.current() {
            Some(f) => format!("round {} frame {}", self.round_number, f.frame),
            None => format!("round {} start", self.round_number),
        }
    }
}

// ---- Live play -------------------------------------------------------------------

/// A MegaMan with 1000 HP and no NaviCust programs of note (Mega level 5,
/// Giga level 1, Regular memory 50).
const LIVE_NAVI: &str = "08000000000100ff00320505010080000000ff00000000000000000101000001010301000000001f0000000a0000ffffff0000000000000000ff00000000e803e803e8030000010000000a0000000000000000000000ffffffffffff0000000000000000";

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
}

impl LiveChoices {
    /// What was drawn, for the terminal: the seed, the field, the Crosses,
    /// and with `folders` the folders.
    pub fn describe(&self, content: &Content, folders: bool) -> String {
        let stage = &content.defs.stage(self.stage).key;
        let mut out = format!("live play: seed {}, stage {stage}", self.seed);
        if let Some(b) = &self.background {
            out.push_str(&format!(", background {b}"));
        }
        for side in 0..2 {
            let who = if side == 0 { "you" } else { "the right navi" };
            let names: Vec<&str> = self.crosses[side].forms().map(|f| content.form(f).name.as_str()).collect();
            out.push_str(&format!("\n  Crosses ({who}): {}", names.join(", ")));
            if folders {
                out.push_str(&format!("\n  folder ({who}): {}", folders::describe(content, &self.folders[side])));
            }
        }
        out
    }
}

/// The live round on BN6's content, drawn from `seed`: a link battle's
/// stage (`stage`, a stage's key, forces one) and background, a legal
/// random folder for each player (`crate::folders`), and five of MegaMan's
/// ten Crosses, of both games, for each Cross window
/// (`Unlocks::cross_list`, docs/engine/custom-screen.md §4.1). Both
/// players are 1000-HP MegaMen of Falzar.
pub fn bn6_live_setup(content: &Content, seed: u32, stage: Option<&str>) -> Result<(RoundSetup, LiveChoices), String> {
    let mut draws = Draws::new(seed);
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
    // The round's field, then the set's later rounds' (the original's
    // init exchange carries those).
    let (mut first, background) = field(&mut draws);
    let later = [field(&mut draws), field(&mut draws)];
    if let Some(key) = stage {
        first = content.defs.stage_by_key(key).filter(|s| stages.contains(s)).ok_or_else(|| {
            let keys: Vec<&str> = stages.iter().map(|&s| content.defs.stage(s).key.as_str()).collect();
            format!("no link battle stage {key:?}; the content's are {}", keys.join(", "))
        })?;
    }
    let background_id =
        |s: StageHandle, b: &Option<String>| b.as_ref().map_or(content.stage(s).background, |b| content.assets.backgrounds[b]);
    let settings = BattleSettings {
        stage: first,
        background: background_id(first, &background),
        effects: content.stage(first).effects | MATCH_EFFECTS,
    };
    let limits = FolderLimits::of(&live_navi(content));
    let folders = [folders::random_folder(content, limits, &mut draws), folders::random_folder(content, limits, &mut draws)];
    let crosses = [random_crosses(content, &mut draws)?, random_crosses(content, &mut draws)?];
    let mut setup = live_setup(content, settings, folders, seed);
    for (p, list) in setup.players.iter_mut().zip(crosses) {
        p.unlocks.cross_list = Some(list);
    }
    setup.later_stages = later.map(|(s, b)| Stage { stage: s, background: background_id(s, &b) });
    Ok((setup, LiveChoices { seed, stage: first, background, folders, crosses }))
}

/// Five of the form-changing navi's Crosses of both games, drawn at
/// random, listed in the games' order (Gregar's, then Falzar's).
fn random_crosses(content: &Content, draws: &mut Draws) -> Result<CrossList, String> {
    let navi = content.form_changing_navi().ok_or("the content has no navi that changes form")?;
    let forms = content.navi(navi).forms.as_ref().ok_or("the navi that changes form has no forms")?;
    let all: Vec<FormHandle> =
        [GameVersion::Gregar, GameVersion::Falzar].iter().flat_map(|&g| forms.of(g).crosses.iter().copied()).collect();
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

    fn prompt(&self, b: &Battle) -> Option<String> {
        custom_screen_text(b, b.setup.local_side as usize)
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
            _ => screen.chip_in(slot, folder).map(|c| format!("{} {}", b.content.chip(c.id).name, c.code.letter())).unwrap_or_default(),
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
            Some(c) => format!("{} {}", b.content.chip(c.id).name, c.code.letter()),
            None => "BEAST OUT".to_string(),
        })
        .collect();
    if !picks.is_empty() {
        out.push_str(&format!("\nPICKED: {}", picks.join(", ")));
    }
    let w = &screen.crosses;
    let cross_name = |place: u8| match s.unlocks.cross_at(&*b.content, b.stats[side].navi, place) {
        Some(f) => b.content.form(f).name.to_uppercase(),
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
}
