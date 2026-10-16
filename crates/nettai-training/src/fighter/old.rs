//! The fighter as it was before it planned by tiles (2026-10-13 to 2026-10-16), kept for the measurements
//! that compare the two (`against_random_presses` with `FIGHTER_PLAYER=old-hard`, the game-sense scenarios).
//! Test-only: not an option of the crate.
#![allow(dead_code)]

use std::time::{Duration, Instant};

use nettai_battle::Battle;
use nettai_battle::battle::mode;
use nettai_battle::custom::screen::{MAX_SELECTIONS, OK_SLOT, Screen};
use nettai_battle::content::{CustomScreenLayout, ScreenKey};
use nettai_battle::custom::{FolderChip, Phase, SlotKind, SlotState};
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
    /// It decides again at once (a reaction time on, as ever) when the
    /// other player presses A or B, or it loses HP: it sees an attack come,
    /// not only on its beat (`every`).
    pub alert: bool,
    /// In a hundred, how much of what a plan loses is the most it loses in
    /// any guess (the rest the average over them): 100 keeps out of the way
    /// of any attack that may come, 0 risks what is likely.
    pub caution: u32,
    /// In HP, what using a chip is worth besides what it takes away: a
    /// chip held is no use.
    pub eager: u32,
    /// On its custom screen it picks the chips worth most first (each tried
    /// in a copy of the fight: [`Fighter`]'s docs), the hand used in that
    /// order; else every chip it can, in the order the cursor reaches them.
    pub chooses: bool,
}

impl Level {
    pub const EASY: Level = Level { reaction: 30, horizon: 40, every: 20, blunders: 50, charges: true, alert: false, caution: 50, eager: 10, chooses: false };
    pub const NORMAL: Level = Level { reaction: 14, horizon: 50, every: 12, blunders: 15, charges: true, alert: true, caution: 50, eager: 10, chooses: true };
    pub const HARD: Level = Level { reaction: 6, horizon: 60, every: 12, blunders: 0, charges: true, alert: true, caution: 50, eager: 10, chooses: true };
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
/// The ticks a chip is tried for as its worth is found: long enough for a
/// dimming chip's cut-in and a navi chip's attack, which a decision's
/// horizon is too short to see.
const WORTH_HORIZON: u32 = 180;
/// The ticks it looks at the chips dealt before it picks
/// ([`Level::chooses`]): half a second.
const LOOKS: u32 = 30;
/// The ticks a copy of the battle is given to get from its custom screen to
/// the fight (both screens closing, a turn's start; a round's first, its
/// banner).
const TO_THE_FIGHT: u32 = 900;
/// The ticks on end the fight takes buttons before a chip is tried in it.
const SETTLED: u32 = 8;
/// The ticks a copy of the custom screen is given to make its picks.
const PICKING: u32 = 400;
const OPEN_TICKS: u32 = 12;

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
    /// Whether it's done.
    done: bool,
    /// It holds B ([`Level::charges`]).
    charges: bool,
    /// What it pressed the tick before (a press is a button not held then).
    last: u16,
}

