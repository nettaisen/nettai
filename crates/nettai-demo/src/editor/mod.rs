//! The match editor (docs/frontend.md §6, README.md "The match editor"):
//! the arena, each side's navi, folder, the lists and facts its game's rules
//! take, patch cards, NaviCust and EXE5's auto battle data, checked live
//! against the content. Play plays the match in the same window
//! (`crate::window`).

pub mod app;
pub mod auto_battle;
pub mod facts;
pub mod levels;
pub mod load;
pub mod names;
pub mod navicust;
pub mod order;
pub mod pictures;
pub mod view;

pub use app::{App, Editor, Msg, Options};
