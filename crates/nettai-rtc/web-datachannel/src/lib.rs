//! A browser WebRTC wrapper shaped like [`libdatachannel`]'s Rust API.
//!
//! [`libdatachannel`]: https://github.com/tangobattle/libdatachannel-rs
//!
//! This exists to be the wasm half of `datachannel-facade`, so the two
//! halves are worth describing together: the facade switches between
//! this crate and `libdatachannel` on `target_arch`, and that switch can
//! only stay thin if both present the *same shape*. So this crate is
//! deliberately not the most natural browser API — it is the browser
//! bent into libdatachannel's.
//!
//! # Why everything is sync-returning
//!
//! In the browser, `setLocalDescription`, `setRemoteDescription` and
//! `addIceCandidate` all return Promises. In libdatachannel they return
//! immediately, and what you actually *wanted* — the local SDP, the
//! gathered candidates — arrives later through callbacks anyway.
//!
//! That second point is the whole trick. Since the interesting result of
//! `set_local_description` was never the return value on either side,
//! this crate drives each Promise on the microtask queue
//! ([`wasm_bindgen_futures::spawn_local`]) and reports through the same
//! callbacks libdatachannel uses. Callers get one sync signature on both
//! targets, with identical observable behaviour.
//!
//! A Promise that rejects has nowhere to be returned to, so it is logged
//! and the connection is driven to [`State::Failed`] — which is what a
//! failed description exchange means regardless.
//!
//! # Thread affinity
//!
//! This is the one place the two shapes genuinely differ, and it is not
//! papered over. An `RtcPeerConnection`, its event handlers and the
//! `Rc`/`RefCell` cells sharing them all belong to the thread that made
//! them, so [`PeerConnection`] and [`DataChannel`] are neither `Send`
//! nor `Sync`, and callbacks are taken as plain `Fn + 'static`.
//!
//! libdatachannel fires from its own network threads and so demands
//! `Fn + Send + Sync`. Asserting that here with `unsafe impl` would be
//! wrong rather than merely strict: "wasm has one thread" is a property
//! of one build configuration, not of the target — `-Ctarget-feature=
//! +atomics` gives wasm real threads, and a `Send` JS handle or a raced
//! `Rc` refcount is undefined behaviour the moment one exists.
//!
//! `datachannel-facade` reconciles the two with a marker bound that
//! means `Send + Sync` off wasm and nothing here, so cross-platform
//! callers still write the bound once.

use wasm_bindgen::JsCast as _;

/// Whatever the browser threw, as an error.
#[derive(Debug)]
pub struct Error(String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<wasm_bindgen::JsValue> for Error {
    fn from(value: wasm_bindgen::JsValue) -> Self {
        Self(
            value
                .dyn_ref::<js_sys::Error>()
                .map(|e| String::from(e.message()))
                .unwrap_or_else(|| format!("{value:?}")),
        )
    }
}

/// Which half of an offer/answer exchange a description is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SdpType {
    Answer,
    Offer,
    Pranswer,
    Rollback,
}

impl SdpType {
    fn to_web(self) -> web_sys::RtcSdpType {
        match self {
            SdpType::Answer => web_sys::RtcSdpType::Answer,
            SdpType::Offer => web_sys::RtcSdpType::Offer,
            SdpType::Pranswer => web_sys::RtcSdpType::Pranswer,
            SdpType::Rollback => web_sys::RtcSdpType::Rollback,
        }
    }

    fn from_web(v: web_sys::RtcSdpType) -> Option<Self> {
        Some(match v {
            web_sys::RtcSdpType::Answer => SdpType::Answer,
            web_sys::RtcSdpType::Offer => SdpType::Offer,
            web_sys::RtcSdpType::Pranswer => SdpType::Pranswer,
            web_sys::RtcSdpType::Rollback => SdpType::Rollback,
            _ => return None,
        })
    }
}

/// A full session description.
#[derive(Clone, Debug)]
pub struct Description {
    pub type_: SdpType,
    pub sdp: String,
}

