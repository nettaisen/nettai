//! What rollback costs a world, measured: one step, saving a state (time
//! and bytes), restoring one, a rollback of `k` ticks as getgud does it,
//! and whole frames of a session whose remote input arrives over a
//! simulated network.
//!
//! The measuring is generic, so that another world on getgud (Tango's
//! emulated pair, in the verification workspace's tools/rollback-bench) is
//! measured by the same loops as the battle engine's
//! (`examples/rollback_bench.rs`):
//!
//! - a [`Subject`] is a world stepped by hand on recorded inputs, which a
//!   [`Point`] measures at one tick of a match;
//! - a [`Peer`] is a session on recorded inputs, which a [`Drive`] takes
//!   through the wall-clock frames of a match, the other player's inputs
//!   arriving as [`arrivals`] says a simulated network delivers them.
//!
//! [`Recorded`] and [`RecordedSession`] are the battle engine's, and
//! [`costs_table`] and [`frames_table`] write what was measured, for one
//! world or several side by side.
//!
//! Times are wall-clock (`Instant`, which on Apple hardware ticks every
//! 41.67 ns), one thread, the caller's to keep warm and to interleave with
//! whatever it compares against. Bytes are the heap's ([`Counting`], which
//! the measuring program installs as its allocator).

use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::time::Instant;

use getgud::{Session, World};
use nettai_battle::Battle;
use nettai_battle::battle::{fight, mode, top};

use crate::link::InputLink;
use crate::network::{Network, NetworkConfig};
use crate::protocol::HORIZON;
use crate::world::{BattleState, BattleWorld, Game, Observer};

/// A video frame of the original console, in nanoseconds: 280,896 cycles
/// of its 16,777,216 Hz clock (59.7275 frames a second), the budget a
/// frame's simulation has.
pub const FRAME_NANOS: u64 = 16_742_706;

// ---- The heap ---------------------------------------------------------------

static LIVE: AtomicU64 = AtomicU64::new(0);
static ALLOCATED: AtomicU64 = AtomicU64::new(0);
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

/// The system allocator, counting: a measuring program installs it
/// (`#[global_allocator] static HEAP: Counting = Counting;`) so that
/// [`heap`] can say what a saved state holds. It costs an allocation three
/// uncontended atomic additions.
pub struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size() as u64, Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        LIVE.fetch_sub(layout.size() as u64, Relaxed);
        count(new_size);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

fn count(size: usize) {
    LIVE.fetch_add(size as u64, Relaxed);
    ALLOCATED.fetch_add(size as u64, Relaxed);
    ALLOCATIONS.fetch_add(1, Relaxed);
}

/// The heap as [`Counting`] has counted it (all zero in a program that
/// doesn't install it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Heap {
    /// Bytes allocated and not freed.
    pub live: u64,
    /// Bytes ever allocated.
    pub allocated: u64,
    /// Allocations ever made.
    pub allocations: u64,
}

pub fn heap() -> Heap {
    Heap { live: LIVE.load(Relaxed), allocated: ALLOCATED.load(Relaxed), allocations: ALLOCATIONS.load(Relaxed) }
}

/// The machine's load averages (1, 5 and 15 minutes), to print beside
/// what was measured under them.
pub fn load_average() -> Option<[f64; 3]> {
    let text = match std::fs::read_to_string("/proc/loadavg") {
        Ok(text) => text,
        Err(_) => String::from_utf8(std::process::Command::new("sysctl").args(["-n", "vm.loadavg"]).output().ok()?.stdout).ok()?,
    };
    let mut numbers = text.split_whitespace().filter_map(|w| w.parse::<f64>().ok());
    Some([numbers.next()?, numbers.next()?, numbers.next()?])
}

// ---- Samples ----------------------------------------------------------------

/// Measurements of one thing, in nanoseconds (or bytes, or a count).
#[derive(Clone, Debug, Default)]
pub struct Samples(pub Vec<u64>);

/// A set of samples in a few numbers. `p10`, `p90` and `p99` are the
/// samples that many percent of the way up the sorted set.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Summary {
    pub count: usize,
    pub min: u64,
    pub p10: u64,
    pub median: u64,
    pub p90: u64,
    pub p99: u64,
    pub max: u64,
    pub mean: f64,
}

impl Samples {
    pub fn push(&mut self, value: u64) {
        self.0.push(value);
    }

    pub fn extend(&mut self, other: &Samples) {
        self.0.extend_from_slice(&other.0);
    }

