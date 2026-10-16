//! An opponent that fights: it asks copies of the battle what each place
//! of its area is worth and what it can do from there, and follows the
//! best. Live play's [`Ai`] (its `InFight::Ai` and `OnCustomScreen::Ai`).
//!
//! It knows no game. Everything it knows of one comes from the battle's own
//! simulation: a battle is plain data, so a copy to try something in is a
//! clone (as a rollback's snapshot is), and every chip, form and attack of
//! every game's content is played as the content plays it.
//!
//! **In the fight** a decision tries plans by the places it can reach.
//! First, for each panel of its area it can walk to, the plan "go there by
//! the shortest way and stand": what that loses in the next second or so
//! against a few guesses at the other player (they go on as they are, do
//! again what they did the last second, use their chip or shoot at once,
//! step up or down, walk to its row and attack there) is how dangerous the
//! panel is, attacks already on their way included. Then, from the safest
//! few, "go there and shoot (the charged shot, once the charge is full)"
//! and "go there and use the chip in hand" (B let go, A pressed till the
//! navi takes it), each followed by a retreat to the safest panel where
//! that is clearly safer: what they take away is where its attacks connect.
//! A plan is worth what it takes away on average over the guesses, less
//! what it loses (the most in any and the average, by its caution); a chip
//! whose use takes nothing away is worth nothing, and isn't used for
//! nothing, but for one held a long while (it does what no HP shows, or it
//! would bar the chips behind it). Of plans worth the same the nearest
//! wins, and standing before shooting or a chip, so it keeps its charge and
//! its chips; and it keeps to what it does unless another plan is better by
//! a margin ([`Level::steady`]): no change of mind for nothing, no pacing.
//! A chip's dimming isn't counted against the horizon: a cut-in's attack
//! lands within it.
//!
//! It never opens the custom screen: the player does (live play clears L
//! and R from a computer-played dummy's presses in the fight), so it plans
//! with the hand it has, which may stay empty till then; nothing it tries
//! in a copy presses them either.
//!
//! **On its custom screen** it looks at the chips dealt and picks them by
//! what each is worth in the fight the screen opens on: in a copy, that chip
//! picked, both screens through OK (the other player picking every chip it
//! can), the chip used from each panel it can reach against the guesses,
//! less what standing there without it would make (so a Barrier is worth
//! the hits it stops, a Recover the HP it gives back). A pick can make
//! others unpickable by their codes, so each chip is tried as the first
//! pick, the rest after it by worth, and the picks worth most in all win;
//! where they form a Program Advance on the copy, that is tried in the
//! fight in their place. A chip with no worth of its own may be one the
//! hand folds into the chip picked right before it as OK builds the hand
//! (an attack-up: Atk+10, Navi+20; the engine's `ChipHand::fold_next`),
//! never used alone: it is tried picked right after each of the two
//! worthiest attacking picks, the pair worth what the chip they make takes
//! away in the fight, and goes there where that is more than the attacking
//! chip alone. The hand is used in the order picked,
//! the worthiest first (`Level::chooses`; else every chip it can in the
//! cursor's order). The cursor finds its way by the screen's own moves, each
//! game's (`Screen::target_from`).
//!
//! Then the forms ([`Level::forms`]): it walks the screen's keys in copies
//! (the directions, A and B), from the screen as dealt and from it with its
//! picks made, for every way to make the screen bring the turn something
//! besides chips that the game's rules offer there (a form: EXE6's Crosses
//! and Beast Out, set as they are picked; EXE5's and EXE4's souls, a chip
//! given up for the soul button, the form given at OK); each such screen is
//! played through OK and eight seconds of the fight, both navis lining up
//! with each other's row and attacking, and the best is taken where the
//! fight goes better with it than without (by 5 HP).
//!
//! What makes it a partner to train against rather than one that sees the
//! future is its [`Level`]: it acts on what it decided a reaction time ago,
//! weighs only so many guesses at the other player, looks only so far
//! ahead, and now and then misses what happens (goes on with what it was
//! doing). Normal and Hard decide again as soon as the other player presses
//! A or B or lands a hit, not only on their beat: they see an attack coming.
//! The reaction time is also when it thinks: a decision is worked out on a
//! thread of its own (and helpers) while the battle goes on, and taken on
//! the tick it was due (the battle waits for it if it isn't ready). Its
//! picks are seeded and made on the battle's side: the same battle, seed and
//! level give the same presses, on its thread or not.

use std::collections::{HashSet, VecDeque};
use std::time::{Duration, Instant};

use nettai_battle::Battle;
use nettai_battle::battle::mode;
use nettai_battle::content::{CustomScreenLayout, ScreenKey};
use nettai_battle::custom::screen::{MAX_SELECTIONS, OK_SLOT, Screen};
use nettai_battle::custom::{FolderChip, Phase, SlotKind, SlotState};
use nettai_battle::input::keys;
use nettai_content_api::PanelPos;
use nettai_frontend::driver::Ai;
use nettai_netplay::rng::SplitMix64;
use nettai_netplay::standin::tick_input;

#[cfg(test)]
mod old;

/// How well an opponent that fights plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Level {
    /// Ticks from a decision to its first press: how late it reacts to
    /// what it sees.
    pub reaction: u32,
    /// Ticks of the fight each plan is tried for (a dimming's not counted).
    pub horizon: u32,
    /// Ticks of the fight between decisions.
    pub every: u32,
    /// How many of the guesses at the other player it weighs, in this order:
    /// they go on as they are, do again what they did the last second, use
    /// their chip at once, step up, step down, walk to its row and use their
    /// chip, walk there and shoot, shoot at once (1: it supposes they go on).
    pub guesses: u32,
    /// In a hundred decisions, how many it misses: it goes on with what it
    /// was doing.
    pub blunders: u32,
    /// It decides again at once (a reaction time on, as ever) when the
    /// other player presses A or B, or it loses HP: it sees an attack come,
    /// not only on its beat (`every`).
    pub alert: bool,
    /// In a hundred, how much of what a plan loses is the most it loses in
    /// any guess (the rest the average over them): 100 keeps out of the way
    /// of any attack that may come, 0 risks what is likely.
    pub caution: u32,
    /// In tenths of HP: how much better than what it does another plan must
    /// be for it to change its mind (on average over the guesses), and a
    /// panel safer than the one it attacks from for it to walk there after.
    pub steady: u32,
    /// On its custom screen it picks the chips worth most first (each tried
    /// in a copy of the fight: [`Fighter`]'s docs), the hand used in that
    /// order; else every chip it can, in the order the cursor reaches them.
    pub chooses: bool,
    /// On its custom screen it takes a form the rules offer (a Cross, Beast
    /// Out, a soul) where a copy of the fight goes better with it.
    pub forms: bool,
    /// The ticks it looks at the chips dealt before it presses anything.
    pub looks: u32,
}

impl Level {
    pub const EASY: Level = Level { reaction: 24, horizon: 40, every: 20, guesses: 1, blunders: 30, alert: false, caution: 30, steady: 40, chooses: false, forms: false, looks: 60 };
    pub const NORMAL: Level = Level { reaction: 14, horizon: 50, every: 12, guesses: 5, blunders: 10, alert: true, caution: 40, steady: 30, chooses: true, forms: true, looks: 60 };
    pub const HARD: Level = Level { reaction: 9, horizon: 60, every: 8, guesses: 6, blunders: 0, alert: true, caution: 50, steady: 20, chooses: true, forms: true, looks: 45 };
}

impl Default for Level {
    fn default() -> Level {
        Level::NORMAL
    }
}

/// What a plan does once at its panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Then {
    /// Stand there.
    Hold,
    /// Let go of B, the charged shot once the charge is full.
    Shot,
    /// A: the chip in hand.
    Chip,
}

