//! The window: runs a session at the original's 59.73 frames a second and
//! shows it scaled up by the largest whole factor the window has room for
//! (`present`: resize it at will), with the font mode's text drawn at the
//! window's resolution.

use nettai_frontend::{Session, TickHook};
use nettai_render::Renderer;
use nettai_render::compose::{HEIGHT, WIDTH};
use nettai_render::present::{present, write_rgb_png};
use nettai_render::vfont::TextRenderer;
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use nettai_battle::input::keys;
use std::time::{Duration, Instant};

/// The original's frame rate.
pub const FRAME_RATE: f64 = 59.7275;

const SPEEDS: [f64; 8] = [0.125, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0];

pub struct Options {
    pub scale: usize,
    pub start_paused: bool,
    /// Close after this many ticks (smoke tests).
    pub quit_after: Option<u64>,
}

/// Keyboard to GBA buttons.
const BUTTONS: [(Key, u16); 10] = [
    (Key::Up, keys::UP),
    (Key::Down, keys::DOWN),
    (Key::Left, keys::LEFT),
    (Key::Right, keys::RIGHT),
    (Key::Z, keys::A),
    (Key::X, keys::B),
    (Key::A, keys::L),
    (Key::S, keys::R),
    (Key::Enter, keys::START),
    (Key::Backspace, keys::SELECT),
];

pub const HELP: &str = "\
keys: arrows move, Z = A, X = B, A = L, S = R, Enter = START, Backspace = SELECT
      Space pause, . step one frame (paused), - / = slower / faster, F5 restart
      (not in netplay), H toggle the status line, Esc quit";

/// Show `sessions` one after another (a trace's rounds): a round that
/// runs out of input moves on to the next; one the engine stopped stays.
/// `hooks` see the battle after every tick (sound); `text` draws the font
/// mode's text items.
pub fn run(
    renderer: &mut Renderer,
    mut sessions: Vec<Session>,
    hooks: &mut [Box<dyn TickHook>],
    opts: &Options,
    mut text: Option<&mut TextRenderer>,
) -> Result<(), String> {
    if sessions.is_empty() {
        return Ok(());
    }
    let mut current = 0usize;
    let scale = opts.scale.max(1);
    let (mut w, mut h) = (WIDTH * scale, HEIGHT * scale);
    let options = WindowOptions { resize: true, ..WindowOptions::default() };
    let mut window = Window::new("nettai-demo", w, h, options).map_err(|e| e.to_string())?;
    window.set_target_fps(120);
    let mut buffer = vec![0u32; w * h];
    let mut paused = opts.start_paused;
    let mut speed = 3usize;
    let mut status = true;
    let mut reported = (false, false);
    let mut owed = 0.0f64;
    let mut last = Instant::now();
    let mut title = String::new();
    let mut loops = 0u64;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        loops += 1;
        if sessions[current].finished && current + 1 < sessions.len() {
            current += 1;
            reported = (false, false);
            renderer.reset();
        }
        let session = &mut sessions[current];
        // Netplay runs in real time with the other player: no pause, no
        // other speed, no restart.
        let real_time = session.driver.real_time();
        let mut buttons = 0u16;
        for (k, b) in BUTTONS {
            if window.is_key_down(k) {
                buttons |= b;
            }
        }
        let mut single_step = false;
        for k in window.get_keys_pressed(KeyRepeat::Yes) {
            match k {
                Key::H => status = !status,
                _ if real_time => {}
                Key::Space => paused = !paused,
                Key::Period => single_step = true,
                Key::Minus => speed = speed.saturating_sub(1),
                Key::Equal => speed = (speed + 1).min(SPEEDS.len() - 1),
                Key::F5 => {
                    session.restart();
                    renderer.reset();
                    reported = (false, false);
                }
                _ => {}
            }
        }
        let now = Instant::now();
        let dt = now.duration_since(last).min(Duration::from_millis(250));
        last = now;
        // Run a step: follow what it showed, and hand it to the hooks.
        let mut step = |session: &mut Session| {
            if !session.step(buttons) {
                return false;
            }
            if session.new_round {
                renderer.reset();
            }
            if session.fresh {
                renderer.observe(&session.battle);
            }
            for h in hooks.iter_mut() {
                h.after_tick(session);
            }
            true
        };
        if !paused && session.stopped.is_none() {
            owed += dt.as_secs_f64() * FRAME_RATE * SPEEDS[speed];
            let n = owed.floor() as u32;
            owed -= n as f64;
            for _ in 0..n {
                if !step(session) {
                    break;
                }
            }
        } else {
            owed = 0.0;
            if single_step {
                step(session);
            }
        }
        if let (Some(d), false) = (&session.diverged, reported.0) {
            eprintln!("{d}");
            reported.0 = true;
        }
        if let (Some(s), false) = (&session.stopped, reported.1) {
            eprintln!("{s}");
            reported.1 = true;
        }

        let mut frame = renderer.render(&session.battle);
        let mut lines = Vec::new();
        if let Some(p) = session.driver.prompt(&session.battle) {
            lines.push(p);
        }
        if let Some(s) = session.driver.status().filter(|_| status) {
            lines.push(s);
        }
        if status && (paused || session.stopped.is_some()) {
            lines.push(format!(
                "{}{}  x{}",
                if paused { "PAUSED  " } else { "" },
                session.driver.position(),
                SPEEDS[speed]
            ));
        }
        if let Some(s) = &session.stopped {
            lines.push(s.clone());
        }
        if !lines.is_empty() {
            // (The status text is in front of everything, the text layer's
            // items too.)
            let text = lines.join("\n");
            nettai_frontend::text::draw(&mut frame.pixels, WIDTH, 0, 0, &text, 0x7FFF);
            // Only the boxes the lines are drawn on (`text::draw`'s: four
            // pixels a character and one more, six rows a line).
            let cols = nettai_frontend::text::columns(WIDTH).max(1);
            let mut y = 0;
            for line in text.lines() {
                let chars = line.chars().count();
                for n in (0..chars.max(1)).step_by(cols).map(|i| (chars - i.min(chars)).min(cols)) {
                    for row in y..(y + 6).min(HEIGHT) {
                        frame.depth[row * WIDTH..][..(n * 4 + 1).min(WIDTH)].fill(0);
                    }
                    y += 6;
                }
            }
        }
        let (ww, wh) = window.get_size();
        if (ww, wh) != (w, h) && ww > 0 && wh > 0 {
            (w, h) = (ww, wh);
            buffer = vec![0u32; w * h];
        }
        present(&frame, text.as_deref_mut(), &mut buffer, w, h);
        if loops % 15 == 0 {
            let t = format!("nettai-demo - {} - x{}{}", session.driver.position(), SPEEDS[speed], if paused { " (paused)" } else { "" });
            if t != title {
                window.set_title(&t);
                title = t;
            }
        }
        window.update_with_buffer(&buffer, w, h).map_err(|e| e.to_string())?;
        if opts.quit_after.is_some_and(|n| session.ticks >= n) {
            break;
        }
    }
    // What the window showed last, for a look without a screen grab.
    if let Some(path) = std::env::var_os("NETTAI_WINDOW_SHOT") {
        write_rgb_png(std::path::Path::new(&path), &buffer, w, h).map_err(|e| e.to_string())?;
        eprintln!("the window's last picture ({w}x{h}) is in {}", path.to_string_lossy());
    }
    Ok(())
}
