//! EXE4's assets (Red Sun and Blue Moon, US and Japanese), from the
//! addresses the routine maps give (docs/design/exe4-map.md; the
//! verification workspace's tools/exe5/bmap.py and tools/exe4/emap.py pair
//! EXE4's routines with EXE6's and EXE5's, and their `romdata` the data
//! the paired routines load, in each of the four ROMs).
//!
//! What an EXE4 pack has so far: the battle sprites (the same archives in
//! all four ROMs), the chips' pictures and icons, the fonts and the HUD's
//! text lines in English (the US ROMs) and Japanese (the Japanese ROMs),
//! the battle backgrounds (the same in all four), the sound (the same
//! songs in all four), the field, the HUD and the custom screen (Red Sun
//! US's: graphics.rs, hud.rs: the HP box, the gauge, the banners, the
//! emotion window's faces, "BUSY...", the chatbox; custom.rs), and the
//! Japanese words of the banners, "BUSY..." and the custom screen.

mod custom;
pub(super) mod graphics;
mod hud;
pub(super) mod names;
pub(super) mod rom;

