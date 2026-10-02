//! Rollback netplay for the battle engine, on getgud (Tango's rollback
//! core).
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
//! This crate provides what getgud leaves to the game:
//!
//! - [`world`]: [`BattleWorld`], getgud's `World` for one peer's battle
//!   (its player's side, snapshots, prediction, a tick that panics stopping
//!   the battle), generic over a [`Game`], and the [`Observer`] that sees
//!   every tick it simulates, every rewind and every tick that settles;
//! - [`battle`]: [`nettai_battle::Battle`] as a game, on the engine's per-tick
//!   input record, and the sound cue feed;
//! - [`standin`]: a battle stepped on the buttons alone, for synthetic
//!   matches;
//! - [`network`]: a simulated ordered link with latency and jitter;
//! - [`sim`]: two sessions over simulated links with clock sync, checking
//!   their settled digests against each other and against a plain
//!   lockstep run.
//!
//! A session runs on any thread: a battle world, with its states, inputs
//! and sound feed, is `Send` (checked below at compile time).
//!
//! See docs/design/rollback.md.

pub mod battle;
pub mod network;
pub mod rng;
pub mod sim;
pub mod standin;
pub mod world;

pub use getgud;
pub use getgud::Session;
pub use world::{BattleState, BattleWorld, Game, Observer, step_game};

// A session can run on a network thread: its world (the battle, its side
// and its observer), the states it keeps and the inputs it queues are
// `Send`, and the states and inputs `Sync` too.
const _: () = {
    const fn send<T: Send>() {}
    const fn send_sync<T: Send + Sync>() {}
    send_sync::<BattleState>();
    send_sync::<battle::PlayerInput>();
    send_sync::<battle::CueFeed>();
    send::<BattleWorld<nettai_battle::Battle, battle::CueFeed>>();
    send::<Session<BattleWorld<nettai_battle::Battle, battle::CueFeed>>>();
    send::<Session<BattleWorld<standin::StandInBattle>>>();
};
