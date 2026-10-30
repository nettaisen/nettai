//! iOS's discovery: the system's DNS-SD (`dns_sd.h`), which asks mDNSResponder, so the app needs no multicast
//! entitlement; the app's Info.plist declares the service (`NSBonjourServices`) and says why it looks
//! (`NSLocalNetworkUsageDescription`), and the player is asked once. Browsing alone: a phone doesn't offer.
//!
//! One thread: it browses, and resolves each service found to its host and port, then the host to its IPv4 addresses,
//! waiting on each operation's socket with `poll`. The same API is macOS's, where this crate's tests use it.
// (On macOS, for the tests, the module is built but isn't the system's.)
#![cfg_attr(not(target_os = "ios"), allow(dead_code))]

use crate::{SERVICE, Seen};
use std::collections::{HashMap, HashSet};
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

type DNSServiceRef = *mut c_void;
type Flags = u32;
type Error = i32;

const FLAG_MORE_COMING: Flags = 0x1;
const FLAG_ADD: Flags = 0x2;
const PROTOCOL_IPV4: u32 = 0x1;
const NO_ERROR: Error = 0;

type BrowseReply = extern "C" fn(DNSServiceRef, Flags, u32, Error, *const c_char, *const c_char, *const c_char, *mut c_void);
type ResolveReply = extern "C" fn(DNSServiceRef, Flags, u32, Error, *const c_char, *const c_char, u16, u16, *const u8, *mut c_void);
type AddrInfoReply = extern "C" fn(DNSServiceRef, Flags, u32, Error, *const c_char, *const libc::sockaddr, u32, *mut c_void);

// libSystem's (libsystem_dnssd), linked by default on Apple's systems.
unsafe extern "C" {
    fn DNSServiceBrowse(
        sd_ref: *mut DNSServiceRef,
        flags: Flags,
        interface: u32,
        regtype: *const c_char,
        domain: *const c_char,
        reply: BrowseReply,
        context: *mut c_void,
    ) -> Error;
    fn DNSServiceResolve(
        sd_ref: *mut DNSServiceRef,
        flags: Flags,
        interface: u32,
        name: *const c_char,
        regtype: *const c_char,
        domain: *const c_char,
        reply: ResolveReply,
        context: *mut c_void,
    ) -> Error;
    fn DNSServiceGetAddrInfo(
        sd_ref: *mut DNSServiceRef,
        flags: Flags,
        interface: u32,
        protocol: u32,
        hostname: *const c_char,
        reply: AddrInfoReply,
        context: *mut c_void,
    ) -> Error;
    fn DNSServiceRefSockFD(sd_ref: DNSServiceRef) -> c_int;
    fn DNSServiceProcessResult(sd_ref: DNSServiceRef) -> Error;
    fn DNSServiceRefDeallocate(sd_ref: DNSServiceRef);
    fn TXTRecordGetValuePtr(txt_len: u16, txt: *const c_void, key: *const c_char, value_len: *mut u8) -> *const c_void;
}

/// An operation, deallocated when dropped (which ends it).
struct Operation(DNSServiceRef);

impl Drop for Operation {
    fn drop(&mut self) {
        // SAFETY: the reference came from a call that succeeded, and is deallocated once.
        unsafe { DNSServiceRefDeallocate(self.0) }
    }
}

impl Operation {
    /// Wait up to `wait` for the operation's next reply, and hear it (its callback runs on this thread, within).
    /// Whether one came.
    fn process(&self, wait: Duration) -> Result<bool, String> {
        // SAFETY: the reference is live (self owns it).
        let fd = unsafe { DNSServiceRefSockFD(self.0) };
        let mut polled = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
        // SAFETY: one pollfd, as said.
        let ready = unsafe { libc::poll(&mut polled, 1, wait.as_millis().min(i32::MAX as u128) as c_int) };
        if ready < 0 {
            return Err(format!("poll: {}", std::io::Error::last_os_error()));
        }
        if ready == 0 {
            return Ok(false);
        }
        // SAFETY: the reference is live; the reply's callback gets the context its call was given, which outlives this.
        match unsafe { DNSServiceProcessResult(self.0) } {
            NO_ERROR => Ok(true),
            error => Err(format!("DNS-SD error {error}")),
        }
    }
}

