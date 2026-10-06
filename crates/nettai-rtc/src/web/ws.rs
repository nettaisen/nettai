//! A WebSocket to the signaling server, the browser's: its callbacks put
//! what happens in a queue, which [`Socket::poll`] takes from (the native
//! backend's API: one socket, one connection).

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{CloseEvent, MessageEvent, WebSocket};

/// What happened on the socket.
#[derive(Debug)]
pub(crate) enum WsEvent {
    Open,
    Text(String),
    /// It closed (or never opened), and why; nothing more comes.
    Closed(String),
}

pub(crate) struct Socket {
    ws: Option<WebSocket>,
    events: Rc<RefCell<VecDeque<WsEvent>>>,
    /// What is said before it opens.
    waiting: Vec<String>,
    open: Rc<RefCell<bool>>,
    _callbacks: Vec<Closure<dyn FnMut(JsValue)>>,
}

impl Socket {
    pub(crate) fn connect(url: &str) -> Socket {
        let events: Rc<RefCell<VecDeque<WsEvent>>> = Rc::default();
        let open = Rc::new(RefCell::new(false));
        let ws = match WebSocket::new(url) {
            Ok(ws) => ws,
            Err(e) => {
                events.borrow_mut().push_back(WsEvent::Closed(format!("can't reach the signaling server: {e:?}")));
                return Socket { ws: None, events, waiting: Vec::new(), open, _callbacks: Vec::new() };
            }
        };
        let mut callbacks: Vec<Closure<dyn FnMut(JsValue)>> = Vec::new();
        let (e, o) = (events.clone(), open.clone());
        callbacks.push(Closure::own_assert_unwind_safe(move |_: JsValue| {
            *o.borrow_mut() = true;
            e.borrow_mut().push_back(WsEvent::Open);
        }));
        ws.set_onopen(Some(callbacks.last().unwrap().as_ref().unchecked_ref()));
        let e = events.clone();
        callbacks.push(Closure::own_assert_unwind_safe(move |m: JsValue| {
            if let Some(text) = m.unchecked_into::<MessageEvent>().data().as_string() {
                e.borrow_mut().push_back(WsEvent::Text(text));
            }
        }));
        ws.set_onmessage(Some(callbacks.last().unwrap().as_ref().unchecked_ref()));
        let (e, o) = (events.clone(), open.clone());
        callbacks.push(Closure::own_assert_unwind_safe(move |c: JsValue| {
            let c = c.unchecked_into::<CloseEvent>();
            let why = if *o.borrow() { format!("the signaling server closed it ({} {})", c.code(), c.reason()) } else { "can't reach the signaling server".into() };
            *o.borrow_mut() = false;
            e.borrow_mut().push_back(WsEvent::Closed(why));
        }));
        ws.set_onclose(Some(callbacks.last().unwrap().as_ref().unchecked_ref()));
        Socket { ws: Some(ws), events, waiting: Vec::new(), open, _callbacks: callbacks }
    }

    /// Send a text message (kept while it opens; lost if it closes).
    pub(crate) fn send(&mut self, text: String) {
        match &self.ws {
            Some(ws) if *self.open.borrow() => {
                let _ = ws.send_with_str(&text);
            }
            Some(_) => self.waiting.push(text),
            None => {}
        }
    }

    /// What happened since, without waiting.
    pub(crate) fn poll(&mut self) -> Option<WsEvent> {
        let event = self.events.borrow_mut().pop_front();
        if let (Some(WsEvent::Open), Some(ws)) = (&event, &self.ws) {
            for text in self.waiting.drain(..) {
                let _ = ws.send_with_str(&text);
            }
        }
        event
    }

    /// Close it, after what was sent before.
    pub(crate) fn close(&mut self) {
        if let Some(ws) = &self.ws {
            let _ = ws.close();
        }
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        if let Some(ws) = self.ws.take() {
            ws.set_onopen(None);
            ws.set_onmessage(None);
            ws.set_onclose(None);
            let _ = ws.close();
        }
    }
}
