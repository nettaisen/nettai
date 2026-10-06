//! One peer's end of the input exchange: its player's input stream out and
//! the other player's in, on rennet's reliability streams over whatever
//! datagram channel the host has (the program's UDP socket, or the
//! simulated `network`). Pure: the host passes in the datagrams that arrived and the
//! time, and sends the datagram [`InputLink::datagram`] gives it.
//!
//! Every datagram is one rennet frame ([`crate::protocol::Frame`]): the
//! unconfirmed tail of this player's stream (rennet's redundancy window,
//! which recovers a lost datagram with the next one), the cumulative ack
//! of the other's, and the tick advantage. The host sends one every frame,
//! input or not (an "ack-only" frame while it waits), so acks and the
//! window keep flowing.

use std::collections::VecDeque;
use std::fmt;
use std::io;
use std::marker::PhantomData;

use rennet::{InStream, OutStream};

use crate::protocol::{Element, Frame, HORIZON, Meta, Netplay, WireInput};

/// What the other player's stream delivered, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Delivery<I> {
    /// Their next tick's input, with their tick advantage (the freshest the
    /// link has seen).
    Input { input: I, tick_advantage: i16 },
    /// Their round is over: their inputs after this are the next round's.
    RoundEnd,
    /// They left the match.
    MatchEnd,
}

/// Why the other player's stream can't go on.
#[derive(Debug)]
pub enum LinkError {
    /// A datagram that isn't a frame of the protocol, or an input that
    /// doesn't decode.
    Malformed(io::Error),
    /// A gap wider than the horizon: input that can no longer be recovered
    /// in time. The match is over.
    HorizonExceeded,
}

impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LinkError::Malformed(e) => write!(f, "a malformed datagram from the other player ({e})"),
            LinkError::HorizonExceeded => write!(f, "the other player's input fell more than the rollback horizon behind"),
        }
    }
}

impl std::error::Error for LinkError {}

/// What a link sent and received. Times are in the unit the host passes
/// (the simulator's wall frames, a frontend's milliseconds).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LinkStats {
    /// Datagrams and bytes sent (the frames: no transport header).
    pub sent: u64,
    pub sent_bytes: u64,
    /// The largest datagram sent.
    pub largest: usize,
    /// Elements sent, counting each copy the redundancy window sent again.
    pub sent_elements: u64,
    /// Datagrams and bytes received.
    pub received: u64,
    pub received_bytes: u64,
    /// Received elements that had been delivered already (copies).
    pub redundant: u64,
    /// Elements delivered, each once, in order.
    pub delivered: u64,
    /// Datagrams that arrived after a newer one.
    pub reordered: u64,
    /// The other player's datagrams with new input that never arrived
    /// (an estimate: each of their frames ends at a newer element than the
    /// one before, except around a payload or a marker).
    pub lost: u64,
    /// Their datagrams with new input that did arrive.
    pub fresh: u64,
    /// The latest round trip of an ack, and its smoothed value.
    pub rtt: Option<u64>,
    pub srtt: Option<f64>,
}

impl LinkStats {
    /// The share of the other player's datagrams with new input that were
    /// lost (an estimate, see [`LinkStats::lost`]).
    pub fn loss(&self) -> f64 {
        self.lost as f64 / (self.lost + self.fresh).max(1) as f64
    }

    /// Bytes per datagram sent, on average.
    pub fn mean_size(&self) -> f64 {
        self.sent_bytes as f64 / self.sent.max(1) as f64
    }
}

/// One peer's end of the exchange, for inputs `I`.
pub struct InputLink<I> {
    out: OutStream<Netplay>,
    inn: InStream<Netplay>,
    horizon: u32,
    stats: LinkStats,
    /// The newest element of each datagram sent, the first time it went
    /// out, and when: an ack covering it times the round trip.
    sent_at: VecDeque<(u32, u64)>,
    /// The newest element any of the other's datagrams reached, and which
    /// of the 64 before it were the newest of a datagram that arrived.
    newest: Option<u32>,
    seen: u64,
    _input: PhantomData<fn() -> I>,
}

impl<I: WireInput> InputLink<I> {
    /// A link that gives up on a gap wider than `horizon` elements (at most
    /// [`HORIZON`]).
    pub fn new(horizon: u32) -> InputLink<I> {
        assert!((1..=HORIZON).contains(&horizon), "the horizon is 1 to {HORIZON}, not {horizon}");
        InputLink {
            out: OutStream::new(horizon),
            inn: InStream::new(horizon),
            horizon,
            stats: LinkStats::default(),
            sent_at: VecDeque::new(),
            newest: None,
            seen: 0,
            _input: PhantomData,
        }
    }

    pub fn horizon(&self) -> u32 {
        self.horizon
    }

    /// This player's next tick, which carries the tick advantage.
    pub fn push(&mut self, input: &I, tick_advantage: i16) {
        let (held, flags) = input.encode();
        self.out.push_with_meta(Element::Tick { held, flags }, Meta { tick_advantage });
    }