impl Exec {
    fn new(plan: Plan, charges: bool) -> Exec {
        Exec { plan, step: 0, held: 0, from: None, done: false, charges, last: 0 }
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
            return None;
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
    /// The custom screens whose chips it tried, and the time that took.
    pub chips: u32,
    pub chip_time: Duration,
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
    /// No decision, but what each of these chips, dealt on the custom screen
    /// shown (by slot), is worth in the fight it opens on
    /// ([`Search::worths`]).
    chips: Vec<(u8, FolderChip)>,
}

/// What each plan tried is worth (the plan followed last), and what working
/// it out cost.
struct Answer {
    id: u64,
    tried: Vec<(Exec, i64)>,
    /// The chips' worths, of a question of them.
    worths: Vec<i64>,
    ticks: u64,
    took: Duration,
}

/// The plans of a decision to try, and what they're tried from: the
/// battle as each guess at the other player leaves it as the decision
/// takes effect.
#[derive(Clone)]
struct Job {
    /// Each guess (by its number) that plays out unlike the others, the
    /// battle as it leaves it, and how many guesses play out as it does.
    aheads: Vec<(usize, Battle, i64)>,
    plans: Vec<Exec>,
    side: u8,
    theirs: u16,
    /// The ticks from the decision to the aheads; the level, its horizon
    /// what each plan is tried for.
    waited: u32,
    horizon: u32,
    level: Level,
    /// Each plan tried for its ticks whatever the battle's state
    /// ([`Question::warm`]).
    warm: bool,
    /// A plan is worth what it takes away alone ([`Search::worths`]).
    dealt_only: bool,
}

impl Job {
    /// What plan `i` is worth (what it takes away on average over the
    /// guesses, less the most it loses in any, times the guesses), tried in
    /// `trial`; and the ticks that took.
    fn score(&self, i: usize, trial: &mut Battle) -> (i64, u64) {
        let (mut dealt, mut taken, mut lost, mut ticks) = (0, 0, 0, 0u64);
        for (k, ahead, alike) in &self.aheads {
            let k = *k;
            trial.clone_from(ahead);
            let mut exec = self.plans[i];
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
            lost += l * alike;
        }
        if self.dealt_only {
            return (dealt, ticks);
        }
        // (The most it loses, and the average, by its caution; both times the guesses, as `dealt` is.)
        let caution = self.level.caution.min(100) as i64;
        let taken = (taken * GUESSES as i64 * caution + lost * (100 - caution)) / 100;
        let eager = if self.plans[i].plan.then == Then::Chip { self.level.eager as i64 * DEALT * GUESSES as i64 } else { 0 };
        (dealt - taken + eager, ticks)
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
                let job = std::sync::Arc::new(Job { aheads: vec![(0, b.clone(), 1)], plans: vec![q.now; of], side, theirs, waited: 0, horizon: 1, level: q.level, warm: true, dealt_only: false });
                for (n, helper) in self.helpers.iter().enumerate() {
                    helper.jobs.send((job.clone(), n + 1, of)).expect("the helper is there");
                }
                job.share(0, of, self.trial.get_or_insert_with(|| b.clone()));
                for helper in &self.helpers {
                    helper.worths.recv().expect("the helper is there");
                }
            }
            return Answer { id: q.id, tried: Vec::new(), worths: Vec::new(), ticks: 0, took: start.elapsed() };
        }
        if !q.chips.is_empty() {
            return self.worths(q, start);
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
        let job = Job { aheads, plans, side, theirs, waited, horizon: level.horizon, level, warm: false, dealt_only: false };
        let (worths, t) = self.run(&job);
        ticks += t;
        let tried = job.plans.iter().copied().zip(worths).collect();
        Answer { id: q.id, tried, worths: Vec::new(), ticks, took: start.elapsed() }
    }

