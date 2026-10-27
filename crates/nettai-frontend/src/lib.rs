//! nettai's frontend as a library: it plays battles for a host app to
//! show. It has no window, no audio device and no command line in it, and
//! nothing in it prints or exits: the host owns those (nettai is the app
//! over it, nettai-tools the command line).
//!
//! What it gives a host:
//!
//! - **a game, loaded** ([`game`]): the game's graphics found, the game's content,
//!   its graphics and strings in a language, the text's font, its sound,
//!   in one call or a part at a time with how far it is
//!   ([`game::Loading`]);
//! - **a match made ready** ([`preload`]): what its first tick would
//!   otherwise wait for (the content's runtime on the thread, the first
//!   battle and picture), a step at a time, by side;
//! - **a player** ([`Player`]) over **a driver** ([`driver`]: live play of
//!   a set from the buttons; [`netplay`]: another player over the
//!   network): the host gives it the GBA buttons held and the time that
//!   passed, or asks for a tick, and takes the picture and the sound as
//!   samples. The library draws no text of its own over the picture: where
//!   playback is, why it stopped, a result and a connection's figures are
//!   values the host shows as it likes. Under it, the [`session`];
//! - **a party** ([`party`]): a side's party as a host shows it beside
//!   the battle (EXE5 DS's bottom-screen panel: the members, their HP,
//!   the switches made, which member each of the panel's two buttons
//!   names), read from the battle each frame;
//! - **a replay's viewer** ([`viewer::Viewer`]): a replay played back
//!   with a transport, which seeks to any tick, steps a frame either way,
//!   goes a round back or on, plays at four speeds and shows either
//!   console;
//! - the drawing, which is nettai-render's ([`Renderer`], [`Frame`]).
//!
//! docs/frontend.md §7, and `examples/embed.rs`: a host with no window.

pub mod connection;
pub mod driver;
pub mod game;
pub mod lobby;
pub mod netplay;
pub mod party;
pub mod player;
pub mod preload;
pub mod replay;
pub mod session;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
pub mod viewer;

pub use nettai_render::{Frame, Renderer};
pub use player::Player;
pub use session::Session;
