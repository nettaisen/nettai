//! nettai-demo: edit a match and play it, play a match file (alone or over
//! the network), or watch a golden trace replayed through the engine. The
//! desktop program over the nettai-frontend library: this is its command
//! line. See docs/frontend.md.

use nettai_demo::trace::trace_rounds;
use nettai_demo::net::{Agreed, Link, NetHandshake, Waits};
use nettai_demo::window::{self, Play, PlayOptions, Start, Waiting};
use nettai_demo::{editor, headless};
use nettai_frontend::driver::{Driver, LivePlayer};
use nettai_frontend::game::{Failed, Found, Game, LoadError, Sound};
use nettai_frontend::player::Player;
use nettai_frontend::session;
use nettai_render::textlayer::TextMode;
use nettai_render::vfont::TextRenderer;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

struct Args {
    /// Packs given by directory, each in place of the found one of its game
    /// (docs/design/rules-in-luau.md §7.4).
    packs: Vec<PathBuf>,
    content: Option<PathBuf>,
    mute: bool,
    /// The trace (several with --audit).
    traces: Vec<PathBuf>,
    round: usize,
    /// Edit this match file (none, and nothing else to do: a new match).
    edit: Option<PathBuf>,
    /// The editor's pane to start on, and a screenshot of it to write.
    tab: Option<String>,
    screenshot: Option<PathBuf>,
    /// Play this match file; write the match played to that one.
    match_file: Option<PathBuf>,
    save_match: Option<PathBuf>,
    show_folders: bool,
    keys: Option<String>,
    scale: usize,
    paused: bool,
    headless: Option<String>,
    audit: bool,
    /// With --audit: draw every frame and play every cue as well.
    draw: bool,
    /// Audit traces on this many threads (0: every core's).
    jobs: usize,
    /// With --audit: write each trace's lookups here.
    lookups: Option<PathBuf>,
    audit_content: bool,
    objects: bool,
    /// With --headless: what to mark where it is drawn (`Problems::marking`).
    marks: Vec<String>,
    out: PathBuf,
    png_scale: usize,
    quit_after: Option<u64>,
    text: TextMode,
    font: Option<PathBuf>,
    lang: String,
    /// Netplay: meet in this room of the signaling server, or directly:
    /// host on this UDP port, or join this host.
    room: Option<String>,
    signal: Option<String>,
    host: Option<u16>,
    join: Option<String>,
    /// The STUN servers (none: the default; "none": no STUN), and a TURN
    /// server with its credentials.
    stun: Vec<String>,
    turn: Option<String>,
    turn_user: String,
    turn_pass: String,
    present_delay: u32,
    wait: u64,
    /// Write a replay of the set played (live or netplay) to this file.
    record: Option<PathBuf>,
    /// Play this replay; shown from this side (none: the recorder's).
    replay: Option<PathBuf>,
    side: Option<u8>,
}

/// The most present delay netplay takes (a quarter of a second).
pub const MAX_PRESENT_DELAY: u32 = nettai_demo::window::MAX_PRESENT_DELAY;

