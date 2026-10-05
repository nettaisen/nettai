//! nettai-demo: watch a golden trace replayed through the engine, or play.
//! The desktop program over the nettai-frontend library: this is its
//! command line. See docs/frontend.md.

use nettai_demo::trace::trace_rounds;
use nettai_demo::{app, headless};
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
    /// The game played without a match file (a match file names its own).
    game: Option<String>,
    mute: bool,
    /// The trace (several with --audit).
    traces: Vec<PathBuf>,
    round: usize,
    play: bool,
    seed: Option<u32>,
    stage: Option<String>,
    cards: [Option<String>; 2],
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
    out: PathBuf,
    png_scale: usize,
    quit_after: Option<u64>,
    text: TextMode,
    font: Option<PathBuf>,
    lang: String,
    /// Netplay: host on this UDP port, or join this host.
    host: Option<u16>,
    join: Option<String>,
    delay: u32,
    wait: u64,
}

/// The most input delay netplay takes (a quarter of a second).
const MAX_DELAY: u32 = 15;

const USAGE: &str = "\
usage: nettai-demo [OPTIONS] TRACE.jsonl     watch a trace's rounds
       nettai-demo [OPTIONS] --play          play live (you are the left navi)
       nettai-demo [OPTIONS] --match FILE    play a match file (you are its left side)
       nettai-demo [OPTIONS] --play --host PORT        play another player over the
       nettai-demo [OPTIONS] --play --join ADDR:PORT   network: host, or join the host
       nettai-demo [OPTIONS] TRACE.jsonl --headless FRAMES [--out DIR] [--png-scale N]
       nettai-demo [OPTIONS] --audit-content
       nettai-demo [OPTIONS] --audit TRACE.jsonl...

  You play one game, EXE6 or EXE5: a match file names its game and a trace
  states its own, else --game says it (there is no default game: without
  one the frontend lists those it found a pack of and stops). The battle
  is that game's: its content folder and the
  support folders it uses, drawn and heard from its pack (graphics and
  sound, written from your ROMs by `exe6-extract content <falzar-us>
  <gregar-us> <falzar-jp> <gregar-jp> data/content/exe6`, EXE5's by
  exe5-extract), found in the packs directory, $NETTAI_PACKS, else
  data/content, each pack by the game it says.
  --game GAME      the game played, exe6 or exe5: required without a match
                   file or a trace (a match file's game is its own, and so
                   is a trace's: one of another game than GAME is refused)
  --pack DIR       a pack's directory, in place of the found pack of its game
  --content DIR    the content directory (default: $NETTAI_CONTENT, else
                   this repository's content/)
  --mute           no sound (headless rendering never plays any)
  --round N        the trace round to start with (default 1; later rounds follow)
  --seed N         live play's seed: the field, the folders, the Crosses
                   offered and the battle's RNG are drawn from it (default:
                   from the clock); each start prints it
  --stage NAME     live play on this link battle stage (its name in the
                   game, e.g. netbattle-43) instead of a random one
  --cards NAMES    live play: your patch cards (the Japanese games'
                   Modification Cards), their names comma-separated in the
                   order they apply; -NAME installs one switched off (e.g.
                   canodumb,-shadow)
  --their-cards NAMES  the right navi's patch cards, likewise
  --match FILE     play the match this file sets up (docs/frontend.md §6: its
                   game, the arena, each side's navi, version,
                   folder, Crosses, patch cards and stats, by name in the
                   game; nettai-demo-editor makes them), instead of a random one;
                   you are its left side. With --host or --join the left side
                   is what you bring, and the host's arena is the match's
  --save-match FILE  write the match played (live play's random pick, or the
                   one netplay agreed) to FILE as a match file, to play again
                   or edit
  --show-folders   print both players' live folders
  --scale N        window scale (default 4)
  --paused         start paused
  --headless F     render frames F (e.g. 150,300,600 or 100-120; trace frame
                   numbers, or ticks in live play) to frame_NNNNN.png files
                   in --out (default .), no window
  --objects        with --headless: list every rendered frame's objects (kind,
                   place, sprite, animation, look)
  --keys K         with --headless --play: the buttons you hold, by tick (e.g.
                   232-233:up,300:a+b; a b l r up down left right start
                   select)
  --audit-content  make every lookup the drawing code and the audio make for
                   everything the content defines (every chip's icon,
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
  --quit-after N   close the window after N ticks
  --host PORT      netplay: host a match on this UDP port (forward it on
                   your router to play over the Internet) and wait for a
                   player to join; you are the left navi
  --join ADDR:PORT netplay: join the match hosted there; you are the right
                   navi, seen from your side. Both players need the same
                   engine, game and content (the handshake checks); each
                   brings their own folder, version and Crosses (drawn from
                   their --seed) and patch cards (--cards); the host's
                   --stage picks the stage; the field and the battle's RNG
                   come from both players' seeds
  --delay N        netplay's input delay in frames (default 2): more delay,
                   fewer rollbacks
  --wait SECONDS   how long the host waits for a player, or the joiner for
                   the host (default 300 and 30)";

fn parse() -> Result<Args, String> {
    let mut a = Args {
        packs: Vec::new(),
        content: None,
        game: None,
        mute: false,
        traces: Vec::new(),
        round: 1,
        play: false,
        seed: None,
        stage: None,
        cards: [None, None],
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
        out: PathBuf::from("."),
        png_scale: 1,
        quit_after: None,
        text: TextMode::Font,
        font: None,
        lang: nettai_assets::BASE_LANGUAGE.into(),
        host: None,
        join: None,
        delay: nettai_frontend::netplay::NetOptions::default().delay,
        wait: 0,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().ok_or_else(|| format!("{name} needs a value"));
        let number = |v: String, name: &str| v.parse::<u64>().map_err(|_| format!("bad {name} {v:?}"));
        match arg.as_str() {
            "--pack" => a.packs.push(value("--pack")?.into()),
            "--content" => a.content = Some(value("--content")?.into()),
            "--game" => a.game = Some(value("--game")?),
            "--mute" => a.mute = true,
            "--round" => a.round = number(value("--round")?, "--round")? as usize,
            "--play" => a.play = true,
            "--seed" => a.seed = Some(number(value("--seed")?, "--seed")? as u32),
            "--stage" => a.stage = Some(value("--stage")?),
            "--cards" => a.cards[0] = Some(value("--cards")?),
            "--their-cards" => a.cards[1] = Some(value("--their-cards")?),
            "--match" => a.match_file = Some(value("--match")?.into()),
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
            "--out" => a.out = value("--out")?.into(),
            "--png-scale" => a.png_scale = number(value("--png-scale")?, "--png-scale")? as usize,
            "--quit-after" => a.quit_after = Some(number(value("--quit-after")?, "--quit-after")?),
            "--text" => a.text = value("--text")?.parse()?,
            "--font" => a.font = Some(value("--font")?.into()),
            "--lang" => a.lang = value("--lang")?,
            "--host" => a.host = Some(number(value("--host")?, "--host")?.try_into().map_err(|_| "bad --host port".to_string())?),
            "--join" => a.join = Some(value("--join")?),
            "--delay" => a.delay = number(value("--delay")?, "--delay")? as u32,
            "--wait" => a.wait = number(value("--wait")?, "--wait")?,
            "-h" | "--help" => return Err(String::new()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}")),
            s => a.traces.push(s.into()),
        }
    }
    // A match file is played live.
    a.play |= a.match_file.is_some();
    if a.audit_content {
        if !a.traces.is_empty() || a.play || a.audit || a.headless.is_some() {
            return Err("--audit-content audits the content alone (--audit runs traces)".into());
        }
        return Ok(a);
    }
    if a.traces.is_empty() && !a.play {
        return Err("give a trace file, --play, --match FILE or --audit-content".into());
    }
    if a.traces.len() > 1 && !a.audit {
        return Err("one trace at a time (several with --audit)".into());
    }
    if (a.draw || a.lookups.is_some() || a.jobs != 0) && !a.audit && !a.audit_content {
        return Err("--draw, --jobs and --lookups go with --audit".into());
    }
    if a.audit && (a.play || a.headless.is_some()) {
        return Err("--audit runs traces, without a window".into());
    }
    if a.match_file.is_some() {
        if !a.traces.is_empty() {
            return Err("--match plays a match file, not a trace".into());
        }
        if a.stage.is_some() || a.cards.iter().any(Option::is_some) {
            return Err("the match file names the stage and the patch cards (edit it, or leave out --match)".into());
        }
        if a.game.is_some() {
            return Err("the match file names its game (edit it, or leave out --match)".into());
        }
    }
    if a.save_match.is_some() && !a.play {
        return Err("--save-match writes the match played live".into());
    }
    let netplay = a.host.is_some() || a.join.is_some();
    if netplay {
        if !a.play || a.host.is_some() == a.join.is_some() {
            return Err("netplay is --play with either --host PORT or --join ADDR:PORT".into());
        }
        if a.cards[1].is_some() {
            return Err("in netplay the other player brings their own patch cards (--their-cards is for playing alone)".into());
        }
        if a.join.is_some() && a.stage.is_some() {
            return Err("in netplay the host picks the stage".into());
        }
        if a.headless.is_some() || a.audit {
            return Err("netplay plays in a window".into());
        }
        if a.delay > MAX_DELAY {
            return Err(format!("--delay {} is more than {MAX_DELAY} frames", a.delay));
        }
    }
    Ok(a)
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
        Failed::Content { .. } => format!("{e} (--content, --pack, --game)"),
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

/// A netplay match of `game`: connect (host or join), shake hands, and
/// agree the round; each player brings their side, a match file's left
/// side or one drawn from their own `seed` with their patch cards, and the
/// host its arena or stage.
fn netplay(args: &Args, content: &Arc<nettai_battle::Content>, game: &str, seed: u32, file: Option<nettai_match::Match>) -> Box<dyn Driver> {
    use nettai_frontend::netplay::{NetOptions, NetPlayer, Offer, agree, hello};
    use nettai_match::{Picks, Side, link_stage, patch_cards};
    use nettai_netplay::transport::{Connection, Role, Udp};
    let offer = match file {
        Some(m) => Offer::of_match(m, args.host.is_some()),
        None => {
            let mut side = Side::picked(content, game, &mut Picks::new(seed)).unwrap_or_else(|e| fail(e));
            if let Some(list) = &args.cards[0] {
                side.cards = patch_cards(content, game, list).unwrap_or_else(|e| fail(e));
            }
            let stage = args.stage.as_deref().map(|name| link_stage(content, game, name).unwrap_or_else(|e| fail(e)));
            Offer::of_side(game, side, stage)
        }
    };
    let wait = |default: u64| std::time::Duration::from_secs(if args.wait > 0 { args.wait } else { default });
    let conn = if let Some(port) = args.host {
        let udp = Udp::host(port).unwrap_or_else(|e| fail(format!("can't host on UDP port {port}: {e}")));
        eprintln!(
            "netplay: hosting on UDP port {port}, waiting for a player (they run --play --join <this machine's address>:{port}; \
             over the Internet, forward the port to this machine)"
        );
        Connection::host(udp, hello(Role::Host, content, &offer), wait(300))
    } else {
        let addr = args.join.as_deref().unwrap();
        let udp = Udp::join(addr).unwrap_or_else(|e| fail(format!("can't reach {addr}: {e}")));
        eprintln!("netplay: joining {addr}");
        Connection::join(udp, hello(Role::Join, content, &offer), wait(30))
    }
    .unwrap_or_else(|e| fail(format!("netplay: {e}")));
    let peer = conn.datagram().peer().map_or("the other player".to_string(), |a| a.to_string());
    let (_, set, m) = agree(content, &conn, &offer).unwrap_or_else(|e| fail(format!("netplay: {e}")));
    let side = conn.side();
    eprintln!(
        "netplay: playing {peer}; you are the {} navi (your setup's seed {seed}, the match's {}, input delay {})",
        if side == 0 { "left" } else { "right" },
        conn.seed(),
        args.delay
    );
    eprintln!("{}", nettai_match::describe(content, &m, conn.seed(), args.show_folders, side));
    if let Some(path) = &args.save_match {
        save_match(content, &m, conn.seed(), path);
    }
    let options = NetOptions { delay: args.delay, ..NetOptions::default() };
    Box::new(NetPlayer::new(conn, set, options))
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

/// What the frontend says when nothing says the game (no match file, no
/// trace, no `--game`): the games it found a pack of, each as the option
/// that plays it.
fn which_game(content: Option<&Path>, found: &Found) -> String {
    let packs_dir = &found.dir;
    let games: Vec<String> = match nettai_content::pack::games(content, &found.packs) {
        Ok(games) => games.into_iter().filter(|g| g.pack.is_some()).map(|g| format!("--game {}", g.game)).collect(),
        Err(_) => Vec::new(),
    };
    if games.is_empty() {
        format!("say which game with --game: none has its pack in {} (extract one there, or give it with --pack)", packs_dir.display())
    } else {
        format!("say which game: {}", games.join(" or "))
    }
}

fn main() {
    let args = match parse() {
        Ok(a) => a,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("{e}\n");
            }
            eprintln!("{USAGE}\n\n{}", app::HELP);
            std::process::exit(2);
        }
    };
    // The packs found in the packs directory (and given by --pack).
    let t = Instant::now();
    let found = Found::find(&nettai_content::pack::packs_dir(), &args.packs).unwrap_or_else(|e| load_failed(e));
    show(&found.report);
    // The game played: the match file's; a recording's own (the one its
    // setup states, which --game, if given, must be); else --game's. There
    // is no default game: with none said, the program says which it could
    // play and stops.
    let file_text = args.match_file.as_deref().map(match_text);
    let game = match (&file_text, args.traces.first()) {
        (Some(text), _) => nettai_match::file::game_of(text)
            .unwrap_or_else(|e| fail(format!("{} can't be played: {e}", args.match_file.as_ref().unwrap().display()))),
        (None, Some(trace)) => {
            let stated = nettai_demo::trace::trace_game(trace).unwrap_or_else(|e| fail(format!("can't play {}: {e}", trace.display())));
            if let Some(given) = args.game.as_ref().filter(|g| **g != stated) {
                fail(format!("{} is a recording of {stated}, not of {given} (--game)", trace.display()));
            }
            stated
        }
        (None, None) => args.game.clone().unwrap_or_else(|| fail(which_game(args.content.as_deref(), &found))),
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
    let renderer = graphics.renderer(args.text, font.clone());
    if args.audit {
        let setup = headless::AuditSetup {
            packs: renderer.packs.clone(),
            strings: graphics.strings.clone(),
            text: args.text,
            font,
            sound: (!args.mute).then(|| sound_of(&loaded).banks),
            draw: args.draw,
        };
        audit_traces(&args, &content, &setup);
    }
    let text = font.map(TextRenderer::new);

    // What is played: live play's set or a netplay match, one driver; a
    // recording's rounds, a driver each, in turn.
    let mut drivers: Vec<Box<dyn Driver>> = Vec::new();
    if args.play {
        let file = args.match_file.as_deref().zip(file_text.as_deref()).map(|(path, text)| read_match(&content, path, text));
        let seed = args.seed.or(file.as_ref().and_then(|m| m.seed)).unwrap_or_else(|| {
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1)
        });
        if args.host.is_some() || args.join.is_some() {
            drivers.push(netplay(&args, &content, &game, seed, file));
        } else {
            let m = match file {
                Some(m) => m,
                None => {
                    let stage = args.stage.as_deref().map(|name| nettai_match::link_stage(&content, &game, name).unwrap_or_else(|e| fail(e)));
                    let mut m = nettai_match::pick::live(&content, &game, seed, stage).unwrap_or_else(|e| fail(e));
                    for (side, list) in args.cards.iter().enumerate() {
                        if let Some(list) = list {
                            m.sides[side].cards = nettai_match::patch_cards(&content, &game, list).unwrap_or_else(|e| fail(e));
                        }
                    }
                    let problems = nettai_match::check_match(&content, &m);
                    if !problems.is_empty() {
                        fail(format!("the match can't be played:\n  {}", problems.join("\n  ")));
                    }
                    m
                }
            };
            eprintln!("{}", nettai_match::describe(&content, &m, seed, args.show_folders, 0));
            if let Some(path) = &args.save_match {
                save_match(&content, &m, seed, path);
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
    if drivers.is_empty() {
        fail("nothing to play");
    }
    let first = drivers.remove(0);

    if let Some(list) = &args.headless {
        let wanted = headless::parse_frames(list).unwrap_or_else(|e| fail(e));
        let keys = headless::KeyScript::parse(args.keys.as_deref().unwrap_or("")).unwrap_or_else(|e| fail(e));
        let mut log = |s: &str| eprintln!("{s}");
        // (Headless rendering plays no sound.)
        let mut player = Player::with(renderer, text, None, first);
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
    // device.
    let (audio, device) = if args.mute {
        (None, None)
    } else {
        let sound = sound_of(&loaded);
        let device = nettai_audio::Output::open().unwrap_or_else(|e| fail(format!("no audio output: {e}")));
        (Some(nettai_audio::BattleAudio::with_banks(sound.banks, sound.songs)), Some(device))
    };
    eprintln!("{}", app::HELP);
    let mut player = Player::with(renderer, text, audio, first);
    let opts = app::Options { scale: args.scale, start_paused: args.paused, quit_after: args.quit_after };
    if let Err(e) = app::run(&mut player, drivers, device.as_ref(), &opts) {
        fail(format!("window: {e}"));
    }
}
