//! A round as an episode (docs/design/training-ai.md §6): random builds on a
//! random arena of a random game, both seats played by a policy or a
//! scripted player, run until a policy seat is to decide.
//!
//! - **The fight.** A policy decides every `k` ticks of the env's count
//!   (ticks `t` with `t % k == 0` while the fight runs) from the observation
//!   of `t − D`, its reaction delay, drawn for each round and seat in
//!   `delay`. The observations a decision can need (ticks `s` with
//!   `(s + D) % k == 0`) are encoded as the round goes into a ring, so the
//!   delay costs one encoding every `k` ticks.
//! - **The custom screen.** A policy decides when its screen is choosing
//!   chips and its navigator is done with the last choice, from the screen
//!   as it is (nothing moves on it but its own cursor). At the screen's
//!   first decision, each key is pressed in a copy of the battle to see
//!   which ones open a window, and the one that opens the rules' form list
//!   is the navigator's way into it (the key probe, §4.2).
//! - **Who opens the custom screen** (§6.3): the env, for one seat or the
//!   other, a random while after the gauge fills (0 to 10 seconds, weighted
//!   toward soon), pressing L; no seat's own L or R reaches the fight.
//! - **Practice options** (§6.3), each on in a round with chance
//!   `practice`: the gauge full whenever it runs, the folders dealt in
//!   their own order. Both are the training session's (nettai-frontend's
//!   `Training`), done here on the battle's state as it does them.
//! - **The end.** A KO or the judge's ruling (+1, −1, 0 by seat), the
//!   fight's limit (`max_fight_ticks` of fight: no result, a stall), or a
//!   cut: a custom screen open `max_screen_ticks`, or the engine stopping
//!   the round. How it ended is the step's `ending` (§6.4: a stall costs).
//!   The next round starts at once.

use crate::act::{self, Navigator};
use crate::bots::{Bot, BotKind};
use crate::builds::Maker;
use crate::observe::{self, AttackHistory, Extra, Names, Obs};
use crate::spec;
use crate::stats::{Measure, RoundStats, navi_hp};
use nettai_battle::Battle;
use nettai_battle::battle::{fight, mode};
use nettai_battle::content::WindowHandle;
use nettai_battle::custom::FolderChip;
use nettai_battle::custom::screen::Phase;
use nettai_battle::hud::CustomGauge;
use nettai_battle::input::keys;
use nettai_match::{Picks, Set, Side};
use std::sync::Arc;

/// A game a pool plays: its content and what is made of it once.
pub struct Game {
    pub name: String,
    pub content: Arc<nettai_battle::Content>,
    pub names: Names,
    pub maker: Maker,
}

impl Game {
    /// A game with no fingerprints (its chips' are zeros in a view).
    pub fn new(content: Arc<nettai_battle::Content>) -> Result<Game, String> {
        let maker = Maker::new(&content)?;
        Ok(Game { name: maker.game().to_string(), names: Names::of(&content), maker, content })
    }

    /// A game with its chips' fingerprints (and their pairs, which random
    /// folders may be seeded with).
    pub fn with_fingerprints(content: Arc<nettai_battle::Content>, fingerprints: Arc<crate::fingerprint::Fingerprints>) -> Result<Game, String> {
        let maker = Maker::new(&content)?;
        Ok(Game { name: maker.game().to_string(), names: Names::with_fingerprints(&content, fingerprints), maker, content })
    }
}

/// What an env is set to.
#[derive(Clone, Debug)]
pub struct Config {
    /// Ticks a fight decision is held (a seat's own where it is pinned,
    /// [`Env::set_k`]).
    pub k: u32,
    /// The reaction delay's range, in ticks, drawn each round for each seat.
    pub delay: (u16, u16),
    pub max_fight_ticks: u32,
    /// A custom screen open this long cuts the round off.
    pub max_screen_ticks: u32,
    /// The base HP's range, drawn each round, both sides the same (where
    /// the navi takes one); none: each side's as its build has it.
    pub hp: Option<(u16, u16)>,
    /// Chips held out of training (§6.6's `chip_holdout`).
    pub holdout: crate::builds::Holdout,
    /// The chance each practice option is on in a round.
    pub practice: f64,
    /// Decisions a policy makes on one custom screen before only OK is
    /// allowed.
    pub screen_decisions: u16,
    /// Who plays each seat from the first round.
    pub seats: [Controller; 2],
    /// The chance a random build draws every fact it states (its NaviCust,
    /// its patch cards...), else it keeps the rules' defaults for them
    /// (§6.2).
    pub randomize: f64,
    /// The chance a random folder is seeded with a pair of chips that work
    /// together (a setup and its payoff, from the fingerprints, §6.2).
    pub combos: f64,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            k: 4,
            delay: (6, 30),
            max_fight_ticks: 18000,
            max_screen_ticks: 3600,
            hp: None,
            holdout: Default::default(),
            practice: 0.25,
            screen_decisions: 24,
            seats: [Controller::Policy; 2],
            randomize: 0.75,
            combos: 0.3,
        }
    }
}