    /// Each of `job`'s plans tried, on this thread and its helpers' (each a
    /// share): what each is worth, by plan, and the ticks that took.
    fn run(&mut self, job: &Job) -> (Vec<i64>, u64) {
        let trial = self.trial.get_or_insert_with(|| job.aheads[0].1.clone());
        let mut worths = vec![0; job.plans.len()];
        let mut ticks = 0;
        #[cfg(not(target_arch = "wasm32"))]
        {
            // (Each helper tries its share as this one tries its own: the worths go by plan, whoever tried it.)
            let of = self.helpers.len() + 1;
            let shared = std::sync::Arc::new(job.clone());
            for (n, helper) in self.helpers.iter().enumerate() {
                helper.jobs.send((shared.clone(), n + 1, of)).expect("the helper is there");
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
        }
        #[cfg(target_arch = "wasm32")]
        {
            let (mine, t) = job.share(0, 1, trial);
            ticks += t;
            for (i, w) in mine {
                worths[i] = w;
            }
        }
        (worths, ticks)
    }

    /// How its side picks on the custom screen `q` shows: by slot, the
    /// order to pick them in (the higher first; none below 0 is picked).
    ///
    /// Each chip dealt is worth what it takes away in the fight the screen
    /// opens on ([`Search::chip_worth`]). Picking one can make others
    /// unpickable (a code that goes with none of theirs), so each chip is
    /// tried as the first pick on a copy of the screen, the rest picked
    /// after it by their worth, and the picks worth most in all are the
    /// hand. Where the picks form a Program Advance on the copy, it is
    /// worth what it takes away in the fight in their place.
    fn worths(&mut self, q: &Question, start: Instant) -> Answer {
        let mut ticks = 0;
        let mut worths: Vec<i64> = Vec::with_capacity(q.chips.len());
        for (n, (slot, chip)) in q.chips.iter().enumerate() {
            let worth = match q.chips[..n].iter().position(|(_, c)| c == chip) {
                Some(at) => worths[at],
                None => self.chip_worth(q, &[*slot], &mut ticks),
            };
            worths.push(worth);
        }
        let worth_of = |slot: u8| q.chips.iter().position(|(s, _)| *s == slot).map_or(0, |at| worths[at]);
        // (Each first pick, the rest by worth: the picks, and what they are worth in all.)
        let mut best: Option<(Vec<u8>, i64)> = None;
        for (n, (first, chip)) in q.chips.iter().enumerate() {
            if q.chips[..n].iter().any(|(_, c)| c == chip) {
                continue;
            }
            let (picks, advance) = picks_on_a_copy(q, |s| Some(if s == *first { i64::MAX } else { worth_of(s) }), &mut ticks);
            let mut total: i64 = picks.iter().map(|&s| worth_of(s)).sum();
            if let Some(advance) = advance {
                // (The Program Advance's chips picked first, in the order that formed it, then the rest: its worth
                // in their place.)
                let recipe = &picks[advance.start as usize..(advance.start + advance.len) as usize];
                let order: Vec<u8> = recipe.iter().chain(picks.iter().filter(|s| !recipe.contains(s))).copied().collect();
                let worth = self.chip_worth(q, &order, &mut ticks);
                let rest: i64 = picks.iter().filter(|s| !recipe.contains(s)).map(|&s| worth_of(s)).sum();
                if worth + rest > total {
                    total = worth + rest;
                    if best.as_ref().is_none_or(|(_, b)| total > *b) {
                        best = Some((order, total));
                    }
                    continue;
                }
            }
            if best.as_ref().is_none_or(|(_, b)| total > *b) {
                best = Some((picks, total));
            }
        }
        // (The order as worths by slot: the first pick the highest; a chip not picked, below 0.)
        let order = best.map_or_else(Vec::new, |(order, _)| order);
        let ranks = q.chips.iter().map(|(s, _)| order.iter().position(|o| o == s).map_or(-1, |at| (order.len() - at) as i64)).collect();
        Answer { id: q.id, tried: Vec::new(), worths: ranks, ticks, took: start.elapsed() }
    }

    /// What its side takes away in the fight the custom screen `q` shows
    /// opens on, with the chips in `slots` picked in that order (and both
    /// screens through OK, the other player picking nothing): the hand's
    /// first chip used from where its navi stands or a step or two away
    /// (the most of those), in [`WORTH_HORIZON`] ticks, on average over the
    /// guesses at the other player.
    fn chip_worth(&mut self, q: &Question, slots: &[u8], ticks: &mut u64) -> i64 {
        let (side, level) = (q.side, q.level);
        let fighting = |b: &Battle| fight_goes_on(b) && b.fight.state == nettai_battle::battle::fight::FIGHTING;
        // (These chips picked and OK, the other screen's OK: START, then A, a press each other tick; then the
        // turn's start, till the fight takes buttons a few ticks on end: as the screen closes the fight reads as it
        // was, and the turn's start banner comes after.)
        let mut b = (*q.battle).clone();
        let mut picker = Picker::default();
        let in_order = |s: u8| Some(slots.iter().position(|o| *o == s).map_or(-1, |at| (slots.len() - at) as i64));
        let (mut t, mut on) = (0, 0);
        while on < SETTLED && t < TO_THE_FIGHT {
            let custom = b.round.mode == mode::CUSTOM;
            let mine = if custom { picker.buttons(&b, side as usize, Some(&in_order)) } else { 0 };
            let theirs = if custom { [keys::START, 0, keys::A, 0][t as usize % 4] } else { 0 };
            step(&mut b, side, mine, theirs);
            on = if fighting(&b) { on + 1 } else { 0 };
            t += 1;
        }
        *ticks += t as u64;
        if !fighting(&b) || b.hands[side as usize].remaining() == 0 {
            return 0;
        }
        // (From each place a step or two away, one way there: where it can be by the time it uses it.)
        let plans: Vec<Exec> = plans(true, &DIRS).into_iter().filter(|p| p.then == Then::Wait).map(|p| Exec::new(Plan { then: Then::Chip, ..p }, level.charges)).collect();
        let aheads = (0..GUESSES).map(|k| (k, b.clone(), 1)).collect();
        let job = Job { aheads, plans, side, theirs: 0, waited: 0, horizon: WORTH_HORIZON, level, warm: false, dealt_only: true };
        let (dealt, t) = self.run(&job);
        *ticks += t;
        dealt.into_iter().max().unwrap_or(0)
    }
}

/// The picks its side makes on a copy of the custom screen `q` shows, by
/// `worth` (the picker's: the higher first, none below 0), till the cursor
/// goes to OK; and the Program Advance they form there, if they do.
fn picks_on_a_copy(q: &Question, worth: impl Fn(u8) -> Option<i64>, ticks: &mut u64) -> (Vec<u8>, Option<nettai_battle::custom::builder::FormedAdvance>) {
    let side = q.side as usize;
    let mut b = (*q.battle).clone();
    let mut picker = Picker::default();
    let screen = |b: &Battle| b.custom.sides[side].screen.clone();
    for _ in 0..PICKING {
        let Some(shown) = screen(&b).filter(|s| s.phase == Phase::Choosing && b.round.mode == mode::CUSTOM) else { break };
        if shown.cursor == OK_SLOT && shown.selected > 0 || shown.selected as usize >= MAX_SELECTIONS {
            break;
        }
        let mine = picker.buttons(&b, side, Some(&worth));
        if mine == keys::START {
            break;
        }
        step(&mut b, q.side, mine, 0);
        *ticks += 1;
    }
    let Some(shown) = screen(&b) else { return (Vec::new(), None) };
    (shown.selection[..shown.selected as usize].to_vec(), shown.program_advance)
}

/// Where decisions are worked out: a thread of the fighter's own (with the
/// content's runtime of its own, as every thread that runs a battle has,
/// and helpers with theirs), or, where there are no threads (the web), the
/// caller's; and the answers come but not yet asked for (a decision and a
/// custom screen's chips may both be under way), kept by question.
struct Thinker {
    how: How,
    kept: Vec<Answer>,
    /// Questions up to this one were asked before the set started over: their
    /// answers are dropped.
    forgotten: u64,
}

enum How {
    Inline(Box<Search>),
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
                return Thinker { how: How::Thread { ask, answers }, kept: Vec::new(), forgotten: 0 };
            }
        }
        let _ = threads;
        Thinker::inline()
    }

    /// On the caller's thread.
    fn inline() -> Thinker {
        Thinker { how: How::Inline(Box::default()), kept: Vec::new(), forgotten: 0 }
    }

    /// Warm its threads (none inline: there's nothing to warm).
    fn warm(&mut self, q: Question) {
        #[cfg(not(target_arch = "wasm32"))]
        if let How::Thread { ask, .. } = &self.how {
            ask.send(q).expect("the fighter's thread is there");
        }
        #[cfg(target_arch = "wasm32")]
        let _ = q;
    }

    fn ask(&mut self, q: Question) {
        match &mut self.how {
            How::Inline(search) => self.kept.push(search.answer(&q)),
            #[cfg(not(target_arch = "wasm32"))]
            How::Thread { ask, .. } => ask.send(q).expect("the fighter's thread is there"),
        }
    }

    /// Drop every answer to a question up to `asked`, come or to come.
    fn forget(&mut self, asked: u64) {
        self.forgotten = asked;
        self.kept.clear();
    }

    /// The answer to question `id` if it has come.
    fn poll(&mut self, id: u64) -> Option<Answer> {
        #[cfg(not(target_arch = "wasm32"))]
        if let How::Thread { answers, .. } = &self.how {
            while let Ok(a) = answers.try_recv() {
                if a.id > self.forgotten {
                    self.kept.push(a);
                }
            }
        }
        let at = self.kept.iter().position(|a| a.id == id)?;
        Some(self.kept.swap_remove(at))
    }

    /// The answer to question `id`, waiting for it if need be; whether it
    /// was waited for.
    fn answer(&mut self, id: u64) -> (Answer, bool) {
        if let Some(a) = self.poll(id) {
            return (a, false);
        }
        match &self.how {
            How::Inline(_) => panic!("question {id} was never asked"),
            #[cfg(not(target_arch = "wasm32"))]
            How::Thread { answers, .. } => loop {
                let a = answers.recv().expect("the fighter's thread is there");
                if a.id == id {
                    return (a, true);
                }
                if a.id > self.forgotten {
                    self.kept.push(a);
                }
            },
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
    /// What the other player held the tick before, its own HP then, and
    /// whether it has seen an attack come since it last decided
    /// ([`Level::alert`]).
    last_theirs: u16,
    last_hp: Option<i64>,
    alerted: bool,
    /// It pressed L the tick before (to let go between presses), and a copy
    /// of the battle to see whether L opens the custom screen.
    l_pressed: bool,
    probe: Option<Box<Battle>>,
    /// Its custom screen's: its presses, and the chips dealt with what each is
    /// worth ([`Level::chooses`]).
    picker: Picker,
    chosen: Option<Chosen>,
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
        Fighter::with(level, seed, Thinker::inline())
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
            last_theirs: 0,
            last_hp: None,
            alerted: false,
            l_pressed: false,
            probe: None,
            picker: Picker::default(),
            chosen: None,
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
        (self.last_theirs, self.last_hp, self.alerted) = (0, None, false);
        self.picker = Picker::default();
        self.chosen = None;
        self.thinker.forget(self.asked);
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
            self.thinker.warm(Question { id: self.asked, battle: Box::new(b.clone()), now: self.now, theirs, side, level: self.level, warm: true, chips: Vec::new() });
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
        // (An attack seen coming, or a hit taken: a decision as soon as none is pending.)
        let pressed = theirs & !std::mem::replace(&mut self.last_theirs, theirs);
        let hp_now = hp(b, side);
        let hit = self.last_hp.replace(hp_now).is_some_and(|was| hp_now < was);
        self.alerted |= self.level.alert && (pressed & (keys::A | keys::B) != 0 || hit);
        if (self.since >= self.level.every || self.alerted) && self.pending.is_none() {
            self.since = 0;
            self.alerted = false;
            self.asked += 1;
            self.pending = Some((self.asked, self.ticks + self.level.reaction));
            let q = Question { id: self.asked, battle: Box::new(b.clone()), now: self.now, theirs, side, level: self.level, warm: false, chips: Vec::new() };
            self.thinker.ask(q);
            self.take_decision();
        }
        self.now.buttons(b, side)
    }

    /// Its buttons on its custom screen: the chips worth most first, the
    /// worths of those dealt asked as the screen shows (nothing is pressed
    /// till they come); or, at a level that doesn't choose, every chip it
    /// can in the cursor's order.
    fn choose(&mut self, b: &Battle, side: u8) -> u16 {
        let s = &b.custom.sides[side as usize];
        let shown = s.screen.as_ref().filter(|x| b.round.mode == mode::CUSTOM && x.phase == Phase::Choosing && s.in_custom);
        let (Some(screen), true) = (shown, self.level.chooses) else {
            if shown.is_none() {
                self.chosen = None;
            }
            return self.picker.buttons(b, side as usize, None);
        };
        if self.chosen.is_none() {
            let dealt: Vec<(u8, FolderChip)> = screen
                .slots
                .iter()
                .enumerate()
                .filter_map(|(at, slot)| match slot.kind {
                    SlotKind::Chip { index, .. } => s.folder.chips.get(index as usize).copied().flatten().map(|chip| (at as u8, chip)),
                    _ => None,
                })
                .collect();
            self.asked += 1;
            self.thinker.ask(Question { id: self.asked, battle: Box::new(b.clone()), now: self.now, theirs: 0, side, level: self.level, warm: false, chips: dealt.clone() });
            self.chosen = Some(Chosen { asked: self.asked, looked: 0, slots: dealt.iter().map(|(at, _)| *at).collect(), worths: None });
        }
        let chosen = self.chosen.as_mut().expect("asked");
        if chosen.worths.is_none() {
            // (It looks at the chips a moment, as a player does, and then has their worths: waited for if they
            // haven't come, so its presses are the same however fast its threads are.)
            chosen.looked += 1;
            if chosen.looked < LOOKS {
                return 0;
            }
            let waiting = Instant::now();
            let (answer, late) = self.thinker.answer(chosen.asked);
            if late {
                self.spent.late += 1;
                self.spent.waited += waiting.elapsed();
            }
            self.spent.chips += 1;
            self.spent.ticks += answer.ticks;
            self.spent.chip_time += answer.took;
            chosen.worths = Some(answer.worths);
        }
        let worths = chosen.worths.as_deref().expect("come");
        let worth = |slot: u8| chosen.slots.iter().position(|at| *at == slot).map(|at| worths[at]);
        self.picker.buttons(b, side as usize, Some(&worth))
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
        self.choose(b, side)
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

/// A custom screen's chips dealt (by slot) and what each is worth, once
/// that has come (its question, asked as the screen showed).
struct Chosen {
    asked: u64,
    /// The ticks it has looked at them.
    looked: u32,
    slots: Vec<u8>,
    worths: Option<Vec<i64>>,
}

/// A custom screen's buttons that pick every chip they can, then OK: the
/// worthiest first, by a worth given for each slot (none worth less than
/// nothing), else in the order the cursor reaches them. Only the folder's chips dealt (no button, no chip
/// offered from outside it, no form).
#[derive(Clone, Copy, Debug, Default)]
pub struct Picker {
    /// What it holds, and for how many ticks more.
    held: u16,
    more: u8,
}

impl Picker {
    /// `worth`: what the chip in a slot is worth, if it is known.
    pub fn buttons(&mut self, b: &Battle, side: usize, worth: Option<&dyn Fn(u8) -> Option<i64>>) -> u16 {
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
        let key = pick(screen, &b.game_rules().custom_screen, worth);
        // (A direction moves the cursor on its second tick held: the screen's auto-repeat.)
        self.more = (key & (keys::UP | keys::DOWN | keys::LEFT | keys::RIGHT) != 0) as u8;
        self.held = key;
        key
    }
}

/// The press that picks the next chip (the worthiest, by `worth`, of
/// those it can pick; else the nearest), or goes to OK and presses it.
fn pick(screen: &Screen, layout: &CustomScreenLayout, worth: Option<&dyn Fn(u8) -> Option<i64>>) -> u16 {
    let wanted = |i: u8| {
        let slot = &screen.slots[i as usize];
        matches!(slot.kind, SlotKind::Chip { .. }) && slot.state == SlotState::Selectable
    };
    if (screen.selected as usize) < MAX_SELECTIONS {
        // (A chip worth less than nothing isn't picked.)
        let target = worth.map(|worth| (0..screen.slots.len() as u8).filter(|&i| wanted(i) && worth(i).unwrap_or(0) >= 0).max_by_key(|&i| (worth(i).unwrap_or(0), std::cmp::Reverse(i))));
        let wanted = |i: u8| match target {
            Some(target) => target == Some(i),
            None => wanted(i),
        };
        if wanted(screen.cursor) {
            return keys::A;
        }
        if let Some(dir) = toward(screen, layout, wanted) {
            return dir;
        }
    }
    if screen.cursor != OK_SLOT { keys::START } else { keys::A }
}

/// The first direction of the shortest way the cursor goes to a slot
/// `wanted` takes, by the screen's own moves (the engine's: each game's
/// layout).
fn toward(screen: &Screen, layout: &CustomScreenLayout, wanted: impl Fn(u8) -> bool) -> Option<u16> {
    const SLOTS: usize = nettai_battle::custom::screen::SLOTS;
    let keys = [(keys::UP, ScreenKey::Up), (keys::DOWN, ScreenKey::Down), (keys::LEFT, ScreenKey::Left), (keys::RIGHT, ScreenKey::Right)];
    let mut first: [Option<u16>; SLOTS] = [None; SLOTS];
    let mut queue = std::collections::VecDeque::from([screen.cursor]);
    let mut seen = [false; SLOTS];
    seen[screen.cursor as usize] = true;
    while let Some(at) = queue.pop_front() {
        for (dir, key) in keys {
            let Some(to) = screen.target_from(at, key, layout).filter(|t| (*t as usize) < SLOTS && !seen[*t as usize]) else { continue };
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

