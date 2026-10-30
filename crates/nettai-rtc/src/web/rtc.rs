//! The web's WebRTC connection: the browser's `RTCPeerConnection` and its data channels, behind `crate::rtc`'s API.
//! (It began as tangobattle/datachannel-facade's web backend, MPL-2.0, which this folds in.)
//!
//! The browser answers through events and Promises, on the page's one thread. Each event handler, and each Promise
//! as it settles, puts its [`Event`] in the connection's queue, which [`PeerConnection::poll`] takes from. A
//! Promise that rejects has nowhere to be returned to: it is logged, and the connection says it failed, which is
//! what a failed description or candidate exchange amounts to.
//!
//! An `RTCPeerConnection`, its handlers and the queue they share all belong to the thread that made them, so a
//! [`PeerConnection`] is neither `Send` nor `Sync` (the wasm build has threads: nothing here asserts otherwise).

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast as _, JsValue};
use wasm_bindgen_futures::JsFuture;

use crate::rtc::{ChannelId, ChannelOptions, Configuration, Error, Event, SdpType, State};

type Queue = Rc<RefCell<VecDeque<Event>>>;

/// A JS event handler, kept for as long as what it is attached to: a wasm-bindgen `Closure` unregisters itself
/// when dropped, which would stop the events without a word.
type Handler<T> = Closure<dyn FnMut(T)>;

fn handler<T: wasm_bindgen::convert::FromWasmAbi + 'static>(f: impl FnMut(T) + 'static) -> Handler<T> {
    // (`Closure::wrap` asks an `UnwindSafe` closure of a build with `-C panic=unwind`, which the wasm build is, and
    // a boxed `dyn FnMut` isn't one.)
    Closure::wrap_assert_unwind_safe(Box::new(f) as Box<dyn FnMut(T)>)
}

fn function<T>(h: &Handler<T>) -> &js_sys::Function {
    h.as_ref().unchecked_ref()
}

/// What the browser threw, as an error: a JS `Error`'s message, else the value as it is.
fn js_error(value: JsValue) -> Error {
    Error::new(value.dyn_ref::<js_sys::Error>().map(|e| String::from(e.message())).unwrap_or_else(|| format!("{value:?}")))
}

/// A Promise that rejected, logged, and the connection failed.
fn fail(events: &Queue, what: &str, e: JsValue) {
    log::error!("{what} failed: {}", js_error(e));
    events.borrow_mut().push_back(Event::State(State::Failed));
}

/// A data channel and its handlers.
struct Channel {
    dc: web_sys::RtcDataChannel,
    _on_open: Handler<JsValue>,
    _on_close: Handler<JsValue>,
    _on_message: Handler<web_sys::MessageEvent>,
}

pub(crate) struct PeerConnection {
    channels: Vec<Channel>,
    pc: web_sys::RtcPeerConnection,
    _on_candidate: Handler<web_sys::RtcPeerConnectionIceEvent>,
    _on_state: Handler<JsValue>,
    events: Queue,
}

impl PeerConnection {
    pub(crate) fn new(config: Configuration) -> Result<PeerConnection, Error> {
        let raw = web_sys::RtcConfiguration::new();
        let servers = js_sys::Array::new();
        for server in &config.ice_servers {
            let entry = js_sys::Object::new();
            let urls = js_sys::Array::new();
            for url in &server.urls {
                urls.push(&JsValue::from_str(url));
            }
            set(&entry, "urls", &urls);
            // (A server with no username takes no credentials: a STUN server's.)
            if !server.username.is_empty() {
                set(&entry, "username", &JsValue::from_str(&server.username));
                set(&entry, "credential", &JsValue::from_str(&server.credential));
            }
            servers.push(&entry);
        }
        raw.set_ice_servers(&servers);
        raw.set_ice_transport_policy(if config.relay_only { web_sys::RtcIceTransportPolicy::Relay } else { web_sys::RtcIceTransportPolicy::All });
        let pc = web_sys::RtcPeerConnection::new_with_configuration(&raw).map_err(js_error)?;

        let events = Queue::default();
        let q = events.clone();
        let on_candidate = handler(move |ev: web_sys::RtcPeerConnectionIceEvent| {
            // (No candidate, or an empty one: the end of gathering, which says nothing.)
            if let Some(line) = ev.candidate().map(|c| c.candidate()).filter(|line| !line.is_empty()) {
                q.borrow_mut().push_back(Event::Candidate(line));
            }
        });
        pc.set_onicecandidate(Some(function(&on_candidate)));
        let (q, of) = (events.clone(), pc.clone());
        let on_state = handler(move |_: JsValue| {
            let state = match of.connection_state() {
                web_sys::RtcPeerConnectionState::Connected => State::Connected,
                web_sys::RtcPeerConnectionState::Disconnected => State::Disconnected,
                web_sys::RtcPeerConnectionState::Failed => State::Failed,
                web_sys::RtcPeerConnectionState::Closed => State::Closed,
                // (`new` and `connecting` are both not up yet.)
                _ => State::Connecting,
            };
            q.borrow_mut().push_back(Event::State(state));
        });
        pc.set_onconnectionstatechange(Some(function(&on_state)));
        Ok(PeerConnection { channels: Vec::new(), pc, _on_candidate: on_candidate, _on_state: on_state, events })
    }

