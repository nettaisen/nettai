//! The window: runs a session at the original's 59.73 frames a second and
//! shows it scaled up by an integer factor.

use crate::compose::{HEIGHT, WIDTH, to_rgb};
use crate::render::Renderer;
use crate::session::Session;
use bn6_battle::input::keys;
use minifb::{Key, KeyRepeat, Window, WindowOptions};
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
    (Key::X, keys::A),
    (Key::Z, keys::B),
    (Key::A, keys::L),
    (Key::S, keys::R),
    (Key::Enter, keys::START),
    (Key::Backspace, keys::SELECT),
];

pub const HELP: &str = "\
keys: arrows move, X = A, Z = B, A = L, S = R, Enter = START, Backspace = SELECT
      Space pause, . step one frame (paused), - / = slower / faster, F5 restart,
      H toggle the status line, Esc quit";

/// Show `sessions` one after another (a trace's rounds): a round that
/// runs out of input moves on to the next; one the engine stopped stays.
pub fn run(renderer: &mut Renderer, mut sessions: Vec<Session>, opts: &Options) -> Result<(), String> {
    if sessions.is_empty() {
        return Ok(());
    }
    let mut current = 0usize;
    let scale = opts.scale.max(1);
    let (w, h) = (WIDTH * scale, HEIGHT * scale);
    let mut window = Window::new("bn6-frontend", w, h, WindowOptions::default()).map_err(|e| e.to_string())?;
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
        let mut buttons = 0u16;
        for (k, b) in BUTTONS {
            if window.is_key_down(k) {
                buttons |= b;
            }
        }
        let mut single_step = false;
        for k in window.get_keys_pressed(KeyRepeat::Yes) {
            match k {
                Key::Space => paused = !paused,
                Key::Period => single_step = true,
                Key::Minus => speed = speed.saturating_sub(1),
                Key::Equal => speed = (speed + 1).min(SPEEDS.len() - 1),
                Key::F5 => {
                    session.restart();
                    renderer.reset();
                    reported = (false, false);
                }
                Key::H => status = !status,
                _ => {}
            }
        }
        let now = Instant::now();
        let dt = now.duration_since(last).min(Duration::from_millis(250));
        last = now;
        if !paused && session.stopped.is_none() {
            owed += dt.as_secs_f64() * FRAME_RATE * SPEEDS[speed];
            let n = owed.floor() as u32;
            owed -= n as f64;
            for _ in 0..n {
                if !session.step(buttons) {
                    break;
                }
                renderer.observe(&session.battle);
            }
        } else {
            owed = 0.0;
            if single_step && session.step(buttons) {
                renderer.observe(&session.battle);
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
        if let Some(p) = session.driver.prompt() {
            lines.push(p.to_string());
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
            crate::text::draw(&mut frame, WIDTH, 0, 0, &lines.join("\n"), 0x7FFF);
        }
        for y in 0..h {
            let row = &frame[(y / scale) * WIDTH..][..WIDTH];
            for x in 0..w {
                buffer[y * w + x] = to_rgb(row[x / scale]);
            }
        }
        if loops % 15 == 0 {
            let t = format!("bn6-frontend - {} - x{}{}", session.driver.position(), SPEEDS[speed], if paused { " (paused)" } else { "" });
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
    Ok(())
}