    pub fn summary(&self) -> Summary {
        if self.0.is_empty() {
            return Summary::default();
        }
        let mut sorted = self.0.clone();
        sorted.sort_unstable();
        let at = |percent: usize| sorted[((sorted.len() - 1) * percent + 50) / 100];
        Summary {
            count: sorted.len(),
            min: sorted[0],
            p10: at(10),
            median: at(50),
            p90: at(90),
            p99: at(99),
            max: sorted[sorted.len() - 1],
            mean: sorted.iter().sum::<u64>() as f64 / sorted.len() as f64,
        }
    }
}

// ---- A world stepped by hand ------------------------------------------------

/// A world stepped by hand on a recorded match: the calls a getgud session
/// makes of its world, with the match's own inputs. Ticks are numbered as
/// the subject numbers them: "parked at `t`" is the state after ticks
/// `0..t`.
pub trait Subject {
    type State;

    /// Simulate tick `tick`, the one the subject is parked at.
    fn step(&mut self, tick: usize);

    /// Save the state the subject is parked at.
    fn save(&mut self) -> Self::State;

    /// Go back to a saved state.
    fn load(&mut self, state: &Self::State);
}

/// What a subject's calls cost, over the points measured.
#[derive(Clone, Debug, Default)]
pub struct Costs {
    /// One step, nanoseconds.
    pub step: Samples,
    /// One save, nanoseconds.
    pub save: Samples,
    /// One restore, nanoseconds.
    pub restore: Samples,
    /// Freeing the states a rollback throws away, nanoseconds per rollback.
    pub discard: Samples,
    /// A rollback of the key's depth, nanoseconds (see [`Point::run`]).
    pub rollback: BTreeMap<usize, Samples>,
    /// What a saved state holds on the heap, bytes, and the allocations
    /// making it took (one sample a point; zero without [`Counting`]).
    pub state_bytes: Samples,
    pub state_allocations: Samples,
}

impl Costs {
    pub fn extend(&mut self, other: &Costs) {
        self.step.extend(&other.step);
        self.save.extend(&other.save);
        self.restore.extend(&other.restore);
        self.discard.extend(&other.discard);
        for (depth, samples) in &other.rollback {
            self.rollback.entry(*depth).or_default().extend(samples);
        }
        self.state_bytes.extend(&other.state_bytes);
        self.state_allocations.extend(&other.state_allocations);
    }
}

fn nanos(from: Instant, to: Instant) -> u64 {
    to.duration_since(from).as_nanos() as u64
}

/// One tick of a match, where a subject's rollbacks are measured: the
/// state there, and the states after each of the `depth` ticks that
/// follow, as a session that had speculated them would hold them.
pub struct Point<S: Subject> {
    at: usize,
    depth: usize,
    /// Parked at `at`.
    base: S::State,
    /// `ring[j]` is parked at `at + j + 1`.
    ring: Vec<S::State>,
    /// By depth: the states the last rollback of that depth saved, which
    /// the next throws away.
    tails: BTreeMap<usize, Vec<S::State>>,
}

impl<S: Subject> Point<S> {
    /// `subject` is parked at `at`. Saves it there (which is where a
    /// state's bytes are counted), then steps the `depth` ticks from `at`,
    /// saving after each; those steps and saves are samples too. The
    /// subject is left parked at `at + depth`.
    pub fn open(subject: &mut S, at: usize, depth: usize, costs: &mut Costs) -> Point<S> {
        let before = heap();
        let t = Instant::now();
        let base = subject.save();
        costs.save.push(nanos(t, Instant::now()));
        let after = heap();
        costs.state_bytes.push(after.live.saturating_sub(before.live) + size_of::<S::State>() as u64);
        costs.state_allocations.push(after.allocations - before.allocations);
        let mut ring = Vec::with_capacity(depth);
        for j in 0..depth {
            let t0 = Instant::now();
            subject.step(at + j);
            let t1 = Instant::now();
            ring.push(subject.save());
            let t2 = Instant::now();
            costs.step.push(nanos(t0, t1));
            costs.save.push(nanos(t1, t2));
        }
        Point { at, depth, base, ring, tails: BTreeMap::new() }
    }

    /// One rollback of each of `depths` (none deeper than the point's), as
    /// a getgud session makes it when the other player's input for the
    /// oldest of `k` speculated ticks turns out wrong, in the frame that
    /// also adds a tick (`Session::advance`): it throws away the `k`
    /// speculated states and the settled one it replaces, restores the
    /// settled state, steps the corrected tick and saves the new settled
    /// state, then steps and saves the `k` ticks up to the new frame. That
    /// is one restore, `k + 1` steps and `k + 1` saves, timed together into
    /// `costs.rollback[k]` and singly into `step`, `save` and `restore`.
    ///
    /// Every depth re-simulates the ticks that end at `at + depth`. The
    /// first rollback of a depth at a point runs once unmeasured before
    /// it, which leaves the states the measured one throws away; and every
    /// call starts with an unmeasured rollback of its first depth, so that
    /// a subject measured in turn with another starts warm.
    pub fn run(&mut self, subject: &mut S, depths: &[usize], costs: &mut Costs) {
        let mut unmeasured = Costs::default();
        if let Some(&k) = depths.first() {
            self.rollback(subject, k, &mut unmeasured);
        }
        for &k in depths {
            if !self.tails.contains_key(&k) {
                self.rollback(subject, k, &mut unmeasured);
            }
            self.rollback(subject, k, costs);
        }
    }

