//! nettai's frontend as a library: it plays battles for a host app to
//! show. It has no window, no audio device and no command line in it, and
//! nothing in it prints or exits: the host owns those (nettai is the app
//! over it, nettai-tools the command line).
//!
//! What it gives a host:
//!
//! - **a game, loaded** ([`game`]): the packs found, the game's content,
//!   its graphics and strings in a language, the text's font, its sound;
//! - **a player** ([`Player`]) over **a driver** ([`driver`]: live play of
//!   a set from the buttons; [`netplay`]: another player over the
//!   network): the host gives it the GBA buttons held and the time that
//!   passed, or asks for a tick, and takes the picture and the sound as
//!   samples. The library draws no text of its own over the picture: where
//!   playback is, why it stopped, a result and a connection's figures are
//!   values the host shows as it likes. Under it, the [`session`];
//! - the drawing, which is nettai-render's ([`Renderer`], [`Frame`]).
//!
//! docs/frontend.md §7, and `examples/embed.rs`: a host with no window.

pub mod driver;
pub mod game;
pub mod lobby;
pub mod netplay;
pub mod player;
pub mod replay;
pub mod session;

pub use nettai_render::{Frame, Renderer};
pub use player::Player;
pub use session::Session;
