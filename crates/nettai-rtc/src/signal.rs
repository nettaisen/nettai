//! The signaling server's protocol: how two players meet in a room and
//! trade what their connections need (docs/design/rollback.md §4.8). The
//! server is `signaling/` (a Cloudflare Worker; [`crate::testing`] has one
//! in process); this module is what the clients say and hear, as JSON
//! text messages on a WebSocket. This is version [`VERSION`] of it.
//!
//! **A room's code is the server's to pick**, never a client's:
//! [`CODE_LENGTH`] characters of [`ALPHABET`], capitals and digits none of
//! which reads as another. A player gives it to the other (an invite), who
//! types it: [`normalize_code`] takes what was typed.
//!
//! - To make a room a client opens `<server>/rooms?peer=<ID>&v=1`
//!   ([`new_room_url`]): ID is the client's own, random, the same on every
//!   reconnection. The server makes a room under a code no room has; the
//!   client is its host, and the welcome says the code.
//! - To join a room, or to come back to one after its socket dropped, a
//!   client opens `<server>/rooms/<CODE>?peer=<ID>&v=1` ([`room_url`]). A
//!   code no room has is refused: nothing is made of it.
//!
//! The room has two places: its maker is its host, the next ID to come its
//! joiner; an ID that comes again gets its place back (that is how a
//! reconnecting player keeps its side), and a third is refused. The server
//! says:
//!
//! - `{"type":"welcome","role":"host"|"join","peer":BOOL,"code":CODE}`:
//!   this client's place, whether the other is there, and the room's code;
//! - `{"type":"peer","present":BOOL}`: the other came, or its socket went;
//! - `{"type":"signal","data":...}`: what the other sent;
//! - `{"type":"error","reason":REASON,"message":"..."}`: refused, and the
//!   socket closes. The reason is one of [`Refusal`]'s; the message is for a
//!   log.
//!
//! A client says `{"type":"signal","data":...}` (to the other, if it is
//! there; dropped if not) and `{"type":"leave"}` (it gives its place up; a
//! room both have left is over, and its code names no room). A text message
//! `ping` is answered `pong`, the keepalive. The server reads none of the
//! data, which is [`Signal`]: the offer, the answer, trickled candidates and
//! the host's call to dial again, each of a generation (each new connection
//! is one more), so that what a dropped one still had on its way is told
//! from the next one's.
//!
//! A room is kept while a socket of it is open and ten minutes after the
//! last one goes; then it is forgotten, and its code with it.

use serde::{Deserialize, Serialize};

use crate::{Candidate, Role};

/// The protocol's version, which a client says as it connects (`v=`): a
/// server of another refuses it ([`Refusal::Version`]).
pub const VERSION: u32 = 1;

/// The characters of a room's code: capitals and digits, without `0` and
/// `O`, and without `1`, `I` and `L`, so that none reads as another on a
/// screen or said aloud.
pub const ALPHABET: &str = "ABCDEFGHJKMNPQRSTUVWXYZ23456789";

/// How many characters a room's code has.
pub const CODE_LENGTH: usize = 6;

/// What a client says to the server.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ToServer {
    Signal { data: Signal },
    Leave,
}

/// What the server says to a client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FromServer {
    Welcome {
        role: Role,
        peer: bool,
        code: String,
    },
    Peer {
        present: bool,
    },
    Signal {
        data: Signal,
    },
    Error {
        #[serde(default)]
        reason: Refusal,
        message: String,
    },
}

/// Why the server refuses a client.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Refusal {
    /// No room has that code (it never had one, or its room is over).
    NotFound,
    /// The room has two players already.
    Full,
    /// The client speaks another version of this protocol.
    Version,
    /// The request isn't one: no peer ID, or a bad one.
    BadRequest,
    /// The server found no free code for a new room.
    Busy,
    /// A reason this client doesn't know (a newer server's).
    #[default]
    #[serde(other)]
    Unknown,
}

/// What one client sends the other through the room.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Signal {
    /// The joiner's offer, for connection `generation`.
    Offer { generation: u32, sdp: String },
    /// The host's answer to it.
    Answer { generation: u32, sdp: String },
    /// A candidate of connection `generation`'s.
    Candidate {
        generation: u32,
        #[serde(flatten)]
        candidate: Candidate,
    },
    /// The host lost connection `generation`: the joiner dials again.
    Redial { generation: u32 },
}

/// The keepalive a client sends, and the answer.
pub const PING: &str = "ping";
pub const PONG: &str = "pong";

/// Whether `code` is a room's code as the server writes one:
/// [`CODE_LENGTH`] characters of [`ALPHABET`].
pub fn valid_code(code: &str) -> bool {
    code.len() == CODE_LENGTH && code.chars().all(|c| ALPHABET.contains(c))
}

/// A room's code from what a player typed or pasted: capitals for its
/// letters, and the spaces and hyphens a code shown in groups has left out.
/// None: no room has such a code (the wrong length, or a character codes
/// don't have).
pub fn normalize_code(typed: &str) -> Option<String> {
    let code: String = typed.chars().filter(|c| !c.is_whitespace() && *c != '-').map(|c| c.to_ascii_uppercase()).collect();
    valid_code(&code).then_some(code)
}

