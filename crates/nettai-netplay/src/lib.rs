//! Rollback netplay for the battle engine, on getgud (Tango's rollback
//! core) and rennet (Tango's netplay transport).
//!
//! Each peer runs the whole battle in a getgud [`Session`]. It knows its
//! own player's input at once and the other player's a few ticks late.
//! Rather than wait, the session predicts the missing input (the last one
//! that arrived, repeated) and speculates ahead of the confirmed ticks,
//! saving a snapshot per speculated tick. When the real input arrives, the
//! ticks whose prediction held are promoted as they are; from the first
//! wrong one, the session restores the settled state and simulates again.
//! The *settled state* is built from confirmed input rows only (both
//! players' inputs known), which getgud hands back as they settle; both
//! peers' settled states must be identical, which the state digest checks.
//!
//! The players' inputs travel over a datagram channel that loses, reorders
//! and duplicates (UDP, or WebRTC's unreliable data channel); rennet turns
//! it into an ordered stream, each player's inputs delivered once, in
//! order, with a lost datagram's inputs recovered from the next one.
//!
//! This crate provides what getgud and rennet leave to the game:
//!
//! - [`world`]: [`BattleWorld`], getgud's `World` for one peer's battle
//!   (its player's side, snapshots, prediction), generic over a [`Game`],
//!   and the [`Observer`] that sees every tick it simulates;
//! - [`battle`]: [`nettai_battle::Battle`] as a game, on the engine's per-tick
//!   input record, and the sound cue feed;
//! - [`standin`]: a battle stepped on the buttons alone, for synthetic
//!   matches;
//! - [`protocol`]: nettai's rennet protocol: the elements a player's stream
//!   carries (ticks, payloads, round and match markers), the per-frame
//!   meta (the tick advantage), their byte-minimal codecs, the horizon;
//! - [`wire`]: byte codecs for the engine types that travel (a recorded
//!   custom-screen result, a player's folder and Crosses);
//! - [`link`]: one peer's end of the input exchange on rennet's streams;
//! - [`peer`]: a peer of a match, session and link, and what a host does
//!   each frame (clock sync, the stall guard, rounds);
//! - [`transport`]: the [`transport::Datagram`] channel a peer sends over,
//!   with UDP, and the handshake that starts a match;
//! - [`network`]: a simulated datagram network (latency, jitter, loss,
//!   duplication), seeded;
//! - [`sim`]: two peers over the simulated network with clock sync,
//!   checking their settled digests against each other and against a
//!   plain lockstep run.
//!
//! See docs/design/rollback.md.

pub mod battle;
pub mod link;
pub mod network;
pub mod peer;
pub mod protocol;
pub mod rng;
pub mod sim;
pub mod standin;
pub mod transport;
pub mod wire;
pub mod world;

pub use getgud;
pub use getgud::Session;
pub use rennet;
pub use peer::{Peer, PeerConfig, Wait};
pub use world::{BattleState, BattleWorld, Game, Observer};
