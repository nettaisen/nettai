//! nettai's netplay protocol on rennet: what each player's stream of
//! elements carries, and its wire form.
//!
//! A player's peer sends its player's input for every tick, in order, on
//! one rennet stream ([`rennet::OutStream`] / [`rennet::InStream`]): the
//! transport is a datagram channel that loses, reorders and duplicates,
//! and rennet makes it an ordered stream again (docs/design/rollback.md
//! §4.6). This module names the stream's [`Element`]s and the per-frame
//! [`Meta`], and packs them byte-minimally:
//!
//! - [`Element::Tick`]: one tick of the player's input, their held buttons
//!   and a few flags for the tick's events. A tick with no flags and only A,
//!   B and the directions held is one byte; any other buttons, two; flags,
//!   up to three.
//! - [`Element::Payload`]: bytes the next tick carries besides (the rare
//!   event with data, such as a recorded custom-screen result), up to
//!   [`CHUNK`] per element, in order before that tick.
//! - [`Element::RoundEnd`] and [`Element::MatchEnd`]: the in-band markers.
//!   The sender's round is over, and what follows is the next round's
//!   input; the sender left the match.
//! - [`Meta`]: the sender's tick advantage (getgud's
//!   `local_tick_advantage`) as of its newest input, for clock sync.
//!
//! A game's input goes on the wire through [`WireInput`]: its buttons, its
//! flags and its payload.
//!
//! The element's wire form is a LEB128 head, then a payload element's
//! bytes:
//!
//! ```text
//! head & 1 == 0   a tick: head >> 1 = buttons (10 bits, in WIRE_ORDER) | flags << 10
//! head & 1 == 1   head >> 1: 0 the round's end, 1 the match's end,
//!                 2 + n - 1 a payload of n bytes (1..=CHUNK), which follow
//! ```

use std::io::{self, Read, Write};

use nettai_battle::input::keys;
use rennet::{read_svarint, read_uvarint, write_svarint, write_uvarint};

/// The protocol's version: peers whose versions differ can't play together
/// (the handshake refuses).
/// 2: a transformation request carries BN5's soul turns and Chaos Unison.
/// 3: a player's offer carries their tactics (BN5's computer-navi data).
/// 4: an offer's side carries the navi code's level as an option, Beast
/// Out unlocked and the SP deletion times, and a player's BN6 setup is its
/// systems' (the battle's digest differs).
/// 5: an offer's side carries its systems' setups (by system and field) and
/// the souls it has; NaviStats carries BN5's Hub Style; a system's setup
/// starts from its defaults (BN5's light/dark value 500).
pub const VERSION: u16 = 5;

/// The rollback horizon, in elements (ticks, besides the rare payload or
/// marker): the widest gap a player's stream may have at the other peer
/// before that peer gives up on it ([`rennet::HorizonExceeded`]), and the
/// most unconfirmed elements a sender keeps to send again. getgud itself
/// has no limit; a peer's stall guard (`max_lead`) bounds how far its
/// player's input runs ahead of the other's, and each side's lead adds up,
/// so a gap reaches at most twice the stall guard. [`max_lead`] keeps the
/// stall guard below half the horizon. 240 ticks are four seconds.
pub const HORIZON: u32 = 240;

/// The largest stall guard a horizon allows: a gap reaches up to twice the
/// stall guard, plus the payloads and markers in it.
pub fn max_lead(horizon: u32) -> u32 {
    horizon.saturating_sub(MARGIN) / 2
}

/// The elements a horizon leaves for payloads and markers in a gap.
const MARGIN: u32 = 16;

/// The most bytes one payload element carries.
pub const CHUNK: usize = 32;

/// The protocol: rennet's `Protocol` for nettai's input streams.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Netplay;

impl rennet::Protocol for Netplay {
    type Element = Element;
    type Meta = Meta;
    /// A sender's window is at most a horizon of elements.
    const MAX_RUN: usize = HORIZON as usize;
}

/// One datagram of the protocol.
pub type Frame = rennet::Frame<Netplay>;

/// One element of a player's stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Element {
    /// One tick of the player's input: the buttons they hold (the GBA's
    /// bits, `nettai_battle::input::keys`) and the game's flags for the
    /// tick's events ([`WireInput`]).
    Tick { held: u16, flags: u8 },
    /// Bytes the next tick carries besides, in order.
    Payload(Chunk),
    /// The sender's round is over: the elements after it are the next
    /// round's.
    RoundEnd,
    /// The sender left the match.
    MatchEnd,
}

/// Up to [`CHUNK`] bytes of a tick's payload.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    len: u8,
    bytes: [u8; CHUNK],
}

