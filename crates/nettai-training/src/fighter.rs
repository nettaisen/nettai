//! An opponent that fights: it tries plans in copies of the battle and
//! follows the one that comes out best. Live play's [`Ai`] (its
//! `InFight::Ai` and `OnCustomScreen::Ai`).
//!
//! It knows no game. A plan is buttons (a step or two, then a chip, a shot,
//! a charged shot or nothing), and what a plan is worth is what the
//! battle's own simulation makes of it a second or so on: the HP each navi
//! has then. Every chip of every game's content is played as the content
//! plays it, with nothing about any of them here. It presses only buttons,
//! as a player does, and asks the engine for nothing else; a battle is
//! plain data, so a copy to try a plan in is a clone (as a rollback's
//! snapshot is).
//!
//! What makes it a partner to train against rather than one that sees the
//! future is its [`Level`]: it acts on what it decided a reaction time ago,
//! it picks among the plans near the best rather than always the best, and
//! it looks only so far ahead. Its picks are seeded: the same battle, seed
//! and level give the same presses.

use std::time::{Duration, Instant};

use nettai_battle::Battle;
use nettai_battle::battle::{battle_flags, mode};
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
}

impl Level {
    pub const EASY: Level = Level { reaction: 24, horizon: 40, every: 16, blunders: 35 };
    pub const NORMAL: Level = Level { reaction: 14, horizon: 50, every: 12, blunders: 15 };
    pub const HARD: Level = Level { reaction: 6, horizon: 60, every: 8, blunders: 0 };
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
    /// What it pressed the tick before (a press is a button not held then).
    last: u16,
}

impl Exec {
    fn new(plan: Plan) -> Exec {
        Exec { plan, step: 0, held: 0, from: None, done: false, after: 0, trial: false, last: 0 }
    }

    /// Its buttons this tick, for the navi of `side` in `b`.
    fn buttons(&mut self, b: &Battle, side: u8) -> u16 {
        let pressed = self.next(b, side);
        self.last = pressed;
        pressed
    }

    fn next(&mut self, b: &Battle, side: u8) -> u16 {
        let Some(r) = b.player(side) else { return 0 };
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
                    return 0;
                }
                continue;
            }
            self.held += 1;
            return dir;
        }
        if self.done {
            self.after += 1;
            return if self.trial && self.after % TRIAL_SHOTS == 0 { keys::B } else { 0 };
        }
        let tap = |e: &mut Exec, button: u16| {
            if e.last & button != 0 {
                return 0;
            }
            e.done = true;
            button
        };
        match self.plan.then {
            Then::Wait => {
                self.done = true;
                0
            }
            Then::Chip => tap(self, keys::A),
            Then::Shot => tap(self, keys::B),
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
    let them = side ^ 1;
    let deleted = |s: u8| if after.is_battle_over() && hp(after, s) == 0 { DELETED } else { 0 };
    ((hp(before, them) - hp(after, them)) * DEALT + deleted(them), (hp(before, side) - hp(after, side)) * TAKEN + deleted(side))
}

/// What the other player may do, as a plan is tried: by the ticks since
/// the decision, from what they hold now. They go on holding it a moment,
/// or step up, or step down. A plan is worth what it takes away on average
/// over them, less the most it loses in any.
const GUESSES: usize = 3;

fn guess(k: usize, theirs: u16, t: u32) -> u16 {
    match k {
        0 if t < PLAYER_HOLDS => theirs,
        1 if t < 2 => keys::UP,
        2 if t < 2 => keys::DOWN,
        _ => 0,
    }
}

/// What deciding has cost so far.
#[derive(Clone, Copy, Debug, Default)]
pub struct Spent {
    pub decisions: u32,
    /// The ticks run in copies of the battle.
    pub ticks: u64,
    pub time: Duration,
    /// The longest decision.
    pub longest: Duration,
}

/// An opponent that fights, for the navi of one side.
pub struct Fighter {
    level: Level,
    seed: u64,
    rng: SplitMix64,
    /// What it does now, and what it decided to do next and from which of
    /// its ticks.
    now: Exec,
    next: Option<(u32, Exec)>,
    /// Its ticks, and the ticks of the fight since it last decided.
    ticks: u32,
    since: u32,
    /// Its last press of L or R, to let go between them.
    asked: bool,
    /// Its custom screen's.
    picker: Picker,
    /// Copies of the battle that plans are tried in, one for each guess at
    /// the other player as the decision takes effect (kept for their
    /// memory).
    aheads: Vec<Battle>,
    trial: Option<Battle>,
    pub spent: Spent,
}

impl Fighter {
    pub fn new(level: Level, seed: u64) -> Fighter {
        Fighter {
            level,
            seed,
            rng: SplitMix64::new(seed),
            now: Exec::new(Plan { steps: [0, 0], then: Then::Wait }),
            next: None,
            ticks: 0,
            since: 0,
            asked: false,
            picker: Picker::default(),
            aheads: Vec::new(),
            trial: None,
            spent: Spent::default(),
        }
    }

