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
//! And, where it stands and on the safest panel, "stand there holding the
//! chip, and use it a reaction time after a threat shows" (the other player
//! pressing A or B or letting go of B, or the name of their dimming chip,
//! which it cuts in on: [`Then::React`]), played so in the copies against
//! the same guesses: using a chip now is weighed against holding it to
//! react with it, not against never using it. A plan is worth what it
//! takes away on average over the guesses, less what it loses (the most in
//! any and the average, by its caution); a chip is spent only where using
//! it now makes clearly more than keeping it ([`SPENT`]), so a chip whose
//! worth is in reacting (Invisibl, AntiDmg, a Barrier) is kept while no
//! threat shows, though a guess has the other player attack any moment
//! (holding it to react does as well there), and used once an attack is on
//! its way that it alone stops (a shot in flight, an attack starting: then
//! reacting later is too late). A chip held a long while is used where it
//! does best though no plan shows it taking anything away, where it bars a
//! chip not its like behind it (the hand is used in order) and holding it
//! to react shows no worth ([`STALE`]); and then not at once where using it
//! a while on makes more, but at the moment within a few seconds where
//! using it costs least, tried in longer copies against a player who fights
//! on ([`Search::stale_wait`]): a second Invisibl waits till the first's
//! invisibility ends rather than refresh it. A decision asked with other
//! chips in hand than it has as it is taken is dropped and asked again: its
//! chip plans were for the chip that was next (a push decided for the first
//! Invisibl, taken after the first was used, pushed out the second). Of
//! plans worth the same those that
//! keep the chip win, then the nearest, standing before shooting before
//! reacting, so it keeps its charge and its chips; and it keeps to what it
//! does unless another plan is better by a margin ([`Level::steady`]): no
//! change of mind for nothing, no pacing.
//! A chip's dimming isn't counted against the horizon: a copy that enters
//! one plays on till it ends, and a moment of the fight after, so the
//! attack's hits land in it; and a plan stands still in it, as the fight
//! does (what it waits for, it waits for after).
//!
//! What a copy makes is HP and ground alike, wherever copies are scored (a
//! plan, a turn on the custom screen, a press while dimmed): the HP it takes
//! away and loses, the panels its side takes or loses ([`AREA`]: a column
//! about a modest hit), and the panels its navi has to move on ([`ROOM`]: a
//! navi boxed in has no room to dodge), as the copy ends. So a chip that
//! takes area (AreaGrab) is used where taking it pays, and losing a column
//! is a loss, though no HP shows it.
//!
//! **While the battle is dimmed** ([`dimmed`]) nothing moves but what the
//! chip brings, so what a press of A does then is the battle's alone to
//! say, and it is found in copies: a chip that waits for its user's A (a
//! sight circling the other side's area, a navi turning his aim) is
//! pressed on the tick that does most, and a dimming chip next in hand
//! cuts in on the other player's where that does more than letting theirs
//! run (an attack that answers theirs, or a chip that stops it: in every
//! game a chip that dims can cut in, EXE6's Invisibl, Barrier and AntiDmg
//! among them). Live, that is a decision of its own, asked as the dimming begins
//! (and again where it goes otherwise, or after a press) and taken a
//! reaction time on: each tick a press may be made on, played through the
//! dimming, the other player aiming theirs as well as it can. In the
//! copies a plan is tried in, both sides aim so too, more coarsely
//! ([`dimmed::aim`]), so a chip is worth what it makes aimed, not pressed
//! at random.
//!
//! It never opens the custom screen: the player does (live play clears L
//! and R from a computer-played dummy's presses in the fight), so it plans
//! with the hand it has, which may stay empty till then; nothing it tries
//! in a copy presses them either.
//!
//! **On its custom screen** it looks at the chips dealt and picks the hand
//! worth most over the turn the screen opens: in a copy, the hand picked,
//! both screens through OK (the other player picking every chip it can),
//! and the turn played out against the guesses (as long as the custom gauge
//! takes to fill, the shortest a turn can be: the player opens the screen,
//! not it), its navi lined up with the other's row and its chips used in
//! the order picked, each once using it pays more than holding it to react
//! with it (tried in copies a moment ahead, from where it stands and its
//! row's other panels: once the other navi is in reach, once the chip
//! before it is done and its hit has passed, once a panel opens, once an
//! attack is on its way), and cut in on the other player's dimming chip
//! where that pays clearly ([`dimmed::CUT_IN_MARGIN`], as live), less
//! what the same turn makes with no new
//! hand (picking nothing keeps the old hand; picking any replaces it, its
//! chips lost unused). So a Barrier is worth the hits it stops, a Recover
//! the HP it gives back, and a hand's second and third chips what they do
//! later in the turn; and a chip whose worth is in reacting goes after the
//! chips that attack, where it bars none of them, unless what it does at
//! the front (a cut-in ready) is worth more than what it bars: the turn's
//! copies weigh the two. The hand is built a chip at a time: the chip whose
//! addition raises its worth most goes after the others, while one raises
//! it at all. Then every other chip goes after them, as a player picks all
//! the codes allow (a chip left is dealt again first next turn, in the
//! place of a new one): only one that alone clearly costs the turn is
//! left. A pick can make others unpickable by their codes, so each
//! chip worth something alone is tried as the first pick (the worthiest
//! first), and the hand worth most wins; where its picks form a Program
//! Advance on the copy, the copy plays it in their place, as OK forms it.
//! A chip with no worth of its own may be one the hand folds into the chip
//! picked right before it as OK builds the hand (an attack-up: Atk+10,
//! Navi+20, EXE6's Uninstll; the engine's hand builder), never used alone:
//! it is also tried right after the hand's first and second picks, and a
//! hand the copy's OK leaves it standing alone in (picked first, or after a
//! chip it doesn't attach to: used so, it poofs) isn't picked; nor is it
//! picked among the chips after the hand. The work is
//! the level's ([`Level::budget`], in ticks of the copies), not a time: the
//! same screen gives the same hand on any machine. It presses nothing for
//! its look time ([`Level::looks`]), and then nothing till its hand is
//! found: the battle goes on meanwhile, never waiting for it (as it goes on
//! while a link's other player sits on their screen), so on a slower
//! machine it only looks longer. The hand is used in the order picked
//! (`Level::chooses`; else every chip it can in the cursor's order). Either
//! way the picker never picks a chip OK would leave standing alone (a chip
//! that folds, picked first or after a chip it doesn't attach to: the
//! engine's own test, `custom::builder::stays_unfolded`), so no level's
//! hand holds one; one left alone in hand otherwise (nothing to push it out
//! for) is never used, as using it makes nothing. The
//! cursor finds its way by the screen's own moves, each game's
//! (`Screen::target_from`).
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
//! level give the same presses, on its thread or not (but for the ticks it
//! looks at its custom screen, waiting for its hand: the hand is the same).

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

mod dimmed;

use dimmed::{Branches, DimAnswer, Dimmed, Memo};

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
    /// On its custom screen it picks the hand worth most over a turn (tried
    /// in copies of the fight: [`Fighter`]'s docs), used in the order
    /// picked; else every chip it can, in the order the cursor reaches them.
    pub chooses: bool,
    /// On its custom screen it takes a form the rules offer (a Cross, Beast
    /// Out, a soul) where a copy of the fight goes better with it.
    pub forms: bool,
    /// The ticks it looks at the chips dealt before it presses anything, at
    /// least: it looks on (the battle going on, as it does while a link's
    /// other player sits on their screen) till the hand it picks is found.
    pub looks: u32,
    /// The work it puts into finding that hand, in ticks of the copies it
    /// plays (the same screen gives the same hand on any machine, however
    /// fast: only how long it looks differs).
    pub budget: u32,
}

impl Level {
    pub const EASY: Level = Level { reaction: 24, horizon: 40, every: 20, guesses: 1, blunders: 30, alert: false, caution: 30, steady: 40, chooses: false, forms: false, looks: 60, budget: 0 };
    pub const NORMAL: Level = Level { reaction: 14, horizon: 50, every: 12, guesses: 5, blunders: 10, alert: true, caution: 40, steady: 30, chooses: true, forms: true, looks: 60, budget: 80_000 };
    pub const HARD: Level = Level { reaction: 9, horizon: 60, every: 8, guesses: 6, blunders: 0, alert: true, caution: 50, steady: 20, chooses: true, forms: true, looks: 45, budget: 100_000 };
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
    /// Stand there holding the chip in hand, and use it a reaction time
    /// after a threat shows: the other player pressing A or B or letting go
    /// of B (an attack starting), or their dimming chip's name (a cut-in).
    /// Live, it stands: the reaction is its next decision's (asked at once
    /// as the other player presses, [`Level::alert`]), or the dimming's
    /// ([`dimmed`]); in a copy it is played so, a reaction time on.
    React,
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
/// The dimmed ticks a copy is played for at most: a chip's dimming, its
/// aims made, and what it brings about (EXE4's GutsSoul's some 310).
const DIMMED_MOST: u32 = 400;
/// The ticks of the fight a copy is played for at least after a dimming
/// ends, its horizon or not: the attack's hits land.
const AFTER_DIMMING: u32 = 30;
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
    /// Of a plan that reacts ([`Then::React`]): its reaction time, and once
    /// a threat has shown, the ticks left till it reacts.
    reaction: u16,
    alarm: Option<u16>,
    /// The ticks of the fight it stands where it is before it sets out, B
    /// let go (a chip used a while on: [`Search::stale_wait`]).
    wait: u16,
}

impl Exec {
    fn new(plan: Plan) -> Exec {
        Exec { plan, dir: 0, held: 0, from: None, acted: false, waited: 0, hand: 0, refused: false, last: 0, reaction: 0, alarm: None, wait: 0 }
    }

