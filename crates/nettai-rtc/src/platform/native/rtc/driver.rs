//! A connection's state, the calls on it, and the task that drives it
//! (the module's docs).
//!
//! str0m's one rule: every change to it (a datagram, a timeout, a send, a
//! description, a candidate) is followed by taking all its output, up to its
//! next timeout, before the next change ([`Inner::drain`]).

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use str0m::change::{SdpAnswer, SdpOffer, SdpPendingOffer};
use str0m::channel::{ChannelConfig, ChannelId as Str0mChannel, Reliability};
use str0m::net::{Protocol, Receive};
use str0m::config::Fingerprint;
use str0m::{Candidate, Event as Str0mEvent, IceConnectionState, IceCreds, Input, Output, Rtc};

use super::gather::{Came, Found, Io, Sock};
use crate::rtc::{ChannelId, ChannelOptions, Configuration, Error, Event, SdpType, State};

/// Nothing due: the far future.
const NEVER: Duration = Duration::from_secs(3600);

/// The longest an ICE check waits before it is sent again (and how often a
/// pair in use is checked).
const CHECK: Duration = Duration::from_millis(500);

/// A data channel: str0m's id once str0m has it (with this side's
/// description).
struct Channel {
    config: ChannelConfig,
    id: Option<Str0mChannel>,
    open: bool,
}

enum Negotiation {
    /// No description yet.
    Idle,
    /// This side offered; the other side's answer is awaited.
    Offered(SdpPendingOffer),
    /// The other side offered; this side's answer is to be made.
    Offer(SdpOffer),
    /// Both descriptions in.
    Done,
}

struct Inner {
    rtc: Rtc,
    io: Io,
    channels: Vec<Channel>,
    negotiation: Negotiation,
    /// The other side's description is in str0m: its candidates go there
    /// at once (and before it, wait in `early`).
    remote: bool,
    early: Vec<Candidate>,
    /// This side's description is said: its candidates may be (and before
    /// it, wait in `unsaid`).
    described: bool,
    unsaid: Vec<String>,
    events: VecDeque<Event>,
    /// Names for the task to resolve.
    resolve: Vec<(usize, String, u16)>,
    /// str0m's next timeout.
    next: Instant,
    /// Until when the task is waiting: a sooner deadline wakes it.
    waiting_until: Option<Instant>,
    state: Option<State>,
    closed: bool,
    /// str0m said an error: nothing more is asked of it.
    dead: bool,
}

pub(super) struct Shared {
    inner: Mutex<Inner>,
    wake: tokio::sync::Notify,
}