    pub(crate) fn create_channel(&mut self, label: &str, options: ChannelOptions) -> Result<ChannelId, Error> {
        let init = web_sys::RtcDataChannelInit::new();
        init.set_ordered(options.ordered);
        // (The browser takes a number of retransmits for an unreliable channel alone.)
        if let Some(retransmits) = options.max_retransmits {
            init.set_max_retransmits(retransmits);
        }
        init.set_negotiated(true);
        init.set_id(options.stream);
        let dc = self.pc.create_data_channel_with_data_channel_dict(label, &init);
        // Messages are bytes, never strings: without this the browser hands a message over as a Blob, whose bytes
        // can only be read later.
        dc.set_binary_type(web_sys::RtcDataChannelType::Arraybuffer);
        let id = ChannelId(self.channels.len());
        let q = self.events.clone();
        let on_open = handler(move |_: JsValue| q.borrow_mut().push_back(Event::Open(id)));
        let q = self.events.clone();
        let on_close = handler(move |_: JsValue| q.borrow_mut().push_back(Event::Closed(id)));
        let q = self.events.clone();
        let on_message = handler(move |ev: web_sys::MessageEvent| {
            let Ok(buffer) = ev.data().dyn_into::<js_sys::ArrayBuffer>() else {
                log::warn!("data channel: ignoring non-binary message");
                return;
            };
            q.borrow_mut().push_back(Event::Message(id, js_sys::Uint8Array::new(&buffer).to_vec()));
        });
        dc.set_onopen(Some(function(&on_open)));
        dc.set_onclose(Some(function(&on_close)));
        dc.set_onmessage(Some(function(&on_message)));
        self.channels.push(Channel { dc, _on_open: on_open, _on_close: on_close, _on_message: on_message });
        Ok(id)
    }

    /// This side's description made and set: the browser creates it (`createOffer`, `createAnswer`), and it comes
    /// as [`Event::Description`] once set.
    pub(crate) fn set_local_description(&mut self, type_: SdpType) -> Result<(), Error> {
        let (pc, events) = (self.pc.clone(), self.events.clone());
        wasm_bindgen_futures::spawn_local(async move {
            let (made, what) = match type_ {
                SdpType::Offer => (JsFuture::from(pc.create_offer()).await, "createOffer"),
                SdpType::Answer => (JsFuture::from(pc.create_answer()).await, "createAnswer"),
            };
            let init: web_sys::RtcSessionDescriptionInit = match made {
                Ok(made) => made.unchecked_into(),
                Err(e) => return fail(&events, what, e),
            };
            if let Err(e) = JsFuture::from(pc.set_local_description(&init)).await {
                return fail(&events, "setLocalDescription", e);
            }
            if let Some(description) = pc.local_description() {
                events.borrow_mut().push_back(Event::Description(description.sdp()));
            }
        });
        Ok(())
    }

    pub(crate) fn set_remote_description(&mut self, type_: SdpType, sdp: &str) -> Result<(), Error> {
        let init = web_sys::RtcSessionDescriptionInit::new(web_type(type_));
        init.set_sdp(sdp);
        let (pc, events) = (self.pc.clone(), self.events.clone());
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(e) = JsFuture::from(pc.set_remote_description(&init)).await {
                fail(&events, "setRemoteDescription", e);
            }
        });
        Ok(())
    }

    /// The other side's candidate (`candidate:...`); an empty one is the end of its candidates, which the browser
    /// spells as no candidate.
    ///
    /// The browser refuses a candidate that names no media section (`RTCIceCandidate` throws when both `sdpMid`
    /// and `sdpMLineIndex` are null), and a candidate line carries neither: a connection here is a bundled
    /// data channel connection, with one media section, so the section is index 0.
    pub(crate) fn add_remote_candidate(&mut self, candidate: &str) -> Result<(), Error> {
        let candidate = if candidate.is_empty() {
            None
        } else {
            let init = web_sys::RtcIceCandidateInit::new(candidate);
            init.set_sdp_m_line_index(Some(0));
            Some(web_sys::RtcIceCandidate::new(&init).map_err(js_error)?)
        };
        let (pc, events) = (self.pc.clone(), self.events.clone());
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(e) = JsFuture::from(pc.add_ice_candidate_with_opt_rtc_ice_candidate(candidate.as_ref())).await {
                fail(&events, "addIceCandidate", e);
            }
        });
        Ok(())
    }

    pub(crate) fn send(&mut self, channel: ChannelId, data: &[u8]) -> Result<(), Error> {
        // Copied into a buffer of JS's own: wasm-bindgen would pass `data` as a view on the module's memory, which
        // in a build with shared memory (the web build's threads) `send` refuses. A message is small.
        let copy = js_sys::Uint8Array::new_with_length(data.len() as u32);
        copy.copy_from(data);
        self.channels[channel.0].dc.send_with_array_buffer(&copy.buffer()).map_err(js_error)
    }

    pub(crate) fn close(&mut self) {
        for c in &self.channels {
            c.dc.close();
        }
        self.pc.close();
    }

    pub(crate) fn poll(&mut self) -> Option<Event> {
        self.events.borrow_mut().pop_front()
    }
}

impl Drop for PeerConnection {
    /// The channels closed, then the connection, and their handlers taken off first: one the browser called after
    /// its `Closure` is dropped would throw.
    fn drop(&mut self) {
        for c in &self.channels {
            c.dc.set_onopen(None);
            c.dc.set_onclose(None);
            c.dc.set_onmessage(None);
            c.dc.close();
        }
        self.pc.set_onicecandidate(None);
        self.pc.set_onconnectionstatechange(None);
        self.pc.close();
    }
}

fn web_type(type_: SdpType) -> web_sys::RtcSdpType {
    match type_ {
        SdpType::Offer => web_sys::RtcSdpType::Offer,
        SdpType::Answer => web_sys::RtcSdpType::Answer,
    }
}

fn set(object: &js_sys::Object, key: &str, value: &JsValue) {
    let _ = js_sys::Reflect::set(object, &JsValue::from_str(key), value);
}