/// Connection lifecycle, named as libdatachannel names it. The browser's
/// `new` maps onto `Connecting`: libdatachannel has no distinct
/// pre-connection state, and callers keyed on `Connected` / `Failed` /
/// `Closed` don't care about the difference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Connecting,
    Connected,
    Disconnected,
    Failed,
    Closed,
}

/// ICE gathering progress, named as libdatachannel names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GatheringState {
    New,
    InProgress,
    Complete,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TransportPolicy {
    #[default]
    All,
    Relay,
}

/// A STUN or TURN server.
#[derive(Clone, Debug, Default)]
pub struct IceServer {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

/// Peer connection configuration.
///
/// Only what a browser can actually honour. libdatachannel's
/// `Configuration` has a good deal more — bind address, port range, DTLS
/// certificate files, fingerprint verification — none of which any
/// browser exposes. `datachannel-facade` keeps those in a native-only
/// extension trait rather than offering them here and ignoring them.
#[derive(Clone, Debug, Default)]
pub struct Configuration {
    pub ice_servers: Vec<IceServer>,
    pub ice_transport_policy: TransportPolicy,
}

/// Reliability settings for a data channel, in libdatachannel's shape.
#[derive(Clone, Copy, Debug, Default)]
pub struct Reliability {
    pub unordered: bool,
    pub unreliable: bool,
    pub max_packet_life_time: u32,
    pub max_retransmits: u32,
}

#[derive(Clone, Debug, Default)]
pub struct DataChannelOptions {
    pub reliability: Reliability,
    pub protocol: String,
    pub negotiated: bool,
    /// Pre-agreed stream id. Required when `negotiated` is set.
    pub stream: Option<u16>,
}

/// A caller-supplied callback, shared with the spawned tasks that
/// outlive the call which started them.
type Shared<T> = std::rc::Rc<std::cell::RefCell<Option<Box<T>>>>;

/// A JS event handler, kept alive for as long as the object it is
/// attached to: a `wasm_bindgen` `Closure` unregisters itself when
/// dropped, so dropping one silently stops the events.
type Handler<T> = wasm_bindgen::closure::Closure<dyn FnMut(T)>;

/// Handlers with nothing useful in the event argument.
type PlainHandler = Handler<wasm_bindgen::JsValue>;

fn handler<T: wasm_bindgen::convert::FromWasmAbi + 'static>(f: impl FnMut(T) + 'static) -> Handler<T> {
    // (nettai's copy: `wrap` asks an `UnwindSafe` closure of a build with
    // `-C panic=unwind`, which nettai's wasm build is, and a boxed `dyn
    // FnMut` isn't one; the facade's `Closure::wrap` doesn't compile there.)
    wasm_bindgen::closure::Closure::wrap_assert_unwind_safe(Box::new(f) as Box<dyn FnMut(T)>)
}

/// Attach `h` by handing its JS function to `set`, then keep it.
fn attach<T>(set: impl FnOnce(Option<&js_sys::Function>), h: Option<Handler<T>>) -> Option<Handler<T>> {
    set(h.as_ref().map(|h| h.as_ref().unchecked_ref()));
    h
}

#[derive(Default)]
struct PeerCallbacks {
    on_ice_candidate: Option<Handler<web_sys::RtcPeerConnectionIceEvent>>,
    on_state_change: Option<PlainHandler>,
    on_gathering_state_change: Option<PlainHandler>,
    on_data_channel: Option<Handler<web_sys::RtcDataChannelEvent>>,
}

pub struct PeerConnection {
    pc: web_sys::RtcPeerConnection,
    callbacks: std::rc::Rc<std::cell::RefCell<PeerCallbacks>>,
    /// Where a rejected Promise reports to. Shared with the spawned
    /// tasks, which outlive the call that started them.
    on_state_change: Shared<dyn Fn(State)>,
    /// Set by `set_local_description`, since the browser has no
    /// equivalent of libdatachannel's `on_local_description`.
    on_local_description: Shared<dyn Fn(&str, SdpType)>,
}

