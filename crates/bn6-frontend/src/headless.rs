//! Rendering chosen frames to PNG files without a window.

use crate::audit::Problems;
use crate::compose::{HEIGHT, WIDTH, to_rgb};
use crate::render::Renderer;
use crate::session::Session;
use std::collections::BTreeSet;
use std::path::Path;

/// Parse a frame list like "150,300,600" or "100-120,500".
pub fn parse_frames(s: &str) -> Result<BTreeSet<u32>, String> {
    let mut set = BTreeSet::new();
    for part in s.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let num = |v: &str| v.parse::<u32>().map_err(|_| format!("bad frame number {v:?}"));
        match part.split_once('-') {
            Some((a, b)) => set.extend(num(a)?..=num(b)?),
            None => {
                set.insert(num(part)?);
            }
        }
    }
    Ok(set)
}

/// Write a BGR555 frame as an RGB PNG, scaled up by an integer factor.
pub fn write_png(path: &Path, frame: &[u16], scale: usize) -> std::io::Result<()> {
    let scale = scale.max(1);
    let (w, h) = (WIDTH * scale, HEIGHT * scale);
    let mut rgb = Vec::with_capacity(w * h * 3);
    for y in 0..h {
        for x in 0..w {
            let c = to_rgb(frame[(y / scale) * WIDTH + x / scale]);
            rgb.extend_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8]);
        }
    }
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut enc = png::Encoder::new(file, w as u32, h as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().map_err(std::io::Error::other)?;
    writer.write_image_data(&rgb).map_err(std::io::Error::other)?;
    Ok(())
}

/// Run sessions in order and write `frame_NNNNN.png` for each wanted
/// trace frame. Returns the frames written; a session that stops early
/// (engine panic, end of trace) moves on to the next.
pub fn render_frames(
    renderer: &mut Renderer,
    sessions: Vec<Session>,
    wanted: &BTreeSet<u32>,
    out: &Path,
    scale: usize,
    log: &mut dyn FnMut(&str),
) -> std::io::Result<Vec<u32>> {
    std::fs::create_dir_all(out)?;
    let mut written = Vec::new();
    let last = wanted.iter().next_back().copied().unwrap_or(0);
    for mut s in sessions {
        renderer.reset();
        while s.step(0) {
            renderer.observe(&s.battle);
            let Some(f) = s.frame else { continue };
            renderer.problems.at(Some(f));
            if wanted.contains(&f) {
                let frame = renderer.render(&s.battle);
                write_png(&out.join(format!("frame_{f:05}.png")), &frame, scale)?;
                written.push(f);
            }
            if f >= last {
                break;
            }
        }
        if let Some(d) = &s.diverged {
            log(d);
        }
        if let Some(why) = &s.stopped {
            log(why);
        }
        if s.frame.is_some_and(|f| f >= last) {
            break;
        }
    }
    Ok(written)
}

/// What an audit found.
#[derive(Clone, Debug, Default)]
pub struct Audit {
    /// Frames drawn.
    pub frames: u32,
    /// Sound cues played.
    pub cues: u32,
    /// What the frames and the cues named that the pack doesn't have.
    pub problems: Problems,
    /// Sessions the engine stopped, or that left their trace.
    pub stopped: Vec<String>,
}

/// Run sessions to their end, drawing every frame and playing every
/// tick's sound cues into nothing (with `sound`, the pack's): everything a
/// battle shows and plays is asked of the pack once, and what it lacks is
/// collected (see [`crate::audit`]).
pub fn audit(renderer: &mut Renderer, sessions: Vec<Session>, sound: Option<std::sync::Arc<m4a::SoundBank>>) -> Audit {
    let mut out = Audit::default();
    renderer.problems.clear();
    let mut audio = sound.clone().map(bn6_audio::BattleAudio::new);
    let mut samples = Vec::new();
    for mut s in sessions {
        renderer.reset();
        while s.step(0) {
            renderer.observe(&s.battle);
            renderer.problems.at(s.frame);
            renderer.render(&s.battle);
            out.frames += 1;
            let cues = s.battle.sound_cues();
            out.cues += cues.len() as u32;
            if let Some(bank) = &sound {
                for &cue in cues {
                    crate::audit::check_cue(&s.battle, bank, cue, &mut renderer.problems);
                }
            }
            if let Some(a) = &mut audio {
                a.handle(cues);
                samples.clear();
                a.tick(&mut samples);
            }
        }
        out.stopped.extend(s.diverged.clone());
        out.stopped.extend(s.stopped.clone().filter(|_| !s.finished));
    }
    out.problems = std::mem::take(&mut renderer.problems);
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn frame_lists() {
        let s = super::parse_frames("3, 10-12,7").unwrap();
        assert_eq!(s.into_iter().collect::<Vec<_>>(), vec![3, 7, 10, 11, 12]);
        assert!(super::parse_frames("x").is_err());
    }
}