fn started(sd_ref: DNSServiceRef, error: Error, what: &str) -> Result<Operation, String> {
    match error {
        NO_ERROR => Ok(Operation(sd_ref)),
        error => Err(failure(error, what)),
    }
}

/// kDNSServiceErr_PolicyDenied: the player said no to the local network, or the Info.plist doesn't declare the
/// service.
const POLICY_DENIED: Error = -65570;

fn failure(error: Error, what: &str) -> String {
    match error {
        POLICY_DENIED => format!("{what}: the local network isn't allowed (Settings > Privacy & Security > Local Network)"),
        error => format!("{what}: DNS-SD error {error}"),
    }
}

/// The browse's end for `error`: not allowed, or failed.
fn stopped(error: Error, what: &str) -> Seen {
    match error {
        POLICY_DENIED => Seen::NotAllowed,
        error => Seen::Failed { why: failure(error, what) },
    }
}

/// A service browsed: its instance name, type and domain as the browse said them, and the interface.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Instance {
    name: CString,
    regtype: CString,
    domain: CString,
}

impl Instance {
    fn key(&self) -> String {
        format!("{}.{}{}", self.name.to_string_lossy(), self.regtype.to_string_lossy(), self.domain.to_string_lossy())
    }
}

/// The browse's replies since last heard: each instance added or removed, on an interface.
#[derive(Default)]
struct Browsed {
    changes: Vec<(Instance, u32, bool)>,
    failed: Option<Error>,
}

extern "C" fn on_browse(
    _: DNSServiceRef,
    flags: Flags,
    interface: u32,
    error: Error,
    name: *const c_char,
    regtype: *const c_char,
    domain: *const c_char,
    context: *mut c_void,
) {
    // SAFETY: the context is the Browsed the browse was started with, alive and not otherwise borrowed while
    // DNSServiceProcessResult runs on its thread.
    let browsed = unsafe { &mut *context.cast::<Browsed>() };
    if error != NO_ERROR {
        browsed.failed = Some(error);
        return;
    }
    // SAFETY: on success the strings are DNS-SD's, terminated, valid for the callback.
    let (name, regtype, domain) = unsafe { (CStr::from_ptr(name).into(), CStr::from_ptr(regtype).into(), CStr::from_ptr(domain).into()) };
    browsed.changes.push((Instance { name, regtype, domain }, interface, flags & FLAG_ADD != 0));
}

/// A resolve's reply: the host and port, and the TXT record's name.
#[derive(Default)]
struct Resolved {
    host: Option<(CString, u16, Option<String>)>,
}

extern "C" fn on_resolve(
    _: DNSServiceRef,
    _: Flags,
    _: u32,
    error: Error,
    _: *const c_char,
    host: *const c_char,
    port: u16,
    txt_len: u16,
    txt: *const u8,
    context: *mut c_void,
) {
    // SAFETY: the context is the Resolved the resolve was started with (as for `on_browse`).
    let resolved = unsafe { &mut *context.cast::<Resolved>() };
    if error != NO_ERROR {
        return;
    }
    let mut len = 0u8;
    // SAFETY: the TXT record is txt_len bytes, valid for the callback; the key is terminated; the value, where found,
    // is `len` bytes within the record.
    let name = unsafe {
        let value = TXTRecordGetValuePtr(txt_len, txt.cast(), c"n".as_ptr(), &mut len);
        (!value.is_null()).then(|| String::from_utf8_lossy(std::slice::from_raw_parts(value.cast::<u8>(), len as usize)).into_owned())
    };
    // SAFETY: the host is DNS-SD's terminated string, valid for the callback.
    let host = unsafe { CStr::from_ptr(host) }.into();
    resolved.host = Some((host, u16::from_be(port), name));
}

