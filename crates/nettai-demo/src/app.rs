//! The window: a host loop over the library's player. It gives the player
//! the time that passed and the buttons held, shows its picture scaled up by
//! the largest whole factor the window has room for (resize it at will; the
//! font mode's text is drawn at the window's resolution), and plays its
//! sound through the audio device. The keys are the window's.

use minifb::{Key, KeyRepeat, Window, WindowOptions};
use nettai_battle::input::keys;
use nettai_frontend::driver::Driver;
use nettai_frontend::player::Player;
use nettai_render::compose::{HEIGHT, WIDTH};
use nettai_render::present::write_rgb_png;
use std::time::Instant;

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

/// Show what `player` plays, then each of `rest` in turn (a recording's
/// later rounds): one that comes to its end moves on to the next; one the
/// engine stopped stays. `audio` plays the player's sound.
pub fn run(player: &mut Player, rest: Vec<Box<dyn Driver>>, audio: Option<&nettai_audio::Output>, opts: &Options) -> Result<(), String> {
    let mut rest = rest.into_iter();
    let scale = opts.scale.max(1);
    let (mut w, mut h) = (WIDTH * scale, HEIGHT * scale);
    let options = WindowOptions { resize: true, ..WindowOptions::default() };
    let mut window = Window::new("nettai-demo", w, h, options).map_err(|e| e.to_string())?;
    window.set_target_fps(120);
    let mut buffer = vec![0u32; w * h];
    player.set_paused(opts.start_paused);
    let mut reported = (false, false);
    let mut last = Instant::now();
    let mut title = String::new();
    let mut samples = Vec::new();
    let mut loops = 0u64;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        loops += 1;
        if player.finished()
            && let Some(next) = rest.next()
        {
            player.play(next);
            reported = (false, false);
        }
        let mut buttons = 0u16;
        for (k, b) in BUTTONS {
            if window.is_key_down(k) {
                buttons |= b;
            }
        }
        // (Netplay runs in real time with the other player: the player
        // refuses a pause, another speed and a restart.)
        let mut single_step = false;
        for k in window.get_keys_pressed(KeyRepeat::Yes) {
            match k {
                Key::H => player.show_status(!player.status_shown()),
                Key::Space => {
                    player.set_paused(!player.paused());
                }
                Key::Period => single_step = !player.real_time(),
                Key::Minus => {
                    player.slower();
                }
                Key::Equal => {
                    player.faster();
                }
                Key::F5 => {
                    if player.restart() {
                        reported = (false, false);
                    }
                }
                _ => {}
            }
        }
        let now = Instant::now();
        let elapsed = now.duration_since(last);
        last = now;
        if single_step && (player.paused() || player.stopped().is_some()) {
            player.tick(buttons);
        } else {
            player.advance(elapsed, buttons);
        }
        if let Some(out) = audio {
            samples.clear();
            player.take_samples(&mut samples);
            out.queue(&samples);
        }
        if let (Some(d), false) = (player.diverged(), reported.0) {
            eprintln!("{d}");
            reported.0 = true;
        }
        if let (Some(s), false) = (player.stopped(), reported.1) {
            eprintln!("{s}");
            reported.1 = true;
        }

        let (ww, wh) = window.get_size();
        if (ww, wh) != (w, h) && ww > 0 && wh > 0 {
            (w, h) = (ww, wh);
            buffer = vec![0u32; w * h];
        }
        player.present(&mut buffer, w, h);
        if loops % 15 == 0 {
            let t = format!("nettai-demo - {} - x{}{}", player.position(), player.speed(), if player.paused() { " (paused)" } else { "" });
            if t != title {
                window.set_title(&t);
                title = t;
            }
        }
        window.update_with_buffer(&buffer, w, h).map_err(|e| e.to_string())?;
        if opts.quit_after.is_some_and(|n| player.ticks() >= n) {
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