impl Shared {
    /// A connection on `config`, its task on `runtime`.
    pub(super) fn new(config: Configuration, runtime: &tokio::runtime::Handle) -> Result<Arc<Shared>, Error> {
        let _context = runtime.enter();
        let (io, found) = Io::new(&config)?;
        let now = Instant::now();
        let mut builder = Rtc::builder().set_fingerprint_verification(!config.system.skip_fingerprint_check);
        if let Some((ufrag, pass)) = config.system.ice_credentials {
            builder = builder.set_local_ice_credentials(IceCreds { ufrag, pass });
        }
        // ICE checks every half second at most (str0m's default backs off to
        // 3 s), for `checks_for` (9 checks at least, str0m's default).
        builder.set_max_stun_rto(CHECK);
        builder.set_max_stun_retransmits(((config.system.checks_for.as_millis() / CHECK.as_millis()) as usize).max(9));
        let socks = io.socks.clone();
        let mut inner = Inner {
            rtc: builder.build(now),
            io,
            channels: Vec::new(),
            negotiation: Negotiation::Idle,
            remote: false,
            early: Vec::new(),
            described: false,
            unsaid: Vec::new(),
            events: VecDeque::new(),
            resolve: Vec::new(),
            next: now,
            waiting_until: None,
            state: None,
            closed: false,
            dead: false,
        };
        inner.drain();
        inner.found(found);
        let shared = Arc::new(Shared { inner: Mutex::new(inner), wake: tokio::sync::Notify::new() });
        runtime.spawn(drive(shared.clone(), socks));
        Ok(shared)
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// After a call's change: wake the task if it has names to resolve, a
    /// sooner deadline, or the connection closed.
    fn changed(&self, inner: MutexGuard<'_, Inner>) {
        let wake = !inner.resolve.is_empty() || inner.closed || inner.waiting_until.is_some_and(|w| inner.deadline() < w);
        drop(inner);
        if wake {
            self.wake.notify_one();
        }
    }

    pub(super) fn poll(&self) -> Option<Event> {
        self.lock().events.pop_front()
    }

    pub(super) fn create_channel(&self, label: &str, options: ChannelOptions) -> Result<ChannelId, Error> {
        let mut s = self.lock();
        if s.described || !matches!(s.negotiation, Negotiation::Idle | Negotiation::Offer(_)) {
            return Err(Error::new("a channel is made before this side's description"));
        }
        let reliability = match options.max_retransmits {
            Some(retransmits) => Reliability::MaxRetransmits { retransmits },
            None => Reliability::Reliable,
        };
        let config = ChannelConfig { label: label.to_string(), ordered: options.ordered, reliability, negotiated: Some(options.stream), protocol: String::new() };
        s.channels.push(Channel { config, id: None, open: false });
        Ok(ChannelId(s.channels.len() - 1))
    }

    pub(super) fn set_local_description(&self, type_: SdpType) -> Result<(), Error> {
        let mut s = self.lock();
        if s.closed {
            return Err(Error::new("the connection is closed"));
        }
        let sdp = match type_ {
            SdpType::Offer => s.offer()?,
            SdpType::Answer => s.answer()?,
        };
        s.drain();
        s.events.push_back(Event::Description(strip_candidates(&sdp)));
        s.described = true;
        for line in std::mem::take(&mut s.unsaid) {
            s.events.push_back(Event::Candidate(line));
        }
        let mut found = Vec::new();
        s.resolve = s.io.start(Instant::now(), &mut found);
        s.found(found);
        self.changed(s);
        Ok(())
    }

    pub(super) fn set_remote_description(&self, type_: SdpType, sdp: &str) -> Result<(), Error> {
        let mut s = self.lock();
        if s.closed {
            return Err(Error::new("the connection is closed"));
        }
        match type_ {
            SdpType::Offer => {
                if !matches!(s.negotiation, Negotiation::Idle) {
                    return Err(Error::new("an offer comes first"));
                }
                let offer = SdpOffer::from_sdp_string(sdp).map_err(|e| Error::new(format!("the offer: {e}")))?;
                // (Taken when the answer is made.)
                s.negotiation = Negotiation::Offer(offer);
            }
            SdpType::Answer => {
                let Negotiation::Offered(pending) = std::mem::replace(&mut s.negotiation, Negotiation::Done) else {
                    return Err(Error::new("an answer needs this side's offer"));
                };
                let answer = SdpAnswer::from_sdp_string(sdp).map_err(|e| Error::new(format!("the answer: {e}")))?;
                s.rtc.sdp_api().accept_answer(pending, answer).map_err(|e| Error::new(format!("can't take the answer: {e}")))?;
                s.drain();
                s.remote_in();
            }
        }
        self.changed(s);
        Ok(())
    }

    /// Start without descriptions (direct connect): the other side's ICE
    /// credentials known; this side ICE's controlling side, the DTLS client
    /// and the SCTP client if `dialing`. (The other side's certificate is
    /// unknown: its check is off, and str0m is given a stand-in.)
    pub(super) fn start_direct(&self, ufrag: &str, pwd: &str, dialing: bool) -> Result<(), Error> {
        let mut s = self.lock();
        if s.closed || !matches!(s.negotiation, Negotiation::Idle) {
            return Err(Error::new("a direct start comes first"));
        }
        s.start_direct(ufrag, pwd, dialing)?;
        self.changed(s);
        Ok(())
    }

    pub(super) fn add_remote_candidate(&self, line: &str) -> Result<(), Error> {
        let line = line.trim();
        // (The end of the other side's candidates: nothing to do.)
        if line.is_empty() {
            return Ok(());
        }
        let candidate = match Candidate::from_sdp_string(line.strip_prefix("a=").unwrap_or(line)) {
            Ok(c) => c,
            // (One str0m can't read is one it couldn't reach.)
            Err(e) => {
                log::info!("a candidate left out ({e}): {line}");
                return Ok(());
            }
        };
        let mut s = self.lock();
        if s.remote {
            s.add_remote(candidate);
        } else {
            s.early.push(candidate);
        }
        self.changed(s);
        Ok(())
    }

    pub(super) fn send(&self, channel: ChannelId, data: &[u8]) -> Result<(), Error> {
        let mut s = self.lock();
        let c = &s.channels[channel.0];
        let Some(id) = c.id.filter(|_| c.open && !s.closed && !s.dead) else {
            return Err(Error::new("the channel isn't open"));
        };
        let written = match s.rtc.channel(id) {
            Some(mut c) => c.write(true, data),
            None => return Err(Error::new("the channel isn't open")),
        };
        s.drain();
        self.changed(s);
        match written {
            Ok(true) => Ok(()),
            Ok(false) => Err(Error::new("the channel's buffer is full")),
            Err(e) => Err(Error::new(format!("can't send: {e}"))),
        }
    }

    pub(super) fn close(&self) {
        let mut s = self.lock();
        if s.closed {
            return;
        }
        s.closed = true;
        if !s.dead {
            s.rtc.disconnect();
            s.drain();
        }
        for i in 0..s.channels.len() {
            if std::mem::take(&mut s.channels[i].open) {
                s.events.push_back(Event::Closed(ChannelId(i)));
            }
        }
        s.set_state(State::Closed);
        self.changed(s);
    }
}

impl Inner {
    /// [`Shared::start_direct`].
    fn start_direct(&mut self, ufrag: &str, pwd: &str, dialing: bool) -> Result<(), Error> {
        let mut api = self.rtc.direct_api();
        api.set_ice_controlling(dialing);
        api.set_remote_ice_credentials(IceCreds { ufrag: ufrag.to_string(), pass: pwd.to_string() });
        api.set_remote_fingerprint(Fingerprint { hash_func: "sha-256".into(), bytes: vec![0; 32] });
        api.start_dtls(dialing).map_err(|e| Error::new(format!("can't start DTLS: {e}")))?;
        api.start_sctp(dialing);
        self.drain();
        for i in 0..self.channels.len() {
            let id = self.rtc.direct_api().create_data_channel(self.channels[i].config.clone());
            self.channels[i].id = Some(id);
            self.drain();
        }
        self.negotiation = Negotiation::Done;
        // (No candidate of this side's is said to anyone.)
        self.described = true;
        self.unsaid.clear();
        self.remote_in();
        Ok(())
    }

