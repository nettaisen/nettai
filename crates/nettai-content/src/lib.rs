//! The battle content: the content directory's packs ([`index`]: content/'s
//! game packs and the support packs they depend on), which define a game's
//! battle, name its assets and time its sprites. A game's assets themselves
//! (its graphics and sound) are extracted from its ROMs (`nettai_extract`).
//!
//! - Battle content: the Luau modules that define the chips, navis, forms,
//!   weapons, stages, rules and sprites' timing, and the code that runs
//!   them ([`index`]), loaded alone into the engine's
//!   `nettai_battle::Content` ([`load::load_battle`]); what a game's assets
//!   hold of it, for drawing and sound, is [`load::asset_numbers`].
//! - Display text: the content's `locales/<language>/*.toml`, by
//!   definition key; the own language's is the content's strings, which the
//!   battle counts the chatbox's timing from, the others a frontend's alone
//!   ([`locale`]).
//! - The library order: a game's `library.toml`, the order its library
//!   screens list its chips in by category, and its NaviCust programs' and
//!   patch cards' orders, which menus list things in ([`library`]).
//! - The look: a game's `look.toml`, the colors the app's menus are drawn
//!   in while it is selected and the pattern of their backdrop ([`look`]).
//! - Bundles: a game's content as it is published, one deterministic tar
//!   named by its SHA-256 ([`bundle`]), and an app's store of them,
//!   unpacked ([`store`]; docs/design/content-deployment.md).
//!
//! What a check or a load found is said in a [`report::Report`].

pub mod bundle;
pub mod index;
pub mod library;
pub mod lint;
pub mod load;
pub mod locale;
pub mod look;
pub mod report;
pub mod store;
