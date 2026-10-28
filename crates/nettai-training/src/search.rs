//! Lookahead for live play (training-ai.md §10.7): at a fight decision, the
//! network's likeliest actions, and the steps beside its first, each tried
//! on a copy of the battle as it stands, the action held for the decision's
//! `k` ticks (then B alone if it held B: a charge isn't let go), the other
//! side pressing nothing, for [`Search::horizon`] undimmed ticks (a dimming
//! plays out whole: a navi chip lands in its own). Each is worth the HP it
//! took from the other navi less the HP it cost, as fractions of their max
//! HP, plus [`Search::prior`] times its log-probability under the network;
//! the most worth is taken, the network's own order breaking ties.
//!
//! What it tries is cut where it can't matter: while dimmed the network's
//! choice is played (inputs mostly wait a dimming out, and each try would
//! play it out whole); and where standing (the first's A and B, no step)
//! costs nothing, only what can deal damage is tried besides the first (A
//! pressed, or B other than the first's), not the steps alone.
//!
//! It reads the battle as it is, not the delayed view the network decides
//! from: what is already on the field and how it moves (attacks in flight,
//! a charge about to be let go), not the other player's next press.
//!
//! In live play ([`Search::ahead`], on a thread of its own: `Worker`) it
//! thinks a decision over for [`Search::lead`] ticks before it takes
//! effect: from the battle as it was then, its own held action pressed
//! through them and the other side's nothing, and with luck of its own (the
//! copy's random numbers drawn anew), so it can't foresee the battle's.

use crate::act;
use crate::env::{fight_runs, step_battle};
use crate::spec;
use crate::stats::navi_hp;
use nettai_battle::{Battle, Rng};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "live")]
mod worker;
#[cfg(feature = "live")]
pub use worker::Worker;

/// Every lookahead's decisions, tries and ticks stepped so far, in this
/// process (for measuring its cost).
pub static DECISIONS: AtomicU64 = AtomicU64::new(0);
pub static TRIES: AtomicU64 = AtomicU64::new(0);
pub static TICKS: AtomicU64 = AtomicU64::new(0);
/// Live play's decisions whose lookahead wasn't done when they took effect
/// (the battle's thread waited for it: [`Worker::wait`]).
pub static LATE: AtomicU64 = AtomicU64::new(0);

/// How a decision looks ahead.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Search {
    /// The network's likeliest actions tried (the steps beside its first
    /// are tried as well).
    pub candidates: usize,
    /// Undimmed ticks each is followed for, and the ticks at most in all.
    pub horizon: u32,
    pub cap: u32,
    /// Worth a nat of the network's log-probability, in HP fractions.
    pub prior: f32,
    /// Live play's: the ticks a decision is thought over before it takes
    /// effect ([`Search::ahead`]; at least 1, at most the decision's `k`).
    pub lead: u32,
}

impl Default for Search {
    fn default() -> Search {
        // (A nat of the network's preference is worth 5 HP of 1000: a hit of
        // 40 outweighs a choice the network makes 99 times in 100.)
        // (Four ticks to think in: a decision's tries take about 5 ms on
        // the desktop, the most about 50.)
        Search { candidates: 6, horizon: 40, cap: 480, prior: 0.005, lead: 4 }
    }
}

/// The actions a decision tries: the network's `n` likeliest by `logits`
/// (in order), then each direction with the first's A and B.
pub fn candidates(logits: &[f32], n: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..logits.len().min(spec::FIGHT_ACTIONS)).filter(|&a| logits[a].is_finite()).collect();
    order.sort_by(|&a, &b| logits[b].total_cmp(&logits[a]).then(a.cmp(&b)));
    let mut out: Vec<usize> = order.iter().copied().take(n).collect();
    if let Some(&first) = order.first() {
        for dir in 0..act::DIRECTIONS {
            let a = dir * 4 + (first & 3);
            if !out.contains(&a) && logits.get(a).is_some_and(|l| l.is_finite()) {
                out.push(a);
            }
        }
    }
    out
}

impl Search {
    /// The action side `side` takes in `b` at a decision held `k` ticks,
    /// the network's fight `logits` before it. None: no action is allowed.
    pub fn pick(&self, b: &Battle, side: u8, k: u32, logits: &[f32]) -> Option<usize> {
        let mut tried = candidates(logits, self.candidates);
        let first = *tried.first()?;
        if b.is_dimmed() {
            return Some(first);
        }
        DECISIONS.fetch_add(1, Ordering::Relaxed);
        let max = tried.iter().map(|&a| logits[a]).fold(f32::NEG_INFINITY, f32::max);
        let total: f32 = (0..logits.len()).filter(|&a| logits[a].is_finite()).map(|a| (logits[a] - max).exp()).sum();
        // (Standing: the first's buttons, no step. Costing nothing, the steps alone can't help.)
        let stand = first & 3;
        let (dealt, taken) = self.traded(b, side, k, stand);
        let mut known = vec![(stand, dealt - taken)];
        if taken == 0.0 {
            tried.retain(|&a| a == first || a & 2 != 0 || a & 1 != first & 1);
        }
        let mut best: Option<(usize, f32)> = None;
        for &a in &tried {
            let outcome = match known.iter().find(|&&(k, _)| k == a) {
                Some(&(_, o)) => o,
                None => {
                    let o = self.outcome(b, side, k, a);
                    known.push((a, o));
                    o
                }
            };
            let worth = outcome + self.prior * (logits[a] - max - total.ln());
            if best.is_none_or(|(_, w)| worth > w) {
                best = Some((a, worth));
            }
        }
        best.map(|(a, _)| a)
    }

