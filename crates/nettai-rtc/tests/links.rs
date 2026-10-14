#![cfg(not(target_arch = "wasm32"))]
//! Two links on this machine, directly and through a signaling server (the
//! in-process one; `NETTAI_TEST_SIGNAL=ws://127.0.0.1:8787` takes the
//! Worker under `wrangler dev` instead): one asks the server for a room and
//! the other joins it by the code the server gave; they connect, trade
//! datagrams, and come back after an outage, or give up after the timeout.
//! What goes wrong comes as a kind a player can be told.

use std::time::{Duration, Instant};

use nettai_rtc::signal::{FromServer, Refusal};
use nettai_rtc::testing::{Client, Server};
use nettai_rtc::{Config, Error, ErrorKind, Link, Role, signal};

/// Links that find each other on loopback, with no STUN server, quick to
/// notice a drop.
fn config() -> Config {
    Config {
        ice_servers: Vec::new(),
        loopback: true,
        silence: Duration::from_secs(1),
        dial_timeout: Duration::from_secs(5),
        reconnect_timeout: Duration::from_secs(20),
        ..Config::default()
    }
}

/// One side: its link, and what came.
struct Side {
    link: Link,
    got: Vec<u32>,
    sent: u32,
    error: Option<Error>,
}

impl Side {
    fn new(link: Link) -> Side {
        Side { link, got: Vec::new(), sent: 0, error: None }
    }
}

