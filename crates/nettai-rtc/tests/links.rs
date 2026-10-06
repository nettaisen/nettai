//! Two links on this machine, directly and through a signaling server (the
//! in-process one; `NETTAI_TEST_SIGNAL=ws://127.0.0.1:8787` takes the
//! Worker under `wrangler dev` instead): they connect, trade datagrams,
//! and come back after an outage, or give up after the timeout.

use std::time::{Duration, Instant};

use nettai_rtc::testing::Server;
use nettai_rtc::{Config, Link, Role};

/// Links that find each other on loopback, with no STUN server, quick to
/// notice a drop.
fn config() -> Config {
    Config { ice_servers: Vec::new(), loopback: true, silence: Duration::from_secs(1), dial_timeout: Duration::from_secs(5), reconnect_timeout: Duration::from_secs(20) }
}

/// One side: its link, and what came.
struct Side {
    link: Link,
    got: Vec<u32>,
    sent: u32,
    error: Option<String>,
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
                s.error = Some(e.0);
                continue;
            }
            loop {
                match s.link.recv() {
                    Ok(Some(d)) => s.got.push(u32::from_le_bytes(d.try_into().unwrap())),
                    Ok(None) => break,
                    Err(e) => {
                        s.error = Some(e.0);
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

/// A room code no other run uses.
fn code(what: &str) -> String {
    format!("{what}-{}-{}", std::process::id(), Instant::now().elapsed().as_nanos())
}

fn room(url: &str, code: &str, config: Config) -> [Side; 2] {
    // (The first in the room hosts: the host is let in first.)
    let mut host = Link::room(url, code, config.clone()).unwrap();
    let start = Instant::now();
    while host.role().is_none() {
        assert!(start.elapsed() < Duration::from_secs(10), "no welcome: {:?}", host.problem());
        host.poll().unwrap();
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(host.role(), Some(Role::Host));
    [Side::new(host), Side::new(Link::room(url, code, config).unwrap())]
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

#[test]
fn room_links_trade_datagrams() {
    let (_server, url) = server();
    let code = code("trade");
    let mut sides = room(&url, &code, config());
    connected(&mut sides);
    assert_eq!(sides.each_ref().map(|s| s.link.role()), [Some(Role::Host), Some(Role::Join)]);
    // A third player is refused.
    let mut third = Link::room(&url, &code, config()).unwrap();
    let start = Instant::now();
    let refused = loop {
        assert!(start.elapsed() < Duration::from_secs(10), "not refused");
        if let Err(e) = third.poll() {
            break e.0;
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    assert_eq!(refused, "the signaling server refused: the room is full");
    assert!(third.send(b"x").is_err() && third.recv().is_err());
}

/// One side's network goes for `length`: both see it down (by silence, or
/// the connection's state), and it comes back after it, a new connection
/// (the next generation), the datagrams going again.
fn outage(sides: &mut [Side; 2], side: usize, length: Duration) {
    let generation = sides[0].link.generation();
    sides[side].link.outage(length);
    run(sides, Duration::from_secs(10), |s| s.iter().all(|s| s.link.down_for().is_some()));
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

#[test]
fn a_room_link_comes_back_after_an_outage() {
    let (server, url) = server();
    let code = code("outage");
    let mut sides = room(&url, &code, config());
    connected(&mut sides);
    outage(&mut sides, 0, Duration::from_secs(3));
    outage(&mut sides, 1, Duration::from_secs(3));
    // The signaling server's sockets drop (the connection doesn't mind),
    // then an outage: they meet again through the room.
    if let Some(server) = server {
        server.kick(&code, Role::Host);
        server.kick(&code, Role::Join);
        let before = sides.each_ref().map(|s| s.got.len());
        run(&mut sides, Duration::from_secs(5), heard_more(before));
        outage(&mut sides, 1, Duration::from_secs(2));
    }
}

/// An outage longer than the reconnection timeout: both give up, saying
/// so, directly and in a room.
#[test]
fn a_link_gives_up_after_the_timeout() {
    let (_server, url) = server();
    let config = Config { reconnect_timeout: Duration::from_secs(3), ..config() };
    for mut sides in [direct(config.clone()), room(&url, &code("give-up"), config.clone())] {
        connected(&mut sides);
        sides[1].link.outage(Duration::from_secs(60));
        run(&mut sides, Duration::from_secs(15), |s| s.iter().all(|s| s.error.is_some()));
        for s in &sides {
            let e = s.error.as_deref().unwrap();
            assert!(e.starts_with("the connection to the other player dropped (") && e.ends_with(") and didn't come back within 3 seconds"), "{e}");
        }
    }
}
