//! While the battle is dimmed: what a press of A does then, found in
//! copies.
//!
//! A dimming stands everything still but what its chip brings, so what a
//! press does then is decided by the battle alone: a chip that waits for
//! its user's A (a sight circling the other side's area, a navi turning
//! his aim) and a cut-in (A during the other side's chip's name, with a
//! dimming chip next in hand) play out the same in a copy as they will.
//! So the press is tried on each tick it can be made on, each copy played
//! on, and the tick that does most is taken.
//!
//! Which ticks those are is the engine's to say: a press is an aim where
//! the chip reads its user's A on that tick (the battle counts content's
//! reads of a side's keys while dimmed: `Battle::dimmed_key_reads`), tried
//! over a window from the first such tick (each later one repeats what the
//! chip has already shown); and a cut-in where the other side's chip's
//! name shows with a dimming chip next in hand (the dimming records:
//! `Battle::dimming`), tried on the first such tick (a later one does the
//! same).

use super::*;
use nettai_battle::content::ChipFlags;
use nettai_battle::dimming::DimmingState;
use std::collections::HashMap;

/// Aims found in copies, by the battle they start from: a copy's state
/// repeats where plans and guesses lead to the same dimming.
pub(super) type Memo = HashMap<u64, Option<u32>>;

/// The aims a memo keeps at most before it starts over.
const MEMO_MOST: usize = 4096;

/// How a side presses while a copy of the battle is dimmed: what it holds,
/// and, as an effect of its own runs (its dimming record), A where [`aim`]
/// finds best, looked for as the effect first reads its keys.
#[derive(Clone, Copy, Debug)]
pub(super) struct Dimmer {
    aims: bool,
    /// The effects of its own about to run that it leaves alone (the one
    /// whose press is being tried).
    skip: u8,
    /// Its record's state the tick before.
    last: DimmingState,
    /// The effect of its own that runs is aimed already (or left alone).
    settled: bool,
    /// Its ticks, and the one it presses A on.
    t: u32,
    press: Option<u32>,
}

impl Dimmer {
    /// One that aims every effect of its own that runs.
    pub(super) const AIMS: Dimmer = Dimmer { aims: true, skip: 0, last: DimmingState::Idle, settled: true, t: 0, press: None };
    /// One that never presses.
    const STILL: Dimmer = Dimmer { aims: false, ..Dimmer::AIMS };

    /// Its record as the tick begins: an effect of its own that starts to
    /// run is one to aim.
    fn note(&mut self, b: &Battle, side: u8) {
        let state = b.dimming[side as usize].state;
        if state == DimmingState::Running && self.last != DimmingState::Running {
            self.press = None;
            self.settled = !self.aims || self.skip > 0;
            self.skip = self.skip.saturating_sub(1);
        }
        self.last = state;
    }

    /// Whether it looks for where to press as soon as its keys are read.
    fn wants(&self) -> bool {
        !self.settled && self.last == DimmingState::Running
    }

    fn buttons(&mut self, held: u16) -> u16 {
        let t = self.t;
        self.t = self.t.saturating_add(1);
        held | if self.press == Some(t) { keys::A } else { 0 }
    }
}

/// Both sides' dimmers in a copy, the battle as it was before the tick
/// (an aim is looked for from there), and the aims looked for (in a memo
/// or not: the same count on any thread, [`AIM_WORK`]).
#[derive(Clone)]
pub(super) struct Dimmed {
    dims: [Dimmer; 2],
    saved: Option<Battle>,
    pub(super) aims: u32,
}

/// What an aim counts for in a custom screen's work (`Level::budget`), in
/// ticks of the copies: about what one costs looked for ([`aim`]'s copies,
/// [`Look::TRIAL`]: EXE6's EraseMan and CircGun some 250 to 290), whether
/// this thread's memo holds it or not, so the work is counted alike on any
/// machine and bounds the work done.
pub(super) const AIM_WORK: u64 = 300;

impl Dimmed {
    pub(super) fn new(dims: [Dimmer; 2]) -> Dimmed {
        Dimmed { dims, saved: None, aims: 0 }
    }

    /// Both aim.
    pub(super) fn aiming() -> Dimmed {
        Dimmed::new([Dimmer::AIMS; 2])
    }

