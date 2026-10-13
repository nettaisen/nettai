//! An opponent that fights: it tries plans in copies of the battle and
//! follows the one that comes out best. Live play's [`Ai`] (its
//! `InFight::Ai` and `OnCustomScreen::Ai`).
//!
//! It knows no game. A plan is buttons: a step or two, then a chip, a shot
//! or nothing, with B held all the while to charge the buster, as a player
//! does. What a plan is worth is what the battle's own simulation makes of
//! it a second or so on (the HP each navi has then), against a few guesses
//! at the other player: they go on as they are, step up or down, or attack
//! at once. A plan counts what it takes away on average over the guesses,
//! less the most it loses in any, so it keeps out of the way of an attack
//! that may come. Every chip of every game's content is played as the
//! content plays it, with nothing about any of them here. It presses only
//! buttons and asks the engine for nothing else; a battle is plain data, so
//! a copy to try a plan in is a clone (as a rollback's snapshot is).
//!
//! What makes it a partner to train against rather than one that sees the
//! future is its [`Level`]: it acts on what it decided a reaction time ago,
//! it now and then picks a plan at random, and it looks only so far ahead.
//! The reaction time is also when it thinks: a decision is worked out on a
//! thread of its own while the battle goes on, and taken on the tick it was
//! due (the battle waits for it if it isn't ready). Its picks are seeded and
//! made on the battle's side: the same battle, seed and level give the same
//! presses, on its thread or not.

use std::time::{Duration, Instant};

use nettai_battle::Battle;
use nettai_battle::battle::mode;
use nettai_battle::custom::screen::{MAX_SELECTIONS, OK_SLOT, Screen};
use nettai_battle::custom::{Phase, SlotKind, SlotState};
use nettai_battle::input::keys;
use nettai_content_api::PanelPos;
use nettai_frontend::driver::Ai;
use nettai_netplay::rng::SplitMix64;
use nettai_netplay::standin::tick_input;

/// How well an opponent that fights plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Level {
    /// Ticks from a decision to its first press: how late it reacts to
    /// what it sees.
    pub reaction: u32,
    /// Ticks of the fight each plan is tried for.
    pub horizon: u32,
    /// Ticks of the fight between decisions.
    pub every: u32,
    /// In a hundred decisions, how many pick any plan at random instead of
    /// the best.
    pub blunders: u32,
    /// It holds B whatever else it does, as a player charging the buster
    /// does: a shot lets go (the charged shot, once the charge is full) and
    /// presses it again.
    pub charges: bool,
}

impl Level {
    pub const EASY: Level = Level { reaction: 24, horizon: 40, every: 16, blunders: 35, charges: true };
    pub const NORMAL: Level = Level { reaction: 14, horizon: 50, every: 12, blunders: 15, charges: true };
    pub const HARD: Level = Level { reaction: 6, horizon: 60, every: 12, blunders: 0, charges: true };
}

impl Default for Level {
    fn default() -> Level {
        Level::NORMAL
    }
}

/// What a plan does once its steps are taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Then {
    Wait,
    /// A: the chip in hand.
    Chip,
    /// B: the buster.
    Shot,
}

/// A plan: up to two steps (direction keys, 0 for none), then what it does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Plan {
    steps: [u16; 2],
    then: Then,
}

/// The ticks a step's direction is held for at most, where the navi
/// doesn't move (a panel it can't step on, a hit taking it).
const STEP_MOST: u8 = 20;
/// With nothing in hand, how often it sees whether L opens the custom
/// screen, and the ticks it gives the screen to open.
const OPEN_EVERY: u32 = 8;
const OPEN_TICKS: u32 = 12;

/// In a plan's trial, once it's done, it shoots every so many ticks: what
/// a place is worth is also what the buster does from there.
const TRIAL_SHOTS: u16 = 8;
/// The ticks it supposes the player goes on holding what they hold now.
const PLAYER_HOLDS: u32 = 8;
/// What a plan is worth, in tenths of HP: an HP it takes away, and one it
/// loses (losing counts a little more: it would rather not trade).
const DEALT: i64 = 10;
const TAKEN: i64 = 12;
/// A navi deleted, worth more than any HP.
const DELETED: i64 = 1_000_000;

/// A plan being followed: where it is in it.
#[derive(Clone, Copy, Debug)]
struct Exec {
    plan: Plan,
    /// The step being taken (2: past them), the ticks its direction has been
    /// held and the panel it started from.
    step: u8,
    held: u8,
    from: Option<PanelPos>,
    /// Whether it's done, and the ticks since.
    done: bool,
    after: u16,
    /// It's being tried: once done, it shoots ([`TRIAL_SHOTS`]).
    trial: bool,
    /// It holds B ([`Level::charges`]).
    charges: bool,
    /// What it pressed the tick before (a press is a button not held then).
    last: u16,
}

impl Exec {
    fn new(plan: Plan, charges: bool) -> Exec {
        Exec { plan, step: 0, held: 0, from: None, done: false, after: 0, trial: false, charges, last: 0 }
    }

    /// Its buttons this tick, for the navi of `side` in `b`.
    fn buttons(&mut self, b: &Battle, side: u8) -> u16 {
        let pressed = self.next(b, side);
        self.last = pressed;
        pressed
    }

    fn next(&mut self, b: &Battle, side: u8) -> u16 {
        let held = if self.charges { keys::B } else { 0 };
        match self.part(b, side) {
            // (A shot: B let go a tick, then pressed.)
            Some(keys::B) if self.charges && self.last & keys::B != 0 => 0,
            Some(keys::B) => keys::B,
            Some(pressed) => pressed | held,
            None => held,
        }
    }

