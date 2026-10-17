//! What drives the battle each tick: a driver gives the session each
//! tick's inputs ([`Driver`]), as live play does from the buttons
//! ([`LivePlayer`]), or runs the battle itself, as netplay does
//! (`crate::netplay`). A host may bring its own (the desktop program's
//! replays the original's recordings).

use nettai_battle::battle::mode;
use nettai_battle::console::ConsoleSetup;
use nettai_battle::cues::CueAction;
use nettai_battle::content::{ChipCode, Content};
use nettai_battle::custom::{self, BattleFolder, FolderChip, Phase, PlayerSetup, SavedFolder, SlotKind, SlotState};
use nettai_battle::hud::CustomGauge;
use nettai_battle::input::keys;
use nettai_battle::setup::{BattleSettings, RoundSetup, SetScore};
use nettai_battle::{Battle, BattleResult, PlayerTick, Rng, TickEvents, TickInput};
use nettai_match::{After, Set};
use nettai_netplay::standin::Masher;

use crate::connection::{Quality, Reconnecting, Traffic};
#[cfg(any(test, feature = "testing"))]
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
    /// For a driver that plays a set by giving each tick's inputs (`next`):
    /// how the set goes on after the tick that left the battle as `b` is,
    /// if that ended the round (`nettai_match::Set::after`). The session
    /// goes on with the next round's battle, or is finished with the
    /// result. None: the round goes on (all a driver of one round says).
    fn round_ended(&mut self, _b: &Battle) -> Option<After> {
        None
    }
    /// A short description of where playback is.
    fn position(&self) -> String;
    /// The game version of the console whose screen this is, as its pack
    /// names its versions' assets, for a game whose versions the engine
    /// doesn't tell apart (EXE5's "protoman" and "colonel": the other
    /// version's chips; `Renderer::console_version`). None: the
    /// version the console's player brought (the fact `version`).
    fn console_version(&self) -> Option<&'static str> {
        None
    }
    /// The trace frames this round covers (a trace's driver).
    fn frame_range(&self) -> Option<(u32, u32)> {
        None
    }
    /// For a driver that runs the battle itself (netplay's rollback
    /// session), one wall-clock frame with the local player's buttons:
    /// put the frame to show in `shown` and say what happened, or why the
    /// battle can't go on. None: the driver gives each tick's inputs instead
    /// (`next`).
    fn run_frame(&mut self, _keys: u16, _shown: &mut Battle) -> Option<Result<Ran, Stop>> {
        None
    }
    /// How the connection to the other player is doing, of a driver that
    /// has one (netplay's): figures for a host to show as it likes.
    fn net_status(&self) -> Option<NetStatus> {
        None
    }
    /// The set's result for the local player, of a driver that knows it
    /// while it still runs (netplay's: the match is over, and the players
    /// are connected until one leaves). A driver that gives each tick's
    /// inputs says it as the set ends (`round_ended`).
    fn result(&self) -> Option<BattleResult> {
        None
    }
    /// The battle runs in real time with another player: no pause, no
    /// other speed, no restart.
    fn real_time(&self) -> bool {
        false
    }
    /// What the navi the local player fights does, of a driver that plays
    /// it (live play's); none from one that doesn't.
    fn opponent(&self) -> Option<Opponent> {
        None
    }
    /// Have it do `opponent` from the next tick on; false from a driver
    /// that plays no opponent.
    fn set_opponent(&mut self, _opponent: Opponent) -> bool {
        false
    }
    /// Live play's driver, for what a training session sets on it beside
    /// the opponent (its options, its AI and its second pad:
    /// [`LivePlayer`]); none from any other.
    fn live(&self) -> Option<&LivePlayer> {
        None
    }
    fn live_mut(&mut self) -> Option<&mut LivePlayer> {
        None
    }
    /// Change the battle before the next tick's inputs are asked, as the
    /// driver's own options say (live play's training options,
    /// [`Training`]). Nothing from a driver without any, nor from one whose
    /// options are all at their normal values: the battle is then a
    /// function of its setup and the buttons, as ever.
    fn prepare(&mut self, _b: &mut Battle) {}
    /// Have the opponent it plays press nothing from the next tick on, or
    /// press again as it chose (`held` false); false from a driver that
    /// plays no opponent. Its choices and its random presses go on as
    /// they would: only what reaches the battle is held.
    fn hold_opponent(&mut self, _held: bool) -> bool {
        false
    }
    /// The player's input delay from the next frame on, in ticks, of a
    /// driver that has one (netplay's: `crate::netplay::NetOptions`); false
    /// from one that doesn't.
    fn set_input_delay(&mut self, _ticks: u32) -> bool {
        false
    }
    /// Record what it plays from now on as a replay (`crate::replay`), of
    /// a driver that plays a match's set on buttons alone and hasn't
    /// started (live play, netplay); false: it can't be (a trace's, a
    /// replay's, one under way). A driver that gives each tick's inputs
    /// (`next`) is recorded by the session from them; one that runs the
    /// battle itself hands its ticks over as they settle
    /// (`take_recorded`).
    fn record(&mut self) -> bool {
        false
    }
    /// The ticks that settled since the last call, for the recording, of a
    /// driver that runs the battle itself and records (`record`).
    fn take_recorded(&mut self, _out: &mut Vec<nettai_replay::Tick>) {}
    /// How a recording's replay shifts what the recording console's screen
    /// shows on the frame just played, against the original's, for the
    /// frame comparison: none from a driver that replays no recording.
    fn feed_shift(&self) -> FeedShift {
        FeedShift::default()
    }
    /// Go on from another place in what it plays, of a driver that plays a
    /// recording it can go about in (a replay's: `crate::viewer::Viewer`
    /// puts the battle as it stood there in the session, and this tells
    /// the driver where it is); false, and nothing changes, from one that
    /// can't.
    fn go_to(&mut self, _place: Place) -> bool {
        false
    }
}

/// A place in a set being played back (`Driver::go_to`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Place {
    /// The set's ticks played before it, through every round.
    pub tick: u32,
    /// The round it is in, from 1.
    pub round: u32,
    /// The side whose console is shown.
    pub viewer: u8,
}

/// What a recording's replay shows later than the original did
/// (`Driver::feed_shift`). The engine has no link cable: a replay of a
/// recording made over one feeds a side's custom screen OK the cable's
/// delay late (compat's `Round::fed`), so that its result arrives, and the
/// fight resumes, on the original's frame. The recording console's screen
/// shows its OK that much later, and what counts from its send after it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FeedShift {
    /// The custom screen holds back the OK the original's had taken (from
    /// that OK until the replay feeds it): it shows the choosing on.
    pub held: bool,
    /// The custom screen runs this many frames behind the original's (from
    /// the OK fed until the fight resumes).
    pub late: u32,
    /// What counts from the side's send (the full custom gauge's stripes)
    /// runs this many frames behind the original's.
    pub hud_late: u32,
}

/// Why a battle stopped, as a host can tell its player: each reason in the
/// host's own words (a catalog's string, in the player's language). The
/// detail beside it (`Player::stopped`, `Session::stopped`) is for a log.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StopReason {
    /// What was played came to its end: the set was decided, or the input
    /// ran out (a recording's round). Nothing went wrong.
    Finished,
    /// The other player left the match (before its end, or after: the
    /// result, if there is one, says which).
    OpponentLeft,
    /// The connection to the other player was lost for good: it dropped
    /// and didn't come back, or nothing came from them for too long.
    ConnectionLost,
    /// The engine stopped, or the other side's datagrams stopped making
    /// sense: nothing the player did or can mend.
    Internal,
}

/// A battle that can't go on, from a driver that runs it itself
/// (`Driver::run_frame`): the reason, for the player, and the detail, in
/// English, for a log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stop {
    pub reason: StopReason,
    pub detail: String,
}

impl Stop {
    pub fn new(reason: StopReason, detail: impl Into<String>) -> Stop {
        Stop { reason, detail: detail.into() }
    }
}

/// How a netplay match's connection and rollback are doing
/// (`Driver::net_status`, `Player::net_status`): what a host shows of them
/// is its own to compose. An indicator of four bars shows `quality` (and
/// `ping_ms` beside it), or, while `reconnecting` is some, that and its
/// countdown instead; a fuller view of the link (the app's link panel) has
/// the rest.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NetStatus {
    /// The ping: the smoothed round trip, in milliseconds; none until one
    /// is measured. (As the match sees it: a datagram's trip, and the wait
    /// for the frame that takes it on each side, a tick or so in all.)
    pub ping_ms: Option<f64>,
    /// The jitter: how unsteady the round trip is, the smoothed distance of
    /// each from the ping, in milliseconds (`crate::connection`).
    pub jitter_ms: Option<f64>,
    /// The connection's quality, from the ping and the jitter
    /// (`crate::connection::Quality::of`, which documents the levels); none
    /// until a round trip is measured.
    pub quality: Option<Quality>,
    /// The latest round trip as it was measured, not smoothed, in
    /// milliseconds: what a graph of the round trip over time plots. None
    /// until one is.
    pub rtt_ms: Option<u64>,
    /// The share of the other player's datagrams lost, 0 to 1.
    pub loss: f64,
    /// What went to the other player and what came from them.
    pub sent: Traffic,
    pub received: Traffic,
    /// The player's input delay, in ticks: how many the frame shown is
    /// behind their newest input (`crate::netplay::NetOptions`).
    pub input_delay: u32,
    /// The last rollback's depth in ticks, the deepest so far, and how many
    /// there have been.
    pub last_rollback: u32,
    pub max_rollback: u32,
    pub rollbacks: u64,
    /// Frames the battle waited on the other player (stalled or parked).
    pub waits: u64,
    /// Clock sync's skew, in ticks, as the last frame read it (getgud's
    /// `skew`): about 0 while the two players run in step, positive while
    /// this one runs ahead of the other (and stalls a frame now and then to
    /// let them catch up), negative while the other runs ahead.
    pub skew: i32,
    /// The lead, in ticks: this player's inputs the other's haven't matched
    /// yet (getgud's local queue), as the last frame read it. About the
    /// one-way trip while the two run in step; it climbs while the other
    /// player's input is late, up to the stall guard.
    pub lead: u32,
    /// How many times in this match the connection went down and was made
    /// again (one that is down now among them).
    pub reconnects: u32,
    /// The connection is down and its transport is making it again (the
    /// battle waits meanwhile): for how long so far, and how long the
    /// transport has left; none while it is up.
    pub reconnecting: Option<Reconnecting>,
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
    /// that turned out wrong is canceled).
    pub sound: Vec<CueAction>,
    /// The players who asked for the pause on a tick this frame simulated
    /// for the first time, by side (`Battle::pause_requests`). Netplay says
    /// its own player's alone: the other's buttons are a prediction until
    /// they come, and this player's menu is none of theirs.
    pub pause_requests: [bool; 2],
}

