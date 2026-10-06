//! The signaling server's protocol: how two players meet in a room and
//! trade what their connections need (docs/design/rollback.md §4.8). The
//! server is `signaling/` (a Cloudflare Worker; [`crate::testing`] has one
//! in process); this module is what the clients say and hear, as JSON
//! text messages on a WebSocket.
//!
//! A client opens `<server>/rooms/<CODE>?peer=<ID>`: CODE is the room's
//! (1 to 64 of `A-Z a-z 0-9 _ -`, [`valid_code`]), ID the client's own,
//! random, the same on every reconnection. The room has two places: the
//! first ID to come is its host, the second its joiner; an ID that comes
//! again gets its place back (that is how a reconnecting player keeps its
//! side), and a third is refused. The server says:
//!
//! - `{"type":"welcome","role":"host"|"join","peer":BOOL}`: this client's
//!   place, and whether the other is there;
//! - `{"type":"peer","present":BOOL}`: the other came, or its socket went;
//! - `{"type":"signal","data":...}`: what the other sent;
//! - `{"type":"error","message":"..."}`: refused (the room is full, a bad
//!   code), and the socket closes.
//!
//! A client says `{"type":"signal","data":...}` (to the other, if it is
//! there; dropped if not) and `{"type":"leave"}` (it gives its place up).
//! A text message `ping` is answered `pong`, the keepalive. The server
//! reads none of the data, which is [`Signal`]: the offer, the answer,
//! trickled candidates and the host's call to dial again, each of a
//! generation (each new connection is one more), so that what a dropped
//! one still had on its way is told from the next one's.

use serde::{Deserialize, Serialize};

use crate::{Candidate, Role};

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
    Welcome { role: Role, peer: bool },
    Peer { present: bool },
    Signal { data: Signal },
    Error { message: String },
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

/// Whether `code` names a room: 1 to 64 letters, digits, `_` and `-`.
pub fn valid_code(code: &str) -> bool {
    (1..=64).contains(&code.len()) && code.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// The WebSocket URL of room `code` on the server at `server` (`ws://`,
/// `wss://`, or `http(s)://` for the same), for the client `peer`.
pub fn room_url(server: &str, code: &str, peer: &str) -> String {
    let server = server.trim_end_matches('/');
    let server = match server.split_once("://") {
        Some(("http", rest)) => format!("ws://{rest}"),
        Some(("https", rest)) => format!("wss://{rest}"),
        Some(_) => server.to_string(),
        None => format!("wss://{server}"),
    };
    format!("{server}/rooms/{code}?peer={peer}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The messages are the JSON the Worker reads and writes.
    #[test]
    fn messages_are_the_workers_json() {
        let said = |m: &FromServer| serde_json::to_string(m).unwrap();
        assert_eq!(said(&FromServer::Welcome { role: Role::Join, peer: true }), r#"{"type":"welcome","role":"join","peer":true}"#);
        assert_eq!(said(&FromServer::Peer { present: false }), r#"{"type":"peer","present":false}"#);
        let candidate = Signal::Candidate { generation: 2, candidate: Candidate { candidate: "candidate:1 1 udp 1 192.0.2.1 9 typ host".into(), mid: Some("0".into()), mline: Some(0) } };
        let text = serde_json::to_string(&ToServer::Signal { data: candidate.clone() }).unwrap();
        assert_eq!(text, r#"{"type":"signal","data":{"kind":"candidate","generation":2,"candidate":"candidate:1 1 udp 1 192.0.2.1 9 typ host","mid":"0","mline":0}}"#);
        assert_eq!(serde_json::from_str::<ToServer>(&text).unwrap(), ToServer::Signal { data: candidate });
        assert_eq!(serde_json::to_string(&ToServer::Leave).unwrap(), r#"{"type":"leave"}"#);
        let error: FromServer = serde_json::from_str(r#"{"type":"error","message":"the room is full"}"#).unwrap();
        assert_eq!(error, FromServer::Error { message: "the room is full".into() });
    }

    #[test]
    fn rooms_by_code() {
        assert!(valid_code("abc-123_X") && !valid_code("") && !valid_code("a b") && !valid_code(&"a".repeat(65)));
        assert_eq!(room_url("https://signal.example/", "abc", "p1"), "wss://signal.example/rooms/abc?peer=p1");
        assert_eq!(room_url("ws://127.0.0.1:8787", "abc", "p1"), "ws://127.0.0.1:8787/rooms/abc?peer=p1");
        assert_eq!(room_url("signal.example", "abc", "p1"), "wss://signal.example/rooms/abc?peer=p1");
    }
}
