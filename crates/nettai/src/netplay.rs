//! Netplay's seam: the link to the other player, and the match agreed over
//! it.
//!
//! The link is nettai-rtc's (a room through the signaling server by its
//! code, or a direct connection), which this build doesn't have yet:
//! [`Link`] is its shape (its `role`, `status`, `send` and `recv`), so its
//! type drops in as this trait's implementation, or in its place. What runs
//! over a link is the library's: the lobby and the handshake
//! (`nettai_frontend::lobby`, which does no IO: [`Agreeing`] carries its
//! datagrams), then the match, on [`Framed`], the link as the library's
//! `Channel`, its frames told from the lobby's by their first byte as
//! nettai-demo's UDP connection tells them.
//!
//! The lobby screen drives an [`Agreeing`]: the player's game and side
//! proposed, their readiness; the other's, shown; once both are ready on
//! the same settings the match is agreed, and the battle is a `NetPlayer`
//! over the link.

// (Nothing here runs until nettai-rtc's link does; the tests run it over
// links in memory.)
#![allow(dead_code)]

use nettai_battle::Content;
use nettai_frontend::lobby::{Agreement, Kind, Lobby, Role, Settings, Status};
use nettai_frontend::netplay::{Channel, NetOptions, NetPlayer};
use nettai_match::Side;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How a link stands (nettai-rtc's `status()`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkStatus {
    /// In the room, nobody else there yet.
    Waiting,
    Connecting,
    Open,
    /// Down since then; frames sent meanwhile are dropped.
    Reconnecting { since: Instant },
}

/// A link to the other player: a datagram channel that never waits, and
/// what it knows of the room. nettai-rtc's `Link` (`Link::room(signal_url,
/// code, config)`; natively also `Link::direct_host(port, config)` and
/// `Link::direct_join(addr, config)`) has this shape.
pub trait Link {
    /// The side this player plays: known at once on a direct link, from
    /// the server's welcome in a room.
    fn role(&self) -> Option<Role>;
    fn status(&self) -> LinkStatus;
    /// Send a datagram; one that can't go now (the link is down) is
    /// dropped, as the network may drop it.
    fn send(&mut self, datagram: &[u8]) -> Result<(), String>;
    /// The next datagram that came, if one has.
    fn recv(&mut self) -> Result<Option<&[u8]>, String>;
    /// How long the link has been down, if it is (the battle shows
    /// "reconnecting").
    fn down_for(&self) -> Option<Duration> {
        match self.status() {
            LinkStatus::Reconnecting { since } => Some(since.elapsed()),
            _ => None,
        }
    }
}

/// How the agreeing stands after a poll.
pub enum Progress<L: Link> {
    /// The link isn't open, or the players haven't agreed.
    Pending,
    /// The match is agreed: the link and what both play.
    Agreed(Box<Agreed<L>>),
    /// No match, and why (the timeout, a refusal, the link's error).
    Failed(String),
}

pub struct Agreed<L: Link> {
    pub link: L,
    pub agreement: Agreement,
}

impl<L: Link + 'static> Agreed<L> {
    /// The agreed match's driver: the battle with the other player over
    /// the link.
    pub fn player(self, options: NetOptions) -> NetPlayer<Framed<L>> {
        let Agreement { set, side, answer, .. } = self.agreement;
        NetPlayer::new(Framed { link: self.link, answer, frame: Vec::new() }, side, set, options)
    }
}

/// The lobby and the handshake over a link: what the player proposes (the
/// game and its rounds, and their side) and whether they are ready; the
/// other player's, as their datagrams tell.
pub struct Agreeing<L: Link> {
    link: Option<L>,
    content: Arc<Content>,
    settings: Settings,
    side: Side,
    ready: bool,
    /// Made once the link knows this player's role.
    lobby: Option<Lobby>,
    timeout: Duration,
}

impl<L: Link> Agreeing<L> {
    pub fn new(link: L, content: &Arc<Content>, settings: Settings, side: Side, timeout: Duration) -> Agreeing<L> {
        Agreeing { link: Some(link), content: content.clone(), settings, side, ready: false, lobby: None, timeout }
    }

    /// Propose other settings (another game's rounds): both players'
    /// readiness is cleared.
    pub fn propose(&mut self, settings: Settings) {
        self.settings = settings.clone();
        if let Some(l) = &mut self.lobby {
            l.propose(settings);
        }
    }