/// The server at `server` as a WebSocket URL's start (`ws://`, `wss://`,
/// or `http(s)://` for the same; no scheme: `wss://`).
fn websocket(server: &str) -> String {
    let server = server.trim_end_matches('/');
    match server.split_once("://") {
        Some(("http", rest)) => format!("ws://{rest}"),
        Some(("https", rest)) => format!("wss://{rest}"),
        Some(_) => server.to_string(),
        None => format!("wss://{server}"),
    }
}

/// The WebSocket URL that makes a new room on the server at `server`, for
/// the client `peer`.
pub fn new_room_url(server: &str, peer: &str) -> String {
    format!("{}/rooms?peer={peer}&v={VERSION}", websocket(server))
}

/// The WebSocket URL of the room `code` on the server at `server`, for the
/// client `peer`.
pub fn room_url(server: &str, code: &str, peer: &str) -> String {
    format!("{}/rooms/{code}?peer={peer}&v={VERSION}", websocket(server))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The messages are the JSON the Worker reads and writes.
    #[test]
    fn messages_are_the_workers_json() {
        let said = |m: &FromServer| serde_json::to_string(m).unwrap();
        assert_eq!(said(&FromServer::Welcome { role: Role::Join, peer: true, code: "ABC234".into() }), r#"{"type":"welcome","role":"join","peer":true,"code":"ABC234"}"#);
        assert_eq!(said(&FromServer::Peer { present: false }), r#"{"type":"peer","present":false}"#);
        let candidate = Signal::Candidate { generation: 2, candidate: Candidate { candidate: "candidate:1 1 udp 1 192.0.2.1 9 typ host".into(), mid: Some("0".into()), mline: Some(0) } };
        let text = serde_json::to_string(&ToServer::Signal { data: candidate.clone() }).unwrap();
        assert_eq!(text, r#"{"type":"signal","data":{"kind":"candidate","generation":2,"candidate":"candidate:1 1 udp 1 192.0.2.1 9 typ host","mid":"0","mline":0}}"#);
        assert_eq!(serde_json::from_str::<ToServer>(&text).unwrap(), ToServer::Signal { data: candidate });
        assert_eq!(serde_json::to_string(&ToServer::Leave).unwrap(), r#"{"type":"leave"}"#);
    }

    /// A refusal's reason is one of the protocol's; one this client doesn't
    /// know, or none (an older server's), reads as unknown.
    #[test]
    fn refusals_have_reasons() {
        let read = |text: &str| serde_json::from_str::<FromServer>(text).unwrap();
        for (reason, name) in [(Refusal::NotFound, "not-found"), (Refusal::Full, "full"), (Refusal::Version, "version"), (Refusal::BadRequest, "bad-request"), (Refusal::Busy, "busy")] {
            let text = format!(r#"{{"type":"error","reason":"{name}","message":"why"}}"#);
            assert_eq!(read(&text), FromServer::Error { reason, message: "why".into() });
            assert_eq!(serde_json::to_string(&FromServer::Error { reason, message: "why".into() }).unwrap(), text);
        }
        assert_eq!(read(r#"{"type":"error","reason":"something-new","message":"why"}"#), FromServer::Error { reason: Refusal::Unknown, message: "why".into() });
        assert_eq!(read(r#"{"type":"error","message":"the room is full"}"#), FromServer::Error { reason: Refusal::Unknown, message: "the room is full".into() });
    }

    #[test]
    fn rooms_by_code() {
        assert_eq!((ALPHABET.len(), CODE_LENGTH), (31, 6));
        assert!(!ALPHABET.contains(['0', 'O', '1', 'I', 'L']) && ALPHABET.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
        assert!(valid_code("ABC234") && !valid_code("") && !valid_code("ABC23") && !valid_code("ABC2345") && !valid_code("abc234"));
        // (None of the characters left out: a zero, a one, an I, an L, an O.)
        assert!(["ABC230", "ABC231", "ABCI34", "ABCL34", "ABCO34"].iter().all(|c| !valid_code(c)));
        assert_eq!(normalize_code(" abc-234\n").as_deref(), Some("ABC234"));
        assert_eq!(normalize_code("ABC 234").as_deref(), Some("ABC234"));
        assert_eq!((normalize_code("ABC-23"), normalize_code("ABC-230"), normalize_code("")), (None, None, None));
        assert_eq!(room_url("https://signal.example/", "ABC234", "p1"), "wss://signal.example/rooms/ABC234?peer=p1&v=1");
        assert_eq!(room_url("ws://127.0.0.1:8787", "ABC234", "p1"), "ws://127.0.0.1:8787/rooms/ABC234?peer=p1&v=1");
        assert_eq!(new_room_url("signal.example", "p1"), "wss://signal.example/rooms?peer=p1&v=1");
        assert_eq!(new_room_url("http://127.0.0.1:8787/", "p1"), "ws://127.0.0.1:8787/rooms?peer=p1&v=1");
    }
}
