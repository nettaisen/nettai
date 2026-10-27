//! The battle content, the content directory's packs ([`index`]: content/'s
//! game packs and the support packs they depend on), which name a game's
//! assets and define the sprites' timing; and the asset pack format, a
//! game's graphics and sound in open formats (`pack`; no longer what any
//! game's assets load from: they are extracted from its ROMs, `nettai_extract`).
//!
//! - Battle content: the Luau modules that define the chips, navis, forms,
//!   weapons, stages, rules and sprites' timing, and the code that runs
//!   them ([`index`]), loaded alone into the engine's
//!   `nettai_battle::Content` ([`load::load_battle`]); what a game's assets
//!   hold of it, for drawing and sound, is [`load::asset_numbers`].
//! - Sprites: an indexed-PNG part atlas, and part layouts and the frames
//!   their animations draw (`sprite.json`) per sprite ([`sprite`]).
//! - Field, backgrounds (Tiled maps), HUD, the custom screen: indexed PNGs
//!   and JSON ([`stage`], [`hud`], [`custom`]).
//! - Sound: songs as MIDI in mid2agb's conventions plus a TOML sidecar
//!   ([`song`]), voicegroups, key maps and PSG waves as TOML, samples as
//!   WAV ([`sound`]).
//!
//! - Display text: the content's `locales/<language>/*.toml`, by
//!   definition key; the own language's is the content's strings, which the
//!   battle counts the chatbox's timing from, the others a frontend's alone
//!   ([`locale`]).
//! - The library order: a game's `library.toml`, the order its library
//!   screens list its chips in by category, and its NaviCust programs' and
//!   patch cards' orders, which menus list things in ([`library`]).
//! - The look: a game's `look.toml`, the colors the app's menus are drawn
//!   in while it is selected and the pattern of their backdrop ([`look`]).
//!
//! - Bundles: a game's content as it is published, one deterministic tar
//!   named by its SHA-256 ([`bundle`]), and an app's store of them,
//!   unpacked ([`store`]; docs/design/content-deployment.md).
//!
//! [`pack`] ties the formats together. Every import reports what it found in
//! a [`report::Report`]; nothing is silently approximated.
//!
//! The graphics and sound modules know GBA-style data (4bpp tiles,
//! 16-color palettes, OAM parts, M4A songs) but no EXE6 rule; only [`hud`],
//! [`custom`] and the field's panel tables are EXE6-shaped.

pub mod aseprite;
pub mod bundle;
pub mod custom;
pub mod hud;
pub mod image;
pub mod letters;
pub mod library;
pub mod lint;
pub mod load;
pub mod locale;
pub mod look;
pub mod midi;
pub mod names;
pub mod pack;
pub mod report;
pub mod shapes;
pub mod index;
pub mod song;
pub mod sound;
pub mod sprite;
pub mod stage;
pub mod store;
pub mod tiles;
pub mod timeline;
pub mod verify;
pub mod wav;
