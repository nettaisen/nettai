//! Rendering chosen frames to PNG files without a window.

use crate::audit::Problems;
use crate::compose::{HEIGHT, WIDTH};
use crate::render::{Frame, Renderer};
use crate::session::Session;
use crate::vfont::TextRenderer;
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

/// Buttons held on chosen ticks, for headless live play (`--keys`): a
/// list like "232-233:up,300:a+b" (ticks, then buttons by name: a, b, l,
/// r, up, down, left, right, start, select). Other ticks hold none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyScript(Vec<(u32, u32, u16)>);

impl KeyScript {
    pub fn parse(s: &str) -> Result<KeyScript, String> {
        use nettai_battle::input::keys;
        let mut out = Vec::new();
        for part in s.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let (ticks, buttons) = part.split_once(':').ok_or_else(|| format!("{part:?} is not TICKS:BUTTONS"))?;
            let num = |v: &str| v.trim().parse::<u32>().map_err(|_| format!("bad tick {v:?}"));
            let (from, to) = match ticks.split_once('-') {
                Some((a, b)) => (num(a)?, num(b)?),
                None => (num(ticks)?, num(ticks)?),
            };
            let mut held = 0;
            for b in buttons.split('+') {
                held |= match b.trim().to_ascii_lowercase().as_str() {
                    "a" => keys::A,
                    "b" => keys::B,
                    "l" => keys::L,
                    "r" => keys::R,
                    "up" => keys::UP,
                    "down" => keys::DOWN,
                    "left" => keys::LEFT,
                    "right" => keys::RIGHT,
                    "start" => keys::START,
                    "select" => keys::SELECT,
                    other => return Err(format!("no button {other:?}")),
                };
            }
            out.push((from, to, held));
        }
        Ok(KeyScript(out))
    }

    /// The buttons held on tick `tick`.
    pub fn held(&self, tick: u32) -> u16 {
        self.0.iter().filter(|(a, b, _)| (*a..=*b).contains(&tick)).fold(0, |k, (_, _, h)| k | h)
    }
}

/// Write a frame as an RGB PNG, scaled up by an integer factor, with its
/// text items drawn at that scale by `text` (`present`).
pub fn write_png(path: &Path, frame: &Frame, scale: usize, text: Option<&mut TextRenderer>) -> std::io::Result<()> {
    let scale = scale.max(1);
    let (w, h) = (WIDTH * scale, HEIGHT * scale);
    let mut out = vec![0u32; w * h];
    crate::present::present(frame, text, &mut out, w, h);
    write_rgb_png(path, &out, w, h)
}

/// Write 0RGB pixels, `w` by `h`, as an RGB PNG.
pub fn write_rgb_png(path: &Path, out: &[u32], w: usize, h: usize) -> std::io::Result<()> {
    let rgb: Vec<u8> = out.iter().flat_map(|&c| [(c >> 16) as u8, (c >> 8) as u8, c as u8]).collect();
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
    render_frames_with(renderer, sessions, wanted, out, scale, false, &KeyScript::default(), None, log)
}

/// [`render_frames`], and with `objects` every written frame's objects
/// (`objects::describe`) and text items (`textlayer::describe`) go to the
/// log as the renderer sees them; `keys` are the local player's buttons,
/// by tick (live play's); `text` draws the font mode's text items.
#[allow(clippy::too_many_arguments)]
pub fn render_frames_with(
    renderer: &mut Renderer,
    sessions: Vec<Session>,
    wanted: &BTreeSet<u32>,
    out: &Path,
    scale: usize,
    objects: bool,
    keys: &KeyScript,
    mut text: Option<&mut TextRenderer>,
    log: &mut dyn FnMut(&str),
) -> std::io::Result<Vec<u32>> {
    std::fs::create_dir_all(out)?;
    let _ = std::fs::remove_file(out.join("known.tsv"));
    let mut written = Vec::new();
    let last = wanted.iter().next_back().copied().unwrap_or(0);
    for mut s in sessions {
        renderer.reset();
        renderer.console_region = s.driver.console_region();
        while s.step(keys.held(s.ticks as u32 + 1)) {
            renderer.observe(&s.battle);
            let Some(f) = s.frame else { continue };
            renderer.problems.at(Some(f));
            if wanted.contains(&f) {
                let frame = renderer.render(&s.battle);
                write_png(&out.join(format!("frame_{f:05}.png")), &frame, scale, text.as_deref_mut())?;
                written.push(f);
                // Where the frame differs from the original on purpose
                // (`known.tsv`: frame, x, y, width, height, why), for the
                // frame comparison to leave out.
                if !renderer.problems.known.is_empty() {
                    use std::io::Write;
                    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(out.join("known.tsv"))?;
                    for k in &renderer.problems.known {
                        writeln!(file, "{f}\t{}\t{}\t{}\t{}\t{}", k.x, k.y, k.width, k.height, k.why)?;
                    }
                }
                if objects {
                    for line in crate::objects::describe(&s.battle, &Renderer::view(&s.battle)) {
                        log(&format!("frame {f}: {line}"));
                    }
                    for line in crate::textlayer::describe(&frame) {
                        log(&format!("frame {f}: {line}"));
                    }
                }
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
/// tick's sound cues into nothing (with `sound`, the packs' banks by
/// `PackId`): everything a battle shows and plays is asked of the packs
/// once, and what they lack is collected (see [`crate::audit`]).
pub fn audit(renderer: &mut Renderer, sessions: Vec<Session>, sound: Option<Vec<std::sync::Arc<m4a::SoundBank>>>) -> Audit {
    let mut out = Audit::default();
    renderer.problems.clear();
    let songs = sessions.first().map(|s| nettai_audio::Songs::of(&s.battle.content.assets)).unwrap_or_default();
    let mut audio = sound.clone().map(|banks| nettai_audio::BattleAudio::with_banks(banks, songs));
    let mut samples = Vec::new();
    for mut s in sessions {
        renderer.reset();
        renderer.console_region = s.driver.console_region();
        while s.step(0) {
            renderer.observe(&s.battle);
            renderer.problems.at(s.frame);
            renderer.render(&s.battle);
            out.frames += 1;
            let cues = s.battle.sound_cues();
            out.cues += cues.len() as u32;
            if let Some(banks) = &sound {
                for &cue in cues {
                    crate::audit::check_cue(&s.battle, banks, cue, &mut renderer.problems);
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

    #[test]
    fn key_scripts() {
        use nettai_battle::input::keys;
        let k = super::KeyScript::parse("232-233:up, 300:a+b").unwrap();
        assert_eq!((k.held(231), k.held(232), k.held(233), k.held(300)), (0, keys::UP, keys::UP, keys::A | keys::B));
        assert!(super::KeyScript::parse("3:jump").is_err());
    }
}
