//! Netplay: the link to the other player (nettai-rtc's, a room of the
//! signaling server by its code), and the match agreed over it.
//!
//! What runs over the link is the library's: the lobby and the handshake
//! (`nettai_frontend::lobby`, which does no IO: [`Agreeing`] carries its
//! datagrams), then the match, on [`Framed`]: the link as the library's
//! `Channel`, its frames told from the lobby's by their first byte as
//! the retired nettai-demo's connection told them. [`Link`] is what both need of a
//! link (nettai-rtc's `Link` is one).
//!
//! The lobby screen drives an [`Agreeing`]: the player's game and side
//! proposed, their readiness; the other's, shown; once both are ready on
//! the same settings the match is agreed, and the battle is a `NetPlayer`
//! over the link.

use nettai_battle::Content;
use nettai_frontend::lobby::{Agreement, Kind, Lobby, Role, Settings, Status};
use nettai_frontend::netplay::{Channel, NetOptions, NetPlayer};
use nettai_match::Side;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// What netplay needs of a link to the other player: a datagram channel
/// that never waits, this side's role once known, and how it stands.
pub trait Link {
    /// The side this player plays: known at once on a direct link, from
    /// the server's welcome in a room.
    fn role(&self) -> Option<Role>;
    /// Send a datagram; one that can't go now (the link is down) is
    /// dropped, as the network may drop it. An error: the link gave up.
    fn send(&mut self, datagram: &[u8]) -> Result<(), String>;
    /// The next datagram that came, if one has.
    fn recv(&mut self) -> Result<Option<&[u8]>, String>;
    /// The channel is open: what is sent goes.
    fn is_open(&self) -> bool;
    /// How long the channel has been down, while it is being made again.
    fn down_for(&self) -> Option<Duration>;
    /// What keeps it from connecting, if something does (the signaling
    /// server can't be reached).
    fn problem(&self) -> Option<String>;
}

impl Link for nettai_rtc::Link {
    fn role(&self) -> Option<Role> {
        nettai_rtc::Link::role(self).map(|r| match r {
            nettai_rtc::Role::Host => Role::Host,
            nettai_rtc::Role::Join => Role::Join,
        })
    }

    fn send(&mut self, datagram: &[u8]) -> Result<(), String> {
        nettai_rtc::Link::send(self, datagram).map_err(|e| e.0)
    }

    fn recv(&mut self) -> Result<Option<&[u8]>, String> {
        nettai_rtc::Link::recv(self).map_err(|e| e.0)
    }

    fn is_open(&self) -> bool {
        nettai_rtc::Link::is_open(self)
    }

    fn down_for(&self) -> Option<Duration> {
        nettai_rtc::Link::down_for(self)
    }

    fn problem(&self) -> Option<String> {
        nettai_rtc::Link::problem(self)
    }
}

/// How the link stands, for the lobby screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// Reaching the signaling server (the role isn't known).
    Connecting,
    /// In the room; the channel to the other player isn't open.
    Waiting,
    Open,
    /// Down, being made again, for this long.
    Reconnecting(Duration),
}

/// How the agreeing stands after a poll.
pub enum Progress<L: Link> {
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

impl<L: Link> Agreed<L> {
    /// The agreed match's driver (the battle with the other player over
    /// the link), and what was agreed.
    pub fn player(self, options: NetOptions) -> (NetPlayer<Framed<L>>, Agreement) {
        let Agreed { link, agreement } = self;
        let framed = Framed { link, answer: agreement.answer.clone(), frame: Vec::new() };
        (NetPlayer::new(framed, agreement.side, agreement.set.clone(), options), agreement)
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
    /// This player's name, said in the lobby.
    name: String,
    /// The build this player brings, as the app labels it (its name, its
    /// version), said in the lobby.
    build: (String, String),
}

impl<L: Link> Agreeing<L> {
    /// Agreeing a match of `settings` on `content` over `link`, bringing
    /// `side`; it fails if nothing comes from the other player for
    /// `timeout` once the lobby is made.
    pub fn new(link: L, content: &Arc<Content>, settings: Settings, side: Side, timeout: Duration) -> Agreeing<L> {
        Agreeing { link: Some(link), content: content.clone(), settings, side, ready: false, lobby: None, timeout, name: String::new(), build: Default::default() }
    }

    /// Propose other settings and side (another game's, on its content):
    /// both players' readiness is cleared.
    pub fn propose(&mut self, content: &Arc<Content>, settings: Settings, side: Side) {
        self.ready = false;
        self.settings = settings.clone();
        self.side = side;
        if !Arc::ptr_eq(content, &self.content) {
            // (Another game: another lobby, on its content, made at the
            // next poll; the other player's own goes on hearing this one.)
            self.content = content.clone();
            self.lobby = None;
            return;
        }
        if let Some(l) = &mut self.lobby {
            l.propose(settings);
            // (The side too: the one revealed is the one brought now.)
            l.set_side(self.side.clone());
            l.set_ready(false);
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

    /// Say this player's name to the other.
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
        if let Some(l) = &mut self.lobby {
            l.set_name(name);
        }
    }

    /// Say the build this player brings (its name, its version) to the
    /// other.
    pub fn set_build(&mut self, name: &str, version: &str) {
        self.build = (name.to_string(), version.to_string());
        if let Some(l) = &mut self.lobby {
            l.set_build(name, version);
        }
    }

    /// The build the other player brings, as they said it (shown, not
    /// trusted).
    pub fn their_build(&self) -> Option<(&str, &str)> {
        self.lobby.as_ref().and_then(Lobby::their_build)
    }

    /// The other player's name, as they said it (shown, not trusted).
    pub fn their_name(&self) -> Option<&str> {
        self.lobby.as_ref().and_then(Lobby::their_name)
    }

    /// The navi the other player's side plays, by its name, as they said
    /// it (shown before the match).
    pub fn their_navi(&self) -> Option<&str> {
        self.lobby.as_ref().and_then(Lobby::their_navi)
    }

    /// The other player's settings, once they have said them (or why they
    /// don't read here).
    pub fn theirs(&self) -> Option<&Result<Settings, String>> {
        self.lobby.as_ref().and_then(Lobby::theirs)
    }

    /// How the link stands.
    pub fn standing(&self) -> Standing {
        let Some(link) = &self.link else { return Standing::Connecting };
        match (link.role(), link.is_open(), link.down_for()) {
            (_, _, Some(d)) => Standing::Reconnecting(d),
            (None, _, None) => Standing::Connecting,
            (Some(_), false, None) => Standing::Waiting,
            (Some(_), true, None) => Standing::Open,
        }
    }

    /// What keeps the link from connecting, if it knows.
    pub fn problem(&self) -> Option<String> {
        self.link.as_ref().and_then(Link::problem)
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
                // (What the lobby says last goes: a refusal.)
                for d in self.lobby.as_mut().map(Lobby::outgoing).unwrap_or_default() {
                    let _ = link.send(&d);
                }
                Progress::Failed(why)
            }
        }
    }