    /// Live play's decision: the action side `side` takes `lead` ticks
    /// after `b`, the battle then foreseen from `b` with `held` pressed on
    /// its side through them and nothing on the other's, `luck` the copy's
    /// random numbers; the network's fight `logits` before it, its action
    /// held `k` ticks. The network's first where the fight doesn't run
    /// then. None: no action is allowed.
    pub fn ahead(&self, b: &Battle, side: u8, k: u32, held: u16, logits: &[f32], luck: u64) -> Option<usize> {
        let mut c = b.clone();
        c.rng = Rng::new(luck as u32);
        for (i, console) in c.consoles.iter_mut().enumerate() {
            console.rng = Rng::new((luck >> 32) as u32 ^ i as u32);
        }
        for _ in 0..self.lead {
            let mut press = [0u16; 2];
            press[side as usize] = held;
            if !step_battle(&mut c, press) || c.round_end().is_some() || !fight_runs(&c) {
                return candidates(logits, 1).first().copied();
            }
        }
        self.pick(&c, side, k, logits)
    }

    /// A teacher's label for training (training-ai.md §8, the lookahead's
    /// lesson): side `side`'s best fight action in `b` among each direction
    /// with and without A, B as it holds it now (`holding_b`), each tried as
    /// [`Search::outcome`] tries it; none while dimmed, or where the best is
    /// worth no more than `margin` above the worst (nothing to dodge or to
    /// hit: the choice is the network's own). Ties go to the first in the
    /// order: no direction before the steps, A let go before pressed. Most
    /// decisions have nothing in either, so standing and pressing A are
    /// tried first: where standing costs nothing and neither deals anything,
    /// the rest aren't tried.
    pub fn teach(&self, b: &Battle, side: u8, k: u32, holding_b: bool, margin: f32) -> Option<usize> {
        if b.is_dimmed() {
            return None;
        }
        let stand = holding_b as usize;
        let (stood, pressed) = (self.traded(b, side, k, stand), self.traded(b, side, k, stand + 2));
        if stood == (0.0, 0.0) && pressed.0 == 0.0 {
            return None;
        }
        let known = [(stand, stood.0 - stood.1), (stand + 2, pressed.0 - pressed.1)];
        let (mut best, mut worst): (Option<(usize, f32)>, f32) = (None, f32::INFINITY);
        for dir in 0..act::DIRECTIONS {
            for a_held in [0, 2] {
                let a = dir * 4 + a_held + holding_b as usize;
                let o = known.iter().find(|&&(k, _)| k == a).map_or_else(|| self.outcome(b, side, k, a), |&(_, o)| o);
                worst = worst.min(o);
                if best.is_none_or(|(_, w)| o > w) {
                    best = Some((a, o));
                }
            }
        }
        best.filter(|&(_, w)| w - worst > margin).map(|(a, _)| a)
    }

    /// What action `a` held for `k` ticks comes to for side `side` of `b`:
    /// the other navi's HP lost less its own, as fractions of their max HP.
    pub fn outcome(&self, b: &Battle, side: u8, k: u32, a: usize) -> f32 {
        let (dealt, taken) = self.traded(b, side, k, a);
        dealt - taken
    }

    /// The same, the two apart: the other navi's HP lost, and its own.
    fn traded(&self, b: &Battle, side: u8, k: u32, a: usize) -> (f32, f32) {
        let mut c = b.clone();
        let (mine0, theirs0) = (navi_hp(&c, side, Some(0)).unwrap_or(0), navi_hp(&c, side ^ 1, Some(0)).unwrap_or(0));
        let max = |c: &Battle, s: u8| c.player(s).filter(|&r| c.objects.is_allocated(r)).map_or(1, |r| c.objects.get(r).max_hp.max(1)) as f32;
        let (mine_max, theirs_max) = (max(&c, side), max(&c, side ^ 1));
        let held = act::fight_buttons(a as i32);
        let after = held & nettai_battle::input::keys::B;
        TRIES.fetch_add(1, Ordering::Relaxed);
        let (mut undimmed, mut t) = (0, 0);
        while undimmed < self.horizon && t < self.cap {
            let mine = if t < k { held } else { after };
            let mut press = [0u16; 2];
            press[side as usize] = mine;
            if !step_battle(&mut c, press) || c.round_end().is_some() {
                break;
            }
            t += 1;
            undimmed += !c.is_dimmed() as u32;
            if navi_hp(&c, side, Some(0)) == Some(0) || navi_hp(&c, side ^ 1, Some(0)) == Some(0) {
                break;
            }
        }
        TICKS.fetch_add(t as u64, Ordering::Relaxed);
        let (mine, theirs) = (navi_hp(&c, side, Some(0)).unwrap_or(0), navi_hp(&c, side ^ 1, Some(0)).unwrap_or(0));
        (theirs0.saturating_sub(theirs) as f32 / theirs_max, mine0.saturating_sub(mine) as f32 / mine_max)
    }
}

#[cfg(test)]
mod tests;