/// Who plays a seat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Controller {
    Policy,
    Bot(BotKind),
}

impl Controller {
    pub fn name(self) -> &'static str {
        match self {
            Controller::Policy => "policy",
            Controller::Bot(k) => k.name(),
        }
    }

    pub fn named(name: &str) -> Option<Controller> {
        if name == "policy" { Some(Controller::Policy) } else { BotKind::named(name).map(Controller::Bot) }
    }
}

/// What a step asks of a seat: no decision, a fight decision, a custom
/// screen decision (§6.6's `phase`).
pub const NONE: u8 = 0;
pub const FIGHT: u8 = 1;
pub const SCREEN: u8 = 2;

/// How a round ended (§6.4, the pool's `ending`): a KO (or the rules' own
/// end), the judge's ruling on damage once the final turn's time ran out,
/// or the fight's limit (`max_fight_ticks`) with no result.
pub const BY_KO: u8 = 0;
pub const BY_JUDGE: u8 = 1;
pub const BY_LIMIT: u8 = 2;

/// What a step ended on.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Outcome {
    pub phase: [u8; 2],
    pub done: bool,
    pub truncated: bool,
    pub result: [i8; 2],
    /// How the round ended, where it did and wasn't cut ([`BY_KO`],
    /// [`BY_JUDGE`], [`BY_LIMIT`]).
    pub ending: u8,
    /// (HP the other lost / its max HP − HP it lost / its own) since the
    /// seat's last decision: valid where the seat decides or the round
    /// ended.
    pub hp_gain: [f32; 2],
}

struct Seat {
    controller: Controller,
    next: Controller,
    pinned_delay: Option<u16>,
    delay: u16,
    /// Ticks its fight decisions are held this round, and the same from the
    /// next where it is pinned (none: the env's k).
    pinned_k: Option<u32>,
    k: u32,
    action: i32,
    nav: Navigator,
    probe: Option<[bool; act::KEYS.len()]>,
    decisions: u16,
    decided_at: Option<u32>,
    gain: f32,
    /// Observations a fight decision may need, by `(s + D) / k` modulo its
    /// length: their ticks and them.
    ring: Vec<(Option<u32>, Obs)>,
    bot: Bot,
}

/// Where an env's time went, in nanoseconds, and what it did: for the
/// throughput measures (§6.5).
#[derive(Clone, Copy, Debug, Default)]
pub struct Profile {
    /// Encoding observations (the ring's and the screen's).
    pub encode: u64,
    /// Starting rounds: builds made then (none made ahead), the round set up.
    pub reset: u64,
    /// The key probes.
    pub probe: u64,
    /// Builds made ahead (`Env::prepare`).
    pub prepare: u64,
    pub ticks: u64,
    pub decisions: u64,
    pub rounds: u64,
}

/// One env: a round being played, and how it goes on.
pub struct Env {
    games: Arc<Vec<Game>>,
    cfg: Config,
    picks: Picks,
    game: usize,
    battle: Battle,
    folders: Vec<nettai_battle::custom::SavedFolder>,
    t: u32,
    screen_ticks: u32,
    in_screen: bool,
    seats: [Seat; 2],
    asked: [u8; 2],
    log: AttackHistory,
    opener: Option<(usize, u32)>,
    full_gauge: bool,
    measure: Measure,
    hp: [Option<u16>; 2],
    max_hp: [u16; 2],
    /// Builds made ahead, by game, the oldest first, and each game's own
    /// stream of draws for its builds: a round's builds are the same
    /// whenever they were made.
    ready: Vec<std::collections::VecDeque<crate::builds::Build>>,
    build_picks: Vec<Picks>,
    /// Rounds ended since they were last taken.
    pub ended: Vec<RoundStats>,
    pub profile: Profile,
    /// The observations and the actions allowed of the seats asked.
    pub obs: [Obs; 2],
    pub legal_fight: [[bool; spec::FIGHT_ACTIONS]; 2],
    pub legal_screen: [[bool; spec::SCREEN_ACTIONS]; 2],
}

