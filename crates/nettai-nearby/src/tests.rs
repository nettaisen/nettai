//! Transfers over the loopback, asked by port (no discovery: the offers aren't advertised).

use crate::{Event, File, Nearby, protocol};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

fn wait_for(nearby: &mut Nearby, what: impl Fn(&Event) -> bool) -> Event {
    let until = Instant::now() + Duration::from_secs(10);
    while Instant::now() < until {
        if let Some(event) = nearby.poll().into_iter().find(|e| what(e)) {
            return event;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("no such event in time");
}

fn files() -> Vec<File> {
    vec![File { name: "BR6E.zip".into(), bytes: vec![7; 300_000] }, File { name: "BR5J.zip".into(), bytes: vec![9; 5] }]
}

/// An offer, unadvertised, and where to ask it.
fn offerer() -> (Nearby, Vec<SocketAddr>) {
    let mut offerer = Nearby::named("computer".into(), false);
    offerer.offer(true);
    let port = offerer.port().expect("offering");
    (offerer, vec![SocketAddr::from(([127, 0, 0, 1], port))])
}

/// Asked over the loopback: the offerer's player is asked, sends, and the asker has the files whole; then a second
/// ask is declined.
#[test]
fn an_ask_is_answered_with_the_files() {
    let (mut offerer, at) = offerer();
    let mut asker = Nearby::named("phone".into(), false);

    asker.ask_at("computer".into(), at.clone());
    let Event::Asked { request, name } = wait_for(&mut offerer, |e| matches!(e, Event::Asked { .. })) else { unreachable!() };
    assert_eq!(name, "phone");
    offerer.answer(request, Some(files()));
    let Event::Received { name, files: got } = wait_for(&mut asker, |e| matches!(e, Event::Received { .. })) else { unreachable!() };
    assert_eq!((name.as_str(), got), ("computer", files()));
    assert!(matches!(wait_for(&mut offerer, |e| matches!(e, Event::Sent { .. })), Event::Sent { name } if name == "phone"));

    asker.ask_at("computer".into(), at);
    let Event::Asked { request, .. } = wait_for(&mut offerer, |e| matches!(e, Event::Asked { .. })) else { unreachable!() };
    offerer.answer(request, None);
    assert!(matches!(wait_for(&mut asker, |e| matches!(e, Event::Declined { .. })), Event::Declined { name } if name == "computer"));
}

/// While one asker waits for the offerer's player, another is told it is busy.
#[test]
fn a_second_asker_is_told_busy() {
    let (mut offerer, at) = offerer();
    let mut first = Nearby::named("phone".into(), false);
    first.ask_at("computer".into(), at.clone());
    let Event::Asked { request, .. } = wait_for(&mut offerer, |e| matches!(e, Event::Asked { .. })) else { unreachable!() };
    let mut second = Nearby::named("tablet".into(), false);
    second.ask_at("computer".into(), at);
    assert!(matches!(wait_for(&mut second, |e| matches!(e, Event::Busy { .. })), Event::Busy { name } if name == "computer"));
    offerer.answer(request, None);
    assert!(matches!(wait_for(&mut first, |e| matches!(e, Event::Declined { .. })), Event::Declined { .. }));
}

/// An asker of another version is told the offerer's, and the offerer's player isn't asked.
#[test]
fn another_version_is_refused() {
    let (mut offerer, at) = offerer();
    let mut stream = TcpStream::connect(at[0]).unwrap();
    use std::io::Write;
    stream.write_all(&protocol::MAGIC).unwrap();
    stream.write_all(&(protocol::VERSION + 1).to_be_bytes()).unwrap();
    stream.write_all(&[0, 1, b'x']).unwrap();
    assert_eq!(protocol::read_reply(&mut stream).unwrap(), protocol::Reply::OtherVersion(protocol::VERSION));
    let said = wait_for(&mut offerer, |e| matches!(e, Event::OtherVersion { .. }));
    assert!(matches!(said, Event::OtherVersion { version, .. } if version == protocol::VERSION + 1));
}

/// Nothing answers at the address: the ask fails, and a next ask can be made.
#[test]
fn an_ask_of_nothing_fails() {
    let (offerer, at) = offerer();
    drop(offerer);
    // (The listener's thread lets the port go within its next turn.)
    std::thread::sleep(Duration::from_millis(200));
    let mut asker = Nearby::named("phone".into(), false);
    asker.ask_at("computer".into(), at.clone());
    assert!(matches!(wait_for(&mut asker, |e| matches!(e, Event::Failed { .. })), Event::Failed { name, .. } if name == "computer"));
    asker.ask_at("computer".into(), at);
    assert!(matches!(wait_for(&mut asker, |e| matches!(e, Event::Failed { .. })), Event::Failed { why, .. } if why.contains("connecting")));
}
