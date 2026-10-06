#![cfg(target_arch = "wasm32")]
//! The web backend in a browser (`wasm-bindgen-test-runner`, which the
//! workspace's wasm32 runner is; `CHROMEDRIVER=...` for a headless Chrome):
//! two connections in one page, their descriptions and candidates handed
//! across in memory, open and trade datagrams; and with
//! `NETTAI_TEST_SIGNAL=ws://127.0.0.1:8787` when built (the Worker under
//! `wrangler dev`), two links meet in a room of it.

use nettai_rtc::{Config, Link, PeerConnection, PeerEvent, Role};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// Wait `ms` milliseconds (the browser's event loop runs meanwhile).
async fn sleep(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let window: web_sys::Window = js_sys::global().unchecked_into();
        window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms).unwrap();
    });
    wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
}

/// No STUN server: the two are in one page.
fn config() -> Config {
    Config { ice_servers: Vec::new(), ..Config::default() }
}

#[wasm_bindgen_test]
async fn two_connections_in_a_page_trade_datagrams() {
    let mut a = PeerConnection::offer(&config()).unwrap();
    let mut b: Option<PeerConnection> = None;
    let mut open = [false, false];
    let mut got: [Vec<Vec<u8>>; 2] = Default::default();
    for _ in 0..1000 {
        while let Some(e) = a.poll() {
            match e {
                PeerEvent::Description(offer) => b = Some(PeerConnection::answer(&config(), &offer).unwrap()),
                PeerEvent::Candidate(c) => b.as_mut().expect("the offer first").add_candidate(&c).unwrap(),
                PeerEvent::Open => open[0] = true,
                PeerEvent::Down(why) => panic!("a: {why}"),
            }
        }
        if let Some(b) = &mut b {
            while let Some(e) = b.poll() {
                match e {
                    PeerEvent::Description(answer) => a.set_answer(&answer).unwrap(),
                    PeerEvent::Candidate(c) => a.add_candidate(&c).unwrap(),
                    PeerEvent::Open => open[1] = true,
                    PeerEvent::Down(why) => panic!("b: {why}"),
                }
            }
            if open == [true, true] {
                a.send(b"from a");
                b.send(b"from b");
            }
            got[0].extend(std::iter::from_fn(|| a.recv()));
            got[1].extend(std::iter::from_fn(|| b.recv()));
        }
        if !got[0].is_empty() && !got[1].is_empty() {
            assert_eq!((got[0][0].as_slice(), got[1][0].as_slice()), (&b"from b"[..], &b"from a"[..]));
            return;
        }
        sleep(10).await;
    }
    panic!("not connected: open {open:?}");
}

#[wasm_bindgen_test]
async fn two_links_meet_in_a_room() {
    let Some(server) = option_env!("NETTAI_TEST_SIGNAL") else { return };
    let code = format!("web-{}", (js_sys::Math::random() * 1e9) as u64);
    let mut host = Link::room(server, &code, config()).unwrap();
    for _ in 0..500 {
        host.poll().unwrap();
        if host.role().is_some() {
            break;
        }
        sleep(10).await;
    }
    assert_eq!(host.role(), Some(Role::Host));
    let mut links = [host, Link::room(server, &code, config()).unwrap()];
    let mut got = [0, 0];
    for _ in 0..1000 {
        for (link, got) in links.iter_mut().zip(&mut got) {
            link.send(b"hello").unwrap();
            while let Some(d) = link.recv().unwrap() {
                assert_eq!(d, b"hello");
                *got += 1;
            }
        }
        if got.iter().all(|g| *g > 10) {
            assert_eq!(links[1].role(), Some(Role::Join));
            return;
        }
        sleep(10).await;
    }
    panic!("not connected: got {got:?}");
}

/// A browser's link and a native one: with `NETTAI_TEST_ROOM` too when
/// built, this joins the room that the native test
/// `a_native_link_meets_a_browser` (tests/links.rs, ignored) hosts.
#[wasm_bindgen_test]
async fn a_link_meets_a_native_one() {
    let (Some(server), Some(code)) = (option_env!("NETTAI_TEST_SIGNAL"), option_env!("NETTAI_TEST_ROOM")) else { return };
    let mut link = Link::room(server, code, config()).unwrap();
    let mut got = 0;
    for _ in 0..2000 {
        link.send(b"from a browser").unwrap();
        while let Some(d) = link.recv().unwrap() {
            assert_eq!(d, b"from a native link");
            got += 1;
        }
        if got > 20 {
            assert_eq!(link.role(), Some(Role::Join));
            return;
        }
        sleep(10).await;
    }
    panic!("not connected: got {got}");
}