    fn step(&mut self, link: &mut L, now: Instant) -> Result<Option<Agreement>, String> {
        if self.lobby.is_none() {
            // (The lobby starts once the role is known; meanwhile the link
            // is kept up, which in a room is what brings the role.)
            let Some(role) = link.role() else {
                link.recv()?;
                return Ok(None);
            };
            let mut lobby = Lobby::new(role, &self.content, self.settings.clone(), self.side.clone(), nettai_frontend::lobby::entropy(), self.timeout);
            lobby.set_ready(self.ready);
            lobby.set_name(&self.name);
            lobby.set_build(&self.build.0, &self.build.1);
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

    fn down_for(&self) -> Option<Duration> {
        self.link.down_for()
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

/// The signaling server: `$NETTAI_SIGNAL` (`ws://127.0.0.1:8787` for
/// signaling/ run by `npx wrangler dev`), if it is set.
pub fn signal_server() -> Option<String> {
    std::env::var("NETTAI_SIGNAL").ok().filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_frontend::Session;

    /// Poll both once each.
    fn poll<L: Link>(both: &mut [Agreeing<L>; 2], out: &mut [Option<Progress<L>>; 2]) {
        for (i, a) in both.iter_mut().enumerate() {
            if out[i].is_none() {
                match a.poll(Instant::now()) {
                    Progress::Pending => {}
                    p => out[i] = Some(p),
                }
            }
        }
    }

    /// Two players in a room of a signaling server (in process), on WebRTC
    /// links: neither is agreed while one isn't ready, and each hears the
    /// other's readiness; once both are, both agree the same match, the
    /// room's first player hosting (side 0), and play it over the link.
    #[test]
    fn two_players_agree_a_match_in_a_room_and_play_it() {
        let content = nettai_match::testing::exe6_content();
        let server = nettai_rtc::testing::Server::start();
        let config = nettai_rtc::Config { ice_servers: Vec::new(), loopback: true, ..nettai_rtc::Config::default() };
        let m = nettai_match::pick::live(&content, "exe6", 9, None).unwrap();
        let settings = Settings::of_match(&m);
        // (The first in the room hosts: the host is let in first.)
        let mut host = nettai_rtc::Link::room(&server.url(), "lobby-test", config.clone()).unwrap();
        let start = Instant::now();
        while nettai_rtc::Link::role(&host).is_none() {
            assert!(start.elapsed() < Duration::from_secs(10), "not let in");
            host.poll().unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        let join = nettai_rtc::Link::room(&server.url(), "lobby-test", config).unwrap();
        let timeout = Duration::from_secs(20);
        let mut both = [
            Agreeing::new(host, &content, settings.clone(), m.sides[0].clone(), timeout),
            Agreeing::new(join, &content, settings, m.sides[1].clone(), timeout),
        ];
        both[0].set_ready(true);
        let mut out = [None, None];
        let start = Instant::now();
        while !both[1].their_ready() {
            assert!(start.elapsed() < Duration::from_secs(20), "the joiner never hears the host ready ({:?})", both[1].standing());
            poll(&mut both, &mut out);
            assert!(out.iter().all(Option::is_none), "agreed with one player not ready");
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!((both[0].standing(), both[1].standing()), (Standing::Open, Standing::Open));
        assert!(!both[0].their_ready());
        both[1].set_ready(true);
        let start = Instant::now();
        while out.iter().any(Option::is_none) {
            assert!(start.elapsed() < Duration::from_secs(20), "not agreed");
            poll(&mut both, &mut out);
            std::thread::sleep(Duration::from_millis(1));
        }
        let [Some(Progress::Agreed(a)), Some(Progress::Agreed(b))] = out else { panic!("not agreed") };
        let ((pa, aa), (pb, ab)) = (a.player(NetOptions::default()), b.player(NetOptions::default()));
        assert_eq!((aa.seed, aa.side, ab.side), (ab.seed, 0, 1));
        assert_eq!(aa.m, ab.m);
        // The match, over the link: a second of frames each, nothing stops.
        let mut sessions = [Session::new(Box::new(pa)), Session::new(Box::new(pb))];
        for _ in 0..60 {
            for s in &mut sessions {
                assert!(s.step(0), "{:?}", s.stopped);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn room_codes_read_alike_aloud() {
        let code = room_code(12345);
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
        assert!(!code.contains(['0', 'O', '1', 'I', 'L']));
        assert_ne!(room_code(1), room_code(2));
    }
}