const USAGE: &str = "\
usage: nettai-demo [OPTIONS]                 edit a new match (the window asks its game)
       nettai-demo [OPTIONS] --edit FILE     edit a match file
       nettai-demo [OPTIONS] TRACE.jsonl     watch a trace's rounds
       nettai-demo [OPTIONS] --match FILE    play a match file (you are its left side)
       nettai-demo [OPTIONS] --match FILE --room CODE        play another player over the
       nettai-demo [OPTIONS] --match FILE --host PORT        network: meet in a room, or
       nettai-demo [OPTIONS] --match FILE --join ADDR:PORT   host, or join the host
       nettai-demo [OPTIONS] --replay FILE   watch a replay (--record writes one)
       nettai-demo [OPTIONS] TRACE.jsonl --headless FRAMES [--out DIR] [--png-scale N]
       nettai-demo [OPTIONS] --match FILE --audit-content
       nettai-demo [OPTIONS] --audit TRACE.jsonl...

  You play one game, EXE6 or EXE5: a match file names its game and a trace
  states its own. Live play's setup comes from the match file, which the
  editor makes (with no trace or match to play, the window opens on it; its
  Play plays the match in the same window, Esc comes back). The battle is that game's:
  its content folder and the support folders it uses, drawn and heard
  from its pack (graphics and sound, written from any subset of your ROMs
  by `nettai-extract <exe5|exe6> <pack-dir> [ROM ...]`), found in the packs
  directory, $NETTAI_PACKS, else
  data, each pack by the game it says.
  --pack DIR       a pack's directory, in place of the found pack of its game
  --content DIR    the content directory (default: $NETTAI_CONTENT, else
                   this repository's content/)
  --mute           no sound (headless rendering never plays any)
  --round N        the trace round to start with (default 1; later rounds follow)
  --edit FILE      open the editor on this match file (docs/frontend.md §6;
                   README.md, \"The match editor\")
  --tab NAME       with the editor: start on a pane: arena, or left- or right-
                   and navi, folder, auto-battle, stats, or a pane of the
                   side's setup (navicust, patch-cards, sp-times; a list of
                   definitions by its name: exe6's crosses, exe5's souls)
  --screenshot PNG with the editor: write the window to PNG once it has drawn,
                   and quit
  --match FILE     play the match this file sets up (docs/frontend.md §6: its
                   game, its rounds, each side's navi, version,
                   folder, Crosses, patch cards and stats, by name in the
                   game; the editor makes them); you are its left side.
                   Its seed sets the battle's RNG (else from the clock).
                   With --audit-content, select the game's content to audit.
                   With --room, --host or --join the left side
                   is what you bring, and both files must state the same
                   game and rounds (else it stops, saying what differs)
  --save-match FILE  write the match played with its seed (the file's setup,
                   or the one netplay agreed) to FILE as a match file,
                   to play again or edit
  --show-folders   print both players' live folders
  --scale N        the window's size when it opens to play, in screens
                   (default 4); resize it at will
  --paused         start paused
  --headless F     render frames F (e.g. 150,300,600 or 100-120; trace frame
                   numbers, or ticks in live play) to frame_NNNNN.png files
                   in --out (default .), no window
  --objects        with --headless: list every rendered frame's objects (kind,
                   place, sprite, animation, look)
  --mark M,...     with --headless: write where each of these is drawn, by
                   frame, to marks.tsv in --out: sprite:NAME (an object drawn
                   with it), background:NAME, chip:KEY (its picture in the
                   chip window), chip-window-at-close (the HUD's chip name
                   before the navi's first decision); for a frame comparison
                   that knows what a console shows otherwise there
  --keys K         with --headless --match: the buttons you hold, by tick (e.g.
                   232-233:up,300:a+b; a b l r up down left right start
                   select)
  --audit-content  with --match FILE: make every lookup the drawing code
                   and audio make for everything the content defines (every chip's icon,
                   picture and name, every navi's and form's face, every
                   asset the content names), in every language, and list
                   what the packs don't have; exits 1 if there was any
  --audit          run the traces (several at a time) making each frame's
                   lookups and checking each sound cue, no window, and list
                   what they named that the packs don't have (a sprite, an
                   animation, a palette, a chip's icon or name glyph, a
                   banner, a song); exits 1 if there was any
  --draw           with --audit: draw every frame and play every sound cue
                   into nothing as well (slower)
  --jobs N         with --audit: traces at a time (default: one a core)
  --lookups FILE   with --audit: write each trace's lookups to FILE (a line
                   each: the trace, a tab, the lookup), for the trace cover;
                   with --audit-content, its own (as the trace \"content\")
  --text MODE      how strings are drawn: font (default) draws the names, the
                   telop, the chatbox and the HUD's lines with a vector font at
                   the window's resolution, over the scaled frame; original
                   draws them in the game's own fonts into the frame, as the
                   original does (what the frame comparison uses)
  --font PATH      the font mode's font (a TrueType or OpenType file) instead
                   of the bundled one (Murecho)
  --lang LANG      the language of the battle's text: en (default, the
                   content's own) or ja (the Japanese games' names,
                   descriptions and messages, from the content's
                   locales/ja.toml, and their fonts and pictures with text,
                   from the pack); either text mode. Only what is shown
                   changes: the battle, and a netbattle with a player of
                   another language, are the same
  --quit-after N   close the window after N ticks (NETTAI_PLAY_STATS: print
                   what each frame costs to show; NETTAI_WINDOW_SHOT=PNG: the
                   last picture shown)
  --room CODE      netplay: meet the other player in this room of the
                   signaling server (--signal), each giving the same code
                   (1 to 64 letters, digits, _ and -); the first in hosts,
                   the left navi. A WebRTC connection, through NATs (STUN,
                   and TURN if given)
  --signal URL     with --room: the signaling server (ws://, wss://;
                   default: $NETTAI_SIGNAL); signaling/ is one, a Cloudflare
                   Worker (`npx wrangler dev` runs it at ws://127.0.0.1:8787)
  --stun URL       with --room: a STUN server (stun:HOST[:PORT]; again for
                   more; default stun:stun.l.google.com:19302; none: no STUN)
  --turn URL       with --room: a TURN server to relay through when no
                   direct path is found (turn:HOST[:PORT]), with
  --turn-user U    its credentials (Cloudflare's TURN service hands out
  --turn-pass P    short-lived ones from its API: get a pair, give it here)
  --host PORT      netplay, directly: host a match on this UDP port (forward
                   it on your router to play over the Internet) and wait
                   for a player to join; you are the left navi
  --join ADDR:PORT netplay, directly: join the match hosted there; you are
                   the right navi, seen from your side. Direct connect
                   authenticates nobody (as plain UDP doesn't); a room's
                   connection checks the certificates it was told of.
                   Both players need the same engine, game and content
                   (the handshake checks); each brings their match file's
                   left side, both files stating the same rounds, and the
                   battle's RNG comes from both players' randomly generated
                   seed halves. A connection that drops is made again (the
                   battle waits, then goes on); after 30 seconds the match
                   ends
  --present-delay N  netplay: show the battle N frames behind your newest
                   input (default 0: the newest, the other player's input
                   predicted and corrected by rollback; more: fewer
                   corrections seen, your own input that much later); [ and
                   ] change it during the match; yours alone, the other
                   player chooses theirs
  --wait SECONDS   how long the host waits for a player, or the joiner for
                   the host (default 300 and 30; in a room, by the role it
                   gets)
  --record FILE    with --match (alone or netplay): write the set
                   played to FILE as a replay (docs/frontend.md §8: the
                   match and every tick's buttons, both players'; netplay
                   writes each tick as it settles, so both players' replays
                   are the same but for who recorded)
  --replay FILE    play a replay: the set again, round by round, on the
                   engine and content it was made with (refused on others),
                   checked against the digests it kept; with --headless F,
                   render its frames F (ticks of the set)
  --side SIDE      with --replay: the console shown, left or right
                   (default: the side that recorded it)";

fn parse(mut it: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut a = Args {
        packs: Vec::new(),
        content: None,
        mute: false,
        traces: Vec::new(),
        round: 1,
        edit: None,
        tab: None,
        screenshot: None,
        match_file: None,
        save_match: None,
        show_folders: false,
        keys: None,
        scale: 4,
        paused: false,
        headless: None,
        audit: false,
        draw: false,
        jobs: 0,
        lookups: None,
        audit_content: false,
        objects: false,
        marks: Vec::new(),
        out: PathBuf::from("."),
        png_scale: 1,
        quit_after: None,
        text: TextMode::Font,
        font: None,
        lang: nettai_assets::BASE_LANGUAGE.into(),
        room: None,
        signal: None,
        host: None,
        join: None,
        stun: Vec::new(),
        turn: None,
        turn_user: String::new(),
        turn_pass: String::new(),
        present_delay: nettai_frontend::netplay::NetOptions::default().present_delay,
        wait: 0,
        record: None,
        replay: None,
        side: None,
    };
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().ok_or_else(|| format!("{name} needs a value"));
        let number = |v: String, name: &str| v.parse::<u64>().map_err(|_| format!("bad {name} {v:?}"));
        match arg.as_str() {
            "--pack" => a.packs.push(value("--pack")?.into()),
            "--content" => a.content = Some(value("--content")?.into()),
            "--mute" => a.mute = true,
            "--round" => a.round = number(value("--round")?, "--round")? as usize,
            "--match" => a.match_file = Some(value("--match")?.into()),
            "--edit" => a.edit = Some(value("--edit")?.into()),
            "--tab" => a.tab = Some(value("--tab")?),
            "--screenshot" => a.screenshot = Some(value("--screenshot")?.into()),
            "--save-match" => a.save_match = Some(value("--save-match")?.into()),
            "--show-folders" => a.show_folders = true,
            "--keys" => a.keys = Some(value("--keys")?),
            "--scale" => a.scale = number(value("--scale")?, "--scale")? as usize,
            "--paused" => a.paused = true,
            "--headless" => a.headless = Some(value("--headless")?),
            "--audit" => a.audit = true,
            "--draw" => a.draw = true,
            "--jobs" => a.jobs = number(value("--jobs")?, "--jobs")? as usize,
            "--lookups" => a.lookups = Some(value("--lookups")?.into()),
            "--audit-content" => a.audit_content = true,
            "--objects" => a.objects = true,
            "--mark" => a.marks.extend(value("--mark")?.split(',').filter(|m| !m.is_empty()).map(str::to_string)),
            "--out" => a.out = value("--out")?.into(),
            "--png-scale" => a.png_scale = number(value("--png-scale")?, "--png-scale")? as usize,
            "--quit-after" => a.quit_after = Some(number(value("--quit-after")?, "--quit-after")?),
            "--text" => a.text = value("--text")?.parse()?,
            "--font" => a.font = Some(value("--font")?.into()),
            "--lang" => a.lang = value("--lang")?,
            "--host" => a.host = Some(number(value("--host")?, "--host")?.try_into().map_err(|_| "bad --host port".to_string())?),
            "--join" => a.join = Some(value("--join")?),
            "--room" => a.room = Some(value("--room")?),
            "--signal" => a.signal = Some(value("--signal")?),
            "--stun" => a.stun.push(value("--stun")?),
            "--turn" => a.turn = Some(value("--turn")?),
            "--turn-user" => a.turn_user = value("--turn-user")?,
            "--turn-pass" => a.turn_pass = value("--turn-pass")?,
            "--present-delay" => a.present_delay = number(value("--present-delay")?, "--present-delay")? as u32,
            "--wait" => a.wait = number(value("--wait")?, "--wait")?,
            "--record" => a.record = Some(value("--record")?.into()),
            "--replay" => a.replay = Some(value("--replay")?.into()),
            "--side" => {
                a.side = Some(match value("--side")?.as_str() {
                    "left" => 0,
                    "right" => 1,
                    s => return Err(format!("bad --side {s:?} (left or right)")),
                })
            }
            "-h" | "--help" => return Err(String::new()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}")),
            s => a.traces.push(s.into()),
        }
    }
    if a.match_file.is_some() && !a.traces.is_empty() {
        return Err("--match takes a match file, not a trace".into());
    }
    if a.side.is_some() && a.replay.is_none() {
        return Err("--side goes with --replay".into());
    }
    if a.record.is_some() && (a.match_file.is_none() || a.audit_content || a.edit.is_some()) {
        return Err("--record writes the set a match file plays (--match, alone or with --room, --host or --join)".into());
    }
    if a.replay.is_some() {
        if a.match_file.is_some() || !a.traces.is_empty() || a.edit.is_some() || a.audit || a.audit_content {
            return Err("--replay plays a replay alone (not with a match file, a trace, the editor or an audit)".into());
        }
        if a.netplay() || a.save_match.is_some() || a.keys.is_some() {
            return Err("--replay plays what was recorded (no --room, --host, --join, --save-match or --keys)".into());
        }
    }
    if a.audit_content {
        if !a.traces.is_empty() || a.audit || a.headless.is_some() || a.netplay() || a.save_match.is_some() {
            return Err("--audit-content audits the content alone (--audit runs traces)".into());
        }
        if a.match_file.is_none() {
            return Err("--audit-content needs --match FILE to select the game".into());
        }
        return Ok(a);
    }
    // The editor: a match file to edit, or nothing else to do.
    let editing = a.edit.is_some() || (a.traces.is_empty() && a.match_file.is_none() && a.replay.is_none() && !a.audit);
    if editing {
        if !a.traces.is_empty() || a.match_file.is_some() || a.audit || a.headless.is_some() || a.netplay() {
            return Err("--edit opens the editor alone (play its match from there, or with --match)".into());
        }
        if a.save_match.is_some() || a.quit_after.is_some() {
            return Err("--save-match and --quit-after go with a match played (--match)".into());
        }
        return Ok(a);
    }
    if a.tab.is_some() || a.screenshot.is_some() {
        return Err("--tab and --screenshot go with the editor".into());
    }
    if a.traces.is_empty() && a.match_file.is_none() && a.replay.is_none() {
        return Err("give a trace file, --match FILE or --replay FILE".into());
    }
    if a.traces.len() > 1 && !a.audit {
        return Err("one trace at a time (several with --audit)".into());
    }
    if (a.draw || a.lookups.is_some() || a.jobs != 0) && !a.audit && !a.audit_content {
        return Err("--draw, --jobs and --lookups go with --audit".into());
    }
    if a.audit && (a.match_file.is_some() || a.headless.is_some()) {
        return Err("--audit runs traces, without a window".into());
    }
    if a.save_match.is_some() && a.match_file.is_none() {
        return Err("--save-match writes the match played live".into());
    }
    let servers = !a.stun.is_empty() || a.turn.is_some() || !a.turn_user.is_empty() || !a.turn_pass.is_empty();
    if (a.signal.is_some() || servers) && a.room.is_none() {
        return Err("--signal, --stun and --turn go with --room (a direct connection uses none)".into());
    }
    if a.turn.is_none() && (!a.turn_user.is_empty() || !a.turn_pass.is_empty()) {
        return Err("--turn-user and --turn-pass go with --turn".into());
    }
    if a.netplay() {
        let ways = [a.room.is_some(), a.host.is_some(), a.join.is_some()].iter().filter(|w| **w).count();
        if a.match_file.is_none() || ways != 1 {
            return Err("netplay is --match FILE with one of --room CODE, --host PORT or --join ADDR:PORT".into());
        }
        if a.room.is_some() && a.signal.is_none() && std::env::var_os("NETTAI_SIGNAL").is_none() {
            return Err("--room needs the signaling server: --signal URL, or $NETTAI_SIGNAL".into());
        }
        if a.headless.is_some() || a.audit {
            return Err("netplay plays in a window".into());
        }
        if a.present_delay > MAX_PRESENT_DELAY {
            return Err(format!("--present-delay {} is more than {MAX_PRESENT_DELAY} frames", a.present_delay));
        }
    }
    Ok(a)
}

impl Args {
    /// Whether it plays another player over the network.
    fn netplay(&self) -> bool {
        self.room.is_some() || self.host.is_some() || self.join.is_some()
    }

    /// The netplay connection's settings: the STUN and TURN servers.
    fn rtc_config(&self) -> nettai_rtc::Config {
        let mut config = nettai_rtc::Config::default();
        if !self.stun.is_empty() {
            config.ice_servers = self.stun.iter().filter(|s| *s != "none").map(|s| nettai_rtc::IceServer::new(s)).collect();
        }
        if let Some(turn) = &self.turn {
            config.ice_servers.push(nettai_rtc::IceServer { urls: vec![turn.clone()], username: self.turn_user.clone(), credential: self.turn_pass.clone() });
        }
        config
    }
}

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

/// Show what loading a pack found (warnings and errors).
fn show(report: &nettai_content::report::Report) {
    for i in report.issues.iter().filter(|i| i.level != nettai_content::report::Level::Note) {
        eprintln!("{i}");
    }
}

/// Stop with a step of loading that failed: what its loaders reported, then
/// what failed, with the options that say where it is looked for.
fn load_failed(e: LoadError) -> ! {
    show(&e.report);
    fail(match &e.what {
        Failed::Packs => format!("{e} (--pack)"),
        Failed::Content { .. } => format!("{e} (--content, --pack)"),
        _ => e.to_string(),
    })
}

/// How long a part took to load, for whoever asks (`NETTAI_LOAD_TIMES`).
fn load_time(what: &str, since: Instant) {
    if std::env::var_os("NETTAI_LOAD_TIMES").is_some() {
        eprintln!("loaded the {what} in {:.1?}", since.elapsed());
    }
}

/// Load one part of the pack, or stop with what is wrong with it (the
/// content audit's own loading: every language's, none chosen).
fn load<T>(pack: &Path, what: &str, f: impl Fn(&Path) -> Result<(T, nettai_content::report::Report), nettai_content::report::Report>) -> T {
    let t = Instant::now();
    match f(pack) {
        Ok((v, r)) => {
            show(&r);
            load_time(what, t);
            v
        }
        Err(r) => {
            show(&r);
            fail(format!("can't load the {what} of the content pack {} (extract it again: README.md, \"Getting started\")", pack.display()))
        }
    }
}

/// The game's sound, or stop with what is wrong with it.
fn sound_of(game: &Game) -> Sound {
    let t = Instant::now();
    let sound = game.sound().unwrap_or_else(|e| load_failed(e));
    show(&sound.report);
    load_time("sound", t);
    sound
}

/// A match file's text.
fn match_text(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| fail(format!("can't read {}: {e}", path.display())))
}

