//! Netplay over real UDP on loopback: two peers, each with its own socket,
//! shake hands (the content's hash, the seed), then play a synthetic
//! battle with rollback, their inputs going over rennet in datagrams, each
//! at its own pace. Their settled states must agree at every tick both
//! settled. Once in two threads of this process, and once in two
//! processes (this test binary run again, as each peer).

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use nettai_battle::Battle;
use nettai_battle::content::testing;
use nettai_netplay::standin::{Masher, StandInBattle, folder, netbattle};
use nettai_netplay::transport::{Connection, Hello, Role, Udp};
use nettai_netplay::{BattleWorld, Peer, PeerConfig};

/// Ticks each peer plays.
const TICKS: u32 = 600;

fn content() -> Arc<nettai_battle::Content> {
    testing::content()
}

/// One peer: shake hands on `udp`, then play `TICKS` ticks at `frame` a
/// tick, calling `settled(tick, digest)` after every advance that settled
/// something. Returns the peer's report line.
fn play(udp: Udp, role: Role, frame: Duration, mut settled: impl FnMut(u32, u64)) -> String {
    let c = content();
    let hello = Hello::new(role, c.hash(), Vec::new(), role.side() as u64 ^ 0xC0FFEE);
    let mut conn = match role {
        Role::Host => Connection::host(udp, hello, Duration::from_secs(30)),
        Role::Join => Connection::join(udp, hello, Duration::from_secs(30)),
    }
    .unwrap_or_else(|e| panic!("{role:?}: {e}"));
    let side = conn.side();
    use testing::{SUN_GUN_1, SUN_GUN_3};
    let f = || folder(&c, &[(SUN_GUN_3, 0), (SUN_GUN_1, 0)]);
    let setup = netbattle(&c, testing::LINK_BATTLE, 300, conn.seed(), [f(), f()]);
    let mut peer = Peer::new(BattleWorld::new(StandInBattle::new(Battle::new(setup, c.clone())), side), PeerConfig::new(1, 12));
    let mut masher = Masher::new(conn.seed() as u64 ^ side as u64);
    masher.buster = true;
    let start = Instant::now();
    let ms = || start.elapsed().as_millis() as u64;
    let mut last = 0;
    let mut done_at: Option<Instant> = None;
    for wall in 0u64.. {
        // Settled every tick: play on a little, so the other side gets
        // the inputs it still needs.
        if done_at.is_some_and(|t| t.elapsed() > Duration::from_millis(500)) {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(60), "{role:?}: stuck at tick {last}");
        while let Some(frame) = conn.try_recv_frame().unwrap() {
            peer.receive(frame, ms()).unwrap();
        }
        let decided = done_at.is_none() && peer.wait().is_none();
        if decided {
            peer.decide(masher.buttons());
        }
        conn.send_frame(&peer.datagram(ms())).unwrap();
        if decided {
            peer.advance(|_| ());
            let state = peer.session().settled_state();
            if state.tick() > last {
                last = state.tick();
                settled(last, state.battle().digest());
            }
            if last >= TICKS {
                done_at = Some(Instant::now());
            }
        }
        // The next frame's time.
        let next = start + frame * (wall as u32 + 1);
        std::thread::sleep(next.saturating_duration_since(Instant::now()));
    }
    let s = peer.stats();
    let l = peer.link_stats();
    format!(
        "{role:?} (side {side}, seed {:#x}): settled {last} ticks; rollbacks {} (deepest {}), stalls {}, parked {}; \
         {} datagrams sent of {:.1} bytes, {} received, round trip {:?} ms",
        conn.seed(),
        s.rollbacks,
        s.max_rollback,
        s.stalls,
        s.parked,
        l.sent,
        l.mean_size(),
        l.received,
        l.srtt.map(|r| r.round()),
    )
}

/// The digests at the ticks both peers settled must be equal.
fn compare(a: &[(u32, u64)], b: &[(u32, u64)]) -> usize {
    let b: std::collections::HashMap<u32, u64> = b.iter().copied().collect();
    let mut common = 0;
    for (tick, digest) in a {
        if let Some(other) = b.get(tick) {
            assert_eq!(digest, other, "the peers' settled states differ at tick {tick}");
            common += 1;
        }
    }
    common
}

