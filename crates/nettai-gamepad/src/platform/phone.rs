//! A phone's gamepads, chosen here as `system`: iOS's GameController framework ([`ios`]) or Android's gamepad events
//! ([`android`]), each keeping its pads' state in the one differ they share ([`pads`]).

pub(crate) mod pads;
// (On macOS too, for its tests: the framework is the same.)
#[cfg(any(ios, all(test, target_os = "macos")))]
pub(crate) mod ios;
// (In tests anywhere too: its key and axis map's tests run on the desktop; JNI builds there.)
#[cfg(any(android, test))]
pub(crate) mod android;

#[cfg(ios)]
pub(crate) use ios as system;
#[cfg(android)]
pub(crate) use android as system;
