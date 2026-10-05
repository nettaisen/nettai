//! Without a window: chosen frames of a session written to PNG files
//! (`nettai_render::present::write_png`), and the trace audits.

use nettai_frontend::Session;
use nettai_frontend::driver::Driver;
use nettai_frontend::player::Player;
use nettai_render::Renderer;
use nettai_render::audit::Problems;
use nettai_render::present::write_png;
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

/// Run what `player` plays, then each of `rest` in turn (a recording's later
/// rounds), and write `frame_NNNNN.png` for each wanted frame. Returns the
/// frames written; a round that stops early (engine panic, end of trace)
/// moves on to the next.
pub fn render_frames(
    player: &mut Player,
    rest: Vec<Box<dyn Driver>>,
    wanted: &BTreeSet<u32>,
    out: &Path,
    scale: usize,
    log: &mut dyn FnMut(&str),
) -> std::io::Result<Vec<u32>> {
    render_frames_with(player, rest, wanted, out, scale, false, &KeyScript::default(), log)
}

/// [`render_frames`], and with `objects` every written frame's objects
/// (`objects::describe`) and text items (`textlayer::describe`) go to the
/// log as the renderer sees them; `keys` are the local player's buttons,
/// by tick (live play's). The frames are the player's own
/// (`Player::frame`: what a host shows), the font mode's text items drawn
/// by its text renderer.
#[allow(clippy::too_many_arguments)]
pub fn render_frames_with(
    player: &mut Player,
    rest: Vec<Box<dyn Driver>>,
    wanted: &BTreeSet<u32>,
    out: &Path,
    scale: usize,
    objects: bool,
    keys: &KeyScript,
    log: &mut dyn FnMut(&str),
) -> std::io::Result<Vec<u32>> {
    std::fs::create_dir_all(out)?;
    let _ = std::fs::remove_file(out.join("known.tsv"));
    let mut written = Vec::new();
    let last = wanted.iter().next_back().copied().unwrap_or(0);
    let mut rest = rest.into_iter();
    loop {
        while player.tick(keys.held(player.ticks() as u32 + 1)) {
            let Some(f) = player.session().frame else { continue };
            player.renderer().problems.at(Some(f));
            if wanted.contains(&f) {
                let frame = player.frame();
                write_png(&out.join(format!("frame_{f:05}.png")), &frame, scale, player.text())?;
                written.push(f);
                // Where the frame differs from the original on purpose
                // (`known.tsv`: frame, x, y, width, height, why), for the
                // frame comparison to leave out.
                if !player.renderer().problems.known.is_empty() {
                    use std::io::Write;
                    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(out.join("known.tsv"))?;
                    for k in &player.renderer().problems.known {
                        writeln!(file, "{f}\t{}\t{}\t{}\t{}\t{}", k.x, k.y, k.width, k.height, k.why)?;
                    }
                }
                if objects {
                    for line in nettai_render::objects::describe(player.battle(), &Renderer::view(player.battle())) {
                        log(&format!("frame {f}: {line}"));
                    }
                    for line in nettai_render::textlayer::describe(&frame) {
                        log(&format!("frame {f}: {line}"));
                    }
                }
            }
            if f >= last {
                break;
            }
        }
        if let Some(d) = player.diverged() {
            log(d);
        }
        if let Some(why) = player.stopped() {
            log(why);
        }
        if player.session().frame.is_some_and(|f| f >= last) {
            break;
        }
        let Some(next) = rest.next() else { break };
        player.play(next);
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

/// Run sessions to their end, making every frame's lookups and checking
/// every tick's sound cues (with `sound`, the packs' banks by `PackId`):
/// everything a battle shows and plays is asked of the packs, and what
/// they lack is collected (see [`nettai_render::audit`]). With `draw` every frame
/// is drawn and every cue played into nothing besides (the thorough audit,
/// `--audit --draw`); else only the lookups are made, which is what the
/// audit checks.
pub fn audit(renderer: &mut Renderer, sessions: Vec<Session>, sound: Option<Vec<std::sync::Arc<m4a::SoundBank>>>, draw: bool) -> Audit {
    let mut out = Audit::default();
    renderer.problems.clear();
    renderer.set_lookups_only(!draw);
    let songs = sessions.first().map(|s| nettai_audio::Songs::of(&s.battle.content.assets)).unwrap_or_default();
    let mut audio = sound.clone().filter(|_| draw).map(|banks| nettai_audio::BattleAudio::with_banks(banks, songs));
    let mut samples = Vec::new();
    for mut s in sessions {
        renderer.reset();
        renderer.console_region = s.driver.console_region();
        renderer.console_version = s.driver.console_version();
        while s.step(0) {
            renderer.observe(&s.battle);
            renderer.problems.at(s.frame);
            renderer.render(&s.battle);
            out.frames += 1;
            let cues = s.battle.sound_cues();
            out.cues += cues.len() as u32;
            if let Some(banks) = &sound {
                for &cue in cues {
                    crate::sound_lookups::check_cue(&s.battle, banks, cue, &mut renderer.problems);
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

/// One trace's audit ([`audit_traces`]).
#[derive(Clone, Debug)]
pub struct TraceAudit {
    pub trace: std::path::PathBuf,
    /// The audit, or why the trace couldn't be read or audited.
    pub audit: Result<Audit, String>,
}

/// What a renderer of [`audit_traces`] is made with.
pub struct AuditSetup {
    pub packs: nettai_render::packs::PackGraphics,
    pub strings: Option<std::sync::Arc<nettai_content::locale::Strings>>,
    pub text: nettai_render::textlayer::TextMode,
    pub font: Option<std::sync::Arc<nettai_render::vfont::VectorFont>>,
    /// The packs' sound, by `PackId` (none: cues aren't checked).
    pub sound: Option<Vec<std::sync::Arc<m4a::SoundBank>>>,
    /// Draw every frame and play every cue too (`--draw`).
    pub draw: bool,
}

/// Audit each of `traces` (its rounds one after another, as [`audit`]) on
/// `jobs` threads, each trace with a renderer of its own over the shared
/// packs; `done` hears of each as it ends. Returns them in `traces`' order.
pub fn audit_traces(
    content: &std::sync::Arc<nettai_battle::Content>,
    setup: &AuditSetup,
    traces: &[std::path::PathBuf],
    jobs: usize,
    done: &(dyn Fn(&TraceAudit) + Sync),
) -> Vec<TraceAudit> {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let next = AtomicUsize::new(0);
    let out: Mutex<Vec<Option<TraceAudit>>> = Mutex::new(vec![None; traces.len()]);
    let one = |path: &std::path::Path| -> Result<Audit, String> {
        let rounds = crate::trace::trace_rounds(path, content).map_err(|e| format!("can't play it: {e}"))?;
        let sessions: Vec<Session> = rounds.into_iter().map(|(_, r)| Session::new(r)).collect();
        let mut renderer = Renderer::with_packs(setup.packs.clone());
        renderer.set_strings(setup.strings.clone());
        renderer.set_text(setup.text, setup.font.clone());
        Ok(audit(&mut renderer, sessions, setup.sound.clone(), setup.draw))
    };
    std::thread::scope(|s| {
        for _ in 0..jobs.clamp(1, traces.len().max(1)) {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(path) = traces.get(i) else { break };
                    let audit = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| one(path)))
                        .unwrap_or_else(|p| Err(format!("the frontend panicked: {}", panic_text(&p))));
                    let t = TraceAudit { trace: path.clone(), audit };
                    done(&t);
                    out.lock().unwrap()[i] = Some(t);
                }
            });
        }
    });
    out.into_inner().unwrap().into_iter().flatten().collect()
}

/// A panic's message.
fn panic_text(p: &Box<dyn std::any::Any + Send>) -> String {
    p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_else(|| "?".into())
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
