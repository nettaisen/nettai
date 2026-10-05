//! nettai's frontend as a library: it plays battles for a host app to
//! show. It has no window, no audio device and no command line in it, and
//! nothing in it prints or exits: the host owns those (nettai-demo is the
//! desktop program over it).
//!
//! What it gives a host:
//!
//! - **a game, loaded** ([`game`]): the packs found, the game's content,
//!   its graphics and strings in a language, the text's font, its sound;
//! - **a player** ([`Player`]) over **a driver** ([`driver`]: live play of
//!   a set from the buttons; [`netplay`]: another player over the
//!   network): the host gives it the GBA buttons held and the time that
//!   passed, or asks for a tick, and takes the picture, the sound as
//!   samples and what there is to say. Under it, the [`session`];
//! - the drawing, which is nettai-render's ([`Renderer`], [`Frame`]), and
//!   the status lines' text ([`text`]).
//!
//! docs/frontend.md §7, and `examples/embed.rs`: a host with no window.

pub mod driver;
pub mod game;
pub mod netplay;
pub mod player;
pub mod session;
pub mod text;

pub use nettai_render::{Frame, Renderer};
pub use player::Player;
pub use session::Session;