    /// A plan set out on `wait` ticks of the fight on, standing till then.
    fn later(plan: Plan, wait: u32) -> Exec {
        Exec { wait: wait.min(u16::MAX as u32) as u16, ..Exec::new(plan) }
    }

    /// A plan that reacts, `reaction` ticks after a threat shows.
    fn reacting(plan: Plan, reaction: u32) -> Exec {
        Exec { reaction: reaction.min(u16::MAX as u32) as u16, ..Exec::new(plan) }
    }

    /// What it does at its panel now: a plan that reacts holds its chip
    /// till its alarm has gone off, then uses it.
    fn then(&self) -> Then {
        match self.plan.then {
            Then::React if self.alarm == Some(0) => Then::Chip,
            Then::React => Then::Hold,
            then => then,
        }
    }

    /// A plan that reacts, in a copy, the other player played as `guess`:
    /// its alarm set where a threat has shown (the guess's attack, from
    /// the tick it started; the other player's dimming chip's name, from
    /// now), and a chip its cut-in took counted done.
    fn heed(&mut self, b: &Battle, side: u8, guess: &Guessed) {
        if self.plan.then != Then::React || self.acted {
            return;
        }
        if self.alarm.is_none() {
            let since = guess.struck.map(|at| guess.t.saturating_sub(at));
            let set = match since {
                Some(since) => Some(self.reaction.saturating_sub(since.min(u16::MAX as u32) as u16)),
                None => (b.is_dimmed() && dimmed::cut_window(b, side)).then_some(self.reaction),
            };
            if set.is_some() {
                (self.alarm, self.hand) = (set, b.hands[side as usize].remaining());
            }
        } else if b.is_dimmed() && b.hands[side as usize].remaining() < self.hand {
            self.acted = true;
        }
    }

    /// Whether it cuts in now, in a copy: a plan that reacts, its alarm
    /// gone off, where its side may cut in.
    fn cuts_in(&self, b: &Battle, side: u8) -> bool {
        self.plan.then == Then::React && !self.acted && self.alarm == Some(0) && b.is_dimmed() && dimmed::cut_window(b, side)
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
        if let Some(a) = self.alarm.as_mut() {
            *a = a.saturating_sub(1);
        }
        pressed
    }