/// A match file (its text), checked against the content.
fn read_match(content: &Arc<nettai_battle::Content>, path: &Path, text: &str) -> nettai_match::Match {
    nettai_match::parse(content, text).unwrap_or_else(|problems| {
        fail(format!("{} can't be played:\n  {}", path.display(), problems.join("\n  ")))
    })
}

/// Write the match played, with the seed it was played from.
fn save_match(content: &nettai_battle::Content, m: &nettai_match::Match, seed: u32, path: &Path) {
    let m = nettai_match::Match { seed: Some(seed), ..m.clone() };
    std::fs::write(path, nettai_match::write(content, &m)).unwrap_or_else(|e| fail(format!("can't write {}: {e}", path.display())));
    eprintln!("wrote the match to {}", path.display());
}

/// Connect (in a room, or directly: host or join) and start the handshake
/// that agrees the match: each player proposes their match file's settings
/// (its game and its rounds: both files must state the same, or it stops
/// saying what differs) and brings its left side. The window polls it each
/// frame, saying the second half.
fn handshake(args: &Args, content: &Arc<nettai_battle::Content>, m: nettai_match::Match) -> (NetHandshake<Link>, String) {
    use nettai_frontend::lobby::Settings;
    let settings = Settings::of_match(&m);
    let [side, _] = m.sides;
    let wait = |default: u64| std::time::Duration::from_secs(if args.wait > 0 { args.wait } else { default });
    let waits = Waits { host: wait(300), join: wait(30) };
    let config = args.rtc_config();
    let (link, text) = if let Some(code) = &args.room {
        let signal = args.signal.clone().or_else(|| std::env::var("NETTAI_SIGNAL").ok()).expect("checked");
        let link = Link::room(&signal, code, config).unwrap_or_else(|e| fail(format!("can't meet in room {code}: {e}")));
        eprintln!("netplay: in room {code} of {signal}, waiting for the other player (they run --match FILE --room {code})");
        (link, format!("waiting in room {code}"))
    } else if let Some(port) = args.host {
        let link = Link::host(port, config).unwrap_or_else(|e| fail(format!("can't host on UDP port {port}: {e}")));
        eprintln!(
            "netplay: hosting on UDP port {port}, waiting for a player (they run --match FILE --join <this machine's address>:{port}; \
             over the Internet, forward the port to this machine)"
        );
        (link, format!("waiting for a player on UDP port {port}"))
    } else {
        let addr = args.join.as_deref().unwrap();
        let link = Link::join(addr, config).unwrap_or_else(|e| fail(format!("can't reach {addr}: {e}")));
        eprintln!("netplay: joining {addr}");
        (link, format!("joining {addr}, waiting for the host"))
    };
    (NetHandshake::new(link, content, settings, side, waits), text)
}