impl PeerConnection {
    pub fn new(config: Configuration) -> Result<Self, Error> {
        let raw = web_sys::RtcConfiguration::new();
        let servers = js_sys::Array::new();
        for server in &config.ice_servers {
            let obj = js_sys::Object::new();
            let urls = js_sys::Array::new();
            for url in &server.urls {
                urls.push(&wasm_bindgen::JsValue::from_str(url));
            }
            set(&obj, "urls", &urls);
            if let Some(username) = &server.username {
                set(&obj, "username", &wasm_bindgen::JsValue::from_str(username));
            }
            if let Some(credential) = &server.credential {
                set(&obj, "credential", &wasm_bindgen::JsValue::from_str(credential));
            }
            servers.push(&obj);
        }
        raw.set_ice_servers(&servers);
        raw.set_ice_transport_policy(match config.ice_transport_policy {
            TransportPolicy::All => web_sys::RtcIceTransportPolicy::All,
            TransportPolicy::Relay => web_sys::RtcIceTransportPolicy::Relay,
        });

        Ok(Self {
            pc: web_sys::RtcPeerConnection::new_with_configuration(&raw)?,
            callbacks: Default::default(),
            on_state_change: Default::default(),
            on_local_description: Default::default(),
        })
    }

    pub fn close(&self) -> Result<(), Error> {
        self.pc.close();
        Ok(())
    }

    /// Set the local description, generating it as libdatachannel does.
    ///
    /// Returns as soon as the work is queued; the resulting SDP arrives
    /// through [`Self::set_on_local_description`], the way
    /// libdatachannel delivers it.
    ///
    /// The browser has no argument-less `setLocalDescription`, so the
    /// description is created explicitly — which is what libdatachannel
    /// does internally anyway. `None` picks offer or answer from the
    /// current signaling state, the same rule the implicit form uses.
    pub fn set_local_description(&self, type_: Option<SdpType>) -> Result<(), Error> {
        let pc = self.pc.clone();
        let on_local_description = self.on_local_description.clone();
        let on_state_change = self.on_state_change.clone();

        wasm_bindgen_futures::spawn_local(async move {
            let type_ = type_.unwrap_or_else(|| match pc.signaling_state() {
                // Mid-negotiation with their offer in hand: we answer.
                web_sys::RtcSignalingState::HaveRemoteOffer | web_sys::RtcSignalingState::HaveLocalPranswer => {
                    SdpType::Answer
                }
                _ => SdpType::Offer,
            });

            let init = match type_ {
                // Rollback discards the pending local description; there
                // is nothing to create.
                SdpType::Rollback => web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Rollback),
                SdpType::Answer | SdpType::Pranswer => {
                    match wasm_bindgen_futures::JsFuture::from(pc.create_answer()).await {
                        Ok(v) => v.unchecked_into(),
                        Err(e) => return fail(&on_state_change, "createAnswer", e),
                    }
                }
                SdpType::Offer => match wasm_bindgen_futures::JsFuture::from(pc.create_offer()).await {
                    Ok(v) => v.unchecked_into(),
                    Err(e) => return fail(&on_state_change, "createOffer", e),
                },
            };

            if let Err(e) = wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&init)).await {
                return fail(&on_state_change, "setLocalDescription", e);
            }