    /// What its plan presses this tick besides B held (none: nothing).
    fn part(&mut self, b: &Battle, side: u8) -> Option<u16> {
        let Some(r) = b.player(side) else { return None };
        let at = b.objects.get(r).panel;
        while (self.step as usize) < self.plan.steps.len() {
            let dir = self.plan.steps[self.step as usize];
            let from = *self.from.get_or_insert(at);
            if dir == 0 || at != from || self.held >= STEP_MOST {
                self.step += 1;
                self.held = 0;
                self.from = None;
                if dir != 0 {
                    // (Let go a tick: the next step's direction, or the button after, is a press.)
                    return None;
                }
                continue;
            }
            self.held += 1;
            return Some(dir);
        }
        if self.done {
            self.after += 1;
            return (self.trial && self.after % TRIAL_SHOTS == 0).then_some(keys::B);
        }
        match self.plan.then {
            Then::Wait => {
                self.done = true;
                None
            }
            // (A press: let go first if it's held.)
            Then::Chip if self.last & keys::A != 0 => None,
            Then::Chip => {
                self.done = true;
                Some(keys::A)
            }
            // (A press: let go first if it's held; held to charge, letting go is the charged shot's.)
            Then::Shot if self.last & keys::B != 0 => self.charges.then_some(keys::B),
            Then::Shot => {
                self.done = true;
                Some(keys::B)
            }
        }
    }
}

const DIRS: [u16; 4] = [keys::UP, keys::DOWN, keys::LEFT, keys::RIGHT];

/// Every plan it may try: each place a step away, with each thing to do
/// once there (a chip only with one in hand); and each place two steps
/// away (one way there), to stand on. Its first step only where the navi
/// can step now (`steps`, the directions that move it).
fn plans(chip: bool, steps: &[u16]) -> Vec<Plan> {
    let opposite = |a: u16, b: u16| matches!((a, b), (keys::UP, keys::DOWN) | (keys::DOWN, keys::UP) | (keys::LEFT, keys::RIGHT) | (keys::RIGHT, keys::LEFT));
    let mut paths = vec![[0, 0]];
    paths.extend(steps.iter().map(|&d| [d, 0]));
    for (i, &a) in DIRS.iter().enumerate().filter(|(_, d)| steps.contains(d)) {
        for (j, &b) in DIRS.iter().enumerate() {
            // (Up then left reaches where left then up does: one of them. And no step undone.)
            if opposite(a, b) || (a != b && j < i) {
                continue;
            }
            paths.push([a, b]);
        }
    }
    let thens: &[Then] = if chip { &[Then::Wait, Then::Chip, Then::Shot] } else { &[Then::Wait, Then::Shot] };
    paths.iter().flat_map(|&steps| if steps[1] == 0 { thens } else { &[Then::Wait][..] }.iter().map(move |&then| Plan { steps, then })).collect()
}

/// Whether a plan tried in `b` goes on: the fight (dimmed or not) of a
/// round not yet decided, not in the game's pause.
fn fight_goes_on(b: &Battle) -> bool {
    b.round.mode == mode::FIGHTING && !b.is_battle_over() && b.paused_by().is_none() && !b.is_stopped()
}

fn hp(b: &Battle, side: u8) -> i64 {
    b.player(side).map_or(0, |r| b.objects.get(r).hp as i64)
}

/// What a trial did, for side `side`, from the battle `before` to `after`:
/// the HP it took away and the HP it lost, in tenths of HP (a navi
/// deleted counts as [`DELETED`]).
fn outcome(before: &Battle, after: &Battle, side: u8) -> (i64, i64) {
    // (A battle the engine stopped is the worst: the session ends there.)
    if after.is_stopped() {
        return (0, DELETED);
    }
    let them = side ^ 1;
    let deleted = |s: u8| if after.is_battle_over() && hp(after, s) == 0 { DELETED } else { 0 };
    ((hp(before, them) - hp(after, them)) * DEALT + deleted(them), (hp(before, side) - hp(after, side)) * TAKEN + deleted(side))
}

/// What the other player may do, as a plan is tried: by the ticks since
/// the decision, from what they hold now. They go on holding it a moment,
/// step up, step down, use their next chip at once, or shoot at once. A plan
/// is worth what it takes away on average over them, less the most it loses
/// in any.
const GUESSES: usize = 5;

/// The threads a fighter tries its plans on at most ([`Fighter::new`]):
/// its own and helpers.
pub const THREADS: usize = 3;

fn guess(k: usize, theirs: u16, t: u32) -> u16 {
    match k {
        0 if t < PLAYER_HOLDS => theirs,
        1 if t < 2 => keys::UP,
        2 if t < 2 => keys::DOWN,
        // (Let go first: what they hold now isn't a press.)
        3 if t == 1 => keys::A,
        4 if t == 1 => keys::B,
        _ => 0,
    }
}

/// What deciding has cost so far.
#[derive(Clone, Copy, Debug, Default)]
pub struct Spent {
    pub decisions: u32,
    /// The ticks run in copies of the battle.
    pub ticks: u64,
    /// The time the decisions took (on its own thread, where it has one),
    /// and the longest.
    pub time: Duration,
    pub longest: Duration,
    /// The decisions not ready when they were to take effect, and the time
    /// the battle waited for them.
    pub late: u32,
    pub waited: Duration,
}

/// A decision to work out: the battle as it stands, the plan it follows, and
/// what the other player holds.
struct Question {
    id: u64,
    battle: Box<Battle>,
    now: Exec,
    theirs: u16,
    side: u8,
    level: Level,
    /// No decision, only its threads warmed: each steps a copy of the battle
    /// a tick (the first battle a thread runs loads the content's runtime
    /// there).
    warm: bool,
}

/// What each plan tried is worth (the plan followed last), and what working
/// it out cost.
struct Answer {
    id: u64,
    tried: Vec<(Exec, i64)>,
    ticks: u64,
    took: Duration,
}

/// The plans of a decision to try, and what they're tried from: the
/// battle as each guess at the other player leaves it as the decision
/// takes effect.
struct Job {
    /// Each guess (by its number) that plays out unlike the others, the
    /// battle as it leaves it, and how many guesses play out as it does.
    aheads: Vec<(usize, Battle, i64)>,
    plans: Vec<Exec>,
    side: u8,
    theirs: u16,
    /// The ticks from the decision to the aheads.
    waited: u32,
    horizon: u32,
    /// Each plan tried for its ticks whatever the battle's state
    /// ([`Question::warm`]).
    warm: bool,
}