impl Chunk {
    /// The first [`CHUNK`] bytes of `bytes` (at least one).
    pub fn new(bytes: &[u8]) -> Chunk {
        assert!(!bytes.is_empty() && bytes.len() <= CHUNK, "a chunk holds 1 to {CHUNK} bytes, not {}", bytes.len());
        let mut c = Chunk { len: bytes.len() as u8, bytes: [0; CHUNK] };
        c.bytes[..bytes.len()].copy_from_slice(bytes);
        c
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }
}

impl std::fmt::Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Chunk({:02x?})", self.bytes())
    }
}

/// The buttons' order on the wire, lowest bit first: A, B and the
/// directions (what a player holds most) in the six bits a one-byte tick
/// has room for, then SELECT, START, R and L.
pub const WIRE_ORDER: [u16; 10] =
    [keys::A, keys::B, keys::RIGHT, keys::LEFT, keys::UP, keys::DOWN, keys::SELECT, keys::START, keys::R, keys::L];

/// The buttons as the wire orders them.
pub fn buttons_to_wire(held: u16) -> u16 {
    WIRE_ORDER.iter().enumerate().fold(0, |w, (bit, &key)| w | (((held & key != 0) as u16) << bit))
}

/// The buttons from their wire order.
pub fn buttons_from_wire(wire: u16) -> u16 {
    WIRE_ORDER.iter().enumerate().fold(0, |held, (bit, &key)| if wire & (1 << bit) != 0 { held | key } else { held })
}

/// The buttons a tick carries (the GBA's ten).
pub const BUTTONS: u16 = 0x3FF;

const ROUND_END: u64 = 0;
const MATCH_END: u64 = 1;
const PAYLOAD: u64 = 2;

impl rennet::Codec for Element {
    fn encode<W: Write>(&self, w: &mut W) -> io::Result<()> {
        match *self {
            Element::Tick { held, flags } => {
                assert_eq!(held & !BUTTONS, 0, "a tick carries the ten buttons only: {held:#06x}");
                let value = buttons_to_wire(held) as u64 | (flags as u64) << 10;
                write_uvarint(w, value << 1)
            }
            Element::Payload(chunk) => {
                write_uvarint(w, (PAYLOAD + chunk.len as u64 - 1) << 1 | 1)?;
                w.write_all(chunk.bytes())
            }
            Element::RoundEnd => write_uvarint(w, ROUND_END << 1 | 1),
            Element::MatchEnd => write_uvarint(w, MATCH_END << 1 | 1),
        }
    }

    fn decode<R: Read>(r: &mut R) -> io::Result<Option<Element>> {
        // The run's end: no byte left.
        let mut first = [0u8; 1];
        if r.read(&mut first)? == 0 {
            return Ok(None);
        }
        // The head's first byte is read; the rest of a LEB128 number is a
        // LEB128 number of its own (the bits above the first seven).
        let mut head = (first[0] & 0x7F) as u64;
        if first[0] & 0x80 != 0 {
            let rest = read_uvarint(r)?;
            if rest >= 1 << 57 {
                return Err(invalid("element head too long"));
            }
            head |= rest << 7;
        }
        let kind = head >> 1;
        let element = if head & 1 == 0 {
            let flags = u8::try_from(kind >> 10).map_err(|_| invalid("tick flags out of range"))?;
            Element::Tick { held: buttons_from_wire(kind as u16 & BUTTONS), flags }
        } else {
            match kind {
                ROUND_END => Element::RoundEnd,
                MATCH_END => Element::MatchEnd,
                k if k >= PAYLOAD && k < PAYLOAD + CHUNK as u64 => {
                    let len = (k - PAYLOAD + 1) as usize;
                    let mut bytes = [0u8; CHUNK];
                    r.read_exact(&mut bytes[..len])?;
                    Element::Payload(Chunk { len: len as u8, bytes })
                }
                k => return Err(invalid(&format!("unknown element kind {k}"))),
            }
        };
        Ok(Some(element))
    }
}

/// What rides on every frame besides the elements.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Meta {
    /// The sender's tick advantage as of its newest input (getgud's
    /// `local_tick_advantage`), for the receiver's clock sync.
    pub tick_advantage: i16,
}

impl rennet::Codec for Meta {
    fn encode<W: Write>(&self, w: &mut W) -> io::Result<()> {
        write_svarint(w, self.tick_advantage as i64)
    }

    fn decode<R: Read>(r: &mut R) -> io::Result<Option<Meta>> {
        // Every frame has one: a missing meta is a truncated frame.
        let tick_advantage = i16::try_from(read_svarint(r)?).map_err(|_| invalid("tick advantage out of range"))?;
        Ok(Some(Meta { tick_advantage }))
    }
}

