//! How good the connection to the other player is: what an indicator of
//! four bars needs, in the lobby (`Lobby::ping`) and in a match
//! (`Player::net_status`).
//!
//! Two figures are measured, both in milliseconds: the **ping**, the round
//! trip of a message to the other player and back, smoothed; and the
//! **jitter**, how unsteady it is, the smoothed distance of each round trip
//! from that mean. They are RFC 6298's SRTT and RTTVAR: each new round trip
//! moves the ping an eighth of the way to it and the jitter a quarter of the
//! way to its distance from the ping.
//!
//! The **quality** is a level from them ([`Quality::of`]). What a rollback
//! match feels is the other player's input arriving late, by half the round
//! trip, and by more when it is unsteady: an input later than the player's
//! input delay is predicted, and a wrong prediction is a rollback. So the
//! level goes by the ping plus twice the jitter (a round trip few are slower
//! than), against how many ticks each way that is (a tick is 16.7 ms):
//!
//! | Level | Bars | Ping + 2 × jitter | Each way |
//! |---|---|---|---|
//! | [`Quality::Excellent`] | 4 | up to [`EXCELLENT_MS`] (70 ms) | about 2 ticks |
//! | [`Quality::Good`] | 3 | up to [`GOOD_MS`] (135 ms) | about 4 ticks: the default input delay hides it |
//! | [`Quality::Fair`] | 2 | up to [`FAIR_MS`] (200 ms) | about 6 ticks: rollbacks of a tick or two at the default input delay |
//! | [`Quality::Poor`] | 1 | more | more |
//!
//! While the connection is down and being made again there is no quality to
//! show: a host shows that instead ([`Reconnecting`]), with the time left.

use std::time::Duration;

/// The slowest ping plus twice the jitter, in milliseconds, of an
/// excellent connection (four bars): about two ticks each way.
pub const EXCELLENT_MS: f64 = 70.0;
/// The same of a good connection (three bars): about four ticks each way,
/// the default input delay.
pub const GOOD_MS: f64 = 135.0;
/// The same of a fair connection (two bars): about six ticks each way.
pub const FAIR_MS: f64 = 200.0;

/// How good a connection is, for an indicator of four bars (the module's
/// docs have the thresholds).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Quality {
    Poor = 1,
    Fair = 2,
    Good = 3,
    Excellent = 4,
}

impl Quality {
    /// The quality of a connection with this ping and this jitter, both in
    /// milliseconds.
    pub fn of(ping_ms: f64, jitter_ms: f64) -> Quality {
        let slow = ping_ms + 2.0 * jitter_ms;
        if slow <= EXCELLENT_MS {
            Quality::Excellent
        } else if slow <= GOOD_MS {
            Quality::Good
        } else if slow <= FAIR_MS {
            Quality::Fair
        } else {
            Quality::Poor
        }
    }

    /// How many of four bars it lights: 1 to 4.
    pub fn bars(self) -> u8 {
        self as u8
    }
}

/// A connection's round trip as measured so far: the ping and the jitter,
/// in milliseconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ping {
    /// The round trip, smoothed.
    pub ms: f64,
    /// How unsteady it is: the smoothed distance of a round trip from `ms`.
    pub jitter_ms: f64,
}

impl Ping {
    /// The ping after one more round trip of `sample_ms` (the first: that
    /// round trip, and half of it for the jitter, as RFC 6298 starts).
    pub fn after(ping: Option<Ping>, sample_ms: f64) -> Ping {
        match ping {
            None => Ping { ms: sample_ms, jitter_ms: sample_ms / 2.0 },
            Some(Ping { ms, jitter_ms }) => Ping { ms: ms + (sample_ms - ms) / 8.0, jitter_ms: jitter_ms + ((sample_ms - ms).abs() - jitter_ms) / 4.0 },
        }
    }

    pub fn quality(self) -> Quality {
        Quality::of(self.ms, self.jitter_ms)
    }
}

/// What went one way over a connection: datagrams, and their bytes (the
/// protocol's frames, without the transport's headers).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Traffic {
    pub datagrams: u64,
    pub bytes: u64,
}

/// A connection that is down while its transport makes it again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reconnecting {
    /// How long it has been down.
    pub elapsed: Duration,
    /// How long the transport goes on trying before it gives up and the
    /// match ends, if it says: what a countdown shows.
    pub gives_up_in: Option<Duration>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The levels at their thresholds, the jitter counting twice.
    #[test]
    fn quality_goes_by_ping_and_jitter() {
        assert_eq!(Quality::of(0.0, 0.0), Quality::Excellent);
        assert_eq!((Quality::of(70.0, 0.0), Quality::of(70.1, 0.0)), (Quality::Excellent, Quality::Good));
        assert_eq!((Quality::of(135.0, 0.0), Quality::of(135.1, 0.0)), (Quality::Good, Quality::Fair));
        assert_eq!((Quality::of(200.0, 0.0), Quality::of(200.1, 0.0)), (Quality::Fair, Quality::Poor));
        // A steady 60 ms is excellent; the same ping, unsteady by 10 ms, is
        // good, and by 40 ms fair.
        assert_eq!((Quality::of(60.0, 5.0), Quality::of(60.0, 10.0), Quality::of(60.0, 40.0)), (Quality::Excellent, Quality::Good, Quality::Fair));
        assert_eq!([Quality::Poor, Quality::Fair, Quality::Good, Quality::Excellent].map(Quality::bars), [1, 2, 3, 4]);
        assert!(Quality::Poor < Quality::Excellent);
    }

    /// A steady round trip settles at itself with no jitter; a jump moves
    /// the ping slowly and the jitter at once.
    #[test]
    fn ping_is_smoothed() {
        let mut ping = None;
        for _ in 0..100 {
            ping = Some(Ping::after(ping, 40.0));
        }
        let steady = ping.unwrap();
        assert!((steady.ms - 40.0).abs() < 1e-6 && steady.jitter_ms < 1e-6, "{steady:?}");
        assert_eq!(steady.quality(), Quality::Excellent);
        let jumped = Ping::after(ping, 200.0);
        assert!((jumped.ms - 60.0).abs() < 1e-6 && (jumped.jitter_ms - 40.0).abs() < 1e-6, "{jumped:?}");
        assert_eq!(jumped.quality(), Quality::Fair);
    }
}
