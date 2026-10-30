//! The two sides of a transfer, each on a thread of its own: the asker's ([`receive`]) and the offerer's, which
//! accepts connections ([`Offerer::accept`]) and serves each ([`Offerer::serve`]).

use crate::{ANSWER_WAIT, Answers, Event, Request, protocol};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::time::Duration;

/// How long a connection may go silent mid-transfer.
const SILENCE: Duration = Duration::from_secs(30);
/// How long connecting to one of a peer's addresses may take.
const CONNECTING: Duration = Duration::from_secs(4);

/// The asker's side: connect, say who asks, and read the answer and the files.
pub fn receive(me: &str, addrs: &[SocketAddr], name: &str, tell: &Sender<Event>) -> Result<Event, String> {
    let mut stream = addrs
        .iter()
        .find_map(|a| TcpStream::connect_timeout(a, CONNECTING).ok())
        .ok_or_else(|| format!("connecting to {name} at {addrs:?} failed"))?;
    let _ = stream.set_nodelay(true);
    stream.set_write_timeout(Some(SILENCE)).map_err(|e| e.to_string())?;
    protocol::write_hello(&mut stream, me).map_err(|e| format!("asking: {e}"))?;
    stream.set_read_timeout(Some(ANSWER_WAIT)).map_err(|e| e.to_string())?;
    let reply = protocol::read_reply(&mut stream).map_err(|e| format!("the answer: {e}"))?;
    let name = name.to_string();
    Ok(match reply {
        protocol::Reply::Declined => Event::Declined { name },
        protocol::Reply::Busy => Event::Busy { name },
        protocol::Reply::OtherVersion(version) => Event::OtherVersion { name, version },
        protocol::Reply::Accepted { count, total } => {
            stream.set_read_timeout(Some(SILENCE)).map_err(|e| e.to_string())?;
            let mut said = 0;
            let mut input = std::io::BufReader::with_capacity(256 << 10, &mut stream);
            let files = protocol::read_files(&mut input, count, total, &mut |done, total| say_progress(tell, &mut said, done, total))
                .map_err(|e| format!("receiving: {e}"))?;
            Event::Received { name, files }
        }
    })
}

/// The offerer's side: what each connection's thread shares.
pub struct Offerer {
    pub tell: Sender<Event>,
    pub answers: Answers,
    pub next: Arc<AtomicU32>,
    pub busy: Arc<AtomicBool>,
}

impl Offerer {
    /// Accept connections till `stop`, each served on a thread of its own.
    pub fn accept(self, listener: TcpListener, stop: &AtomicBool) {
        while !stop.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let offerer = Offerer { tell: self.tell.clone(), answers: self.answers.clone(), next: self.next.clone(), busy: self.busy.clone() };
                    let _ = std::thread::Builder::new().name("nettai-nearby-send".into()).spawn(move || offerer.serve(stream));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(50)),
                Err(e) => {
                    let _ = self.tell.send(Event::Failed { name: String::new(), why: format!("accepting: {e}") });
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
        }
    }

    /// One connection: hear who asks, ask the player, and send or say no.
    fn serve(self, mut stream: TcpStream) {
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_nodelay(true);
        let _ = stream.set_read_timeout(Some(SILENCE));
        let _ = stream.set_write_timeout(Some(SILENCE));
        let hello = match protocol::read_hello(&mut stream) {
            Ok(hello) => hello,
            Err(e) => {
                log::warn!("nearby: a connection that isn't nettai's asking: {e}");
                return;
            }
        };
        let name = hello.name;
        if hello.version != protocol::VERSION {
            let _ = protocol::write_refusal(&mut stream, protocol::Reply::OtherVersion(protocol::VERSION));
            let _ = self.tell.send(Event::OtherVersion { name, version: hello.version });
            return;
        }
        if self.busy.swap(true, Ordering::Relaxed) {
            let _ = protocol::write_refusal(&mut stream, protocol::Reply::Busy);
            return;
        }
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (answer, answered) = mpsc::channel();
        self.answers.lock().unwrap_or_else(|e| e.into_inner()).insert(id, answer);
        let _ = self.tell.send(Event::Asked { request: Request(id), name: name.clone() });
        let files = answered.recv_timeout(ANSWER_WAIT).ok().flatten();
        self.answers.lock().unwrap_or_else(|e| e.into_inner()).remove(&id);
        let event = match files {
            None => {
                let _ = protocol::write_refusal(&mut stream, protocol::Reply::Declined);
                None
            }
            Some(files) => {
                let mut said = 0;
                let mut out = std::io::BufWriter::with_capacity(256 << 10, &mut stream);
                let sent = protocol::write_files(&mut out, &files, &mut |done, total| say_progress(&self.tell, &mut said, done, total)).and_then(|()| {
                    use std::io::Write;
                    out.flush()
                });
                Some(match sent {
                    Ok(()) => Event::Sent { name },
                    Err(e) => Event::Failed { name, why: format!("sending: {e}") },
                })
            }
        };
        self.busy.store(false, Ordering::Relaxed);
        if let Some(event) = event {
            let _ = self.tell.send(event);
        }
    }
}

/// Progress said as each whole percent is passed.
fn say_progress(tell: &Sender<Event>, said: &mut u64, done: u64, total: u64) {
    let percent = done * 100 / total.max(1);
    if percent != *said || done == total {
        *said = percent;
        let _ = tell.send(Event::Progress { done, total });
    }
}
