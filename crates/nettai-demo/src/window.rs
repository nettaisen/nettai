//! The window (iced): the match editor, and the battle played in the same
//! window through the library's [`Player`]. A host loop over the player: it
//! gives the player the time that passed and the buttons held, shows its
//! picture at the window's own resolution (the frame scaled up by the
//! largest whole factor that fits; the font mode's text drawn at that
//! resolution), and plays its sound through the audio device. The keys are
//! the window's. Netplay's handshake runs in the same loop (`crate::net`,
//! polled each frame), so the window stays responsive while it waits.
//!
//! What the window shows of a battle is the picture the player presents
//! into when a tick ran (or the window or the language changed), at the
//! window's size ([`crate::picture`]): on iced's GPU renderer one texture,
//! written in place with each new picture, so memory stays flat. (Its
//! software renderer, the fallback, draws every pixel of a HiDPI window
//! itself and keeps up with half the display's rate.) The picture's widget
//! is the play's clock and keyboard as well (`picture::surface`): each frame
//! the battle advances before that frame is drawn. `NETTAI_PLAY_STATS`
//! prints the cost and the latency (`NETTAI_KEY_PROBE` presses a key now and
//! then to measure it); docs/frontend.md §7 has the measurements.

use crate::editor;
use crate::net::{Agreed, NetHandshake, Progress, Udp};
use crate::picture::{self, Picture};
use iced::widget::{container, text};
use iced::{Color, Element, Length, Size, Subscription, Task, keyboard, window};
use nettai_battle::input::keys;
use nettai_frontend::driver::{Driver, LivePlayer, NetStatus};
use nettai_frontend::game::{Game, Graphics, TextMode};
use nettai_frontend::player::Player;
use nettai_render::compose::{HEIGHT, WIDTH};
use nettai_render::vfont::TextRenderer;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const HELP: &str = "\
keys: arrows move, Z = A, X = B, A = L, S = R, Enter = START, Backspace = SELECT
      Space pause, . step one frame (paused), - / = slower / faster, F5 restart
      (not in netplay), [ / ] less / more present delay (netplay), Tab the next
      language, Esc stop (back to the editor, or quit)";

/// The most present delay netplay takes (a quarter of a second).
pub const MAX_PRESENT_DELAY: u32 = 15;

/// The font the window writes with: the frontend's bundled Murecho (Latin,
/// kana and kanji, for the Japanese names).
const FONT: &[u8] = nettai_render::vfont::BUNDLED;

/// How the battle is shown and played.
#[derive(Clone)]
pub struct PlayOptions {
    /// The window's size at the start, in screens (default 4: 960x640).
    pub scale: usize,
    pub start_paused: bool,
    /// Stop after this many ticks (smoke tests).
    pub quit_after: Option<u64>,
    /// The battle's text, and the font mode's font file.
    pub text: TextMode,
    pub font: Option<PathBuf>,
    /// No sound.
    pub mute: bool,
}

/// What the window starts on.
pub enum Start {
    /// The editor: a match file (`--edit`), or a new match whose game it
    /// asks.
    Edit,
    /// A battle to play (a match's set, a recording's rounds, one after
    /// another); the window closes when it is stopped.
    Play(Box<Play>),
    /// A netplay match being agreed: once it is, `then` makes its driver,
    /// played as `Play` is.
    Net(Box<Waiting>),
}

/// The languages the window cycles through (Tab): the content's, each one's
/// graphics loaded the first time it is shown and kept.
pub struct Languages {
    game: Game,
    names: Vec<String>,
    loaded: Vec<Option<Graphics>>,
    shown: usize,
}