/// Each side sends a counter every 2 ms (dropped while it is down) and takes
/// what came, until `done` or `timeout` (which panics); the time it took.
fn run(sides: &mut [Side; 2], timeout: Duration, mut done: impl FnMut(&[Side; 2]) -> bool) -> Duration {
    let start = Instant::now();
    while !done(sides) {
        assert!(start.elapsed() < timeout, "not done in {timeout:?}: got {:?}, errors {:?}", sides.each_ref().map(|s| s.got.len()), sides.each_ref().map(|s| s.error.clone()));
        for s in sides.iter_mut().filter(|s| s.error.is_none()) {
            s.sent += 1;
            if let Err(e) = s.link.send(&s.sent.to_le_bytes()) {
                s.error = Some(e);
                continue;
            }
            loop {
                match s.link.recv() {
                    Ok(Some(d)) => s.got.push(u32::from_le_bytes(d.try_into().unwrap())),
                    Ok(None) => break,
                    Err(e) => {
                        s.error = Some(e);
                        break;
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    start.elapsed()
}

/// Both sides heard from the other since they had `before` each.
fn heard_more(before: [usize; 2]) -> impl FnMut(&[Side; 2]) -> bool {
    move |s| s[0].got.len() > before[0] + 20 && s[1].got.len() > before[1] + 20
}

fn direct(config: Config) -> [Side; 2] {
    let host = Link::host(0, config.clone()).unwrap();
    let port = host.port().unwrap();
    let join = Link::join(&format!("127.0.0.1:{port}"), config).unwrap();
    assert_eq!((host.role(), join.role()), (Some(Role::Host), Some(Role::Join)));
    [Side::new(host), Side::new(join)]
}

/// The server's URL: the in-process one, or the one the environment names.
fn server() -> (Option<Server>, String) {
    match std::env::var("NETTAI_TEST_SIGNAL") {
        Ok(url) => (None, url),
        Err(_) => {
            let s = Server::start();
            let url = s.url();
            (Some(s), url)
        }
    }
}

/// A new room of the server at `url`: its host's link, let in, and the code
/// the server gave it.
fn invite(url: &str, config: Config) -> (Link, String) {
    let mut host = Link::create_room(url, config);
    assert_eq!((host.code(), host.role()), (None, None), "the code and the place are the server's to say");
    let start = Instant::now();
    while host.role().is_none() {
        assert!(start.elapsed() < Duration::from_secs(10), "no welcome: {:?}", host.problem());
        host.poll().unwrap();
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(host.role(), Some(Role::Host));
    let code = host.code().expect("the server said the room's code").to_string();
    assert!(signal::valid_code(&code), "{code}");
    (host, code)
}

/// A room of the server at `url`: its host, and a joiner by the room's code.
fn room(url: &str, config: Config) -> ([Side; 2], String) {
    let (host, code) = invite(url, config.clone());
    let join = Link::join_room(url, &code, config).unwrap();
    assert_eq!(join.code(), Some(code.as_str()));
    ([Side::new(host), Side::new(join)], code)
}

/// The error `link` ends with, polled until it does.
fn refused(mut link: Link) -> Error {
    let start = Instant::now();
    loop {
        assert!(start.elapsed() < Duration::from_secs(15), "not refused");
        if let Err(e) = link.poll() {
            // (Every call after errs the same.)
            assert_eq!((link.send(b"x"), link.recv().map(|d| d.map(<[u8]>::to_vec))), (Err(e.clone()), Err(e.clone())));
            return e;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Both sides' links, connected, with datagrams going each way.
fn connected(sides: &mut [Side; 2]) {
    let took = run(sides, Duration::from_secs(20), heard_more([0, 0]));
    eprintln!("connected and trading in {took:?}");
    assert!(sides.iter().all(|s| s.link.is_open() && s.link.down_for().is_none()));
    assert_eq!(sides.each_ref().map(|s| s.link.generation()), [1, 1]);
}

#[test]
fn direct_links_trade_datagrams() {
    let mut sides = direct(config());
    connected(&mut sides);
}

/// Both sides through: each has left its rooms, with the same reconnect
/// code agreed (another than `before`), and the in-process server has no
/// room and no socket left (the Worker, which can't be asked, is given a
/// moment to read it). The code.
fn through(sides: &mut [Side; 2], server: Option<&Server>, before: Option<&str>) -> String {
    run(sides, Duration::from_secs(10), |s| {
        s.iter().all(|s| !s.link.in_a_room()) && s[0].link.reconnect_code().is_some() && s[0].link.reconnect_code() == s[1].link.reconnect_code()
    });
    let code = sides[0].link.reconnect_code().unwrap().to_string();
    assert!(signal::valid_reconnect_code(&code), "{code}");
    assert_ne!(Some(code.as_str()), before, "a new code for each connection");
    if let Some(server) = server {
        let start = Instant::now();
        while !(server.codes().is_empty() && server.reconnect_rooms().is_empty() && server.sockets() == 0) {
            assert!(start.elapsed() < Duration::from_secs(10), "the server keeps {:?}, {:?}, {} sockets", server.codes(), server.reconnect_rooms(), server.sockets());
            std::thread::sleep(Duration::from_millis(2));
        }
    } else {
        run_for(sides, Duration::from_secs(2));
    }
    code
}

/// The links connect through the invite room, and once they are through
/// both leave it: the server keeps nothing of it, and its code names no
/// room.
#[test]
fn room_links_trade_datagrams() {
    let (server, url) = server();
    let (mut sides, code) = room(&url, config());
    connected(&mut sides);
    assert_eq!(sides.each_ref().map(|s| s.link.role()), [Some(Role::Host), Some(Role::Join)]);
    through(&mut sides, server.as_ref(), None);
    assert_eq!(sides.each_ref().map(|s| s.link.code()), [Some(code.as_str()); 2], "the invite's code is still the link's to say");
    let e = refused(Link::join_room(&url, &code, config()).unwrap());
    assert_eq!((e.kind, e.detail.as_str()), (ErrorKind::NoSuchRoom, "the signaling server refused: no room has that code"));
    // The connection doesn't mind.
    let before = sides.each_ref().map(|s| s.got.len());
    run(&mut sides, Duration::from_secs(5), heard_more(before));
}

/// A third player is refused while the two are in the invite room (here
/// kept there: the host says no code, so the two are never through). The
/// code as a player types it: in small letters, in two groups.
#[test]
fn a_full_room_refuses_a_third() {
    let (_server, url) = server();
    let (mut sides, code) = room(&url, config());
    sides[0].link.hold_codes(true);
    connected(&mut sides);
    let typed = format!("{} {}", &code[..3], &code[3..]).to_lowercase();
    let e = refused(Link::join_room(&url, &typed, config()).unwrap());
    assert_eq!((e.kind, e.detail.as_str()), (ErrorKind::RoomFull, "the signaling server refused: the room is full"));
    assert!(sides.iter().all(|s| s.link.in_a_room() && s.link.reconnect_code().is_none()));
}

/// Before the first connection the invite waits, as long as its room is
/// kept: past the server's and the connection's timeouts, with nobody
/// come; then the other player comes, and they connect.
#[test]
fn before_the_first_connection_an_invite_waits() {
    let (server, url) = server();
    let quick = Config { server_timeout: Duration::from_secs(1), connect_timeout: Duration::from_secs(1), ..config() };
    let (mut host, code) = invite(&url, quick.clone());
    let until = Instant::now() + Duration::from_secs(3);
    while Instant::now() < until {
        host.poll().unwrap();
        assert!(!host.is_open() && host.down_for().is_none() && host.problem().is_none());
        std::thread::sleep(Duration::from_millis(2));
    }
    if let Some(server) = &server {
        assert_eq!(server.codes(), [code.as_str()]);
    }
    let join = Link::join_room(&url, &code, quick).unwrap();
    let mut sides = [Side::new(host), Side::new(join)];
    connected(&mut sides);
    through(&mut sides, server.as_ref(), None);
}

/// The server picks each room's code: six characters of the alphabet,
/// another for every room.
#[test]
fn the_server_assigns_each_rooms_code() {
    let (_server, url) = server();
    let invites: Vec<(Link, String)> = (0..4).map(|_| invite(&url, config())).collect();
    let mut codes: Vec<&str> = invites.iter().map(|(_, code)| code.as_str()).collect();
    assert!(codes.iter().all(|c| c.len() == signal::CODE_LENGTH && c.chars().all(|ch| signal::ALPHABET.contains(ch))), "{codes:?}");
    codes.sort();
    codes.dedup();
    assert_eq!(codes.len(), 4, "every room its own code");
}

/// Joining by a code no room has is refused as that, and makes no room: a
/// code that can't be one (refused before the server is asked), one that
/// never was a room's, and one whose room is over because both players left
/// it.
#[test]
fn a_code_no_room_has_is_refused() {
    let (server, url) = server();
    for bad in ["", "ABC23", "ABC2345", "ABC10O", "abc-de!"] {
        let e = Link::join_room(&url, bad, config()).err().unwrap_or_else(|| panic!("{bad:?} is no code"));
        assert_eq!(e.kind, ErrorKind::NoSuchRoom, "{bad:?}");
    }
    let (host, code) = invite(&url, config());
    // (A code of the alphabet that isn't this room's.)
    let other = if code == "AAAAAA" { "BBBBBB" } else { "AAAAAA" };
    let e = refused(Link::join_room(&url, other, config()).unwrap());
    assert_eq!((e.kind, e.detail.as_str()), (ErrorKind::NoSuchRoom, "the signaling server refused: no room has that code"));
    if let Some(server) = &server {
        assert_eq!(server.codes(), [code.as_str()], "no room was made of the code asked for");
    }
    // The host leaves (its link is dropped): the room is over, once the
    // server has read that (the in-process one says when; the Worker is
    // given a moment).
    drop(host);
    let start = Instant::now();
    match &server {
        Some(server) => {
            while !server.codes().is_empty() {
                assert!(start.elapsed() < Duration::from_secs(10), "the room is still there");
                std::thread::sleep(Duration::from_millis(2));
            }
        }
        None => std::thread::sleep(Duration::from_secs(2)),
    }
    let e = refused(Link::join_room(&url, &code, config()).unwrap());
    assert_eq!(e.kind, ErrorKind::NoSuchRoom);
}

/// A room whose time ran out is over: its code is refused, and a player
/// still in it, coming back to the server, is told the same.
#[test]
fn a_room_expires_with_its_code() {
    // (The in-process server's rooms end when told: the Worker's, ten
    // minutes after their last socket went.)
    let server = Server::start();
    let (mut host, code) = invite(&server.url(), config());
    server.forget(&code);
    let e = refused(Link::join_room(&server.url(), &code, config()).unwrap());
    assert_eq!(e.kind, ErrorKind::NoSuchRoom);
    let start = Instant::now();
    let e = loop {
        assert!(start.elapsed() < Duration::from_secs(10), "the host isn't told");
        if let Err(e) = host.poll() {
            break e;
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    assert_eq!(e.kind, ErrorKind::NoSuchRoom);
}

/// A server that isn't there, and one that never answers, are told from
/// everything else: the server can't be reached.
#[test]
fn a_server_that_cant_be_reached_says_so() {
    // (A port nothing listens on: bound, its number taken, and closed.)
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let url = format!("ws://127.0.0.1:{port}");
    let e = refused(Link::create_room(&url, config()));
    assert_eq!(e.kind, ErrorKind::ServerUnreachable, "{e}");
    let e = refused(Link::join_room(&url, "ABC234", config()).unwrap());
    assert_eq!(e.kind, ErrorKind::ServerUnreachable, "{e}");
    // One that takes the connection and says nothing.
    let silent = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("ws://{}", silent.local_addr().unwrap());
    let start = Instant::now();
    let e = refused(Link::create_room(&url, Config { server_timeout: Duration::from_secs(1), ..config() }));
    assert_eq!((e.kind, e.detail.as_str()), (ErrorKind::ServerUnreachable, "no answer from the signaling server in 1 seconds"));
    assert!(start.elapsed() >= Duration::from_secs(1));
}

/// A server that speaks another version of the protocol refuses, saying
/// that: the program (or the server) is out of date.
#[test]
fn a_server_of_another_version_says_so() {
    let server = Server::speaking(signal::VERSION + 1);
    let e = refused(Link::create_room(&server.url(), config()));
    assert_eq!((e.kind, e.detail.as_str()), (ErrorKind::ServerVersion, "the signaling server refused: this server speaks version 2 of the signaling protocol"));
    let e = refused(Link::join_room(&server.url(), "ABC234", config()).unwrap());
    assert_eq!(e.kind, ErrorKind::ServerVersion);
    assert!(server.codes().is_empty());
}

/// Direct connect's own errors: an address that names no machine, a port
/// another socket has, and a host that isn't there to connect to.
#[test]
fn direct_links_say_what_went_wrong() {
    let e = Link::join("not an address", config()).err().expect("no such address");
    assert_eq!(e.kind, ErrorKind::BadAddress, "{e}");
    let taken = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
    let e = Link::host(taken.local_addr().unwrap().port(), config()).err().expect("the port is taken");
    assert_eq!(e.kind, ErrorKind::PortUnavailable, "{e}");
    // (A port nothing answers on: the socket that has it reads nothing.)
    let quiet = Config { connect_timeout: Duration::from_secs(2), ..config() };
    let e = refused(Link::join(&format!("127.0.0.1:{}", taken.local_addr().unwrap().port()), quiet).unwrap());
    assert_eq!(e.kind, ErrorKind::CantConnect, "{e}");
}

/// What a player's program says last, as it leaves, is still taken: one
/// side sends a few datagrams and drops its link at once, and the other,
/// which sees the connection close in the same moment, takes them before it
/// says it is down. (A player who leaves a lobby or a match says so in
/// their last datagrams.) Directly and in a room.
#[test]
fn what_came_before_a_drop_is_still_taken() {
    let (_server, url) = server();
    for mut sides in [direct(config()), room(&url, config()).0] {
        connected(&mut sides);
        let [mut stays, mut leaves] = sides;
        while stays.link.recv().unwrap().is_some() {}
        for _ in 0..3 {
            leaves.link.send(b"leaving").unwrap();
        }
        drop(leaves);
        // (Time for the datagrams and the close to arrive, all before the
        // one that stays looks again.)
        std::thread::sleep(Duration::from_millis(300));
        let mut last = Vec::new();
        let start = Instant::now();
        while stays.link.down_for().is_none() {
            assert!(start.elapsed() < Duration::from_secs(10), "the connection closed");
            while let Some(d) = stays.link.recv().unwrap() {
                last.push(d.to_vec());
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        while let Some(d) = stays.link.recv().unwrap() {
            last.push(d.to_vec());
        }
        assert!(last.iter().any(|d| d == b"leaving"), "what the other said as it left is lost: {} datagrams taken", last.len());
    }
}

/// One side's network goes for `length`: both see it down (by silence, or
/// the connection's state), and it comes back after it, a new connection
/// (the next generation), the datagrams going again.
fn outage(sides: &mut [Side; 2], side: usize, length: Duration) {
    outage_and(sides, side, length, |_| {});
}

/// [`outage`], doing `meanwhile` once both see it down.
fn outage_and(sides: &mut [Side; 2], side: usize, length: Duration, meanwhile: impl FnOnce(&mut [Side; 2])) {
    let generation = sides[0].link.generation();
    sides[side].link.outage(length);
    run(sides, Duration::from_secs(10), |s| s.iter().all(|s| s.link.down_for().is_some()));
    meanwhile(sides);
    let before = sides.each_ref().map(|s| s.got.len());
    let took = run(sides, length + Duration::from_secs(20), heard_more(before));
    eprintln!("side {side}'s outage of {length:?}: back after {took:?} down, generations {:?}", sides.each_ref().map(|s| s.link.generation()));
    assert!(sides.iter().all(|s| s.link.is_open() && s.link.down_for().is_none() && s.link.generation() > generation && s.error.is_none()));
}

#[test]
fn a_direct_link_comes_back_after_an_outage() {
    let mut sides = direct(config());
    connected(&mut sides);
    // The joiner's network, then the host's.
    outage(&mut sides, 1, Duration::from_secs(3));
    outage(&mut sides, 0, Duration::from_secs(3));
}

/// The reconnect rooms the server made of `codes`, in order. (It may have
/// made others: a host whose newest code isn't acknowledged when the
/// connection drops waits in that code's room too, as when one side's
/// connection opened and the other's dial gave up and went again.)
fn made_of<'a, const N: usize>(server: &Server, codes: [&'a String; N]) -> Vec<&'a String> {
    server.reconnects_made().iter().filter_map(|made| codes.iter().find(|c| **c == made).copied()).collect()
}

/// Both sides go on for `length`, sending and taking.
fn run_for(sides: &mut [Side; 2], length: Duration) {
    let start = Instant::now();
    run(sides, length + Duration::from_secs(5), |_| start.elapsed() >= length);
}

/// A drop mid-link: the two meet again in the reconnect room of the code
/// they agreed (the server's first word of it), are through again with a
/// new code, and leave it; the next drop goes through that one. Then the
/// server's sockets drop in the middle of a reconnection (its restart):
/// they are made again, and the two meet all the same.
#[test]
fn a_room_link_comes_back_after_an_outage() {
    let (server, url) = server();
    let (mut sides, _) = room(&url, config());
    connected(&mut sides);
    let first = through(&mut sides, server.as_ref(), None);
    outage(&mut sides, 0, Duration::from_secs(3));
    let second = through(&mut sides, server.as_ref(), Some(&first));
    outage(&mut sides, 1, Duration::from_secs(3));
    let third = through(&mut sides, server.as_ref(), Some(&second));
    if let Some(server) = &server {
        assert_eq!(made_of(server, [&first, &second, &third]), [&first, &second], "each drop through the code agreed before it");
        outage_and(&mut sides, 1, Duration::from_secs(4), |sides| {
            run(sides, Duration::from_secs(5), |_| server.sockets() >= 2);
            server.kick_all();
        });
        through(&mut sides, Some(server), Some(&third));
        assert_eq!(made_of(server, [&first, &second, &third]), [&first, &second, &third]);
    }
}

/// A code is spent once the next is agreed: a stranger waiting in the room
/// of a spent code meets nobody, while the two meet again through their
/// newest one. A third socket in a reconnect room is refused, and so is a
/// code not of the clients' form.
#[test]
fn a_spent_code_is_no_use_and_a_third_is_refused() {
    let (server, url) = server();
    let (mut sides, _) = room(&url, config());
    connected(&mut sides);
    let first = through(&mut sides, server.as_ref(), None);
    outage(&mut sides, 1, Duration::from_secs(3));
    let second = through(&mut sides, server.as_ref(), Some(&first));
    let mut stranger = Client::open(&signal::reconnect_url(&url, &first, "stranger")).unwrap();
    assert_eq!(stranger.next(Duration::from_secs(5)), Some(FromServer::Welcome { role: None, peer: false, code: None }), "a room nobody else is in");
    outage_and(&mut sides, 0, Duration::from_secs(5), |sides| {
        // (Both in the room of their newest code by now.)
        run_for(sides, Duration::from_secs(1));
        if let Some(server) = &server {
            assert_eq!(server.sockets(), 3, "the two and the stranger");
        }
        let mut third = Client::open(&signal::reconnect_url(&url, &second, "third")).unwrap();
        assert!(matches!(third.next(Duration::from_secs(5)), Some(FromServer::Error { reason: Refusal::Full, .. })), "a third is refused");
        let mut bad = Client::open(&signal::reconnect_url(&url, "ABC234", "bad")).unwrap();
        assert!(matches!(bad.next(Duration::from_secs(5)), Some(FromServer::Error { reason: Refusal::BadRequest, .. })), "a code not of the clients' form is refused");
    });
    assert_eq!(stranger.next(Duration::from_millis(100)), None, "the stranger met nobody");
    stranger.leave();
    let third = through(&mut sides, server.as_ref(), Some(&second));
    assert_ne!(third, first);
    if let Some(server) = &server {
        // (The second time, the stranger's.)
        assert_eq!(made_of(server, [&first, &second]), [&first, &first, &second]);
    }
}

/// A drop before the new code is acknowledged still reconnects. Where the
/// host's code is lost (here it says none), the joiner has only the code
/// from before, and the two meet through that again: before any code is
/// agreed, that is the invite. Where only the joiner's acknowledgment is
/// lost, the joiner has the new code and the host, not knowing whether it
/// has, waits in both rooms: they meet through the new one.
#[test]
fn a_drop_before_the_code_is_acknowledged_still_reconnects() {
    let (server, url) = server();
    let (mut sides, code) = room(&url, config());
    // The first connection's code lost: they meet in the invite room again.
    sides[0].link.hold_codes(true);
    connected(&mut sides);
    outage(&mut sides, 1, Duration::from_secs(3));
    assert!(sides.iter().all(|s| s.link.reconnect_code().is_none() && s.link.in_a_room()));
    if let Some(server) = &server {
        assert_eq!(server.codes(), [code.as_str()]);
        // (The host's code, not said, had a room for the moment the joiner
        // took to come.)
        run(&mut sides, Duration::from_secs(5), |_| server.reconnect_rooms().is_empty());
    }
    sides[0].link.hold_codes(false);
    let first = through(&mut sides, server.as_ref(), None);
    // The next code lost: the drop after goes through the first again.
    sides[0].link.hold_codes(true);
    outage(&mut sides, 0, Duration::from_secs(3));
    run_for(&mut sides, Duration::from_millis(500));
    assert!(sides.iter().all(|s| s.link.reconnect_code() == Some(first.as_str()) && s.link.in_a_room()));
    outage(&mut sides, 1, Duration::from_secs(3));
    sides[0].link.hold_codes(false);
    let second = through(&mut sides, server.as_ref(), Some(&first));
    if let Some(server) = &server {
        assert_eq!(made_of(server, [&first, &second]), [&first], "both drops through the first");
    }
    // Only the acknowledgment lost: the joiner takes the new code and
    // leaves the room; the host waits in it.
    sides[1].link.hold_codes(true);
    outage(&mut sides, 1, Duration::from_secs(3));
    run(&mut sides, Duration::from_secs(10), |s| s[1].link.reconnect_code() != Some(second.as_str()) && !s[1].link.in_a_room());
    let third = sides[1].link.reconnect_code().unwrap().to_string();
    assert!(sides[0].link.reconnect_code() == Some(second.as_str()) && sides[0].link.in_a_room());
    outage(&mut sides, 0, Duration::from_secs(3));
    assert_eq!(sides[0].link.reconnect_code(), Some(third.as_str()), "the host met the joiner through the code only the joiner was known to have");
    sides[1].link.hold_codes(false);
    through(&mut sides, server.as_ref(), Some(&third));
    if let Some(server) = &server {
        assert_eq!(made_of(server, [&first, &second, &third]), [&first, &second, &third]);
    }
}

/// The server out of reach while the connection is down: the link goes on
/// trying, saying the server can't be reached ([`Link::problem`]), and
/// gives up when its time is out, the connection lost.
#[test]
fn a_server_out_of_reach_during_a_reconnect() {
    let server = Some(Server::start());
    let url = server.as_ref().unwrap().url();
    let config = Config { reconnect_timeout: Duration::from_secs(3), ..config() };
    let (mut sides, _) = room(&url, config);
    connected(&mut sides);
    through(&mut sides, server.as_ref(), None);
    server.as_ref().unwrap().set_reachable(false);
    sides[1].link.outage(Duration::from_secs(60));
    run(&mut sides, Duration::from_secs(10), |s| s.iter().all(|s| s.link.down_for().is_some() && s.link.problem().is_some()));
    for s in &sides {
        assert_eq!(s.link.problem().unwrap().kind, ErrorKind::ServerUnreachable);
    }
    run(&mut sides, Duration::from_secs(15), |s| s.iter().all(|s| s.error.is_some()));
    for s in &sides {
        let e = s.error.as_ref().unwrap();
        assert_eq!(e.kind, ErrorKind::ConnectionLost, "{e}");
    }
}

/// An outage longer than the reconnection timeout: both give up, saying
/// so, directly and in a room.
#[test]
fn a_link_gives_up_after_the_timeout() {
    let (_server, url) = server();
    let config = Config { reconnect_timeout: Duration::from_secs(3), ..config() };
    for mut sides in [direct(config.clone()), room(&url, config.clone()).0] {
        connected(&mut sides);
        sides[1].link.outage(Duration::from_secs(60));
        // (While it is down, each side says how long it has left.)
        run(&mut sides, Duration::from_secs(15), |s| s.iter().all(|s| s.link.down_for().is_some()));
        for s in &sides {
            let left = s.link.gives_up_in().expect("down");
            assert!(left <= Duration::from_secs(3) && left + s.link.down_for().unwrap() >= Duration::from_secs(3), "{left:?}");
        }
        run(&mut sides, Duration::from_secs(15), |s| s.iter().all(|s| s.error.is_some()));
        for s in &sides {
            let e = s.error.as_ref().unwrap();
            assert_eq!(e.kind, ErrorKind::ConnectionLost);
            assert!(e.detail.starts_with("the connection to the other player dropped (") && e.detail.ends_with(") and didn't come back within 3 seconds"), "{e}");
        }
    }
}

/// A native link and a browser's, by hand: with `NETTAI_TEST_SIGNAL` set,
/// this asks the server for a room and prints its code, and the web test
/// `a_link_meets_a_native_one` (tests/web.rs, built with the same server
/// and `NETTAI_TEST_ROOM` set to that code) joins it, within ten minutes
/// (the time to build and start it).
#[test]
#[ignore]
fn a_native_link_meets_a_browser() {
    let Ok(url) = std::env::var("NETTAI_TEST_SIGNAL") else { panic!("NETTAI_TEST_SIGNAL, please") };
    let config = Config { ice_servers: Vec::new(), ..Config::default() };
    let (mut link, code) = invite(&url, config);
    eprintln!("the room's code, for the browser's NETTAI_TEST_ROOM: {code}");
    let (start, mut got) = (Instant::now(), 0);
    while got <= 20 {
        assert!(start.elapsed() < Duration::from_secs(600), "no browser came (got {got}): {:?}", link.problem());
        link.send(b"from a native link").unwrap();
        while let Some(d) = link.recv().unwrap() {
            assert_eq!(d, b"from a browser");
            got += 1;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(link.role(), Some(Role::Host));
    // (The browser hears this side's last ones.)
    let until = Instant::now() + Duration::from_secs(2);
    while Instant::now() < until {
        link.send(b"from a native link").unwrap();
        while link.recv().unwrap().is_some() {}
        std::thread::sleep(Duration::from_millis(10));
    }
}
