//! Discovery, a module each system, chosen here and nowhere else:
//!
//! - [`desktop`] (Windows, macOS, Linux): the `mdns-sd` crate, which advertises and browses;
//! - [`ios`]: the system's DNS-SD (`dns_sd.h`, mDNSResponder's), which iOS allows an app that declares the service in
//!   its Info.plist (`NSBonjourServices`; the player is asked once about the local network), where raw multicast needs
//!   an entitlement Apple grants on request; browsing alone (macOS has the API too: the tests run it there);
//! - [`android`]: NsdManager, from Kotlin (`android/nettai/nearby/NettaiNearby.kt`), what it finds heard through
//!   native methods; browsing alone;
//! - [`web`]: none (a page can neither browse nor listen): nothing is offered or found.
//!
//! Each has the same items: whether it offers and seeks ([`system::OFFERS`], [`system::SEEKS`]), the advertisement of
//! a port while it is kept ([`system::advertise`]), the browse that tells what it finds and loses while it is kept
//! ([`system::browse`], told as [`crate::Seen`]), and the device's name ([`system::device_name`]). The rest (the
//! transfers, the peers found) is the same for all.

#[cfg(android)]
#[allow(unsafe_code)]
pub(crate) mod android;
#[cfg(desktop)]
#[allow(unsafe_code)]
pub(crate) mod desktop;
#[cfg(any(ios, all(test, target_os = "macos")))]
#[allow(unsafe_code)]
pub(crate) mod ios;
#[cfg(web)]
pub(crate) mod web;

#[cfg(android)]
pub(crate) use android as system;
#[cfg(desktop)]
pub(crate) use desktop as system;
#[cfg(ios)]
pub(crate) use ios as system;
#[cfg(web)]
pub(crate) use web as system;
