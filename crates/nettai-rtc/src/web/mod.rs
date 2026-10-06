//! The web backend (wasm32): the browser's `RTCPeerConnection` and its data
//! channel, through web-sys. The browser does the sockets, the timers, the
//! gathering (STUN and TURN from the configuration) and the name lookups;
//! its callbacks and its promises (the offer, the answer) put what they
//! learn in a queue, which the same calls as the native backend's take
//! from, never waiting. Direct connect is native only: a browser can't
//! listen on a port, nor dial one without a description from the other
//! side.

pub(crate) mod ws;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use js_sys::{Array, Reflect, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    MessageEvent, RtcConfiguration, RtcDataChannel, RtcDataChannelInit, RtcDataChannelType, RtcIceCandidateInit, RtcIceServer, RtcPeerConnection,
    RtcPeerConnectionIceEvent, RtcPeerConnectionState, RtcSdpType, RtcSessionDescriptionInit,
};

use crate::{Candidate, Config, Error, IceServer, Instant, PeerEvent};

/// The data channel's label (the native backend's).
const LABEL: &str = "nettai";

/// A configuration's STUN and TURN servers, for the browser (which resolves
/// their names itself).
#[derive(Clone, Debug, Default)]
pub(crate) struct Servers(Vec<IceServer>);

impl Servers {
    pub(crate) fn resolve(config: &Config) -> Servers {
        Servers(config.ice_servers.clone())
    }
}

/// What the browser's callbacks and promises found, for the next poll.
#[derive(Default)]
struct Found {
    events: VecDeque<PeerEvent>,
    inbox: VecDeque<Vec<u8>>,
    open: bool,
    down: bool,
    /// The other's description is in (its candidates wait for it).
    remote: bool,
    early: Vec<Candidate>,
}

impl Found {
    fn down(&mut self, why: String) {
        if !self.down {
            self.down = true;
            self.open = false;
            self.events.push_back(PeerEvent::Down(why));
        }
    }
}

type Shared = Rc<RefCell<Found>>;

/// One WebRTC connection and its data channel (the crate's docs).
pub struct PeerConnection {
    pc: RtcPeerConnection,
    channel: RtcDataChannel,
    found: Shared,
    /// The callbacks, kept while the connection is.
    _callbacks: Vec<Closure<dyn FnMut(JsValue)>>,
}

fn js_error(what: &str, e: JsValue) -> Error {
    Error(format!("{what}: {}", e.as_string().unwrap_or_else(|| format!("{e:?}"))))
}

impl PeerConnection {
    /// The dialing side's connection: its offer comes as the first event
    /// (once the browser has made it), then its candidates.
    pub fn offer(config: &Config) -> Result<PeerConnection, Error> {
        PeerConnection::offer_with(config, &Servers::resolve(config))
    }

    /// The answering side's connection to `offer`: its answer comes as the
    /// first event, then its candidates.
    pub fn answer(config: &Config, offer: &str) -> Result<PeerConnection, Error> {
        PeerConnection::answer_with(config, &Servers::resolve(config), offer)
    }

    pub(crate) fn offer_with(_: &Config, servers: &Servers) -> Result<PeerConnection, Error> {
        let c = PeerConnection::new(servers)?;
        let (pc, found) = (c.pc.clone(), c.found.clone());
        spawn_local(async move {
            let made = async {
                let offer = JsFuture::from(pc.create_offer()).await.map_err(|e| js_error("can't make an offer", e))?;
                let sdp = Reflect::get(&offer, &"sdp".into()).ok().and_then(|s| s.as_string()).unwrap_or_default();
                let init = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
                init.set_sdp(&sdp);
                JsFuture::from(pc.set_local_description(&init)).await.map_err(|e| js_error("can't take the offer", e))?;
                Ok::<_, Error>(sdp)
            };
            match made.await {
                Ok(sdp) => found.borrow_mut().events.push_front(PeerEvent::Description(sdp)),
                Err(e) => found.borrow_mut().down(e.0),
            }
        });
        Ok(c)
    }