            // Rollback leaves no local description to report, and
            // libdatachannel reports nothing for it either.
            let Some(desc) = pc.local_description() else {
                return;
            };
            let Some(type_) = SdpType::from_web(desc.type_()) else {
                return;
            };
            if let Some(cb) = on_local_description.borrow().as_ref() {
                cb(&desc.sdp(), type_);
            }
        });
        Ok(())
    }

    /// Set the remote description. Returns as soon as the work is
    /// queued; a rejection drives the connection to [`State::Failed`].
    pub fn set_remote_description(&self, desc: &Description) -> Result<(), Error> {
        let pc = self.pc.clone();
        let on_state_change = self.on_state_change.clone();
        let init = web_sys::RtcSessionDescriptionInit::new(desc.type_.to_web());
        init.set_sdp(&desc.sdp);
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(e) = wasm_bindgen_futures::JsFuture::from(pc.set_remote_description(&init)).await {
                fail(&on_state_change, "setRemoteDescription", e);
            }
        });
        Ok(())
    }

    /// Add a remote ICE candidate. An empty string means end-of-
    /// candidates, which the browser spells as a null candidate.
    ///
    /// The browser refuses a candidate that names no media section —
    /// `RTCIceCandidate` throws when both `sdpMid` and `sdpMLineIndex`
    /// are null — while libdatachannel's candidate strings carry the mid
    /// alongside, out of band, and this API doesn't. Everything here is a
    /// bundled data-channel connection, which has exactly one media
    /// section, so that section is index 0.
    pub fn add_remote_candidate(&self, cand: &str) -> Result<(), Error> {
        let pc = self.pc.clone();
        let on_state_change = self.on_state_change.clone();
        let candidate = if cand.is_empty() {
            None
        } else {
            let init = web_sys::RtcIceCandidateInit::new(cand);
            init.set_sdp_m_line_index(Some(0));
            Some(web_sys::RtcIceCandidate::new(&init)?)
        };
        wasm_bindgen_futures::spawn_local(async move {
            let promise = pc.add_ice_candidate_with_opt_rtc_ice_candidate(candidate.as_ref());
            if let Err(e) = wasm_bindgen_futures::JsFuture::from(promise).await {
                fail(&on_state_change, "addIceCandidate", e);
            }
        });
        Ok(())
    }

    pub fn local_description(&self) -> Result<Description, Error> {
        description(self.pc.local_description())
    }

    pub fn remote_description(&self) -> Result<Description, Error> {
        description(self.pc.remote_description())
    }

    pub fn create_data_channel(&self, label: &str, options: DataChannelOptions) -> Result<DataChannel, Error> {
        let init = web_sys::RtcDataChannelInit::new();
        init.set_ordered(!options.reliability.unordered);
        // The browser accepts at most one of these, and only for an
        // unreliable channel; libdatachannel takes both and ignores them
        // when reliable. maxPacketLifeTime wins if both are set, matching
        // libdatachannel's own precedence.
        if options.reliability.unreliable {
            if options.reliability.max_packet_life_time > 0 {
                init.set_max_packet_life_time(options.reliability.max_packet_life_time as u16);
            } else {
                init.set_max_retransmits(options.reliability.max_retransmits as u16);
            }
        }
        if !options.protocol.is_empty() {
            init.set_protocol(&options.protocol);
        }
        init.set_negotiated(options.negotiated);
        if let Some(stream) = options.stream {
            init.set_id(stream);
        }
        Ok(DataChannel::wrap(
            self.pc.create_data_channel_with_data_channel_dict(label, &init),
        ))
    }

    pub fn set_on_local_description(&mut self, cb: Option<impl Fn(&str, SdpType) + 'static>) {
        *self.on_local_description.borrow_mut() = cb.map(|cb| Box::new(cb) as Box<dyn Fn(&str, SdpType)>);
    }

    pub fn set_on_local_candidate(&mut self, cb: Option<impl Fn(&str) + 'static>) {
        let h = cb.map(|cb| {
            handler(move |ev: web_sys::RtcPeerConnectionIceEvent| {
                // A null candidate is the browser's end-of-gathering
                // marker; libdatachannel signals the same with an empty
                // string.
                cb(&ev.candidate().map(|c| c.candidate()).unwrap_or_default());
            })
        });
        let pc = self.pc.clone();
        self.callbacks.borrow_mut().on_ice_candidate = attach(|f| pc.set_onicecandidate(f), h);
    }

    pub fn set_on_state_change(&mut self, cb: Option<impl Fn(State) + 'static>) {
        // Held twice: the browser's own event reads the live state, and
        // a rejected Promise reports a failure through the same callback
        // (see `fail`).
        let shared = cb.map(|cb| std::rc::Rc::new(cb));
        *self.on_state_change.borrow_mut() = shared.clone().map(|cb| Box::new(move |s| cb(s)) as Box<dyn Fn(State)>);

        let h = shared.map(|cb| {
            let pc = self.pc.clone();
            handler(move |_: wasm_bindgen::JsValue| cb(state_from_web(pc.connection_state())))
        });
        let pc = self.pc.clone();
        self.callbacks.borrow_mut().on_state_change = attach(|f| pc.set_onconnectionstatechange(f), h);
    }

    pub fn set_on_gathering_state_change(&mut self, cb: Option<impl Fn(GatheringState) + 'static>) {
        let h = cb.map(|cb| {
            let pc = self.pc.clone();
            handler(move |_: wasm_bindgen::JsValue| {
                cb(match pc.ice_gathering_state() {
                    web_sys::RtcIceGatheringState::New => GatheringState::New,
                    web_sys::RtcIceGatheringState::Gathering => GatheringState::InProgress,
                    _ => GatheringState::Complete,
                })
            })
        });
        let pc = self.pc.clone();
        self.callbacks.borrow_mut().on_gathering_state_change = attach(|f| pc.set_onicegatheringstatechange(f), h);
    }

    pub fn set_on_data_channel(&mut self, cb: Option<impl Fn(DataChannel) + 'static>) {
        let h = cb.map(|cb| handler(move |ev: web_sys::RtcDataChannelEvent| cb(DataChannel::wrap(ev.channel()))));
        let pc = self.pc.clone();
        self.callbacks.borrow_mut().on_data_channel = attach(|f| pc.set_ondatachannel(f), h);
    }
}

