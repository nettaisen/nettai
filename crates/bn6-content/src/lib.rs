//! Content packs: a battle's data, graphics and sound in open formats that
//! ordinary tools edit, loaded exactly into the data the engine, the
//! frontend and the audio use.
//!
//! - Battle data: chips, navis and forms, object kinds' data, the ruleset's
//!   tables and registries, as TOML files laid out by owner ([`battle`]),
//!   loaded into the engine's `bn6_battle::Content`.
//! - Sprites: an indexed-PNG part atlas, part layouts (`sprite.json`) and
//!   animation timing (`animations.json`) per sprite ([`sprite`]).
//! - Field, backgrounds (Tiled maps), HUD: indexed PNGs and JSON ([`stage`],
//!   [`hud`]).
//! - Sound: songs as MIDI in mid2agb's conventions plus a TOML sidecar
//!   ([`song`]), voicegroups, key maps and PSG waves as TOML, samples as
//!   WAV ([`sound`]).
//!
//! [`pack`] ties them together and loads each part straight from its
//! files; [`timing`] reads the simulation's animation timing alone. Every import reports what
//! it found in a [`report::Report`]; nothing is silently approximated.
//!
//! The graphics and sound modules know GBA-style data (4bpp tiles,
//! 16-colour palettes, OAM parts, M4A songs) but no BN6 rule; only [`hud`]
//! and the field's panel tables are BN6-shaped. [`battle`] is the engine's
//! BN6 data model as files.

pub mod aseprite;
pub mod battle;
pub mod hud;
pub mod image;
pub mod midi;
pub mod pack;
pub mod report;
pub mod song;
pub mod sound;
pub mod sprite;
pub mod stage;
pub mod tiles;
pub mod timeline;
pub mod timing;
pub mod verify;
pub mod wav;
