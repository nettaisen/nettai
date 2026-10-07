//! The build creator: a player's builds, each a side of one game they
//! made, named and kept (`store`), chosen in Training and in the lobby. The
//! creator lays a build out from its game's rules' setup schema
//! (`layout`), edits it through the facts' own writer (`edit`, `grid`,
//! `auto`; a fact stated as one of a few presets, `presets`), shows what
//! the rules say of it where they say it, and reads a
//! side from a save (`import`). docs/app.md §8.

pub mod auto;
pub mod cards;
pub mod edit;
pub mod grid;
pub mod import;
pub mod layout;
pub mod order;
pub mod pictures;
pub mod presets;
pub mod screen;
pub mod store;
pub mod view;