impl Job {
    /// What plan `i` is worth (what it takes away on average over the
    /// guesses, less the most it loses in any, times the guesses), tried in
    /// `trial`; and the ticks that took.
    fn score(&self, i: usize, trial: &mut Battle) -> (i64, u64) {
        let (mut dealt, mut taken, mut ticks) = (0, 0, 0u64);
        for (k, ahead, alike) in &self.aheads {
            let k = *k;
            trial.clone_from(ahead);
            let mut exec = Exec { trial: true, ..self.plans[i] };
            let mut t = 0;
            while t < self.horizon && (self.warm || fight_goes_on(trial)) {
                let mine = exec.buttons(trial, self.side);
                step(trial, self.side, mine, guess(k, self.theirs, self.waited + t));
                t += 1;
            }
            ticks += t as u64;
            let (d, l) = outcome(ahead, trial, self.side);
            dealt += d * alike;
            taken = taken.max(l);
        }
        (dealt - taken * GUESSES as i64, ticks)
    }

    /// Plans `share`, `share + of`, `share + 2 of` and so on: by index, what
    /// each is worth, and the ticks they took.
    fn share(&self, share: usize, of: usize, trial: &mut Battle) -> (Vec<(usize, i64)>, u64) {
        let mut ticks = 0;
        let worths = (share..self.plans.len())
            .step_by(of)
            .map(|i| {
                let (w, t) = self.score(i, trial);
                ticks += t;
                (i, w)
            })
            .collect();
        (worths, ticks)
    }
}

/// A thread that tries its share of a decision's plans
/// ([`Search::helpers`]): the job and its share go, the worths come back.
#[cfg(not(target_arch = "wasm32"))]
struct Helper {
    jobs: std::sync::mpsc::Sender<(std::sync::Arc<Job>, usize, usize)>,
    worths: std::sync::mpsc::Receiver<(Vec<(usize, i64)>, u64)>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Helper {
    fn spawn() -> Option<Helper> {
        let (jobs, asked) = std::sync::mpsc::channel::<(std::sync::Arc<Job>, usize, usize)>();
        let (tell, worths) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("nettai-training helper".into())
            .spawn(move || {
                let mut trial: Option<Battle> = None;
                // (Until the search is gone.)
                while let Ok((job, share, of)) = asked.recv() {
                    let trial = trial.get_or_insert_with(|| job.aheads[0].1.clone());
                    if tell.send(job.share(share, of, trial)).is_err() {
                        break;
                    }
                }
            })
            .ok()?;
        Some(Helper { jobs, worths })
    }
}

/// What works decisions out: a copy of the battle to try plans in (kept for
/// its memory), and threads that try a share of the plans each.
#[derive(Default)]
struct Search {
    trial: Option<Battle>,
    #[cfg(not(target_arch = "wasm32"))]
    helpers: Vec<Helper>,
}

impl Search {
    /// One with `threads` in all trying plans (this one's and helpers).
    fn with(threads: usize) -> Search {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let helpers = (1..threads.max(1)).map_while(|_| Helper::spawn()).collect();
            Search { trial: None, helpers }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = threads;
            Search::default()
        }
    }

    /// From the battle as the plan it follows and each guess would leave it
    /// a reaction time on, try each plan and the plan it follows going on.
    fn answer(&mut self, q: &Question) -> Answer {
        let start = Instant::now();
        let (b, side, theirs, level) = (&*q.battle, q.side, q.theirs, q.level);
        if q.warm {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let of = self.helpers.len() + 1;
                let job = std::sync::Arc::new(Job { aheads: vec![(0, b.clone(), 1)], plans: vec![q.now; of], side, theirs, waited: 0, horizon: 1, warm: true });
                for (n, helper) in self.helpers.iter().enumerate() {
                    helper.jobs.send((job.clone(), n + 1, of)).expect("the helper is there");
                }
                job.share(0, of, self.trial.get_or_insert_with(|| b.clone()));
                for helper in &self.helpers {
                    helper.worths.recv().expect("the helper is there");
                }
            }
            return Answer { id: q.id, tried: Vec::new(), ticks: 0, took: start.elapsed() };
        }
        let mut ticks = 0u64;
        let mut aheads: Vec<(usize, Battle, i64)> = Vec::with_capacity(GUESSES);
        // (By the battle each leaves and the presses each has left, the guesses that play out alike: tried once.)
        let mut kinds: Vec<(u64, [u16; PLAYER_HOLDS as usize])> = Vec::with_capacity(GUESSES);
        let mut waited = 0;
        for k in 0..GUESSES {
            let mut ahead = b.clone();
            let mut now = q.now;
            let mut t = 0;
            while t < level.reaction && fight_goes_on(&ahead) {
                let mine = now.buttons(&ahead, side);
                step(&mut ahead, side, mine, guess(k, theirs, t));
                t += 1;
            }
            ticks += t as u64;
            waited = t;
            let kind = (ahead.digest(), std::array::from_fn(|n| guess(k, theirs, t + n as u32)));
            match kinds.iter().position(|seen| *seen == kind) {
                Some(at) => aheads[at].2 += 1,
                None => {
                    kinds.push(kind);
                    aheads.push((k, ahead, 1));
                }
            }
        }
        let chip = aheads[0].1.hands[side as usize].remaining() > 0;
        let trial = self.trial.get_or_insert_with(|| b.clone());
        // (The directions the navi can step in as it takes effect: each held in a copy until it moves, or not.)
        let mut steps = Vec::new();
        for dir in DIRS {
            trial.clone_from(&aheads[0].1);
            let from = trial.player(side).map(|r| trial.objects.get(r).panel);
            let mut t = 0;
            while t < STEP_MOST as u32 && fight_goes_on(trial) {
                step(trial, side, dir, guess(0, theirs, waited + t));
                t += 1;
                if trial.player(side).map(|r| trial.objects.get(r).panel) != from {
                    steps.push(dir);
                    break;
                }
            }
            ticks += t as u64;
        }
        let plans: Vec<Exec> = plans(chip, &steps).into_iter().map(|p| Exec::new(p, level.charges)).chain(std::iter::once(q.now)).collect();
        let job = Job { aheads, plans, side, theirs, waited, horizon: level.horizon, warm: false };
        let mut worths = vec![0; job.plans.len()];
        #[cfg(not(target_arch = "wasm32"))]
        let job = {
            // (Each helper tries its share as this one tries its own: the worths go by plan, whoever tried it.)
            let of = self.helpers.len() + 1;
            let job = std::sync::Arc::new(job);
            for (n, helper) in self.helpers.iter().enumerate() {
                helper.jobs.send((job.clone(), n + 1, of)).expect("the helper is there");
            }
            let (mine, t) = job.share(0, of, trial);
            ticks += t;
            for (i, w) in mine {
                worths[i] = w;
            }
            for helper in &self.helpers {
                let (theirs, t) = helper.worths.recv().expect("the helper is there");
                ticks += t;
                for (i, w) in theirs {
                    worths[i] = w;
                }
            }
            job
        };
        #[cfg(target_arch = "wasm32")]
        {
            let (mine, t) = job.share(0, 1, trial);
            ticks += t;
            for (i, w) in mine {
                worths[i] = w;
            }
        }
        let tried = job.plans.iter().copied().zip(worths).collect();
        Answer { id: q.id, tried, ticks, took: start.elapsed() }
    }
}

