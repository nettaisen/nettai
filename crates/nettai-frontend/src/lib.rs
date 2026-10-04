//! nettai's desktop app: it runs battles through the engine and shows them
//! drawn by nettai-render, in a window or as PNG files.
//!
//! The app is what is around the drawing: the window and the keys
//! ([`app`]), the sessions ([`session`]) and what drives them ([`driver`]:
//! a golden trace, live play, a match file; [`netplay`]: another player
//! over the network), the sound (a [`TickHook`] to nettai-audio, and the
//! audio's lookups, [`sound_lookups`]), headless output ([`headless`]) and
//! the audits ([`content_audit`], [`headless::audit_traces`]).
//! docs/frontend.md.

pub mod app;
pub mod content_audit;
pub mod driver;
pub mod headless;
pub mod netplay;
pub mod session;
pub mod sound_lookups;
pub mod text;

pub use nettai_render::{Frame, Renderer};
pub use session::{Session, TickHook};

