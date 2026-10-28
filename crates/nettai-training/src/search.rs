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
//! It reads the battle as it is, not the delayed view the network decides
//! from: what is already on the field and how it moves (attacks in flight,
//! a charge about to be let go), not the other player's next press.

use crate::act;
use crate::env::step_battle;
use crate::spec;
use crate::stats::navi_hp;
use nettai_battle::Battle;

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
}

impl Default for Search {
    fn default() -> Search {
        // (A nat of the network's preference is worth 5 HP of 1000: a hit of
        // 40 outweighs a choice the network makes 99 times in 100.)
        Search { candidates: 6, horizon: 40, cap: 480, prior: 0.005 }
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
        let tried = candidates(logits, self.candidates);
        let max = tried.iter().map(|&a| logits[a]).fold(f32::NEG_INFINITY, f32::max);
        let total: f32 = (0..logits.len()).filter(|&a| logits[a].is_finite()).map(|a| (logits[a] - max).exp()).sum();
        let mut best: Option<(usize, f32)> = None;
        for &a in &tried {
            let logp = logits[a] - max - total.ln();
            let worth = self.outcome(b, side, k, a) + self.prior * logp;
            if best.is_none_or(|(_, w)| worth > w) {
                best = Some((a, worth));
            }
        }
        best.map(|(a, _)| a)
    }

    /// What action `a` held for `k` ticks comes to for side `side` of `b`:
    /// the other navi's HP lost less its own, as fractions of their max HP.
    pub fn outcome(&self, b: &Battle, side: u8, k: u32, a: usize) -> f32 {
        let mut c = b.clone();
        let (mine0, theirs0) = (navi_hp(&c, side, Some(0)).unwrap_or(0), navi_hp(&c, side ^ 1, Some(0)).unwrap_or(0));
        let max = |c: &Battle, s: u8| c.player(s).filter(|&r| c.objects.is_allocated(r)).map_or(1, |r| c.objects.get(r).max_hp.max(1)) as f32;
        let (mine_max, theirs_max) = (max(&c, side), max(&c, side ^ 1));
        let held = act::fight_buttons(a as i32);
        let after = held & nettai_battle::input::keys::B;
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
        let (mine, theirs) = (navi_hp(&c, side, Some(0)).unwrap_or(0), navi_hp(&c, side ^ 1, Some(0)).unwrap_or(0));
        theirs0.saturating_sub(theirs) as f32 / theirs_max - mine0.saturating_sub(mine) as f32 / mine_max
    }
}

#[cfg(test)]
mod tests;
