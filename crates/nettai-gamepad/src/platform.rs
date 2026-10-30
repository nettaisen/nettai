//! The system's gamepads, one module chosen here as `system`, each with `init` and `next_event`: SDL3 on the desktop
//! ([`desktop`]; [`none`] without the `sdl3` feature), the browser's Gamepad API ([`web`]), iOS's GameController
//! framework ([`ios`]) and Android's gamepad events ([`android`]). The phones share one differ ([`pads`]).

#[cfg(sdl)]
pub(crate) mod desktop;
#[cfg(all(desktop, not(sdl)))]
pub(crate) mod none;
#[cfg(web)]
pub(crate) mod web;
// (On macOS too, for its tests: the framework is the same.)
#[cfg(any(ios, all(test, target_os = "macos")))]
pub(crate) mod ios;
#[cfg(android)]
pub(crate) mod android;
// What the phones share, and Android's key and axis map (their tests run on the desktop).
#[cfg(any(ios, android, test))]
pub(crate) mod pads;
#[cfg(any(android, test))]
pub(crate) mod android_map;

#[cfg(sdl)]
pub(crate) use desktop as system;
#[cfg(all(desktop, not(sdl)))]
pub(crate) use none as system;
#[cfg(web)]
pub(crate) use web as system;
#[cfg(ios)]
pub(crate) use ios as system;
#[cfg(android)]
pub(crate) use android as system;