/// What netplay needs once the match is agreed (from the command line).
struct NetArgs {
    present_delay: u32,
    show_folders: bool,
    save_match: Option<PathBuf>,
    record: Option<PathBuf>,
}

/// A recording of match `m` (its seed stated) played from `side` to the
/// file `path`, or stop with why not.
fn recorder(content: &Arc<nettai_battle::Content>, m: &nettai_match::Match, side: u8, path: &Path) -> nettai_frontend::replay::Recorder {
    use nettai_frontend::replay::{Info, Recorder};
    let file = std::fs::File::create(path).unwrap_or_else(|e| fail(format!("can't write {}: {e}", path.display())));
    let when = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let info = Info { when, side, names: Default::default() };
    let r = Recorder::new(Box::new(std::io::BufWriter::new(file)), content, m, &info).unwrap_or_else(|e| fail(format!("can't record to {}: {e}", path.display())));
    eprintln!("recording the set to {}", path.display());
    r
}

/// The agreed match's player, and its recording if one is asked for.
fn net_player(net: &NetArgs, content: &Arc<nettai_battle::Content>, agreed: Agreed<Link>) -> (Box<dyn Driver>, Option<nettai_frontend::replay::Recorder>) {
    use nettai_frontend::netplay::{NetOptions, NetPlayer};
    let Agreed { conn, agreement } = agreed;
    let nettai_frontend::lobby::Agreement { set, m, seed, side, .. } = agreement;
    let peer = conn.datagram().describe();
    eprintln!(
        "netplay: playing {peer}; you are the {} navi (the match's seed {seed}, present delay {})",
        if side == 0 { "left" } else { "right" },
        net.present_delay
    );
    eprintln!("{}", nettai_match::describe(content, &m, seed, net.show_folders, side));
    if let Some(path) = &net.save_match {
        save_match(content, &m, seed, path);
    }
    let recording = net.record.as_deref().map(|path| recorder(content, &nettai_match::Match { seed: Some(seed), ..m.clone() }, side as u8, path));
    let options = NetOptions { present_delay: net.present_delay, ..NetOptions::default() };
    (Box::new(NetPlayer::new(conn, side, set, options)), recording)
}