/// A host's IPv4 addresses, as heard.
#[derive(Default)]
struct Addresses {
    found: Vec<Ipv4Addr>,
    more: bool,
}

extern "C" fn on_address(_: DNSServiceRef, flags: Flags, _: u32, error: Error, _: *const c_char, address: *const libc::sockaddr, _: u32, context: *mut c_void) {
    // SAFETY: the context is the Addresses the lookup was started with (as for `on_browse`).
    let addresses = unsafe { &mut *context.cast::<Addresses>() };
    addresses.more = flags & FLAG_MORE_COMING != 0;
    if error != NO_ERROR || address.is_null() || flags & FLAG_ADD == 0 {
        return;
    }
    // SAFETY: a non-null address of family AF_INET is a sockaddr_in.
    unsafe {
        if (*address).sa_family as c_int == libc::AF_INET {
            let address = &*address.cast::<libc::sockaddr_in>();
            addresses.found.push(Ipv4Addr::from(u32::from_be(address.sin_addr.s_addr)));
        }
    }
}

pub const OFFERS: bool = false;
pub const SEEKS: bool = true;

/// Never made: a phone doesn't offer.
pub struct Advertised;

pub fn advertise(_name: &str, _port: u16) -> Result<Advertised, String> {
    Err("a phone doesn't offer".into())
}

/// UIDevice's name: the model's ("iPhone"), as iOS tells apps without an entitlement Apple grants.
#[cfg(ios)]
pub fn device_name() -> String {
    // UIDevice is the main thread's; the app makes its Nearby there.
    let name = objc2::MainThreadMarker::new().map(|mtm| objc2_ui_kit::UIDevice::currentDevice(mtm).name().to_string());
    name.filter(|n| !n.is_empty()).unwrap_or_else(|| "iPhone".into())
}

/// How long resolving a service, or its host, may take. (One gone, still in the system's cache, takes this long to
/// pass over: the browse waits for it.)
const RESOLVING: Duration = Duration::from_secs(3);

