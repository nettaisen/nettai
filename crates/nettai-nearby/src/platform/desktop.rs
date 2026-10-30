//! The desktop's discovery: the `mdns-sd` crate, which answers and asks multicast DNS itself (UDP port 5353, beside
//! the system's own responder where there is one). One daemon for the process, made when first needed. A desktop
//! offers (where the app has the collection) and seeks (where it hasn't).
//!
//! On Windows the first offer brings up the firewall's prompt for nettai (it listens for TCP and for multicast DNS):
//! allowed on private networks, phones find the computer; denied, they don't, and the phone says to check it.

use crate::{SERVICE, Seen, protocol};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const OFFERS: bool = true;
pub const SEEKS: bool = true;

/// The computer's name: its host name's first label, as the system has it.
pub fn device_name() -> String {
    #[cfg(unix)]
    let name = {
        let mut buffer = [0u8; 256];
        // SAFETY: the buffer is as long as said, and gethostname writes a terminated name within it (or fails).
        let ok = unsafe { libc::gethostname(buffer.as_mut_ptr().cast(), buffer.len()) } == 0;
        let len = buffer.iter().position(|b| *b == 0).unwrap_or(0);
        if ok { String::from_utf8_lossy(&buffer[..len]).into_owned() } else { String::new() }
    };
    #[cfg(windows)]
    let name = std::env::var("COMPUTERNAME").unwrap_or_default();
    // (Its first label: not `.local`, nor a network's `.localdomain` or `.lan`.)
    let name = name.split('.').next().unwrap_or_default().to_string();
    if name.is_empty() { "nettai".into() } else { name }
}

fn service_type() -> String {
    format!("{SERVICE}.local.")
}

fn daemon() -> Result<ServiceDaemon, String> {
    static DAEMON: Mutex<Option<ServiceDaemon>> = Mutex::new(None);
    let mut daemon = DAEMON.lock().unwrap_or_else(|e| e.into_inner());
    if daemon.is_none() {
        *daemon = Some(ServiceDaemon::new().map_err(|e| format!("multicast DNS: {e}"))?);
    }
    Ok(daemon.clone().expect("made above"))
}

/// A short tag that keeps two devices of one name apart: the process and the moment it was made in.
fn tag() -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
    format!("{:04x}", (nanos ^ std::process::id().rotate_left(16)) & 0xffff)
}

/// This device advertised, until dropped.
pub struct Advertised {
    daemon: ServiceDaemon,
    fullname: String,
}

impl Drop for Advertised {
    /// The advertisement taken back: its goodbye sent before this returns (a second at most), so that a process
    /// that ends now isn't found by the others till their caches let it go.
    fn drop(&mut self) {
        if let Ok(done) = self.daemon.unregister(&self.fullname) {
            let _ = done.recv_timeout(Duration::from_secs(1));
        }
    }
}

/// Advertise this device as offering, on `port`: the instance named for the device (and a tag), its own host name
/// (so as not to answer for the computer's), on every interface's addresses, the TXT record saying the protocol's
/// version (`v`) and the device's name (`n`).
pub fn advertise(name: &str, port: u16) -> Result<Advertised, String> {
    let daemon = daemon()?;
    let tag = tag();
    let shown: String = name.chars().filter(|c| !c.is_control() && *c != '.').take(40).collect();
    let instance = format!("{shown} {tag}");
    let host = format!("nettai-{tag}.local.");
    let version = protocol::VERSION.to_string();
    let properties = [("v", version.as_str()), ("n", name)];
    let info = ServiceInfo::new(&service_type(), &instance, &host, (), port, &properties[..])
        .map_err(|e| format!("the advertisement: {e}"))?
        .enable_addr_auto();
    let fullname = info.get_fullname().to_string();
    daemon.register(info).map_err(|e| format!("advertising: {e}"))?;
    Ok(Advertised { daemon, fullname })
}

/// Browsing, until dropped.
pub struct Browsing {
    stop: Arc<AtomicBool>,
}

impl Drop for Browsing {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Browse for devices offering; what is found and lost is told to `seen`.
pub fn browse(seen: Sender<Seen>) -> Result<Browsing, String> {
    let daemon = daemon()?;
    let ty = service_type();
    let events = daemon.browse(&ty).map_err(|e| format!("browsing: {e}"))?;
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    std::thread::Builder::new()
        .name("nettai-nearby-browse".into())
        .spawn(move || {
            while !stopped.load(Ordering::Relaxed) {
                let Ok(event) = events.recv_timeout(Duration::from_millis(200)) else {
                    if events.is_disconnected() {
                        break;
                    }
                    continue;
                };
                let said = match event {
                    ServiceEvent::ServiceResolved(service) => {
                        let name = service.get_property_val_str("n").filter(|n| !n.is_empty()).map(str::to_string);
                        let name = name.unwrap_or_else(|| service.fullname.split('.').next().unwrap_or_default().to_string());
                        let addrs = service.get_addresses_v4().into_iter().map(|ip| SocketAddr::from((ip, service.port))).collect();
                        seen.send(Seen::Found { key: service.fullname.clone(), name, addrs })
                    }
                    ServiceEvent::ServiceRemoved(_, fullname) => seen.send(Seen::Lost { key: fullname }),
                    _ => Ok(()),
                };
                if said.is_err() {
                    break;
                }
            }
            let _ = daemon.stop_browse(&ty);
        })
        .map_err(|e| format!("no thread: {e}"))?;
    Ok(Browsing { stop })
}