/// The editor's options, from the command line.
fn editor_options(args: &Args) -> editor::Options {
    let lang = editor::names::Lang::from_code(&args.lang).unwrap_or_else(|| fail(format!("no language {:?} for the editor (en or ja)", args.lang)));
    editor::Options {
        content: args.content.clone(),
        content_dir: PathBuf::new(),
        games: Vec::new(),
        packs: args.packs.clone(),
        file: args.edit.clone(),
        lang,
        tab: args.tab.clone(),
        screenshot: args.screenshot.clone(),
    }
}

/// The editor's window: on `--edit FILE`, else on the choice of a new
/// match's game (none is chosen for it).
fn editor_app(args: &Args) -> editor::App {
    let options = editor_options(args);
    // The match's game, when there is a file: the one it names.
    let opened = args.edit.as_ref().map(|path| {
        let game = std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|t| nettai_match::file::game_of(&t))
            .unwrap_or_else(|e| fail(format!("{}: {e}", path.display())));
        let loaded = editor::load::load_game(options.content.as_deref(), &options.packs, &game).unwrap_or_else(|e| fail(e));
        let mut of_match = options.clone();
        (of_match.content_dir, of_match.games) = (loaded.game.dir.clone(), vec![loaded.game.name.clone()]);
        editor::Editor::new(loaded.game, loaded.pictures, of_match)
    });
    editor::App::new(options, opened)
}

/// `--audit-content`: every lookup for everything the content defines, in
/// each language the content has strings in; then exit.
fn audit_content(args: &Args, game: &Game) -> ! {
    let t = Instant::now();
    let (content, dir, games) = (&*game.content, &*game.dir, std::slice::from_ref(&game.name));
    let bundles = game.packs.iter().map(|p| load(p, "graphics", nettai_content::pack::load_graphics)).collect();
    let banks: Option<Vec<Arc<m4a::SoundBank>>> = (!args.mute).then(|| sound_of(game).banks);
    let own_lang = nettai_content::locale::OWN;
    let mut languages: Vec<nettai_demo::content_audit::Language> = vec![(own_lang.to_string(), None)];
    for lang in nettai_content::locale::languages(dir).into_iter().filter(|l| l != own_lang) {
        let strings = nettai_content::locale::load_for(dir, games, &lang).unwrap_or_else(|e| fail(e));
        languages.push((lang, strings.map(Arc::new)));
    }
    let found = nettai_demo::content_audit::audit(content, bundles, game.own, banks.as_deref(), &languages);
    for p in &found.problems {
        println!("{p}");
    }
    if !found.untranslated.is_empty() {
        eprintln!("audit-content: shown in the content's own (a table lacks them): {}", found.untranslated.join(", "));
    }
    for note in &found.notes {
        eprintln!("audit-content: note: {note}");
    }
    // (As --audit lists a trace's, under the name "content".)
    if let Some(path) = &args.lookups {
        let text: String = found.made.iter().map(|l| format!("content\t{l}\n")).collect();
        std::fs::write(path, text).unwrap_or_else(|e| fail(format!("can't write {}: {e}", path.display())));
    }
    let assets: Vec<String> = found.assets.iter().map(|(k, n)| format!("{n} {k}s")).collect();
    eprintln!(
        "audit-content: {} chips, {} navis, {} forms; the packs' {}; {} lookups in {}{}; {} problems ({:.1?})",
        found.chips,
        found.navis,
        found.forms,
        assets.join(", "),
        found.made.len(),
        found.languages.join(" and "),
        if banks.is_none() { ", no sound (--mute)" } else { "" },
        found.problems.len(),
        t.elapsed()
    );
    std::process::exit(if found.problems.is_empty() { 0 } else { 1 })
}