/// How many builds of each game an env keeps made ahead.
pub const READY: usize = 2;

thread_local! {
    /// A tick runs ([`step_battle`]): a panic now is the engine's stop.
    static IN_ENGINE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Keep the engine's stops off stderr (a round reports its own:
/// `RoundStats::error`); other panics print as usual. Once for the
/// process (a pool does it).
pub fn quiet_engine_panics() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let default = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if !IN_ENGINE.with(|f| f.get()) {
                default(info);
            }
        }));
    });
}

/// One tick of `b` on the players' buttons (by side), as a session runs
/// one: a tick the engine stops on (a panic: a content error, a state the
/// original can't go on from) stops the battle (`Battle::fail`). Whether it
/// went on.
pub fn step_battle(b: &mut Battle, held: [u16; 2]) -> bool {
    let input = nettai_netplay::standin::tick_input(b, held.map(|h| h as u32));
    IN_ENGINE.with(|f| f.set(true));
    let stepped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.step(&input)));
    IN_ENGINE.with(|f| f.set(false));
    match stepped {
        Ok(()) => !b.is_stopped(),
        Err(e) => {
            let why = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_else(|| "a panic".into());
            b.fail(why);
            false
        }
    }
}

/// Whether the fight runs in `b` (decisions are taken in it, dimmed too).
pub fn fight_runs(b: &Battle) -> bool {
    b.round.mode == mode::FIGHTING && b.fight.state == fight::FIGHTING && !b.is_battle_over()
}

/// The window of the rules each key opens on side `side`'s custom screen
/// in `b`, if any (§4.2): each pressed once in a copy of the battle, the
/// other side pressing nothing; only that side's own screen is read of the
/// copy.
pub fn probe_keys(b: &Battle, side: u8) -> [Option<WindowHandle>; act::KEYS.len()] {
    act::KEYS.map(|key| {
        let mut copy = b.clone();
        let direction = key & (keys::UP | keys::DOWN | keys::LEFT | keys::RIGHT) != 0;
        let presses: &[u16] = if direction { &[key, key, 0, 0] } else { &[key, 0, 0] };
        for &held in presses {
            let mut both = [0; 2];
            both[side as usize] = held;
            if !step_battle(&mut copy, both) {
                return None;
            }
            if let Some(Phase::Window { window, .. }) = copy.custom.sides[side as usize].screen.map(|s| s.phase) {
                return Some(window);
            }
        }
        None
    })
}

/// A seat's key probe on its screen in `b` (§4.2), at its first decision
/// there: the flags its view holds, and its navigator's way into the form
/// list.
pub fn probe(b: &Battle, side: u8, nav: &mut Navigator) -> [bool; act::KEYS.len()] {
    let opened = probe_keys(b, side);
    let cursor = b.custom.sides[side as usize].screen.map_or(0, |s| s.cursor);
    nav.opener = act::opener(b, cursor, &opened);
    opened.map(|w| w.is_some())
}