// ---- Live play -------------------------------------------------------------------
//
// What a live round is made of (its place, each side's player) is a match
// (`nettai_match`): live play's random pick of one (`nettai_match::pick`),
// or a match file (`--match`).

/// A round to play live on `content` with these battle settings: two
/// MegaMen of `version` (a version's name: EXE6's Cybeast's) at
/// their fresh stats with a NaviCust of no programs (as live play picks
/// them: their rules build the round's stats on them), each bringing their
/// folder, shuffled from the seed, and of what
/// the game's rules take besides, the version (the engine's version fact),
/// the navi's own forms of it for their form list (the engine's: EXE6's
/// version's five Crosses) and the rules' defaults for the rest (EXE6's
/// Beast Out).
pub fn live_setup(content: &Content, settings: BattleSettings, folders: [SavedFolder; 2], version: &str, seed: u32) -> RoundSetup {
    let megaman = content.form_changing_navi().expect("a navi that changes form");
    let stats = nettai_match::Side::fresh_stats(content, megaman);
    let player = |side: u32| {
        // Each console shuffles its folder with its own RNG (RNG1), which
        // goes on from there.
        let mut rng = Rng::new(seed ^ side.wrapping_mul(0x9E37_79B9));
        let (folder, tag_pair) = BattleFolder::shuffled_with_tag_pair(&folders[side as usize], 0, &mut rng, content);
        let mut player = PlayerSetup {
            folder,
            joypad_phase: 0,
            console: ConsoleSetup { rng: rng.state, tag_pair, ..ConsoleSetup::default() },
            rules: None,
        };
        use nettai_battle::content::PlayerFact;
        use nettai_battle::rules::Fact;
        // (The navi, MegaMan; a NaviCust of no programs, the rules' default.)
        let navi = Fact::Value(nettai_content_api::Value::Def(nettai_content_api::Registry::Navi, megaman.0));
        player.set_fact(content, PlayerFact::Navi.name(), &[navi]).expect("the rules take the navi");
        let version_fact = nettai_match::facts::version_fact(content, version).expect("the rules take the version");
        player.set_fact(content, content.defs.fact_name(PlayerFact::Version).expect("the rules take a version"), &[version_fact]).expect("the rules take the version");
        let own = content.navi(megaman).forms.as_ref().map_or(&[][..], |f| f.listed(version));
        let own: Vec<Fact> = own.iter().map(|f| Fact::Value(nettai_content_api::Value::Def(nettai_content_api::Registry::Form, f.0))).collect();
        if let Some(name) = content.defs.fact_name(PlayerFact::FormList) {
            player.set_fact(content, name, &own).expect("the rules take a form list");
        }
        player
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
        host_keys: nettai_match::HOST_KEYS,
        custom_screen_limit: None,
        players: [player(0), player(1)],
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

/// What the navi the local player fights in live play does: its buttons,
/// a tick at a time, which are the whole of its input as a player's are
/// (a recording keeps them); it asks the engine for nothing a player can't
/// do. Two choices, each made alone and changed at any tick
/// ([`LivePlayer::set_opponent`], [`crate::Player::set_opponent`]): what it
/// does in the fight, and what it does on its custom screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Opponent {
    pub in_fight: InFight,
    pub on_custom_screen: OnCustomScreen,
}

/// An AI that plays the opponent, which a host gives live play
/// ([`LivePlayer::set_ai`]): buttons, a tick at a time, as a player's are.
/// The library has none of its own (nettai-training's is the app's).
pub trait Ai: Send {
    /// Its buttons in the fight for the navi of `side` in `b`, this tick
    /// (`theirs`: what the local player holds).
    fn fight(&mut self, b: &Battle, side: u8, theirs: u16) -> u16;
    /// Its buttons on its custom screen, this tick.
    fn custom_screen(&mut self, b: &Battle, side: u8) -> u16;
    /// The set starts over: it is as it was given.
    fn reset(&mut self);
}

/// What an opponent does in the fight.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InFight {
    /// It presses nothing.
    #[default]
    StandsStill,
    /// It moves about the field at random (seeded) and never attacks.
    MovesAround,
    /// It presses at random (seeded) what a player fights with: it moves,
    /// shoots and charges, uses its chips, and presses L and R
    /// (`nettai_netplay::standin::Masher`). Never START.
    AttacksAtRandom,
    /// A second pad the host has drives it (`LivePlayer::set_second_pad`),
    /// in the fight and on its custom screen, as a second player, but for
    /// START outside the custom screen (the pause is the local player's).
    /// With none, it stands still, and its custom screen does as its choice
    /// says.
    SecondPad,
    /// The host's AI plays it ([`Ai::fight`], [`LivePlayer::set_ai`]).
    /// With none, it stands still.
    Ai,
}

impl InFight {
    /// Every choice, in the order a host offers them.
    pub const ALL: [InFight; 5] = [InFight::StandsStill, InFight::MovesAround, InFight::AttacksAtRandom, InFight::Ai, InFight::SecondPad];
}

/// What an opponent does on its custom screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum OnCustomScreen {
    /// It presses OK with no chip.
    PicksNothing,
    /// It picks the first chip it can and presses OK ([`bot_buttons`]).
    #[default]
    PicksFirst,
    /// The host's AI presses it ([`Ai::custom_screen`],
    /// [`LivePlayer::set_ai`]). With none, it picks the first chip it can.
    Ai,
}

impl OnCustomScreen {
    /// Every choice, in the order a host offers them.
    pub const ALL: [OnCustomScreen; 3] = [OnCustomScreen::PicksNothing, OnCustomScreen::PicksFirst, OnCustomScreen::Ai];
}

/// What a training session changes of live play's battle between its
/// ticks, as a training mode does (`LivePlayer::set_training`,
/// `crate::Player::set_training`). Each is a host's option, applied to the
/// battle's own typed state before each tick (`Driver::prepare`), never a
/// rule of the simulation: a battle with all of them off (the default) is
/// the set's as ever, and one with any on is no match's (it isn't
/// recorded). Each acts at once, and in a battle paused or not.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Training {
    /// By side: its navi's HP returns to its full value once a hit is over,
    /// [`REFILL_REST`] ticks after it last lost any. (A hit, or hits that
    /// close together, of its whole HP still delete it.)
    pub refills: [bool; 2],
    /// The custom gauge is full whenever it runs: L or R opens the custom
    /// screen at any time in the fight. The engine sets the gauge's full
    /// flag and plays its sound as it fills to it, on the next tick that
    /// fills it (none while the battle is paused or dimmed).
    pub full_gauge: bool,
    /// The order both sides' custom screens deal their folders in.
    pub chip_draw: ChipDraw,
}

/// The order a custom screen deals the battle folder in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ChipDraw {
    /// As the round shuffled it (each console's own shuffle at the round's
    /// start: the match's).
    #[default]
    Shuffled,
    /// The folder's own order, dealt from its top, as the games deal a
    /// folder they don't shuffle (the battle's effect 0x80 clear,
    /// `sub_800A3E4`: no Regular chip first and no tag pair either). What a
    /// screen leaves unpicked stays at the top for the next. Set during a
    /// round, what is left of the folder is put in that order (the round's
    /// shuffle again, set back), from the next screen's deal on; a custom
    /// screen being shown keeps its deal.
    FolderOrder,
}

/// How long a navi with refills goes with no HP lost before its HP is full
/// again: a second and a half. The HUD's HP number rolls down to what a hit
/// left in under a second (an eighth of the way and 2 more a tick), so a
/// combo's damage is read before the refill rolls it back up, and the
/// refill doesn't land in the combo.
pub const REFILL_REST: u32 = 90;

/// What live play follows of a navi's HP for its refills.
#[derive(Clone, Copy, Debug, Default)]
struct Refill {
    /// Its HP before the tick just run, and the ticks since it last fell.
    hp: Option<u16>,
    rest: u32,
}

/// Plays a set from the keyboard, round after round to its end: the local
/// player is the left navi, with their own custom screen; the right navi
/// is an [`Opponent`]'s (by default one that stands still, and whose custom
/// screen picks the first chip it can and presses OK). A training session's
/// options ([`Training`]) change the battle between its ticks.
pub struct LivePlayer {
    set: Set,
    /// Ticks played since the start, through every round (a frame's number).
    ticks: u32,
    /// The round being played, from 1.
    round: u8,
    opponent: Opponent,
    /// The opponent's buttons reach nothing (`Driver::hold_opponent`).
    held: bool,
    /// The seed the opponent's random presses start from, and them.
    seed: u64,
    masher: Masher,
    training: Training,
    /// Each side's navi's HP, followed for its refills.
    refill: [Refill; 2],
    /// The order each side's battle folder is in this round.
    arranged: [ChipDraw; 2],
    /// What the host's second pad holds this tick, while it has one.
    second_pad: Option<u16>,
    /// The host's AI, for an opponent whose choices are its.
    ai: Option<Box<dyn Ai>>,
}

/// Put what is left of `side`'s battle folder in `draw`'s order: the
/// folder's own (`saved`, from its top), with no Regular chip first and no
/// tag pair, as a game's unshuffled folder has; or the round's shuffle (its
/// setup's battle folder), with its Regular chip first and its tag pair if
/// no custom screen has dealt from it yet. A chip the order hasn't (none a
/// folder's own) goes last.
fn arrange(b: &mut Battle, side: usize, draw: ChipDraw, saved: &SavedFolder) {
    let dealt = b.custom.sides[side].screen.is_some();
    let setup = &b.setup.players[side];
    let (order, regular, tag_pair): (Vec<FolderChip>, bool, Option<u8>) = match draw {
        ChipDraw::FolderOrder => (saved.chips.to_vec(), false, None),
        ChipDraw::Shuffled => {
            let order = setup.folder.chips.iter().flatten().copied().collect();
            (order, !dealt && setup.folder.regular_pending, setup.console.tag_pair.filter(|_| !dealt))
        }
    };
    let folder = &mut b.custom.sides[side].folder;
    let mut left: Vec<FolderChip> = folder.chips.iter().flatten().copied().collect();
    let mut placed: Vec<FolderChip> = Vec::with_capacity(left.len());
    for chip in order {
        if let Some(at) = left.iter().position(|c| *c == chip) {
            placed.push(left.remove(at));
        }
    }
    placed.append(&mut left);
    folder.chips = std::array::from_fn(|i| placed.get(i).copied());
    folder.regular_pending = regular;
    b.consoles[side].tag_pair = tag_pair;
}

