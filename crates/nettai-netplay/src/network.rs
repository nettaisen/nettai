//! A simulated datagram network, one direction of it: what the Internet
//! does to UDP (or to WebRTC's unordered, unreliable data channel). Each
//! datagram takes `latency` frames plus up to `jitter` more, independently
//! of the others, so jitter reorders them; some are lost, alone or in
//! bursts; some arrive twice. Everything is drawn from a seed, so a run
//! repeats exactly. Time is counted in wall-clock frames.

use crate::rng::SplitMix64;

/// How a direction of the network behaves. Probabilities are in parts per
/// thousand.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NetworkConfig {
    /// Frames every datagram takes.
    pub latency: u32,
    /// Up to this many frames more, uniformly, for each datagram on its own.
    pub jitter: u32,
    /// A datagram is lost with this probability...
    pub loss: u32,
    /// ... and once one is lost, the next is with this one (a burst; 0 for
    /// independent losses).
    pub burst: u32,
    /// A datagram that isn't lost arrives twice with this probability, the
    /// copy with its own delay.
    pub duplicate: u32,
    /// Every datagram sent from frame `.0` up to `.1` is lost (an outage).
    pub outage: Option<(u64, u64)>,
}

impl NetworkConfig {
    /// `latency` frames, up to `jitter` more, nothing lost or duplicated.
    pub fn latency(latency: u32, jitter: u32) -> NetworkConfig {
        NetworkConfig { latency, jitter, ..NetworkConfig::default() }
    }

    /// The same with losses (`loss` per thousand, bursts continuing with
    /// `burst` per thousand) and duplicates.
    pub fn lossy(latency: u32, jitter: u32, loss: u32, burst: u32, duplicate: u32) -> NetworkConfig {
        NetworkConfig { latency, jitter, loss, burst, duplicate, outage: None }
    }

    /// The most frames a datagram that arrives takes.
    pub fn max_delay(&self) -> u32 {
        self.latency + self.jitter
    }
}

/// What a direction did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NetworkStats {
    pub sent: u64,
    pub lost: u64,
    /// Extra copies delivered.
    pub duplicated: u64,
    /// Datagrams delivered after one sent later.
    pub reordered: u64,
    /// The longest run of datagrams lost in a row.
    pub longest_burst: u64,
}

/// Datagrams in flight in one direction.
#[derive(Clone, Debug)]
pub struct Network {
    config: NetworkConfig,
    rng: SplitMix64,
    /// (arrival frame, sent order, datagram), sorted by arrival, then order.
    in_flight: Vec<(u64, u64, Vec<u8>)>,
    sent: u64,
    /// The previous datagram was lost (a burst goes on), and the run so far.
    in_burst: bool,
    burst: u64,
    /// The latest sent order delivered.
    newest_delivered: Option<u64>,
    stats: NetworkStats,
}

impl Network {
    pub fn new(config: NetworkConfig, seed: u64) -> Network {
        Network {
            config,
            rng: SplitMix64::new(seed),
            in_flight: Vec::new(),
            sent: 0,
            in_burst: false,
            burst: 0,
            newest_delivered: None,
            stats: NetworkStats::default(),
        }
    }

    pub fn config(&self) -> &NetworkConfig {
        &self.config
    }

    pub fn stats(&self) -> &NetworkStats {
        &self.stats
    }

    /// Send a datagram at wall frame `now`.
    pub fn send(&mut self, now: u64, datagram: Vec<u8>) {
        let c = self.config;
        let order = self.sent;
        self.sent += 1;
        self.stats.sent += 1;
        let p = if self.in_burst && c.burst > 0 { c.burst } else { c.loss };
        let lost = c.outage.is_some_and(|(from, to)| (from..to).contains(&now)) || (p > 0 && self.rng.chance(p as u64, 1000));
        self.in_burst = lost;
        if lost {
            self.stats.lost += 1;
            self.burst += 1;
            self.stats.longest_burst = self.stats.longest_burst.max(self.burst);
            return;
        }
        self.burst = 0;
        let copies = if c.duplicate > 0 && self.rng.chance(c.duplicate as u64, 1000) { 2 } else { 1 };
        self.stats.duplicated += copies - 1;
        for _ in 0..copies {
            let arrival = now + c.latency as u64 + self.rng.below(c.jitter as u64 + 1);
            let at = self.in_flight.partition_point(|p| (p.0, p.1) <= (arrival, order));
            self.in_flight.insert(at, (arrival, order, datagram.clone()));
        }
    }

    /// The datagrams that have arrived by wall frame `now`, in the order
    /// they arrive.
    pub fn receive(&mut self, now: u64) -> Vec<Vec<u8>> {
        let arrived = self.in_flight.partition_point(|p| p.0 <= now);
        let out: Vec<(u64, u64, Vec<u8>)> = self.in_flight.drain(..arrived).collect();
        for &(_, order, _) in &out {
            match self.newest_delivered {
                Some(newest) if order < newest => self.stats.reordered += 1,
                _ => self.newest_delivered = Some(order),
            }
        }
        out.into_iter().map(|p| p.2).collect()
    }

    pub fn in_flight(&self) -> usize {
        self.in_flight.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(config: NetworkConfig, seed: u64) -> (Vec<(u64, u64)>, NetworkStats) {
        let mut net = Network::new(config, seed);
        let mut got = Vec::new();
        for now in 0..2000u64 {
            if now < 1000 {
                net.send(now, now.to_le_bytes().to_vec());
            }
            for d in net.receive(now) {
                got.push((u64::from_le_bytes(d.try_into().unwrap()), now));
            }
        }
        (got, *net.stats())
    }

    #[test]
    fn datagrams_take_the_latency_and_jitter_reorders_them() {
        let (got, stats) = run(NetworkConfig::latency(3, 4), 7);
        assert_eq!(got.len(), 1000);
        for &(sent, arrived) in &got {
            assert!((3..=7).contains(&(arrived - sent)), "sent {sent}, arrived {arrived}");
        }
        assert!(stats.reordered > 100, "{stats:?}");
        let mut order: Vec<u64> = got.iter().map(|g| g.0).collect();
        order.sort();
        assert_eq!(order, (0..1000).collect::<Vec<_>>());
    }

    #[test]
    fn losses_bursts_and_duplicates() {
        let (got, stats) = run(NetworkConfig::lossy(2, 1, 100, 500, 50), 3);
        assert_eq!(got.len() as u64, stats.sent - stats.lost + stats.duplicated);
        // About 10% start a burst that goes on half the time: about 1 in 6.
        assert!((100..260).contains(&stats.lost), "{stats:?}");
        assert!(stats.longest_burst >= 3, "{stats:?}");
        assert!((20..90).contains(&stats.duplicated), "{stats:?}");
        // The same seed, the same run.
        assert_eq!(run(NetworkConfig::lossy(2, 1, 100, 500, 50), 3).0, got);
    }

    #[test]
    fn an_outage_loses_everything_sent_in_it() {
        let config = NetworkConfig { outage: Some((100, 200)), ..NetworkConfig::latency(2, 0) };
        let (got, stats) = run(config, 1);
        assert_eq!(stats.lost, 100);
        assert!(got.iter().all(|&(sent, _)| !(100..200).contains(&sent)));
    }
}