    /// A tick of `b` while dimmed (or just after), `held`: what each side
    /// holds, `extra` pressed by side `side` besides. Where a side whose
    /// effect wants aiming has its keys read in it, the tick is taken back,
    /// the aim looked for from before it, and the tick played again.
    pub(super) fn tick(&mut self, b: &mut Battle, side: u8, held: [u16; 2], extra: u16, memo: &mut Memo, ticks: &mut u64) {
        let them = side ^ 1;
        for s in [0, 1] {
            self.dims[s as usize].note(b, s);
        }
        let lazy = self.dims.iter().any(Dimmer::wants);
        let before = self.dims;
        let reads = [b.dimmed_key_reads(0, keys::A), b.dimmed_key_reads(1, keys::A)];
        if lazy {
            match &mut self.saved {
                Some(saved) => saved.clone_from(b),
                None => self.saved = Some(b.clone()),
            }
        }
        loop {
            let mine = self.dims[side as usize].buttons(held[side as usize]) | extra;
            let theirs = self.dims[them as usize].buttons(held[them as usize]);
            step(b, side, mine, theirs);
            *ticks += 1;
            if !lazy {
                return;
            }
            let saved = self.saved.as_ref().expect("saved");
            let mut again = false;
            for s in [0u8, 1] {
                let d = &mut self.dims[s as usize];
                if d.wants() && b.dimmed_key_reads(s, keys::A) != reads[s as usize] {
                    // (Its keys are read: where to press is looked for from before this tick, played again.)
                    d.settled = true;
                    d.press = aim(saved, s, held, memo, ticks).map(|p| before[s as usize].t + p);
                    self.aims += 1;
                    #[cfg(test)]
                    AIM_CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    again = true;
                }
            }
            if !again {
                return;
            }
            b.clone_from(saved);
            for s in 0..2 {
                self.dims[s].t = before[s].t;
            }
        }
    }
}

/// Whose dimming runs and the chips in hand: what a cut-in changes.
pub(super) fn situation(b: &Battle) -> u64 {
    nettai_battle::digest::stable_hash(&(&b.dimming, b.hands[0].remaining(), b.hands[1].remaining()))
}

/// Whether a press of A by side `side` may cut in on the other side's
/// dimming now, as the dimming records and its hand say (the other side's
/// chip's name shows, its chip can be cut in on, its own dimming is idle
/// or waits, and its next chip is a dimming chip): the engine's own test,
/// which a copy then plays.
pub(super) fn cut_window(b: &Battle, side: u8) -> bool {
    let (own, other) = (b.dimming[side as usize], b.dimming[(side ^ 1) as usize]);
    other.state == DimmingState::ShowingName
        && !other.no_cut_in
        && matches!(own.state, DimmingState::Idle | DimmingState::Waiting)
        && b.hands[side as usize].next_chip().is_some_and(|c| b.content.chip(c).flags.has(ChipFlags::DIMMING))
}

/// How a copy is played to its end, and how hard the presses are looked
/// for.
#[derive(Clone, Copy, Debug)]
pub(super) struct Look {
    /// The first tick a press may be made on, and the ticks between the
    /// aims tried.
    from: u32,
    stride: u32,
    /// The ticks from the first an aim may be made on it is tried over.
    window: u32,
    /// Cut-ins are tried.
    cuts: bool,
    /// A copy is played till the effect of the side's own that runs ends
    /// (none: till the dimming is over and `settle` ticks of the fight
    /// after), `most` ticks at most.
    effect: bool,
    settle: u32,
    most: u32,
    /// A press tried is played till the navis' HP has stayed as it is
    /// this many ticks since it last changed, where it did (the damage is
    /// settled), at most.
    quiet: Option<u32>,
    /// A press tried is played this many ticks at most (what lands later
    /// isn't counted in its favor).
    reach: Option<u32>,
    /// What pressing nothing makes is worked out (else the best press is
    /// taken whatever it makes).
    never: bool,
}

impl Look {
    /// In copies of the fight a plan is tried in (and its aims' own
    /// copies): every twelfth tick of a second from the first its keys are
    /// read on, each played till the effect ends, its damage is settled, or
    /// for three quarters of a second.
    const TRIAL: Look = Look { from: 0, stride: 12, window: 60, cuts: false, effect: true, settle: 0, most: 420, quiet: Some(20), reach: Some(45), never: false };

    fn trial() -> Look {
        #[cfg(test)]
        {
            static LOOK: std::sync::OnceLock<Look> = std::sync::OnceLock::new();
            *LOOK.get_or_init(|| match std::env::var("FIGHTER_AIM") {
                Ok(v) => {
                    let n: Vec<u32> = v.split(',').map(|x| x.parse().unwrap()).collect();
                    Look { stride: n[0], window: n[1], reach: n.get(2).copied(), ..Look::TRIAL }
                }
                Err(_) => Look::TRIAL,
            })
        }
        #[cfg(not(test))]
        Look::TRIAL
    }

    /// Live: from a reaction time on, every tick of two seconds, each
    /// played through the dimming (its cut-ins, and the other side's
    /// aims) and a third of a second of the fight after.
    pub(super) fn live(reaction: u32) -> Look {
        Look { from: reaction, stride: 1, window: 120, cuts: true, effect: false, settle: 20, most: 1200, quiet: None, reach: None, never: true }
    }
}

