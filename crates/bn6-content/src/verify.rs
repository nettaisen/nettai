//! Comparing what packs load into: battle data record by record, graphics
//! part by part, sprite timing frame for frame, songs command for command
//! (as timelines) and sample for sample (rendered PCM). `bn6-content
//! verify` uses these to check a pack against a reference pack (after an
//! editor round trip, say); the extractor checks its own export.

use crate::timeline::{self, Timeline};
use crate::timing::Timing;
use bn6_assets::Bundle;
use m4a::bank::{Song, SoundBank};
use m4a::{Driver, SongId};
use std::sync::Arc;

/// Differences between two graphics sets, by part (empty: identical).
pub fn compare_graphics(a: &Bundle, b: &Bundle) -> Vec<String> {
    let mut out = Vec::new();
    if a == b {
        return out;
    }
    if a.sprites.len() != b.sprites.len() {
        out.push(format!("{} sprites, {} after", a.sprites.len(), b.sprites.len()));
    }
    for (x, y) in a.sprites.iter().zip(&b.sprites) {
        let what = if (x.category, x.index) != (y.category, y.index) {
            "ids"
        } else if x.tilesets != y.tilesets {
            "tiles"
        } else if x.palette_sets != y.palette_sets {
            "palettes"
        } else if x.part_lists != y.part_lists {
            "layouts"
        } else if x.animations != y.animations {
            "animations"
        } else {
            continue;
        };
        out.push(format!("sprite {:02x}-{:02x}: {what} differ", x.category, x.index));
    }
    if a.field != b.field {
        out.push("field differs".into());
    }
    if a.backgrounds.len() != b.backgrounds.len() {
        out.push(format!("{} backgrounds, {} after", a.backgrounds.len(), b.backgrounds.len()));
    }
    for (i, (x, y)) in a.backgrounds.iter().zip(&b.backgrounds).enumerate() {
        if x != y {
            out.push(format!("background {i} differs"));
        }
    }
    if a.hud != b.hud {
        out.push("HUD differs".into());
    }
    out
}

/// Differences between two battle contents, by part (empty: identical).
pub fn compare_battle(a: &bn6_battle::Content, b: &bn6_battle::Content) -> Vec<String> {
    let mut out = Vec::new();
    if a.chips.len() != b.chips.len() {
        out.push(format!("{} chips, {} after", a.chips.len(), b.chips.len()));
    }
    for (x, y) in a.chips.iter().zip(&b.chips) {
        if x != y {
            out.push(format!("chip {:#05x} {} differs", x.id, x.name));
        }
    }
    let parts: [(&str, bool); 9] = [
        ("navis", a.navis == b.navis),
        ("forms", a.forms == b.forms),
        ("rules", a.rules == b.rules),
        ("objects", a.objects == b.objects),
        ("effects", a.effects == b.effects),
        ("sparks", a.sparks == b.sparks),
        ("regions", a.regions == b.regions),
        ("panel layouts", a.panel_layouts == b.panel_layouts),
        ("sprite timing", a.animations == b.animations),
    ];
    out.extend(parts.iter().filter(|p| !p.1).map(|p| format!("{} differ", p.0)));
    out
}

/// Where a pack's timing table disagrees with a bundle's animations.
pub fn compare_timing(t: &Timing, b: &Bundle) -> Vec<String> {
    let mut out = Vec::new();
    for s in &b.sprites {
        let want: Vec<Vec<(u8, u8)>> = s.animations.iter().map(|a| a.iter().map(|f| (f.duration, f.flags)).collect()).collect();
        let got: Option<Vec<Vec<(u8, u8)>>> = t
            .sprites
            .get(&(s.category, s.index))
            .map(|a| a.iter().map(|fr| fr.iter().map(|f| (f.ticks, f.flags)).collect()).collect());
        if got.as_ref() != Some(&want) {
            out.push(format!("sprite {:02x}-{:02x}: timing differs", s.category, s.index));
        }
    }
    if t.sprites.len() != b.sprites.len() {
        out.push(format!("{} sprites with timing, {} in the graphics", t.sprites.len(), b.sprites.len()));
    }
    out
}

/// A song's tracks as timelines with the song's loop, the form the MIDI
/// mapping writes; two songs the driver can't tell apart have equal ones.
pub fn song_timelines(s: &Song) -> Result<Vec<Timeline>, String> {
    let mut tls = Vec::new();
    for (i, t) in s.tracks.iter().enumerate() {
        tls.push(timeline::linearize(t).map_err(|e| format!("track {i}: {e}"))?);
    }
    crate::song::align_loops(&mut tls)?;
    Ok(tls)
}

/// Song-level differences: header fields and timelines.
pub fn compare_song(a: &Song, b: &Song) -> Option<String> {
    if (a.player, a.priority, a.reverb, a.voicegroup) != (b.player, b.priority, b.reverb, b.voicegroup) {
        return Some("header differs".into());
    }
    match (song_timelines(a), song_timelines(b)) {
        (Ok(x), Ok(y)) if x == y => None,
        (Ok(x), Ok(y)) => {
            let t = x.iter().zip(&y).position(|(p, q)| p != q).unwrap_or(x.len().min(y.len()));
            Some(format!("track {t}'s timeline differs"))
        }
        (Err(e), _) | (_, Err(e)) => Some(e),
    }
}

/// Render a song alone for `frames` frames.
pub fn render(bank: &Arc<SoundBank>, song: SongId, frames: usize) -> Vec<[f32; 2]> {
    let mut d = Driver::new(bank.clone());
    d.start(song);
    let mut out = Vec::with_capacity(frames * 560);
    for _ in 0..frames {
        d.step_frame();
        d.take_output(&mut out);
    }
    out
}

/// Render a schedule of song starts (frame, song) for `frames` frames:
/// songs on different players compete for channels.
pub fn render_mix(bank: &Arc<SoundBank>, starts: &[(usize, SongId)], frames: usize) -> Vec<[f32; 2]> {
    let mut d = Driver::new(bank.clone());
    let mut out = Vec::with_capacity(frames * 560);
    for f in 0..frames {
        d.step_frame();
        for &(at, s) in starts {
            if at == f {
                d.start(s);
            }
        }
        d.take_output(&mut out);
    }
    out
}

/// The first sample index where two renders differ (bit for bit).
pub fn first_difference(a: &[[f32; 2]], b: &[[f32; 2]]) -> Option<usize> {
    if a.len() != b.len() {
        return Some(a.len().min(b.len()));
    }
    a.iter().zip(b).position(|(x, y)| x[0].to_bits() != y[0].to_bits() || x[1].to_bits() != y[1].to_bits())
}