impl Env {
    /// An env playing `games`, its draws from `seed`. Its first round
    /// starts at once; [`Env::step`] runs it to its first decision.
    pub fn new(games: Arc<Vec<Game>>, cfg: Config, seed: u64) -> Result<Env, String> {
        if games.is_empty() {
            return Err("an env plays at least one game".into());
        }
        let ring = (cfg.delay.1 as u32 / cfg.k.max(1)) as usize + 2;
        let seat = |n: u64, c: Controller| Seat {
            controller: c,
            next: c,
            pinned_delay: None,
            delay: cfg.delay.0,
            pinned_k: None,
            k: cfg.k.max(1),
            action: 0,
            nav: Navigator::default(),
            probe: None,
            decisions: 0,
            decided_at: None,
            gain: 0.0,
            ring: (0..ring).map(|_| (None, Obs::new())).collect(),
            bot: Bot::new(BotKind::Still, n),
        };
        let mut picks = Picks::new((seed ^ (seed >> 32)) as u32);
        // (The draws of one seed differ from another's from the first.)
        for _ in 0..(seed % 7) {
            picks.next();
        }
        let mut build_picks: Vec<Picks> = (0..games.len()).map(|g| Picks::new(picks.next() as u32 ^ g as u32)).collect();
        let mut ready: Vec<std::collections::VecDeque<crate::builds::Build>> = games.iter().map(|_| Default::default()).collect();
        let g = picks.below(games.len());
        let first = RoundMade::new(&games[g], &mut picks, &mut build_picks[g], &mut ready[g], &cfg)?;
        let mut env = Env {
            ready,
            build_picks,
            picks,
            game: g,
            battle: first.battle,
            folders: first.folders,
            t: 0,
            screen_ticks: 0,
            in_screen: false,
            seats: [seat(seed, cfg.seats[0]), seat(seed ^ 1, cfg.seats[1])],
            asked: [NONE; 2],
            log: AttackHistory::default(),
            opener: None,
            full_gauge: false,
            measure: Measure::default(),
            hp: [None; 2],
            max_hp: [0; 2],
            ended: Vec::new(),
            profile: Profile::default(),
            obs: [Obs::new(), Obs::new()],
            legal_fight: [[false; spec::FIGHT_ACTIONS]; 2],
            legal_screen: [[false; spec::SCREEN_ACTIONS]; 2],
            games,
            cfg,
        };
        env.start_round(Some((first.m, first.pairs)))?;
        Ok(env)
    }

    pub fn battle(&self) -> &Battle {
        &self.battle
    }

    pub fn game(&self) -> usize {
        self.game
    }

    pub fn delay(&self) -> [u16; 2] {
        [self.seats[0].delay, self.seats[1].delay]
    }

    /// Who plays seat `seat` from the next round.
    pub fn set_seat(&mut self, seat: usize, c: Controller) {
        self.seats[seat].next = c;
    }

    /// The delay seat `seat` plays at from the next round (none: drawn).
    pub fn set_delay(&mut self, seat: usize, ticks: Option<u16>) {
        self.seats[seat].pinned_delay = ticks;
    }

    /// The ticks seat `seat` holds a fight decision from the next round
    /// (none: the env's k), so that networks trained at different k meet
    /// each at its own.
    pub fn set_k(&mut self, seat: usize, k: Option<u32>) {
        self.seats[seat].pinned_k = k.map(|k| k.max(1));
    }

    pub fn set_hp(&mut self, hp: Option<(u16, u16)>) {
        self.cfg.hp = hp.map(|(lo, hi)| (lo, hi.max(lo)));
    }

    /// The chips held out, from the next round.
    pub fn set_chip_holdout(&mut self, holdout: crate::builds::Holdout) {
        self.cfg.holdout = holdout;
        // (Builds made ahead were made by the last.)
        for r in &mut self.ready {
            r.clear();
        }
    }

    /// Make a build ahead, for the game with the fewest made, if one has
    /// fewer than [`READY`]: what a worker does while it waits. Whether it
    /// made one.
    pub fn prepare(&mut self) -> Result<bool, String> {
        let Some(g) = (0..self.games.len()).filter(|&g| self.ready[g].len() < READY).min_by_key(|&g| self.ready[g].len()) else {
            return Ok(false);
        };
        let started = std::time::Instant::now();
        let build = self.games[g].maker.random(&mut self.build_picks[g], &self.cfg.holdout, self.cfg.randomize, combos(&self.games[g], &self.cfg))?;
        self.ready[g].push_back(build);
        self.profile.prepare += started.elapsed().as_nanos() as u64;
        Ok(true)
    }

    fn draw(&mut self, lo: u16, hi: u16) -> u16 {
        draw(&mut self.picks, lo, hi)
    }

    fn chance(&mut self, p: f64) -> bool {
        (self.picks.next() >> 11) as f64 / (1u64 << 53) as f64 <= p && p > 0.0
    }

