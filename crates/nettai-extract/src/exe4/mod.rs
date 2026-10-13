//! EXE4's assets (Red Sun and Blue Moon, US and Japanese), read through the
//! game's own pointers to them, which the routine maps give in each of the
//! four ROMs (docs/design/exe4-map.md; the verification workspace's
//! tools/exe5/bmap.py and tools/exe4/emap.py pair EXE4's routines with
//! EXE6's and EXE5's, and its tools/cross-rom find each block's pointer).
//!
//! What an EXE4 pack has: the battle sprites (the same archives in all four
//! ROMs), the chips' pictures and icons, the battle backgrounds (the same
//! in all four), the sound (the same songs in all four), the field, the HUD
//! and the custom screen (graphics.rs, hud.rs: the HP box, the gauge, the
//! banners, the emotion window's faces, "BUSY...", the chatbox;
//! custom.rs), each from whichever ROM given holds it; the fonts, the
//! HUD's text lines and the words of the banners, "BUSY..." and the custom
//! screen in English (a US ROM's) and Japanese (a Japanese ROM's); a
//! version's souls' faces from a ROM of that version.

mod cards;
mod custom;
pub(super) mod graphics;
mod hud;
pub(super) mod names;
pub(super) mod rom;