    pub(crate) fn answer_with(_: &Config, servers: &Servers, offer: &str) -> Result<PeerConnection, Error> {
        let c = PeerConnection::new(servers)?;
        let (pc, found, offer) = (c.pc.clone(), c.found.clone(), offer.to_string());
        spawn_local(async move {
            let made = async {
                let init = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
                init.set_sdp(&offer);
                JsFuture::from(pc.set_remote_description(&init)).await.map_err(|e| js_error("can't take the offer", e))?;
                remote_in(&pc, &found);
                let answer = JsFuture::from(pc.create_answer()).await.map_err(|e| js_error("can't make an answer", e))?;
                let sdp = Reflect::get(&answer, &"sdp".into()).ok().and_then(|s| s.as_string()).unwrap_or_default();
                let init = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
                init.set_sdp(&sdp);
                JsFuture::from(pc.set_local_description(&init)).await.map_err(|e| js_error("can't take the answer", e))?;
                Ok::<_, Error>(sdp)
            };
            match made.await {
                Ok(sdp) => found.borrow_mut().events.push_front(PeerEvent::Description(sdp)),
                Err(e) => found.borrow_mut().down(e.0),
            }
        });
        Ok(c)
    }

    /// A connection on `servers` with the game's channel (negotiated as
    /// stream 0, unordered, no retransmits), its callbacks set.
    fn new(servers: &Servers) -> Result<PeerConnection, Error> {
        let config = RtcConfiguration::new();
        let list = Array::new();
        for s in &servers.0 {
            let server = RtcIceServer::new();
            server.set_urls(&s.urls.iter().map(|u| JsValue::from_str(u)).collect::<Array>());
            if !s.username.is_empty() {
                server.set_username(&s.username);
                server.set_credential(&s.credential);
            }
            list.push(&server);
        }
        config.set_ice_servers(&list);
        let pc = RtcPeerConnection::new_with_configuration(&config).map_err(|e| js_error("can't make a connection", e))?;
        let init = RtcDataChannelInit::new();
        init.set_negotiated(true);
        init.set_id(0);
        init.set_ordered(false);
        init.set_max_retransmits(0);
        let channel = pc.create_data_channel_with_data_channel_dict(LABEL, &init);
        channel.set_binary_type(RtcDataChannelType::Arraybuffer);
        let found: Shared = Rc::default();
        let mut callbacks: Vec<Closure<dyn FnMut(JsValue)>> = Vec::new();
        let f = found.clone();
        callbacks.push(Closure::own_assert_unwind_safe(move |e: JsValue| {
            let Some(c) = e.unchecked_into::<RtcPeerConnectionIceEvent>().candidate() else { return };
            if !c.candidate().is_empty() {
                f.borrow_mut().events.push_back(PeerEvent::Candidate(Candidate { candidate: c.candidate(), mid: c.sdp_mid(), mline: c.sdp_m_line_index() }));
            }
        }));
        pc.set_onicecandidate(Some(callbacks.last().unwrap().as_ref().unchecked_ref()));
        let (f, p) = (found.clone(), pc.clone());
        callbacks.push(Closure::own_assert_unwind_safe(move |_: JsValue| {
            let state = p.connection_state();
            if matches!(state, RtcPeerConnectionState::Disconnected | RtcPeerConnectionState::Failed | RtcPeerConnectionState::Closed) {
                f.borrow_mut().down(format!("the connection is {state:?}").to_lowercase());
            }
        }));
        pc.set_onconnectionstatechange(Some(callbacks.last().unwrap().as_ref().unchecked_ref()));
        let f = found.clone();
        callbacks.push(Closure::own_assert_unwind_safe(move |_: JsValue| {
            let mut f = f.borrow_mut();
            if !f.down {
                f.open = true;
                f.events.push_back(PeerEvent::Open);
            }
        }));
        channel.set_onopen(Some(callbacks.last().unwrap().as_ref().unchecked_ref()));
        let f = found.clone();
        callbacks.push(Closure::own_assert_unwind_safe(move |_: JsValue| f.borrow_mut().down("the data channel closed".into())));
        channel.set_onclose(Some(callbacks.last().unwrap().as_ref().unchecked_ref()));
        let f = found.clone();
        callbacks.push(Closure::own_assert_unwind_safe(move |e: JsValue| {
            let data = e.unchecked_into::<MessageEvent>().data();
            if let Some(buffer) = data.dyn_ref::<js_sys::ArrayBuffer>() {
                f.borrow_mut().inbox.push_back(Uint8Array::new(buffer).to_vec());
            }
        }));
        channel.set_onmessage(Some(callbacks.last().unwrap().as_ref().unchecked_ref()));
        Ok(PeerConnection { pc, channel, found, _callbacks: callbacks })
    }