    /// A new round: a game, two builds, the HP, an arena and a seed (`made`:
    /// the first round's, made with the env), the seats' players and
    /// delays, the practice options.
    fn start_round(&mut self, made: Option<(nettai_match::Match, [Option<crate::fingerprint::Pair>; 2])>) -> Result<(), String> {
        let started = std::time::Instant::now();
        self.profile.rounds += 1;
        let (m, pairs) = match made {
            Some(m) => m,
            None => {
                let g = self.picks.below(self.games.len());
                let r = RoundMade::new(&self.games[g], &mut self.picks, &mut self.build_picks[g], &mut self.ready[g], &self.cfg)?;
                self.game = g;
                self.battle = r.battle;
                self.folders = r.folders;
                (r.m, r.pairs)
            }
        };
        let g = self.game;
        for s in 0..2 {
            let delay = match self.seats[s].pinned_delay {
                Some(d) => d,
                None => self.draw(self.cfg.delay.0, self.cfg.delay.1),
            };
            let seed = self.picks.next();
            let k = self.seats[s].pinned_k.unwrap_or(self.cfg.k.max(1));
            let seat = &mut self.seats[s];
            seat.controller = seat.next;
            seat.delay = delay;
            seat.k = k;
            // Room for the views of the decisions a delay spans (a pinned
            // delay may pass the drawn range's).
            let len = (delay.max(self.cfg.delay.1) as u32 / k) as usize + 2;
            if seat.ring.len() < len {
                seat.ring.resize_with(len, || (None, Obs::new()));
            }
            seat.action = 0;
            seat.nav.reset();
            seat.probe = None;
            seat.decisions = 0;
            seat.decided_at = None;
            seat.gain = 0.0;
            for slot in &mut seat.ring {
                slot.0 = None;
            }
            seat.bot = Bot::new(match seat.controller {
                Controller::Bot(k) => k,
                Controller::Policy => BotKind::Still,
            }, seed);
        }
        self.full_gauge = self.chance(self.cfg.practice);
        let folder_order = self.chance(self.cfg.practice);
        if folder_order {
            for side in 0..2 {
                folder_order_deal(&mut self.battle, side, &self.folders[side]);
            }
        }
        self.t = 0;
        self.screen_ticks = 0;
        self.in_screen = false;
        self.log = AttackHistory::default();
        self.opener = None;
        self.hp = [None; 2];
        self.max_hp = [0; 2];
        let content = &self.games[g].content;
        let navi = |s: &Side| content.defs.navi(s.navi(content)).key.clone();
        self.measure = Measure::new(RoundStats {
            game: self.games[g].name.clone(),
            controllers: [self.seats[0].controller.name().to_string(), self.seats[1].controller.name().to_string()],
            navis: [navi(&m.sides[0]), navi(&m.sides[1])],
            delay: [self.seats[0].delay, self.seats[1].delay],
            full_gauge: self.full_gauge,
            folder_order,
            ..RoundStats::default()
        });
        // (The pairs the folders were seeded with, watched.)
        let fps = self.games[g].names.fingerprints();
        for (s, p) in pairs.iter().enumerate() {
            if let Some(p) = p {
                let plain = fps.and_then(|f| f.of(p.payoff)).and_then(|f| f.best.map(|b| f.runs[b].0)).unwrap_or(0);
                self.measure.watch_pair(s, *p, plain, content);
            }
        }
        self.profile.reset += started.elapsed().as_nanos() as u64;
        Ok(())
    }

    /// Why the round is over, if it is: its result by seat, whether it was
    /// cut off, how it ended, and the engine's error.
    fn over(&self) -> Option<([i8; 2], bool, u8, Option<String>)> {
        let b = &self.battle;
        if let Some(nettai_battle::RoundEnd::Error(why)) = b.round_end() {
            return Some(([0; 2], true, BY_KO, Some(why.clone())));
        }
        if b.round.mode == mode::FIGHTING && matches!(b.fight.state, fight::WIN | fight::LOSE | fight::DRAW) {
            // (The fight's result is the simulation's own side's.)
            let local = b.round.local_side as usize & 1;
            let mut result = [0i8; 2];
            match b.fight.state {
                fight::WIN => (result[local], result[local ^ 1]) = (1, -1),
                fight::LOSE => (result[local], result[local ^ 1]) = (-1, 1),
                _ => {}
            }
            return Some((result, false, if b.round.time_up != 0 { BY_JUDGE } else { BY_KO }, None));
        }
        if self.measure.stats.fight_ticks > self.cfg.max_fight_ticks {
            return Some(([0; 2], false, BY_LIMIT, None));
        }
        let cut = self.screen_ticks > self.cfg.max_screen_ticks || self.t > 4 * self.cfg.max_fight_ticks + 4 * self.cfg.max_screen_ticks;
        cut.then_some(([0; 2], true, BY_KO, None))
    }