    /// This side's offer, its channels in it.
    fn offer(&mut self) -> Result<String, Error> {
        if !matches!(self.negotiation, Negotiation::Idle) {
            return Err(Error::new("an offer is made first"));
        }
        let configs: Vec<ChannelConfig> = self.channels.iter().map(|c| c.config.clone()).collect();
        let mut api = self.rtc.sdp_api();
        let ids: Vec<Str0mChannel> = configs.into_iter().map(|c| api.add_channel_with_config(c)).collect();
        let Some((offer, pending)) = api.apply() else { return Err(Error::new("nothing to offer: no channel")) };
        for (c, id) in self.channels.iter_mut().zip(ids) {
            c.id = Some(id);
        }
        self.negotiation = Negotiation::Offered(pending);
        Ok(offer.to_sdp_string())
    }

    /// This side's answer to the other's offer, its channels made.
    fn answer(&mut self) -> Result<String, Error> {
        let Negotiation::Offer(offer) = std::mem::replace(&mut self.negotiation, Negotiation::Done) else {
            return Err(Error::new("an answer needs the other side's offer"));
        };
        let answer = self.rtc.sdp_api().accept_offer(offer).map_err(|e| Error::new(format!("can't take the offer: {e}")))?;
        self.drain();
        for i in 0..self.channels.len() {
            let id = self.rtc.direct_api().create_data_channel(self.channels[i].config.clone());
            self.channels[i].id = Some(id);
            self.drain();
        }
        self.remote_in();
        Ok(answer.to_sdp_string())
    }