    /// As it was new: the set starts over.
    pub fn reset(&mut self) {
        let spent = self.spent;
        *self = Fighter { spent, ..Fighter::new(self.level, self.seed) };
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
        self.take_next();
        if !fight_goes_on(b) || b.is_dimmed() {
            return self.now.buttons(b, side);
        }
        // (The custom screen, with nothing in hand and the gauge full: L, let go between presses.)
        let empty = b.hands[side as usize].remaining() == 0;
        if empty && b.round.flags & battle_flags::GAUGE_FULL != 0 && self.now.done {
            self.asked = !self.asked;
            return if self.asked { keys::L } else { 0 };
        }
        self.since += 1;
        if self.since >= self.level.every && self.next.is_none() {
            self.since = 0;
            self.decide(b, side, theirs);
            self.take_next();
        }
        self.now.buttons(b, side)
    }

    /// The plan decided on, once its tick has come.
    fn take_next(&mut self) {
        if let Some((_, exec)) = self.next.filter(|(from, _)| *from <= self.ticks) {
            self.now = exec;
            self.next = None;
        }
    }

    /// Decide what to do a reaction time from now: from the battle as the
    /// plan it follows and the player's holding would leave it then, try
    /// each plan, and the plan it follows going on, and pick.
    fn decide(&mut self, b: &Battle, side: u8, theirs: u16) {
        let start = Instant::now();
        let level = self.level;
        let mut ticks = 0u64;
        // Where the battle will be as it takes effect, by each guess.
        self.aheads.resize_with(GUESSES, || b.clone());
        let mut waited = 0;
        for (k, ahead) in self.aheads.iter_mut().enumerate() {
            ahead.clone_from(b);
            let mut now = self.now;
            let mut t = 0;
            while t < level.reaction && fight_goes_on(ahead) {
                let mine = now.buttons(ahead, side);
                step(ahead, side, mine, guess(k, theirs, t));
                t += 1;
            }
            ticks += t as u64;
            waited = t;
        }
        let chip = self.aheads[0].hands[side as usize].remaining() > 0;
        let mut trial = self.trial.take().unwrap_or_else(|| b.clone());
        // (The directions the navi can step in as it takes effect: each held in a copy until it moves, or not.)
        let mut steps = Vec::new();
        for dir in DIRS {
            trial.clone_from(&self.aheads[0]);
            let from = trial.player(side).map(|r| trial.objects.get(r).panel);
            let mut t = 0;
            while t < STEP_MOST as u32 && fight_goes_on(&trial) {
                step(&mut trial, side, dir, guess(0, theirs, waited + t));
                t += 1;
                if trial.player(side).map(|r| trial.objects.get(r).panel) != from {
                    steps.push(dir);
                    break;
                }
            }
            ticks += t as u64;
        }
        let candidates = plans(chip, &steps).into_iter().map(Exec::new).chain(std::iter::once(self.now));
        let mut tried: Vec<(Exec, i64)> = Vec::new();
        for start in candidates {
            let (mut dealt, mut taken) = (0, 0);
            for (k, ahead) in self.aheads.iter().enumerate() {
                trial.clone_from(ahead);
                let mut exec = Exec { trial: true, ..start };
                let mut t = 0;
                while t < level.horizon && fight_goes_on(&trial) {
                    let mine = exec.buttons(&trial, side);
                    step(&mut trial, side, mine, guess(k, theirs, waited + t));
                    t += 1;
                }
                ticks += t as u64;
                let (d, l) = outcome(ahead, &trial, side);
                dealt += d;
                taken = taken.max(l);
            }
            tried.push((start, dealt - taken * GUESSES as i64));
        }
        let best = tried.iter().map(|(_, w)| *w).max().unwrap_or(0);
        // (The plan it follows, not yet done and as good as the best, goes on: no change of mind for nothing.)
        let following = tried.last().filter(|(e, w)| !e.done && *w >= best).map(|(e, _)| *e);
        let picked = if level.blunders > 0 && self.rng.below(100) < level.blunders as u64 {
            tried[self.rng.below(tried.len() as u64) as usize].0
        } else if let Some(e) = following {
            e
        } else {
            let bests: Vec<Exec> = tried.iter().filter(|(_, w)| *w == best).map(|(e, _)| *e).collect();
            bests[self.rng.below(bests.len() as u64) as usize]
        };
        // (The plan it follows, picked again, goes on from where it is.)
        let picked = if picked.plan == self.now.plan && !self.now.done { self.now } else { picked };
        self.next = Some((self.ticks + level.reaction, picked));
        self.trial = Some(trial);
        let took = start.elapsed();
        self.spent.decisions += 1;
        self.spent.ticks += ticks;
        self.spent.time += took;
        self.spent.longest = self.spent.longest.max(took);
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

/// One tick of `b` with side `side` pressing `mine` and the other `theirs`.
fn step(b: &mut Battle, side: u8, mine: u16, theirs: u16) {
    let buttons = if side == 0 { [mine, theirs] } else { [theirs, mine] };
    let input = tick_input(b, buttons);
    b.tick(&input.players, input.events);
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
    }

    /// The first round of `m` stepped by hand: the left navi pressing at
    /// random, the right one `dummy`; up to `most` ticks.
    fn first_round(content: &Arc<Content>, m: &nettai_match::Match, seed: u32, dummy: &mut dyn Ai, most: u32) -> Round {
        let mut b = Set::of(content, m, seed).start();
        let mut player = RandomPresses::new(seed as u64 ^ 0x5EED);
        let (mut ticks, mut decided) = (0, Decided { started: false });
        while ticks < most && !decided.now(&b) && !b.is_stopped() {
            ticks += 1;
            let [left, right] = both(&b, [&mut player, dummy]);
            step(&mut b, 0, left, right);
        }
        assert!(!b.is_stopped(), "the engine stopped");
        let hp = [hp(&b, 0), hp(&b, 1)];
        let won = match hp {
            [0, 0] => None,
            [0, _] => Some(1),
            [_, 0] => Some(0),
            _ => None,
        };
        Round { won, hp, ticks }
    }

    /// Plugged into live play as its opponent's AI, in the fight and on
    /// the custom screen, it plays as it does by hand: the same battle,
    /// tick for tick (and the same again after the set starts over). It hit
    /// the player at all.
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
        let mut fighter = Fighter::new(Level::HARD, 3);
        for (tick, digest) in live_digests.iter().enumerate() {
            let [left, right] = both(&b, [&mut player, &mut fighter]);
            step(&mut b, 0, left, right);
            assert_eq!(b.digest(), *digest, "tick {tick}");
        }
    }