    pub fn set_ready(&mut self, ready: bool) {
        self.ready = ready;
        if let Some(l) = &mut self.lobby {
            l.set_ready(ready);
        }
    }

    pub fn ready(&self) -> bool {
        self.ready
    }

    /// Whether the other player has said they are ready.
    pub fn their_ready(&self) -> bool {
        self.lobby.as_ref().is_some_and(Lobby::their_ready)
    }

    /// Whether the other player is there (their lobby has spoken).
    pub fn theirs(&self) -> bool {
        self.lobby.as_ref().is_some_and(|l| l.theirs().is_some())
    }

    pub fn status(&self) -> Option<LinkStatus> {
        self.link.as_ref().map(Link::status)
    }

    /// Step at `now`, without waiting: the datagrams that came to the
    /// lobby, its own out over the link.
    pub fn poll(&mut self, now: Instant) -> Progress<L> {
        let Some(mut link) = self.link.take() else { return Progress::Failed("the agreeing ended".into()) };
        match self.step(&mut link, now) {
            Ok(None) => {
                self.link = Some(link);
                Progress::Pending
            }
            Ok(Some(agreement)) => Progress::Agreed(Box::new(Agreed { link, agreement })),
            Err(why) => {
                if let Some(l) = &mut self.lobby {
                    for d in l.outgoing() {
                        let _ = link.send(&d);
                    }
                }
                Progress::Failed(why)
            }
        }
    }

    fn step(&mut self, link: &mut L, now: Instant) -> Result<Option<Agreement>, String> {
        if self.lobby.is_none() {
            // (The lobby starts once the role is known.)
            let Some(role) = link.role() else { return Ok(None) };
            let mut lobby = Lobby::new(role, &self.content, self.settings.clone(), self.side.clone(), nettai_frontend::lobby::entropy(), self.timeout);
            lobby.set_ready(self.ready);
            self.lobby = Some(lobby);
        }
        let lobby = self.lobby.as_mut().expect("made above");
        while let Some(d) = link.recv()? {
            if d.first().is_some_and(|&k| k != Kind::Frame as u8) {
                lobby.receive(now, d);
            }
        }
        let done = match lobby.poll(now) {
            Status::Pending => None,
            Status::Agreed(a) => Some(Ok(a.clone())),
            Status::Failed(why) => Some(Err(why.to_string())),
        };
        for d in lobby.outgoing() {
            link.send(&d)?;
        }
        done.transpose()
    }
}

/// A link as the library's `Channel` for the match: its frames (the
/// lobby's datagrams are told apart by their first byte); a Hello or a
/// Reveal that comes again is answered (the other side hasn't this one's
/// yet), a refusal ends the match.
pub struct Framed<L: Link> {
    link: L,
    answer: Vec<u8>,
    frame: Vec<u8>,
}

impl<L: Link> Framed<L> {
    pub fn link(&self) -> &L {
        &self.link
    }
}

impl<L: Link> Channel for Framed<L> {
    fn send(&mut self, frame: &[u8]) -> Result<(), String> {
        let mut out = Vec::with_capacity(frame.len() + 1);
        out.push(Kind::Frame as u8);
        out.extend_from_slice(frame);
        self.link.send(&out)
    }

    fn recv(&mut self) -> Result<Option<&[u8]>, String> {
        loop {
            let Some(d) = self.link.recv()? else { return Ok(None) };
            match d.split_first().map(|(&k, body)| (Kind::of(k), body)) {
                Some((Some(Kind::Frame), body)) => {
                    // (Kept here: the link's datagram is its own until the
                    // next call.)
                    self.frame.clear();
                    self.frame.extend_from_slice(body);
                    return Ok(Some(&self.frame));
                }
                Some((Some(Kind::Hello | Kind::Reveal), _)) => self.link.send(&self.answer)?,
                Some((Some(Kind::Refuse), body)) => return Err(format!("the other player refused: {}", String::from_utf8_lossy(body))),
                _ => {}
            }
        }
    }
}

/// A room's code: six letters and digits, none that read as another (no
/// 0/O, 1/I/L).
pub fn room_code(seed: u64) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
    let mut x = seed | 1;
    (0..6)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            ALPHABET[(x % ALPHABET.len() as u64) as usize] as char
        })
        .collect()
}
