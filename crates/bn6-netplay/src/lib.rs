//! Rollback netplay, GGPO-style, for the battle engine.
//!
//! Each peer runs the whole simulation. It knows its own player's input
//! at once, and the other player's a few frames late (network latency).
//! Rather than wait, it predicts the missing input (the last one that
//! arrived, repeated), simulates ahead, and saves a snapshot every frame.
//! When the real input arrives and differs from the prediction, it
//! restores the snapshot from before the first wrong frame and simulates
//! the frames again with what it now knows. A frame is *confirmed* once
//! both players' inputs for it are known and it was simulated with them;
//! both peers' confirmed states must then be identical, which the state
//! digest checks.
//!
//! The library part is generic over [`Game`]:
//!
//! - [`Peer`]: one peer's rollback session (input queues, prediction,
//!   snapshots, rollback and re-simulation, confirmation);
//! - [`network`]: a simulated link with latency and jitter;
//! - [`sim`]: two peers over simulated links, checking their confirmed
//!   digests against each other and against a plain lockstep run.
//!
//! [`bn6`] adapts the battle engine: [`bn6_battle::Battle`] with the
//! engine's per-tick input record, and a battle with a stand-in custom
//! screen for synthetic matches. See docs/design/rollback.md.

pub mod bn6;
pub mod network;
mod peer;
pub mod rng;
pub mod sim;
pub mod standin;

pub use peer::{Observer, Peer, PeerStats};

/// A deterministic two-player game that rollback can drive: the next state
/// is a function of the current state and both players' inputs only.
///
/// Snapshots are clones: `load` restores one with `clone_from`.
pub trait Game: Clone {
    /// One player's input for one frame.
    type Input: Clone + PartialEq + std::fmt::Debug;

    /// Simulate one frame.
    fn advance(&mut self, inputs: &[Self::Input; 2]);

    /// A digest of the simulation state, equal on peers that are in sync.
    fn digest(&self) -> u64;

    /// Nothing more happens: the match is over.
    fn is_over(&self) -> bool;

    /// A player's input before any of theirs is known.
    fn blank_input() -> Self::Input;

    /// The input to assume for a frame whose input hasn't arrived, from
    /// the player's latest known input.
    fn predict(last: &Self::Input) -> Self::Input {
        last.clone()
    }
}