/// How far a copy has got: the fight's ticks since its dimming ended, and
/// whether an effect of the side's own has run in it.
#[derive(Clone, Copy, Default)]
struct Progress {
    after: u32,
    ran: bool,
}

impl Progress {
    fn of(b: &Battle, side: u8) -> Progress {
        Progress { after: 0, ran: b.dimming[side as usize].state == DimmingState::Running }
    }

    /// After a tick of `b`.
    fn note(&mut self, b: &Battle, side: u8) {
        self.after = if b.is_dimmed() { 0 } else { self.after + 1 };
        self.ran |= b.dimming[side as usize].state == DimmingState::Running;
    }

    /// The effect of the side's own that ran has ended (or the dimming).
    fn effect_ended(&self, b: &Battle, side: u8) -> bool {
        !b.is_dimmed() || (self.ran && b.dimming[side as usize].state != DimmingState::Running)
    }

    /// Whether a copy played by `look` is done.
    fn done(&self, b: &Battle, side: u8, look: &Look) -> bool {
        if !fight_goes_on(b) {
            return true;
        }
        if look.effect {
            return self.effect_ended(b, side);
        }
        !b.is_dimmed() && self.after >= look.settle
    }
}

/// A press to try: on tick `t` of the copy, from the copy then (`battle`),
/// its sides' dimmers then, and whether it cuts in.
pub(super) struct Start {
    pub(super) t: u32,
    battle: Battle,
    dims: [Dimmer; 2],
    pub(super) cut: bool,
}

/// What a scan of a dimming found ([`scan`]): the presses to try; what
/// pressing nothing makes (where the look says); and the situation tick by
/// tick without a press.
pub(super) struct Scan {
    pub(super) starts: Vec<Start>,
    pub(super) never: i64,
    pub(super) sigs: Vec<u64>,
}

/// The ticks of `b` a press of A by side `side` may be made on, by `look`
/// (the module's docs), the copy pressing nothing played on to find them;
/// and, where `look` says, that copy played to its end. `dims`: how both
/// sides press otherwise.
pub(super) fn scan(b: &Battle, side: u8, held: [u16; 2], dims: [Dimmer; 2], look: &Look, memo: &mut Memo, ticks: &mut u64) -> Scan {
    let mut base = b.clone();
    let mut dimmed = Dimmed::new(dims);
    let (mut starts, mut sigs) = (Vec::new(), Vec::new());
    let (mut first, mut cut) = (None::<u32>, false);
    let mut k = 0u32;
    let mut progress = Progress::of(b, side);
    loop {
        if progress.done(&base, side, look) || k >= look.most {
            break;
        }
        let open = first.is_none_or(|f| k < f + look.window);
        if !open && !look.never {
            break;
        }
        let window = look.cuts && !cut && cut_window(&base, side);
        let start = (open && k >= look.from).then(|| (base.clone(), dimmed.dims));
        sigs.push(situation(&base));
        let reads = base.dimmed_key_reads(side, keys::A);
        dimmed.tick(&mut base, side, held, 0, memo, ticks);
        let read = base.dimmed_key_reads(side, keys::A) != reads;
        if let Some((battle, dims)) = start {
            if window {
                cut = true;
                starts.push(Start { t: k, battle, dims, cut: true });
            } else if read {
                let f = *first.get_or_insert(k);
                if (k - f).is_multiple_of(look.stride) {
                    starts.push(Start { t: k, battle, dims, cut: false });
                }
            }
        }
        k += 1;
        progress.note(&base, side);
    }
    let never = if look.never {
        let (dealt, lost) = outcome(b, &base, side);
        dealt - lost
    } else {
        0
    };
    Scan { starts, never, sigs }
}

/// What a press from `start` makes for side `side`, played by `look` to
/// its end: what it takes away less what it loses since `from`.
#[allow(clippy::too_many_arguments)]
pub(super) fn branch(from: &Battle, start: &Start, side: u8, held: [u16; 2], look: &Look, trial: &mut Battle, memo: &mut Memo, ticks: &mut u64) -> i64 {
    trial.clone_from(&start.battle);
    let mut dimmed = Dimmed::new(start.dims);
    let mut k = start.t;
    let mut progress = Progress::of(trial, side);
    let navis = |b: &Battle| [hp(b, 0), hp(b, 1)];
    let (mut was, mut quiet) = (navis(trial), None::<u32>);
    let mut extra = keys::A;
    let last = look.reach.map_or(look.most + look.settle, |r| (start.t + r).min(look.most + look.settle));
    while !progress.done(trial, side, look) && k < last && quiet.zip(look.quiet).is_none_or(|(q, most)| q < most) {
        dimmed.tick(trial, side, held, std::mem::take(&mut extra), memo, ticks);
        k += 1;
        progress.note(trial, side);
        let now = navis(trial);
        quiet = if now != was { Some(0) } else { quiet.map(|q| q + 1) };
        was = now;
    }
    let (dealt, lost) = outcome(from, trial, side);
    dealt - lost
}

