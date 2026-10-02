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
//!   (its player's side, snapshots, prediction), generic over a [`Game`],
//!   and the [`Observer`] that sees every tick it simulates;
//! - [`battle`]: [`nettai_battle::Battle`] as a game, on the engine's per-tick
//!   input record, and the sound cue feed;
//! - [`standin`]: a battle stepped on the buttons alone, for synthetic
//!   matches;
//! - [`network`]: a simulated ordered link with latency and jitter;
//! - [`sim`]: two sessions over simulated links with clock sync, checking
//!   their settled digests against each other and against a plain
//!   lockstep run.
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
pub use world::{BattleState, BattleWorld, Game, Observer};