impl Languages {
    /// The languages `game`'s content has strings in, with its own, showing
    /// `shown` (whose graphics these are).
    pub fn new(game: Game, shown: &str, graphics: Graphics) -> Languages {
        let own = nettai_content::locale::OWN.to_string();
        let mut names = vec![own];
        for lang in nettai_content::locale::languages(&game.dir) {
            if !names.contains(&lang) {
                names.push(lang);
            }
        }
        if !names.iter().any(|n| n == shown) {
            names.push(shown.to_string());
        }
        let at = names.iter().position(|n| n == shown).expect("the language shown is listed");
        let mut loaded: Vec<Option<Graphics>> = names.iter().map(|_| None).collect();
        loaded[at] = Some(graphics);
        Languages { game, names, loaded, shown: at }
    }

    /// The next language and its graphics; why it can't be shown (the pack
    /// has no lettering in it) leaves the one shown.
    fn next(&mut self) -> Result<(&str, &Graphics), String> {
        let at = (self.shown + 1) % self.names.len();
        if self.loaded[at].is_none() {
            self.loaded[at] = Some(self.game.graphics(&self.names[at]).map_err(|e| e.to_string())?);
        }
        self.shown = at;
        Ok((&self.names[at], self.loaded[at].as_ref().expect("loaded above")))
    }
}

/// A battle being played in the window.
pub struct Play {
    player: Player,
    /// What plays after (a recording's later rounds).
    rest: std::vec::IntoIter<Box<dyn Driver>>,
    languages: Languages,
    /// The GBA buttons the keys hold.
    held: u16,
    last: Option<Instant>,
    /// The picture at the window's resolution (0x00RRGGBB), and its size.
    buffer: Vec<u32>,
    size: (usize, usize),
    /// The picture shown (the buffer's, as of the last present).
    picture: Option<Arc<Picture>>,
    /// The picture is out of date: a tick ran, the window or the language
    /// changed (a display faster than the battle shows the last one again).
    stale: bool,
    /// Whether the divergence and the stop were said.
    reported: (bool, bool),
    samples: Vec<[f32; 2]>,
    quit_after: Option<u64>,
    title: String,
    stats: Option<Stats>,
    /// When the keys pressed since the last tick came (`NETTAI_PLAY_STATS`).
    pressed: Vec<Instant>,
    /// When the first of those a tick since the last picture saw came.
    seen: Option<Instant>,
}

impl Play {
    /// Play `first`, then each of `rest`, from `player`'s parts.
    pub fn new(mut player: Player, rest: Vec<Box<dyn Driver>>, languages: Languages, opts: &PlayOptions) -> Play {
        player.set_paused(opts.start_paused);
        Play {
            player,
            rest: rest.into_iter(),
            languages,
            held: 0,
            last: None,
            buffer: Vec::new(),
            size: (0, 0),
            picture: None,
            stale: true,
            reported: (false, false),
            samples: Vec::new(),
            quit_after: opts.quit_after,
            title: "nettai-demo".into(),
            stats: std::env::var_os("NETTAI_PLAY_STATS").map(|_| Stats::default()),
            pressed: Vec::new(),
            seen: None,
        }
    }

    /// A match of `game`, live: its set from `seed`.
    pub fn of_match(game: Game, m: &nettai_match::Match, seed: u32, lang: &str, opts: &PlayOptions) -> Result<Play, String> {
        let graphics = game.graphics(lang).map_err(|e| e.to_string())?;
        let font = nettai_frontend::game::font(opts.text, opts.font.as_deref()).map_err(|e| e.to_string())?;
        let renderer = graphics.renderer(opts.text, font.clone());
        let audio = if opts.mute {
            None
        } else {
            let sound = game.sound().map_err(|e| e.to_string())?;
            Some(nettai_audio::BattleAudio::with_banks(sound.banks, sound.songs))
        };
        let driver = LivePlayer::new(nettai_match::Set::of(&game.content, m, seed));
        let player = Player::with(renderer, font.map(TextRenderer::new), audio, Box::new(driver));
        Ok(Play::new(player, Vec::new(), Languages::new(game, lang, graphics), opts))
    }
}

