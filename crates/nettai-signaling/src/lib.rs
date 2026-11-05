//! The signaling server: where two players meet before their connection is
//! through, and again after it drops (docs/design/rollback.md §4.8). The
//! server is the Cloudflare Worker in `signaling/` (TypeScript, a Durable
//! Object a room); this crate is the rest of it, on both targets:
//!
//! - its protocol (the crate's root, `protocol`): the rooms, their codes and
//!   URLs, and the JSON messages a client says and hears on a WebSocket;
//! - [`Room`], a client of it in one room: its WebSocket (natively on a
//!   thread of its own, in a browser the browser's), made again whenever it
//!   closes, what it says queued meanwhile, a keepalive, and what the server
//!   says, read; polled, so nothing waits;
//! - with the `testing` feature, [`testing::Server`], a server of its own in
//!   process that speaks the protocol as the Worker does, and
//!   [`testing::Client`], a WebSocket of a test's own to either.
//!
//! What two clients say to each other through a room is theirs: the server
//! reads none of it, and nor does this crate (a [`Room`] carries any
//! serializable type). nettai-rtc's link says its connections' offers,
//! answers and candidates in it.

#![forbid(unsafe_code)]

mod platform;
mod protocol;
mod room;

#[cfg(all(feature = "testing", not(target_arch = "wasm32")))]
pub mod testing;

pub use protocol::*;
pub use room::{Error, ErrorKind, Room, RoomEvent};