#[test]
fn two_threads_over_loopback() {
    let host = Udp::host_on("127.0.0.1:0").unwrap();
    let addr = host.local_addr().unwrap();
    let frame = Duration::from_millis(4);
    let host = std::thread::spawn(move || {
        let mut digests = Vec::new();
        let report = play(host, Role::Host, frame, |t, d| digests.push((t, d)));
        (report, digests)
    });
    // The joiner a bit slower: clock sync evens them out.
    let mut digests = Vec::new();
    let report = play(Udp::join(addr).unwrap(), Role::Join, frame + Duration::from_micros(200), |t, d| digests.push((t, d)));
    let (host_report, host_digests) = host.join().unwrap();
    eprintln!("{host_report}\n{report}");
    let common = compare(&host_digests, &digests);
    eprintln!("{common} settled ticks compared");
    assert!(host_digests.last().unwrap().0 >= TICKS && digests.last().unwrap().0 >= TICKS);
    assert!(common > TICKS as usize / 4, "{common}");
}

/// Two processes: this binary, run again as each peer (`peer_process`),
/// the host first; each prints its settled digests.
#[test]
fn two_processes_over_loopback() {
    let exe = std::env::current_exe().unwrap();
    let run = |role: &str| {
        Command::new(&exe)
            .args(["--exact", "peer_process", "--nocapture", "--test-threads=1"])
            .env("NETTAI_UDP_PEER", role)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap()
    };
    let mut host = run("host");
    let mut out = BufReader::new(host.stdout.take().unwrap());
    let mut line = String::new();
    let port = loop {
        line.clear();
        assert!(out.read_line(&mut line).unwrap() > 0, "the host process printed no port");
        // (The test harness's "test peer_process ... " comes first.)
        if let Some((_, p)) = line.trim().split_once("PORT ") {
            break p.parse::<u16>().unwrap();
        }
    };
    // (The host's output is read as it comes, so that its pipe never fills.)
    let host_out = std::thread::spawn(move || {
        let mut rest = String::new();
        std::io::Read::read_to_string(&mut out, &mut rest).unwrap();
        rest
    });
    let join = run(&format!("join:127.0.0.1:{port}"));
    let parse = |text: &str| -> (Vec<(u32, u64)>, String) {
        let mut digests = Vec::new();
        let mut report = String::new();
        for l in text.lines() {
            if let Some((_, rest)) = l.split_once("DIGEST ") {
                let (t, d) = rest.split_once(' ').unwrap();
                digests.push((t.parse().unwrap(), u64::from_str_radix(d, 16).unwrap()));
            } else if let Some((_, r)) = l.split_once("REPORT ") {
                report = r.to_string();
            }
        }
        (digests, report)
    };
    let joined = join.wait_with_output().unwrap();
    let rest = host_out.join().unwrap();
    assert!(host.wait().unwrap().success() && joined.status.success());
    let (a, ra) = parse(&rest);
    let (b, rb) = parse(&String::from_utf8_lossy(&joined.stdout));
    eprintln!("{ra}\n{rb}");
    let common = compare(&a, &b);
    eprintln!("{common} settled ticks compared across the processes");
    assert!(a.last().unwrap().0 >= TICKS && b.last().unwrap().0 >= TICKS);
    assert!(common > TICKS as usize / 4, "{common}");
}

/// A peer process for `two_processes_over_loopback`: nothing unless
/// `NETTAI_UDP_PEER` says `host` or `join:ADDR`.
#[test]
fn peer_process() {
    let Ok(role) = std::env::var("NETTAI_UDP_PEER") else { return };
    let (udp, role) = if role == "host" {
        let udp = Udp::host_on("127.0.0.1:0").unwrap();
        println!("PORT {}", udp.local_addr().unwrap().port());
        (udp, Role::Host)
    } else {
        (Udp::join(role.strip_prefix("join:").unwrap()).unwrap(), Role::Join)
    };
    std::io::stdout().flush().unwrap();
    let report = play(udp, role, Duration::from_millis(5), |tick, digest| println!("DIGEST {tick} {digest:016x}"));
    println!("REPORT {report}");
}