/// A netplay match being agreed, and what plays it once it is.
pub struct Waiting {
    pub handshake: NetHandshake<Udp>,
    /// What the window says meanwhile.
    pub text: String,
    /// The agreed match's driver.
    pub then: Box<dyn FnOnce(Agreed<Udp>) -> Box<dyn Driver>>,
    /// The player's parts: the renderer, the font mode's text renderer and
    /// the audio.
    pub parts: (nettai_render::Renderer, Option<TextRenderer>, Option<nettai_audio::BattleAudio>),
    pub languages: Languages,
}

/// What `NETTAI_PLAY_STATS` counts: frames shown, the time each took to
/// present (the player's picture into the window's pixels and the image
/// handed to iced), and the longest wait between two frames.
#[derive(Default)]
struct Stats {
    since: Option<Instant>,
    frames: u32,
    presented: u32,
    present: Duration,
    worst_present: Duration,
    worst_gap: Duration,
    /// From a key's event to the tick that saw it.
    key_to_tick: picture::Times,
}

enum Screen {
    Editor,
    Play(Box<Play>),
    Waiting(Box<Waiting>),
}

pub struct Demo {
    editor: editor::App,
    screen: Screen,
    /// Close the window when a battle is stopped (the command line asked to
    /// play, not to edit).
    play_only: bool,
    options: PlayOptions,
    /// The audio device, opened once (none: --mute).
    device: Option<nettai_audio::Output>,
    /// The window's size (logical pixels) and its scale factor.
    size: Size,
    scale: f32,
    /// The window's last picture goes here as the window closes
    /// (`NETTAI_WINDOW_SHOT`).
    shot: Option<PathBuf>,
    screenshot_editor: bool,
    /// Present at the display's own density (`NETTAI_PHYSICAL_PIXELS`).
    physical: bool,
    /// Press a key now and then (`NETTAI_KEY_PROBE`).
    key_probe: bool,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Editor(editor::Msg),
    Frame(Instant),
    Key(keyboard::Event),
    Window(window::Event),
    Scale(f32),
}

/// Keyboard to GBA buttons.
fn button(key: &keyboard::Key) -> Option<u16> {
    use keyboard::key::Named;
    Some(match key.as_ref() {
        keyboard::Key::Named(Named::ArrowUp) => keys::UP,
        keyboard::Key::Named(Named::ArrowDown) => keys::DOWN,
        keyboard::Key::Named(Named::ArrowLeft) => keys::LEFT,
        keyboard::Key::Named(Named::ArrowRight) => keys::RIGHT,
        keyboard::Key::Named(Named::Enter) => keys::START,
        keyboard::Key::Named(Named::Backspace) => keys::SELECT,
        keyboard::Key::Character(c) => match c.to_ascii_lowercase().as_str() {
            "z" => keys::A,
            "x" => keys::B,
            "a" => keys::L,
            "s" => keys::R,
            _ => return None,
        },
        _ => return None,
    })
}

/// A netplay connection's figures, as the window's title says them.
fn net_line(n: &NetStatus) -> String {
    let ping = n.ping_ms.map_or("-".to_string(), |ms| format!("{ms:.0}ms"));
    format!(
        "ping {ping} loss {:.0}% present delay {} rollback {} (max {}, {} in all) waits {}",
        n.loss * 100.0,
        n.present_delay,
        n.last_rollback,
        n.max_rollback,
        n.rollbacks,
        n.waits
    )
}

