//! The system's gamepads, one module chosen here as `system`, each with `init` and `next_event`: SDL3 on the desktop
//! ([`desktop`]; [`none`] without the `sdl3` feature), the browser's Gamepad API ([`web`]), and a phone's ([`phone`]:
//! iOS's GameController framework or Android's gamepad events, through one differ).

#[cfg(sdl)]
pub(crate) mod desktop;
#[cfg(all(desktop, not(sdl)))]
pub(crate) mod none;
#[cfg(web)]
pub(crate) mod web;
// (On the desktop too, for its tests: the differ, Android's map, and on macOS the iOS backend.)
#[cfg(any(phone, test))]
#[cfg_attr(not(phone), allow(dead_code))]
pub(crate) mod phone;

#[cfg(sdl)]
pub(crate) use desktop as system;
#[cfg(all(desktop, not(sdl)))]
pub(crate) use none as system;
#[cfg(web)]
pub(crate) use web as system;
#[cfg(phone)]
pub(crate) use phone::system;
