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
//! order, with a lost datagram's inputs recovered from the next one. The
//! channel is the host's: this crate has no socket and no handshake (the
//! app's are nettai-rtc and nettai's `netplay`).
//!
//! This crate provides what getgud and rennet leave to the game:
//!
//! - [`world`]: [`BattleWorld`], getgud's `World` for one peer's battle
//!   (its player's side, snapshots, prediction, a tick that panics stopping
//!   the battle), generic over a [`Game`], and the [`Observer`] that sees
//!   every tick it simulates, every rewind and every tick that settles;
//! - [`battle`]: [`nettai_battle::Battle`] as a game, on the engine's per-tick
//!   input record, and the sound cue feed;
//! - [`standin`]: a battle stepped on the buttons alone, for synthetic
//!   matches;
//! - [`protocol`]: nettai's rennet protocol: the elements a player's stream
//!   carries (ticks, round and match markers), the per-frame
//!   meta (the tick advantage), their byte-minimal codecs, the horizon;
//! - [`wire`]: byte codecs for the engine types that travel (a recorded
//!   custom-screen result, a player's folder and Crosses);
//! - [`link`]: one peer's end of the input exchange on rennet's streams;
//! - [`peer`]: a peer of a match, session and link, and what a host does
//!   each frame (clock sync, the stall guard, rounds);
//! - [`network`]: a simulated datagram network (latency, jitter, loss,
//!   duplication), seeded;
//! - [`sim`]: two peers over the simulated network with clock sync,
//!   checking their settled digests against each other and against a
//!   plain lockstep run;
//!
//! A session runs on any thread: a battle world, with its states, inputs
//! and sound feed, is `Send` (checked below at compile time).
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
pub mod wire;
pub mod world;

pub use getgud;
pub use getgud::Session;
pub use peer::{Peer, PeerConfig, Wait};
pub use rennet;
pub use world::{BattleState, BattleWorld, Game, Observer, step_game};

// A session can run on a network thread: its world (the battle, its side
// and its observer), the states it keeps and the inputs it queues are
// `Send`, and the states and inputs `Sync` too. So can a whole peer.
const _: () = {
    const fn send<T: Send>() {}
    const fn send_sync<T: Send + Sync>() {}
    send_sync::<BattleState>();
    send_sync::<battle::PlayerInput>();
    send_sync::<battle::CueFeed>();
    send::<BattleWorld<nettai_battle::Battle, battle::CueFeed>>();
    send::<Session<BattleWorld<nettai_battle::Battle, battle::CueFeed>>>();
    send::<Session<BattleWorld<standin::StandInBattle>>>();
    send::<Peer<nettai_battle::Battle, battle::CueFeed>>();
};