    /// The other side's description is in str0m: the candidates that
    /// waited for it go there.
    fn remote_in(&mut self) {
        self.remote = true;
        for c in std::mem::take(&mut self.early) {
            self.add_remote(c);
        }
    }

    fn add_remote(&mut self, candidate: Candidate) {
        let ip = candidate.addr().ip();
        self.rtc.add_remote_candidate(candidate);
        self.drain();
        let mut found = Vec::new();
        self.io.permit(ip, Instant::now(), &mut found);
        self.found(found);
    }

    /// Candidates found: each into str0m, and said (once this side's
    /// description is) if it is one to say.
    fn found(&mut self, found: Vec<Found>) {
        for f in found {
            if self.dead {
                return;
            }
            let (candidate, say) = match f {
                Found::Said(c) => (c, true),
                Found::Unsaid(c) => (c, false),
            };
            let Some(line) = self.rtc.add_local_candidate(candidate).map(|c| c.to_sdp_string()) else { continue };
            self.drain();
            match (say, self.described) {
                (false, _) => {}
                (true, true) => self.events.push_back(Event::Candidate(line)),
                (true, false) => self.unsaid.push(line),
            }
        }
    }

    /// Take str0m's output up to its next timeout: datagrams sent, events
    /// queued.
    fn drain(&mut self) {
        let mut found = Vec::new();
        while !self.dead {
            match self.rtc.poll_output() {
                Ok(Output::Timeout(t)) => {
                    self.next = t;
                    break;
                }
                Ok(Output::Transmit(t)) => {
                    if t.proto == Protocol::Udp {
                        self.io.send(t.source, t.destination, &t.contents, Instant::now(), &mut found);
                    }
                }
                Ok(Output::Event(e)) => self.event(e),
                Err(e) => {
                    log::warn!("the connection broke: {e}");
                    self.dead = true;
                    self.set_state(State::Failed);
                }
            }
        }
        if !found.is_empty() {
            self.found(found);
        }
    }

    fn event(&mut self, event: Str0mEvent) {
        match event {
            Str0mEvent::IceConnectionStateChange(IceConnectionState::Checking) => self.set_state(State::Connecting),
            Str0mEvent::IceConnectionStateChange(IceConnectionState::Disconnected) => self.set_state(State::Disconnected),
            // (Back from Disconnected.)
            Str0mEvent::IceConnectionStateChange(IceConnectionState::Connected | IceConnectionState::Completed)
                if self.state == Some(State::Disconnected) =>
            {
                self.set_state(State::Connected)
            }
            Str0mEvent::Connected => self.set_state(State::Connected),
            Str0mEvent::ChannelOpen(id, _) => {
                if let Some(i) = self.channel(id)
                    && !std::mem::replace(&mut self.channels[i].open, true)
                {
                    self.events.push_back(Event::Open(ChannelId(i)));
                }
            }
            Str0mEvent::ChannelData(d) => {
                if let Some(i) = self.channel(d.id) {
                    self.events.push_back(Event::Message(ChannelId(i), d.data));
                }
            }
            Str0mEvent::ChannelClose(id) => {
                if let Some(i) = self.channel(id)
                    && std::mem::take(&mut self.channels[i].open)
                {
                    self.events.push_back(Event::Closed(ChannelId(i)));
                }
            }
            Str0mEvent::Closed => self.set_state(State::Closed),
            _ => {}
        }
    }

    fn channel(&self, id: Str0mChannel) -> Option<usize> {
        self.channels.iter().position(|c| c.id == Some(id))
    }

    fn set_state(&mut self, state: State) {
        if self.state != Some(state) {
            self.state = Some(state);
            self.events.push_back(Event::State(state));
        }
    }

    /// When the task is next due: str0m's timeout or a server's.
    fn deadline(&self) -> Instant {
        let next = if self.dead { Instant::now() + NEVER } else { self.next };
        self.io.next().map_or(next, |t| t.min(next))
    }