/// A plan: walk to a panel, do something there, then walk to another (the
/// safest) and stand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Plan {
    to: PanelPos,
    then: Then,
    after: Option<PanelPos>,
}

/// The ticks a step's direction is held for at most, where the navi
/// doesn't move (a panel it can't step on, a hit taking it).
const STEP_MOST: u8 = 20;
/// The ticks a shot waits for the charge to be full at most, and A is held
/// for a chip at most.
const CHARGE_MOST: u16 = 150;
const CHIP_HELD_MOST: u16 = 60;
/// The live ticks a plan is tried for past its horizon at most, the
/// dimmings in it counted (a cut-in's).
const DIMMED_MOST: u32 = 200;
/// The steps it walks in a plan at most.
const STEPS_MOST: u8 = 4;
/// The panels it tries attacking from: the safest few, and where it stands.
const ATTACK_PANELS: usize = 4;

/// A plan being followed: where it is in it.
#[derive(Clone, Copy, Debug)]
struct Exec {
    plan: Plan,
    /// The direction held now (0: none), the ticks it has been, and the
    /// panel it started from.
    dir: u16,
    held: u8,
    from: Option<PanelPos>,
    /// What it does there is done.
    acted: bool,
    /// The ticks it has waited for the charge, or held A; the chips in hand
    /// as it pressed A; and whether the navi didn't take the chip (it is one
    /// the game won't use now).
    waited: u16,
    hand: u8,
    refused: bool,
    /// What it pressed the tick before (a press is a button not held then).
    last: u16,
}

impl Exec {
    fn new(plan: Plan) -> Exec {
        Exec { plan, dir: 0, held: 0, from: None, acted: false, waited: 0, hand: 0, refused: false, last: 0 }
    }

    /// Standing where it is.
    fn stand(at: PanelPos) -> Exec {
        Exec::new(Plan { to: at, then: Then::Hold, after: None })
    }

    /// Its buttons this tick, for the navi of `side` in `b`. B is held all
    /// the while, to charge the buster, as a player does.
    fn buttons(&mut self, b: &Battle, side: u8) -> u16 {
        let pressed = self.next(b, side);
        self.last = pressed;
        pressed
    }

    fn next(&mut self, b: &Battle, side: u8) -> u16 {
        const HELD: u16 = keys::B;
        let Some(r) = b.player(side) else { return HELD };
        let at = b.objects.get(r).panel;
        let target = if self.acted { self.plan.after.unwrap_or(at) } else { self.plan.to };
        if at != target {
            if self.dir != 0 {
                if Some(at) != self.from || self.held >= STEP_MOST {
                    // (Let go a tick: the next step's direction is a press.)
                    self.dir = 0;
                    return HELD;
                }
                self.held += 1;
                return self.dir | HELD;
            }
            if let Some(dir) = toward(b, r, at, target) {
                (self.dir, self.held, self.from) = (dir, 1, Some(at));
                return dir | HELD;
            }
            // (No way there: it does what it would there from here.)
        }
        self.dir = 0;
        if self.acted {
            return HELD;
        }
        match self.plan.then {
            Then::Hold => HELD,
            // (A press: let go first if it's held.)
            // (A chip: B let go, as B held keeps some forms shooting (EXE6's Beast Out), and A pressed; then A held
            // while it charges the chip (EXE6's Crosses that charge chips: it goes as A is let go), else pressed again
            // every other tick, till the chip goes from the hand.)
            Then::Chip if self.waited == 0 && self.last & keys::A != 0 => 0,
            Then::Chip if self.waited == 0 => {
                self.waited = 1;
                self.hand = b.hands[side as usize].remaining();
                keys::A
            }
            Then::Chip => {
                let a_charge = b.player_actor(side).map(|a| b.actors.get(a)).filter(|a| a.charge_source == 1).map_or(0, |a| a.charge_level);
                let used = b.hands[side as usize].remaining() < self.hand;
                if used || a_charge >= 2 || self.waited >= CHIP_HELD_MOST {
                    self.refused = !used && a_charge < 2;
                    self.acted = true;
                    return HELD;
                }
                self.waited += 1;
                if a_charge == 1 || self.last == 0 { keys::A } else { 0 }
            }
            Then::Shot => {
                let full = charge(b, side) >= 2;
                if full || self.waited >= CHARGE_MOST {
                    // (Let go: the charged shot; B is pressed again next tick, charging anew.)
                    self.acted = true;
                    return 0;
                }
                self.waited += 1;
                // (B held with no charge, as after a chip: pressed again.)
                if charge(b, side) == 0 && self.waited % 16 == 0 {
                    return 0;
                }
                HELD
            }
        }
    }
}

/// The charge of side `side`'s buster: 0 none, 1 charging, 2 full.
fn charge(b: &Battle, side: u8) -> u8 {
    b.player_actor(side).map_or(0, |a| b.actors.get(a).charge_level)
}

fn navi_at(b: &Battle, side: u8) -> Option<PanelPos> {
    b.player(side).map(|r| b.objects.get(r).panel)
}

/// A direction key and the panel it steps navi `r` to from `at`: LEFT and
/// RIGHT are back and forward, from its side (the right side's navi's
/// back is to its right: `held_direction`'s codes).
fn stepped(b: &Battle, r: nettai_battle::object::ObjectRef, at: PanelPos, dir: u16) -> Option<PanelPos> {
    let flipped = b.objects.get(r).alliance != 0;
    let (x, y) = match (dir, flipped) {
        (keys::UP, _) => (at.x, at.y.checked_sub(1)?),
        (keys::DOWN, _) => (at.x, at.y + 1),
        (keys::LEFT, false) | (keys::RIGHT, true) => (at.x.checked_sub(1)?, at.y),
        _ => (at.x + 1, at.y),
    };
    Some(PanelPos { x, y })
}

const DIRS: [u16; 4] = [keys::UP, keys::DOWN, keys::LEFT, keys::RIGHT];

/// The first step of a way from `at` to `to` the navi `r` can take: along
/// the longer of the two, else the other.
fn toward(b: &Battle, r: nettai_battle::object::ObjectRef, at: PanelPos, to: PanelPos) -> Option<u16> {
    let flipped = b.objects.get(r).alliance != 0;
    let horizontal = match (to.x.cmp(&at.x), flipped) {
        (std::cmp::Ordering::Less, false) | (std::cmp::Ordering::Greater, true) => Some(keys::LEFT),
        (std::cmp::Ordering::Equal, _) => None,
        _ => Some(keys::RIGHT),
    };
    let vertical = match to.y.cmp(&at.y) {
        std::cmp::Ordering::Less => Some(keys::UP),
        std::cmp::Ordering::Greater => Some(keys::DOWN),
        _ => None,
    };
    let order = if at.x.abs_diff(to.x) >= at.y.abs_diff(to.y) { [horizontal, vertical] } else { [vertical, horizontal] };
    order.into_iter().flatten().find(|&dir| stepped(b, r, at, dir).is_some_and(|p| b.can_step(r, p.x, p.y)))
}

/// The panels side `side`'s navi can walk to, by their steps from where it
/// stands (it first, then by steps), [`STEPS_MOST`] at most.
fn reachable(b: &Battle, side: u8) -> Vec<(PanelPos, u8)> {
    let Some(r) = b.player(side) else {
        return Vec::new();
    };
    let start = b.objects.get(r).panel;
    let mut out = vec![(start, 0u8)];
    let mut queue = VecDeque::from([(start, 0u8)]);
    while let Some((at, steps)) = queue.pop_front() {
        if steps >= STEPS_MOST {
            continue;
        }
        for dir in DIRS {
            let Some(p) = stepped(b, r, at, dir) else {
                continue;
            };
            if out.iter().any(|(q, _)| *q == p) || !b.can_step(r, p.x, p.y) {
                continue;
            }
            out.push((p, steps + 1));
            queue.push_back((p, steps + 1));
        }
    }
    out
}