    /// This player's round is over.
    pub fn push_round_end(&mut self) {
        self.out.push(Element::RoundEnd);
    }

    /// This player leaves the match.
    pub fn push_match_end(&mut self) {
        self.out.push(Element::MatchEnd);
    }

    /// The datagram to send now: the unconfirmed tail of this player's
    /// stream, the ack of the other's, and the tick advantage.
    pub fn datagram(&mut self, now: u64) -> Vec<u8> {
        let w = self.out.window();
        if let Some(newest) = self.out.newest_seq()
            && self.sent_at.back().is_none_or(|&(s, _)| s < newest)
        {
            self.sent_at.push_back((newest, now));
        }
        self.stats.sent_elements += w.entries.len() as u64;
        let bytes = Frame::new(w.base, self.inn.ack(), w.meta, w.entries).to_vec();
        self.stats.sent += 1;
        self.stats.sent_bytes += bytes.len() as u64;
        self.stats.largest = self.stats.largest.max(bytes.len());
        bytes
    }

    /// Take a datagram from the other peer, received at `now`, and add what
    /// it delivers to `delivered`. A malformed datagram changes nothing; a
    /// gap past the horizon breaks the link for good.
    pub fn receive(&mut self, datagram: &[u8], now: u64, delivered: &mut Vec<Delivery<I>>) -> Result<(), LinkError> {
        let frame = Frame::decode(&mut &datagram[..]).map_err(LinkError::Malformed)?;
        self.stats.received += 1;
        self.stats.received_bytes += datagram.len() as u64;
        self.out.apply_ack(frame.ack());
        let acked = self.out.peer_ack_base();
        let mut covered = None;
        while let Some(&(seq, at)) = self.sent_at.front()
            && seq < acked
        {
            covered = Some(at);
            self.sent_at.pop_front();
        }
        if let Some(at) = covered {
            let rtt = now.saturating_sub(at);
            self.stats.rtt = Some(rtt);
            self.stats.srtt = Some(self.stats.srtt.map_or(rtt as f64, |s| s + (rtt as f64 - s) / 8.0));
        }
        self.note_newest(&frame);
        let before = self.inn.ack();
        self.stats.redundant += (before.saturating_sub(frame.base) as usize).min(frame.entries.len()) as u64;
        let window = self.inn.accept(&frame).map_err(|_| LinkError::HorizonExceeded)?;
        self.stats.delivered += window.entries.len() as u64;
        for e in window.entries {
            match e {
                Element::Tick { held, flags } => {
                    let input = I::decode(held, flags).map_err(LinkError::Malformed)?;
                    delivered.push(Delivery::Input { input, tick_advantage: window.meta.tick_advantage });
                }
                Element::RoundEnd => delivered.push(Delivery::RoundEnd),
                Element::MatchEnd => delivered.push(Delivery::MatchEnd),
            }
        }
        Ok(())
    }

    /// Count the other's datagrams by the newest element each reached
    /// (`seen` bit k: element `newest - k` was a datagram's newest). One
    /// that reaches further than any before is fresh; an element skipped on
    /// the way counts as lost once it falls 64 behind without a late
    /// datagram reaching it.
    fn note_newest(&mut self, frame: &Frame) {
        let Some(last) = frame.entries.len().checked_sub(1) else { return };
        let newest = frame.base.saturating_add(last as u32);
        let Some(top) = self.newest else {
            self.newest = Some(newest);
            // (Bits past the stream's start count as seen.)
            self.seen = if newest >= 63 { 1 } else { 1 | !0u64 << (newest + 1) };
            self.stats.fresh += 1;
            return;
        };
        if newest > top {
            let shift = newest - top;
            let (leaving, skipped_past) = if shift >= 64 { (self.seen, (shift - 64) as u64) } else { (self.seen >> (64 - shift), 0) };
            let leaving_bits = shift.min(64) as u64;
            self.stats.lost += leaving_bits - leaving.count_ones() as u64 + skipped_past;
            self.seen = if shift >= 64 { 1 } else { self.seen << shift | 1 };
            self.newest = Some(newest);
            self.stats.fresh += 1;
        } else if newest < top {
            self.stats.reordered += 1;
            let back = top - newest;
            if back < 64 && self.seen & (1 << back) == 0 {
                self.seen |= 1 << back;
                self.stats.fresh += 1;
            }
        }
    }

    pub fn stats(&self) -> &LinkStats {
        &self.stats
    }

    /// Elements of this player's stream the other peer hasn't acknowledged.
    pub fn unacked(&self) -> u32 {
        self.out.next_seq() - self.out.peer_ack_base()
    }

    /// Elements of the other's stream received in order so far.
    pub fn received_elements(&self) -> u32 {
        self.inn.ack()
    }
}