    /// The other side's answer to this side's offer.
    pub fn set_answer(&mut self, answer: &str) -> Result<(), Error> {
        let (pc, found, answer) = (self.pc.clone(), self.found.clone(), answer.to_string());
        spawn_local(async move {
            let init = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
            init.set_sdp(&answer);
            match JsFuture::from(pc.set_remote_description(&init)).await {
                Ok(_) => remote_in(&pc, &found),
                Err(e) => found.borrow_mut().down(js_error("can't take the answer", e).0),
            }
        });
        Ok(())
    }

    /// One of the other side's candidates (kept until its description is
    /// in).
    pub fn add_candidate(&mut self, c: &Candidate) -> Result<(), Error> {
        let mut found = self.found.borrow_mut();
        if !found.remote {
            found.early.push(c.clone());
            return Ok(());
        }
        drop(found);
        add_candidate(&self.pc, c);
        Ok(())
    }

    /// The next event, if any (the browser's callbacks have put them in).
    pub fn poll(&mut self) -> Option<PeerEvent> {
        self.next_event()
    }

    pub(crate) fn next_event(&mut self) -> Option<PeerEvent> {
        self.found.borrow_mut().events.pop_front()
    }

    /// Whether a datagram came that wasn't taken.
    pub(crate) fn has_datagram(&self) -> bool {
        !self.found.borrow().inbox.is_empty()
    }

    pub(crate) fn next_datagram(&mut self) -> Option<Vec<u8>> {
        self.found.borrow_mut().inbox.pop_front()
    }

    /// (The browser pumps: nothing to do.)
    pub(crate) fn pump(&mut self, _: Instant) {}

    /// Send a datagram to the other side; dropped if the channel isn't open.
    pub fn send(&mut self, data: &[u8]) {
        if self.found.borrow().open {
            let _ = self.channel.send_with_u8_array(data);
        }
    }

    /// The next datagram that came from the other side, if one has.
    pub fn recv(&mut self) -> Option<Vec<u8>> {
        self.next_datagram()
    }

    pub fn is_open(&self) -> bool {
        let found = self.found.borrow();
        found.open && !found.down
    }

    /// Close it.
    pub fn close(&mut self) {
        self.channel.close();
        self.pc.close();
        let mut found = self.found.borrow_mut();
        found.open = false;
        found.down = true;
    }
}

impl Drop for PeerConnection {
    fn drop(&mut self) {
        self.pc.set_onicecandidate(None);
        self.pc.set_onconnectionstatechange(None);
        self.channel.set_onopen(None);
        self.channel.set_onclose(None);
        self.channel.set_onmessage(None);
        self.close();
    }
}

/// The other's description is in: the candidates that waited for it go.
fn remote_in(pc: &RtcPeerConnection, found: &Shared) {
    let early = {
        let mut f = found.borrow_mut();
        f.remote = true;
        std::mem::take(&mut f.early)
    };
    for c in &early {
        add_candidate(pc, c);
    }
}

fn add_candidate(pc: &RtcPeerConnection, c: &Candidate) {
    let init = RtcIceCandidateInit::new(&c.candidate);
    init.set_sdp_mid(c.mid.as_deref());
    init.set_sdp_m_line_index(c.mline);
    // (A candidate the browser can't use is no reason to stop.)
    let _ = pc.add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(&init));
}