    /// Every datagram that came on socket `i`.
    fn read(&mut self, i: usize, sock: &Sock, buf: &mut [u8]) {
        sock.recv_all(buf, |from, to, data| self.came(i, from, to, data));
    }

    fn came(&mut self, sock: usize, from: SocketAddr, to: SocketAddr, data: &[u8]) {
        let now = Instant::now();
        let mut found = Vec::new();
        match self.io.came(sock, from, to, data, now, &mut found) {
            Came::Taken => {}
            Came::Relayed { peer, relayed, data } => self.input(now, peer, relayed, &data),
            Came::Direct => self.input(now, from, to, data),
        }
        self.found(found);
    }

    fn input(&mut self, now: Instant, source: SocketAddr, destination: SocketAddr, data: &[u8]) {
        if self.dead {
            return;
        }
        // (What is neither STUN, DTLS nor RTP isn't str0m's.)
        let Ok(contents) = data.try_into() else { return };
        if let Err(e) = self.rtc.handle_input(Input::Receive(now, Receive { proto: Protocol::Udp, source, destination, contents })) {
            log::debug!("a datagram from {source} not taken: {e}");
        }
        self.drain();
    }

    /// What is due at `now`.
    fn due(&mut self, now: Instant) {
        if !self.dead && now >= self.next {
            if let Err(e) = self.rtc.handle_input(Input::Timeout(now)) {
                log::warn!("the connection's timeout: {e}");
            }
            self.drain();
        }
        let mut found = Vec::new();
        self.io.due(now, &mut found);
        self.found(found);
    }

    fn resolved(&mut self, index: usize, addrs: &[SocketAddr]) {
        let found = self.io.resolved(index, addrs, Instant::now());
        self.found(found);
    }
}

/// `sdp` without its candidates: they are said one by one.
fn strip_candidates(sdp: &str) -> String {
    let mut out = String::with_capacity(sdp.len());
    for line in sdp.split("\r\n").flat_map(|l| l.split('\n')) {
        if line.is_empty() || line.starts_with("a=candidate") || line.starts_with("a=end-of-candidates") {
            continue;
        }
        out.push_str(line);
        out.push_str("\r\n");
    }
    out
}

/// Socket `i` readable (never, if there is none).
async fn readable(socks: &[Arc<Sock>], i: usize) {
    match socks.get(i) {
        Some(s) => {
            let _ = s.socket.readable().await;
        }
        None => std::future::pending().await,
    }
}

/// The connection's task: wait for something to do, do it; until the
/// connection is closed, then let its sockets go.
async fn drive(shared: Arc<Shared>, socks: Vec<Arc<Sock>>) {
    let (resolved_tx, mut resolved) = tokio::sync::mpsc::unbounded_channel::<(usize, Vec<SocketAddr>)>();
    let mut buf = vec![0u8; 65536];
    loop {
        let wait = {
            let mut s = shared.lock();
            if s.closed {
                break;
            }
            for (i, host, port) in std::mem::take(&mut s.resolve) {
                let tx = resolved_tx.clone();
                tokio::spawn(async move {
                    let addrs = match tokio::net::lookup_host((host.as_str(), port)).await {
                        Ok(a) => a.collect(),
                        Err(e) => {
                            log::warn!("can't resolve {host}: {e}");
                            Vec::new()
                        }
                    };
                    let _ = tx.send((i, addrs));
                });
            }
            let deadline = s.deadline();
            s.waiting_until = Some(deadline);
            deadline.saturating_duration_since(Instant::now())
        };
        let mut names = None;
        tokio::select! {
            biased;
            _ = shared.wake.notified() => {}
            Some(r) = resolved.recv() => names = Some(r),
            _ = readable(&socks, 0) => {}
            _ = readable(&socks, 1) => {}
            _ = tokio::time::sleep(wait) => {}
        }
        let mut s = shared.lock();
        s.waiting_until = None;
        if let Some((i, addrs)) = names {
            s.resolved(i, &addrs);
        }
        for (i, sock) in socks.iter().enumerate() {
            s.read(i, sock, &mut buf);
        }
        s.due(Instant::now());
    }
    // (The sockets close here: the port is free again.)
    shared.lock().io.socks.clear();
    drop(socks);
}
