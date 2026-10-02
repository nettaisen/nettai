//! Animation timing for the simulation, loaded at run time from a pack's
//! `graphics/sprites/*/animations.json` without reading any image.
//!
//! Effect lifetimes and attack timings end on these durations and flag
//! bits, so this is simulation data: the engine's content loader reads it
//! into a table keyed by (sprite, animation). So are the offsets of each
//! frame's parts (from `sprite.json`'s layouts), where objects read a
//! part's position as a point to attach to (`sub_80030BA`).

use crate::report::Report;
use crate::sprite::{self, AnimationsDoc};
use std::collections::BTreeMap;
use std::path::Path;

/// One frame: how long it shows, its flag byte (0x80 last, 0x40 loop,
/// other bits cues) and its layout (an index into the sprite's layouts).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameTiming {
    pub ticks: u8,
    pub flags: u8,
    pub layout: u16,
}

/// Every sprite's animations, by (category, index) then animation; and
/// every sprite's layouts, each its parts' offsets from the object in
/// order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Timing {
    pub sprites: BTreeMap<(u8, u8), Vec<Vec<FrameTiming>>>,
    pub layouts: BTreeMap<(u8, u8), Vec<Vec<[i8; 2]>>>,
}

/// The part of `sprite.json` the simulation reads: the layouts' part
/// offsets.
#[derive(serde::Deserialize)]
struct LayoutsDoc {
    layouts: Vec<Vec<PartOffset>>,
}

#[derive(serde::Deserialize)]
struct PartOffset {
    offset: [i8; 2],
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
            .map(|a| a.iter().map(|f| FrameTiming { ticks: f.ticks, flags: sprite::flags_byte(&f.flags), layout: f.layout }).collect())
            .collect();
        let key = (doc.sprite[0], doc.sprite[1]);
        t.sprites.insert(key, anims);
        let layouts_path = e.path().join("sprite.json");
        if layouts_path.is_file() {
            let name = format!("graphics/sprites/{}/sprite.json", e.file_name().to_string_lossy());
            let doc: LayoutsDoc = sprite::read_json(&layouts_path, &name, report)?;
            t.layouts.insert(key, doc.layouts.into_iter().map(|l| l.into_iter().map(|p| p.offset).collect()).collect());
        }
    }
    Some(t)
}