    fn rollback(&mut self, subject: &mut S, k: usize, costs: &mut Costs) {
        assert!(k <= self.depth, "a rollback of {k} at a point {} deep", self.depth);
        let from = self.at + self.depth - k;
        let settled = if k == self.depth { &self.base } else { &self.ring[self.depth - k - 1] };
        let thrown = self.tails.remove(&k);
        let mut tail = Vec::with_capacity(k + 1);
        let start = Instant::now();
        drop(thrown);
        let mut t = Instant::now();
        costs.discard.push(nanos(start, t));
        subject.load(settled);
        let mut now = Instant::now();
        costs.restore.push(nanos(t, now));
        for j in 0..=k {
            t = now;
            subject.step(from + j);
            now = Instant::now();
            costs.step.push(nanos(t, now));
            t = now;
            tail.push(subject.save());
            now = Instant::now();
            costs.save.push(nanos(t, now));
        }
        costs.rollback.entry(k).or_default().push(nanos(start, now));
        self.tails.insert(k, tail);
    }

    /// Put the subject back at the point's tick.
    pub fn close(self, subject: &mut S) {
        subject.load(&self.base);
    }
}

// ---- A session on a simulated network ---------------------------------------

/// By wall-clock frame, how many of the other player's inputs have arrived
/// by the time a peer advances in it, over a simulated network: both
/// players push an input and send a datagram every frame from frame 0 (no
/// stalls: their clocks agree), over [`InputLink`]s (rennet's streams, which
/// recover a lost datagram's inputs from the next one) and a [`Network`]
/// each way like `network`, seeded. With no latency the count at frame `w`
/// is `w + 1`.
pub fn arrivals(network: NetworkConfig, seed: u64, frames: usize) -> Vec<usize> {
    let mut links = [InputLink::<u16>::new(HORIZON), InputLink::<u16>::new(HORIZON)];
    // networks[p]: from player p to the other.
    let mut networks = [Network::new(network, seed), Network::new(network, seed ^ 0x9E37_79B9_7F4A_7C15)];
    let mut delivered = Vec::new();
    let mut arrived = Vec::with_capacity(frames);
    for now in 0..frames as u64 {
        for p in 0..2 {
            links[p].push(&0, 0);
            let datagram = links[p].datagram(now);
            networks[p].send(now, datagram);
        }
        for p in 0..2 {
            for datagram in networks[1 - p].receive(now) {
                links[p].receive(&datagram, now, &mut delivered).expect("a gap past the horizon: the network is worse than a match survives");
            }
            delivered.clear();
        }
        arrived.push(links[0].received_elements() as usize);
    }
    arrived
}

/// One peer's session on a recorded match. Ticks are numbered as the peer
/// numbers them, from its session's start.
pub trait Peer {
    /// The other player's input for `tick` arrived (they arrive in order).
    fn remote(&mut self, tick: usize);

    /// The player's input for `tick` (the next), and the session's advance
    /// with it; returns how many speculated ticks it threw away (getgud's
    /// `last_misprediction_depth`).
    fn advance(&mut self, tick: usize) -> u32;

    /// Local inputs no remote input has confirmed (getgud's
    /// `local_queue_length`).
    fn unconfirmed(&self) -> usize;

    /// Ticks the next advance could confirm from the remote input queued
    /// (getgud's `matchable`).
    fn matchable(&self) -> usize;
}

/// One frame a peer advanced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Advanced {
    /// The local input's tick.
    pub tick: u32,
    /// The frame's time: taking the remote inputs that arrived, and the
    /// advance.
    pub nanos: u64,
    /// Speculated ticks the advance threw away and simulated again.
    pub rollback: u32,
    /// The speculated states the session held afterwards: the ticks the
    /// presented frame ran past the confirmed input (the unconfirmed local
    /// inputs, less the present delay and the newest, which the next
    /// advance simulates).
    pub speculation: u32,
}

/// Takes a [`Peer`] through a match's wall-clock frames.
#[derive(Clone, Debug)]
pub struct Drive {
    /// The other player's inputs taken so far, and the next local tick.
    taken: usize,
    next: usize,
    /// The match's ticks: the local input ends there.
    ticks: usize,
    /// The session's present delay.
    delay: usize,
    pub frames: Vec<Advanced>,
    /// Wall frames the stall guard held the peer.
    pub parked: u64,
}

