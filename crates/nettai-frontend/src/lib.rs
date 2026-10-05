//! nettai's frontend as a library: it plays battles for a host app to
//! show. It has no window, no audio device and no command line in it, and
//! nothing in it prints or exits: the host owns those (nettai-demo is the
//! desktop program over it).
//!
//! What it gives a host:
//!
//! - **a game, loaded** ([`game`]): the packs found, the game's content,
//!   its graphics and strings in a language, the text's font, its sound;
//! - **a session** ([`session`]) over **a driver** ([`driver`]): live play
//!   of a set from the buttons, or another player over the network
//!   ([`netplay`]), a tick at a time from the GBA button mask;
//! - the drawing, which is nettai-render's ([`Renderer`], [`Frame`]), and
//!   the status lines' text ([`text`]).
//!
//! docs/frontend.md.

pub mod driver;
pub mod game;
pub mod netplay;
pub mod session;
pub mod text;

pub use nettai_render::{Frame, Renderer};
pub use session::{Session, TickHook};
