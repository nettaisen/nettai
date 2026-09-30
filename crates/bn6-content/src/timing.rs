//! Animation timing for the simulation, loaded at run time from a pack's
//! `graphics/sprites/*/animations.json` without reading any image.
//!
//! Effect lifetimes and attack timings end on these durations and flag
//! bits, so this is simulation data: the engine's content loader reads it
//! into a table keyed by (sprite, animation).

use crate::report::Report;
use crate::sprite::{self, AnimationsDoc};
use std::collections::BTreeMap;
use std::path::Path;

/// One frame: how long it shows and its flag byte (0x80 last, 0x40 loop,
/// other bits cues).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameTiming {
    pub ticks: u8,
    pub flags: u8,
}

/// Every sprite's animations, by (category, index) then animation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Timing {
    pub sprites: BTreeMap<(u8, u8), Vec<Vec<FrameTiming>>>,
}

impl Timing {
    pub fn animation(&self, category: u8, index: u8, anim: usize) -> Option<&[FrameTiming]> {
        self.sprites.get(&(category, index))?.get(anim).map(Vec::as_slice)
    }
}

/// Read the timing of every sprite in a pack.
pub fn load(root: &Path, report: &mut Report) -> Option<Timing> {
    let mut t = Timing::default();
    let dir = root.join("graphics/sprites");
    let mut entries: Vec<_> = std::fs::read_dir(&dir).ok()?.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let path = e.path().join("animations.json");
        if !path.is_file() {
            continue;
        }
        let name = format!("graphics/sprites/{}/animations.json", e.file_name().to_string_lossy());
        let doc: AnimationsDoc = sprite::read_json(&path, &name, report)?;
        if doc.format != sprite::ANIMATIONS_FORMAT {
            report.error(&name, "not an animations file");
            continue;
        }
        let anims = doc
            .animations
            .iter()
            .map(|a| a.iter().map(|f| FrameTiming { ticks: f.ticks, flags: sprite::flags_byte(&f.flags) }).collect())
            .collect();
        t.sprites.insert((doc.sprite[0], doc.sprite[1]), anims);
    }
    Some(t)
}
