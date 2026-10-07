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
//! songs in all four), the field and the HUD (Red Sun US's: graphics.rs,
//! hud.rs: the HP box, the gauge, the banners, the emotion window's faces,
//! "BUSY...", the chatbox), and the banners' and "BUSY..."'s Japanese
//! words. The custom screen is EXE4's own code to read still: the
//! extraction fills it with placeholders and says so (`NOT_YET`).

pub(super) mod graphics;
mod hud;
pub(super) mod names;
pub(super) mod rom;

/// What the pack doesn't have of EXE4's yet, which a warning lists.
pub(crate) const NOT_YET: &str = "EXE4's custom screen isn't extracted yet (placeholders): its routines are EXE4's own, to read first (docs/design/exe4-map.md §14)";