    fn has_policy(&self) -> bool {
        self.seats.iter().any(|s| s.controller == Controller::Policy)
    }

    /// Apply `actions` to the seats the last step asked (a fight action, or
    /// a screen action), then play until a policy seat is to decide, a
    /// round ending on the way (the next one starts at once; with no policy
    /// seat, the step ends with the round). The asked seats' observations
    /// and allowed actions are in [`Env::obs`] and the legal arrays.
    pub fn step(&mut self, actions: [i32; 2]) -> Result<Outcome, String> {
        for s in 0..2 {
            let a = actions[s];
            let allowed = match self.asked[s] {
                FIGHT => usize::try_from(a).ok().and_then(|a| self.legal_fight[s].get(a).copied()),
                SCREEN => usize::try_from(a).ok().and_then(|a| self.legal_screen[s].get(a).copied()),
                _ => Some(true),
            };
            if allowed != Some(true) {
                return Err(format!("seat {s}: action {a} isn't allowed at this decision (phase {})", self.asked[s]));
            }
        }
        for s in 0..2 {
            match self.asked[s] {
                FIGHT => self.seats[s].action = actions[s],
                SCREEN => {
                    self.seats[s].nav.point(actions[s]);
                    self.seats[s].decisions += 1;
                }
                _ => {}
            }
        }
        self.asked = [NONE; 2];
        let mut out = Outcome::default();
        loop {
            if let Some((result, truncated, ending, error)) = self.over() {
                let mut stats = std::mem::take(&mut self.measure).stats;
                stats.result = result;
                stats.truncated = truncated;
                stats.ending = ending;
                stats.error = error;
                self.ended.push(stats);
                out.done = true;
                out.truncated = truncated;
                out.result = result;
                out.ending = ending;
                for s in 0..2 {
                    out.hp_gain[s] += std::mem::take(&mut self.seats[s].gain);
                }
                self.start_round(None)?;
                if !self.has_policy() {
                    return Ok(out);
                }
                continue;
            }
            let asked = self.asks();
            if asked != [NONE; 2] {
                for s in 0..2 {
                    if asked[s] == NONE {
                        continue;
                    }
                    self.ask(s, asked[s]);
                    out.phase[s] = asked[s];
                    // (After a round's end in this step, the step's gain is the
                    // ended round's alone: the new round's before its first
                    // decision is no transition's.)
                    let gain = std::mem::take(&mut self.seats[s].gain);
                    if !out.done {
                        out.hp_gain[s] += gain;
                    }
                }
                self.asked = asked;
                return Ok(out);
            }
            self.tick();
        }
    }

    /// The seats a policy decides for now.
    fn asks(&self) -> [u8; 2] {
        let b = &self.battle;
        let mut out = [NONE; 2];
        for s in 0..2 {
            let seat = &self.seats[s];
            if seat.controller != Controller::Policy || seat.decided_at == Some(self.t) {
                continue;
            }
            if fight_runs(b) && self.t.is_multiple_of(seat.k) && b.player(s as u8).is_some() {
                out[s] = FIGHT;
            } else if b.round.mode == mode::CUSTOM
                && seat.nav.idle()
                && b.custom.sides[s].screen.is_some_and(|screen| act::chooses(&screen))
            {
                out[s] = SCREEN;
            }
        }
        out
    }

    fn extra(&self, s: usize) -> Extra {
        let seat = &self.seats[s];
        Extra {
            delay: seat.delay,
            log: self.log,
            probe: seat.probe,
            refused: seat.nav.refused,
            screen_decisions: seat.decisions,
            only_ok: seat.decisions >= self.cfg.screen_decisions,
        }
    }

