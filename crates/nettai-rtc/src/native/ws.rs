//! A WebSocket to the signaling server, on a thread of its own: connecting
//! (the name, TCP, TLS, the upgrade) waits, and so does reading; the
//! caller's thread never does. One socket is one connection: when it
//! closes, the room makes another (`crate::room`).

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::Duration;

use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

/// What happened on the socket.
#[derive(Debug)]
pub(crate) enum WsEvent {
    Open,
    Text(String),
    /// It closed (or never opened), and why; nothing more comes.
    Closed(String),
}

enum Command {
    Send(String),
    Close,
}

/// The caller's end of a WebSocket.
pub(crate) struct Socket {
    commands: Sender<Command>,
    events: Receiver<WsEvent>,
    closed: bool,
}

/// How long a read waits for the server before the thread looks for what to
/// send.
const READ_WAIT: Duration = Duration::from_millis(20);

impl Socket {
    /// Connect to `url`, on a thread: [`WsEvent::Open`] when it is up.
    pub(crate) fn connect(url: &str) -> Socket {
        let (commands, take) = mpsc::channel();
        let (tell, events) = mpsc::channel();
        let url = url.to_string();
        let spawned = std::thread::Builder::new().name("nettai-signal".into()).spawn(move || {
            let why = run(&url, &take, &tell).err().unwrap_or_else(|| "closed".into());
            let _ = tell.send(WsEvent::Closed(why));
        });
        let mut socket = Socket { commands, events, closed: false };
        if let Err(e) = spawned {
            socket.closed = true;
            let (tell, events) = mpsc::channel();
            let _ = tell.send(WsEvent::Closed(format!("can't start the signaling thread: {e}")));
            socket.events = events;
        }
        socket
    }

    /// Send a text message (queued while it opens; lost if it closes).
    pub(crate) fn send(&mut self, text: String) {
        let _ = self.commands.send(Command::Send(text));
    }

    /// What happened since, without waiting.
    pub(crate) fn poll(&mut self) -> Option<WsEvent> {
        match self.events.try_recv() {
            Ok(e) => {
                if let WsEvent::Closed(_) = e {
                    self.closed = true;
                }
                Some(e)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) if !self.closed => {
                self.closed = true;
                Some(WsEvent::Closed("the signaling thread ended".into()))
            }
            Err(TryRecvError::Disconnected) => None,
        }
    }

    /// Close it, after what was sent before.
    pub(crate) fn close(&mut self) {
        let _ = self.commands.send(Command::Close);
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        self.close();
    }
}

fn run(url: &str, take: &Receiver<Command>, tell: &Sender<WsEvent>) -> Result<(), String> {
    let (mut ws, _) = tungstenite::connect(url).map_err(|e| format!("can't reach the signaling server: {e}"))?;
    let tcp = match ws.get_ref() {
        MaybeTlsStream::Plain(s) => s,
        MaybeTlsStream::Rustls(s) => s.get_ref(),
        _ => return Err("the signaling server's stream isn't one this can wait on".into()),
    };
    tcp.set_read_timeout(Some(READ_WAIT)).map_err(|e| e.to_string())?;
    let _ = tcp.set_nodelay(true);
    if tell.send(WsEvent::Open).is_err() {
        return Ok(());
    }
    loop {
        loop {
            match take.try_recv() {
                Ok(Command::Send(text)) => ws.send(Message::text(text)).map_err(|e| e.to_string())?,
                Ok(Command::Close) | Err(TryRecvError::Disconnected) => return close(&mut ws),
                Err(TryRecvError::Empty) => break,
            }
        }
        match ws.read() {
            Ok(Message::Text(text)) => {
                if tell.send(WsEvent::Text(text.to_string())).is_err() {
                    return close(&mut ws);
                }
            }
            Ok(Message::Close(frame)) => return Err(frame.map_or("the server closed it".into(), |f| format!("the server closed it ({})", f.reason))),
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {
                // (What was written while the read waited.)
                match ws.flush() {
                    Ok(()) => {}
                    Err(tungstenite::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                    Err(e) => return Err(e.to_string()),
                }
            }
            Err(e) => return Err(e.to_string()),
        }
    }
}

fn close(ws: &mut WebSocket<MaybeTlsStream<std::net::TcpStream>>) -> Result<(), String> {
    let _ = ws.close(None);
    // (The close goes out; its answer may not come in time to matter.)
    for _ in 0..5 {
        match ws.read() {
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
            Err(_) => break,
        }
    }
    Ok(())
}