/// Where decisions are worked out: a thread of the fighter's own (with the
/// content's runtime of its own, as every thread that runs a battle has,
/// and helpers with theirs), or, where there are no threads (the web), the
/// caller's.
enum Thinker {
    Inline { search: Box<Search>, answer: Option<Answer> },
    #[cfg(not(target_arch = "wasm32"))]
    Thread { ask: std::sync::mpsc::Sender<Question>, answers: std::sync::mpsc::Receiver<Answer> },
}

impl Thinker {
    /// On a thread of its own, with `threads` in all trying plans.
    fn new(threads: usize) -> Thinker {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (ask, questions) = std::sync::mpsc::channel::<Question>();
            let (tell, answers) = std::sync::mpsc::channel();
            let spawned = std::thread::Builder::new().name("nettai-training fighter".into()).spawn(move || {
                let mut search = Search::with(threads);
                // (Until the fighter is gone.)
                while let Ok(q) = questions.recv() {
                    if tell.send(search.answer(&q)).is_err() {
                        break;
                    }
                }
            });
            if spawned.is_ok() {
                return Thinker::Thread { ask, answers };
            }
        }
        let _ = threads;
        Thinker::Inline { search: Box::default(), answer: None }
    }

    /// Warm its threads (none inline: there's nothing to warm).
    fn warm(&mut self, q: Question) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Thinker::Thread { ask, .. } = self {
            ask.send(q).expect("the fighter's thread is there");
        }
        #[cfg(target_arch = "wasm32")]
        let _ = q;
    }

    fn ask(&mut self, q: Question) {
        match self {
            Thinker::Inline { search, answer } => *answer = Some(search.answer(&q)),
            #[cfg(not(target_arch = "wasm32"))]
            Thinker::Thread { ask, .. } => ask.send(q).expect("the fighter's thread is there"),
        }
    }

    /// The answer to question `id`, waiting for it if need be; whether it
    /// was waited for.
    fn answer(&mut self, id: u64) -> (Answer, bool) {
        match self {
            Thinker::Inline { answer, .. } => (answer.take().filter(|a| a.id == id).expect("asked"), false),
            #[cfg(not(target_arch = "wasm32"))]
            Thinker::Thread { answers, .. } => {
                let mut waited = false;
                loop {
                    // (An answer to a question from before the set started over is dropped.)
                    let a = match answers.try_recv() {
                        Ok(a) => a,
                        Err(_) => {
                            waited = true;
                            answers.recv().expect("the fighter's thread is there")
                        }
                    };
                    if a.id == id {
                        return (a, waited);
                    }
                }
            }
        }
    }
}

/// An opponent that fights, for the navi of one side.
pub struct Fighter {
    level: Level,
    seed: u64,
    rng: SplitMix64,
    /// What it does now.
    now: Exec,
    /// The decision asked and not yet taken: its question, and the tick it
    /// takes effect on.
    pending: Option<(u64, u32)>,
    asked: u64,
    /// Its ticks, and the ticks of the fight since it last decided.
    ticks: u32,
    since: u32,
    /// It pressed L the tick before (to let go between presses), and a copy
    /// of the battle to see whether L opens the custom screen.
    l_pressed: bool,
    probe: Option<Box<Battle>>,
    /// Its custom screen's.
    picker: Picker,
    thinker: Thinker,
    pub spent: Spent,
}

impl Fighter {
    /// One that works its decisions out on a thread of its own, where there
    /// are threads, with helpers: [`THREADS`] in all, fewer on a machine
    /// with fewer cores to spare.
    pub fn new(level: Level, seed: u64) -> Fighter {
        let spare = std::thread::available_parallelism().map_or(1, |n| n.get().saturating_sub(2));
        Fighter::threaded(level, seed, spare.clamp(1, THREADS))
    }

    /// The same, with `threads` in all trying plans: the presses are the
    /// same whatever their number.
    pub fn threaded(level: Level, seed: u64, threads: usize) -> Fighter {
        Fighter::with(level, seed, Thinker::new(threads))
    }

    /// One that works its decisions out as it's asked for its buttons.
    pub fn inline(level: Level, seed: u64) -> Fighter {
        Fighter::with(level, seed, Thinker::Inline { search: Box::default(), answer: None })
    }

    fn with(level: Level, seed: u64, thinker: Thinker) -> Fighter {
        Fighter {
            level,
            seed,
            rng: SplitMix64::new(seed),
            now: Exec::new(Plan { steps: [0, 0], then: Then::Wait }, level.charges),
            pending: None,
            asked: 0,
            ticks: 0,
            since: 0,
            l_pressed: false,
            probe: None,
            picker: Picker::default(),
            thinker,
            spent: Spent::default(),
        }
    }

    /// As it was new: the set starts over. (Its thread stays, and what it
    /// was working out is dropped.)
    pub fn reset(&mut self) {
        self.rng = SplitMix64::new(self.seed);
        self.now = Exec::new(Plan { steps: [0, 0], then: Then::Wait }, self.level.charges);
        self.pending = None;
        (self.ticks, self.since, self.l_pressed) = (0, 0, false);
        self.picker = Picker::default();
    }

    pub fn level(&self) -> Level {
        self.level
    }

    pub fn set_level(&mut self, level: Level) {
        self.level = level;
    }