impl Demo {
    fn new(editor: editor::App, start: Start, options: PlayOptions, screenshot_editor: bool) -> Demo {
        let play_only = !matches!(start, Start::Edit);
        let device = if options.mute || !play_only {
            None
        } else {
            match nettai_audio::Output::open() {
                Ok(d) => Some(d),
                Err(e) => {
                    eprintln!("no audio output: {e}");
                    std::process::exit(1);
                }
            }
        };
        let screen = match start {
            Start::Edit => Screen::Editor,
            Start::Play(p) => Screen::Play(p),
            Start::Net(w) => Screen::Waiting(w),
        };
        let size = if play_only {
            let s = options.scale.max(1) as f32;
            Size::new(WIDTH as f32 * s, HEIGHT as f32 * s)
        } else {
            Size::new(1180.0, 800.0)
        };
        Demo {
            editor,
            screen,
            play_only,
            options,
            device,
            size,
            scale: 1.0,
            shot: std::env::var_os("NETTAI_WINDOW_SHOT").map(PathBuf::from),
            screenshot_editor,
            physical: std::env::var_os("NETTAI_PHYSICAL_PIXELS").is_some(),
            key_probe: std::env::var_os("NETTAI_KEY_PROBE").is_some(),
        }
    }

    fn title(&self) -> String {
        match &self.screen {
            Screen::Editor => self.editor.title(),
            Screen::Play(p) => p.title.clone(),
            Screen::Waiting(w) => format!("nettai-demo - {}", w.text),
        }
    }

    /// Stop the battle: back to the editor, or close the window.
    fn stop(&mut self) -> Task<Msg> {
        if let Screen::Play(p) = &self.screen
            && let Some(path) = &self.shot
            && !p.buffer.is_empty()
        {
            let (w, h) = p.size;
            match nettai_render::present::write_rgb_png(path, &p.buffer, w, h) {
                Ok(()) => eprintln!("the window's last picture ({w}x{h}) is in {}", path.display()),
                Err(e) => eprintln!("can't write {}: {e}", path.display()),
            }
        }
        if self.play_only {
            return iced::exit();
        }
        self.screen = Screen::Editor;
        Task::none()
    }

    fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            // Play: the editor's match, in this window.
            Msg::Editor(editor::Msg::Play) => {
                let Some(e) = self.editor.editor.as_mut() else { return Task::none() };
                let Some((game, m, seed)) = e.to_play() else { return Task::none() };
                let lang = e.lang.code().to_string();
                if self.device.is_none() && !self.options.mute {
                    match nettai_audio::Output::open() {
                        Ok(d) => self.device = Some(d),
                        Err(err) => e.status = format!("no audio output ({err}): playing without sound"),
                    }
                }
                let mut opts = self.options.clone();
                opts.mute |= self.device.is_none();
                opts.quit_after = None;
                match Play::of_match(game, &m, seed, &lang, &opts) {
                    Ok(play) => {
                        eprintln!("{}", nettai_match::describe(&e.content, &m, seed, false, 0));
                        self.screen = Screen::Play(Box::new(play));
                    }
                    Err(why) => e.status = format!("can't play: {why}"),
                }
                Task::none()
            }
            Msg::Editor(m) => self.editor.update(m).map(Msg::Editor),
            Msg::Window(event) => {
                match event {
                    window::Event::Opened { size, .. } => {
                        self.size = size;
                        return window::latest().and_then(window::scale_factor).map(Msg::Scale);
                    }
                    window::Event::Resized(size) => self.size = size,
                    window::Event::Rescaled(f) => self.scale = f,
                    // (Keys held when the window loses the keyboard are let go.)
                    window::Event::Unfocused => {
                        if let Screen::Play(p) = &mut self.screen {
                            p.held = 0;
                        }
                    }
                    _ => {}
                }
                Task::none()
            }
            Msg::Scale(f) => {
                self.scale = f;
                Task::none()
            }
            Msg::Key(event) => self.key(event),
            Msg::Frame(now) => self.frame(now),
        }
    }

    fn key(&mut self, event: keyboard::Event) -> Task<Msg> {
        let Screen::Play(p) = &mut self.screen else {
            // (Esc while netplay is agreed stops waiting.)
            if let (Screen::Waiting(_), keyboard::Event::KeyPressed { key: keyboard::Key::Named(keyboard::key::Named::Escape), .. }) = (&self.screen, &event) {
                eprintln!("netplay: stopped waiting");
                return self.stop();
            }
            return Task::none();
        };
        match event {
            keyboard::Event::KeyPressed { key, repeat, .. } => {
                if let Some(b) = button(&key) {
                    if !repeat && p.held & b == 0 && p.stats.is_some() {
                        p.pressed.push(Instant::now());
                    }
                    p.held |= b;
                    return Task::none();
                }
                use keyboard::key::Named;
                // (Netplay runs in real time with the other player: the
                // player refuses a pause, another speed and a restart.)
                match key.as_ref() {
                    keyboard::Key::Named(Named::Escape) if !repeat => return self.stop(),
                    // (The language is the drawing's alone: it changes in
                    // netplay too, and the battle goes on.)
                    keyboard::Key::Named(Named::Tab) => match p.languages.next() {
                        Ok((name, graphics)) => {
                            p.player.set_language(graphics);
                            p.stale = true;
                            eprintln!("language: {name}");
                        }
                        Err(e) => eprintln!("{e}"),
                    },
                    keyboard::Key::Named(Named::Space) => {
                        let paused = p.player.paused();
                        p.player.set_paused(!paused);
                    }
                    keyboard::Key::Named(Named::F5) => {
                        if p.player.restart() {
                            p.reported = (false, false);
                            p.stale = true;
                        }
                    }
                    keyboard::Key::Character(".") => {
                        if !p.player.real_time() && (p.player.paused() || p.player.stopped().is_some()) {
                            p.player.tick(p.held);
                            p.stale = true;
                        }
                    }
                    keyboard::Key::Character("-") => {
                        p.player.slower();
                    }
                    keyboard::Key::Character("=") => {
                        p.player.faster();
                    }
                    // (Netplay's present delay: the player's own, changed
                    // during the match.)
                    keyboard::Key::Character(c @ ("[" | "]")) => {
                        if let Some(n) = p.player.net_status() {
                            let ticks = if c == "[" { n.present_delay.saturating_sub(1) } else { (n.present_delay + 1).min(MAX_PRESENT_DELAY) };
                            if p.player.set_present_delay(ticks) {
                                eprintln!("netplay: present delay {ticks}");
                            }
                        }
                    }
                    _ => {}
                }
            }
            keyboard::Event::KeyReleased { key, .. } => {
                if let Some(b) = button(&key) {
                    p.held &= !b;
                }
            }
            _ => {}
        }
        Task::none()
    }

    fn frame(&mut self, now: Instant) -> Task<Msg> {
        // Netplay: the handshake, polled each frame.
        if let Screen::Waiting(w) = &mut self.screen {
            match w.handshake.poll(Instant::now()) {
                Progress::Pending => return Task::none(),
                Progress::Failed(why) => {
                    eprintln!("netplay: {why}");
                    std::process::exit(1);
                }
                Progress::Agreed(agreed) => {
                    let Screen::Waiting(w) = std::mem::replace(&mut self.screen, Screen::Editor) else { unreachable!() };
                    let Waiting { then, parts: (renderer, text, audio), languages, .. } = *w;
                    let player = Player::with(renderer, text, audio, then(agreed));
                    self.screen = Screen::Play(Box::new(Play::new(player, Vec::new(), languages, &self.options)));
                }
            }
        }
        let Screen::Play(p) = &mut self.screen else { return Task::none() };
        if p.player.finished()
            && let Some(next) = p.rest.next()
        {
            p.player.play(next);
            p.reported = (false, false);
            p.stale = true;
        }
        let elapsed = p.last.map_or(Duration::ZERO, |last| now.saturating_duration_since(last));
        p.last = Some(now);
        if p.player.advance(elapsed, p.held) > 0 {
            p.stale = true;
            // (The keys pressed since the last tick: this one saw them.)
            if let Some(s) = &mut p.stats {
                for &at in &p.pressed {
                    s.key_to_tick.add(at.elapsed());
                }
            }
            p.seen = p.seen.or(p.pressed.first().copied());
            p.pressed.clear();
        }
        if let Some(out) = &self.device {
            p.samples.clear();
            p.player.take_samples(&mut p.samples);
            out.queue(&p.samples);
        }
        if let (Some(d), false) = (p.player.diverged(), p.reported.0) {
            eprintln!("{d}");
            p.reported.0 = true;
        }
        if let (Some(s), false) = (p.player.stopped(), p.reported.1) {
            eprintln!("{s}");
            p.reported.1 = true;
        }
        // The picture, at the window's size in logical pixels (as the
        // original window had it); a display of a higher density scales it
        // up. (`NETTAI_PHYSICAL_PIXELS`: at its own density, the font mode's
        // text sharper, four times the pixels to present on a Retina
        // display.)
        let started = Instant::now();
        let density = if self.physical { self.scale } else { 1.0 };
        let w = ((self.size.width * density).round() as usize).max(1);
        let h = ((self.size.height * density).round() as usize).max(1);
        if p.size != (w, h) {
            p.size = (w, h);
            p.buffer = vec![0u32; w * h];
            p.stale = true;
        }
        if p.stale {
            p.player.present(&mut p.buffer, w, h);
            p.picture = Some(Arc::new(Picture::new(w, h, p.buffer.clone(), p.seen.take())));
            p.stale = false;
            if let Some(s) = &mut p.stats {
                s.presented += 1;
            }
        }
        if let Some(s) = &mut p.stats {
            let took = started.elapsed();
            s.frames += 1;
            s.present += took;
            s.worst_present = s.worst_present.max(took);
            s.worst_gap = s.worst_gap.max(elapsed);
            let since = *s.since.get_or_insert(now);
            if now.duration_since(since) >= Duration::from_secs(2) {
                let (drawn, keys) = picture::take_shown();
                eprintln!(
                    "play stats: {} frames in {:.2?}, {} pictures presented ({w}x{h}): {:.2?} a picture (worst {:.2?}), longest between frames {:.2?}; \
                     {} drawn, {:.2?} after they were presented (worst {:.2?}); {} keys pressed: {:.2?} to the tick that saw it (worst {:.2?}), \
                     {:.2?} to that tick's picture drawn (worst {:.2?})",
                    s.frames,
                    now.duration_since(since),
                    s.presented,
                    s.present / s.presented.max(1),
                    s.worst_present,
                    s.worst_gap,
                    drawn.count,
                    drawn.mean(),
                    drawn.worst,
                    s.key_to_tick.count,
                    s.key_to_tick.mean(),
                    s.key_to_tick.worst,
                    keys.mean(),
                    keys.worst,
                );
                *s = Stats { since: Some(now), ..Stats::default() };
            }
        }
        // What the library leaves a host to say goes in the title: where
        // playback is, its speed, a pause, a netplay connection's figures
        // and the set's result. (Why it stopped, and a difference from a
        // recording, are on stderr above.)
        let mut t = format!("nettai-demo - {} - x{}{}", p.player.position(), p.player.speed(), if p.player.paused() { " (paused)" } else { "" });
        if let Some(n) = p.player.net_status() {
            t.push_str(&format!(" - {}", net_line(&n)));
        }
        match (p.player.stopped().and_then(|s| s.lines().next()), p.player.result()) {
            (Some(why), _) => t.push_str(&format!(" - {why}")),
            (None, Some(r)) => t.push_str(&format!(" - the match is over: {}", nettai_frontend::driver::result_text(r))),
            (None, None) => {}
        }
        p.title = t;
        if p.quit_after.is_some_and(|n| p.player.ticks() >= n) {
            return self.stop();
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Msg> {
        match &self.screen {
            Screen::Editor => editor::view::window(&self.editor).map(Msg::Editor),
            Screen::Play(p) => {
                let picture = picture::surface(p.picture.as_ref(), Msg::Frame, Msg::Key);
                container(picture).width(Length::Fill).height(Length::Fill).style(|_| container::background(Color::BLACK)).into()
            }
            Screen::Waiting(w) => container(text(w.text.clone()).size(18).color(Color::WHITE))
                .center(Length::Fill)
                .style(|_| container::background(Color::BLACK))
                .into(),
        }
    }

    fn subscription(&self) -> Subscription<Msg> {
        let window_events = window::events().map(|(_, e)| Msg::Window(e));
        match &self.screen {
            // (The play's surface is its clock and keyboard: `picture`.)
            Screen::Play(_) if self.key_probe => Subscription::batch([window_events, Subscription::run(key_probe)]),
            Screen::Play(_) => window_events,
            Screen::Waiting(_) => Subscription::batch([window::frames().map(Msg::Frame), keyboard::listen().map(Msg::Key), window_events]),
            Screen::Editor if self.screenshot_editor => {
                Subscription::batch([window::frames().map(|_| Msg::Editor(editor::Msg::Frame)), window_events])
            }
            Screen::Editor => window_events,
        }
    }
}

/// `NETTAI_KEY_PROBE`: the right arrow pressed every 300 to 400 ms (at
/// no particular point between two frames) and let go 100 ms later, as key
/// events come to the window, for `NETTAI_PLAY_STATS`'s figures of the time
/// from a key to the tick that saw it and to that tick's picture drawn.
fn key_probe() -> impl iced::futures::Stream<Item = Msg> {
    use iced::futures::SinkExt;
    use keyboard::key::{Code, Named, Physical};
    iced::stream::channel(4, async |output| {
        std::thread::spawn(move || {
            let mut output = output;
            let key = keyboard::Key::Named(Named::ArrowRight);
            let physical_key = Physical::Code(Code::ArrowRight);
            let modifiers = keyboard::Modifiers::empty();
            let location = keyboard::Location::Standard;
            for i in 0u64.. {
                std::thread::sleep(Duration::from_micros(300_000 + i * 7_919 % 100_000));
                let pressed = keyboard::Event::KeyPressed {
                    key: key.clone(),
                    modified_key: key.clone(),
                    physical_key,
                    location,
                    modifiers,
                    text: None,
                    repeat: false,
                };
                if iced::futures::executor::block_on(output.send(Msg::Key(pressed))).is_err() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
                let released = keyboard::Event::KeyReleased { key: key.clone(), modified_key: key.clone(), physical_key, location, modifiers };
                if iced::futures::executor::block_on(output.send(Msg::Key(released))).is_err() {
                    return;
                }
            }
        });
        std::future::pending::<()>().await
    })
}

/// Open the window on `start`, with `editor` (the editor's state: a match
/// opened, or the choice of a new one's game), until it is closed.
pub fn run(editor: editor::App, start: Start, options: PlayOptions) -> iced::Result {
    let screenshot_editor = editor.options.screenshot.is_some();
    let play_only = !matches!(start, Start::Edit);
    let size = if play_only {
        let s = options.scale.max(1) as f32;
        Size::new(WIDTH as f32 * s, HEIGHT as f32 * s)
    } else {
        Size::new(1180.0, 800.0)
    };
    let boot = std::cell::RefCell::new(Some((editor, start, options)));
    let start = move || {
        let (editor, start, options) = boot.borrow_mut().take().expect("the window boots once");
        Demo::new(editor, start, options, screenshot_editor)
    };
    iced::application(start, Demo::update, Demo::view)
        .title(Demo::title)
        .theme(|d: &Demo| editor::view::theme(&d.editor))
        .subscription(Demo::subscription)
        .font(FONT)
        .default_font(iced::Font::with_name("Murecho"))
        .settings(iced::Settings { default_text_size: iced::Pixels(15.0), ..iced::Settings::default() })
        .window_size(size)
        .run()
}
