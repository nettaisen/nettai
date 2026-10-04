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

/// The battle content a match of `game` plays: the game's content folder
/// and the support folders it uses, drawn on its pack (`found`: the packs
/// found). The one place the frontend loads content, so the loader's
/// one-game form (`nettai_content::pack::load_game`) is a small switch:
/// until it lands this loads every folder whose pack is found (BN5's
/// folder still requires BN6's modules), and the match plays `game`'s
/// alone (`nettai_match::ids`).
pub fn load_game(
    content: Option<&std::path::Path>,
    game: &str,
    found: &[nettai_content::pack::Found],
) -> Result<nettai_content::pack::Loaded, nettai_content::report::Report> {
    let _ = game;
    nettai_content::pack::load_found(content, found)
}