/// Whether a plan tried in `b` goes on: the fight (dimmed or not) of a
/// round not yet decided, not in the game's pause.
fn fight_goes_on(b: &Battle) -> bool {
    b.round.mode == mode::FIGHTING && !b.is_battle_over() && b.paused_by().is_none() && !b.is_stopped()
}

fn hp(b: &Battle, side: u8) -> i64 {
    b.player(side).map_or(0, |r| b.objects.get(r).hp as i64)
}

/// What a plan is worth, in tenths of HP: an HP it takes away, and one it
/// loses (losing counts a little more: it would rather not trade).
const DEALT: i64 = 10;
const TAKEN: i64 = 12;
/// A navi deleted, worth more than any HP.
const DELETED: i64 = 1_000_000;

/// What a trial did, for side `side`, from the battle `before` to `after`:
/// the HP it took away and the HP it lost (less what it got back), in
/// tenths of HP (a navi deleted counts as [`DELETED`]).
fn outcome(before: &Battle, after: &Battle, side: u8) -> (i64, i64) {
    // (A battle the engine stopped is the worst: the session ends there.)
    if after.is_stopped() {
        return (0, DELETED);
    }
    let them = side ^ 1;
    let deleted = |s: u8| {
        if after.is_battle_over() && hp(after, s) == 0 { DELETED } else { 0 }
    };
    ((hp(before, them) - hp(after, them)) * DEALT + deleted(them), (hp(before, side) - hp(after, side)) * TAKEN + deleted(side))
}

/// What the other player may do, as a plan is tried, in the order a level
/// takes them ([`Level::guesses`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Guess {
    /// They go on holding what they hold a moment, then let go.
    Holds,
    /// They do again what they did over the last [`PAST`] ticks of the
    /// fight, press for press (a player shooting every so often, or
    /// pacing, goes on so).
    Repeats,
    /// They use their next chip at once.
    Chip,
    /// They walk to the navi's row (where it is as they go) and use their
    /// chip there.
    TrackChip,
    /// They shoot at once.
    Shot,
    /// They walk to the navi's row and shoot there.
    TrackShot,
    Up,
    Down,
}

const GUESSES: [Guess; 8] = [Guess::Holds, Guess::Repeats, Guess::Chip, Guess::Up, Guess::Down, Guess::TrackChip, Guess::TrackShot, Guess::Shot];

/// The ticks of the fight the other player's presses are kept for
/// ([`Guess::Repeats`]).
const PAST: usize = 60;

/// What the other player pressed, the last [`PAST`] ticks of the fight
/// (the oldest first), but the custom screen's keys (L and R open it,
/// which a guess doesn't: the player decides that), START and SELECT.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Past([u16; PAST]);

impl Past {
    const KEPT: u16 = !(keys::L | keys::R | keys::START | keys::SELECT);

    fn push(&mut self, held: u16) {
        self.0.copy_within(1.., 0);
        self.0[PAST - 1] = held & Past::KEPT;
    }
}

/// The ticks it supposes the player goes on holding what they hold now.
const PLAYER_HOLDS: u32 = 8;

/// A guess being played: the other player's buttons, tick by tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Guessed {
    guess: Guess,
    /// What they held as the decision was asked, and before.
    theirs: u16,
    past: Past,
    /// The ticks since the decision.
    t: u32,
    /// A walking guess: the direction held, its ticks, and whether it has
    /// attacked.
    dir: u16,
    held: u8,
    fired: bool,
    last: u16,
}

impl Guessed {
    fn new(guess: Guess, theirs: u16, past: Past) -> Guessed {
        Guessed { guess, theirs: theirs & Past::KEPT, past, t: 0, dir: 0, held: 0, fired: false, last: 0 }
    }

    /// Their buttons this tick, on side `them` of `b`.
    fn buttons(&mut self, b: &Battle, them: u8) -> u16 {
        let t = self.t;
        self.t += 1;
        let pressed = match self.guess {
            Guess::Holds if t < PLAYER_HOLDS => self.theirs,
            Guess::Repeats => self.past.0[t as usize % PAST],
            Guess::Up if t < 2 => keys::UP,
            Guess::Down if t < 2 => keys::DOWN,
            // (Let go first: what they hold now isn't a press.)
            Guess::Chip if t == 1 => keys::A,
            Guess::Shot if t == 1 => keys::B,
            Guess::TrackChip | Guess::TrackShot if !self.fired => self.track(b, them),
            _ => 0,
        };
        self.last = pressed;
        pressed
    }

    fn track(&mut self, b: &Battle, them: u8) -> u16 {
        let (Some(at), Some(navi)) = (navi_at(b, them), navi_at(b, them ^ 1)) else {
            return 0;
        };
        if at.y != navi.y {
            if self.dir != 0 && self.held < 12 {
                self.held += 1;
                return self.dir;
            }
            if self.dir != 0 || self.last != 0 {
                self.dir = 0;
                return 0;
            }
            self.dir = if navi.y < at.y { keys::UP } else { keys::DOWN };
            self.held = 1;
            return self.dir;
        }
        if self.last != 0 {
            self.dir = 0;
            return 0;
        }
        self.fired = true;
        if self.guess == Guess::TrackChip { keys::A } else { keys::B }
    }

    /// Whether it presses nothing from now on, whatever the battle: two
    /// such guesses from the same battle play out alike.
    fn idle(&self) -> bool {
        match self.guess {
            Guess::Holds => self.t >= PLAYER_HOLDS || self.theirs == 0,
            Guess::Repeats => self.past.0.iter().all(|p| *p == 0),
            Guess::Up | Guess::Down | Guess::Chip | Guess::Shot => self.t >= 2,
            Guess::TrackChip | Guess::TrackShot => self.fired && self.last == 0,
        }
    }
}

/// The threads a fighter tries its plans on at most ([`Fighter::new`]):
/// its own and helpers.
pub const THREADS: usize = 4;

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
    /// The forms it took on its custom screens.
    pub forms: u32,
}

/// What a question asks.
#[derive(Clone)]
enum Asked {
    /// No decision, only its threads warmed: each steps a copy of the
    /// battle a tick (the first battle a thread runs loads the content's
    /// runtime there).
    Warm,
    /// A decision in the fight.
    Decide,
    /// What each of these chips, dealt on the custom screen shown (by
    /// slot), is worth in the fight it opens on ([`Search::worths`]), and
    /// the form to take.
    Chips(Vec<(u8, FolderChip)>),
}

/// A decision to work out: the battle as it stands, the plan it follows, and
/// what the other player holds.
struct Question {
    id: u64,
    battle: Box<Battle>,
    now: Exec,
    theirs: u16,
    past: Past,
    /// The live ticks past [`STALE`] the chip in hand has gone unused.
    stale: u32,
    side: u8,
    level: Level,
    asked: Asked,
}

/// What each plan tried is worth, nearest first (the plan followed last),
/// and what working it out cost.
struct Answer {
    id: u64,
    /// The plans, and the guesses each worth is the sum over.
    tried: Vec<(Exec, i64)>,
    guesses: i64,
    /// The chips' worths, of a question of them, and the form to take.
    worths: Vec<i64>,
    form: Option<FormChoice>,
    ticks: u64,
    took: Duration,
}

/// A form to take on the custom screen: the buttons that take it, tick by
/// tick, from the screen as dealt (`before`: then the picks) or with the
/// picks made.
#[derive(Clone, Debug)]
struct FormChoice {
    script: Vec<u16>,
    before: bool,
}

/// How a job's plans are worth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Worth {
    /// What it takes away on average over the guesses, less what it loses
    /// (the most and the average, by the caution).
    Fight { caution: u32 },
    /// What it takes away less what it loses, on average.
    Net,
}