/// `--audit TRACE...`: the traces' lookups, several traces at a time; then
/// exit. One trace prints as `--audit` always has: its problems, and a
/// summary line.
fn audit_traces(args: &Args, content: &Arc<nettai_battle::Content>, setup: &headless::AuditSetup) -> ! {
    use std::sync::Mutex;
    let t = Instant::now();
    let jobs = if args.jobs > 0 { args.jobs } else { std::thread::available_parallelism().map_or(4, |n| n.get()) };
    let one = args.traces.len() == 1;
    // (Each trace's lines together.)
    let out = Mutex::new(());
    let done = |a: &headless::TraceAudit| {
        let _held = out.lock().unwrap();
        let name = a.trace.display();
        match &a.audit {
            Ok(found) => {
                for s in &found.stopped {
                    eprintln!("{}{s}", if one { String::new() } else { format!("{name}: ") });
                }
                for line in found.problems.lines() {
                    if one {
                        println!("{line}");
                    } else {
                        println!("{name}: {line}");
                    }
                }
                for line in found.problems.said_lines() {
                    eprintln!("{}note: {line}", if one { String::new() } else { format!("{name}: ") });
                }
                if !one {
                    eprintln!("audit {name}: {} frames, {} sound cues, {} problems", found.frames, found.cues, found.problems.len());
                }
            }
            Err(e) => println!("{name}: {e}"),
        }
    };
    let results = headless::audit_traces(content, setup, &args.traces, jobs, &done);
    if let Some(path) = &args.lookups {
        let mut text = String::new();
        for r in &results {
            let Ok(found) = &r.audit else { continue };
            let name = r.trace.display();
            text.push_str(&format!("{name}\t#frames {}\n", found.frames));
            let mut lines: Vec<String> = found.problems.lookups().map(|l| l.describe(content)).collect();
            lines.sort();
            lines.dedup();
            for l in lines {
                text.push_str(&format!("{name}\t{l}\n"));
            }
        }
        std::fs::write(path, text).unwrap_or_else(|e| fail(format!("can't write {}: {e}", path.display())));
    }
    let (frames, cues) = results.iter().filter_map(|r| r.audit.as_ref().ok()).fold((0u64, 0u64), |(f, c), a| (f + a.frames as u64, c + a.cues as u64));
    let problems: usize = results.iter().map(|r| r.audit.as_ref().map_or(1, |a| a.problems.len())).sum();
    if one {
        eprintln!("audit: {frames} frames, {cues} sound cues, {problems} problems");
    } else {
        let bad = results.iter().filter(|r| r.audit.as_ref().map_or(true, |a| !a.problems.is_empty())).count();
        eprintln!(
            "audit: {} traces, {frames} frames, {cues} sound cues, {problems} problems ({bad} traces with problems; {jobs} at a time, {:.1?})",
            results.len(),
            t.elapsed()
        );
    }
    std::process::exit(if problems == 0 { 0 } else { 1 })
}