impl Drive {
    /// A drive through a match of `ticks` ticks, of a peer whose session
    /// presents `delay` ticks behind its newest input.
    pub fn new(ticks: usize, delay: usize) -> Drive {
        Drive { taken: 0, next: 0, ticks, delay, frames: Vec::with_capacity(ticks), parked: 0 }
    }

    /// The local ticks advanced so far.
    pub fn advanced(&self) -> usize {
        self.next
    }

    /// Every local input is in.
    pub fn done(&self) -> bool {
        self.next >= self.ticks
    }

    /// One wall-clock frame: the peer takes the other player's inputs
    /// before tick `arrived`, then advances with its next input, unless the
    /// stall guard holds it (`max_lead` local inputs unconfirmed and
    /// nothing queued to confirm them, as `crate::Peer::wait` parks a peer).
    /// False once the local input has ended.
    pub fn frame<P: Peer>(&mut self, peer: &mut P, arrived: usize, max_lead: usize) -> bool {
        if self.done() {
            return false;
        }
        let start = Instant::now();
        while self.taken < arrived.min(self.ticks) {
            peer.remote(self.taken);
            self.taken += 1;
        }
        if peer.unconfirmed() >= max_lead && peer.matchable() == 0 {
            self.parked += 1;
            return true;
        }
        let rollback = peer.advance(self.next);
        let nanos = nanos(start, Instant::now());
        let speculation = peer.unconfirmed().saturating_sub(1 + self.delay) as u32;
        self.frames.push(Advanced { tick: self.next as u32, nanos, rollback, speculation });
        self.next += 1;
        true
    }
}

// ---- What is measured where, and the reports --------------------------------

/// The simulated links a session is measured over: a name, and the network
/// each way. Latency and jitter are in frames (16.74 ms), losses in parts
/// per thousand.
///
/// - `good`: 1 to 2 frames one way (17 to 33 ms), nothing lost;
/// - `typical`: 4 to 6 frames (67 to 100 ms), 1% lost;
/// - `bad`: 8 to 12 frames (134 to 201 ms), 5% lost and 40% of the
///   datagrams after a lost one (bursts), 1% duplicated.
pub const LINKS: [(&str, NetworkConfig); 3] = [
    ("good", NetworkConfig { latency: 1, jitter: 1, loss: 0, burst: 0, duplicate: 0, outage: None }),
    ("typical", NetworkConfig { latency: 4, jitter: 2, loss: 10, burst: 0, duplicate: 0, outage: None }),
    ("bad", NetworkConfig { latency: 8, jitter: 4, loss: 50, burst: 400, duplicate: 10, outage: None }),
];

/// A link in words: its one-way delay in milliseconds and what it loses.
pub fn describe(network: &NetworkConfig) -> String {
    let ms = |frames: u32| (frames as u64 * FRAME_NANOS + 500_000) / 1_000_000;
    let mut text = format!("{} to {} ms one way", ms(network.latency), ms(network.max_delay()));
    if network.loss > 0 {
        text += &format!(", {}% lost", network.loss as f64 / 10.0);
    }
    if network.burst > 0 {
        text += &format!(" ({}% after a loss)", network.burst as f64 / 10.0);
    }
    if network.duplicate > 0 {
        text += &format!(", {}% duplicated", network.duplicate as f64 / 10.0);
    }
    text
}

/// Up to `n` of the places where `eligible` holds, spread evenly over them.
pub fn spread(eligible: &[bool], n: usize) -> Vec<usize> {
    let places: Vec<usize> = (0..eligible.len()).filter(|&i| eligible[i]).collect();
    if places.len() <= n {
        return places;
    }
    (0..n).map(|i| places[(i * 2 + 1) * places.len() / (n * 2)]).collect()
}

/// Nanoseconds as microseconds, to three figures.
pub fn micros(nanos: u64) -> String {
    let us = nanos as f64 / 1e3;
    if us < 9.995 {
        format!("{us:.2}")
    } else if us < 99.95 {
        format!("{us:.1}")
    } else {
        format!("{us:.0}")
    }
}

/// A share of the frame, in percent.
pub fn share(nanos: u64) -> String {
    let percent = nanos as f64 * 100.0 / FRAME_NANOS as f64;
    if percent < 0.995 {
        format!("{percent:.2}%")
    } else if percent < 9.95 {
        format!("{percent:.1}%")
    } else {
        format!("{percent:.0}%")
    }
}