    /// Its buttons in the fight on side `side` of `b` this tick; `theirs`:
    /// what the other player holds (it supposes they go on holding it a
    /// moment).
    pub fn buttons(&mut self, b: &Battle, side: u8, theirs: u16) -> u16 {
        self.ticks += 1;
        if self.ticks == 1 {
            // (A question no one takes, as the round opens: its threads load the content's runtime then, not as the
            // fight's first decision is due.)
            self.asked += 1;
            self.thinker.warm(Question { id: self.asked, battle: Box::new(b.clone()), now: self.now, theirs, side, level: self.level, warm: true });
        }
        self.take_decision();
        if !fight_goes_on(b) || b.is_dimmed() {
            return self.now.buttons(b, side);
        }
        // (With nothing in hand, the custom screen: L, where a copy shows it opens the screen.)
        if self.l_pressed {
            self.l_pressed = false;
            return 0;
        }
        if b.hands[side as usize].remaining() == 0 && self.ticks % OPEN_EVERY == 0 && self.opens(b, side, theirs) {
            self.l_pressed = true;
            return keys::L;
        }
        self.since += 1;
        if self.since >= self.level.every && self.pending.is_none() {
            self.since = 0;
            self.asked += 1;
            self.pending = Some((self.asked, self.ticks + self.level.reaction));
            let q = Question { id: self.asked, battle: Box::new(b.clone()), now: self.now, theirs, side, level: self.level, warm: false };
            self.thinker.ask(q);
            self.take_decision();
        }
        self.now.buttons(b, side)
    }

    /// Whether L pressed now opens the custom screen: in a copy, a few
    /// ticks on.
    fn opens(&mut self, b: &Battle, side: u8, theirs: u16) -> bool {
        let probe = self.probe.get_or_insert_with(|| Box::new(b.clone()));
        Battle::clone_from(probe, b);
        (0..OPEN_TICKS).any(|t| {
            step(probe, side, if t == 0 { keys::L } else { 0 }, theirs);
            probe.round.mode == mode::CUSTOM
        })
    }

    /// The decision asked, once its tick has come: its answer (waited for,
    /// if it isn't ready), and the plan picked from it.
    fn take_decision(&mut self) {
        let Some((id, _)) = self.pending.filter(|(_, at)| *at <= self.ticks) else { return };
        self.pending = None;
        let waiting = Instant::now();
        let (answer, waited) = self.thinker.answer(id);
        if waited {
            self.spent.late += 1;
            self.spent.waited += waiting.elapsed();
        }
        self.spent.decisions += 1;
        self.spent.ticks += answer.ticks;
        self.spent.time += answer.took;
        self.spent.longest = self.spent.longest.max(answer.took);
        let tried = answer.tried;
        let best = tried.iter().map(|(_, w)| *w).max().unwrap_or(0);
        // (The plan it followed as it asked, not yet done and as good as the best, goes on: no change of mind for
        // nothing.)
        let following = tried.last().filter(|(e, w)| !e.done && *w >= best).map(|(e, _)| *e);
        let picked = if self.level.blunders > 0 && self.rng.below(100) < self.level.blunders as u64 {
            tried[self.rng.below(tried.len() as u64) as usize].0
        } else if let Some(e) = following {
            e
        } else {
            let bests: Vec<Exec> = tried.iter().filter(|(_, w)| *w == best).map(|(e, _)| *e).collect();
            bests[self.rng.below(bests.len() as u64) as usize]
        };
        // (The plan it follows, picked again, goes on from where it is.)
        if picked.plan != self.now.plan || self.now.done {
            self.now = picked;
        }
    }
}

impl Ai for Fighter {
    fn fight(&mut self, b: &Battle, side: u8, theirs: u16) -> u16 {
        self.buttons(b, side, theirs)
    }

    fn custom_screen(&mut self, b: &Battle, side: u8) -> u16 {
        self.picker.buttons(b, side as usize)
    }

    fn reset(&mut self) {
        Fighter::reset(self);
    }
}

/// One tick of `b` with side `side` pressing `mine` and the other `theirs`,
/// as the session runs one: a tick the engine stops on (a state the
/// original can't go on from) stops the battle (`Battle::fail`).
fn step(b: &mut Battle, side: u8, mine: u16, theirs: u16) {
    let buttons = if side == 0 { [mine, theirs] } else { [theirs, mine] };
    let input = tick_input(b, buttons);
    if let Err(e) = nettai_frontend::session::in_engine(|| b.tick(&input.players, input.events)) {
        b.fail(nettai_frontend::session::panic_message(e.as_ref()));
    }
}

/// A custom screen's buttons that pick every chip they can, in the order
/// the cursor reaches them, then OK. Only the folder's chips dealt (no
/// button, no chip offered from outside it, no form).
#[derive(Clone, Copy, Debug, Default)]
pub struct Picker {
    /// What it holds, and for how many ticks more.
    held: u16,
    more: u8,
}

impl Picker {
    pub fn buttons(&mut self, b: &Battle, side: usize) -> u16 {
        let s = &b.custom.sides[side];
        let Some(screen) = s.screen.as_ref().filter(|s| b.round.mode == mode::CUSTOM && s.phase == Phase::Choosing) else {
            *self = Picker::default();
            return 0;
        };
        if !s.in_custom {
            return 0;
        }
        if self.more > 0 {
            self.more -= 1;
            return self.held;
        }
        // (Let go a tick between presses.)
        if std::mem::take(&mut self.held) != 0 {
            return 0;
        }
        let key = pick(screen);
        // (A direction moves the cursor on its second tick held: the screen's auto-repeat.)
        self.more = (key & (keys::UP | keys::DOWN | keys::LEFT | keys::RIGHT) != 0) as u8;
        self.held = key;
        key
    }
}

/// The press that picks the next chip, or goes to OK and presses it.
fn pick(screen: &Screen) -> u16 {
    let wanted = |i: u8| {
        let slot = &screen.slots[i as usize];
        matches!(slot.kind, SlotKind::Chip { .. }) && slot.state == SlotState::Selectable
    };
    if (screen.selected as usize) < MAX_SELECTIONS {
        if wanted(screen.cursor) {
            return keys::A;
        }
        if let Some(dir) = toward(screen, wanted) {
            return dir;
        }
    }
    if screen.cursor != OK_SLOT { keys::START } else { keys::A }
}