/// Whether the fight runs on the tick after `b` (not the turn's banner,
/// the custom screen, the game's pause or a round decided).
#[cfg(test)]
fn fight_runs(b: &Battle) -> bool {
    b.round.mode == mode::FIGHTING && b.fight.state == nettai_battle::battle::fight::FIGHTING && !b.paused && !b.is_battle_over()
}

impl LivePlayer {
    pub fn new(set: Set) -> LivePlayer {
        LivePlayer::against(set, Opponent::default(), 0)
    }

    /// A set against `opponent`, whose random presses start from `seed`,
    /// the same each time the set starts.
    pub fn against(set: Set, opponent: Opponent, seed: u64) -> LivePlayer {
        LivePlayer {
            set,
            ticks: 0,
            round: 1,
            opponent,
            held: false,
            seed,
            masher: masher(seed),
            training: Training::default(),
            refill: Default::default(),
            arranged: [ChipDraw::Shuffled; 2],
            second_pad: None,
            ai: None,
        }
    }

    /// The host's AI, which plays the opponent where its choices are the
    /// AI's ([`InFight::Ai`], [`OnCustomScreen::Ai`]), from the next tick
    /// (none: there it stands still, and picks the first chip it can).
    pub fn set_ai(&mut self, ai: Option<Box<dyn Ai>>) {
        self.ai = ai;
    }

    /// What a second pad of the host's holds from the next tick, while it
    /// has one (none: it has no second pad): the opponent's buttons, when
    /// that is its choice ([`InFight::SecondPad`]).
    pub fn set_second_pad(&mut self, held: Option<u16>) {
        self.second_pad = held;
    }

    /// The same, with a training session's options from its first tick.
    pub fn with_training(mut self, training: Training) -> LivePlayer {
        self.training = training;
        self
    }

    /// The set it plays.
    pub fn set(&self) -> &Set {
        &self.set
    }

    /// Play `set` from its next start (`Driver::start`: the player's
    /// `restart`) in place of the one it plays, which goes on until then:
    /// a training session's set on another place.
    pub fn play_set(&mut self, set: Set) {
        self.set = set;
    }

    /// The training session's options.
    pub fn training(&self) -> Training {
        self.training
    }

    /// Have the training session's options change the battle `b` (the
    /// set's, as it stands) from now: at once, and before every tick after
    /// (`Driver::prepare`). A navi whose refills come on with HP missing has
    /// it back at once.
    pub fn set_training(&mut self, training: Training, b: &mut Battle) {
        let was = std::mem::replace(&mut self.training, training);
        for side in 0..2 {
            if training.refills[side] && !was.refills[side] {
                self.refill[side].rest = REFILL_REST;
            }
        }
        self.apply_training(b);
    }

    /// The options on `b` as it stands: what [`Driver::prepare`] does, with
    /// no tick counted.
    fn apply_training(&mut self, b: &mut Battle) {
        let over = b.is_battle_over();
        for side in 0..2 {
            let Some(r) = b.player(side as u8) else { continue };
            let navi = b.objects.get_mut(r);
            // (A navi at 0 is being deleted, and a round that is over is
            // decided: those go on.)
            if self.training.refills[side] && self.refill[side].rest >= REFILL_REST && navi.hp > 0 && navi.hp < navi.max_hp && !over {
                navi.hp = navi.max_hp;
            }
            self.refill[side].hp = Some(navi.hp);
        }
        // (The gauge runs from each custom screen's end until the next opens, and not after the round is decided.)
        if self.training.full_gauge && b.gauge.enabled && b.gauge.value < CustomGauge::FULL {
            b.gauge.value = CustomGauge::FULL;
        }
        // (A custom screen being shown deals from where its folder's chips are: it keeps them there.)
        for side in 0..2 {
            if self.arranged[side] != self.training.chip_draw && b.round.mode != mode::CUSTOM {
                arrange(b, side, self.training.chip_draw, &self.set.folders()[side]);
                self.arranged[side] = self.training.chip_draw;
            }
        }
    }

    /// What the opponent does.
    pub fn opponent(&self) -> Opponent {
        self.opponent
    }

    /// Have the opponent do `opponent` from the next tick on. The random
    /// presses it takes go on from where the set's ticks have brought them
    /// (they run every tick, taken or not): what it presses on a tick goes
    /// by the seed, the tick, the battle and its choices on that tick,
    /// whatever its choices were before.
    pub fn set_opponent(&mut self, opponent: Opponent) {
        self.opponent = opponent;
    }

    /// The opponent's buttons on side `side` this tick (`keys`: the local
    /// player's, which its AI hears of).
    fn opponent_buttons(&mut self, b: &Battle, side: usize, keys: u16) -> u16 {
        const MOVES: u16 = keys::UP | keys::DOWN | keys::LEFT | keys::RIGHT;
        let mashed = self.masher.buttons();
        // (START outside the custom screen is the pause's, the local player's alone: the game's own pause, in a
        // round that pauses itself.)
        if let Some(held) = self.second_pad.filter(|_| self.opponent.in_fight == InFight::SecondPad) {
            return if b.round.mode == mode::CUSTOM { held } else { held & !keys::START };
        }
        if b.round.mode == mode::CUSTOM {
            return match (self.opponent.on_custom_screen, &mut self.ai) {
                (OnCustomScreen::Ai, Some(ai)) => ai.custom_screen(b, side as u8),
                (choice, _) => custom_buttons(b, side, self.ticks, choice != OnCustomScreen::PicksNothing),
            };
        }
        // (Only the player opens the custom screen: a dummy the computer plays never presses L or R in the fight, and
        // makes its choices once the player has opened it. The user: "in training mode, don't have the dummy go into
        // custom automatically. only the player should trigger custom and then the dummy will make its custom
        // selections". A second pad is a person, and opens it as the player does.)
        let pressed = match self.opponent.in_fight {
            InFight::StandsStill => 0,
            InFight::MovesAround => mashed & MOVES,
            InFight::AttacksAtRandom => mashed,
            InFight::SecondPad => 0,
            InFight::Ai => self.ai.as_mut().map_or(0, |ai| ai.fight(b, side as u8, keys)),
        };
        pressed & !(keys::L | keys::R)
    }
}

/// An opponent's random presses from `seed`, the buster's among them: one
/// who only moves takes their directions.
fn masher(seed: u64) -> Masher {
    let mut m = Masher::new(seed);
    m.buster = true;
    m
}

/// A set's result as its player is told it.
pub fn result_text(r: BattleResult) -> &'static str {
    match r {
        BattleResult::Won => "you won",
        BattleResult::Lost => "you lost",
        BattleResult::Drawn => "a draw",
        _ => "cut short",
    }
}

/// The right navi's buttons: on its custom screen, A on the chip under the
/// cursor (the first one) if it can be picked, then START and A, a press
/// every other tick. (A host's demo battle presses the left one's custom
/// screen with them too.)
pub fn bot_buttons(b: &Battle, side: usize, tick: u32) -> u16 {
    custom_buttons(b, side, tick, true)
}

/// A custom screen's buttons straight to OK with no chip, a press every
/// other tick, as an opponent that picks nothing presses them (a host's
/// battle that only has to get to its fight).
pub fn ok_buttons(b: &Battle, side: usize, tick: u32) -> u16 {
    custom_buttons(b, side, tick, false)
}

/// A custom screen's buttons as [`bot_buttons`] presses them; without
/// `picks`, straight to OK with no chip.
fn custom_buttons(b: &Battle, side: usize, tick: u32, picks: bool) -> u16 {
    let s = &b.custom.sides[side];
    let Some(screen) = s.screen.as_ref().filter(|_| b.round.mode == mode::CUSTOM && s.in_custom) else { return 0 };
    if screen.phase != Phase::Choosing || tick % 2 == 0 {
        return 0;
    }
    let here = &screen.slots[screen.cursor as usize];
    if picks && screen.selected == 0 && matches!(here.kind, SlotKind::Chip { .. }) && here.state == SlotState::Selectable {
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
        self.round = 1;
        self.masher = masher(self.seed);
        if let Some(ai) = &mut self.ai {
            ai.reset();
        }
        self.refill = Default::default();
        self.arranged = [ChipDraw::Shuffled; 2];
        self.set.start()
    }

    /// The training session's options on the battle the last tick left:
    /// each navi with refills that lost no HP for [`REFILL_REST`] ticks has
    /// all of it back.
    fn prepare(&mut self, b: &mut Battle) {
        if self.training == Training::default() && self.arranged == [ChipDraw::Shuffled; 2] {
            return;
        }
        for side in 0..2 {
            let hp = b.player(side as u8).map(|r| b.objects.get(r).hp);
            let f = &mut self.refill[side];
            f.rest = if matches!((f.hp, hp), (Some(was), Some(now)) if now < was) { 0 } else { f.rest.saturating_add(1) };
        }
        self.apply_training(b);
    }

    fn next(&mut self, b: &Battle, keys: u16) -> Option<Step> {
        self.ticks += 1;
        let local = b.setup.local_side as usize;
        // (Held, the opponent's presses are made as ever, and go nowhere.)
        let buttons = [0, 1].map(|side| if side == local { keys } else { self.opponent_buttons(b, side, keys) * !self.held as u16 });
        // (The buttons are the whole input, as a netplay peer's are.)
        let TickInput { players, events } = nettai_netplay::standin::tick_input(b, buttons);
        Some(Step { input: players, events, frame: Some(self.ticks) })
    }

    fn round_ended(&mut self, b: &Battle) -> Option<After> {
        let after = self.set.after(b, b.setup.local_side)?;
        if matches!(after, After::Round(_)) {
            self.round += 1;
            // (The next round's folders are shuffled as its start, and its navis are new.)
            self.arranged = [ChipDraw::Shuffled; 2];
            self.refill = Default::default();
        }
        Some(after)
    }

    fn position(&self) -> String {
        format!("live round {} tick {}", self.round, self.ticks)
    }

    /// (A set changed by a training session's options is no match's: it
    /// isn't recorded.)
    fn record(&mut self) -> bool {
        self.ticks == 0 && self.training == Training::default()
    }

    fn live(&self) -> Option<&LivePlayer> {
        Some(self)
    }

    fn live_mut(&mut self) -> Option<&mut LivePlayer> {
        Some(self)
    }

    fn opponent(&self) -> Option<Opponent> {
        Some(self.opponent)
    }

    fn set_opponent(&mut self, opponent: Opponent) -> bool {
        LivePlayer::set_opponent(self, opponent);
        true
    }

    fn hold_opponent(&mut self, held: bool) -> bool {
        self.held = held;
        true
    }
}

