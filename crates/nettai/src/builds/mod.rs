//! A player's builds, each a side of one game they made, named and kept
//! (`store`), and what edits one: a build laid out from its game's rules'
//! setup schema (`layout`), edited through the facts' own writer (`edit`,
//! `grid`, `auto`; a fact stated as one of a few presets, `presets`), read
//! from a save (`import`), with the pictures of what it holds (`pictures`).
//! This is the old creator's logic, kept for the Builds screen
//! (`crate::screens::builds`) to take what it reuses of (docs/app.md §8).
//! The game's builds data (its versions, a new build, a build's problems
//! and overview) is nettai-match's `build` now; `presets`, `layout` and
//! builds.toml are what that replaces.

// (The Builds screen isn't built yet: what it will call isn't called.)
#![allow(dead_code)]

pub mod auto;
pub mod cards;
pub mod edit;
pub mod grid;
pub mod import;
pub mod layout;
pub mod order;
pub mod pictures;
pub mod presets;
pub mod store;