impl Drop for PeerConnection {
    fn drop(&mut self) {
        self.pc.close();
    }
}

#[derive(Default)]
struct ChannelCallbacks {
    on_open: Option<PlainHandler>,
    on_closed: Option<PlainHandler>,
    on_error: Option<Handler<web_sys::Event>>,
    on_message: Option<Handler<web_sys::MessageEvent>>,
    on_buffered_amount_low: Option<PlainHandler>,
}

pub struct DataChannel {
    dc: web_sys::RtcDataChannel,
    callbacks: std::rc::Rc<std::cell::RefCell<ChannelCallbacks>>,
}

impl DataChannel {
    fn wrap(dc: web_sys::RtcDataChannel) -> Self {
        // Messages are bytes here, never strings. Without this the
        // browser hands back a Blob, whose contents are only readable
        // asynchronously — which would break the synchronous
        // `on_message(&[u8])` contract libdatachannel sets.
        dc.set_binary_type(web_sys::RtcDataChannelType::Arraybuffer);
        Self {
            dc,
            callbacks: Default::default(),
        }
    }

    pub fn send(&self, buf: &[u8]) -> Result<(), Error> {
        // Not `send_with_u8_array(buf)`: wasm-bindgen passes a `&[u8]`
        // as a view on the module's own memory, and in a build with
        // shared memory (threads — tango-lite-web with the DS games on)
        // `RTCDataChannel.send` refuses shared views outright. Copying
        // into a fresh JS-side buffer works on both kinds of memory,
        // and a wire packet is small.
        let copy = js_sys::Uint8Array::new_with_length(buf.len() as u32);
        copy.copy_from(buf);
        Ok(self.dc.send_with_array_buffer(&copy.buffer())?)
    }

    pub fn close(&self) -> Result<(), Error> {
        self.dc.close();
        Ok(())
    }

    pub fn is_open(&self) -> bool {
        self.dc.ready_state() == web_sys::RtcDataChannelState::Open
    }

    pub fn is_closed(&self) -> bool {
        self.dc.ready_state() == web_sys::RtcDataChannelState::Closed
    }

    pub fn buffered_amount(&self) -> Result<usize, Error> {
        Ok(self.dc.buffered_amount() as usize)
    }

    pub fn set_buffered_amount_low_threshold(&self, amount: usize) -> Result<(), Error> {
        self.dc.set_buffered_amount_low_threshold(amount as u32);
        Ok(())
    }