/// A short set for tests (`crate::testing`): a match a shot decides, the
/// buttons that win it, and a driver that plays both sides of one.
#[cfg(any(test, feature = "testing"))]
pub mod short_set {
    use super::*;

    /// A match of `game` picked from `seed` (two MegaMen, the first a
    /// version's that has one), the right navi with 1 HP: a round is over
    /// when the left one's buster hits.
    pub fn of(content: &Arc<Content>, game: &str, seed: u32) -> nettai_match::Match {
        let mut m = nettai_match::pick::live(content, game, seed, None).unwrap();
        // (A base HP of 1: what the save brings, which the NaviCust's
        // compile makes the maximum.)
        let base_hp = nettai_battle::content::PlayerFact::BaseHp.name();
        m.sides[1].set_fact(content, base_hp, &[nettai_battle::rules::Fact::Value(nettai_content_api::Value::Int(1))]).unwrap();
        assert_eq!(nettai_match::check_match(content, &m), Vec::<String>::new());
        m
    }

    /// The buttons of the player on `side` who wins it: on the custom
    /// screen a chip and OK, as the standing navi's screen presses them; in
    /// the fight the buster, again and again.
    pub fn shooter(b: &Battle, side: usize, tick: u32) -> u16 {
        if b.round.mode == mode::CUSTOM {
            bot_buttons(b, side, tick)
        } else if tick % 8 < 2 {
            keys::B
        } else {
            0
        }
    }

    /// A match of `game` whose set is first to `wins` round wins (each
    /// round's place left to `seed`), picked as [`of`] picks one, both navis
    /// with 1 HP: a round is over when the shooter's buster hits.
    pub fn duel(content: &Arc<Content>, game: &str, seed: u32, wins: u8) -> nettai_match::Match {
        let mut m = of(content, game, seed);
        m.rounds = vec![nettai_match::RoundSettings::default(); nettai_match::rounds_for(wins)];
        let base_hp = nettai_battle::content::PlayerFact::BaseHp.name();
        m.sides[0].set_fact(content, base_hp, &[nettai_battle::rules::Fact::Value(nettai_content_api::Value::Int(1))]).unwrap();
        assert_eq!(nettai_match::check_match(content, &m), Vec::<String>::new());
        m
    }

    /// Live play of a set by both players, each round won as `winners`
    /// says: in round `n` the side `winners[n - 1]` shoots and the other
    /// only presses through its custom screen (past the list, side 0
    /// shoots). The local player's buttons are not asked.
    pub struct Scripted {
        pub live: LivePlayer,
        pub winners: Vec<usize>,
    }

    impl Driver for Scripted {
        fn start(&mut self) -> Battle {
            self.live.start()
        }

        fn next(&mut self, b: &Battle, _keys: u16) -> Option<Step> {
            self.live.ticks += 1;
            let ticks = self.live.ticks;
            let winner = self.winners.get(self.live.round as usize - 1).copied().unwrap_or(0);
            let buttons = [0, 1].map(|side| if side == winner { shooter(b, side, ticks) } else { bot_buttons(b, side, ticks) });
            let TickInput { players, events } = nettai_netplay::standin::tick_input(b, buttons);
            Some(Step { input: players, events, frame: Some(ticks) })
        }

        fn round_ended(&mut self, b: &Battle) -> Option<After> {
            self.live.round_ended(b)
        }

        fn position(&self) -> String {
            self.live.position()
        }