    /// The opponent that fights at each level against a player pressing at
    /// random, and one that presses at random for comparison: the first
    /// rounds of each game's matches, both sides the same build. Prints
    /// its wins and what deciding cost. A measurement: `cargo test -p
    /// nettai-training --lib against_random_presses -- --ignored --nocapture`
    /// (`FIGHTER_THREADS`, `FIGHTER_SEEDS`, `FIGHTER_GAMES`).
    #[test]
    #[ignore]
    fn against_random_presses() {
        const HP: i64 = 400;
        const MOST: u32 = 60 * 60 * 3;
        let var = |name: &str, or: usize| std::env::var(name).ok().and_then(|t| t.parse().ok()).unwrap_or(or);
        let (threads, seeds) = (var("FIGHTER_THREADS", 6), var("FIGHTER_SEEDS", 12) as u32);
        let games = std::env::var("FIGHTER_GAMES").unwrap_or_else(|_| "exe6,exe5,exe4".into());
        let runs: Vec<(&str, &str, Option<Level>)> = games
            .split(',')
            .flat_map(|game| [(game, "random", None), (game, "easy", Some(Level::EASY)), (game, "normal", Some(Level::NORMAL)), (game, "hard", Some(Level::HARD))])
            .collect();
        println!("| game | dummy | dummy won | player won | undecided | HP left (dummy / player) | ticks a round | decisions a round | ms a decision (mean / longest) | trial ticks a decision | µs a trial tick |");
        println!("|---|---|---|---|---|---|---|---|---|---|---|");
        for (game, name, level) in runs {
            let content = content(game);
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
            let n = results.len() as f64;
            let count = |won: Option<u8>| results.iter().filter(|(r, _)| r.won == won).count();
            let mean = |f: &dyn Fn(&(Round, Spent)) -> f64| results.iter().map(f).sum::<f64>() / n;
            let decisions: u64 = results.iter().map(|(_, s)| s.decisions as u64).sum();
            let time: Duration = results.iter().map(|(_, s)| s.time).sum();
            let trial: u64 = results.iter().map(|(_, s)| s.ticks).sum();
            let longest = results.iter().map(|(_, s)| s.longest).max().unwrap_or_default();
            let per = |x: f64| if decisions == 0 { 0.0 } else { x / decisions as f64 };
            println!(
                "| {game} | {name} | {} | {} | {} | {:.0} / {:.0} | {:.0} | {:.0} | {:.2} / {:.1} | {:.0} | {:.2} |",
                count(Some(1)),
                count(Some(0)),
                count(None),
                mean(&|(r, _)| r.hp[1] as f64),
                mean(&|(r, _)| r.hp[0] as f64),
                mean(&|(r, _)| r.ticks as f64),
                decisions as f64 / n,
                per(time.as_secs_f64() * 1000.0),
                longest.as_secs_f64() * 1000.0,
                per(trial as f64),
                if trial == 0 { 0.0 } else { time.as_secs_f64() * 1e6 / trial as f64 },
            );
        }
    }
}
