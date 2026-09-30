//! Checks that a pack reads back as the data it was exported from:
//! graphics byte for byte, sprite timing frame for frame, songs command
//! for command (as timelines) and sample for sample (rendered PCM).

use crate::timeline::{self, Timeline};
use crate::timing::Timing;
use bn6_assets::Bundle;
use m4a::bank::{Song, SoundBank};
use m4a::{Driver, SongId};
use std::sync::Arc;

/// Differences between two bundles, by part (empty: identical bytes).
pub fn compare_graphics(a: &Bundle, b: &Bundle) -> Vec<String> {
    let mut out = Vec::new();
    if a.to_bytes() == b.to_bytes() {
        return out;
    }
    if a.sprites.len() != b.sprites.len() {
        out.push(format!("{} sprites, {} after", a.sprites.len(), b.sprites.len()));
    }
    for (x, y) in a.sprites.iter().zip(&b.sprites) {
        let bytes = |s: &bn6_assets::SpriteSheet| Bundle { sprites: vec![s.clone()], ..Default::default() }.to_bytes();
        if bytes(x) != bytes(y) {
            let what = if x.tilesets != y.tilesets {
                "tiles"
            } else if x.palette_sets != y.palette_sets {
                "palettes"
            } else if x.part_lists != y.part_lists {
                "layouts"
            } else {
                "animations"
            };
            out.push(format!("sprite {:02x}-{:02x}: {what} differ", x.category, x.index));
        }
    }
    let field = |f: &Bundle| Bundle { field: f.field.clone(), ..Default::default() }.to_bytes();
    if field(a) != field(b) {
        out.push("field differs".into());
    }
    for (i, (x, y)) in a.backgrounds.iter().zip(&b.backgrounds).enumerate() {
        let bg = |v: &Option<bn6_assets::Background>| Bundle { backgrounds: vec![v.clone()], ..Default::default() }.to_bytes();
        if bg(x) != bg(y) {
            out.push(format!("background {i} differs"));
        }
    }
    let hud = |h: &Bundle| Bundle { hud: h.hud.clone(), ..Default::default() }.to_bytes();
    if hud(a) != hud(b) {
        out.push("HUD differs".into());
    }
    if out.is_empty() {
        out.push("bundles differ".into());
    }
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
        out.push(format!("{} sprites with timing, {} in the bundle", t.sprites.len(), b.sprites.len()));
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