        fn record(&mut self) -> bool {
            self.live.record()
        }
    }
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
        let mut live = LivePlayer::new(Set::new(content.clone(), live_setup(&content, settings, [folder, folder], "falzar", 7), [folder, folder]));
        let mut b = live.start();
        let mut shown = false;
        for tick in 0..3000u32 {
            let s = &b.custom.sides[0];
            let choosing = s.in_custom && s.screen.as_ref().is_some_and(|x| x.phase == Phase::Choosing);
            shown |= choosing && s.screen.as_ref().is_some_and(|x| x.slots[custom::screen::OK_SLOT as usize].kind == SlotKind::Ok);
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

    /// A match of EXE6 picked from seed 5, 1000 HP each so a round lasts
    /// the test.
    fn long_match(content: &Arc<Content>) -> nettai_match::Match {
        let mut m = nettai_match::pick::live(content, "exe6", 5, None).unwrap();
        for s in &mut m.sides {
            s.set_fact(content, "hp", &[nettai_battle::rules::Fact::Value(nettai_content_api::Value::Int(1000))]).unwrap();
        }
        m
    }

    /// Every opponent: each choice in the fight with each on the custom
    /// screen.
    fn every_opponent() -> impl Iterator<Item = Opponent> {
        InFight::ALL.into_iter().flat_map(|in_fight| OnCustomScreen::ALL.map(|on_custom_screen| Opponent { in_fight, on_custom_screen }))
    }

    /// Each opponent through a few custom screens and fights, the local
    /// player standing still: one that picks the first chip starts the
    /// fight with it in hand and one that picks nothing with none, whatever
    /// it does in the fight; one that moves around or attacks leaves the
    /// panel it starts on, one that stands still never does; only one that
    /// attacks presses anything but a direction in the fight; a set against
    /// one started again gets the same presses.
    #[test]
    fn each_opponent_plays() {
        let content = nettai_match::testing::exe6_content();
        let m = long_match(&content);
        for opponent in every_opponent() {
            let mut live = LivePlayer::against(Set::of(&content, &m, 5), opponent, 9);
            let mut run = || {
                let mut b = live.start();
                let (mut hand, mut start, mut moved, mut attacked, mut presses) = (None, None, false, false, Vec::new());
                for tick in 0..1500u32 {
                    let keys = if b.round.mode == mode::CUSTOM { bot_buttons(&b, 0, tick) } else { 0 };
                    let custom = b.round.mode == mode::CUSTOM;
                    let step = live.next(&b, keys).unwrap();
                    presses.push(step.input[1]);
                    attacked |= !custom && step.input[1].held & (keys::A | keys::B | keys::L | keys::R) != 0;
                    assert!(custom || step.input[1].held & keys::START == 0, "{opponent:?}: START outside the custom screen at tick {tick}");
                    assert!(custom || opponent.in_fight == InFight::SecondPad || step.input[1].held & (keys::L | keys::R) == 0, "{opponent:?}: L or R in the fight at tick {tick}: only the player opens the custom screen");
                    b.tick(&step.input, step.events);
                    assert!(!b.is_stopped(), "{opponent:?}: tick {tick}");
                    if b.round.mode == mode::FIGHTING && hand.is_none() {
                        hand = Some(b.hands[1].remaining());
                    }
                    if let Some(p) = b.player(1).filter(|_| b.round.mode == mode::FIGHTING) {
                        let at = b.objects.get(p).panel;
                        moved |= start.is_some_and(|s| s != at);
                        start.get_or_insert(at);
                    }
                }
                (hand, moved, attacked, presses)
            };
            let (hand, moved, attacked, presses) = run();
            assert_eq!(hand, Some(if opponent.on_custom_screen == OnCustomScreen::PicksNothing { 0 } else { 1 }), "{opponent:?}");
            assert_eq!(moved, matches!(opponent.in_fight, InFight::MovesAround | InFight::AttacksAtRandom), "{opponent:?}: nothing recorded, no second pad and no AI given, it stands still");
            assert_eq!(attacked, opponent.in_fight == InFight::AttacksAtRandom, "{opponent:?}");
            assert_eq!(run().3, presses, "{opponent:?}: the same presses again");
        }
    }

    /// The opponent's choices change while the set runs, each alone: from
    /// the tick after a change it presses what the new choice presses.
    /// What it presses in the fight on a tick goes by the seed and the
    /// tick, whatever it did before (its random presses run every tick,
    /// taken or not): a set changed on the way presses, from the change on,
    /// what the seed's presses are on those ticks. Its custom screen's
    /// choice changed before a screen opens is that screen's.
    #[test]
    fn the_opponents_choices_change_while_the_set_runs() {
        const MOVES: u16 = keys::UP | keys::DOWN | keys::LEFT | keys::RIGHT;
        let content = nettai_match::testing::exe6_content();
        let m = long_match(&content);
        let mut live = LivePlayer::against(Set::of(&content, &m, 5), Opponent { in_fight: InFight::StandsStill, on_custom_screen: OnCustomScreen::PicksNothing }, 9);
        assert_eq!(Driver::opponent(&live), Some(live.opponent()));
        let mut b = live.start();
        // The seed's presses, a tick at a time, as the driver draws them.
        let mut seeded = masher(9);
        let mut hands = Vec::new();
        let (mut stood, mut moved, mut attacked) = (0, 0, 0);
        for tick in 0..4000u32 {
            // The fight's choice changes at ticks 900 and 1500; the custom
            // screen's once the first fight has started with no chip.
            let now = live.opponent();
            match tick {
                900 => assert!(Driver::set_opponent(&mut live, Opponent { in_fight: InFight::MovesAround, ..now })),
                1500 => live.set_opponent(Opponent { in_fight: InFight::AttacksAtRandom, ..now }),
                _ => {}
            }
            if hands.len() == 1 && live.opponent().on_custom_screen == OnCustomScreen::PicksNothing {
                live.set_opponent(Opponent { on_custom_screen: OnCustomScreen::PicksFirst, ..live.opponent() });
            }
            // The local player opens the custom screen again when it can.
            let keys = if b.round.mode == mode::CUSTOM {
                bot_buttons(&b, 0, tick)
            } else if b.round.flags & nettai_battle::battle::battle_flags::GAUGE_FULL != 0 && tick % 2 == 1 {
                keys::L
            } else {
                0
            };
            let custom = b.round.mode == mode::CUSTOM;
            let step = live.next(&b, keys).unwrap();
            let (pressed, mashed) = (step.input[1].held, seeded.buttons());
            if !custom {
                match live.opponent().in_fight {
                    InFight::StandsStill => {
                        assert_eq!(pressed, 0, "tick {tick}");
                        stood += 1;
                    }
                    InFight::MovesAround => {
                        assert_eq!(pressed, mashed & MOVES, "tick {tick}");
                        moved += (pressed != 0) as u32;
                    }
                    InFight::AttacksAtRandom => {
                        // (Not L or R: only the player opens the custom screen.)
                        assert_eq!(pressed, mashed & 0x3FF & !(keys::L | keys::R), "tick {tick}");
                        attacked += (pressed & !MOVES != 0) as u32;
                    }
                    InFight::SecondPad | InFight::Ai => unreachable!("not chosen here"),
                }
            }
            let turn = b.round.turn;
            b.tick(&step.input, step.events);
            assert!(!b.is_stopped(), "tick {tick}");
            // A fight starts: the hand the opponent brought from the screen.
            if b.round.mode == mode::FIGHTING && hands.len() < turn as usize {
                hands.push(b.hands[1].remaining());
            }
        }
        assert!(stood > 300 && moved > 100 && attacked > 100, "{stood} ticks standing, {moved} moving, {attacked} attacking");
        assert!(hands.len() >= 2, "{hands:?}");
        assert_eq!((hands[0], hands[1]), (0, 1), "no chip from the first screen, the first chip from the second: {hands:?}");
    }

    /// The opponent's buttons as live play pressed them before the training
    /// session's options and the dummy's recording and second pad (2026-10-12,
    /// nettai-ui at 6834a06ca), frozen here: the seed's presses every tick,
    /// taken or not; on the custom screen its choice's presses.
    struct FrozenOpponent {
        opponent: Opponent,
        masher: Masher,
        ticks: u32,
    }

    impl FrozenOpponent {
        fn buttons(&mut self, b: &Battle) -> u16 {
            const MOVES: u16 = keys::UP | keys::DOWN | keys::LEFT | keys::RIGHT;
            self.ticks += 1;
            let mashed = self.masher.buttons();
            if b.round.mode == mode::CUSTOM {
                return custom_buttons(b, 1, self.ticks, self.opponent.on_custom_screen != OnCustomScreen::PicksNothing);
            }
            match self.opponent.in_fight {
                InFight::StandsStill => 0,
                InFight::MovesAround => mashed & MOVES,
                // (Not L or R: only the player opens the custom screen.)
                InFight::AttacksAtRandom => mashed & !(keys::L | keys::R),
                // (No second pad: it stands still.)
                InFight::SecondPad => 0,
                // (No AI given: it stands still.)
                InFight::Ai => 0,
            }
        }
    }

    /// A set of `m` played as a training session plays it, with every
    /// option at its normal value, through the session (whose driver's
    /// hooks run before every tick), beside the same set stepped by hand
    /// as live play stepped it before those options: the two are the same
    /// battle, tick for tick, by digest. The local player mashes in the
    /// fight and picks through each custom screen. The digests, folded (for
    /// the test's output), the rounds that ended and whether the set did.
    fn same_as_before(content: &Arc<Content>, m: &nettai_match::Match, seed: u32, opponent: Opponent, ticks: u32) -> (u64, u32, bool) {
        let set = Set::of(content, m, seed);
        let mut session = crate::session::Session::new(Box::new(LivePlayer::against(set.clone(), opponent, seed as u64)));
        let mut frozen = FrozenOpponent { opponent, masher: masher(seed as u64), ticks: 0 };
        let mut by_hand = set.start();
        let mut player = Masher::new(seed as u64 ^ 0x5EED);
        let (mut fold, mut rounds) = (0u64, 0);
        for tick in 0..ticks {
            let mashed = player.buttons();
            let keys = if by_hand.round.mode == mode::CUSTOM { bot_buttons(&by_hand, 0, tick) } else { mashed };
            let them = frozen.buttons(&by_hand);
            let TickInput { players, events } = nettai_netplay::standin::tick_input(&by_hand, [keys, them]);
            by_hand.tick(&players, events);
            let went_on = session.step(keys);
            // (A round that ended is the next one's battle at its start, as the session shows it.)
            match set.after(&by_hand, 0) {
                None => assert!(went_on, "{opponent:?}: tick {tick}"),
                Some(After::Round(next)) => {
                    by_hand = *next;
                    rounds += 1;
                }
                Some(_) => {
                    assert_eq!(session.battle.digest(), by_hand.digest(), "{opponent:?}: tick {tick}");
                    assert!(session.finished, "{opponent:?}: the set ended at tick {tick}");
                    return (fold ^ by_hand.digest(), rounds + 1, true);
                }
            }
            assert_eq!(session.battle.digest(), by_hand.digest(), "{opponent:?}: tick {tick}");
            fold = fold.rotate_left(5) ^ by_hand.digest();
        }
        (fold, rounds, false)
    }

    /// A set played with the training session's options all at their
    /// normal values is the set live play played before there were any:
    /// every game, every opponent, a long round and a short set's rounds
    /// to its end.
    #[test]
    fn a_set_with_the_normal_options_is_the_set_as_before() {
        for game in ["exe4", "exe5", "exe6"] {
            let content = match game {
                "exe4" => nettai_match::testing::exe4_content(),
                "exe5" => nettai_match::testing::exe5_content(),
                _ => nettai_match::testing::exe6_content(),
            };
            let mut long = nettai_match::pick::live(&content, game, 5, None).unwrap();
            for s in &mut long.sides {
                s.set_fact(&content, "hp", &[nettai_battle::rules::Fact::Value(nettai_content_api::Value::Int(1000))]).unwrap();
            }
            let mut folds = Vec::new();
            for opponent in every_opponent() {
                folds.push(same_as_before(&content, &long, 5, opponent, 1500).0);
            }
            let short = short_set::duel(&content, game, 7, nettai_match::TRIPLE_BATTLE);
            let (fold, rounds, ended) = same_as_before(&content, &short, 7, Opponent { in_fight: InFight::MovesAround, on_custom_screen: OnCustomScreen::PicksFirst }, 30_000);
            assert!(ended && rounds >= 2, "{game}: the short set played {rounds} rounds and ended: {ended}");
            folds.push(fold);
            eprintln!("{game}: {folds:x?}");
        }
    }

    /// A navi's HP and its full HP, by side, in `b` (none before it is on the field).
    fn hp(b: &Battle, side: u8) -> (u16, u16) {
        b.player(side).map_or((0, 0), |r| (b.objects.get(r).hp, b.objects.get(r).max_hp))
    }

    /// With refills, a navi's HP is full again [`REFILL_REST`] ticks after
    /// it last lost any, and not before: shot again and again (the buster
    /// every few ticks), the dummy's HP goes down and stays down while the
    /// shots land, and comes back once they stop; the other side's, without
    /// refills, is left as it goes. Turned on with HP missing, at once. And
    /// a set with refills isn't recorded.
    #[test]
    fn hp_refills_once_a_hit_is_over() {
        let content = nettai_match::testing::exe6_content();
        let m = long_match(&content);
        let refills = Training { refills: [false, true], ..Training::default() };
        let mut live = LivePlayer::against(Set::of(&content, &m, 5), Opponent { in_fight: InFight::StandsStill, on_custom_screen: OnCustomScreen::PicksFirst }, 9).with_training(refills);
        assert!(!live.record(), "a set with the options isn't recorded");
        let mut s = crate::session::Session::new(Box::new(live));
        let (mut lost_at, mut losses, mut refilled) = (None, 0, 0);
        for tick in 0..3000u32 {
            let b = &s.battle;
            // The buster for a stretch, then nothing; on the custom screen, a chip and OK.
            let keys = if b.round.mode == mode::CUSTOM { bot_buttons(b, 0, tick) } else if tick < 1200 { short_set::shooter(b, 0, tick) } else { 0 };
            let before = hp(&s.battle, 1).0;
            assert!(s.step(keys), "tick {tick}");
            let (now, max) = hp(&s.battle, 1);
            if now < before && before > 0 {
                lost_at = Some(tick);
                losses += 1;
            } else if now > before && before > 0 {
                assert_eq!(now, max, "tick {tick}: all of it");
                assert_eq!(lost_at.map(|at| tick - at), Some(REFILL_REST + 1), "tick {tick}: a rest after the last hit");
                refilled += 1;
            }
        }
        assert!(losses > 10 && refilled > 0, "{losses} hits, {refilled} refills");
        assert_eq!(hp(&s.battle, 1).0, hp(&s.battle, 1).1, "full once the shots stopped");
        assert!(!s.battle.is_battle_over());
        // The player's, without refills (its HP taken down by hand: the dummy never shoots).
        let r = s.battle.player(0).unwrap();
        s.battle.objects.get_mut(r).hp -= 100;
        for _ in 0..200 {
            s.step(0);
        }
        assert_eq!(hp(&s.battle, 0).0 + 100, hp(&s.battle, 0).1, "no refills: as it goes");
        // Refills turned on with HP missing: at once.
        let both = Training { refills: [true, true], ..Training::default() };
        s.driver.live_mut().unwrap().set_training(both, &mut s.battle);
        assert_eq!(hp(&s.battle, 0).0, hp(&s.battle, 0).1);
    }

    /// With the gauge always full, it is full whenever it runs, from the
    /// tick after each custom screen ends, and the engine flags it full (L
    /// or R opens the screen) on the first tick it fills, once the turn's
    /// start lets it (about a second, the turn's banner); without, it fills
    /// at its rate, ten seconds a turn. Turned on mid-fight, at once.
    #[test]
    fn the_gauge_is_always_full() {
        use nettai_battle::battle::battle_flags;
        let content = nettai_match::testing::exe6_content();
        let m = long_match(&content);
        for full_gauge in [false, true] {
            let live = LivePlayer::against(Set::of(&content, &m, 5), Opponent::default(), 9).with_training(Training { full_gauge, ..Training::default() });
            let mut s = crate::session::Session::new(Box::new(live));
            let (mut opened, mut running, mut waits) = (0, 0u32, Vec::new());
            for tick in 0..6000u32 {
                let b = &s.battle;
                // On the custom screen a chip and OK; in the fight L as soon as the gauge is full.
                let keys = if b.round.mode == mode::CUSTOM {
                    bot_buttons(b, 0, tick)
                } else if b.round.flags & battle_flags::GAUGE_FULL != 0 && tick % 2 == 1 {
                    keys::L
                } else {
                    0
                };
                let (custom, was_full) = (b.round.mode == mode::CUSTOM, b.round.flags & battle_flags::GAUGE_FULL != 0);
                assert!(s.step(keys), "tick {tick}");
                let b = &s.battle;
                opened += (!custom && b.round.mode == mode::CUSTOM) as u32;
                if b.gauge.enabled && b.round.mode == mode::FIGHTING && !was_full {
                    running += 1;
                    if full_gauge && running > 1 {
                        assert_eq!(b.gauge.value, CustomGauge::FULL, "tick {tick}");
                    }
                    if b.round.flags & battle_flags::GAUGE_FULL != 0 {
                        waits.push(running);
                        running = 0;
                    }
                }
            }
            if full_gauge {
                assert!(opened >= 10 && waits.iter().all(|&w| w < 100), "{opened} screens, waits {waits:?}");
            } else {
                assert!(opened >= 2 && waits.iter().all(|&w| w > 500), "{opened} screens, waits {waits:?}");
            }
        }
        // Turned on in the fight, with the gauge running: full at once.
        let mut s = crate::session::Session::new(Box::new(LivePlayer::against(Set::of(&content, &m, 5), Opponent::default(), 9)));
        while !(s.battle.gauge.enabled && s.battle.round.mode == mode::FIGHTING && s.battle.gauge.value > 0) {
            let keys = if s.battle.round.mode == mode::CUSTOM { bot_buttons(&s.battle, 0, s.ticks as u32) } else { 0 };
            assert!(s.step(keys) && s.ticks < 3000);
        }
        s.driver.live_mut().unwrap().set_training(Training { full_gauge: true, ..Training::default() }, &mut s.battle);
        assert_eq!(s.battle.gauge.value, CustomGauge::FULL);
    }

    /// A second pad drives the opponent while that is its choice: what it
    /// holds is the opponent's buttons, in the fight and on its custom
    /// screen (the pad picking its chips); with no pad the opponent stands
    /// still and its screen does as its choice says; with another choice
    /// the pad is nobody's.
    #[test]
    fn a_second_pad_drives_the_opponent() {
        let content = nettai_match::testing::exe6_content();
        let m = long_match(&content);
        let mut live = LivePlayer::against(Set::of(&content, &m, 5), Opponent { in_fight: InFight::SecondPad, ..Opponent::default() }, 9);
        let mut b = live.start();
        let mut custom_ticks = 0;
        for tick in 0..3000u32 {
            // The second pad: on the custom screen as the player's own is pressed, in the fight up and down, and
            // L once the gauge is full.
            let full = b.round.flags & nettai_battle::battle::battle_flags::GAUGE_FULL != 0;
            let pad = if b.round.mode == mode::CUSTOM {
                bot_buttons(&b, 1, tick)
            } else if full && tick % 2 == 1 {
                keys::L
            } else {
                [keys::UP, keys::UP, 0, 0, keys::DOWN, keys::DOWN, 0, 0][tick as usize % 8]
            };
            let mine = if b.round.mode == mode::CUSTOM { bot_buttons(&b, 0, tick) } else { 0 };
            let with_pad = tick < 2000;
            live.set_second_pad(with_pad.then_some(pad));
            custom_ticks += (b.round.mode == mode::CUSTOM) as u32;
            let step = live.next(&b, mine).unwrap();
            if with_pad {
                assert_eq!(step.input[1].held, pad, "tick {tick}");
                assert!(b.round.mode == mode::CUSTOM || pad & keys::START == 0);
            } else if b.round.mode != mode::CUSTOM {
                assert_eq!(step.input[1].held, 0, "tick {tick}: no pad");
            }
            b.tick(&step.input, step.events);
        }
        assert!(custom_ticks > 0 && b.round.turn >= 2, "the pad's custom screens went on: turn {}", b.round.turn);
        while b.round.mode == mode::CUSTOM {
            let step = live.next(&b, bot_buttons(&b, 0, b.round.ticks)).unwrap();
            b.tick(&step.input, step.events);
        }
        live.set_second_pad(Some(keys::START | keys::B));
        assert_eq!(live.next(&b, 0).unwrap().input[1].held, keys::B, "the pause is the local player's");
        live.set_opponent(Opponent::default());
        live.set_second_pad(Some(keys::B));
        assert_eq!(live.next(&b, 0).unwrap().input[1].held, 0, "standing still: the pad is nobody's");
    }

    /// The options on a battle stepping in the game's own pause, as a
    /// training menu holds one (a round that pauses itself, START given and
    /// nothing else): the refill, the full gauge and the folder's order are
    /// applied in the pause and leave it as it is, and START takes the
    /// fight out of it as ever.
    #[test]
    fn the_options_act_in_the_games_own_pause() {
        let content = nettai_match::testing::exe6_content();
        let m = long_match(&content);
        let mut first = m.round(&content, 5);
        first.host_keys.pause = false;
        let set = Set::new(content.clone(), first, [0, 1].map(|side| m.sides[side].folder(&content).saved().unwrap()));
        let mut live = LivePlayer::against(set, Opponent::default(), 9);
        let mut b = live.start();
        let screen = |tick: u32, b: &Battle| if b.round.mode == mode::CUSTOM { bot_buttons(b, 0, tick) } else { 0 };
        play_until(&mut live, &mut b, 3000, screen, fight_runs);
        // The dummy's HP down; the gauge running.
        let r = b.player(1).unwrap();
        b.objects.get_mut(r).hp -= 100;
        // The game's pause: START, and no buttons.
        keys_in_turn(&mut live, &mut b, &[keys::START, 0]);
        assert_eq!(b.paused_by(), Some(0));
        let options = Training { refills: [true, true], full_gauge: true, chip_draw: ChipDraw::FolderOrder };
        live.set_training(options, &mut b);
        for tick in 0..300u32 {
            let step = live.next(&b, 0).unwrap();
            assert_eq!(step.input[1].held, 0, "tick {tick}: the dummy presses nothing in the pause");
            Driver::prepare(&mut live, &mut b);
            b.tick(&step.input, step.events);
            assert_eq!(b.paused_by(), Some(0), "tick {tick}: still in the game's pause");
        }
        assert_eq!(hp(&b, 1).0, hp(&b, 1).1, "the refill, in the pause");
        assert_eq!(b.gauge.value, CustomGauge::FULL);
        let saved = live.set().folders()[0].chips;
        let left: Vec<FolderChip> = b.custom.sides[0].folder.chips.iter().flatten().copied().collect();
        assert!(in_order(&left, &saved), "in the folder's order: {left:?}");
        // START: the fight goes on.
        keys_in_turn(&mut live, &mut b, &[keys::START, 0]);
        play_until(&mut live, &mut b, 100, |_, _| 0, fight_runs);
        assert_eq!(b.paused_by(), None);
    }

    /// What is left of each side's battle folder as each custom screen
    /// deals from it (in the order it deals), and the round's own shuffle
    /// of it, through `screens` screens of a set of `m`: the player and the
    /// dummy each pick the first chip they can; the player opens a screen
    /// as soon as the gauge is full, and from the `shoot`th screen on shoots
    /// instead (a match of 1 HP: the round ends). `changes`: the options
    /// set in the fight after that many screens.
    fn deals(content: &Arc<Content>, m: &nettai_match::Match, training: Training, changes: &[(usize, Training)], shoot: usize, screens: usize) -> Vec<[(Vec<FolderChip>, Vec<FolderChip>); 2]> {
        let live = LivePlayer::against(Set::of(content, m, 5), Opponent::default(), 9).with_training(training);
        let mut s = crate::session::Session::new(Box::new(live));
        let mut out = Vec::new();
        let mut was_open = false;
        for tick in 0..60_000u32 {
            let b = &s.battle;
            let open = b.round.mode == mode::CUSTOM && (0..2).all(|side| b.custom.sides[side].screen.as_ref().is_some_and(|x| x.phase == Phase::Choosing));
            if open && !was_open {
                out.push([0, 1].map(|side| {
                    let left = b.custom.sides[side].folder.chips.iter().flatten().copied().collect();
                    (left, b.setup.players[side].folder.chips.iter().flatten().copied().collect())
                }));
                if out.len() == screens {
                    break;
                }
            }
            was_open = open;
            if b.round.mode == mode::FIGHTING
                && let Some(&(_, then)) = changes.iter().find(|(after, then)| *after == out.len() && s.driver.live().unwrap().training() != *then)
            {
                s.driver.live_mut().unwrap().set_training(then, &mut s.battle);
            }
            let b = &s.battle;
            let keys = if b.round.mode == mode::CUSTOM {
                bot_buttons(b, 0, tick)
            } else if out.len() >= shoot {
                short_set::shooter(b, 0, tick)
            } else if b.round.flags & nettai_battle::battle::battle_flags::GAUGE_FULL != 0 && tick % 2 == 1 {
                keys::L
            } else {
                0
            };
            assert!(s.step(keys), "tick {tick}");
        }
        assert_eq!(out.len(), screens);
        out
    }

    /// Whether `part` is in `whole`'s order (each of its chips found
    /// further on than the last).
    fn in_order(part: &[FolderChip], whole: &[FolderChip]) -> bool {
        let mut rest = whole.iter();
        part.iter().all(|c| rest.any(|w| w == c))
    }

    /// Folder order deals each folder from its top, every screen and every
    /// round, for both sides: what is left of the folder is in the folder's
    /// order, and a round's first screen deals the whole folder as it was
    /// saved (no Regular chip first). Shuffled deals as each round shuffled
    /// it. Set in the fight, from the next screen on; set back, the next
    /// deals the round's shuffle again.
    #[test]
    fn folder_order_deals_from_the_top() {
        let content = nettai_match::testing::exe6_content();
        let m = short_set::duel(&content, "exe6", 7, nettai_match::TRIPLE_BATTLE);
        let saved = Set::of(&content, &m, 5).folders().map(|f| f.chips.to_vec());
        let folder_order = Training { chip_draw: ChipDraw::FolderOrder, ..Training::default() };
        // (Three screens of the first round, then the second round's first: two rounds won end the set.)
        let ordered = deals(&content, &m, folder_order, &[], 3, 4);
        let mut firsts = 0;
        for (n, deal) in ordered.iter().enumerate() {
            for side in 0..2 {
                let (left, _) = &deal[side];
                assert!(in_order(left, &saved[side]), "screen {n}, side {side}: {left:?}");
                if left.len() == 30 {
                    assert_eq!(left, &saved[side], "screen {n}, side {side}: a round's first");
                    firsts += 1;
                }
            }
        }
        assert!(firsts >= 4, "two rounds' first screens, both sides: {firsts}");
        let shuffled = deals(&content, &m, Training::default(), &[], 3, 4);
        for (n, deal) in shuffled.iter().enumerate() {
            for (side, (left, round)) in deal.iter().enumerate() {
                assert!(in_order(left, round), "screen {n}, side {side}");
            }
        }
        assert!(shuffled.iter().any(|d| d.iter().zip(&saved).any(|((left, _), saved)| !in_order(left, saved))), "a shuffle isn't the folder's order");
        // Set after the first screen, and back after the second.
        let changed = deals(&content, &m, Training::default(), &[(1, folder_order), (2, Training::default())], 99, 3);
        for side in 0..2 {
            assert!(in_order(&changed[1][side].0, &saved[side]), "side {side}");
            assert!(in_order(&changed[2][side].0, &changed[2][side].1), "side {side}");
        }
        assert_eq!(changed[0], shuffled[0]);
    }

    /// `--save-match` then `--match`: live play's draw for a seed, written
    /// as a match file and played from it, is the same battle as playing
    /// the draw itself, the same digest every tick (the local player
    /// mashing, the right navi the stand-in).
    #[test]
    fn a_saved_match_plays_the_same_battle() {
        let content = nettai_match::testing::exe6_content();
        for seed in [5, 77] {
            let mut drawn = nettai_match::pick::live(&content, "exe6", seed, None).unwrap();
            // (1000 HP each, so the round lasts the test.)
            for s in &mut drawn.sides {
                s.set_fact(&content, "hp", &[nettai_battle::rules::Fact::Value(nettai_content_api::Value::Int(1000))]).unwrap();
            }
            let text = nettai_match::write(&content, &drawn);
            let read = nettai_match::parse(&content, &text).unwrap();
            let mut a = LivePlayer::new(Set::of(&content, &drawn, seed));
            let mut b = LivePlayer::new(Set::of(&content, &read, read.seed.unwrap()));
            let (mut x, mut y) = (a.start(), b.start());
            let mut masher = nettai_netplay::standin::Masher::new(seed as u64);
            for tick in 0..2500 {
                // On the custom screen, a pick and OK as the stand-in does;
                // in the fight, anything.
                let mashed = masher.buttons();
                let keys = if x.round.mode == mode::CUSTOM { bot_buttons(&x, 0, tick) } else { mashed };
                let (sa, sb) = (a.next(&x, keys).unwrap(), b.next(&y, keys).unwrap());
                x.tick(&sa.input, sa.events);
                y.tick(&sb.input, sb.events);
                assert_eq!(x.digest(), y.digest(), "seed {seed}: tick {tick}");
            }
            assert!(x.round.turn > 1, "seed {seed}: the battle went on (turn {})", x.round.turn);
        }
    }

    /// Live play's keys on each game's custom screen, held as a person
    /// holds them (eight ticks), the cursor on the first chip: R opens the
    /// chip's description, SELECT hides the window; each stays when the key
    /// is let go. L, the run key, is the match's host's
    /// (`nettai_match::HOST_KEYS`: nobody runs from a match), so it opens
    /// nothing; in a round whose host takes no keys, a console's, it opens
    /// the navi's no-running message, which stays too.
    #[test]
    fn rl_and_select_work_on_each_games_live_screen() {
        for game in ["exe4", "exe5", "exe6"] {
            let content = match game {
                "exe4" => nettai_match::testing::exe4_content(),
                "exe5" => nettai_match::testing::exe5_content(),
                _ => nettai_match::testing::exe6_content(),
            };
            let hosts = nettai_match::HOST_KEYS;
            for (key, host_keys) in [(keys::R, hosts), (keys::L, hosts), (keys::L, nettai_battle::HostKeys::NONE), (keys::SELECT, hosts)] {
                let m = nettai_match::pick::live(&content, game, 5, None).unwrap();
                let set = Set::of(&content, &m, 5);
                let first = RoundSetup { host_keys, ..set.first().clone() };
                let folders = [0, 1].map(|side| m.sides[side].folder(&content).saved().unwrap());
                let mut live = LivePlayer::new(Set::new(content.clone(), first, folders));
                let mut b = live.start();
                play_until(&mut live, &mut b, 4000, |_, _| 0, |b| choosing(b).is_some());
                keys_in_turn(&mut live, &mut b, &[0; 20]);
                assert!(matches!(choosing(&b).unwrap().slots[0].kind, SlotKind::Chip { .. }), "{game}: a chip dealt");
                keys_in_turn(&mut live, &mut b, &[key; 8]);
                keys_in_turn(&mut live, &mut b, &[0; 8]);
                let phase = b.custom.sides[0].screen.unwrap().phase;
                let right = match key {
                    keys::R => matches!(phase, Phase::Description { window: None, .. }),
                    keys::L if host_keys.run => phase == Phase::Choosing,
                    keys::L => matches!(phase, Phase::RunMessage { chatbox: Some(_) }),
                    _ => matches!(phase, Phase::Hidden { .. }),
                };
                assert!(right, "{game}: key {key:#x}, {host_keys:?}: {phase:?}");
            }
        }
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

    /// Crosses of the other game on EXE6's content: a Falzar player offered
    /// HeatCross, Gregar's, chooses it on the custom screen and fights in
    /// it (its form, element, buster and charged shot: HeatCross's flame);
    /// on the next screen Beast Out from it is HeatCross's Beast form, a
    /// Gregar Beast, with its weapons.
    #[test]
    fn a_falzar_player_plays_a_gregar_cross() {
        use nettai_battle::battle::battle_flags;
        use nettai_battle::content::Element;
        use nettai_battle::kinds::player::{NaviAction, navi_action};
        let content = nettai_match::testing::exe6_content();
        let heat = content.defs.form_by_key("heatcross").unwrap();
        let heat_beast = content.defs.form_by_key("heatcross-beast").unwrap();
        let stage = nettai_match::link_battle_stages(&content, "exe6")[0];
        let settings = BattleSettings { stage, background: Default::default(), effects: content.stage(stage).effects | nettai_match::MATCH_EFFECTS };
        let folder = folder_of(&content, &[("cannon", 0)]);
        let mut setup = live_setup(&content, settings, [folder, folder], "falzar", 5);
        setup.players[0].set_fact(&content, "crosses", &[form_fact(heat)]).unwrap();
        let mut live = LivePlayer::new(Set::new(content.clone(), setup, [folder, folder]));
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
                let w = b.form_list(0).unwrap_or_default();
                match screen.phase {
                    Phase::Choosing if w.chosen.is_none() => [keys::UP, keys::UP, 0][tick as usize % 3],
                    // (The window up, past its first tick, which reads no
                    // keys.)
                    Phase::Window { window, tick: 1.. } if b.content.defs.window(window).name == "cross_window" && w.chosen.is_none() => {
                        let under_cursor = w.forms[w.cursor as usize];
                        assert_eq!((w.count, under_cursor), (1, Some(heat)));
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
        assert_eq!(content.form(heat).version.as_deref(), Some("gregar"));
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
                beast |= screen.form.is_some();
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
        assert_eq!(content.form(heat_beast).version.as_deref(), Some("gregar"));
    }

    // EXE6's Cross window (rules/cross's: content/exe6/rules/cross/
    // window.luau) with Crosses of both games, which no recording covers.

    /// A link battle on EXE6's content whose side 0 is a `version` player
    /// with the Crosses `list` (form keys; none: the version's own five),
    /// its stats changed by `tweak`, run to its first screen's choosing.
    fn cross_battle(
        version: &str,
        list: Option<&[&str]>,
        tweak: impl FnOnce(&Content, &mut nettai_battle::setup::NaviStats),
    ) -> (Arc<Content>, LivePlayer, Battle) {
        cross_battle_timed(version, list, tweak, None)
    }

    /// [`cross_battle`] with the custom screen's time limit `limit`, in
    /// ticks.
    fn cross_battle_timed(
        version: &str,
        list: Option<&[&str]>,
        tweak: impl FnOnce(&Content, &mut nettai_battle::setup::NaviStats),
        limit: Option<u16>,
    ) -> (Arc<Content>, LivePlayer, Battle) {
        let content = nettai_match::testing::exe6_content();
        let stage = nettai_match::link_battle_stages(&content, "exe6")[0];
        let settings = BattleSettings { stage, background: Default::default(), effects: content.stage(stage).effects | nettai_match::MATCH_EFFECTS };
        let folder = folder_of(&content, &[("cannon", 0)]);
        let mut setup = live_setup(&content, settings, [folder, folder], "falzar", 5);
        setup.players[0].set_fact(&content, "cybeast", &[nettai_match::facts::version_fact(&content, version).unwrap()]).unwrap();
        let megaman = content.navi(setup.navi_stats[0].navi).forms.as_ref().expect("MegaMan's forms");
        let list: Vec<_> = match list {
            Some(list) => list.iter().map(|k| form_fact(form_of(&content, k))).collect(),
            None => megaman.listed(version).iter().map(|&f| form_fact(f)).collect(),
        };
        setup.players[0].set_fact(&content, "crosses", &list).unwrap();
        tweak(&content, &mut setup.navi_stats[0]);
        setup.custom_screen_limit = limit;
        let mut live = LivePlayer::new(Set::new(content.clone(), setup, [folder, folder]));
        let mut b = live.start();
        play_until(&mut live, &mut b, 3000, |_, _| 0, |b| choosing(b).is_some());
        (content, live, b)
    }

    /// A form as a setup's fact takes it (an entry of a form list).
    fn form_fact(form: nettai_content_api::FormHandle) -> nettai_battle::rules::Fact<'static> {
        nettai_battle::rules::Fact::Value(nettai_content_api::Value::Def(nettai_content_api::Registry::Form, form.0))
    }

    /// The form `key` (EXE6's, without its prefix).
    fn form_of(content: &Content, key: &str) -> nettai_content_api::FormHandle {
        content.defs.form_by_key(&format!("{key}")).unwrap_or_else(|| panic!("no form {key}"))
    }

    /// Side 0's keys, one per tick, then on to the next tick.
    fn keys_in_turn(live: &mut LivePlayer, b: &mut Battle, held: &[u16]) {
        for &h in held {
            let step = live.next(b, h).unwrap();
            b.tick(&step.input, step.events);
        }
    }

    /// From choosing, UP opens the Cross window (a direction acts on a
    /// hold's second tick); then the window is up and takes keys.
    fn open_cross_window(live: &mut LivePlayer, b: &mut Battle) {
        keys_in_turn(live, b, &[keys::UP, keys::UP, 0]);
        play_until(live, b, 100, |_, _| 0, |b| {
            let s = b.custom.sides[0].screen.as_ref().unwrap();
            matches!(s.phase, Phase::Window { window, tick: 1.. } if b.content.defs.window(window).name == "cross_window")
        });
    }

    /// In the open Cross window, DOWN `down` times and A: the Cross under
    /// the cursor is chosen, and the screen is back to choosing chips.
    fn choose_cross(live: &mut LivePlayer, b: &mut Battle, down: usize) {
        for _ in 0..down {
            keys_in_turn(live, b, &[keys::DOWN, keys::DOWN, 0]);
        }
        keys_in_turn(live, b, &[keys::A, 0]);
        play_until(live, b, 100, |_, _| 0, |b| choosing(b).is_some());
    }

    /// OK, and what side 0 sends.
    fn confirm(live: &mut LivePlayer, b: &mut Battle) -> nettai_battle::CustomResult {
        keys_in_turn(live, b, &[keys::START, 0, keys::A, 0]);
        play_until(live, b, 200, |_, _| 0, |b| b.custom.sides[0].sent.is_some());
        b.custom.sides[0].sent.as_ref().unwrap().result.clone()
    }

    /// The rules' record of the Crosses used this round (EXE6's cross
    /// part's).
    fn crosses_used(b: &Battle) -> [bool; 5] {
        let (schema, state) = b.rules_state(0).expect("EXE6's rules");
        let i = schema.index_of("crosses_used").unwrap();
        std::array::from_fn(|k| state.get_elem(schema, i, k) == Some(nettai_content_api::FieldValue::Bool(true)))
    }

    /// A setup's Cross list offers Crosses of either game in its order: a
    /// Falzar player offered HeatCross (Gregar's first) and GroundCross
    /// (Falzar's fourth) gets those two; the one chosen shows its face and
    /// goes out, is used for the round, and isn't offered on the round's
    /// next screen.
    #[test]
    fn a_cross_list_offers_crosses_of_either_game_once_a_round() {
        use nettai_battle::battle::battle_flags;
        let (content, mut live, mut b) = cross_battle("falzar", Some(&["heatcross", "groundcross"]), |_, _| {});
        let heat = form_of(&content, "heatcross");
        let w = b.form_list(0).unwrap();
        assert_eq!((w.count, &w.offered[..2]), (2, &[0, 1][..]));
        open_cross_window(&mut live, &mut b);
        choose_cross(&mut live, &mut b, 0);
        let screen = b.custom.sides[0].screen.unwrap();
        assert_eq!((screen.look.face, b.form_list(0).unwrap().chosen), (Some(heat), Some(0)));
        assert_eq!(confirm(&mut live, &mut b).transform.form, Some(heat));
        assert_eq!(crosses_used(&b), [true, false, false, false, false]);
        // The round's next screen, opened with L once the gauge is full.
        play_until(
            &mut live,
            &mut b,
            6000,
            |tick, b| if b.round.mode == mode::FIGHTING && b.round.flags & battle_flags::GAUGE_FULL != 0 && tick % 2 == 1 { keys::L } else { 0 },
            |b| b.round.turn >= 2 && choosing(b).is_some(),
        );
        let w = b.form_list(0).unwrap();
        assert_eq!((w.count, w.offered[0]), (1, 1));
    }

    /// R in the Cross window describes the Cross under the cursor by its
    /// form: with a Cross list mixing both games, a Falzar player's window
    /// shows HeatCross's own description, not that of Falzar's Cross in its
    /// place (SpoutCross); without a list, the version's Crosses in order.
    #[test]
    fn r_in_the_cross_window_describes_the_cross_under_the_cursor() {
        for (list, down, key) in [(true, 0, "heatcross"), (true, 1, "groundcross"), (false, 0, "spoutcross"), (false, 1, "tomahawkcross")] {
            let list = list.then_some(&["heatcross", "groundcross"][..]);
            let (content, mut live, mut b) = cross_battle("falzar", list, |_, _| {});
            open_cross_window(&mut live, &mut b);
            for _ in 0..down {
                keys_in_turn(&mut live, &mut b, &[keys::DOWN, keys::DOWN, 0]);
            }
            keys_in_turn(&mut live, &mut b, &[keys::R]);
            let phase = b.custom.sides[0].screen.unwrap().phase;
            let Phase::Description { window: Some(_), form, chatbox } = phase else { panic!("{key}: no description: {phase:?}") };
            let want = form_of(&content, key);
            assert_eq!(form, Some(want), "{key}");
            let breaks = content.form(want).description_lines - 1;
            assert_eq!(chatbox.script(), custom::chatbox::Script::Description { breaks }, "{key}");
        }
    }

    /// In a Beast form a Cross list offers the Crosses whose Beast it is: in
    /// Falzar's Beast Falzar's, in a Gregar Cross's Beast form Gregar's;
    /// each takes the navi to its form in Beast Out. (The navi starts the
    /// battle in the Beast form: a spawned navi takes its starting form.)
    #[test]
    fn in_a_beast_form_a_cross_list_offers_that_beasts_crosses() {
        for (beast, place, result) in [("falzar-beast", 1, "groundcross-beast"), ("heatcross-beast", 0, "heatcross-beast")] {
            let (content, mut live, mut b) = cross_battle("falzar", Some(&["heatcross", "groundcross"]), |c, stats| {
                stats.form = form_of(c, beast);
                stats.starting_form = stats.form;
            });
            assert_eq!(b.stats[0].form, form_of(&content, beast));
            let w = b.form_list(0).unwrap();
            assert_eq!((w.count, w.offered[0]), (1, place), "{beast}");
            open_cross_window(&mut live, &mut b);
            choose_cross(&mut live, &mut b, 0);
            assert_eq!(confirm(&mut live, &mut b).transform.form, Some(form_of(&content, result)), "{beast}");
        }
    }

    /// A Cross list offers Crosses only, and leaves out the navi's starting
    /// form, as the original's window does.
    #[test]
    fn a_cross_list_offers_crosses_only() {
        let list = ["spoutcross", "gregar-beast", "eleccross", "tomahawkcross"];
        let (_, _, b) = cross_battle("gregar", Some(&list), |c, stats| stats.starting_form = form_of(c, "eleccross"));
        let w = b.form_list(0).unwrap();
        assert_eq!((w.count, &w.offered[..2]), (2, &[0, 3][..]));
    }

    /// B with nothing picked takes the Cross chosen back: its face and its
    /// form go, Beast Out is on offer again, and nothing goes out.
    #[test]
    fn b_takes_the_cross_back() {
        let (_, mut live, mut b) = cross_battle("falzar", None, |_, _| {});
        open_cross_window(&mut live, &mut b);
        choose_cross(&mut live, &mut b, 1);
        let screen = b.custom.sides[0].screen.unwrap();
        assert!(screen.form.is_some() && screen.look.face.is_some());
        assert_eq!(screen.slots[custom::screen::SPECIAL_SLOT as usize].state, SlotState::Unavailable);
        keys_in_turn(&mut live, &mut b, &[keys::B, 0]);
        let screen = b.custom.sides[0].screen.unwrap();
        assert_eq!((screen.form, screen.look.face), (None, None));
        assert_eq!(screen.slots[custom::screen::SPECIAL_SLOT as usize].state, SlotState::Selectable);
        assert_eq!(b.form_list(0).unwrap().chosen, None);
        assert_eq!(confirm(&mut live, &mut b).transform.form, None);
        assert_eq!(crosses_used(&b), [false; 5]);
    }

    /// The custom screen's time limit (nettai's own) runs out with the Cross
    /// window up: the window goes as if cancelled, the Cross under its
    /// cursor not chosen, and the screen closes as if OK were pressed, the
    /// chips picked so far the hand; a Cross chosen before stays chosen.
    #[test]
    fn time_up_in_the_cross_window_cancels_it_and_confirms() {
        const LIMIT: u16 = 240;
        let (content, mut live, mut b) = cross_battle_timed("falzar", Some(&["heatcross", "groundcross"]), |_, _| {}, Some(LIMIT));
        keys_in_turn(&mut live, &mut b, &[keys::A, 0]);
        open_cross_window(&mut live, &mut b);
        let up = |b: &Battle| matches!(b.custom.sides[0].screen.unwrap().phase, Phase::Window { .. });
        while b.custom.sides[0].sent.is_none() {
            assert!(up(&b) || b.custom.sides[0].clock == LIMIT, "the window stays up until the time is up");
            // (A held: the window's keys are ignored once the time is up.)
            let step = live.next(&b, if b.custom.sides[0].clock + 1 >= LIMIT { keys::A } else { 0 }).unwrap();
            b.tick(&step.input, step.events);
        }
        assert_eq!(b.custom.sides[0].clock, LIMIT);
        let sent = b.custom.sides[0].sent.as_ref().unwrap().result.clone();
        assert_eq!(sent.transform.form, None);
        assert_eq!(sent.hand.expect("the chip picked").ids[0], Some(content.defs.chip_by_key("cannon").unwrap()));
        assert_eq!(crosses_used(&b), [false; 5]);
        // A Cross chosen, then the time runs out: the Cross goes out.
        let (content, mut live, mut b) = cross_battle_timed("falzar", Some(&["heatcross", "groundcross"]), |_, _| {}, Some(LIMIT));
        open_cross_window(&mut live, &mut b);
        choose_cross(&mut live, &mut b, 0);
        play_until(&mut live, &mut b, LIMIT as u32 + 100, |_, _| 0, |b| b.custom.sides[0].sent.is_some());
        let sent = b.custom.sides[0].sent.as_ref().unwrap().result.clone();
        assert_eq!((sent.transform.form, sent.hand), (Some(form_of(&content, "heatcross")), None));
        assert_eq!(crosses_used(&b), [true, false, false, false, false]);
    }

    /// MegaMan received from a navi code (event flag 0x163, the setup's
    /// level) has no Beast Out button (`sub_8029FB4`), and his Cross window
    /// stays (`sub_8029F70`: with the flag, MegaMan's); without a code the
    /// button is there.
    #[test]
    fn a_navi_code_seals_beast_out() {
        for level in [None, Some(3)] {
            let content = nettai_match::testing::exe6_content();
            let stage = nettai_match::link_battle_stages(&content, "exe6")[0];
            let settings = BattleSettings { stage, background: Default::default(), effects: content.stage(stage).effects | nettai_match::MATCH_EFFECTS };
            let folder = folder_of(&content, &[("cannon", 0)]);
            let mut setup = live_setup(&content, settings, [folder, folder], "falzar", 5);
            let stated = level.map_or(nettai_content_api::Value::Nil, |l| nettai_content_api::Value::Int(l as i64));
            setup.players[0].set_fact(&content, "level", &[nettai_battle::rules::Fact::Value(stated)]).unwrap();
            let mut live = LivePlayer::new(Set::new(content.clone(), setup, [folder, folder]));
            let mut b = live.start();
            play_until(&mut live, &mut b, 3000, |_, _| 0, |b| choosing(b).is_some());
            let screen = b.custom.sides[0].screen.unwrap();
            let button = match screen.slots[custom::screen::SPECIAL_SLOT as usize].kind {
                SlotKind::Button { button, .. } => Some(b.content.defs.button(button).name.as_str()),
                _ => None,
            };
            assert_eq!(button, if level.is_none() { Some("beast_out") } else { None }, "level {level:?}");
            assert_eq!(b.form_list(0).unwrap().count, 5, "level {level:?}");
        }
    }
}