/// How many times `b` is `a`.
fn times(a: u64, b: u64) -> String {
    if b == 0 {
        return "-".to_string();
    }
    let x = a as f64 / b as f64;
    if x < 9.95 { format!("{x:.1}x") } else { format!("{x:.0}x") }
}

/// What the subjects' calls cost, as a Markdown table: a column a subject
/// (by name), and with two of them how many times the second's median the
/// first's is. Times are in microseconds: the median, the samples 10% and
/// 90% of the way up, and the worst; a rollback's also as its median's
/// share of the frame.
pub fn costs_table(subjects: &[(&str, &Costs)]) -> String {
    let two = subjects.len() == 2;
    let mut out = String::from("| microseconds: median (10% to 90%; worst) |");
    for (name, _) in subjects {
        out += &format!(" {name} |");
    }
    if two {
        out += &format!(" {} / {} |", subjects[0].0, subjects[1].0);
    }
    out += "\n|---|";
    out += &"---|".repeat(subjects.len() + two as usize);
    out.push('\n');
    let mut row = |label: String, of: &dyn Fn(&Costs) -> Option<Samples>, cell: &dyn Fn(&Summary) -> String| {
        let summaries: Vec<Option<Summary>> = subjects.iter().map(|(_, c)| of(c).map(|s| s.summary()).filter(|s| s.count > 0)).collect();
        if summaries.iter().all(|s| s.is_none()) {
            return;
        }
        out += &format!("| {label} |");
        for s in &summaries {
            out += &format!(" {} |", s.as_ref().map_or("-".to_string(), cell));
        }
        if two {
            out += &match (&summaries[0], &summaries[1]) {
                (Some(a), Some(b)) => format!(" {} |", times(a.median, b.median)),
                _ => " - |".to_string(),
            };
        }
        out.push('\n');
    };
    let time = |s: &Summary| format!("{} ({} to {}; {})", micros(s.median), micros(s.p10), micros(s.p90), micros(s.max));
    row("one step".to_string(), &|c| Some(c.step.clone()), &time);
    row("saving a state".to_string(), &|c| Some(c.save.clone()), &time);
    row("restoring one".to_string(), &|c| Some(c.restore.clone()), &time);
    let depths: std::collections::BTreeSet<usize> = subjects.iter().flat_map(|(_, c)| c.rollback.keys().copied()).collect();
    for k in depths {
        let cell = |s: &Summary| format!("{} ({} to {}; {}), {} of a frame", micros(s.median), micros(s.p10), micros(s.p90), micros(s.max), share(s.median));
        row(format!("a rollback of {k} (a restore, {} steps and saves)", k + 1), &|c| c.rollback.get(&k).cloned(), &cell);
    }
    row("freeing a rollback's states".to_string(), &|c| Some(c.discard.clone()), &time);
    let bytes = |s: &Summary| if s.min == s.max { format!("{}", s.median) } else { format!("{} ({} to {})", s.median, s.min, s.max) };
    row("a saved state, bytes".to_string(), &|c| Some(c.state_bytes.clone()).filter(|s| s.0.iter().any(|&b| b > 0)), &bytes);
    row("allocations a save makes".to_string(), &|c| Some(c.state_allocations.clone()).filter(|s| s.0.iter().any(|&b| b > 0)), &bytes);
    out
}

/// The frames sessions advanced, as a Markdown table: a row for each
/// label, over its runs' frames together. Times are in microseconds: the
/// median, the 99th percentile and the worst frame (and its share of the
/// frame's 16.74 ms), then the range of the runs' own medians and 99th
/// percentiles; then the frames that rolled back (and their share), those
/// frames' median, how deep they rolled back (mean and deepest), the most
/// speculated states a session held, and the frames that took longer than
/// a frame has.
pub fn frames_table(rows: &[(String, Vec<&[Advanced]>)]) -> String {
    let mut out = String::from(
        "| | frames | median | 99% | worst | worst, of a frame | medians by run | 99% by run | rolled back | their median | mean depth | deepest | most states held | over 16.74 ms |\n\
         |---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n",
    );
    for (label, runs) in rows {
        let all: Vec<Advanced> = runs.iter().flat_map(|r| r.iter().copied()).collect();
        if all.is_empty() {
            continue;
        }
        let of = |frames: &[Advanced]| Samples(frames.iter().map(|f| f.nanos).collect()).summary();
        let s = of(&all);
        let by_run: Vec<Summary> = runs.iter().filter(|r| !r.is_empty()).map(|r| of(r)).collect();
        let range = |pick: &dyn Fn(&Summary) -> u64| {
            let (low, high) = (by_run.iter().map(pick).min().unwrap(), by_run.iter().map(pick).max().unwrap());
            format!("{} to {}", micros(low), micros(high))
        };
        let rollbacks: Vec<u32> = all.iter().map(|f| f.rollback).filter(|&d| d > 0).collect();
        let (mean_depth, their_median) = if rollbacks.is_empty() {
            ("-".to_string(), "-".to_string())
        } else {
            let rolled: Vec<Advanced> = all.iter().filter(|f| f.rollback > 0).copied().collect();
            (format!("{:.1}", rollbacks.iter().sum::<u32>() as f64 / rollbacks.len() as f64), micros(of(&rolled).median))
        };
        out += &format!(
            "| {label} | {} | {} | {} | {} | {} | {} | {} | {} ({:.1}%) | {their_median} | {} | {} | {} | {} |\n",
            all.len(),
            micros(s.median),
            micros(s.p99),
            micros(s.max),
            share(s.max),
            range(&|s| s.median),
            range(&|s| s.p99),
            rollbacks.len(),
            rollbacks.len() as f64 * 100.0 / all.len() as f64,
            mean_depth,
            rollbacks.iter().max().copied().unwrap_or(0),
            all.iter().map(|f| f.speculation).max().unwrap_or(0),
            all.iter().filter(|f| f.nanos > FRAME_NANOS).count(),
        );
    }
    out
}

