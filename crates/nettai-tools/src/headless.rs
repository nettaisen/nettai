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
/// r, up, down, left, right, start, select). Other ticks hold none. A
/// party's switch is asked for the same way, by the member: switch0 (the
/// side's own navi), switch1, switch2 (its partners).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyScript(Vec<(u32, u32, u16, Option<u8>)>);

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
            let (mut held, mut switch) = (0, None);
            for b in buttons.split('+') {
                let name = b.trim().to_ascii_lowercase();
                if let Some(member) = name.strip_prefix("switch") {
                    switch = Some(member.parse::<u8>().ok().filter(|&m| m < 3).ok_or_else(|| format!("no member {member:?} to switch to"))?);
                    continue;
                }
                held |= match name.as_str() {
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
            out.push((from, to, held, switch));
        }
        Ok(KeyScript(out))
    }

    /// The buttons held on tick `tick`.
    pub fn held(&self, tick: u32) -> u16 {
        self.0.iter().filter(|(a, b, ..)| (*a..=*b).contains(&tick)).fold(0, |k, (_, _, h, _)| k | h)
    }

    /// The party member a switch is asked to on tick `tick`, if any.
    pub fn switch(&self, tick: u32) -> Option<u8> {
        self.0.iter().filter(|(a, b, ..)| (*a..=*b).contains(&tick)).find_map(|(.., s)| *s)
    }

    /// One tick of `player` with what the script holds on it.
    fn tick(&self, player: &mut Player) -> bool {
        let tick = player.ticks() as u32 + 1;
        if let Some(member) = self.switch(tick) {
            player.ask_switch(member);
        }
        player.tick(self.held(tick))
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
/// (`Player::frame`: what a host shows), the enhanced mode's text items drawn
/// by its text renderer.
#[allow(clippy::too_many_arguments)]
/// Make and present each of `wanted` (the first round's) at `scale` times
/// the frame's size, writing none, and log how long each took on average
/// and at most, and the drawings' files read and rasterized (in all, and
/// from the second frame on: none while it plays).
pub fn time_frames(player: &mut Player, wanted: &BTreeSet<u32>, scale: usize, keys: &KeyScript, log: &mut dyn FnMut(&str)) {
    let scale = scale.max(1);
    let size = player.renderer().frame_size();
    let (w, h) = (size.width * scale, size.height * scale);
    let mut out = vec![0u32; w * h];
    let last = wanted.iter().next_back().copied().unwrap_or(0);
    let (mut made, mut shown, mut most, mut n) = (0f64, 0f64, 0f64, 0u32);
    let mut after_first = None;
    let start = nettai_assets::shapes::work_done();
    while keys.tick(player) {
        let Some(f) = player.session().frame else { continue };
        if wanted.contains(&f) {
            let t = std::time::Instant::now();
            let frame = player.frame();
            let t1 = t.elapsed().as_secs_f64();
            nettai_render::present::present(&frame, player.text(), &mut out, w, h);
            let t2 = t.elapsed().as_secs_f64() - t1;
            made += t1;
            shown += t2;
            most = most.max(t1 + t2);
            n += 1;
            if after_first.is_none() {
                after_first = Some(nettai_assets::shapes::work_done());
            }
        }
        if f >= last {
            break;
        }
    }
    let end = nettai_assets::shapes::work_done();
    let first = after_first.unwrap_or(end);
    let n = n.max(1) as f64;
    log(&format!(
        "{} frames at {scale}x: {:.2} ms a frame made, {:.2} ms presented, {:.2} ms at most; drawings read {} and rasterized {} in all, {} and {} after the first frame",
        n as u32,
        1000.0 * made / n,
        1000.0 * shown / n,
        1000.0 * most,
        end.0 - start.0,
        end.1 - start.1,
        end.0 - first.0,
        end.1 - first.1
    ));
}

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
    let _ = std::fs::remove_file(out.join("marks.tsv"));
    let _ = std::fs::remove_file(out.join("shifted.tsv"));
    let mut written = Vec::new();
    let last = wanted.iter().next_back().copied().unwrap_or(0);
    let mut rest = rest.into_iter();
    loop {
        while keys.tick(player) {
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
                // Where what the caller named is drawn (`marks.tsv`: frame,
                // x, y, width, height, what: `--mark`).
                if !player.renderer().problems.marks.is_empty() {
                    use std::io::Write;
                    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(out.join("marks.tsv"))?;
                    for m in &player.renderer().problems.marks {
                        writeln!(file, "{f}\t{}\t{}\t{}\t{}\t{}", m.x, m.y, m.width, m.height, m.what)?;
                    }
                }
                // What a recording's replay shows later than the original
                // (`shifted.tsv`: frame, what, frames late, x, y, width,
                // height): the custom screen held back on its OK or late
                // after it (the whole screen), the full gauge's stripes (the
                // gauge's place), for the frame comparison to compare as the
                // known shift (`Driver::feed_shift`).
                let shift = player.session().driver.feed_shift();
                if shift != Default::default() {
                    use nettai_render::compose::{HEIGHT, WIDTH};
                    use std::io::Write;
                    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(out.join("shifted.tsv"))?;
                    if shift.held {
                        writeln!(file, "{f}\theld\t0\t0\t0\t{WIDTH}\t{HEIGHT}")?;
                    }
                    if shift.late != 0 {
                        writeln!(file, "{f}\tlate\t{}\t0\t0\t{WIDTH}\t{HEIGHT}", shift.late)?;
                    }
                    if shift.hud_late != 0 {
                        let (x, y, w, h) = nettai_render::hud::GAUGE_AREA;
                        writeln!(file, "{f}\tgauge\t{}\t{x}\t{y}\t{w}\t{h}", shift.hud_late)?;
                    }
                }
                if objects {
                    for line in nettai_render::objects::describe(player.battle(), &Renderer::view(player.battle())) {
                        log(&format!("frame {f}: {line}"));
                    }
                    // What the frame's sprite draw took of its budget, and
                    // the sprites it had no room for.
                    let d = &player.renderer().sprite_drawn;
                    log(&format!(
                        "frame {f}: sprite draw: {} of {} tiles, {} of {} palette slots{}",
                        d.tiles,
                        d.tile_room,
                        d.palettes,
                        d.palette_slots,
                        if d.left_out.is_empty() { String::new() } else { format!("; left out: {}", d.left_out.join(", ")) }
                    ));
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
    /// What the frames and the cues named that the game's assets don't have.
    pub problems: Problems,
    /// Sessions the engine stopped, or that left their trace.
    pub stopped: Vec<String>,
}

/// Run sessions to their end, making every frame's lookups and checking
/// every tick's sound cues (with `sound`, the game's bank): everything a
/// battle shows and plays is asked of the game's assets, and what they
/// lack is collected (see [`nettai_render::audit`]). With `draw` every frame
/// is drawn and every cue played into nothing besides (the thorough audit,
/// `--audit --draw`); else only the lookups are made, which is what the
/// audit checks.
pub fn audit(renderer: &mut Renderer, sessions: Vec<Session>, sound: Option<std::sync::Arc<m4a::SoundBank>>, draw: bool) -> Audit {
    let mut out = Audit::default();
    renderer.problems.clear();
    renderer.set_lookups_only(!draw);
    let songs = nettai_audio::Songs::of(renderer.graphics().numbers());
    let mut audio = sound.clone().filter(|_| draw).map(|bank| nettai_audio::BattleAudio::new(bank, songs));
    let mut samples = Vec::new();
    for mut s in sessions {
        renderer.reset();
        renderer.console_version = s.driver.console_version();
        while s.step(0) {
            renderer.observe(&s.battle);
            renderer.problems.at(s.frame);
            renderer.render(&s.battle);
            out.frames += 1;
            let cues = s.battle.sound_cues();
            out.cues += cues.len() as u32;
            if let Some(bank) = &sound {
                for &cue in cues {
                    let numbers = renderer.graphics().numbers().clone();
                    crate::sound_lookups::check_cue(&s.battle, &numbers, bank, cue, &mut renderer.problems);
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
    pub graphics: nettai_render::graphics::GameGraphics,
    pub strings: Option<std::sync::Arc<nettai_content::locale::Strings>>,
    pub mode: nettai_render::GraphicsMode,
    pub font: Option<std::sync::Arc<nettai_render::vfont::VectorFont>>,
    /// The game's sound (none: cues aren't checked).
    pub sound: Option<std::sync::Arc<m4a::SoundBank>>,
    /// Draw every frame and play every cue too (`--draw`).
    pub draw: bool,
}

/// Audit each of `traces` (its rounds one after another, as [`audit`]) on
/// `jobs` threads, each trace with a renderer of its own over the shared
/// graphics; `done` hears of each as it ends. Returns them in `traces`' order.
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
        let rounds = crate::trace::trace_rounds(path, content, None).map_err(|e| format!("can't play it: {e}"))?;
        let sessions: Vec<Session> = rounds.into_iter().map(|(_, r)| Session::new(r)).collect();
        let mut renderer = Renderer::with_graphics(setup.graphics.clone());
        renderer.set_strings(setup.strings.clone());
        renderer.set_mode(setup.mode, setup.font.clone());
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
        let k = super::KeyScript::parse("10:switch1, 11:a+switch0").unwrap();
        assert_eq!((k.switch(9), k.switch(10), k.switch(11), k.held(11)), (None, Some(1), Some(0), keys::A));
        assert!(super::KeyScript::parse("3:switch3").is_err());
    }
}