/// A game's per-tick input on the wire: the buttons held, a few flags for
/// the tick's events, and the bytes of the rare event that carries data.
/// `decode` must give back the input `encode` wrote.
pub trait WireInput: Sized {
    /// Write the input's payload, if it has one, and return its buttons
    /// and flags.
    fn encode(&self, payload: &mut Vec<u8>) -> (u16, u8);

    /// The input from its buttons, flags and payload.
    fn decode(held: u16, flags: u8, payload: &[u8]) -> io::Result<Self>;
}

/// Buttons alone (the stand-in battle's input).
impl WireInput for u16 {
    fn encode(&self, _: &mut Vec<u8>) -> (u16, u8) {
        (*self & BUTTONS, 0)
    }

    fn decode(held: u16, flags: u8, payload: &[u8]) -> io::Result<u16> {
        if flags != 0 || !payload.is_empty() {
            return Err(invalid("buttons carry no flags or payload"));
        }
        Ok(held)
    }
}

pub(crate) fn invalid(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rennet::Codec;

    fn bytes(e: Element) -> Vec<u8> {
        let mut out = Vec::new();
        e.encode(&mut out).unwrap();
        out
    }

    fn roundtrip(e: Element) {
        let b = bytes(e);
        let mut r = &b[..];
        assert_eq!(Element::decode(&mut r).unwrap(), Some(e), "{b:02x?}");
        assert!(r.is_empty(), "{e:?} left {r:02x?}");
    }

    #[test]
    fn the_wire_order_is_a_permutation() {
        for held in 0..=BUTTONS {
            assert_eq!(buttons_from_wire(buttons_to_wire(held)), held);
        }
        assert_eq!(buttons_to_wire(keys::A | keys::B | keys::RIGHT | keys::LEFT | keys::UP | keys::DOWN), 0x3F);
    }

    /// Ticks with A, B and the directions are a byte, other buttons two,
    /// flags up to three; markers one; a payload its length and one.
    #[test]
    fn element_sizes() {
        assert_eq!(bytes(Element::Tick { held: 0, flags: 0 }), [0x00]);
        assert_eq!(bytes(Element::Tick { held: keys::A, flags: 0 }), [0x02]);
        assert_eq!(bytes(Element::Tick { held: keys::DOWN | keys::LEFT | keys::B, flags: 0 }).len(), 1);
        assert_eq!(bytes(Element::Tick { held: keys::L, flags: 0 }).len(), 2);
        assert_eq!(bytes(Element::Tick { held: BUTTONS, flags: 1 }).len(), 2);
        assert_eq!(bytes(Element::Tick { held: BUTTONS, flags: 0xFF }).len(), 3);
        assert_eq!(bytes(Element::RoundEnd), [0x01]);
        assert_eq!(bytes(Element::MatchEnd), [0x03]);
        assert_eq!(bytes(Element::Payload(Chunk::new(&[7; 5]))).len(), 6);
        assert_eq!(bytes(Element::Payload(Chunk::new(&[7; CHUNK]))).len(), CHUNK + 1);
    }

    #[test]
    fn elements_roundtrip() {
        for held in [0, keys::A, keys::START | keys::L, BUTTONS] {
            for flags in [0, 1, 0x7F, 0xFF] {
                roundtrip(Element::Tick { held, flags });
            }
        }
        roundtrip(Element::RoundEnd);
        roundtrip(Element::MatchEnd);
        for n in [1, 2, 31, CHUNK] {
            roundtrip(Element::Payload(Chunk::new(&(0..n as u8).collect::<Vec<_>>())));
        }
    }

    /// A frame: base, ack, meta and the run, byte for byte.
    #[test]
    fn frame_bytes() {
        let f = Frame::new(
            300,
            298,
            Meta { tick_advantage: -2 },
            vec![Element::Tick { held: keys::A, flags: 0 }, Element::RoundEnd, Element::Tick { held: 0, flags: 0 }],
        );
        let b = f.to_vec();
        // base 300 (two bytes), ack -2 from it, advantage -2, three one-byte elements.
        assert_eq!(b, [0xAC, 0x02, 0x03, 0x03, 0x02, 0x01, 0x00]);
        assert_eq!(Frame::decode(&mut &b[..]).unwrap(), f);
    }

    #[test]
    fn malformed_elements_are_errors() {
        // An unknown kind, a payload cut short, a head cut short.
        for b in [&[0x05 + 2 * CHUNK as u8][..], &[0x07, 0xAA], &[0x80]] {
            assert!(Element::decode(&mut &b[..]).is_err(), "{b:02x?}");
        }
        // Flags past a byte.
        let mut b = Vec::new();
        write_uvarint(&mut b, (0x100 << 10) << 1).unwrap();
        assert!(Element::decode(&mut &b[..]).is_err());
    }
}