// ---- The battle engine's ----------------------------------------------------

/// Where a round is, as far as its cost goes: the two phases cost
/// differently in every world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase {
    /// The custom screens: the players choose their chips.
    Custom,
    /// The fight itself: the navis move and their chips fire.
    Fight,
    /// The rest of a round: its intro, a turn's start banner (the battle
    /// paused under it), its end.
    Other,
}

impl Phase {
    pub fn of(battle: &Battle) -> Phase {
        match (battle.round.top, battle.round.mode) {
            (top::RUNNING, mode::CUSTOM) => Phase::Custom,
            (top::RUNNING, mode::FIGHTING) if battle.fight.state == fight::FIGHTING => Phase::Fight,
            _ => Phase::Other,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Phase::Custom => "custom screen",
            Phase::Fight => "fight",
            Phase::Other => "other",
        }
    }
}

/// The phase a round is parked in at each tick, `0..=inputs.len()`, from a
/// plain run of it.
pub fn phases<G: Game + Clone>(start: &G, inputs: &[[G::Input; 2]]) -> Vec<Phase> {
    let mut game = start.clone();
    let mut phases = vec![Phase::of(game.battle())];
    for [a, b] in inputs {
        crate::world::step_game(&mut game, [a, b]);
        phases.push(Phase::of(game.battle()));
    }
    phases
}

/// By tick of a round, whether a [`Point`] `depth` deep there lies wholly
/// in `phase`: the round is parked in it from the point's tick to the tick
/// after the newest it simulates. ([`spread`] picks points among them.)
pub fn within(phases: &[Phase], phase: Phase, depth: usize) -> Vec<bool> {
    (0..phases.len()).map(|at| at + depth + 1 < phases.len() && phases[at..=at + depth + 1].iter().all(|&p| p == phase)).collect()
}

/// A battle world stepped by hand on a recorded round's inputs (by tick,
/// by side): the engine's [`Subject`].
pub struct Recorded<G: Game, O: Observer<G> = ()> {
    pub world: BattleWorld<G, O>,
    pub inputs: Vec<[G::Input; 2]>,
}

impl<G: Game, O: Observer<G>> Recorded<G, O> {
    /// Simulate tick `tick` as a session with both inputs in hand does: the
    /// step, and the tick settled (which the observer hears).
    pub fn settle(&mut self, tick: usize) {
        self.step(tick);
        self.world.observer_mut().confirmed(tick as u32, None);
    }
}

impl<G: Game, O: Observer<G>> Subject for Recorded<G, O> {
    type State = BattleState;

    fn step(&mut self, tick: usize) {
        let side = self.world.side();
        let [local, remote] = [&self.inputs[tick][side], &self.inputs[tick][1 - side]];
        let Ok(()) = self.world.step(local, std::slice::from_ref(remote));
    }

    fn save(&mut self) -> BattleState {
        let Ok(state) = self.world.save();
        state
    }

    fn load(&mut self, state: &BattleState) {
        let Ok(()) = self.world.load(state);
    }
}

/// A session on a battle world and a recorded round's inputs: the engine's
/// [`Peer`].
pub struct RecordedSession<G: Game, O: Observer<G> = ()> {
    pub session: Session<BattleWorld<G, O>>,
    pub inputs: Vec<[G::Input; 2]>,
}

impl<G: Game, O: Observer<G>> Peer for RecordedSession<G, O> {
    fn remote(&mut self, tick: usize) {
        let side = self.session.world().side();
        self.session.add_remote_input(0, self.inputs[tick][1 - side].clone(), 0);
    }