/// Resolve `instance` on `interface` to its name (from the TXT record, else the instance's) and IPv4 addresses.
fn resolve(instance: &Instance, interface: u32) -> Result<(String, Vec<SocketAddr>), String> {
    let mut resolved = Resolved::default();
    let mut sd_ref = std::ptr::null_mut();
    // SAFETY: the strings are terminated; the context outlives the operation, which is dropped below, in this scope.
    let error = unsafe {
        DNSServiceResolve(&mut sd_ref, 0, interface, instance.name.as_ptr(), instance.regtype.as_ptr(), instance.domain.as_ptr(), on_resolve, (&raw mut resolved).cast())
    };
    let operation = started(sd_ref, error, "resolving")?;
    let until = Instant::now() + RESOLVING;
    while resolved.host.is_none() && Instant::now() < until {
        operation.process(until - Instant::now())?;
    }
    drop(operation);
    let (host, port, name) = resolved.host.ok_or("no answer resolving")?;

    let mut addresses = Addresses::default();
    let mut sd_ref = std::ptr::null_mut();
    // SAFETY: as for the resolve.
    let error = unsafe { DNSServiceGetAddrInfo(&mut sd_ref, 0, interface, PROTOCOL_IPV4, host.as_ptr(), on_address, (&raw mut addresses).cast()) };
    let operation = started(sd_ref, error, "looking up the host")?;
    let until = Instant::now() + RESOLVING;
    while (addresses.found.is_empty() || addresses.more) && Instant::now() < until {
        operation.process(until - Instant::now())?;
    }
    drop(operation);
    let name = name.filter(|n| !n.is_empty()).unwrap_or_else(|| instance.name.to_string_lossy().into_owned());
    Ok((name, addresses.found.into_iter().map(|ip| SocketAddr::from((ip, port))).collect()))
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

/// Browse for devices offering; what is found and lost is told to `seen`, and so is the browse's end where the system
/// ends it (the local network not allowed): here, only a thread that won't start fails.
pub fn browse(seen: Sender<Seen>) -> Result<Browsing, String> {
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    std::thread::Builder::new()
        .name("nettai-nearby-browse".into())
        .spawn(move || {
            if let Err(end) = run(&seen, &stopped) {
                log::warn!("nearby: {end:?}");
                let _ = seen.send(end);
            }
        })
        .map_err(|e| format!("no thread: {e}"))?;
    Ok(Browsing { stop })
}

/// Browse till `stop`; the end, where it ends otherwise.
fn run(seen: &Sender<Seen>, stop: &AtomicBool) -> Result<(), Seen> {
    let regtype = CString::new(format!("{SERVICE}.")).expect("no NUL");
    let mut state = Browsed::default();
    // The callback's way to the state, and this function's too (so neither invalidates the other's).
    let heard = &raw mut state;
    let mut sd_ref = std::ptr::null_mut();
    // SAFETY: the type is terminated, the domain null (the default: local); the state outlives the operation, which
    // is dropped before it (declared after it).
    let error = unsafe { DNSServiceBrowse(&mut sd_ref, 0, 0, regtype.as_ptr(), std::ptr::null(), on_browse, heard.cast()) };
    let operation = started(sd_ref, error, "browsing").map_err(|_| stopped(error, "browsing"))?;
    // Each instance's interfaces it was found on, and the addresses resolved on them.
    let mut found: HashMap<Instance, (HashSet<u32>, String, Vec<SocketAddr>)> = HashMap::new();
    while !stop.load(Ordering::Relaxed) {
        operation.process(Duration::from_millis(200)).map_err(|why| Seen::Failed { why })?;
        // SAFETY: no callback runs now (they run within `process`, on this thread).
        let (failed, changes) = unsafe { ((*heard).failed.take(), std::mem::take(&mut (*heard).changes)) };
        if let Some(error) = failed {
            return Err(stopped(error, "browsing"));
        }
        for (instance, interface, added) in changes {
            // (One resolved already, found on another interface too: the same service.)
            if added && let Some(entry) = found.get_mut(&instance) {
                entry.0.insert(interface);
                continue;
            }
            if added {
                let (name, addrs) = match resolve(&instance, interface) {
                    Ok(resolved) => resolved,
                    Err(why) => {
                        log::warn!("nearby: {}: {why}", instance.key());
                        continue;
                    }
                };
                let entry = found.entry(instance.clone()).or_insert_with(|| (HashSet::new(), name.clone(), Vec::new()));
                entry.0.insert(interface);
                entry.1 = name;
                for addr in addrs {
                    if !entry.2.contains(&addr) {
                        entry.2.push(addr);
                    }
                }
                if seen.send(Seen::Found { key: instance.key(), name: entry.1.clone(), addrs: entry.2.clone() }).is_err() {
                    return Ok(());
                }
            } else if let Some(entry) = found.get_mut(&instance) {
                entry.0.remove(&interface);
                if entry.0.is_empty() {
                    found.remove(&instance);
                    if seen.send(Seen::Lost { key: instance.key() }).is_err() {
                        return Ok(());
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The desktop's advertisement (mdns-sd's) is found by the system's DNS-SD, with its name and an address of this
    /// computer's. Ignored by default: it needs an interface that carries multicast.
    #[test]
    #[ignore]
    fn the_system_finds_the_desktops_offer() {
        let advertised = super::super::desktop::advertise("test computer", 4321).unwrap();
        let (tell, heard) = std::sync::mpsc::channel();
        let _browsing = browse(tell).unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        while Instant::now() < until {
            if let Ok(Seen::Found { name, addrs, .. }) = heard.recv_timeout(Duration::from_millis(100))
                && name == "test computer"
            {
                assert!(addrs.iter().all(|a| a.port() == 4321) && !addrs.is_empty(), "{addrs:?}");
                drop(advertised);
                return;
            }
        }
        panic!("not found");
    }
}
