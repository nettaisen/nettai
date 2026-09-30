//! A simulated one-way link: each packet arrives `latency` ticks after it
//! was sent, plus up to `jitter` more (so packets can overtake each
//! other). Time is counted in wall-clock frames.

use crate::rng::SplitMix64;

/// Link timing, in frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkConfig {
    pub latency: u32,
    pub jitter: u32,
}

/// Packets in flight on one link.
#[derive(Clone, Debug)]
pub struct Link<T> {
    config: LinkConfig,
    rng: SplitMix64,
    /// (arrival time, send order, payload).
    in_flight: Vec<(u64, u64, T)>,
    sent: u64,
}

impl<T> Link<T> {
    pub fn new(config: LinkConfig, seed: u64) -> Link<T> {
        Link { config, rng: SplitMix64::new(seed), in_flight: Vec::new(), sent: 0 }
    }

    /// Send at wall time `now`.
    pub fn send(&mut self, now: u64, payload: T) {
        let delay = self.config.latency as u64 + self.rng.below(self.config.jitter as u64 + 1);
        self.in_flight.push((now + delay, self.sent, payload));
        self.sent += 1;
    }

    /// Packets that have arrived by wall time `now`, in arrival order.
    pub fn receive(&mut self, now: u64) -> Vec<T> {
        let (mut arrived, waiting): (Vec<_>, Vec<_>) = self.in_flight.drain(..).partition(|p| p.0 <= now);
        self.in_flight = waiting;
        arrived.sort_by_key(|p| (p.0, p.1));
        arrived.into_iter().map(|p| p.2).collect()
    }

    pub fn in_flight(&self) -> usize {
        self.in_flight.len()
    }
}