    fn next(&mut self, b: &Battle, side: u8) -> u16 {
        const HELD: u16 = keys::B;
        // (A dimming stands the plan still, as it stands the fight: its waits aren't counted then. A chip the
        // other player's dimming held up is pressed for again after it, not given up as one the navi didn't take.)
        if b.is_dimmed() {
            return HELD;
        }
        // (Waiting, B let go: what it held fires now, as it would as A is pressed at once, and no charge is
        // held for the wait.)
        if self.wait > 0 {
            self.wait -= 1;
            return 0;
        }
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
        match self.then() {
            Then::Hold | Then::React => HELD,
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
                if charge(b, side) == 0 && self.waited.is_multiple_of(16) {
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

/// What a panel of the field is worth to the side that owns it, in tenths
/// of HP: a column taken (three panels) is worth 18 HP, about a modest hit
/// (less than most chips that attack take away), and a column lost costs as
/// much. Area is reach (a side's attacks that strike a panel or so ahead
/// reach further from a column further on) and room the other side loses to
/// dodge in; stolen panels go back to their owner after a while, which a
/// copy plays out where its ticks reach that far, so only what it ends with
/// counts.
const AREA: i64 = 60;
/// What a panel its navi can step on is worth to a side besides, in tenths
/// of HP: a navi boxed into a column or two (its area taken, its panels
/// broken or taken up by obstacles) has no room to dodge in, a danger its HP
/// doesn't show yet.
const ROOM: i64 = 20;

/// The panels side `side` owns in `b` (each panel's owner as the engine
/// keeps it).
fn area(b: &Battle, side: u8) -> i64 {
    b.field.panels.iter().flatten().filter(|p| p.alliance == side).count() as i64
}

/// The panels side `side`'s navi in `b` can step on (none where it has no
/// navi): its own and not broken or taken up, as the engine's step test
/// says, and the one it stands on and the one it is stepping to (its body
/// and its reservation, which that test counts as taken: a navi on its way
/// has no less room than one standing).
fn room(b: &Battle, side: u8) -> i64 {
    let Some(r) = b.player(side) else {
        return 0;
    };
    let at = b.objects.get(r).panel;
    let mut n = 0;
    for (y, row) in b.field.panels.iter().enumerate() {
        for (x, p) in row.iter().enumerate() {
            let (x, y) = (x as u8, y as u8);
            n += (b.can_step(r, x, y) || p.reserver == Some(r) || PanelPos { x, y } == at) as i64;
        }
    }
    n
}

/// What a trial did, for side `side`, from the battle `before` to `after`:
/// the HP it took away and the HP it lost (less what it got back), in
/// tenths of HP (a navi deleted counts as [`DELETED`]); and the ground: the
/// panels it took ([`AREA`]) and the room its navi has to move in
/// ([`ROOM`]), what it gained counted with what it took away, what it lost
/// with what it lost.
fn outcome(before: &Battle, after: &Battle, side: u8) -> (i64, i64) {
    // (A battle the engine stopped is the worst: the session ends there.)
    if after.is_stopped() {
        return (0, DELETED);
    }
    let them = side ^ 1;
    let deleted = |s: u8| {
        if after.is_battle_over() && hp(after, s) == 0 { DELETED } else { 0 }
    };
    let ground = (area(after, side) - area(before, side)) * AREA + (room(after, side) - room(before, side)) * ROOM;
    (
        (hp(before, them) - hp(after, them)) * DEALT + deleted(them) + ground.max(0),
        (hp(before, side) - hp(after, side)) * TAKEN + deleted(side) + (-ground).max(0),
    )
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
    /// They fight on with their buster as a plain fight goes ([`Rollout`],
    /// their chips kept): walk to the navi's row, charge and shoot there,
    /// again and again. Not a level's guess at the next moment, but what
    /// comes over a longer while ([`Search::stale_wait`]).
    Fights,
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
    /// What they held the tick before (as the decision was asked, what
    /// they held then), and the tick their last attack started on
    /// ([`threat`]).
    before: u16,
    struck: Option<u32>,
    /// A guess that fights on ([`Guess::Fights`]): how it plays.
    roll: Rollout,
}

/// Whether a player whose buttons go from `before` to `now` starts an
/// attack: A or B pressed (a chip, the buster), or B let go (the charged
/// shot). What a plan that reacts reacts to ([`Then::React`]).
fn threat(before: u16, now: u16) -> bool {
    now & !before & (keys::A | keys::B) != 0 || before & !now & keys::B != 0
}

impl Guessed {
    fn new(guess: Guess, theirs: u16, past: Past) -> Guessed {
        let theirs = theirs & Past::KEPT;
        Guessed { guess, theirs, past, t: 0, dir: 0, held: 0, fired: false, last: 0, before: theirs, struck: None, roll: Rollout { keeps_chips: true, ..Rollout::default() } }
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
            Guess::Fights => self.roll.buttons(b, them),
            _ => 0,
        };
        self.last = pressed;
        if threat(std::mem::replace(&mut self.before, pressed), pressed) {
            self.struck = Some(t);
        }
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
            Guess::Fights => false,
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
    /// The custom screens whose hand it found after its look time
    /// ([`Level::looks`]), and the ticks it looked on for them.
    pub screens_late: u32,
    pub looked_longer: u32,
    /// The decisions while dimmed (where to press A), the time they took,
    /// and the longest.
    pub dimmings: u32,
    pub dim_time: Duration,
    pub dim_longest: Duration,
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
    /// A decision while the battle is dimmed: the tick to press A on, if
    /// any ([`dimmed`]).
    Dimmed,
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
    /// The live ticks past [`STALE`] the chip in hand has gone unused, and
    /// whether when to push it out has been chosen already
    /// ([`Search::stale_wait`]: it is pushed out now).
    stale: u32,
    stale_timed: bool,
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
    /// A dimming's press, of a question of one.
    dimmed: Option<DimAnswer>,
    /// Of a decision in the fight: where a chip held long would first be
    /// pushed out ([`STALE`]), the ticks of the fight to wait till it is
    /// (0: now), where using it then costs least ([`Search::stale_wait`]).
    stale_wait: Option<u32>,
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

/// A copy being played for a plan: its horizon in ticks of the fight, a
/// dimming's not counted; [`AFTER_DIMMING`] ticks of the fight at least
/// after a dimming ends; [`DIMMED_MOST`] dimmed ticks at most. Out of a
/// dimming each side presses as it plays; in one, as its dimmer (both
/// aim: [`dimmed`]), the fighter's side holding B, each side's play going
/// on as it would, its presses aside.
struct Play {
    side: u8,
    until: u32,
    live: u32,
    dimmed: u32,
    most: u32,
    dims: Dimmed,
}

impl Play {
    fn new(side: u8, horizon: u32) -> Play {
        Play { side, until: horizon, live: 0, dimmed: 0, most: DIMMED_MOST, dims: Dimmed::aiming() }
    }

    /// Whether it plays on in `b` (`warm`: whatever the battle's state).
    fn goes_on(&self, b: &Battle, warm: bool) -> bool {
        self.live < self.until && self.dimmed < self.most && (warm || fight_goes_on(b))
    }

    /// A tick of `b`: out of a dimming, `mine` and `theirs`; in one, its
    /// side presses `cut` besides (A: a cut-in, where it may cut in).
    fn tick(&mut self, b: &mut Battle, memo: &mut Memo, ticks: &mut u64, cut: u16, mine: impl FnOnce(&Battle) -> u16, theirs: impl FnOnce(&Battle) -> u16) {
        let was = b.is_dimmed();
        if was && fight_goes_on(b) {
            let mut held = [0; 2];
            held[self.side as usize] = keys::B;
            // (Each side's play goes on as it would, its presses aside, as live.)
            let _ = (mine(b), theirs(b));
            self.dims.tick(b, self.side, held, cut, memo, ticks);
        } else {
            let (m, t) = (mine(b), theirs(b));
            step(b, self.side, m, t);
            *ticks += 1;
        }
        if b.is_dimmed() {
            self.dimmed += 1;
        } else {
            self.live += 1;
            if was {
                self.until = self.until.max(self.live + AFTER_DIMMING);
            }
        }
    }
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
    fn score(&self, i: usize, trial: &mut Battle, memo: &mut Memo) -> (i64, u64) {
        let (mut dealt, mut taken, mut lost, mut ticks, mut count) = (0, 0, 0, 0u64, 0);
        for (guess, ahead, alike) in &self.aheads {
            trial.clone_from(ahead);
            let mut exec = self.plans[i];
            let mut guess = *guess;
            let mut play = Play::new(self.side, self.horizon);
            // (A plan that reacts heeds the threats the guess shows, those of the ticks the decision takes too.)
            exec.heed(trial, self.side, &guess);
            while play.goes_on(trial, self.warm) {
                let cut = if exec.cuts_in(trial, self.side) { keys::A } else { 0 };
                play.tick(trial, memo, &mut ticks, cut, |b| exec.buttons(b, self.side), |b| guess.buttons(b, self.side ^ 1));
                exec.heed(trial, self.side, &guess);
            }
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
    fn share(&self, share: usize, of: usize, trial: &mut Battle, memo: &mut Memo) -> (Vec<(usize, i64)>, u64) {
        let mut ticks = 0;
        let worths = (share..self.plans.len())
            .step_by(of)
            .map(|i| {
                let (w, t) = self.score(i, trial, memo);
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
    jobs: std::sync::mpsc::Sender<(std::sync::Arc<Work>, usize, usize)>,
    worths: std::sync::mpsc::Receiver<(Vec<Done>, u64)>,
}

/// What a search's threads share out: a decision's plans, a dimming's
/// presses, or a custom screen's hands.
enum Work {
    Plans(Job),
    Presses(Box<Branches>),
    Hands(Box<Hands>),
}

/// An item of a work done: its index, what it is worth, and the work it
/// took as a custom screen's budget counts it ([`Hands`]; else none).
type Done = (usize, i64, u64);

impl Work {
    fn len(&self) -> usize {
        match self {
            Work::Plans(job) => job.plans.len(),
            Work::Presses(b) => b.pick.len(),
            Work::Hands(h) => h.hands.len(),
        }
    }

    /// A battle to make a thread's copy from.
    fn first(&self) -> &Battle {
        match self {
            Work::Plans(job) => &job.aheads[0].1,
            Work::Presses(b) => &b.from,
            Work::Hands(h) => &h.from,
        }
    }

    fn share(&self, share: usize, of: usize, trial: &mut Battle, memo: &mut Memo) -> (Vec<Done>, u64) {
        let (worths, ticks) = match self {
            Work::Plans(job) => job.share(share, of, trial, memo),
            Work::Presses(b) => b.share(share, of, trial, memo),
            Work::Hands(h) => return h.share(share, of, trial, memo),
        };
        (worths.into_iter().map(|(i, w)| (i, w, 0)).collect(), ticks)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Helper {
    fn spawn() -> Option<Helper> {
        let (jobs, asked) = std::sync::mpsc::channel::<(std::sync::Arc<Work>, usize, usize)>();
        let (tell, worths) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("nettai-training helper".into())
            .spawn(move || {
                let mut trial: Option<Battle> = None;
                let mut memo = Memo::default();
                // (Until the search is gone.)
                while let Ok((work, share, of)) = asked.recv() {
                    let trial = trial.get_or_insert_with(|| work.first().clone());
                    if tell.send(work.share(share, of, trial, &mut memo)).is_err() {
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
    /// Its thread's aims found ([`dimmed::aim`]).
    memo: Memo,
    #[cfg(not(target_arch = "wasm32"))]
    helpers: Vec<Helper>,
}

impl Search {
    /// One with `threads` in all trying plans (this one's and helpers).
    fn with(threads: usize) -> Search {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let helpers = (1..threads.max(1)).map_while(|_| Helper::spawn()).collect();
            Search { trial: None, memo: Memo::default(), helpers }
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
                Answer { id: q.id, tried: Vec::new(), guesses: 1, worths: Vec::new(), form: None, dimmed: None, ticks: 0, took: start.elapsed(), stale_wait: None }
            }
            Asked::Chips(chips) => self.worths(q, chips, start),
            Asked::Decide => self.decide(q, start),
            Asked::Dimmed => self.dimmed(q, start),
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
    #[allow(clippy::too_many_arguments)]
    fn aheads(&mut self, b: &Battle, side: u8, now: Exec, theirs: u16, past: Past, guesses: &[Guess], ticks: u32) -> (Vec<(Guessed, Battle, i64)>, Exec, u64) {
        let mut spent = 0;
        let mut followed = now;
        let mut aheads: Vec<(Guessed, Battle, i64)> = Vec::with_capacity(guesses.len());
        let mut kinds: Vec<u64> = Vec::with_capacity(guesses.len());
        for &g in guesses {
            let mut ahead = b.clone();
            let mut now = now;
            let mut guess = Guessed::new(g, theirs, past);
            let mut t = 0;
            let mut play = Play::new(side, 0);
            while t < ticks && fight_goes_on(&ahead) {
                play.tick(&mut ahead, &mut self.memo, &mut spent, 0, |b| now.buttons(b, side), |b| guess.buttons(b, side ^ 1));
                t += 1;
            }
            if aheads.is_empty() {
                followed = now;
            }
            // (By the battle each leaves, the guesses that press nothing more play out alike: tried once. A threat
            // shown or not is what a plan that reacts heeds: alike too.)
            let digest = ahead.digest();
            match guess.idle().then(|| kinds.iter().position(|d| *d == digest)).flatten() {
                Some(at) if aheads[at].0.idle() && aheads[at].0.struck.is_some() == guess.struck.is_some() => aheads[at].2 += 1,
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
    /// a shot and the chip in hand, each followed by the safest panel; and
    /// where it stands and on the safest, the chip held to react with.
    fn decide(&mut self, q: &Question, start: Instant) -> Answer {
        let (b, side, level) = (&*q.battle, q.side, q.level);
        let guesses = &GUESSES[..(level.guesses as usize).clamp(1, GUESSES.len())];
        let (aheads, followed, mut ticks) = self.aheads(b, side, q.now, q.theirs, q.past, guesses, level.reaction);
        let ahead = &aheads[0].1;
        let panels = reachable(ahead, side);
        if panels.is_empty() || !fight_goes_on(ahead) {
            return Answer { id: q.id, tried: Vec::new(), guesses: 1, worths: Vec::new(), form: None, dimmed: None, ticks, took: start.elapsed(), stale_wait: None };
        }
        let chip = ahead.hands[side as usize].remaining() > 0;
        // (Whether the chip in hand bars another behind it: one not its like.)
        let hand = &ahead.hands[side as usize];
        let next = hand.cursor as usize;
        let bars = hand.ids[next..].iter().map_while(|id| *id).skip(1).any(|id| Some(id) != hand.ids[next]);
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
        let mut attacks = Vec::with_capacity(by_worth.len() * 2 + 2);
        for &i in &by_worth {
            let (to, after) = (panels[i].0, after(i));
            attacks.push(Exec::new(Plan { to, then: Then::Shot, after }));
            if chip {
                attacks.push(Exec::new(Plan { to, then: Then::Chip, after }));
                // (Holding it to react, where it stands and on the safest panel: what using it now is weighed
                // against.)
                if i == 0 || Some(i) == safest {
                    attacks.push(Exec::reacting(Plan { to, then: Then::React, after }, level.reaction));
                }
            }
        }
        let Job { aheads, plans: stands, .. } = job;
        let job = Job { aheads, plans: attacks, side, horizon: level.horizon, worth, warm: false };
        let (mut attack_worths, t) = self.run(&job);
        ticks += t;
        // (A chip held a long while is used where it does best, though it takes nothing away or costs some: it would
        // bar the chips behind it (the hand is used in order), or it does what no HP shows. The longer, the more it
        // costs to hold. But not a chip with none behind it but its like (it bars nothing), nor one that holding to
        // react with pays (a plan that reacts worth more than standing: its worth is in reacting, and it is kept for
        // that).)
        let mut followed = (stands[stands.len() - 1], stand_worths[stands.len() - 1]);
        let most_of = |then: Then, worths: &[i64], plans: &[Exec]| plans.iter().zip(worths).filter(|(e, _)| e.plan.then == then).map(|(_, w)| *w).max();
        let reacts = most_of(Then::React, &attack_worths, &job.plans).is_some_and(|r| stand_worths.iter().all(|w| r > *w));
        let count = stand_count(&job);
        // (But not where using it a while on costs less, the first time it would be: a chip whose use now is
        // wasted, its like's effect still up, waits till it isn't, and is pushed out then.)
        let mut stale_wait = None;
        let eager = if q.stale > 0 && bars && !reacts {
            let wait = if q.stale_timed { 0 } else { self.stale_wait(&job.aheads[0].1, side, q.past, &mut ticks) };
            stale_wait = (!q.stale_timed).then_some(wait);
            if wait == 0 { STALE_EAGER * (60 + q.stale as i64) / 60 * count } else { 0 }
        } else {
            0
        };
        // (A chip not yet pressed for costs what it is to keep it ([`SPENT`]): it is spent only where that makes
        // clearly more than keeping it. The push counts for the plan it follows too where that has pressed for the
        // chip a reaction time on and the chip isn't yet taken: else a push decided again finds the plan under way
        // worth no more than standing, and starts another, walking from panel to panel, never pressing.)
        for (e, w) in job.plans.iter().zip(attack_worths.iter_mut()).chain(std::iter::once((&followed.0, &mut followed.1))) {
            if e.plan.then == Then::Chip && !e.acted {
                *w += eager - if e.waited == 0 { SPENT * count } else { 0 };
            }
        }
        // (Those that keep the chip first: a chip is spent only where that makes more than keeping it. Then the
        // nearest, standing before a shot before reacting before a chip; the plan it follows last.)
        let mut tried: Vec<(Exec, i64)> = stands[..stands.len() - 1].iter().copied().zip(stand_worths).chain(job.plans.iter().copied().zip(attack_worths)).collect();
        tried.sort_by_key(|(e, _)| (e.plan.then == Then::Chip, steps_to(e.plan.to), e.plan.then));
        tried.push(followed);
        let guesses = stand_count(&job);
        Answer { id: q.id, tried, guesses, worths: Vec::new(), form: None, dimmed: None, ticks, took: start.elapsed(), stale_wait }
    }

    /// When a chip held long ([`STALE`]) that bars a chip behind it is best
    /// pushed out, in ticks of the fight from `from` (0: now): it is used
    /// now and after each [`STALE_STEP`] more of [`STALE_WAITS`], standing
    /// where it stands, each copy played as long ([`STALE_LOOK`] past the
    /// last), the other player fighting on ([`Guess::Fights`]: what comes
    /// over a longer while, not the next moment); each is worth what it
    /// makes less what waiting costs (the chips behind it barred,
    /// [`STALE_BARS`] a second), and the earliest of the most is taken.
    /// It is chosen once for a chip held long: it is pushed out at the
    /// moment chosen, not looked for again (another copy could always find a
    /// better moment a while on, and the chips behind it would wait for
    /// ever). So a chip used where its use now
    /// is wasted (its like's effect still up: Invisibl while invisible, a
    /// Barrier while its barrier stands, AntiDmg while its trap is set,
    /// where a use refreshes or replaces what stands) waits till using it
    /// makes more, as its like's effect ends; one whose use makes as much
    /// whenever (an attack that can't land) goes at once.
    fn stale_wait(&mut self, from: &Battle, side: u8, past: Past, ticks: &mut u64) -> u32 {
        let Some(at) = navi_at(from, side) else {
            return 0;
        };
        let plans = (0..=STALE_WAITS).map(|k| Exec::later(Plan { to: at, then: Then::Chip, after: None }, k * STALE_STEP)).collect();
        let job = Job { aheads: vec![(Guessed::new(Guess::Fights, 0, past), from.clone(), 1)], plans, side, horizon: STALE_WAITS * STALE_STEP + STALE_LOOK, worth: Worth::Net, warm: false };
        let (worths, t) = self.run(&job);
        *ticks += t;
        let waited = |k: u32| worths[k as usize] - STALE_BARS * (k * STALE_STEP) as i64 / 60;
        (0..=STALE_WAITS).max_by_key(|&k| (waited(k), std::cmp::Reverse(k))).map_or(0, |k| k * STALE_STEP)
    }

    /// A decision while the battle is dimmed: from the battle as it stands,
    /// each press of A its side may make (a reaction time on, at the
    /// earliest) that changes anything, played through the dimming and a
    /// moment after ([`dimmed::scan`], [`dimmed::Look::live`]), the other
    /// side aiming its own effects as they start, and its own too where a
    /// cut-in starts one. The press that makes most (the earliest of
    /// those), where it makes more than none (a cut-in by
    /// [`dimmed::CUT_IN_MARGIN`]).
    fn dimmed(&mut self, q: &Question, start: Instant) -> Answer {
        let (b, side) = (&*q.battle, q.side);
        let mut held = [0; 2];
        held[side as usize] = keys::B;
        held[(side ^ 1) as usize] = q.theirs;
        let look = dimmed::Look::live(q.level.reaction);
        let mut ticks = 0;
        let scan = dimmed::scan(b, side, held, dimmed::live_dimmers(b, side), &look, &mut self.memo, &mut ticks);
        let cuts: Vec<bool> = scan.starts.iter().map(|s| s.cut).collect();
        let at: Vec<u32> = scan.starts.iter().map(|s| s.t).collect();
        let starts = std::sync::Arc::new(scan.starts);
        // (What each makes more than none: an aim as much will do (sooner than the effect's own time running out), a
        // cut-in by a margin.)
        let gain = |i: usize, worth: i64| worth - scan.never - if cuts[i] { dimmed::CUT_IN_MARGIN } else { 0 };
        let mut gains: Vec<Option<i64>> = vec![None; at.len()];
        let tried = |search: &mut Search, pick: Vec<usize>, gains: &mut [Option<i64>], ticks: &mut u64| {
            if pick.is_empty() {
                return;
            }
            let work = Branches { from: b.clone(), starts: starts.clone(), pick: pick.clone(), side, held, look };
            let (worths, t) = search.run_work(Work::Presses(Box::new(work)));
            *ticks += t;
            for (i, w) in pick.into_iter().zip(worths) {
                gains[i] = Some(gain(i, w));
            }
        };
        // (The cut-in and every few aims first, then the aims around the best of those.)
        let first = at.iter().zip(&cuts).find(|(_, c)| !**c).map_or(0, |(t, _)| *t);
        let coarse = (0..at.len()).filter(|&i| cuts[i] || (at[i] - first).is_multiple_of(dimmed::COARSE)).collect();
        tried(self, coarse, &mut gains, &mut ticks);
        let best_of = |gains: &[Option<i64>]| (0..at.len()).filter_map(|i| gains[i].map(|g| (i, g))).max_by_key(|&(i, g)| (g, std::cmp::Reverse(at[i])));
        if let Some((i, _)) = best_of(&gains).filter(|&(i, _)| !cuts[i]) {
            let around = (0..at.len()).filter(|&j| !cuts[j] && gains[j].is_none() && at[j].abs_diff(at[i]) < dimmed::COARSE).collect();
            tried(self, around, &mut gains, &mut ticks);
        }
        // (The earliest of the most.)
        let press = best_of(&gains).filter(|&(i, g)| if cuts[i] { g > 0 } else { g >= 0 }).map(|(i, _)| at[i]);
        let sigs = match press {
            Some(t) => scan.sigs[..=(t as usize).min(scan.sigs.len() - 1)].to_vec(),
            None => scan.sigs,
        };
        Answer { id: q.id, tried: Vec::new(), guesses: 1, worths: Vec::new(), form: None, dimmed: Some(DimAnswer { press, sigs }), ticks, took: start.elapsed(), stale_wait: None }
    }

    /// Each of `job`'s plans tried, on this thread and its helpers' (each a
    /// share): what each is worth, by plan, and the ticks that took.
    fn run(&mut self, job: &Job) -> (Vec<i64>, u64) {
        self.run_work(Work::Plans(job.clone()))
    }

    /// The same for any work: each item's worth, by item.
    fn run_work(&mut self, work: Work) -> (Vec<i64>, u64) {
        let (done, ticks) = self.run_items(work);
        (done.into_iter().map(|(w, _)| w).collect(), ticks)
    }

    /// The same, with the work each item took ([`Done`]).
    fn run_items(&mut self, work: Work) -> (Vec<(i64, u64)>, u64) {
        let trial = self.trial.get_or_insert_with(|| work.first().clone());
        let mut worths = vec![(0, 0); work.len()];
        let mut ticks = 0;
        #[cfg(not(target_arch = "wasm32"))]
        {
            // (Each helper tries its share as this one tries its own: the worths go by item, whoever tried it.)
            let of = self.helpers.len() + 1;
            let shared = std::sync::Arc::new(work);
            for (n, helper) in self.helpers.iter().enumerate() {
                helper.jobs.send((shared.clone(), n + 1, of)).expect("the helper is there");
            }
            let (mine, t) = shared.share(0, of, trial, &mut self.memo);
            ticks += t;
            for (i, w, n) in mine {
                worths[i] = (w, n);
            }
            for helper in &self.helpers {
                let (theirs, t) = helper.worths.recv().expect("the helper is there");
                ticks += t;
                for (i, w, n) in theirs {
                    worths[i] = (w, n);
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let (mine, t) = work.share(0, 1, trial, &mut self.memo);
            ticks += t;
            for (i, w, n) in mine {
                worths[i] = (w, n);
            }
        }
        (worths, ticks)
    }

    /// How its side picks on the custom screen `q` shows: by slot, the
    /// order to pick them in (the higher first; none below 0 is picked),
    /// and the form to take.
    ///
    /// A hand is worth what the fight makes with it over a turn ([`Hands`]:
    /// its chips used in the order picked, each when using it pays), less
    /// what the same turn makes with no new hand (the old one kept, as the
    /// game keeps it: picking nothing keeps the hand, picking anything
    /// replaces it). The hand is built a chip at a time: from a first pick,
    /// the chip whose addition raises the hand's worth most goes after the
    /// others (it is used after them), while one raises it at all (the rest
    /// go after them: below). Picking one can make others
    /// unpickable (a code that goes with none of theirs), so each chip worth
    /// something alone is tried as the first pick, the worthiest first (but
    /// one a hand already built holds: it would build that hand again), and
    /// the hand worth most wins. A chip worth nothing alone is also tried
    /// right after the hand's first and second picks: it may be one the hand
    /// folds into the chip before it as OK builds the hand (an attack-up:
    /// Atk+10, Navi+20, EXE6's Uninstll; the engine's hand builder,
    /// `custom::builder`). Where the copy's hand after OK still holds such a
    /// chip as an entry of its own ([`stands_alone`]: picked first, or after
    /// a chip it doesn't attach to, it would poof as it is used), the hand
    /// isn't one to pick ([`UNPICKABLE`]): so such a chip is picked only
    /// right after a chip it folds into, and only where that raises the
    /// hand's worth. A Program Advance the picks form on the copy is played
    /// in their place, as OK forms it.
    ///
    /// Then every other chip is picked after the hand (the worthiest alone
    /// first), as a player picks all the codes allow: a chip left is dealt
    /// again first next turn, in the place of a new one, and one picked and
    /// not used costs nothing once the old hand is spent. Only a chip that
    /// alone clearly costs the turn ([`LEAVE`]) is left, and one that can't
    /// be picked alone (a chip that folds, which would stand alone after the
    /// hand: it stays on the screen, dealt again first next turn, where a
    /// chip it folds into may be dealt with it).
    ///
    /// A chip that adds nothing in a round isn't tried again from that
    /// first (what a chip adds rarely grows as the hand does). The work is
    /// the level's ([`Level::budget`], in the ticks [`Hands::value`]
    /// counts): of a round's additions, the worthiest alone first, as many
    /// are tried as the work done leaves room for (by what a hand has taken
    /// on average), and where that isn't all of them, the search ends with
    /// that round. So the same screen gives the same hand on any machine,
    /// however fast.
    fn worths(&mut self, q: &Question, chips: &[(u8, FolderChip)], start: Instant) -> Answer {
        let mut ticks = 0;
        let mut valued = Valued::default();
        let budget = q.level.budget as u64;
        let chip_at = |slot: u8| chips.iter().find(|(s, _)| *s == slot).map(|(_, c)| *c);
        // (No new hand, and each chip alone: two slots dealt the same chip are one.)
        let firsts: Vec<u8> = chips.iter().enumerate().filter(|(n, (_, c))| !chips[..*n].iter().any(|(_, d)| d == c)).map(|(_, (s, _))| *s).collect();
        let tried: Vec<Vec<u8>> = std::iter::once(Vec::new()).chain(firsts.iter().map(|s| vec![*s])).collect();
        let w = self.value(q, chips, &tried, &mut valued, &mut ticks);
        let none = w[0];
        let alone: Vec<(u8, i64)> = firsts.iter().zip(&w[1..]).map(|(s, w)| (*s, if *w == UNPICKABLE { UNPICKABLE } else { w - none })).collect();
        let alone_of = |slot: u8| alone.iter().find(|(s, _)| chip_at(*s) == chip_at(slot)).map_or(UNPICKABLE, |(_, w)| *w);
        let mut order: Vec<(u8, i64)> = alone.iter().copied().filter(|(_, w)| *w > 0).collect();
        order.sort_by_key(|&(s, w)| (std::cmp::Reverse(w), s));
        // (The hand worth most, and the chips of every hand built.)
        let mut best: (Vec<u8>, i64) = (Vec::new(), 0);
        let mut built: Vec<FolderChip> = Vec::new();
        let mut spent = false;
        for (first, alone) in order {
            if spent || chip_at(first).is_some_and(|c| built.contains(&c)) {
                continue;
            }
            let (mut hand, mut worth) = (vec![first], alone);
            // (The chips that added nothing in a round: they aren't tried again for this first.)
            let mut out: Vec<u8> = Vec::new();
            while hand.len() < MAX_SELECTIONS {
                // (Each chip not in the hand (the first slot of each chip dealt) after the others, the worthiest alone
                // first; one worth nothing alone also right after the first and the second pick.)
                let mut adds: Vec<(u8, Vec<u8>)> = Vec::new();
                for &(slot, chip) in chips {
                    if hand.contains(&slot) || out.contains(&slot) || chips.iter().any(|(s, c)| *c == chip && *s < slot && !hand.contains(s)) {
                        continue;
                    }
                    let mut places = vec![hand.len()];
                    if alone_of(slot) <= 0 {
                        places.extend((1..=hand.len().min(2)).filter(|at| *at < hand.len()));
                    }
                    for at in places {
                        let mut h = hand.clone();
                        h.insert(at, slot);
                        adds.push((slot, h));
                    }
                }
                adds.sort_by_key(|(s, h)| (std::cmp::Reverse(alone_of(*s)), *s, std::cmp::Reverse(h.iter().position(|o| o == s))));
                // (As many as the budget leaves room for, by what a hand has taken on average: none, and the hand is as
                // it is, and no other first is tried.)
                let room = budget.saturating_sub(valued.work) / (valued.work / valued.hands.max(1)).max(1);
                let mut fit = 0;
                while fit < adds.len() && valued.unknown(chips, &adds[..fit + 1].iter().map(|(_, h)| h.clone()).collect::<Vec<_>>()) as u64 <= room {
                    fit += 1;
                }
                spent = fit < adds.len();
                adds.truncate(fit);
                if adds.is_empty() {
                    break;
                }
                let hands: Vec<Vec<u8>> = adds.iter().map(|(_, h)| h.clone()).collect();
                let w = self.value(q, chips, &hands, &mut valued, &mut ticks);
                let mut most: Option<(usize, i64)> = None;
                for (i, w) in w.iter().enumerate() {
                    if *w != UNPICKABLE && most.is_none_or(|(_, m)| *w > m) {
                        most = Some((i, *w));
                    }
                }
                for (slot, _) in &adds {
                    let added = adds.iter().zip(&w).filter(|((s, _), w)| s == slot && **w != UNPICKABLE).map(|(_, w)| w - none - worth).max();
                    if added.is_none_or(|a| a <= 0) {
                        out.push(*slot);
                    }
                }
                let Some((i, w)) = most.filter(|(_, w)| w - none > worth) else {
                    break;
                };
                (hand, worth) = (adds.swap_remove(i).1, w - none);
                if spent {
                    break;
                }
            }
            built.extend(hand.iter().filter_map(|s| chip_at(*s)));
            if worth > best.1 {
                best = (hand, worth);
            }
        }
        // (Then every other chip, the worthiest alone first, but one that clearly costs the turn, or that alone
        // can't be picked (a chip OK folds into the chip before it, standing alone: [`stands_alone`]): a player picks
        // all the codes allow. A chip left is dealt again first next turn, in the place of a new one; one picked and
        // not used costs nothing once the old hand is spent. The codes the hand's picks leave decide which can be
        // picked: the picker skips one that isn't selectable by then.)
        let mut order = best.0;
        let mut rest: Vec<(u8, i64)> = firsts.iter().filter(|s| !order.iter().any(|o| chip_at(*o) == chip_at(**s))).map(|s| (*s, alone_of(*s))).filter(|(_, w)| *w >= -LEAVE).collect();
        rest.sort_by_key(|&(s, w)| (std::cmp::Reverse(w), s));
        // (Two slots dealt the same chip: each is picked, after the first; but a chip OK folds into the chip before
        // it, which would stand alone there.)
        for (slot, _) in rest {
            order.push(slot);
        }
        for &(slot, chip) in chips {
            if !order.contains(&slot) && order.iter().any(|o| chip_at(*o) == Some(chip)) && alone_of(slot) != UNPICKABLE {
                order.push(slot);
            }
        }
        // (The order as worths by slot: the first pick the highest; a chip not picked, below 0.)
        let ranks: Vec<i64> = chips.iter().map(|(s, _)| order.iter().position(|o| o == s).map_or(-1, |at| (order.len() - at) as i64)).collect();
        let form = if q.level.forms { self.form(q, chips, &ranks, &mut ticks) } else { None };
        Answer { id: q.id, tried: Vec::new(), guesses: 1, worths: ranks, form, dimmed: None, ticks, took: start.elapsed(), stale_wait: None }
    }

    /// What each of `hands` (the slots of the screen `q` shows, in the order
    /// picked) makes over a turn ([`Hands`]), those not yet valued tried on
    /// this thread and its helpers'.
    fn value(&mut self, q: &Question, chips: &[(u8, FolderChip)], hands: &[Vec<u8>], valued: &mut Valued, ticks: &mut u64) -> Vec<i64> {
        let mut new: Vec<Vec<u8>> = Vec::new();
        for h in hands {
            let key = Valued::key(chips, h);
            if !valued.worths.contains_key(&key) && !new.iter().any(|n| Valued::key(chips, n) == key) {
                new.push(h.clone());
            }
        }
        if !new.is_empty() {
            let guesses = [Guess::Holds, Guess::Chip, Guess::Up, Guess::Down][..(q.level.guesses as usize).clamp(1, 4)].to_vec();
            let keys: Vec<Vec<FolderChip>> = new.iter().map(|h| Valued::key(chips, h)).collect();
            let work = Hands { from: (*q.battle).clone(), side: q.side, guesses, hands: new, reaction: q.level.reaction };
            let (done, t) = self.run_items(Work::Hands(Box::new(work)));
            *ticks += t;
            for (key, (w, n)) in keys.into_iter().zip(done) {
                valued.worths.insert(key, w);
                valued.work += n;
                valued.hands += 1;
            }
        }
        hands.iter().map(|h| valued.worths[&Valued::key(chips, h)]).collect()
    }

    /// The form to take on the custom screen `q` shows, if any makes the
    /// fight better: each way the screen's keys put the navi in another form
    /// (walked in copies, [`forms_on_a_copy`]), from the screen as dealt
    /// and from it with the picks made (`ranks`), played through OK and a few
    /// seconds of the fight ([`rollout`]) against the picks alone.
    fn form(&mut self, q: &Question, chips: &[(u8, FolderChip)], ranks: &[i64], ticks: &mut u64) -> Option<FormChoice> {
        let side = q.side;
        let worth = |slot: u8| chips.iter().position(|(s, _)| *s == slot).map(|at| ranks[at]);
        let (_, picked) = picks_on_a_copy(&q.battle, side, worth, ticks);
        let base = to_the_fight(&picked, side, &worth, ticks).map(|b| rollout(&b, side, &mut self.memo, ticks))?;
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
                let made = rollout(&b, side, &mut self.memo, ticks);
                if made > base + FORM_MARGIN && best.as_ref().is_none_or(|(_, w)| made > *w) {
                    best = Some((FormChoice { script: goal.script, before }, made));
                }
            }
        }
        best.map(|(f, _)| f)
    }
}

/// The live ticks of the fight a hand is played out for ([`turn`]): a
/// turn, as short as it can be. The custom gauge fills in 512 ticks of the
/// fight (docs/engine/battle-flow.md §7.1: 0x20 a tick to 0x4000; the chip
/// effects that fill it at once aside) and the screen opens only once it is
/// full, the player opening it (the fighter never does), so a turn lasts
/// that long at least, and in live play often longer.
const TURN: u32 = 512;
/// The live ticks a copy tries using the next chip now, and not, for
/// ([`TurnPlay`]): long enough for a navi chip's attack.
const PROBE: u32 = 120;
/// The live ticks before it tries the next chip again where using it now
/// didn't pay: at first, then twice as long each time, up to [`PROBE`].
const BEAT: u32 = 20;
/// A hand whose picks can't be made in its order (a code that goes with
/// none of the others').
const UNPICKABLE: i64 = i64::MIN;
/// How much a chip alone may cost the turn, in tenths of HP, and still be
/// picked after the hand ([`Search::worths`]): about nothing, as a copy's
/// turns differ by a few HP whatever is picked.
const LEAVE: i64 = 100;

/// The hands a custom screen's search has valued ([`Search::worths`]), by
/// their chips in order (two slots dealt the same chip make the same
/// hand), and the work that took, in the ticks [`Hands`] counts.
#[derive(Default)]
struct Valued {
    worths: std::collections::HashMap<Vec<FolderChip>, i64>,
    work: u64,
    hands: u64,
}

impl Valued {
    fn key(chips: &[(u8, FolderChip)], hand: &[u8]) -> Vec<FolderChip> {
        hand.iter().filter_map(|s| chips.iter().find(|(t, _)| t == s).map(|(_, c)| *c)).collect()
    }

    /// How many of `hands` aren't valued yet (each once).
    fn unknown(&self, chips: &[(u8, FolderChip)], hands: &[Vec<u8>]) -> usize {
        let mut seen: Vec<Vec<FolderChip>> = Vec::new();
        for h in hands {
            let key = Valued::key(chips, h);
            if !self.worths.contains_key(&key) && !seen.contains(&key) {
                seen.push(key);
            }
        }
        seen.len()
    }
}

/// A custom screen's hands to value, for the search's threads: each the
/// slots to pick on the screen `from` shows, in order; picked on a copy,
/// both screens through OK (the other player picking every chip it can),
/// and the turn played out against each guess at the other player
/// ([`turn`]). A hand is worth what its turns make in all
/// ([`UNPICKABLE`] where its picks can't be made in its order, or where
/// OK leaves a chip that folds standing alone: [`stands_alone`]).
struct Hands {
    from: Battle,
    side: u8,
    guesses: Vec<Guess>,
    hands: Vec<Vec<u8>>,
    /// The level's reaction time ([`TurnPlay`] reacts so).
    reaction: u32,
}

impl Hands {
    /// Hands `share`, `share + of` and so on: by index, what each is worth
    /// and the work it took ([`Hands::value`]), and every tick that took.
    fn share(&self, share: usize, of: usize, trial: &mut Battle, memo: &mut Memo) -> (Vec<Done>, u64) {
        let mut ticks = 0;
        let done = (share..self.hands.len())
            .step_by(of)
            .map(|i| {
                let (w, n) = self.value(&self.hands[i], trial, memo, &mut ticks);
                (i, w, n)
            })
            .collect();
        (done, ticks)
    }

    /// What the hand `slots` makes over a turn, and the work that took: the
    /// ticks of the screen and of the fight played out, not of the aims a
    /// dimming's copies search ([`dimmed::aim`]), which a thread may have
    /// found before (its memo): the work is the same on any thread, so the
    /// budget is spent alike on any machine.
    fn value(&self, slots: &[u8], trial: &mut Battle, memo: &mut Memo, ticks: &mut u64) -> (i64, u64) {
        let mut work = 0;
        let rank = |s: u8| Some(slots.iter().position(|o| *o == s).map_or(-1, |at| (slots.len() - at) as i64));
        let (picks, copy) = picks_on_a_copy(&self.from, self.side, rank, &mut work);
        if picks != slots {
            *ticks += work;
            return (UNPICKABLE, work);
        }
        let b = to_the_fight(&copy, self.side, &rank, &mut work);
        *ticks += work;
        let Some(b) = b else {
            return (0, work);
        };
        // (A chip that OK folds into the chip before it, standing in the hand as an entry of its own, didn't fold:
        // the chip before it isn't one it attaches to, or there is none. Used, it does nothing, and it is gone (it
        // poofs): a hand with one isn't a hand to pick. A hand that picks nothing keeps the old one as it is.)
        if !slots.is_empty() && stands_alone(&b, self.side) {
            return (UNPICKABLE, work);
        }
        let mut made = 0;
        for &g in &self.guesses {
            trial.clone_from(&b);
            made += turn(trial, &b, self.side, Guessed::new(g, 0, Past([0; PAST])), self.reaction, memo, ticks, &mut work);
        }
        (made, work)
    }
}

/// Whether side `side`'s hand in `b`, from its cursor, holds a chip that
/// OK folds into the chip before it as it builds the hand (an attack-up,
/// EXE6's Uninstll: the content's `modifier`), as an entry of its own: the
/// builder (`custom::builder`) left it there, as nothing before it is a
/// chip it attaches to.
fn stands_alone(b: &Battle, side: u8) -> bool {
    let hand = &b.hands[side as usize];
    hand.ids.iter().skip(hand.cursor as usize).map_while(|id| *id).any(|id| b.content.chip(id).modifier.is_some())
}

/// What side `side` makes over a turn ([`TURN`]) from `b` (`from`, the
/// fight a custom screen opened on), the other player playing `guess`:
/// what it takes away less what it loses, in tenths of HP; its side played
/// as [`TurnPlay`] plays it, reacting `reaction` ticks after a threat.
/// `ticks`: every tick it took, the aims' too; `work`: the ticks played of
/// the turn and of the copies tried in it.
#[allow(clippy::too_many_arguments)]
fn turn(b: &mut Battle, from: &Battle, side: u8, guess: Guessed, reaction: u32, memo: &mut Memo, ticks: &mut u64, work: &mut u64) -> i64 {
    let (mut me, mut guess) = (TurnPlay::new(reaction), guess);
    let mut play = Play::new(side, TURN);
    while play.goes_on(b, false) {
        me.heed(&guess, play.live);
        if me.due(b, side, play.live) {
            me.choose(b, side, guess, play.live, memo, ticks, work);
        }
        let cut = me.cut_in(b, side, guess, memo, ticks, work);
        play.tick(b, memo, ticks, cut, |x| me.buttons(x, side), |x| guess.buttons(x, side ^ 1));
    }
    *work += (play.live + play.dimmed) as u64;
    let (dealt, lost) = outcome(from, b, side);
    dealt - lost
}

/// How a copy plays its side over a turn ([`turn`]): as [`Rollout`] does
/// (lined up with the other navi's row, B held), but its charged shot never
/// let go (the buster is the fight's whatever the hand: what a hand is
/// worth is what its chips do) and its chips held, the next in hand used
/// once that pays: where, tried in copies over the next [`PROBE`] ticks,
/// using it now (from where it stands, or from another panel of its row it
/// can reach) makes more than holding it to react with it (used a reaction
/// time after the other player's next attack starts, or cut in on their
/// dimming chip where it can: what a chip that defends is worth). Where it
/// doesn't, it is tried again a beat ([`BEAT`]) on, the beat twice as long
/// each time, and a reaction time after the other player's attack starts;
/// a chip that never pays is never used (the hand is worth no more with
/// it). So each chip of a hand is used in turn when it pays: once the other
/// navi is in reach, once the chip before it is done, once a panel opens,
/// once an attack is on its way. And on the other player's dimming chip, a
/// reaction time into its name, it cuts in with its next chip where it may
/// and that pays (tried in copies: played through the dimming and [`PROBE`]
/// ticks after, cut in and not), so a hand with a chip that cuts in at its
/// front is worth what that does, less what it does to the chips it bars
/// behind it while it waits.
#[derive(Clone, Copy, Debug)]
struct TurnPlay {
    roll: Rollout,
    /// The chip being used: walking to its panel and pressing A.
    using: Option<Exec>,
    /// The live tick the next chip is tried on at the earliest, and the
    /// beat after that.
    next: u32,
    beat: u32,
    /// Its reaction time; the live tick it reacts on to the attack it last
    /// saw start, and that attack's tick (the guess's).
    reaction: u32,
    alarm: Option<u32>,
    seen: Option<u32>,
    /// It uses the next chip as its alarm goes off, and cuts in where it
    /// may, a reaction time on (a copy holding its chip to react: no copies
    /// of its own).
    reacts: bool,
    /// The ticks the other player's dimming has shown a name it may cut in
    /// on, and whether it cuts in on it.
    window: u32,
    cutting: bool,
}

impl TurnPlay {
    fn new(reaction: u32) -> TurnPlay {
        TurnPlay { roll: Rollout { holds: true, ..Rollout::default() }, using: None, next: 0, beat: BEAT, reaction, alarm: None, seen: None, reacts: false, window: 0, cutting: false }
    }

    fn buttons(&mut self, b: &Battle, side: u8) -> u16 {
        if let Some(e) = self.using.as_mut() {
            let pressed = e.buttons(b, side);
            if e.acted {
                self.using = None;
            }
            return pressed;
        }
        self.roll.buttons(b, side)
    }

    /// The attack the guess shows starting, if it is new: its alarm set a
    /// reaction time on (from live tick `live`).
    fn heed(&mut self, guess: &Guessed, live: u32) {
        if guess.struck.is_some() && guess.struck != self.seen {
            self.seen = guess.struck;
            self.alarm = Some(live + self.reaction);
        }
    }

    /// Whether its alarm has gone off by live tick `live`.
    fn alarmed(&self, live: u32) -> bool {
        self.alarm.is_some_and(|a| live >= a)
    }

    /// Whether it tries the next chip in hand now: none being used, its
    /// beat come or its alarm gone off, and not between two steps of a
    /// walk.
    fn due(&self, b: &Battle, side: u8, live: u32) -> bool {
        self.using.is_none() && (live >= self.next || self.alarmed(live)) && self.roll.dir == 0 && !b.is_dimmed() && b.hands[side as usize].remaining() > 0
    }

    /// The next chip in hand tried: used where that makes most over the
    /// next [`PROBE`] ticks, if more than holding it to react; else tried
    /// again a beat on. (Its row's other panels only the first time it is
    /// tried: where it stands then is where it lined up.)
    #[allow(clippy::too_many_arguments)]
    fn choose(&mut self, b: &Battle, side: u8, guess: Guessed, live: u32, memo: &mut Memo, ticks: &mut u64, work: &mut u64) {
        let Some(at) = navi_at(b, side) else {
            return;
        };
        if self.alarmed(live) {
            self.alarm = None;
        }
        let mut panels = vec![at];
        if self.beat == BEAT {
            panels.extend(reachable(b, side).into_iter().filter(|(p, _)| p.y == at.y && *p != at).map(|(p, _)| p));
        }
        let mut best = (self.probe(b, side, guess, None, true, 0, memo, ticks, work), None);
        for to in panels {
            let e = Exec::new(Plan { to, then: Then::Chip, after: None });
            let made = self.probe(b, side, guess, Some(e), false, 0, memo, ticks, work);
            if made > best.0 {
                best = (made, Some(e));
            }
        }
        match best.1 {
            Some(e) => (self.using, self.beat) = (Some(e), BEAT),
            None => (self.next, self.beat) = (live + self.beat, (self.beat * 2).min(PROBE)),
        }
    }

    /// What it presses besides while dimmed (A: a cut-in): on the other
    /// player's dimming chip's name, where it may cut in, a reaction time
    /// into it, if a copy cut in makes clearly more than one not
    /// ([`TurnPlay::probe`], by [`dimmed::CUT_IN_MARGIN`], as live: a
    /// defense their chip goes through, a copy a few HP luckier, isn't spent
    /// on it); a copy holding its chip to react cuts in so whatever it
    /// makes.
    #[allow(clippy::too_many_arguments)]
    fn cut_in(&mut self, b: &Battle, side: u8, guess: Guessed, memo: &mut Memo, ticks: &mut u64, work: &mut u64) -> u16 {
        if !(b.is_dimmed() && fight_goes_on(b) && dimmed::cut_window(b, side)) {
            self.window = 0;
            return 0;
        }
        self.window += 1;
        if self.window <= self.reaction {
            return 0;
        }
        if self.window == self.reaction + 1 {
            self.cutting = self.reacts || self.probe(b, side, guess, None, false, keys::A, memo, ticks, work) > self.probe(b, side, guess, None, false, 0, memo, ticks, work) + dimmed::CUT_IN_MARGIN;
        }
        if self.cutting { keys::A } else { 0 }
    }

    /// What [`PROBE`] ticks from `b` make, `using` the chip so (else none,
    /// or held to react where it `reacts`), `cut` pressed on the first if it
    /// is dimmed, as in [`turn`].
    #[allow(clippy::too_many_arguments)]
    fn probe(&self, b: &Battle, side: u8, guess: Guessed, using: Option<Exec>, reacts: bool, cut: u16, memo: &mut Memo, ticks: &mut u64, work: &mut u64) -> i64 {
        let mut c = b.clone();
        let (mut me, mut guess) = (TurnPlay { using, reacts, ..*self }, guess);
        let mut play = Play::new(side, PROBE);
        let mut cut = cut;
        while play.goes_on(&c, false) {
            me.heed(&guess, play.live);
            // (Holding its chip to react: it uses it as its alarm goes off.)
            if me.reacts && me.using.is_none() && me.alarmed(play.live) && me.roll.dir == 0 && !c.is_dimmed() && c.hands[side as usize].remaining() > 0 {
                me.alarm = None;
                me.using = navi_at(&c, side).map(|to| Exec::new(Plan { to, then: Then::Chip, after: None }));
            }
            let extra = std::mem::take(&mut cut) | if me.reacts { me.cut_in(&c, side, guess, memo, ticks, work) } else { 0 };
            play.tick(&mut c, memo, ticks, extra, |x| me.buttons(x, side), |x| guess.buttons(x, side ^ 1));
        }
        *work += (play.live + play.dimmed) as u64;
        let (dealt, lost) = outcome(b, &c, side);
        dealt - lost
    }
}

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
fn rollout(b: &Battle, side: u8, memo: &mut Memo, ticks: &mut u64) -> i64 {
    let mut trial = b.clone();
    let [mut mine, mut theirs] = [Rollout::default(), Rollout::default()];
    let mut play = Play::new(side, ROLLOUT);
    play.most = 4 * DIMMED_MOST;
    while play.goes_on(&trial, false) {
        play.tick(&mut trial, memo, ticks, 0, |b| mine.buttons(b, side), |b| theirs.buttons(b, side ^ 1));
    }
    let (dealt, lost) = outcome(b, &trial, side);
    dealt - lost
}

/// A plain way to play a fight out: walk to the other navi's row, charge,
/// shoot once the charge is full, and use a chip there every so often
/// (unless it `holds` both: a turn's copy uses its chips by its own
/// reckoning and keeps its charge, [`TurnPlay`]; or `keeps_chips`: a
/// guess that fights on with its buster, [`Guess::Fights`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
struct Rollout {
    holds: bool,
    keeps_chips: bool,
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
        if charge(b, side) >= 2 && !self.holds {
            return 0;
        }
        if !self.holds && !self.keeps_chips && b.hands[side as usize].remaining() > 0 && self.ticks.is_multiple_of(30) && self.last & keys::A == 0 {
            return keys::A | keys::B;
        }
        keys::B
    }
}

/// The picks side `side` makes on a copy of the custom screen `from`
/// shows, by `worth` (the picker's: the higher first, none below 0), till
/// there is none left to pick; and the copy then.
fn picks_on_a_copy(from: &Battle, side: u8, worth: impl Fn(u8) -> Option<i64>, ticks: &mut u64) -> (Vec<u8>, Battle) {
    let (me, side) = (side, side as usize);
    let mut b = from.clone();
    let mut picker = Picker::default();
    for _ in 0..PICKING {
        let Some(mine) = picker.step(&b, side, Some(&worth), false) else {
            break;
        };
        step(&mut b, me, mine, 0);
        *ticks += 1;
    }
    let screen = b.custom.sides[side].screen;
    match screen {
        Some(shown) => (shown.selection[..shown.selected as usize].to_vec(), b),
        None => (Vec::new(), b),
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
    /// answers are dropped; and so are these questions' (a custom screen
    /// gone before its answer came).
    forgotten: u64,
    abandoned: Vec<u64>,
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
                return Thinker { how: How::Thread { ask, answers }, kept: Vec::new(), forgotten: 0, abandoned: Vec::new() };
            }
        }
        let _ = threads;
        Thinker::inline()
    }

    /// On the caller's thread.
    fn inline() -> Thinker {
        Thinker { how: How::Inline(Box::default()), kept: Vec::new(), forgotten: 0, abandoned: Vec::new() }
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
        self.abandoned.clear();
    }

    /// Drop the answer to question `id`, come or to come.
    fn abandon(&mut self, id: u64) {
        self.kept.retain(|a| a.id != id);
        self.abandoned.push(id);
    }

    /// Whether an answer to question `id` is kept when it comes.
    fn wanted(&self, id: u64) -> bool {
        id > self.forgotten && !self.abandoned.contains(&id)
    }

    /// The answer to question `id` if it has come.
    fn poll(&mut self, id: u64) -> Option<Answer> {
        #[cfg(not(target_arch = "wasm32"))]
        if let How::Thread { answers, .. } = &self.how {
            while let Ok(a) = answers.try_recv() {
                if self.wanted(a.id) {
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
                if self.wanted(a.id) {
                    self.kept.push(a);
                }
            },
        }
    }
}

/// The live ticks of the fight a chip stays in hand before it is used where
/// it does best, though no plan shows it taking anything away
/// ([`Question::stale`]), where it bars a chip not its like behind it and
/// holding it to react shows no worth; and what using it is worth then, in
/// tenths of HP, and as much again each second on.
const STALE: u32 = 180;
const STALE_EAGER: i64 = 50;
/// When such a chip is pushed out ([`Search::stale_wait`]): now, or after
/// each of a few steps of the fight's ticks, each copy played as long, and
/// so far past the last (long enough for an effect used now to have ended
/// where the last is still up: EXE6's Invisibl lasts 360 ticks).
const STALE_STEP: u32 = 60;
const STALE_WAITS: u32 = 4;
const STALE_LOOK: u32 = 360;
/// What waiting to push such a chip out costs, in tenths of HP a second:
/// the chips behind it barred a while longer (what they would make is
/// unknown till they are next: a little, less than a hit a few seconds
/// waiting avoids, as a chip whose use is wasted now makes no hit's worth
/// till its like's effect ends).
const STALE_BARS: i64 = 20;
/// What a chip in hand is worth kept, in tenths of HP, against what using
/// it now makes ([`Search::decide`]): a little more than what two copies of
/// the same moment differ by (a dimming's ticks more in one, the fight-time
/// HP drain), so a chip isn't spent for about nothing, and a chip whose
/// worth is in reacting is kept for a threat to react to.
const SPENT: i64 = 30;

/// Where it stands before it has seen its navi: no panel.
const NOWHERE: PanelPos = PanelPos { x: 0, y: 0 };

/// An opponent that fights, for the navi of one side.
pub struct Fighter {
    level: Level,
    seed: u64,
    rng: SplitMix64,
    /// What it does now.
    now: Exec,
    /// The decision asked and not yet taken: its question, the tick it
    /// takes effect on, and the chips in hand as it was asked.
    pending: Option<(u64, u32, u8)>,
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
    /// The live ticks the chip in hand is held before it counts as held
    /// long ([`STALE`], or later where pushing it out waits:
    /// [`Answer::stale_wait`]), and whether that moment has been chosen.
    stale_after: u32,
    stale_timed: bool,
    /// Its custom screen's: its presses, and the chips dealt with what each is
    /// worth ([`Level::chooses`]).
    picker: Picker,
    chosen: Option<Chosen>,
    /// It waits for its custom screen's answer once its look time is over,
    /// the battle with it (the tests' that play its thread tick for tick
    /// against its inline self), instead of looking on till it comes.
    waits_for_chips: bool,
    /// The dimming decision asked or taken ([`Dim`]).
    dim: Option<Dim>,
    thinker: Thinker,
    pub spent: Spent,
}

/// A decision while the battle is dimmed: its question, the tick it was
/// asked on and the tick it takes effect on, and once taken, what it
/// decided.
struct Dim {
    asked: u64,
    at: u32,
    take: u32,
    answer: Option<DimAnswer>,
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
            stale_after: STALE,
            stale_timed: false,
            last_hp: None,
            alerted: false,
            picker: Picker::default(),
            chosen: None,
            waits_for_chips: false,
            dim: None,
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
        (self.past, self.hand, self.held, self.refused, self.stale_after, self.stale_timed) = (Past([0; PAST]), 0, 0, false, STALE, false);
        self.picker = Picker::default();
        self.chosen = None;
        self.dim = None;
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
            stale: if self.refused || self.held < self.stale_after { 0 } else { self.held.saturating_sub(STALE - 1) },
            stale_timed: self.stale_timed,
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
        if self.now.plan.to == NOWHERE
            && let Some(at) = navi_at(b, side)
        {
            self.now = Exec::stand(at);
        }
        self.take_decision(b.hands[side as usize].remaining());
        if fight_goes_on(b) && b.is_dimmed() {
            // (The plan goes on as it would, its presses aside: what it waits for, it waits for still.)
            let _ = self.now.buttons(b, side);
            return self.dimmed(b, side, theirs);
        }
        self.dim = None;
        if !fight_goes_on(b) {
            return self.now.buttons(b, side);
        }
        self.since += 1;
        self.past.push(theirs);
        let hand = b.hands[side as usize].remaining();
        self.held = if hand != self.hand { 0 } else { self.held + 1 };
        if self.held == 0 {
            (self.stale_after, self.stale_timed) = (STALE, false);
        }
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
            self.pending = Some((self.asked, self.ticks + self.level.reaction, hand));
            let q = self.question(b, side, theirs, Asked::Decide);
            self.thinker.ask(q);
            self.take_decision(hand);
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
            // (A question not answered yet: its answer, when it comes, is dropped.)
            if let Some(Chosen { asked: Some(asked), worths: None, .. }) = self.chosen.take() {
                self.thinker.abandon(asked);
            }
            return self.picker.buttons(b, side as usize, None);
        };
        if self.chosen.is_none() {
            if screen.phase != Phase::Choosing {
                return 0;
            }
            let dealt = dealt(b, side);
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
            // (Then it has their worths once they have come: till then it looks on, the battle going on as it does
            // while a link's other player sits on their screen, never waiting for it. Its search's work is the
            // level's, not a time, so what it picks is the same however fast its threads are; only how long it
            // looks isn't. The tests that play it tick for tick against its inline self wait instead.)
            let answer = if self.waits_for_chips {
                let waiting = Instant::now();
                let (answer, late) = self.thinker.answer(asked);
                if late {
                    self.spent.late += 1;
                    self.spent.waited += waiting.elapsed();
                }
                Some(answer)
            } else {
                self.thinker.poll(asked)
            };
            let Some(answer) = answer else {
                chosen.looked += 1;
                self.spent.looked_longer += 1;
                return 0;
            };
            self.spent.chips += 1;
            self.spent.screens_late += (chosen.looked > self.level.looks) as u32;
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

    /// Its buttons while the battle is dimmed: B held (as ever), and A on
    /// the tick a decision asked as the dimming began finds best (taken a
    /// reaction time on, the battle waiting for it if it isn't ready). Where
    /// the dimming goes otherwise than it supposed (the other player cut
    /// in), and after its press, it asks again.
    fn dimmed(&mut self, b: &Battle, side: u8, theirs: u16) -> u16 {
        if self.dim.is_none() {
            self.asked += 1;
            self.dim = Some(Dim { asked: self.asked, at: self.ticks, take: self.ticks + self.level.reaction, answer: None });
            let q = self.question(b, side, theirs, Asked::Dimmed);
            self.thinker.ask(q);
        }
        let dim = self.dim.as_mut().expect("asked");
        if dim.answer.is_none() && self.ticks >= dim.take {
            let waiting = Instant::now();
            let (answer, waited) = self.thinker.answer(dim.asked);
            if waited {
                self.spent.late += 1;
                self.spent.waited += waiting.elapsed();
            }
            self.spent.dimmings += 1;
            self.spent.ticks += answer.ticks;
            self.spent.dim_time += answer.took;
            self.spent.dim_longest = self.spent.dim_longest.max(answer.took);
            dim.answer = Some(answer.dimmed.unwrap_or_default());
        }
        let Some(answer) = &dim.answer else {
            return keys::B;
        };
        let k = (self.ticks - dim.at) as usize;
        if answer.sigs.get(k).is_some_and(|s| *s != dimmed::situation(b)) {
            // (Not as it supposed: asked again.)
            self.dim = None;
            return self.dimmed(b, side, theirs);
        }
        if answer.press == Some(k as u32) {
            // (Pressed: asked again from the next tick, for a press more.)
            self.dim = None;
            return keys::A | keys::B;
        }
        keys::B
    }

    /// The decision asked, once its tick has come: its answer (waited for,
    /// if it isn't ready), and the plan picked from it. Where the chips in
    /// hand (`hand`, now) aren't those it was asked with, it is dropped and
    /// another asked at once: its plans with a chip were for the chip that
    /// was next (a chip pushed out a reaction time ago, used meanwhile by
    /// the plan it followed, would push out the chip behind it: a second
    /// Invisibl while the first's invisibility is up).
    fn take_decision(&mut self, hand: u8) {
        let Some((id, _, asked_with)) = self.pending.filter(|(_, at, _)| *at <= self.ticks) else {
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
        if hand != asked_with {
            self.since = self.level.every;
            return;
        }
        if let Some(wait) = answer.stale_wait {
            (self.stale_after, self.stale_timed) = (self.held + wait, true);
        }
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

/// The chips of its folder dealt on side `side`'s custom screen in `b`, by
/// slot (none where no screen shows).
fn dealt(b: &Battle, side: u8) -> Vec<(u8, FolderChip)> {
    let s = &b.custom.sides[side as usize];
    let Some(screen) = s.screen.as_ref() else {
        return Vec::new();
    };
    screen
        .slots
        .iter()
        .enumerate()
        .filter_map(|(at, slot)| match slot.kind {
            SlotKind::Chip { index, .. } => s.folder.chips.get(index as usize).copied().flatten().map(|chip| (at as u8, chip)),
            _ => None,
        })
        .collect()
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
/// nothing), else in the order the cursor reaches them; but never a chip OK
/// folds into the chip before it (an attack-up) where it wouldn't fold
/// (picked first, or after a chip it doesn't attach to: it would stand alone
/// in the hand, and poof as it is used). Only the folder's chips dealt (no
/// button, no chip offered from outside it, no form).
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
        // (A chip OK folds into the chip before it (an attack-up) is picked only right after one it folds into:
        // else it stands in the hand alone, and poofs as it is used. The engine's own test, `stays_unfolded`, from
        // the hand the picks so far build.)
        let library: &dyn nettai_battle::custom::library::Library = &*b.content;
        let chip_in = |slot: u8| screen.chip_in(slot, &s.folder).map(|c| c.id);
        let mut last = None;
        for &slot in screen.selection() {
            if let Some(c) = chip_in(slot)
                && (library.chip(c).modifier.is_none() || nettai_battle::custom::builder::stays_unfolded(library, last, c))
            {
                last = Some(c);
            }
        }
        let folds = |slot: u8| chip_in(slot).is_none_or(|c| !nettai_battle::custom::builder::stays_unfolded(library, last, c));
        let key = match pick(screen, &b.game_rules().custom_screen, worth, &folds) {
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
/// Only a slot `folds` says may be picked now is (a chip that would stand
/// alone in the hand isn't).
fn pick(screen: &Screen, layout: &CustomScreenLayout, worth: Option<&dyn Fn(u8) -> Option<i64>>, folds: &dyn Fn(u8) -> bool) -> Option<u16> {
    let wanted = |i: u8| {
        let slot = &screen.slots[i as usize];
        matches!(slot.kind, SlotKind::Chip { .. }) && slot.state == SlotState::Selectable && folds(i)
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