    /// Ask seat `s` for a decision of kind `phase`: its observation and
    /// what it may do.
    fn ask(&mut self, s: usize, phase: u8) {
        self.seats[s].decided_at = Some(self.t);
        self.profile.decisions += 1;
        let names = &self.games[self.game].names;
        match phase {
            FIGHT => {
                let seat = &self.seats[s];
                let k = seat.k;
                let want = self.t.checked_sub(seat.delay as u32);
                let slot = &seat.ring[((self.t / k) as usize) % seat.ring.len()];
                let started = std::time::Instant::now();
                if want.is_some() && slot.0 == want {
                    self.obs[s].clone_from(&slot.1);
                } else {
                    // (A decision earlier than the delay: what it sees now.)
                    let extra = self.extra(s);
                    observe::encode(&self.battle, s as u8, names, &extra, &mut self.obs[s]);
                }
                self.profile.encode += started.elapsed().as_nanos() as u64;
                self.legal_fight[s] = [true; spec::FIGHT_ACTIONS];
                self.legal_screen[s] = [false; spec::SCREEN_ACTIONS];
            }
            _ => {
                if self.seats[s].probe.is_none() {
                    let started = std::time::Instant::now();
                    let seat = &mut self.seats[s];
                    seat.probe = Some(probe(&self.battle, s as u8, &mut seat.nav));
                    self.profile.probe += started.elapsed().as_nanos() as u64;
                }
                let extra = self.extra(s);
                let started = std::time::Instant::now();
                observe::encode(&self.battle, s as u8, names, &extra, &mut self.obs[s]);
                self.profile.encode += started.elapsed().as_nanos() as u64;
                self.legal_screen[s] = act::legal_screen(&self.battle, s as u8, &self.seats[s].nav, extra.only_ok);
                self.legal_fight[s] = [false; spec::FIGHT_ACTIONS];
            }
        }
    }

    /// One tick of the round.
    fn tick(&mut self) {
        let custom = self.battle.round.mode == mode::CUSTOM;
        if custom && !self.in_screen {
            // A new custom screen.
            for seat in &mut self.seats {
                seat.nav.reset();
                seat.probe = None;
                seat.decisions = 0;
            }
            self.screen_ticks = 0;
            self.opener = None;
        }
        self.in_screen = custom;
        self.screen_ticks += custom as u32;
        let runs = fight_runs(&self.battle);
        let mut held = [0u16; 2];
        for s in 0..2 {
            let b = &self.battle;
            let seat = &mut self.seats[s];
            held[s] = if custom {
                if seat.controller != Controller::Policy && seat.nav.idle() {
                    let p = act::scripted_pointer(b, s as u8, &seat.nav);
                    seat.nav.point(p);
                }
                seat.nav.buttons(b, s as u8)
            } else if runs {
                match seat.controller {
                    Controller::Policy => act::fight_buttons(seat.action),
                    Controller::Bot(_) => seat.bot.fight(b, s as u8),
                }
            } else {
                0
            };
        }
        if let Some(s) = self.opens() {
            held[s] |= keys::L;
        }
        if self.full_gauge && self.battle.gauge.enabled && self.battle.gauge.value < CustomGauge::FULL {
            self.battle.gauge.value = CustomGauge::FULL;
        }
        step_battle(&mut self.battle, held);
        self.t += 1;
        self.profile.ticks += 1;
        self.log.push(&self.battle);
        self.measure.tick(&self.battle, self.t);
        self.gains();
        // The observations later decisions will need.
        let names = &self.games[self.game].names;
        for s in 0..2 {
            let seat = &self.seats[s];
            let d = seat.delay as u32;
            if seat.controller != Controller::Policy || !(self.t + d).is_multiple_of(seat.k) {
                continue;
            }
            let extra = self.extra(s);
            let at = (((self.t + d) / seat.k) as usize) % seat.ring.len();
            let started = std::time::Instant::now();
            let slot = &mut self.seats[s].ring[at];
            slot.0 = Some(self.t);
            observe::encode(&self.battle, s as u8, names, &extra, &mut slot.1);
            self.profile.encode += started.elapsed().as_nanos() as u64;
        }
    }

    /// The seat the env presses L for this tick, to open the custom screen
    /// (§6.3): a random while after the gauge is full, for a random seat.
    fn opens(&mut self) -> Option<usize> {
        let b = &self.battle;
        if !fight_runs(b) || b.is_dimmed() || b.gauge.value < CustomGauge::FULL {
            return None;
        }
        let (seat, at) = match self.opener {
            Some(o) => o,
            None => {
                let u = (self.picks.next() >> 11) as f64 / (1u64 << 53) as f64;
                let wait = (600.0 * u * u) as u32;
                let seat = self.picks.below(2);
                self.opener = Some((seat, self.t + wait));
                (seat, self.t + wait)
            }
        };
        // (A press and a let-go, until the screen opens.)
        (self.t >= at && (self.t - at).is_multiple_of(2)).then_some(seat)
    }

