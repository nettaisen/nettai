//! A simulated one-way link that delivers in order, as the ordered
//! channel netplay runs on does (getgud takes each remote's inputs in tick
//! order). Each packet takes `latency` ticks plus up to `jitter` more; one
//! that would overtake the packet before it waits for it instead
//! (head-of-line blocking), so late packets arrive in bursts. Time is
//! counted in wall-clock frames.

use std::collections::VecDeque;

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
    /// (arrival time, payload), in the order sent; arrival times never
    /// decrease.
    in_flight: VecDeque<(u64, T)>,
}

impl<T> Link<T> {
    pub fn new(config: LinkConfig, seed: u64) -> Link<T> {
        Link { config, rng: SplitMix64::new(seed), in_flight: VecDeque::new() }
    }

    /// Send at wall time `now`.
    pub fn send(&mut self, now: u64, payload: T) {
        let delay = self.config.latency as u64 + self.rng.below(self.config.jitter as u64 + 1);
        let arrival = self.in_flight.back().map_or(0, |p| p.0).max(now + delay);
        self.in_flight.push_back((arrival, payload));
    }

    /// Packets that have arrived by wall time `now`, in the order sent.
    pub fn receive(&mut self, now: u64) -> impl Iterator<Item = T> + '_ {
        let arrived = self.in_flight.iter().take_while(|p| p.0 <= now).count();
        self.in_flight.drain(..arrived).map(|p| p.1)
    }

    pub fn in_flight(&self) -> usize {
        self.in_flight.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packets_arrive_in_order_after_the_latency() {
        let mut link = Link::new(LinkConfig { latency: 3, jitter: 4 }, 7);
        let mut got = Vec::new();
        for now in 0..200u64 {
            if now < 100 {
                link.send(now, now);
            }
            for sent in link.receive(now) {
                // Waiting for the packet before never takes longer than
                // the longest delay: that one was sent earlier.
                assert!((3..=7).contains(&(now - sent)), "sent {sent}, arrived {now}");
                got.push((sent, now));
            }
        }
        assert_eq!(got.iter().map(|p| p.0).collect::<Vec<_>>(), (0..100).collect::<Vec<_>>());
        // Some packets waited for a slower one before them.
        assert!(got.windows(2).any(|w| w[0].1 == w[1].1));
    }
}