/// The first direction of the shortest way the cursor goes to a slot
/// `wanted` takes, by the screen's own moves.
fn toward(screen: &Screen, wanted: impl Fn(u8) -> bool) -> Option<u16> {
    let mut first: [Option<u16>; nettai_battle::custom::screen::SLOTS] = [None; nettai_battle::custom::screen::SLOTS];
    let mut queue = std::collections::VecDeque::from([screen.cursor]);
    let mut seen = [false; nettai_battle::custom::screen::SLOTS];
    seen[screen.cursor as usize] = true;
    while let Some(at) = queue.pop_front() {
        let slot = &screen.slots[at as usize];
        for (dir, to) in [(keys::UP, slot.vertical), (keys::LEFT, slot.left), (keys::RIGHT, slot.right)] {
            let Some(to) = to.filter(|t| (*t as usize) < seen.len() && !seen[*t as usize]) else { continue };
            seen[to as usize] = true;
            first[to as usize] = first[at as usize].or(Some(dir));
            if wanted(to) {
                return first[to as usize];
            }
            queue.push_back(to);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_battle::Content;
    use nettai_frontend::driver::{Driver, InFight, LivePlayer, OnCustomScreen, Opponent};
    use nettai_match::Set;
    use nettai_netplay::standin::Masher;
    use std::sync::Arc;

    fn content(game: &str) -> Arc<Content> {
        match game {
            "exe4" => nettai_match::testing::exe4_content(),
            "exe5" => nettai_match::testing::exe5_content(),
            _ => nettai_match::testing::exe6_content(),
        }
    }

    /// A match of `game` picked from `seed`, both sides the first's player
    /// with `hp`.
    fn even_match(content: &Arc<Content>, game: &str, seed: u32, hp: i64) -> nettai_match::Match {
        let mut m = nettai_match::pick::live(content, game, seed, None).unwrap();
        m.sides[1] = m.sides[0].clone();
        let base_hp = nettai_battle::content::PlayerFact::BaseHp.name();
        for side in &mut m.sides {
            side.set_fact(content, base_hp, &[nettai_battle::rules::Fact::Value(nettai_content_api::Value::Int(hp))]).unwrap();
        }
        assert_eq!(nettai_match::check_match(content, &m), Vec::<String>::new());
        m
    }

    /// Presses at random in the fight, as live play's
    /// `InFight::AttacksAtRandom` does, and picks most on the custom screen:
    /// the player of these tests, and the opponent to compare with.
    struct RandomPresses {
        masher: Masher,
        picker: Picker,
    }

    impl RandomPresses {
        fn new(seed: u64) -> RandomPresses {
            let mut masher = Masher::new(seed);
            masher.buster = true;
            RandomPresses { masher, picker: Picker::default() }
        }
    }

    impl Ai for RandomPresses {
        fn fight(&mut self, _b: &Battle, _side: u8, _theirs: u16) -> u16 {
            self.masher.buttons()
        }

        fn custom_screen(&mut self, b: &Battle, side: u8) -> u16 {
            // (Its presses go on as the screen is shown: one a tick, as live play's.)
            self.masher.buttons();
            self.picker.buttons(b, side as usize)
        }

        fn reset(&mut self) {}
    }

    /// Whether the round is decided: both navis came on the field, and one
    /// is deleted or the time is up. (Before they come, the round reads as
    /// over: no navis left.)
    struct Decided {
        started: bool,
    }

    impl Decided {
        fn now(&mut self, b: &Battle) -> bool {
            self.started |= b.player(0).is_some() && b.player(1).is_some() && !b.is_battle_over();
            self.started && b.is_battle_over()
        }
    }

    /// One tick of each of `ais`' buttons for its side of `b`.
    fn both(b: &Battle, ais: [&mut dyn Ai; 2]) -> [u16; 2] {
        let custom = b.round.mode == mode::CUSTOM;
        let [left, right] = ais;
        let l = if custom { left.custom_screen(b, 0) } else { left.fight(b, 0, 0) };
        let r = if custom { right.custom_screen(b, 1) } else { right.fight(b, 1, l) };
        [l, r]
    }

    /// How a round went: the side that won (none: undecided or a draw),
    /// each navi's HP at its end, and its ticks.
    #[derive(Debug)]
    struct Round {
        won: Option<u8>,
        hp: [i64; 2],
        ticks: u32,
        /// By side: the chips used (its hand one fewer in the fight), and
        /// the custom screens opened.
        chips: [u32; 2],
        screens: u32,
        /// By side: the longest stretch of the fight, in ticks, with chips in
        /// hand and none used.
        holding: [u32; 2],
    }

    /// The first round of `m` stepped by hand: the left navi pressing at
    /// random, the right one `dummy`; up to `most` ticks.
    fn first_round(content: &Arc<Content>, m: &nettai_match::Match, seed: u32, dummy: &mut dyn Ai, most: u32) -> Round {
        let mut b = Set::of(content, m, seed).start();
        let mut player = RandomPresses::new(seed as u64 ^ 0x5EED);
        let (mut ticks, mut decided) = (0, Decided { started: false });
        let (mut chips, mut screens, mut holding, mut held) = ([0; 2], 0, [0; 2], [0u32; 2]);
        while ticks < most && !decided.now(&b) && !b.is_stopped() {
            ticks += 1;
            let [left, right] = both(&b, [&mut player, dummy]);
            let (hands, custom) = (b.hands.each_ref().map(|h| h.remaining()), b.round.mode == mode::CUSTOM);
            step(&mut b, 0, left, right);
            for side in 0..2 {
                let used = b.hands[side].remaining() != hands[side];
                chips[side] += (b.round.mode == mode::FIGHTING && b.hands[side].remaining() < hands[side]) as u32;
                held[side] = if used || hands[side] == 0 { 0 } else { held[side] + fight_goes_on(&b) as u32 };
                holding[side] = holding[side].max(held[side]);
            }
            screens += (!custom && b.round.mode == mode::CUSTOM) as u32;
        }
        if let Some(nettai_battle::battle::RoundEnd::Error(why)) = b.round_end() {
            println!("seed {seed}: the engine stopped at tick {ticks}: {why}");
        }
        let hp = [hp(&b, 0), hp(&b, 1)];
        let won = match hp {
            [0, 0] => None,
            [0, _] => Some(1),
            [_, 0] => Some(0),
            _ => None,
        };
        Round { won, hp, ticks, chips, screens, holding }
    }

    /// Plugged into live play as its opponent's AI, in the fight and on
    /// the custom screen, it plays as it does by hand: the same battle,
    /// tick for tick (and the same again after the set starts over), one
    /// thinking on its own thread as one thinking inline. It deletes a
    /// player pressing at random.
    #[test]
    fn plugged_into_live_play_it_plays_as_by_hand() {
        let content = content("exe6");
        let m = even_match(&content, "exe6", 3, 100);
        let opponent = Opponent { in_fight: InFight::Ai, on_custom_screen: OnCustomScreen::Ai };
        let mut live = LivePlayer::against(Set::of(&content, &m, 3), opponent, 3);
        live.set_ai(Some(Box::new(Fighter::new(Level::HARD, 3))));
        let by_live = |live: &mut LivePlayer| {
            let mut b = live.start();
            let mut player = RandomPresses::new(3 ^ 0x5EED);
            let (mut digests, mut decided) = (Vec::new(), Decided { started: false });
            while digests.len() < 3000 && !decided.now(&b) {
                let left = if b.round.mode == mode::CUSTOM { player.custom_screen(&b, 0) } else { player.fight(&b, 0, 0) };
                let s = live.next(&b, left).unwrap();
                b.tick(&s.input, s.events);
                digests.push(b.digest());
            }
            (digests, decided.now(&b), [hp(&b, 0), hp(&b, 1)])
        };
        let (live_digests, over, hp) = by_live(&mut live);
        // (The fighter deletes the player.)
        assert_eq!((over, hp[0]), (true, 0), "{} ticks, HP {hp:?}", live_digests.len());
        assert!(hp[1] > 0);
        assert_eq!(by_live(&mut live).0, live_digests, "the set started over");
        // By hand.
        let mut b = Set::of(&content, &m, 3).start();
        let mut player = RandomPresses::new(3 ^ 0x5EED);
        let mut fighter = Fighter::inline(Level::HARD, 3);
        for (tick, digest) in live_digests.iter().enumerate() {
            let [left, right] = both(&b, [&mut player, &mut fighter]);
            step(&mut b, 0, left, right);
            assert_eq!(b.digest(), *digest, "tick {tick}");
        }
    }

    /// A level by name (`easy`, `normal`, `hard`) or by its parts
    /// (`r6h60e8b0`: reaction, horizon, every, blunders, any left out as
    /// Hard's); none for `random`.
    fn level_named(name: &str) -> Option<Level> {
        match name {
            "random" => return None,
            "easy" => return Some(Level::EASY),
            "normal" => return Some(Level::NORMAL),
            "hard" => return Some(Level::HARD),
            _ => {}
        }
        let mut level = Level::HARD;
        let mut rest = name;
        while let Some(c) = rest.chars().next() {
            let digits: String = rest[1..].chars().take_while(|c| c.is_ascii_digit()).collect();
            let n: u32 = digits.parse().unwrap_or_else(|_| panic!("a level's part {c} has no number: {name:?}"));
            match c {
                'r' => level.reaction = n,
                'h' => level.horizon = n,
                'e' => level.every = n,
                'b' => level.blunders = n,
                'c' => level.charges = n != 0,
                _ => panic!("no level part {c:?} in {name:?}"),
            }
            rest = &rest[1 + digits.len()..];
        }
        Some(level)
    }

    /// The opponent that fights against a player pressing at random, and
    /// one that presses at random for comparison: the first rounds of each
    /// game's matches, both sides the same build. Prints its wins, the HP
    /// it takes away and loses a minute, and what deciding cost. A
    /// measurement: `cargo test -p nettai-training --lib
    /// against_random_presses -- --ignored --nocapture`, with
    /// `FIGHTER_GAMES` (exe6,exe5,exe4), `FIGHTER_LEVELS`
    /// (random,easy,normal,hard; or parts, [`level_named`]),
    /// `FIGHTER_SEEDS` (24) and `FIGHTER_THREADS` (4).
    #[test]
    #[ignore]
    fn against_random_presses() {
        nettai_frontend::session::quiet_engine_panics();
        const HP: i64 = 400;
        const MOST: u32 = 60 * 60 * 3;
        let var = |name: &str, or: &str| std::env::var(name).unwrap_or_else(|_| or.into());
        let threads: usize = var("FIGHTER_THREADS", "4").parse().unwrap();
        let seeds: u32 = var("FIGHTER_SEEDS", "24").parse().unwrap();
        let (games, levels) = (var("FIGHTER_GAMES", "exe6,exe5,exe4"), var("FIGHTER_LEVELS", "random,easy,normal,hard"));
        println!("| game | dummy | won | lost | undecided | HP a minute (dealt / taken) | chips a minute (its / player's) | longest hold, s (its / player's) | custom screens a minute | ticks a round | decisions a round | ms a decision (mean / longest) | late a round (ms waited) | trial ticks a decision | µs a trial tick |");
        println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
        for game in games.split(',') {
            let content = content(game);
            for name in levels.split(',') {
                let level = level_named(name);
                let results = std::sync::Mutex::new(Vec::new());
                let next = std::sync::atomic::AtomicU32::new(0);
                std::thread::scope(|s| {
                    for _ in 0..threads {
                        s.spawn(|| {
                            loop {
                                let seed = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                if seed >= seeds {
                                    break;
                                }
                                let m = even_match(&content, game, seed, HP);
                                let (round, spent) = match level {
                                    Some(level) => {
                                        let mut fighter = Fighter::new(level, seed as u64);
                                        (first_round(&content, &m, seed, &mut fighter, MOST), fighter.spent)
                                    }
                                    None => (first_round(&content, &m, seed, &mut RandomPresses::new(seed as u64), MOST), Spent::default()),
                                };
                                results.lock().unwrap().push((round, spent));
                            }
                        });
                    }
                });
                let results = results.into_inner().unwrap();
                let count = |won: Option<u8>| results.iter().filter(|(r, _)| r.won == won).count();
                let ticks: u64 = results.iter().map(|(r, _)| r.ticks as u64).sum();
                let minutes = ticks as f64 / 3600.0;
                let dealt: i64 = results.iter().map(|(r, _)| HP - r.hp[0]).sum();
                let taken: i64 = results.iter().map(|(r, _)| HP - r.hp[1]).sum();
                let decisions: u64 = results.iter().map(|(_, s)| s.decisions as u64).sum();
                let time: Duration = results.iter().map(|(_, s)| s.time).sum();
                let trial: u64 = results.iter().map(|(_, s)| s.ticks).sum();
                let longest = results.iter().map(|(_, s)| s.longest).max().unwrap_or_default();
                let per = |x: f64| if decisions == 0 { 0.0 } else { x / decisions as f64 };
                println!(
                    "| {game} | {name} | {} | {} | {} | {:.0} / {:.0} | {:.1} / {:.1} | {:.1} / {:.1} | {:.1} | {:.0} | {:.0} | {:.2} / {:.1} | {:.1} ({:.0}) | {:.0} | {:.2} |",
                    count(Some(1)),
                    count(Some(0)),
                    count(None),
                    dealt as f64 / minutes,
                    taken as f64 / minutes,
                    results.iter().map(|(r, _)| r.chips[1]).sum::<u32>() as f64 / minutes,
                    results.iter().map(|(r, _)| r.chips[0]).sum::<u32>() as f64 / minutes,
                    results.iter().map(|(r, _)| r.holding[1]).sum::<u32>() as f64 / 60.0 / results.len() as f64,
                    results.iter().map(|(r, _)| r.holding[0]).sum::<u32>() as f64 / 60.0 / results.len() as f64,
                    results.iter().map(|(r, _)| r.screens).sum::<u32>() as f64 / minutes,
                    ticks as f64 / results.len() as f64,
                    decisions as f64 / results.len() as f64,
                    per(time.as_secs_f64() * 1000.0),
                    longest.as_secs_f64() * 1000.0,
                    results.iter().map(|(_, s)| s.late).sum::<u32>() as f64 / results.len() as f64,
                    results.iter().map(|(_, s)| s.waited).sum::<Duration>().as_secs_f64() * 1000.0 / results.len() as f64,
                    per(trial as f64),
                    if trial == 0 { 0.0 } else { time.as_secs_f64() * 1e6 / trial as f64 },
                );
            }
        }
    }

    /// What a tick of each game's battle costs, stepped with random presses
    /// on both sides, and a copy of it: what the trials are made of. A
    /// measurement: `cargo test -p nettai-training --lib tick_cost --
    /// --ignored --nocapture`.
    #[test]
    #[ignore]
    fn tick_cost() {
        nettai_frontend::session::quiet_engine_panics();
        for game in ["exe6", "exe5", "exe4"] {
            let content = content(game);
            let (mut ticks, mut stepping, mut copies, mut copying) = ([0u64; 2], [Duration::ZERO; 2], 0u64, Duration::ZERO);
            for seed in 0..4 {
                let m = even_match(&content, game, seed, 400);
                let mut b = Set::of(&content, &m, seed).start();
                let mut copy = b.clone();
                let (mut left, mut right) = (RandomPresses::new(seed as u64), RandomPresses::new(seed as u64 ^ 1));
                for _ in 0..3000 {
                    let [l, r] = both(&b, [&mut left, &mut right]);
                    let fight = fight_goes_on(&b) as usize;
                    let at = Instant::now();
                    step(&mut b, 0, l, r);
                    stepping[fight] += at.elapsed();
                    ticks[fight] += 1;
                    if b.round.mode == mode::FIGHTING {
                        let at = Instant::now();
                        copy.clone_from(&b);
                        copying += at.elapsed();
                        copies += 1;
                    }
                }
            }
            let per = |k: usize| stepping[k].as_secs_f64() * 1e6 / ticks[k].max(1) as f64;
            println!("{game}: {:.1} µs a tick of the fight ({} ticks), {:.1} µs a tick otherwise ({}), {:.1} µs a copy", per(1), ticks[1], per(0), ticks[0], copying.as_secs_f64() * 1e6 / copies.max(1) as f64);
        }
    }

    /// Whether its decisions keep up with a battle run at its own speed (60
    /// ticks a second, as the app runs one): a round of each game at each
    /// level for `FIGHTER_SECONDS` (20), the dummy thinking on its own
    /// thread. Prints the decisions that weren't ready when they were to
    /// take effect and the time the battle waited for them. A measurement:
    /// `cargo test -p nettai-training --lib real_time -- --ignored
    /// --nocapture` (`FIGHTER_GAMES`, `FIGHTER_LEVELS`).
    #[test]
    #[ignore]
    fn real_time() {
        nettai_frontend::session::quiet_engine_panics();
        let var = |name: &str, or: &str| std::env::var(name).unwrap_or_else(|_| or.into());
        let seconds: u32 = var("FIGHTER_SECONDS", "20").parse().unwrap();
        let (games, levels) = (var("FIGHTER_GAMES", "exe6,exe5,exe4"), var("FIGHTER_LEVELS", "easy,normal,hard"));
        println!("| game | dummy | decisions | late | waited (ms, all / longest) | ms a decision (mean / longest) |");
        println!("|---|---|---|---|---|---|");
        for game in games.split(',') {
            let content = content(game);
            for name in levels.split(',') {
                let level = level_named(name).expect("a level");
                let m = even_match(&content, game, 1, 400);
                let mut b = Set::of(&content, &m, 1).start();
                let mut player = RandomPresses::new(1 ^ 0x5EED);
                let mut fighter = Fighter::new(level, 1);
                let (mut decided, mut longest_wait) = (Decided { started: false }, Duration::ZERO);
                let start = Instant::now();
                for tick in 0..seconds * 60 {
                    let due = start + Duration::from_micros(tick as u64 * 16_667);
                    if let Some(rest) = due.checked_duration_since(Instant::now()) {
                        std::thread::sleep(rest);
                    }
                    let waited = fighter.spent.waited;
                    let [left, right] = both(&b, [&mut player, &mut fighter]);
                    longest_wait = longest_wait.max(fighter.spent.waited - waited);
                    step(&mut b, 0, left, right);
                    if decided.now(&b) {
                        // (Another round, from where the set starts.)
                        b = Set::of(&content, &m, tick).start();
                        decided = Decided { started: false };
                    }
                }
                let s = fighter.spent;
                println!(
                    "| {game} | {name} | {} | {} | {:.0} / {:.0} | {:.1} / {:.0} |",
                    s.decisions,
                    s.late,
                    s.waited.as_secs_f64() * 1000.0,
                    longest_wait.as_secs_f64() * 1000.0,
                    s.time.as_secs_f64() * 1000.0 / s.decisions.max(1) as f64,
                    s.longest.as_secs_f64() * 1000.0,
                );
            }
        }
    }
}
