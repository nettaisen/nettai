//! The link cable between the two consoles, as part of the simulation.
//!
//! In the original each console sends a packet every frame (its player's
//! held buttons and its status byte, plus one word of a block transfer
//! when one is running) and both consoles apply both packets `delay`
//! frames later, the same frame on both. Here both players' joypads come
//! in on every tick and the link carries them to the fight: what the
//! fight sees on a tick is what the players held `delay` ticks earlier.
//! See docs/engine/custom-screen.md §6 and battle-flow.md §6.

use std::collections::VecDeque;

/// One player's packet for a tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Packet {
    /// The buttons the player held when the packet went out.
    pub held: u16,
    /// The sender's custom screen was open (status bit 2).
    pub in_custom: bool,
}

/// Packets on their way, both players' per tick.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Link {
    /// Ticks from sending to arrival.
    pub delay: u8,
    /// Packets sent and not yet arrived, oldest first.
    in_flight: VecDeque<[Packet; 2]>,
}

impl Link {
    /// The recorded sessions' latency.
    pub const RECORDED_DELAY: u8 = 4;

    /// A link with nothing on its way (no buttons held, no custom screen
    /// open).
    pub fn new(delay: u8) -> Link {
        Link { delay, in_flight: (0..delay).map(|_| [Packet::default(); 2]).collect() }
    }

    /// Send this tick's packets; returns the packets that arrive this tick
    /// (sent `delay` ticks ago).
    pub fn exchange(&mut self, sent: [Packet; 2]) -> [Packet; 2] {
        self.in_flight.push_back(sent);
        self.in_flight.pop_front().expect("a packet is in flight")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packets_arrive_after_the_delay() {
        let mut link = Link::new(4);
        let p = |held| [Packet { held, in_custom: false }; 2];
        let arrived: Vec<u16> = (1..=6).map(|t| link.exchange(p(t))[0].held).collect();
        assert_eq!(arrived, [0, 0, 0, 0, 1, 2]);
        let mut direct = Link::new(0);
        assert_eq!(direct.exchange(p(7))[1].held, 7);
    }
}
