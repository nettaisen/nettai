//! EXE4's assets (Red Sun and Blue Moon, US and Japanese), from the
//! addresses the routine maps give (docs/design/exe4-map.md; the
//! verification workspace's tools/exe5/bmap.py and tools/exe4/emap.py pair
//! EXE4's routines with EXE6's and EXE5's, and their `romdata` the data
//! the paired routines load, in each of the four ROMs).
//!
//! What an EXE4 pack has so far: the battle sprites (the same archives in
//! all four ROMs), the chips' pictures and icons, the fonts and the HUD's
//! text lines in English (the US ROMs) and Japanese (the Japanese ROMs),
//! and the sound (the same songs in all four). The rest of what a battle
//! draws is EXE4's own code to read first (the custom screen, the banners,
//! the faces, the field, the backgrounds, the chatbox): the extraction
//! fills it with placeholders and says so (`NOT_YET`).

pub(super) mod graphics;
pub(super) mod names;
pub(super) mod rom;

/// What the pack doesn't have of EXE4's yet, which a warning lists.
pub(crate) const NOT_YET: &str = "EXE4's field, backgrounds, banners, emotion faces, chatbox and custom screen aren't extracted yet (placeholders): their routines are EXE4's own, to read first (docs/design/exe4-map.md §13)";