    fn advance(&mut self, tick: usize) -> u32 {
        let side = self.session.world().side();
        let Ok(_) = self.session.advance(self.inputs[tick][side].clone());
        self.session.last_misprediction_depth()
    }

    fn unconfirmed(&self) -> usize {
        self.session.local_queue_length()
    }

    fn matchable(&self) -> usize {
        self.session.matchable()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::standin::{StandInBattle, folder, netbattle};
    use nettai_battle::Battle;
    use nettai_battle::content::testing;
    use nettai_battle::input::keys;

    fn standin() -> StandInBattle {
        use testing::{SUN_GUN_1, SUN_GUN_3};
        let c = testing::content();
        let f = || folder(&c, &[(SUN_GUN_3, 0), (SUN_GUN_1, 0)]);
        StandInBattle::new(Battle::new(netbattle(&c, testing::LINK_BATTLE, 300, 0x1357, [f(), f()]), c))
    }

    fn inputs(ticks: usize) -> Vec<[u16; 2]> {
        (0..ticks).map(|t| [if t % 9 < 4 { keys::A } else { keys::LEFT }, if t / 5 % 3 == 0 { keys::UP } else { keys::B }]).collect()
    }

    /// Counts a world's calls.
    #[derive(Default)]
    struct Calls {
        steps: usize,
        saves: usize,
        loads: usize,
    }

    struct Counted(Recorded<StandInBattle>, Calls);

    impl Subject for Counted {
        type State = BattleState;
        fn step(&mut self, tick: usize) {
            self.1.steps += 1;
            self.0.step(tick);
        }
        fn save(&mut self) -> BattleState {
            self.1.saves += 1;
            self.0.save()
        }
        fn load(&mut self, state: &BattleState) {
            self.1.loads += 1;
            self.0.load(state);
        }
    }

    /// A point's rollback of `k` is one restore, `k + 1` steps and `k + 1`
    /// saves, ends on the state a plain run reaches, and closes back where
    /// it opened.
    #[test]
    fn a_point_rolls_back_and_closes_where_it_opened() {
        let inputs = inputs(60);
        let mut plain = Recorded { world: BattleWorld::new(standin(), 0), inputs: inputs.clone() };
        let mut s = Counted(Recorded { world: BattleWorld::new(standin(), 0), inputs }, Calls::default());
        for t in 0..20 {
            plain.step(t);
            s.0.step(t);
        }
        let opened = s.0.world.game().battle().digest();
        let mut costs = Costs::default();
        let mut point = Point::open(&mut s, 20, 8, &mut costs);
        for t in 20..29 {
            plain.step(t);
        }
        // A call's first rollback runs twice (once unmeasured).
        s.1 = Calls::default();
        point.run(&mut s, &[3], &mut costs);
        assert_eq!((s.1.loads, s.1.steps, s.1.saves), (2, 8, 8));
        assert_eq!(s.0.world.tick(), 29);
        assert_eq!(s.0.world.game().battle().digest(), plain.world.game().battle().digest());
        // And so does the first rollback of a depth at the point.
        s.1 = Calls::default();
        point.run(&mut s, &[3, 8], &mut costs);
        assert_eq!((s.1.loads, s.1.steps, s.1.saves), (2 + 2, 8 + 18, 8 + 18));
        assert_eq!(costs.rollback[&3].0.len(), 2);
        assert_eq!(costs.rollback[&8].0.len(), 1);
        assert_eq!(costs.restore.0.len(), 3);
        point.close(&mut s);
        assert_eq!(s.0.world.tick(), 20);
        assert_eq!(s.0.world.game().battle().digest(), opened);
    }

    /// What a point times is what a session does: with the other player's
    /// input `k + 1` frames late and wrong every frame, an advance is one
    /// load, `k + 1` steps and `k + 1` saves.
    #[test]
    fn a_session_rolls_back_as_a_point_does() {
        #[derive(Default)]
        struct Tally(Calls);
        impl<G> Observer<G> for Tally {
            fn rolled_back(&mut self, _: u32) {
                self.0.loads += 1;
            }
            fn simulated(&mut self, _: u32, _: &G) {
                self.0.steps += 1;
            }
        }
        const K: usize = 5;
        // The other player's buttons change every tick, so every prediction
        // (the last input again) is wrong.
        let inputs: Vec<[u16; 2]> = (0..80).map(|t| [0, if t % 2 == 0 { keys::UP } else { keys::DOWN }]).collect();
        let mut peer =
            RecordedSession { session: BattleWorld::with_observer(standin(), 0, Tally::default()).session(0), inputs };
        let mut drive = Drive::new(80, 0);
        let mut before = (0, 0);
        for w in 0..60usize {
            // The inputs before tick `w - K` have arrived: the presented
            // frame (tick `w`) runs `K` ticks past them.
            drive.frame(&mut peer, w.saturating_sub(K), 30);
            if w == 40 {
                let calls = &peer.session.world().observer().0;
                before = (calls.loads, calls.steps);
            }
        }
        let frames = &drive.frames[41..];
        assert!(frames.iter().all(|f| f.rollback == K as u32 && f.speculation == K as u32), "{frames:?}");
        let calls = &peer.session.world().observer().0;
        assert_eq!((calls.loads - before.0, calls.steps - before.1), (frames.len(), frames.len() * (K + 1)));
    }

    /// A network with no latency delivers every input in its frame; one
    /// with latency that many frames later, and what a lossy one loses
    /// comes with a later datagram.
    #[test]
    fn inputs_arrive_as_the_network_delivers_them() {
        assert_eq!(arrivals(NetworkConfig::latency(0, 0), 1, 5), [1, 2, 3, 4, 5]);
        assert_eq!(arrivals(NetworkConfig::latency(3, 0), 1, 6), [0, 0, 0, 1, 2, 3]);
        let lossy = arrivals(NetworkConfig::lossy(2, 2, 100, 300, 20), 7, 600);
        assert!(lossy.windows(2).all(|w| w[0] <= w[1]));
        assert!(lossy.iter().enumerate().all(|(w, &n)| n <= w.saturating_sub(1)));
        assert!(*lossy.last().unwrap() > 580, "{:?}", lossy.last());
        assert_eq!(lossy, arrivals(NetworkConfig::lossy(2, 2, 100, 300, 20), 7, 600));
    }

    /// A drive stops where the local input ends, and parks a peer whose
    /// inputs nothing confirms.
    #[test]
    fn a_drive_parks_and_ends() {
        let mut peer = RecordedSession { session: BattleWorld::new(standin(), 0).session(0), inputs: inputs(12) };
        let mut drive = Drive::new(12, 0);
        for _ in 0..8 {
            assert!(drive.frame(&mut peer, 0, 4));
        }
        assert_eq!((drive.advanced(), drive.parked), (4, 4));
        for w in 0..20 {
            drive.frame(&mut peer, w, 4);
        }
        assert!(drive.done() && !drive.frame(&mut peer, 12, 4));
        assert_eq!(drive.frames.len(), 12);
        assert_eq!(Samples(vec![5, 1, 4, 2, 3]).summary().median, 3);
    }

    /// The reports' figures and rows.
    #[test]
    fn the_reports_read() {
        assert_eq!([micros(1_234), micros(12_345), micros(1_234_567)], ["1.23", "12.3", "1235"]);
        assert_eq!(spread(&[false, true, true, false, true, true, true, true], 3), [2, 5, 7]);
        assert_eq!(spread(&[false, true, false], 3), [1]);
        assert_eq!(describe(&LINKS[2].1), "134 to 201 ms one way, 5% lost (40% after a loss), 1% duplicated");
        use Phase::{Custom, Fight, Other};
        let round = [Other, Custom, Custom, Custom, Custom, Fight, Fight, Fight, Fight, Fight, Other];
        assert_eq!(spread(&within(&round, Custom, 2), 9), [1]);
        assert_eq!(spread(&within(&round, Fight, 2), 9), [5, 6]);
        assert_eq!(spread(&within(&round, Fight, 2), 1), [6]);
        let game = standin();
        assert_eq!(phases(&game, &inputs(3)), [Other; 4]);
        let mut a = Costs::default();
        a.step.push(2_000);
        a.rollback.entry(4).or_default().push(FRAME_NANOS / 2);
        let mut b = Costs::default();
        b.step.push(1_000);
        let table = costs_table(&[("A", &a), ("B", &b)]);
        assert!(table.contains("| one step | 2.00 (2.00 to 2.00; 2.00) | 1.00 (1.00 to 1.00; 1.00) | 2.0x |"), "{table}");
        assert!(table.contains("| a rollback of 4 (a restore, 5 steps and saves) | 8371 (8371 to 8371; 8371), 50% of a frame | - | - |"), "{table}");
        let frames = [Advanced { tick: 0, nanos: 1_000, rollback: 0, speculation: 2 }, Advanced { tick: 1, nanos: 3_000, rollback: 2, speculation: 3 }];
        let table = frames_table(&[("x".to_string(), vec![&frames[..]])]);
        assert!(table.contains("| x | 2 | 3.00 | 3.00 | 3.00 | 0.02% | 3.00 to 3.00 | 3.00 to 3.00 | 1 (50.0%) | 3.00 | 2.0 | 2 | 3 | 0 |"), "{table}");
    }
}