/// The tick (from `b`'s) side `side` presses A on to aim the effect of its
/// own that runs in `b`, its keys read on `b`'s next tick: tried in copies
/// by [`Look::TRIAL`] (nothing else pressed in them), the earliest of
/// those that make most. Kept in `memo` by the battle it starts from.
pub(super) fn aim(b: &Battle, side: u8, held: [u16; 2], memo: &mut Memo, ticks: &mut u64) -> Option<u32> {
    let key = nettai_battle::digest::stable_hash(&(b.digest(), side, held));
    if let Some(&found) = memo.get(&key) {
        return found;
    }
    #[cfg(test)]
    AIMED[(held[side as usize] == 0) as usize].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let look = Look::trial();
    #[cfg(test)]
    let before = *ticks;
    let scan = scan(b, side, held, [Dimmer::STILL; 2], &look, memo, ticks);
    let mut trial = b.clone();
    let mut best: Option<(u32, i64)> = None;
    for start in &scan.starts {
        let worth = branch(b, start, side, held, &look, &mut trial, memo, ticks);
        if best.is_none_or(|(_, w)| worth > w) {
            best = Some((start.t, worth));
        }
    }
    #[cfg(test)]
    AIM_TICKS.fetch_add(*ticks - before, std::sync::atomic::Ordering::Relaxed);
    let found = best.map(|(t, _)| t);
    if memo.len() >= MEMO_MOST {
        memo.clear();
    }
    memo.insert(key, found);
    found
}

/// A live dimming's presses to try, for the search's threads: the starts
/// picked (by index), each played to its end ([`branch`]).
pub(super) struct Branches {
    pub(super) from: Battle,
    pub(super) starts: std::sync::Arc<Vec<Start>>,
    pub(super) pick: Vec<usize>,
    pub(super) side: u8,
    pub(super) held: [u16; 2],
    pub(super) look: Look,
}

impl Branches {
    /// Picks `share`, `share + of` and so on: by pick, what each makes,
    /// and the ticks that took.
    pub(super) fn share(&self, share: usize, of: usize, trial: &mut Battle, memo: &mut Memo) -> (Vec<(usize, i64)>, u64) {
        let mut ticks = 0;
        let worths = (share..self.pick.len())
            .step_by(of)
            .map(|i| (i, branch(&self.from, &self.starts[self.pick[i]], self.side, self.held, &self.look, trial, memo, &mut ticks)))
            .collect();
        (worths, ticks)
    }
}

/// The ticks between the aims a live dimming tries first; then those
/// around the best of them.
pub(super) const COARSE: u32 = 4;

/// The dimmers of a live dimming's copies, from `b`: both sides aim the
/// effects that start to run, but side `side`'s own about to run (its
/// press is what is tried).
pub(super) fn live_dimmers(b: &Battle, side: u8) -> [Dimmer; 2] {
    let mut dims = [Dimmer::AIMS; 2];
    for (s, d) in dims.iter_mut().enumerate() {
        d.last = b.dimming[s].state;
    }
    let own = b.dimming[side as usize].state;
    dims[side as usize].skip = matches!(own, DimmingState::Registered | DimmingState::ShowingName | DimmingState::Waiting) as u8;
    dims
}

/// A live dimming's decision: the tick (from the question's) to press A
/// on, if any, and the situation tick by tick as it supposes it goes to
/// then ([`situation`]: where it doesn't, it asks again).
#[derive(Clone, Debug, Default)]
pub(super) struct DimAnswer {
    pub(super) press: Option<u32>,
    pub(super) sigs: Vec<u64>,
}

/// In tenths of HP, how much more a cut-in must make than none for it to
/// be made, live and in a turn's copies (`TurnPlay::cut_in`): a chip spent
/// for clearly more than its use's own noise (a defense the other player's
/// chip goes through, as JudgeMan's books strike an invisible navi, makes
/// nothing, and a copy a few HP luckier isn't reason enough).
pub(super) const CUT_IN_MARGIN: i64 = 100;

/// The aims searched (not found in a memo), the fighter's side's and the
/// other's (held nothing), for the tests' measurements.
#[cfg(test)]
pub(super) static AIMED: [std::sync::atomic::AtomicU64; 2] = [std::sync::atomic::AtomicU64::new(0), std::sync::atomic::AtomicU64::new(0)];
/// The ticks those aims took.
#[cfg(test)]
pub(super) static AIM_TICKS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// The aims looked for, in a memo or not.
#[cfg(test)]
pub(super) static AIM_CALLS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