/// The plans of a decision to try, and what they're tried from: the
/// battle as each guess at the other player leaves it as the decision
/// takes effect.
#[derive(Clone)]
struct Job {
    /// Each guess that plays out unlike the others, as it stands then, the
    /// battle as it leaves it, and how many guesses play out as it does.
    aheads: Vec<(Guessed, Battle, i64)>,
    plans: Vec<Exec>,
    side: u8,
    /// The ticks each plan is tried for, the dimmings not counted.
    horizon: u32,
    worth: Worth,
    /// Each plan tried for its ticks whatever the battle's state
    /// ([`Asked::Warm`]).
    warm: bool,
}

impl Job {
    /// What plan `i` is worth (times the guesses), tried in `trial`; and the
    /// ticks that took.
    fn score(&self, i: usize, trial: &mut Battle) -> (i64, u64) {
        let (mut dealt, mut taken, mut lost, mut ticks, mut count) = (0, 0, 0, 0u64, 0);
        for (guess, ahead, alike) in &self.aheads {
            trial.clone_from(ahead);
            let mut exec = self.plans[i];
            let mut guess = *guess;
            let (mut live, mut t) = (0, 0);
            while live < self.horizon && t < self.horizon + DIMMED_MOST && (self.warm || fight_goes_on(trial)) {
                let mine = exec.buttons(trial, self.side);
                let theirs = guess.buttons(trial, self.side ^ 1);
                step(trial, self.side, mine, theirs);
                t += 1;
                live += !trial.is_dimmed() as u32;
            }
            ticks += t as u64;
            let (d, l) = outcome(ahead, trial, self.side);
            dealt += d * alike;
            taken = taken.max(l);
            lost += l * alike;
            count += alike;
        }
        let worth = match self.worth {
            Worth::Net => dealt - lost,
            Worth::Fight { caution } => {
                let caution = caution.min(100) as i64;
                dealt - (taken * count * caution + lost * (100 - caution)) / 100
            }
        };
        (worth, ticks)
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

/// How many guesses a job's worths are times.
fn stand_count(job: &Job) -> i64 {
    job.aheads.iter().map(|(_, _, alike)| alike).sum()
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

    fn answer(&mut self, q: &Question) -> Answer {
        let start = Instant::now();
        match &q.asked {
            Asked::Warm => {
                let b = &*q.battle;
                let of = self.threads();
                let job = Job { aheads: vec![(Guessed::new(Guess::Holds, 0, q.past), b.clone(), 1)], plans: vec![q.now; of], side: q.side, horizon: 1, worth: Worth::Net, warm: true };
                self.run(&job);
                Answer { id: q.id, tried: Vec::new(), guesses: 1, worths: Vec::new(), form: None, ticks: 0, took: start.elapsed() }
            }
            Asked::Chips(chips) => self.worths(q, chips, start),
            Asked::Decide => self.decide(q, start),
        }
    }

    fn threads(&self) -> usize {
        #[cfg(not(target_arch = "wasm32"))]
        return self.helpers.len() + 1;
        #[cfg(target_arch = "wasm32")]
        1
    }

    /// The battle as each of the level's guesses would leave it `ticks` on
    /// (the plan it follows, `now`, going on), those that play out alike
    /// together; and the ticks that took.
    fn aheads(b: &Battle, side: u8, now: Exec, theirs: u16, past: Past, guesses: &[Guess], ticks: u32) -> (Vec<(Guessed, Battle, i64)>, Exec, u64) {
        let mut spent = 0;
        let mut followed = now;
        let mut aheads: Vec<(Guessed, Battle, i64)> = Vec::with_capacity(guesses.len());
        let mut kinds: Vec<u64> = Vec::with_capacity(guesses.len());
        for &g in guesses {
            let mut ahead = b.clone();
            let mut now = now;
            let mut guess = Guessed::new(g, theirs, past);
            let mut t = 0;
            while t < ticks && fight_goes_on(&ahead) {
                let mine = now.buttons(&ahead, side);
                let theirs = guess.buttons(&ahead, side ^ 1);
                step(&mut ahead, side, mine, theirs);
                t += 1;
            }
            spent += t as u64;
            if aheads.is_empty() {
                followed = now;
            }
            // (By the battle each leaves, the guesses that press nothing more play out alike: tried once.)
            let digest = ahead.digest();
            match guess.idle().then(|| kinds.iter().position(|d| *d == digest)).flatten() {
                Some(at) if aheads[at].0.idle() => aheads[at].2 += 1,
                _ => {
                    kinds.push(digest);
                    aheads.push((guess, ahead, 1));
                }
            }
        }
        (aheads, followed, spent)
    }

    /// A decision in the fight: from the battle as the plan it follows and
    /// each guess would leave it a reaction time on, each panel it can
    /// reach to stand on (and the plan it follows going on); then from each,
    /// a shot and the chip in hand, each followed by the safest panel.
    fn decide(&mut self, q: &Question, start: Instant) -> Answer {
        let (b, side, level) = (&*q.battle, q.side, q.level);
        let guesses = &GUESSES[..(level.guesses as usize).clamp(1, GUESSES.len())];
        let (aheads, followed, mut ticks) = Search::aheads(b, side, q.now, q.theirs, q.past, guesses, level.reaction);
        let ahead = &aheads[0].1;
        let panels = reachable(ahead, side);
        if panels.is_empty() || !fight_goes_on(ahead) {
            return Answer { id: q.id, tried: Vec::new(), guesses: 1, worths: Vec::new(), form: None, ticks, took: start.elapsed() };
        }
        let chip = ahead.hands[side as usize].remaining() > 0;
        let steps_to = |p: PanelPos| panels.iter().find(|(q, _)| *q == p).map_or(u8::MAX, |(_, s)| *s);
        let worth = Worth::Fight { caution: level.caution };
        // Standing on each panel: how dangerous it is.
        let stands: Vec<Exec> = panels.iter().map(|(p, _)| Exec::new(Plan { to: *p, then: Then::Hold, after: None })).chain(std::iter::once(followed)).collect();
        let job = Job { aheads, plans: stands, side, horizon: level.horizon, worth, warm: false };
        let (stand_worths, t) = self.run(&job);
        ticks += t;
        // (The safest: worth most, the nearest of those. An attack from a panel about as safe stays there after: no
        // walking back and forth for nothing.)
        let safest = (0..panels.len()).max_by_key(|&i| (stand_worths[i], std::cmp::Reverse(panels[i].1)));
        let steady = level.steady.max(1) as i64 * stand_count(&job);
        let after = |i: usize| safest.filter(|&s| stand_worths[i] + steady <= stand_worths[s]).map(|s| panels[s].0);
        // Attacking from the safest few (where it stands among them).
        let mut by_worth: Vec<usize> = (0..panels.len()).collect();
        by_worth.sort_by_key(|&i| (std::cmp::Reverse(stand_worths[i]), panels[i].1));
        by_worth.truncate(ATTACK_PANELS);
        if !by_worth.contains(&0) {
            by_worth.push(0);
        }
        let mut attacks = Vec::with_capacity(by_worth.len() * 2);
        for &i in &by_worth {
            let (to, after) = (panels[i].0, after(i));
            attacks.push(Exec::new(Plan { to, then: Then::Shot, after }));
            if chip {
                attacks.push(Exec::new(Plan { to, then: Then::Chip, after }));
            }
        }
        let Job { aheads, plans: stands, .. } = job;
        let job = Job { aheads, plans: attacks, side, horizon: level.horizon, worth, warm: false };
        let (mut attack_worths, t) = self.run(&job);
        ticks += t;
        // (A chip held a long while is used where it does best, though it takes nothing away or costs some: it does
        // what no HP shows, or it would bar the chips behind it. The longer, the more it costs to hold.)
        let mut followed = (stands[stands.len() - 1], stand_worths[stands.len() - 1]);
        if q.stale > 0 {
            let count: i64 = job.aheads.iter().map(|(_, _, alike)| alike).sum();
            let eager = STALE_EAGER * (60 + q.stale as i64) / 60 * count;
            for (e, w) in job.plans.iter().zip(attack_worths.iter_mut()).chain(std::iter::once((&followed.0, &mut followed.1))) {
                if e.plan.then == Then::Chip {
                    *w += eager;
                }
            }
        }
        // (Nearest first, standing before a shot before a chip; the plan it follows last.)
        let mut tried: Vec<(Exec, i64)> = stands[..stands.len() - 1].iter().copied().zip(stand_worths).chain(job.plans.iter().copied().zip(attack_worths)).collect();
        tried.sort_by_key(|(e, _)| (steps_to(e.plan.to), e.plan.then));
        tried.push(followed);
        let guesses = stand_count(&job);
        Answer { id: q.id, tried, guesses, worths: Vec::new(), form: None, ticks, took: start.elapsed() }
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
    /// order to pick them in (the higher first; none below 0 is picked),
    /// and the form to take.
    ///
    /// Each chip dealt is worth what it makes in the fight the screen
    /// opens on ([`Search::chip_worth`]). Picking one can make others
    /// unpickable (a code that goes with none of theirs), so each chip is
    /// tried as the first pick on a copy of the screen, the rest picked
    /// after it by their worth, and the picks worth most in all are the
    /// hand. Where the picks form a Program Advance on the copy, it is
    /// worth what it makes in the fight in their place; where a chip worth
    /// nothing alone makes the one picked before it worth more, the two go
    /// together.
    fn worths(&mut self, q: &Question, chips: &[(u8, FolderChip)], start: Instant) -> Answer {
        let mut ticks = 0;
        let mut worths: Vec<i64> = Vec::with_capacity(chips.len());
        for (n, (slot, chip)) in chips.iter().enumerate() {
            let worth = match chips[..n].iter().position(|(_, c)| c == chip) {
                Some(at) => worths[at],
                None => self.chip_worth(q, &[*slot], &mut ticks),
            };
            worths.push(worth);
        }
        let worth_of = |slot: u8| chips.iter().position(|(s, _)| *s == slot).map_or(0, |at| worths[at]);
        // (Each first pick, the rest by worth: the picks, and what they are worth in all.)
        let mut best: Option<(Vec<u8>, i64)> = None;
        // (A chip worth nothing isn't picked: alone in the hand it would do nothing, and it stays in the folder for
        // another screen. A chip the hand folds into the one before it is picked after that one, below.)
        for (n, (first, chip)) in chips.iter().enumerate() {
            if chips[..n].iter().any(|(_, c)| c == chip) || worth_of(*first) <= 0 {
                continue;
            }
            let (picks, advance, _) = picks_on_a_copy(
                q,
                |s| {
                    Some(if s == *first {
                        i64::MAX
                    } else if worth_of(s) > 0 {
                        worth_of(s)
                    } else {
                        -1
                    })
                },
                &mut ticks,
            );
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
        let mut order = best.map_or_else(Vec::new, |(order, _)| order);
        // (A chip worth nothing alone may be one that the hand folds into the chip picked before it, as OK builds the
        // hand (an attack-up: the engine's `ChipHand::fold_next`): picked right after each of the two worthiest
        // attacking picks on a copy, the two worth what the chip they make takes away in the fight. Where that is
        // more than the attacking chip alone, it goes there in the order.)
        let leads: Vec<u8> = order.iter().copied().filter(|&s| worth_of(s) > 0).take(2).collect();
        let mut boosted: Vec<u8> = Vec::new();
        for &(slot, chip) in chips {
            if worth_of(slot) > 0 || order.iter().position(|s| *s == slot).is_some_and(|at| at > 0 && boosted.contains(&order[at - 1])) {
                continue;
            }
            if chips.iter().take_while(|(s, _)| *s != slot).any(|(_, c)| *c == chip) {
                continue;
            }
            let mut found: Option<(u8, i64, Vec<u8>)> = None;
            for &lead in leads.iter().filter(|l| !boosted.contains(l)) {
                let mut tried: Vec<u8> = order.iter().copied().filter(|s| *s != slot).collect();
                let at = tried.iter().position(|s| *s == lead).expect("a pick") + 1;
                tried.insert(at, slot);
                let rank = |s: u8| tried.iter().position(|o| *o == s).map(|at| (tried.len() - at) as i64).or(Some(-1));
                let (picks, _, _) = picks_on_a_copy(q, rank, &mut ticks);
                let Some(at) = picks.iter().position(|s| *s == lead).filter(|at| picks.get(at + 1) == Some(&slot)) else {
                    continue;
                };
                let gain = self.chip_worth(q, &picks[at..at + 2], &mut ticks) - worth_of(lead);
                if gain > DEALT && found.as_ref().is_none_or(|(_, g, _)| gain > *g) {
                    found = Some((lead, gain, picks));
                }
            }
            if let Some((lead, _, picks)) = found {
                boosted.push(lead);
                order = picks;
            }
        }
        // (The order as worths by slot: the first pick the highest; a chip not picked, below 0.)
        let ranks: Vec<i64> = chips.iter().map(|(s, _)| order.iter().position(|o| o == s).map_or(-1, |at| (order.len() - at) as i64)).collect();
        let form = if q.level.forms { self.form(q, chips, &ranks, &mut ticks) } else { None };
        Answer { id: q.id, tried: Vec::new(), guesses: 1, worths: ranks, form, ticks, took: start.elapsed() }
    }

    /// What its side makes in the fight the custom screen `q` shows opens
    /// on, with the chips in `slots` picked in that order (both screens
    /// through OK, the other player picking every chip it can): the hand's
    /// first chip used from the panel it can reach where that makes most,
    /// over [`WORTH_HORIZON`] ticks, on average over the guesses, less what
    /// standing where it stands makes.
    fn chip_worth(&mut self, q: &Question, slots: &[u8], ticks: &mut u64) -> i64 {
        let side = q.side;
        let in_order = |s: u8| Some(slots.iter().position(|o| *o == s).map_or(-1, |at| (slots.len() - at) as i64));
        let Some(b) = to_the_fight(&q.battle, side, &in_order, ticks) else {
            return 0;
        };
        if b.hands[side as usize].remaining() == 0 {
            return 0;
        }
        let panels = reachable(&b, side);
        let Some(&(at, _)) = panels.first() else {
            return 0;
        };
        let mut plans = vec![Exec::stand(at)];
        plans.extend(panels.iter().map(|(p, _)| Exec::new(Plan { to: *p, then: Then::Chip, after: None })));
        let guesses = &[Guess::Holds, Guess::Chip, Guess::Up, Guess::Down][..(q.level.guesses as usize).clamp(1, 4)];
        let (aheads, _, t) = Search::aheads(&b, side, Exec::stand(at), 0, Past([0; PAST]), guesses, 0);
        *ticks += t;
        let job = Job { aheads, plans, side, horizon: WORTH_HORIZON, worth: Worth::Net, warm: false };
        let (worths, t) = self.run(&job);
        *ticks += t;
        worths[1..].iter().max().map_or(0, |best| best - worths[0])
    }

    /// The form to take on the custom screen `q` shows, if any makes the
    /// fight better: each way the screen's keys put the navi in another form
    /// (walked in copies, [`forms_on_a_copy`]), from the screen as dealt
    /// and from it with the picks made (`ranks`), played through OK and a few
    /// seconds of the fight ([`rollout`]) against the picks alone.
    fn form(&mut self, q: &Question, chips: &[(u8, FolderChip)], ranks: &[i64], ticks: &mut u64) -> Option<FormChoice> {
        let side = q.side;
        let worth = |slot: u8| chips.iter().position(|(s, _)| *s == slot).map(|at| ranks[at]);
        let (_, _, picked) = picks_on_a_copy(q, worth, ticks);
        let base = to_the_fight(&picked, side, &worth, ticks).map(|b| rollout(&b, side, ticks))?;
        let mut best: Option<(FormChoice, i64)> = None;
        let mut seen = Vec::new();
        for (from, before) in [(&*q.battle, true), (&picked, false)] {
            for goal in forms_on_a_copy(from, side, !before, ticks) {
                if seen.contains(&goal.brings) {
                    continue;
                }
                seen.push(goal.brings);
                let Some(b) = to_the_fight(&goal.battle, side, &worth, ticks) else {
                    continue;
                };
                let made = rollout(&b, side, ticks);
                if made > base + FORM_MARGIN && best.as_ref().is_none_or(|(_, w)| made > *w) {
                    best = Some((FormChoice { script: goal.script, before }, made));
                }
            }
        }
        best.map(|(f, _)| f)
    }
}

/// The ticks a chip is tried for as its worth is found (a dimming's not
/// counted): long enough for a navi chip's attack.
const WORTH_HORIZON: u32 = 120;
/// The ticks a copy of the battle is given to get from its custom screen to
/// the fight (both screens closing, a turn's start; a round's first, its
/// banner).
const TO_THE_FIGHT: u32 = 900;
/// The ticks on end the fight takes buttons before a chip is tried in it.
const SETTLED: u32 = 8;
/// The ticks a copy of the custom screen is given to make its picks.
const PICKING: u32 = 400;
/// The live ticks of the fight a form is tried for ([`rollout`]).
const ROLLOUT: u32 = 480;
/// In tenths of HP, how much better the fight must go with a form for it to
/// be taken.
const FORM_MARGIN: i64 = 50;

/// The fight the custom screen of `from` opens on, side `side` picking by
/// `worth` (the picker's) and the other picking every chip it can: both
/// screens through OK, then the turn's start, till the fight takes buttons
/// a few ticks on end (as the screen closes the fight reads as it was, and
/// the turn's start banner comes after). None if it doesn't get there.
fn to_the_fight(from: &Battle, side: u8, worth: &dyn Fn(u8) -> Option<i64>, ticks: &mut u64) -> Option<Battle> {
    let fighting = |b: &Battle| fight_goes_on(b) && b.fight.state == nettai_battle::battle::fight::FIGHTING;
    let mut b = from.clone();
    let (mut mine, mut theirs) = (Picker::default(), Picker::default());
    let (mut t, mut on) = (0, 0);
    while on < SETTLED && t < TO_THE_FIGHT && !b.is_stopped() {
        let custom = b.round.mode == mode::CUSTOM;
        let m = if custom { mine.buttons(&b, side as usize, Some(worth)) } else { 0 };
        let o = if custom { theirs.buttons(&b, (side ^ 1) as usize, None) } else { 0 };
        step(&mut b, side, m, o);
        on = if fighting(&b) { on + 1 } else { 0 };
        t += 1;
    }
    *ticks += t as u64;
    fighting(&b).then_some(b)
}

/// How a fight goes for side `side` from `b` over [`ROLLOUT`] live ticks,
/// both navis lining up with each other's row, charging and shooting, and
/// using their chips there ([`Rollout`]): what it takes away less what it
/// loses, in tenths of HP.
fn rollout(b: &Battle, side: u8, ticks: &mut u64) -> i64 {
    let mut trial = b.clone();
    let mut players = [Rollout::default(), Rollout::default()];
    let (mut live, mut t) = (0, 0);
    while live < ROLLOUT && t < ROLLOUT + 4 * DIMMED_MOST && fight_goes_on(&trial) {
        let mine = players[0].buttons(&trial, side);
        let theirs = players[1].buttons(&trial, side ^ 1);
        step(&mut trial, side, mine, theirs);
        t += 1;
        live += !trial.is_dimmed() as u32;
    }
    *ticks += t as u64;
    let (dealt, lost) = outcome(b, &trial, side);
    dealt - lost
}

/// A plain way to play a fight out: walk to the other navi's row, charge,
/// shoot once the charge is full, and use a chip there every so often.
#[derive(Clone, Copy, Debug, Default)]
struct Rollout {
    dir: u16,
    held: u8,
    from: Option<PanelPos>,
    ticks: u32,
    last: u16,
}

impl Rollout {
    fn buttons(&mut self, b: &Battle, side: u8) -> u16 {
        let pressed = self.next(b, side);
        self.last = pressed;
        self.ticks += 1;
        pressed
    }

    fn next(&mut self, b: &Battle, side: u8) -> u16 {
        let (Some(r), Some(them)) = (b.player(side), navi_at(b, side ^ 1)) else {
            return keys::B;
        };
        let at = b.objects.get(r).panel;
        if at.y != them.y {
            if self.dir != 0 {
                if Some(at) != self.from || self.held >= STEP_MOST {
                    self.dir = 0;
                    return keys::B;
                }
                self.held += 1;
                return self.dir | keys::B;
            }
            if let Some(dir) = toward(b, r, at, PanelPos { x: at.x, y: them.y }) {
                (self.dir, self.held, self.from) = (dir, 1, Some(at));
                return dir | keys::B;
            }
        }
        self.dir = 0;
        if charge(b, side) >= 2 {
            return 0;
        }
        if b.hands[side as usize].remaining() > 0 && self.ticks % 30 == 0 && self.last & keys::A == 0 {
            return keys::A | keys::B;
        }
        keys::B
    }
}

/// The picks its side makes on a copy of the custom screen `q` shows, by
/// `worth` (the picker's: the higher first, none below 0), till there is
/// none left to pick; the Program Advance they form there, if they do; and
/// the copy then.
fn picks_on_a_copy(q: &Question, worth: impl Fn(u8) -> Option<i64>, ticks: &mut u64) -> (Vec<u8>, Option<nettai_battle::custom::builder::FormedAdvance>, Battle) {
    let side = q.side as usize;
    let mut b = (*q.battle).clone();
    let mut picker = Picker::default();
    for _ in 0..PICKING {
        let Some(mine) = picker.step(&b, side, Some(&worth), false) else {
            break;
        };
        step(&mut b, q.side, mine, 0);
        *ticks += 1;
    }
    let screen = b.custom.sides[side].screen;
    match screen {
        Some(shown) => (shown.selection[..shown.selected as usize].to_vec(), shown.program_advance, b),
        None => (Vec::new(), None, b),
    }
}

/// A way the screen's keys put the navi in another form: the buttons, tick
/// by tick, what the turn then brings ([`brings`]), and the copy then.
struct FormGoal {
    script: Vec<u16>,
    brings: u64,
    battle: Battle,
}

/// What a custom screen's choices bring the turn besides its chips, as a
/// digest: the form a pick puts the navi in (EXE6's Crosses and Beast Out),
/// a chip traded or held for a rules' button (EXE5's and EXE4's souls,
/// which give their form at OK, and its arm change), the rules' buttons
/// picked, and the rules' state (which soul a button offers).
fn brings(b: &Battle, side: u8) -> Option<u64> {
    let s = b.custom.sides[side as usize].screen?;
    let buttons: Vec<SlotState> = s.slots.iter().filter(|slot| matches!(slot.kind, SlotKind::Button { .. })).map(|slot| slot.state).collect();
    let taken = s.form.is_some() || s.trade.is_some() || s.hold.is_some() || buttons.contains(&SlotState::Selected);
    taken.then(|| nettai_battle::digest::stable_hash(&(s.form, s.trade.map(|t| t.button), s.hold.map(|h| h.button), buttons, &b.rules[side as usize])))
}

/// The screens a form walk visits at most, the presses of a way at most,
/// and the ways it keeps at most.
const FORM_NODES: usize = 160;
const FORM_GOALS: usize = 10;
const FORM_PRESSES: usize = 10;
/// The ticks a press out of choosing (a window's) is given before the next.
const FORM_SETTLE: u32 = 40;

/// Every way side `side`'s custom screen in `from` can be made to bring the
/// turn something new besides chips ([`brings`]: a form) by its keys (the
/// directions, A and B; A on a chip only where `may_pick`), each the first
/// found (the fewest presses), back on the screen choosing: walked in
/// copies, screen by screen, each press of each, a screen seen once.
fn forms_on_a_copy(from: &Battle, side: u8, may_pick: bool, ticks: &mut u64) -> Vec<FormGoal> {
    let screen = |b: &Battle| b.custom.sides[side as usize].screen.filter(|s| b.round.mode == mode::CUSTOM && s.phase.chooses() && b.custom.sides[side as usize].in_custom);
    let Some(first) = screen(from).filter(|s| s.phase == Phase::Choosing) else {
        return Vec::new();
    };
    let _ = first;
    let had = brings(from, side);
    let key = |b: &Battle| {
        let s = screen(b).expect("on the screen");
        let phase = match s.phase {
            Phase::Window { window, .. } => Some(window),
            _ => None,
        };
        let states: Vec<SlotState> = s.slots.iter().map(|slot| slot.state).collect();
        nettai_battle::digest::stable_hash(&(std::mem::discriminant(&s.phase), phase, s.cursor, &s.selection[..s.selected as usize], s.form, s.trade, s.hold, states, &b.rules[side as usize]))
    };
    let mut goals: Vec<FormGoal> = Vec::new();
    let mut seen = HashSet::from([key(from)]);
    let mut queue = VecDeque::from([(from.clone(), Vec::<u16>::new(), 0usize)]);
    let mut nodes = 0;
    while let Some((b, script, presses)) = queue.pop_front() {
        if nodes >= FORM_NODES {
            break;
        }
        nodes += 1;
        let shown = screen(&b).expect("on the screen");
        for press in [keys::UP, keys::DOWN, keys::LEFT, keys::RIGHT, keys::A, keys::B] {
            if press == keys::A && !may_pick && shown.phase == Phase::Choosing && matches!(shown.slots[shown.cursor as usize].kind, SlotKind::Chip { .. }) {
                continue;
            }
            let mut c = b.clone();
            let mut s = script.clone();
            // (A direction moves on its second tick held, the screen's auto-repeat; a tick let go after.)
            let held: &[u16] = if press & (keys::UP | keys::DOWN | keys::LEFT | keys::RIGHT) != 0 { &[press, press, 0] } else { &[press, 0] };
            for &p in held {
                step(&mut c, side, p, 0);
                s.push(p);
            }
            let mut t = 0;
            while screen(&c).is_some_and(|x| x.phase != Phase::Choosing) && t < FORM_SETTLE {
                step(&mut c, side, 0, 0);
                s.push(0);
                t += 1;
            }
            *ticks += s.len() as u64 - script.len() as u64;
            let Some(now) = screen(&c) else { continue };
            if !seen.insert(key(&c)) {
                continue;
            }
            let new = brings(&c, side);
            if now.phase == Phase::Choosing && new.is_some() && new != had {
                if let Some(brings) = new.filter(|n| goals.iter().all(|g| g.brings != *n)) {
                    goals.push(FormGoal { script: s, brings, battle: c });
                    if goals.len() >= FORM_GOALS {
                        return goals;
                    }
                }
                continue;
            }
            if presses + 1 < FORM_PRESSES {
                queue.push_back((c, s, presses + 1));
            }
        }
    }
    goals
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
    Thread {
        ask: std::sync::mpsc::Sender<Question>,
        answers: std::sync::mpsc::Receiver<Answer>,
    },
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

/// The live ticks of the fight a chip stays in hand before it is used where
/// it does best, though no plan shows it taking anything away
/// ([`Question::stale`]); and what using it is worth then, in tenths of HP,
/// and as much again each second on.
const STALE: u32 = 180;
const STALE_EAGER: i64 = 50;

/// Where it stands before it has seen its navi: no panel.
const NOWHERE: PanelPos = PanelPos { x: 0, y: 0 };

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
    /// The other player's presses ([`Guess::Repeats`]).
    past: Past,
    /// The chips in hand, and the live ticks of the fight since that
    /// changed; and whether the navi didn't take the chip in hand when it
    /// was used (no hurry to use it, then, till the hand changes).
    hand: u8,
    held: u32,
    refused: bool,
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
            now: Exec::stand(NOWHERE),
            pending: None,
            asked: 0,
            ticks: 0,
            since: 0,
            last_theirs: 0,
            past: Past([0; PAST]),
            hand: 0,
            held: 0,
            refused: false,
            last_hp: None,
            alerted: false,
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
        self.now = Exec::stand(NOWHERE);
        self.pending = None;
        (self.ticks, self.since) = (0, 0);
        (self.last_theirs, self.last_hp, self.alerted) = (0, None, false);
        (self.past, self.hand, self.held, self.refused) = (Past([0; PAST]), 0, 0, false);
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

    fn question(&self, b: &Battle, side: u8, theirs: u16, asked: Asked) -> Question {
        Question {
            id: self.asked,
            battle: Box::new(b.clone()),
            now: self.now,
            theirs,
            past: self.past,
            stale: if self.refused { 0 } else { self.held.saturating_sub(STALE - 1) },
            side,
            level: self.level,
            asked,
        }
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
            self.thinker.warm(self.question(b, side, theirs, Asked::Warm));
        }
        if self.now.plan.to == NOWHERE {
            if let Some(at) = navi_at(b, side) {
                self.now = Exec::stand(at);
            }
        }
        self.take_decision();
        if !fight_goes_on(b) || b.is_dimmed() {
            return self.now.buttons(b, side);
        }
        self.since += 1;
        self.past.push(theirs);
        let hand = b.hands[side as usize].remaining();
        self.held = if hand != self.hand { 0 } else { self.held + 1 };
        self.refused &= hand == self.hand;
        self.hand = hand;
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
            let q = self.question(b, side, theirs, Asked::Decide);
            self.thinker.ask(q);
            self.take_decision();
        }
        let pressed = self.now.buttons(b, side);
        self.refused |= self.now.refused;
        pressed
    }

    /// Its buttons on its custom screen: the chips worth most first and the
    /// form to take, asked as the screen shows (nothing is pressed till
    /// they come); or, at a level that doesn't choose, every chip it can in
    /// the cursor's order.
    fn choose(&mut self, b: &Battle, side: u8) -> u16 {
        let s = &b.custom.sides[side as usize];
        let shown = s.screen.as_ref().filter(|x| b.round.mode == mode::CUSTOM && x.phase.chooses() && s.in_custom);
        let Some(screen) = shown else {
            self.chosen = None;
            return self.picker.buttons(b, side as usize, None);
        };
        if self.chosen.is_none() {
            if screen.phase != Phase::Choosing {
                return 0;
            }
            let dealt: Vec<(u8, FolderChip)> = screen
                .slots
                .iter()
                .enumerate()
                .filter_map(|(at, slot)| match slot.kind {
                    SlotKind::Chip { index, .. } => s.folder.chips.get(index as usize).copied().flatten().map(|chip| (at as u8, chip)),
                    _ => None,
                })
                .collect();
            let asked = (self.level.chooses || self.level.forms).then(|| {
                self.asked += 1;
                self.thinker.ask(self.question(b, side, 0, Asked::Chips(dealt.clone())));
                self.asked
            });
            self.chosen = Some(Chosen { asked, looked: 0, slots: dealt.iter().map(|(at, _)| *at).collect(), worths: None, form: None, at: 0 });
        }
        let chosen = self.chosen.as_mut().expect("asked");
        // (It looks at the chips a moment, as a player does.)
        if chosen.looked < self.level.looks {
            chosen.looked += 1;
            return 0;
        }
        if let (Some(asked), None) = (chosen.asked, &chosen.worths) {
            // (Then it has their worths: waited for if they haven't come, so its presses are the same however fast
            // its threads are.)
            let waiting = Instant::now();
            let (answer, late) = self.thinker.answer(asked);
            if late {
                self.spent.late += 1;
                self.spent.waited += waiting.elapsed();
            }
            self.spent.chips += 1;
            self.spent.ticks += answer.ticks;
            self.spent.chip_time += answer.took;
            self.spent.forms += answer.form.is_some() as u32;
            chosen.worths = Some(answer.worths);
            chosen.form = answer.form;
        }
        let chooses = self.level.chooses;
        let worths = chosen.worths.clone().filter(|_| chooses);
        let slots = chosen.slots.clone();
        let worth = |slot: u8| slots.iter().position(|at| *at == slot).map(|at| worths.as_ref().map_or(0, |w| w[at]));
        let worth: Option<&dyn Fn(u8) -> Option<i64>> = if worths.is_some() { Some(&worth) } else { None };
        // (The form's presses, before the picks or after them, each tick of them once begun.)
        let script = |chosen: &mut Chosen, begin: bool| {
            let form = chosen.form.as_ref()?;
            if !(begin || chosen.at > 0) {
                return None;
            }
            let p = form.script.get(chosen.at).copied();
            chosen.at += 1;
            p
        };
        let before = chosen.form.as_ref().is_some_and(|f| f.before);
        if let Some(p) = script(chosen, before) {
            return p;
        }
        if let Some(p) = self.picker.step(b, side as usize, worth, false) {
            return p;
        }
        if let Some(p) = script(chosen, true) {
            return p;
        }
        self.picker.buttons(b, side as usize, worth)
    }

    /// The decision asked, once its tick has come: its answer (waited for,
    /// if it isn't ready), and the plan picked from it.
    fn take_decision(&mut self) {
        let Some((id, _)) = self.pending.filter(|(_, at)| *at <= self.ticks) else {
            return;
        };
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
        let Some(&(followed, following)) = tried.last() else {
            return;
        };
        // (A miss: it goes on with what it does.)
        if self.level.blunders > 0 && self.rng.below(100) < self.level.blunders as u64 {
            return;
        }
        let best = tried.iter().map(|(_, w)| *w).max().unwrap_or(0);
        // (What it does goes on, unless another plan is clearly better: standing where it goes, or going there to
        // attack. An attack done, it picks anew.)
        if !followed.acted && following + self.level.steady.max(1) as i64 * answer.guesses > best {
            return;
        }
        // (Else the first worth most: the nearest, standing before shooting before a chip.)
        let picked = tried[..tried.len() - 1].iter().find(|(_, w)| *w == best).or(tried.last()).map(|(e, _)| *e).expect("tried");
        // (The plan it follows, picked again, goes on from where it is; done, it starts over: an A the navi didn't
        // take is pressed again.)
        if picked.plan != self.now.plan || self.now.acted {
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
/// that has come (its question, asked as the screen showed, if one was),
/// the form to take, and how far its presses have gone.
struct Chosen {
    asked: Option<u64>,
    /// The ticks it has looked at them.
    looked: u32,
    slots: Vec<u8>,
    worths: Option<Vec<i64>>,
    form: Option<FormChoice>,
    at: usize,
}

/// A custom screen's buttons that pick every chip they can, then OK: the
/// worthiest first, by a worth given for each slot (none worth less than
/// nothing), else in the order the cursor reaches them. Only the folder's
/// chips dealt (no button, no chip offered from outside it, no form).
#[derive(Clone, Copy, Debug, Default)]
pub struct Picker {
    /// What it holds, and for how many ticks more.
    held: u16,
    more: u8,
}

impl Picker {
    /// `worth`: what the chip in a slot is worth, if it is known.
    pub fn buttons(&mut self, b: &Battle, side: usize, worth: Option<&dyn Fn(u8) -> Option<i64>>) -> u16 {
        self.step(b, side, worth, true).unwrap_or(0)
    }

    /// The same; with `finish` false, none once there is no chip left to
    /// pick (instead of going to OK and pressing it).
    fn step(&mut self, b: &Battle, side: usize, worth: Option<&dyn Fn(u8) -> Option<i64>>, finish: bool) -> Option<u16> {
        let s = &b.custom.sides[side];
        let Some(screen) = s.screen.as_ref().filter(|s| b.round.mode == mode::CUSTOM && s.phase == Phase::Choosing) else {
            *self = Picker::default();
            return Some(0);
        };
        if !s.in_custom {
            return Some(0);
        }
        if self.more > 0 {
            self.more -= 1;
            return Some(self.held);
        }
        // (Let go a tick between presses.)
        if std::mem::take(&mut self.held) != 0 {
            return Some(0);
        }
        let key = match pick(screen, &b.game_rules().custom_screen, worth) {
            Some(key) => key,
            None if finish => {
                if screen.cursor != OK_SLOT {
                    keys::START
                } else {
                    keys::A
                }
            }
            None => return None,
        };
        // (A direction moves the cursor on its second tick held: the screen's auto-repeat.)
        self.more = (key & (keys::UP | keys::DOWN | keys::LEFT | keys::RIGHT) != 0) as u8;
        self.held = key;
        Some(key)
    }
}

/// The press that picks the next chip (the worthiest, by `worth`, of
/// those it can pick; else the nearest); none if there is none to pick.
fn pick(screen: &Screen, layout: &CustomScreenLayout, worth: Option<&dyn Fn(u8) -> Option<i64>>) -> Option<u16> {
    let wanted = |i: u8| {
        let slot = &screen.slots[i as usize];
        matches!(slot.kind, SlotKind::Chip { .. }) && slot.state == SlotState::Selectable
    };
    if (screen.selected as usize) >= MAX_SELECTIONS {
        return None;
    }
    // (A chip worth less than nothing isn't picked.)
    let target = worth.map(|worth| (0..screen.slots.len() as u8).filter(|&i| wanted(i) && worth(i).unwrap_or(0) >= 0).max_by_key(|&i| (worth(i).unwrap_or(0), std::cmp::Reverse(i))));
    let wanted = |i: u8| match target {
        Some(target) => target == Some(i),
        None => wanted(i),
    };
    if wanted(screen.cursor) {
        return Some(keys::A);
    }
    toward_slot(screen, layout, wanted)
}

/// The first direction of the shortest way the cursor goes to a slot
/// `wanted` takes, by the screen's own moves (the engine's: each game's
/// layout).
fn toward_slot(screen: &Screen, layout: &CustomScreenLayout, wanted: impl Fn(u8) -> bool) -> Option<u16> {
    const SLOTS: usize = nettai_battle::custom::screen::SLOTS;
    let keys = [(keys::UP, ScreenKey::Up), (keys::DOWN, ScreenKey::Down), (keys::LEFT, ScreenKey::Left), (keys::RIGHT, ScreenKey::Right)];
    let mut first: [Option<u16>; SLOTS] = [None; SLOTS];
    let mut queue = VecDeque::from([screen.cursor]);
    let mut seen = [false; SLOTS];
    seen[screen.cursor as usize] = true;
    while let Some(at) = queue.pop_front() {
        for (dir, key) in keys {
            let Some(to) = screen.target_from(at, key, layout).filter(|t| (*t as usize) < SLOTS && !seen[*t as usize]) else {
                continue;
            };
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
mod tests;