fn main() {
    let args = match parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("{e}\n");
            }
            eprintln!("{USAGE}\n\n{}", window::HELP);
            std::process::exit(2);
        }
    };
    let play = PlayOptions {
        scale: args.scale,
        start_paused: args.paused,
        quit_after: args.quit_after,
        text: args.text,
        font: args.font.clone(),
        mute: args.mute,
    };
    // The editor: its window, which plays its match in place.
    if args.traces.is_empty() && args.match_file.is_none() && args.replay.is_none() && !args.audit {
        // (A round that doesn't start is a problem the editor shows, not a
        // message on the terminal.)
        std::panic::set_hook(Box::new(|_| {}));
        if let Err(e) = window::run(editor_app(&args), Start::Edit, play) {
            fail(format!("window: {e}"));
        }
        return;
    }
    // The packs found in the packs directory (and given by --pack).
    let t = Instant::now();
    let found = Found::find(&nettai_content::pack::packs_dir(), &args.packs).unwrap_or_else(|e| load_failed(e));
    show(&found.report);
    // The game played is the match file's, the replay's or the trace's own.
    let file_text = args.match_file.as_deref().map(match_text);
    let replay = args.replay.as_deref().map(|path| {
        let bytes = std::fs::read(path).unwrap_or_else(|e| fail(format!("can't read {}: {e}", path.display())));
        nettai_frontend::replay::Replay::read(&bytes).unwrap_or_else(|e| fail(format!("can't play {}: {e}", path.display())))
    });
    let game = match (&file_text, &replay, args.traces.first()) {
        (Some(text), _, _) => nettai_match::file::game_of(text)
            .unwrap_or_else(|e| fail(format!("{} can't be played: {e}", args.match_file.as_ref().unwrap().display()))),
        (None, Some(r), _) => r.head.game.clone(),
        (None, None, Some(trace)) => nettai_demo::trace::trace_game(trace).unwrap_or_else(|e| fail(format!("can't play {}: {e}", trace.display()))),
        (None, None, None) => unreachable!("argument parsing requires a match file, a replay or a trace"),
    };
    // The game's content.
    let loaded = Game::load(&found, args.content.as_deref(), &game).unwrap_or_else(|e| load_failed(e));
    show(&loaded.report);
    if std::env::var_os("NETTAI_LOAD_TIMES").is_some() {
        eprintln!("loaded the battle content in {:.1?} (pack {})", t.elapsed(), loaded.packs[loaded.own.index()].display());
    }
    let content = loaded.content.clone();
    // (The session reports an engine panic; the program keeps it off stderr.)
    session::quiet_engine_panics();
    if args.audit_content {
        audit_content(&args, &loaded);
    }
    // The game's pack's graphics, in the player's language.
    let t = Instant::now();
    let graphics = loaded.graphics(&args.lang).unwrap_or_else(|e| load_failed(e));
    show(&graphics.report);
    load_time("graphics", t);
    // The font mode's font, shared by the renderer (which strings it has)
    // and the text layer's drawing.
    let font = nettai_frontend::game::font(args.text, args.font.as_deref()).unwrap_or_else(|e| load_failed(e));
    let mut renderer = graphics.renderer(args.text, font.clone());
    renderer.problems.marking = args.marks.iter().cloned().collect();
    if args.audit {
        let setup = headless::AuditSetup {
            packs: renderer.graphics().clone(),
            strings: graphics.strings.clone(),
            text: args.text,
            font,
            sound: (!args.mute).then(|| sound_of(&loaded).banks),
            draw: args.draw,
        };
        audit_traces(&args, &content, &setup);
    }
    let text = font.map(TextRenderer::new);

    // What is played: live play's set, one driver; a netplay match, once
    // the window has agreed it; a recording's rounds, a driver each, in turn.
    let mut drivers: Vec<Box<dyn Driver>> = Vec::new();
    let mut netplay = None;
    let mut recording = None;
    if let (Some(path), Some(r)) = (&args.replay, &replay) {
        let player = nettai_frontend::replay::ReplayPlayer::new(&content, r, args.side).unwrap_or_else(|e| fail(format!("can't play {}: {e}", path.display())));
        let m = nettai_match::binary::read_match(&content, &r.match_bytes).expect("a replay that plays has a match");
        eprintln!("{}", nettai_match::describe(&content, &m, m.seed.unwrap_or(0), args.show_folders, args.side.unwrap_or(r.info.side) as usize));
        let rounds = r.rounds().len();
        eprintln!(
            "replay: {} ticks, {rounds} round{}, {}",
            r.ticks.len(),
            if rounds == 1 { "" } else { "s" },
            match r.end {
                nettai_frontend::replay::End::Set => "to the set's end",
                nettai_frontend::replay::End::Open => "stopped before the set's end",
                nettai_frontend::replay::End::Cut => "cut short",
            }
        );
        drivers.push(Box::new(player));
    } else if let Some((path, text)) = args.match_file.as_deref().zip(file_text.as_deref()) {
        let m = read_match(&content, path, text);
        if args.netplay() {
            netplay = Some(handshake(&args, &content, m));
        } else {
            let seed = m.seed.unwrap_or_else(|| {
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1)
            });
            eprintln!("{}", nettai_match::describe(&content, &m, seed, args.show_folders, 0));
            if let Some(path) = &args.save_match {
                save_match(&content, &m, seed, path);
            }
            if let Some(path) = &args.record {
                recording = Some(recorder(&content, &nettai_match::Match { seed: Some(seed), ..m.clone() }, 0, path));
            }
            drivers.push(Box::new(LivePlayer::new(nettai_match::Set::of(&content, &m, seed))));
        }
    } else if let Some(path) = args.traces.first() {
        let rounds = trace_rounds(path, &content).unwrap_or_else(|e| fail(format!("can't play {}: {e}", path.display())));
        for (number, r) in rounds.into_iter().skip(args.round.saturating_sub(1)) {
            if let Some((a, b)) = r.frame_range() {
                eprintln!("round {number}: frames {a}..={b}");
            }
            drivers.push(r);
        }
    }
    if drivers.is_empty() && netplay.is_none() {
        fail("nothing to play");
    }

    // (Netplay plays in a window.)
    if let Some(list) = &args.headless {
        let first = drivers.remove(0);
        let wanted = headless::parse_frames(list).unwrap_or_else(|e| fail(e));
        let keys = headless::KeyScript::parse(args.keys.as_deref().unwrap_or("")).unwrap_or_else(|e| fail(e));
        let mut log = |s: &str| eprintln!("{s}");
        // (Headless rendering plays no sound.)
        let mut player = Player::with(renderer, text, None, first);
        if let Some(r) = recording {
            player.record(r).unwrap_or_else(|e| fail(e));
        }
        let rendered = headless::render_frames_with(&mut player, drivers, &wanted, &args.out, args.png_scale, args.objects, &keys, &mut log);
        match rendered {
            Ok(written) => {
                eprintln!("wrote {} frames to {}", written.len(), args.out.display());
                if written.len() < wanted.len() {
                    std::process::exit(1);
                }
            }
            Err(e) => fail(e),
        }
        return;
    }

    // The window: the player's sound as samples, played through the audio
    // device (the window opens it).
    let audio = if args.mute {
        None
    } else {
        let sound = sound_of(&loaded);
        Some(nettai_audio::BattleAudio::with_banks(sound.banks, sound.songs))
    };
    eprintln!("{}", window::HELP);
    let start = match netplay {
        // A netplay match: the window stays responsive (Esc quits) while
        // the handshake agrees it.
        Some((handshake, text_line)) => {
            let net = NetArgs {
                present_delay: args.present_delay,
                show_folders: args.show_folders,
                save_match: args.save_match.clone(),
                record: args.record.clone(),
            };
            let content = content.clone();
            Start::Net(Box::new(Waiting {
                handshake,
                text: text_line,
                then: Box::new(move |agreed| net_player(&net, &content, agreed)),
                parts: (renderer, text, audio),
            }))
        }
        None => {
            let first = drivers.remove(0);
            let mut player = Player::with(renderer, text, audio, first);
            if let Some(r) = recording {
                player.record(r).unwrap_or_else(|e| fail(e));
            }
            Start::Play(Box::new(Play::new(player, drivers, &play)))
        }
    };
    if let Err(e) = window::run(editor::App::new(editor_options(&args), None), start, play) {
        fail(format!("window: {e}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Result<Args, String> {
        parse(values.iter().map(|s| s.to_string()))
    }

    #[test]
    fn match_settings_are_not_cli_options() {
        for option in ["--game", "--seed", "--stage", "--cards", "--their-cards", "--play"] {
            assert_eq!(args(&["--match", "match.toml", option]).err(), Some(format!("unknown option {option}")));
        }
    }

    #[test]
    fn a_match_supports_live_headless_and_netplay() {
        let live = args(&["--match", "match.toml", "--save-match", "played.toml", "--show-folders"]).unwrap();
        assert_eq!(live.match_file.as_deref(), Some(Path::new("match.toml")));
        assert_eq!(live.save_match.as_deref(), Some(Path::new("played.toml")));
        assert!(live.show_folders);
        let headless = args(&["--match", "match.toml", "--headless", "1-120", "--keys", "100:a"]).unwrap();
        assert_eq!(headless.headless.as_deref(), Some("1-120"));
        assert_eq!(headless.keys.as_deref(), Some("100:a"));
        assert_eq!(args(&["--match", "match.toml", "--host", "7777"]).unwrap().host, Some(7777));
        assert_eq!(args(&["--match", "match.toml", "--join", "127.0.0.1:7777"]).unwrap().join.as_deref(), Some("127.0.0.1:7777"));
        for options in [
            vec!["--host", "7777"],
            vec!["trace.jsonl", "--join", "127.0.0.1:7777"],
            vec!["--match", "match.toml", "--host", "7777", "--join", "127.0.0.1:7777"],
            vec!["--match", "match.toml", "--host", "7777", "--headless", "1"],
            vec!["--match", "match.toml", "--host", "7777", "--present-delay", "16"],
        ] {
            assert!(args(&options).is_err(), "{options:?}");
        }
    }

    /// Netplay in a room: the signaling server, the STUN and TURN servers
    /// (a room's alone: a direct connection has none).
    #[test]
    fn netplay_meets_in_a_room() {
        let room = args(&["--match", "m.toml", "--room", "abc", "--signal", "ws://127.0.0.1:8787", "--stun", "stun:192.0.2.1", "--stun", "stun:192.0.2.2:3479"]).unwrap();
        assert_eq!((room.room.as_deref(), room.signal.as_deref()), (Some("abc"), Some("ws://127.0.0.1:8787")));
        let urls = |c: nettai_rtc::Config| c.ice_servers.into_iter().flat_map(|s| s.urls).collect::<Vec<_>>();
        assert_eq!(urls(room.rtc_config()), ["stun:192.0.2.1", "stun:192.0.2.2:3479"]);
        // (The public STUN server by default; none on request; TURN with
        // its credentials.)
        let plain = args(&["--match", "m.toml", "--room", "abc", "--signal", "wss://signal.example"]).unwrap();
        assert_eq!(urls(plain.rtc_config()), [nettai_rtc::DEFAULT_STUN]);
        let relayed = args(&["--match", "m.toml", "--room", "abc", "--signal", "wss://s", "--stun", "none", "--turn", "turn:192.0.2.3", "--turn-user", "u", "--turn-pass", "p"]).unwrap();
        let config = relayed.rtc_config();
        assert_eq!(config.ice_servers.len(), 1);
        assert_eq!((config.ice_servers[0].urls[0].as_str(), config.ice_servers[0].username.as_str(), config.ice_servers[0].credential.as_str()), ("turn:192.0.2.3", "u", "p"));
        for (options, why) in [
            (vec!["--match", "m.toml", "--room", "abc", "--host", "7777"], "netplay is --match FILE with one of --room CODE, --host PORT or --join ADDR:PORT"),
            (vec!["trace.jsonl", "--room", "abc", "--signal", "ws://s"], "netplay is --match FILE with one of --room CODE, --host PORT or --join ADDR:PORT"),
            (vec!["--match", "m.toml", "--host", "7777", "--stun", "stun:192.0.2.1"], "--signal, --stun and --turn go with --room (a direct connection uses none)"),
            (vec!["--match", "m.toml", "--join", "192.0.2.1:7777", "--signal", "ws://s"], "--signal, --stun and --turn go with --room (a direct connection uses none)"),
            (vec!["--match", "m.toml", "--room", "abc", "--signal", "ws://s", "--turn-user", "u"], "--turn-user and --turn-pass go with --turn"),
        ] {
            assert_eq!(args(&options).err().as_deref(), Some(why), "{options:?}");
        }
    }

    /// A match played (alone or over the network) is recorded with
    /// `--record`; a replay is played alone, from either side's console.
    #[test]
    fn replays_are_recorded_and_played() {
        for options in [vec!["--match", "m.toml", "--record", "set.ntrp"], vec!["--match", "m.toml", "--host", "7777", "--record", "set.ntrp"]] {
            assert_eq!(args(&options).unwrap().record.as_deref(), Some(Path::new("set.ntrp")), "{options:?}");
        }
        let replay = args(&["--replay", "set.ntrp", "--side", "right"]).unwrap();
        assert_eq!((replay.replay.as_deref(), replay.side), (Some(Path::new("set.ntrp")), Some(1)));
        assert!(args(&["--replay", "set.ntrp", "--headless", "1-60"]).is_ok());
        for (options, why) in [
            (vec!["--record", "set.ntrp"], "--record writes the set a match file plays (--match, alone or with --room, --host or --join)"),
            (vec!["trace.jsonl", "--record", "set.ntrp"], "--record writes the set a match file plays (--match, alone or with --room, --host or --join)"),
            (vec!["--match", "m.toml", "--side", "left"], "--side goes with --replay"),
            (vec!["--replay", "set.ntrp", "--side", "up"], "bad --side \"up\" (left or right)"),
            (vec!["--replay", "set.ntrp", "--match", "m.toml"], "--replay plays a replay alone (not with a match file, a trace, the editor or an audit)"),
            (vec!["--replay", "set.ntrp", "--host", "7777"], "--replay plays what was recorded (no --room, --host, --join, --save-match or --keys)"),
        ] {
            assert_eq!(args(&options).err().as_deref(), Some(why), "{options:?}");
        }
    }

    #[test]
    fn content_audits_use_a_match_to_select_the_game() {
        assert!(args(&["--match", "match.toml", "--audit-content", "--lookups", "lookups.txt"]).unwrap().audit_content);
        assert_eq!(args(&["--audit-content"]).err().as_deref(), Some("--audit-content needs --match FILE to select the game"));
        for extra in [
            vec!["trace.jsonl"],
            vec!["--audit"],
            vec!["--headless", "1"],
            vec!["--host", "7777"],
            vec!["--join", "127.0.0.1:7777"],
            vec!["--save-match", "played.toml"],
        ] {
            let mut options = vec!["--match", "match.toml", "--audit-content"];
            options.extend(extra);
            assert!(args(&options).is_err(), "{options:?}");
        }
    }

    #[test]
    fn traces_are_played_and_audited_without_a_match() {
        assert_eq!(args(&["trace.jsonl"]).unwrap().traces, vec![PathBuf::from("trace.jsonl")]);
        assert!(args(&["--audit", "one.jsonl", "two.jsonl"]).unwrap().audit);
        assert!(args(&["--match", "match.toml", "trace.jsonl"]).is_err());
        assert!(args(&["--match", "match.toml", "--audit"]).is_err());
        assert!(args(&["trace.jsonl", "--save-match", "match.toml"]).is_err());
    }

    /// With nothing to play the window opens on the editor: a new match, or
    /// a file (`--edit`), on a pane (`--tab`); the editor plays its match
    /// itself, so nothing that plays one goes with it.
    #[test]
    fn the_editor_is_the_window_with_nothing_to_play() {
        let new = args(&[]).unwrap();
        assert!(new.edit.is_none() && new.traces.is_empty() && new.match_file.is_none());
        let opened = args(&["--edit", "match.toml", "--tab", "left-folder", "--screenshot", "shot.png"]).unwrap();
        assert_eq!(opened.edit.as_deref(), Some(Path::new("match.toml")));
        assert_eq!((opened.tab.as_deref(), opened.screenshot.as_deref()), (Some("left-folder"), Some(Path::new("shot.png"))));
        for options in [
            vec!["--edit", "match.toml", "--match", "other.toml"],
            vec!["--edit", "match.toml", "trace.jsonl"],
            vec!["--edit", "match.toml", "--host", "7777"],
            vec!["--edit", "match.toml", "--headless", "1"],
            vec!["--save-match", "played.toml"],
            vec!["--match", "match.toml", "--tab", "arena"],
            vec!["trace.jsonl", "--screenshot", "shot.png"],
        ] {
            assert!(args(&options).is_err(), "{options:?}");
        }
    }
}
