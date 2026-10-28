//! The web's WebRTC connection: the browser's `RTCPeerConnection`, through
//! web-datachannel, behind `crate::rtc`'s API. The browser says what
//! happened through callbacks, on the page's one thread; each puts its
//! [`Event`] in the connection's queue, which [`PeerConnection::poll`] takes
//! from.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::rtc::{ChannelId, ChannelOptions, Configuration, Error, Event, SdpType, State};

type Queue = Rc<RefCell<VecDeque<Event>>>;

pub(crate) struct PeerConnection {
    // (The channels before the connection: they are dropped first.)
    channels: Vec<web_datachannel::DataChannel>,
    pc: web_datachannel::PeerConnection,
    events: Queue,
}

fn err(e: web_datachannel::Error) -> Error {
    Error::new(e.to_string())
}

impl PeerConnection {
    pub(crate) fn new(config: Configuration) -> Result<PeerConnection, Error> {
        let ice_servers = config
            .ice_servers
            .iter()
            .map(|s| {
                let credentials = !s.username.is_empty();
                web_datachannel::IceServer { urls: s.urls.clone(), username: credentials.then(|| s.username.clone()), credential: credentials.then(|| s.credential.clone()) }
            })
            .collect();
        let ice_transport_policy = if config.relay_only { web_datachannel::TransportPolicy::Relay } else { web_datachannel::TransportPolicy::All };
        let mut pc = web_datachannel::PeerConnection::new(web_datachannel::Configuration { ice_servers, ice_transport_policy }).map_err(err)?;
        let events: Queue = Queue::default();
        let q = events.clone();
        pc.set_on_local_description(Some(move |sdp: &str, _| q.borrow_mut().push_back(Event::Description(sdp.to_string()))));
        let q = events.clone();
        pc.set_on_local_candidate(Some(move |line: &str| {
            // (The end of gathering: nothing to say.)
            if !line.is_empty() {
                q.borrow_mut().push_back(Event::Candidate(line.to_string()));
            }
        }));
        let q = events.clone();
        pc.set_on_state_change(Some(move |state| {
            let state = match state {
                web_datachannel::State::Connecting => State::Connecting,
                web_datachannel::State::Connected => State::Connected,
                web_datachannel::State::Disconnected => State::Disconnected,
                web_datachannel::State::Failed => State::Failed,
                web_datachannel::State::Closed => State::Closed,
            };
            q.borrow_mut().push_back(Event::State(state));
        }));
        Ok(PeerConnection { channels: Vec::new(), pc, events })
    }

    pub(crate) fn create_channel(&mut self, label: &str, options: ChannelOptions) -> Result<ChannelId, Error> {
        let reliability = web_datachannel::Reliability {
            unordered: !options.ordered,
            unreliable: options.max_retransmits.is_some(),
            max_retransmits: options.max_retransmits.unwrap_or(0).into(),
            ..web_datachannel::Reliability::default()
        };
        let options = web_datachannel::DataChannelOptions { reliability, negotiated: true, stream: Some(options.stream), ..web_datachannel::DataChannelOptions::default() };
        let mut channel = self.pc.create_data_channel(label, options).map_err(err)?;
        let id = ChannelId(self.channels.len());
        let q = self.events.clone();
        channel.set_on_open(Some(move || q.borrow_mut().push_back(Event::Open(id))));
        let q = self.events.clone();
        channel.set_on_closed(Some(move || q.borrow_mut().push_back(Event::Closed(id))));
        let q = self.events.clone();
        channel.set_on_message(Some(move |data: &[u8]| q.borrow_mut().push_back(Event::Message(id, data.to_vec()))));
        self.channels.push(channel);
        Ok(id)
    }

    pub(crate) fn set_local_description(&mut self, type_: SdpType) -> Result<(), Error> {
        self.pc.set_local_description(Some(web_type(type_))).map_err(err)
    }

    pub(crate) fn set_remote_description(&mut self, type_: SdpType, sdp: &str) -> Result<(), Error> {
        self.pc.set_remote_description(&web_datachannel::Description { type_: web_type(type_), sdp: sdp.to_string() }).map_err(err)
    }

    pub(crate) fn add_remote_candidate(&mut self, candidate: &str) -> Result<(), Error> {
        self.pc.add_remote_candidate(candidate).map_err(err)
    }

    pub(crate) fn send(&mut self, channel: ChannelId, data: &[u8]) -> Result<(), Error> {
        self.channels[channel.0].send(data).map_err(err)
    }

    pub(crate) fn close(&mut self) {
        for c in &self.channels {
            let _ = c.close();
        }
        let _ = self.pc.close();
    }

    pub(crate) fn poll(&mut self) -> Option<Event> {
        self.events.borrow_mut().pop_front()
    }
}

fn web_type(type_: SdpType) -> web_datachannel::SdpType {
    match type_ {
        SdpType::Offer => web_datachannel::SdpType::Offer,
        SdpType::Answer => web_datachannel::SdpType::Answer,
    }
}
