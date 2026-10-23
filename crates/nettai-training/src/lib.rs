//! The learned training AI's Rust side (docs/design/training-ai.md): what a
//! side sees, what it presses, and the episodes a trainer learns from, for
//! any game the engine runs. Nothing here names a game or a game's feature:
//! what it reads goes through the engine's per-viewer views and the
//! content's own descriptions (its rules' schemas and roles, its builds'
//! data), so a game added later is played by the same code.
//!
//! - [`observe`]: a side's view as fixed-size token sets ([`spec`]), names
//!   as hashed ids ([`names`]), mirrored to the viewer's side, tested for
//!   fairness (nothing it must not see changes it).
//! - [`act`]: the fight's 20 joint actions; the custom screen's pointer and
//!   the navigator that presses the way there.
//! - [`builds`]: random builds the rules accept.
//! - [`env`]: a round as an episode: the reaction delay, decisions every
//!   `k` ticks, who opens the custom screen, the practice options, rewards.
//! - [`pool`]: many envs on persistent worker threads, stepped as a batch
//!   into buffers the bridge (training/bridge) shows Python without a copy.
//! - [`bots`]: the scripted players; [`stats`]: §9's measures.
//!
//! With the `live` feature (the app's), the player for live play is here
//! too (a later step); without it nothing pulls in the frontend, the
//! renderer or the sound, so the bridge builds on a server.

pub mod act;
pub mod bots;
pub mod builds;
pub mod env;
pub mod fingerprint;
pub mod names;
pub mod observe;
pub mod pool;
pub mod spec;
pub mod stats;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