    /// The tick's HP lost by each side, into each seat's gain (§6.4), each
    /// side's loss by its own max HP.
    fn gains(&mut self) {
        let b = &self.battle;
        let mut lost = [0.0f32; 2];
        for side in 0..2u8 {
            let i = side as usize;
            let now = navi_hp(b, side, self.hp[i]);
            if let Some(r) = b.player(side).filter(|&r| b.objects.is_allocated(r)) {
                self.max_hp[i] = self.max_hp[i].max(b.objects.get(r).max_hp);
            }
            if let (Some(was), Some(now)) = (self.hp[i], now) {
                lost[i] = (was as f32 - now as f32) / self.max_hp[i].max(1) as f32;
            }
            self.hp[i] = now;
        }
        self.seats[0].gain += lost[1] - lost[0];
        self.seats[1].gain += lost[0] - lost[1];
    }
}

#[cfg(test)]
impl Env {
    /// Play up to `ticks` ticks of the round as its seats play it (a test
    /// takes the battle as it stands after).
    pub(crate) fn run(&mut self, ticks: u32) {
        for _ in 0..ticks {
            if self.over().is_some() {
                return;
            }
            self.tick();
        }
    }

    /// What seat `s`'s view takes besides the battle, now.
    pub(crate) fn extra_now(&self, s: usize) -> Extra {
        self.extra(s)
    }

    pub(crate) fn names(&self) -> &Names {
        &self.games[self.game].names
    }
}

/// A number from `lo` to `hi`.
fn draw(picks: &mut Picks, lo: u16, hi: u16) -> u16 {
    lo + picks.below((hi.max(lo) - lo) as usize + 1) as u16
}

/// A round of a game made: two builds (made ahead, `ready`, or now), the
/// HP from `hp`, an arena and a seed; its battle at its start, and the
/// players' folders.
struct RoundMade {
    battle: Battle,
    folders: Vec<nettai_battle::custom::SavedFolder>,
    m: nettai_match::Match,
    pairs: [Option<crate::fingerprint::Pair>; 2],
}

/// The pairs a game's random folders may be seeded with, at `cfg`'s rate.
fn combos<'a>(game: &'a Game, cfg: &Config) -> crate::builds::Combos<'a> {
    match game.names.fingerprints() {
        Some(f) => crate::builds::Combos { pairs: &f.pairs, rate: cfg.combos },
        None => crate::builds::Combos::NONE,
    }
}

impl RoundMade {
    fn new(
        game: &Game,
        picks: &mut Picks,
        stream: &mut Picks,
        ready: &mut std::collections::VecDeque<crate::builds::Build>,
        cfg: &Config,
    ) -> Result<RoundMade, String> {
        let mut build = || match ready.pop_front() {
            Some(b) => Ok(b),
            None => game.maker.random(stream, &cfg.holdout, cfg.randomize, combos(game, cfg)),
        };
        let builds = [build()?, build()?];
        let pairs = [builds[0].pair, builds[1].pair];
        let sides = builds.map(|b| b.side);
        let hp = cfg.hp.map(|(lo, hi)| draw(picks, lo, hi));
        let seed = picks.next() as u32;
        let m = game.maker.round_with(sides, hp, seed)?;
        let set = Set::of(&game.content, &m, seed);
        Ok(RoundMade { battle: set.start(), folders: set.folders().to_vec(), m, pairs })
    }
}

/// Deal side `side`'s battle folder in its own order (the training
/// session's chip draw, nettai-frontend's `ChipDraw::FolderOrder`, at a
/// round's start): the saved folder's order, no Regular chip first, no tag
/// pair.
fn folder_order_deal(b: &mut Battle, side: usize, saved: &nettai_battle::custom::SavedFolder) {
    let folder = &mut b.custom.sides[side].folder;
    let mut left: Vec<FolderChip> = folder.chips.iter().flatten().copied().collect();
    let mut placed: Vec<FolderChip> = Vec::with_capacity(left.len());
    for chip in saved.chips {
        if let Some(at) = left.iter().position(|c| *c == chip) {
            placed.push(left.remove(at));
        }
    }
    placed.append(&mut left);
    folder.chips = std::array::from_fn(|i| placed.get(i).copied());
    folder.regular_pending = false;
    b.consoles[side].tag_pair = None;
}

#[cfg(test)]
pub(crate) mod tests;