    pub fn set_on_open(&mut self, cb: Option<impl Fn() + 'static>) {
        let h = cb.map(|cb| handler(move |_: wasm_bindgen::JsValue| cb()));
        let dc = self.dc.clone();
        self.callbacks.borrow_mut().on_open = attach(|f| dc.set_onopen(f), h);
    }

    pub fn set_on_closed(&mut self, cb: Option<impl Fn() + 'static>) {
        let h = cb.map(|cb| handler(move |_: wasm_bindgen::JsValue| cb()));
        let dc = self.dc.clone();
        self.callbacks.borrow_mut().on_closed = attach(|f| dc.set_onclose(f), h);
    }

    pub fn set_on_error(&mut self, cb: Option<impl Fn(&str) + 'static>) {
        let h = cb.map(|cb| {
            handler(move |ev: web_sys::Event| {
                let msg = ev
                    .dyn_ref::<web_sys::ErrorEvent>()
                    .map(|e| e.message())
                    .unwrap_or_else(|| "data channel error".to_owned());
                cb(&msg);
            })
        });
        let dc = self.dc.clone();
        self.callbacks.borrow_mut().on_error = attach(|f| dc.set_onerror(f), h);
    }

    pub fn set_on_message(&mut self, cb: Option<impl Fn(&[u8]) + 'static>) {
        let h = cb.map(|cb| {
            handler(move |ev: web_sys::MessageEvent| {
                // `binaryType` is pinned to arraybuffer in `wrap`, so
                // this is the only shape that arrives.
                let Ok(buf) = ev.data().dyn_into::<js_sys::ArrayBuffer>() else {
                    log::warn!("data channel: ignoring non-binary message");
                    return;
                };
                cb(&js_sys::Uint8Array::new(&buf).to_vec());
            })
        });
        let dc = self.dc.clone();
        self.callbacks.borrow_mut().on_message = attach(|f| dc.set_onmessage(f), h);
    }

    pub fn set_on_buffered_amount_low(&mut self, cb: Option<impl Fn() + 'static>) {
        let h = cb.map(|cb| handler(move |_: wasm_bindgen::JsValue| cb()));
        let dc = self.dc.clone();
        self.callbacks.borrow_mut().on_buffered_amount_low = attach(|f| dc.set_onbufferedamountlow(f), h);
    }
}

impl Drop for DataChannel {
    fn drop(&mut self) {
        self.dc.close();
    }
}

fn set(obj: &js_sys::Object, key: &str, value: &wasm_bindgen::JsValue) {
    let _ = js_sys::Reflect::set(obj, &wasm_bindgen::JsValue::from_str(key), value);
}

fn description(desc: Option<web_sys::RtcSessionDescription>) -> Result<Description, Error> {
    let desc = desc.ok_or_else(|| Error("no description".to_owned()))?;
    let type_ = SdpType::from_web(desc.type_()).ok_or_else(|| Error("unknown sdp type".to_owned()))?;
    Ok(Description { type_, sdp: desc.sdp() })
}

fn state_from_web(state: web_sys::RtcPeerConnectionState) -> State {
    match state {
        web_sys::RtcPeerConnectionState::Connected => State::Connected,
        web_sys::RtcPeerConnectionState::Disconnected => State::Disconnected,
        web_sys::RtcPeerConnectionState::Failed => State::Failed,
        web_sys::RtcPeerConnectionState::Closed => State::Closed,
        // `new` and `connecting` both mean "not up yet"; libdatachannel
        // draws no distinction.
        _ => State::Connecting,
    }
}

/// Report a rejected Promise. There is nowhere to return it to, so it is
/// logged and the connection is driven to [`State::Failed`] — which is
/// what a failed description or candidate exchange amounts to.
fn fail(on_state_change: &Shared<dyn Fn(State)>, what: &str, e: wasm_bindgen::JsValue) {
    log::error!("{what} failed: {}", Error::from(e));
    if let Some(cb) = on_state_change.borrow().as_ref() {
        cb(State::Failed);
    }
}
