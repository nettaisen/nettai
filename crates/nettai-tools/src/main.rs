//! nettai-tool: nettai's command line without a window, for the developer
//! and the verification: chosen frames of a battle to PNGs (a trace's, a
//! match file's, a replay's), the content and trace audits, a match's setup
//! and a replay checked (docs/tools.md). Its options are the retired
//! nettai-demo's for the same.

use nettai_frontend::driver::{Driver, LivePlayer};
use nettai_frontend::game::{Failed, Found, Game, LoadError, Sound};
use nettai_frontend::player::Player;
use nettai_frontend::session;
use nettai_render::textlayer::TextMode;
use nettai_render::vfont::TextRenderer;
use nettai_tools::headless;
use nettai_tools::trace::trace_rounds;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

struct Args {
    /// Packs given by directory, each in place of the found one of its game.
    packs: Vec<PathBuf>,
    content: Option<PathBuf>,
    mute: bool,
    /// The trace (several with --audit).
    traces: Vec<PathBuf>,
    round: usize,
    /// Play this match file; write the match played to that one.
    match_file: Option<PathBuf>,
    save_match: Option<PathBuf>,
    show_folders: bool,
    keys: Option<String>,
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
    text: TextMode,
    font: Option<PathBuf>,
    lang: String,
    /// Write a replay of the set played headless to this file.
    record: Option<PathBuf>,
    /// Play this replay; shown from this side (none: the recorder's).
    replay: Option<PathBuf>,
    side: Option<u8>,
}

const USAGE: &str = "\
usage: nettai-tool [OPTIONS] TRACE.jsonl --headless FRAMES [--out DIR] [--png-scale N]
       nettai-tool [OPTIONS] --match FILE --headless FRAMES [--keys K]
       nettai-tool [OPTIONS] --replay FILE --headless FRAMES [--side SIDE]
       nettai-tool [OPTIONS] --match FILE [--show-folders] [--save-match FILE]
       nettai-tool [OPTIONS] --replay FILE
       nettai-tool [OPTIONS] --match FILE --audit-content
       nettai-tool [OPTIONS] --audit TRACE.jsonl...

  The battles of one game, EXE6 or EXE5: a match file names its game, a
  replay and a trace state their own. The game is played on its content
  folder and the support folders it uses, drawn and heard from its pack
  (written by `nettai-extract <exe5|exe6> <pack-dir> [ROM ...]`), found in
  the packs directory, $NETTAI_PACKS, else data, each pack by the game it
  says. Nothing opens a window: a battle is played in a window by nettai.
  --pack DIR       a pack's directory, in place of the found pack of its game
  --content DIR    the content directory (default: $NETTAI_CONTENT, else
                   this repository's content/)
  --mute           no sound in the audits (headless rendering plays none)
  --round N        the trace round to start with (default 1; later rounds follow)
  --match FILE     the match this file sets up (docs/frontend.md §6): its
                   setup said on the terminal (the folders with
                   --show-folders), its frames rendered with --headless; with
                   --audit-content, the game whose content is audited.
                   Its seed sets the battle's RNG (else the clock's)
  --save-match FILE  write the match played with its seed to FILE as a match
                   file, to play again
  --show-folders   say both players' folders with the setup
  --record FILE    with --match --headless: write the set played to FILE as
                   a replay (docs/frontend.md §8)
  --replay FILE    a replay: played to its end and checked against the
                   digests it kept (what happened, and whether it
                   reproduced; exits 1 where it doesn't); with --headless F,
                   its frames F (ticks of the set) rendered
  --side SIDE      with --replay --headless: the console shown, left or right
                   (default: the side that recorded it)
  --headless F     render frames F (e.g. 150,300,600 or 100-120; trace frame
                   numbers, or ticks in live play) to frame_NNNNN.png files
                   in --out (default .)
  --out DIR        where --headless writes
  --png-scale N    with --headless: each frame N times its size
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
                   and audio make for everything the content defines (every
                   chip's icon, picture and name, every navi's and form's
                   face, every asset the content names), in every language,
                   and list what the packs don't have; exits 1 if there was any
  --audit          run the traces (several at a time) making each frame's
                   lookups and checking each sound cue, and list what they
                   named that the packs don't have (a sprite, an animation, a
                   palette, a chip's icon or name glyph, a banner, a song);
                   exits 1 if there was any
  --draw           with --audit: draw every frame and play every sound cue
                   into nothing as well (slower)
  --jobs N         with --audit: traces at a time (default: one a core)
  --lookups FILE   with --audit: write each trace's lookups to FILE (a line
                   each: the trace, a tab, the lookup), for the trace cover;
                   with --audit-content, its own (as the trace \"content\")
  --text MODE      how strings are drawn: font (default) draws the names, the
                   telop, the chatbox and the HUD's lines with a vector font
                   over the scaled frame; original draws them in the game's
                   own fonts into the frame, as the original does (what the
                   frame comparison uses)
  --font PATH      the font mode's font (a TrueType or OpenType file) instead
                   of the bundled one (Murecho)
  --lang LANG      the language of the battle's text: en (default, the
                   content's own) or ja (the content's locales/ja.toml, and
                   the pack's fonts and pictures with text)";

fn parse(mut it: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut a = Args {
        packs: Vec::new(),
        content: None,
        mute: false,
        traces: Vec::new(),
        round: 1,
        match_file: None,
        save_match: None,
        show_folders: false,
        keys: None,
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
        text: TextMode::Font,
        font: None,
        lang: nettai_assets::BASE_LANGUAGE.into(),
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
            "--save-match" => a.save_match = Some(value("--save-match")?.into()),
            "--show-folders" => a.show_folders = true,
            "--keys" => a.keys = Some(value("--keys")?),
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
            "--text" => a.text = value("--text")?.parse()?,
            "--font" => a.font = Some(value("--font")?.into()),
            "--lang" => a.lang = value("--lang")?,
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
            s if s.starts_with('-') => return Err(format!("unknown option {s} (a window's options are nettai's)")),
            s => a.traces.push(s.into()),
        }
    }
    let sources = [a.match_file.is_some(), a.replay.is_some(), !a.traces.is_empty()].iter().filter(|s| **s).count();
    if sources != 1 {
        return Err("give one of a trace file (several with --audit), --match FILE or --replay FILE".into());
    }
    if a.side.is_some() && a.replay.is_none() {
        return Err("--side goes with --replay".into());
    }
    if a.record.is_some() && (a.match_file.is_none() || a.headless.is_none()) {
        return Err("--record writes the set a match file plays headless (--match FILE --headless F)".into());
    }
    if a.keys.is_some() && (a.match_file.is_none() || a.headless.is_none()) {
        return Err("--keys holds buttons in a match file's set played headless (--match FILE --headless F)".into());
    }
    if a.save_match.is_some() && a.match_file.is_none() {
        return Err("--save-match writes a match file's match".into());
    }
    if a.audit_content {
        if a.match_file.is_none() {
            return Err("--audit-content needs --match FILE to select the game".into());
        }
        if a.audit || a.headless.is_some() || a.save_match.is_some() {
            return Err("--audit-content audits the content alone (--audit runs traces)".into());
        }
        return Ok(a);
    }
    if a.audit && (a.traces.is_empty() || a.headless.is_some()) {
        return Err("--audit runs traces (no --headless)".into());
    }
    if (a.draw || a.lookups.is_some() || a.jobs != 0) && !a.audit {
        return Err("--draw, --jobs and --lookups go with --audit (--lookups with --audit-content too)".into());
    }
    if a.traces.len() > 1 && !a.audit {
        return Err("one trace at a time (several with --audit)".into());
    }
    if !a.traces.is_empty() && a.headless.is_none() && !a.audit {
        return Err("a trace is rendered (--headless F) or audited (--audit); no window watches one".into());
    }
    let headless_only = a.objects || !a.marks.is_empty() || a.png_scale != 1;
    if headless_only && a.headless.is_none() {
        return Err("--objects, --mark and --png-scale go with --headless".into());
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
    nettai_match::parse(content, text).unwrap_or_else(|problems| fail(format!("{} can't be played:\n  {}", path.display(), problems.join("\n  "))))
}

/// Write the match played, with the seed it was played from.
fn save_match(content: &nettai_battle::Content, m: &nettai_match::Match, seed: u32, path: &Path) {
    let m = nettai_match::Match { seed: Some(seed), ..m.clone() };
    std::fs::write(path, nettai_match::write(content, &m)).unwrap_or_else(|e| fail(format!("can't write {}: {e}", path.display())));
    eprintln!("wrote the match to {}", path.display());
}

/// A recording of match `m` (its seed stated) played from side 0 to the
/// file `path`, or stop with why not.
fn recorder(content: &Arc<nettai_battle::Content>, m: &nettai_match::Match, path: &Path) -> nettai_frontend::replay::Recorder {
    use nettai_frontend::replay::{Info, Recorder};
    let file = std::fs::File::create(path).unwrap_or_else(|e| fail(format!("can't write {}: {e}", path.display())));
    let when = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let info = Info { when, side: 0, names: Default::default() };
    let r = Recorder::new(Box::new(std::io::BufWriter::new(file)), content, m, &info).unwrap_or_else(|e| fail(format!("can't record to {}: {e}", path.display())));
    eprintln!("recording the set to {}", path.display());
    r
}

/// `--audit-content`: every lookup for everything the content defines, in
/// each language the content has strings in; then exit.
fn audit_content(args: &Args, game: &Game) -> ! {
    let t = Instant::now();
    let (content, dir, games) = (&*game.content, &*game.dir, std::slice::from_ref(&game.name));
    let bundles = game.packs.iter().map(|p| load(p, "graphics", nettai_content::pack::load_graphics)).collect();
    let banks: Option<Vec<Arc<m4a::SoundBank>>> = (!args.mute).then(|| sound_of(game).banks);
    let own_lang = nettai_content::locale::OWN;
    let mut languages: Vec<nettai_tools::content_audit::Language> = vec![(own_lang.to_string(), None)];
    for lang in nettai_content::locale::languages(dir).into_iter().filter(|l| l != own_lang) {
        let strings = nettai_content::locale::load_for(dir, games, &lang).unwrap_or_else(|e| fail(e));
        languages.push((lang, strings.map(Arc::new)));
    }
    let found = nettai_tools::content_audit::audit(content, bundles, game.own, banks.as_deref(), &languages);
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

/// A replay played to its end and checked: what happened, and whether it
/// reproduced; then exit (1 where it doesn't).
fn check_replay(content: &Arc<nettai_battle::Content>, replay: &nettai_frontend::replay::Replay, path: &Path) -> ! {
    let outcome = nettai_frontend::replay::play_out(content, replay).unwrap_or_else(|e| fail(format!("can't play {}: {e}", path.display())));
    let rounds: Vec<&str> = outcome.rounds.iter().map(|w| match w {
        Some(0) => "left",
        Some(_) => "right",
        None => "draw",
    }).collect();
    println!("{}: {} ticks; rounds won: {}; the set: {:?}", path.display(), outcome.ticks, rounds.join(", "), outcome.result);
    if let Some(why) = &outcome.stopped {
        println!("stopped: {why}");
    }
    match &outcome.diverged {
        Some(at) => {
            println!("doesn't reproduce: {at}");
            std::process::exit(1)
        }
        None => {
            println!("reproduces");
            std::process::exit(0)
        }
    }
}

fn main() {
    let args = match parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("{e}\n");
            }
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    };
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
        (Some(text), _, _) => {
            nettai_match::file::game_of(text).unwrap_or_else(|e| fail(format!("{} can't be played: {e}", args.match_file.as_ref().unwrap().display())))
        }
        (None, Some(r), _) => r.head.game.clone(),
        (None, None, Some(trace)) => nettai_tools::trace::trace_game(trace).unwrap_or_else(|e| fail(format!("can't play {}: {e}", trace.display()))),
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
    // Without frames: a replay checked, a match's setup said.
    if args.headless.is_none() && !args.audit {
        if let (Some(path), Some(r)) = (&args.replay, &replay) {
            check_replay(&content, r, path);
        }
        if let Some((path, text)) = args.match_file.as_deref().zip(file_text.as_deref()) {
            let m = read_match(&content, path, text);
            let seed = m.seed.unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1));
            println!("{}", nettai_match::describe(&content, &m, seed, args.show_folders, 0));
            if let Some(path) = &args.save_match {
                save_match(&content, &m, seed, path);
            }
            return;
        }
    }
    // The game's pack's graphics, in the language.
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

    // What is played: a replay; live play's set; a recording's rounds, a
    // driver each, in turn.
    let mut drivers: Vec<Box<dyn Driver>> = Vec::new();
    let mut recording = None;
    if let (Some(path), Some(r)) = (&args.replay, &replay) {
        let player = nettai_frontend::replay::ReplayPlayer::new(&content, r, args.side).unwrap_or_else(|e| fail(format!("can't play {}: {e}", path.display())));
        let m = nettai_match::binary::read_match(&content, &r.match_bytes).expect("a replay that plays has a match");
        eprintln!("{}", nettai_match::describe(&content, &m, m.seed.unwrap_or(0), args.show_folders, args.side.unwrap_or(r.info.side) as usize));
        drivers.push(Box::new(player));
    } else if let Some((path, text)) = args.match_file.as_deref().zip(file_text.as_deref()) {
        let m = read_match(&content, path, text);
        let seed = m.seed.unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1));
        eprintln!("{}", nettai_match::describe(&content, &m, seed, args.show_folders, 0));
        if let Some(path) = &args.save_match {
            save_match(&content, &m, seed, path);
        }
        if let Some(path) = &args.record {
            recording = Some(recorder(&content, &nettai_match::Match { seed: Some(seed), ..m.clone() }, path));
        }
        drivers.push(Box::new(LivePlayer::new(nettai_match::Set::of(&content, &m, seed))));
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
    let list = args.headless.as_deref().expect("frames are rendered here");
    let first = drivers.remove(0);
    let wanted = headless::parse_frames(list).unwrap_or_else(|e| fail(e));
    let keys = headless::KeyScript::parse(args.keys.as_deref().unwrap_or("")).unwrap_or_else(|e| fail(e));
    let mut log = |s: &str| eprintln!("{s}");
    // (Headless rendering plays no sound.)
    let mut player = Player::with(renderer, text, None, first);
    if let Some(r) = recording {
        player.record(r).unwrap_or_else(|e| fail(e));
    }
    match headless::render_frames_with(&mut player, drivers, &wanted, &args.out, args.png_scale, args.objects, &keys, &mut log) {
        Ok(written) => {
            eprintln!("wrote {} frames to {}", written.len(), args.out.display());
            if written.len() < wanted.len() {
                std::process::exit(1);
            }
        }
        Err(e) => fail(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Result<Args, String> {
        parse(values.iter().map(|s| s.to_string()))
    }

    /// The options verify's scripts give (the retired nettai-demo's, for the same).
    #[test]
    fn verifys_command_lines_parse() {
        for options in [
            vec!["--text", "original", "--lang", "ja", "--mark", "sprite:megaman,chip-window-at-close", "--pack", "p", "t.jsonl", "--headless", "1-100000", "--out", "f"],
            vec!["--pack", "p", "t.jsonl", "--audit"],
            vec!["--pack", "p", "--draw", "--audit", "a.jsonl", "b.jsonl", "--lookups", "l.tsv"],
            vec!["--match", "exe6.toml", "--pack", "p", "--audit-content", "--lookups", "s.tsv"],
            vec!["--match", "m.toml", "--mute", "--headless", "100,300", "--out", "o", "--keys", "100:a"],
            vec!["--replay", "set.ntrp", "--headless", "1-60", "--side", "right"],
            vec!["--match", "m.toml", "--show-folders", "--save-match", "played.toml"],
            vec!["--replay", "set.ntrp"],
        ] {
            assert!(args(&options).is_ok(), "{options:?}: {:?}", args(&options).err());
        }
    }

    /// What needs a window isn't here; what goes together does.
    #[test]
    fn what_doesnt_go_together_is_refused() {
        for (options, why) in [
            (vec![], "give one of a trace file (several with --audit), --match FILE or --replay FILE"),
            (vec!["--match", "m.toml", "--host", "7777"], "unknown option --host (a window's options are nettai's)"),
            (vec!["--match", "m.toml", "--replay", "r.ntrp"], "give one of a trace file (several with --audit), --match FILE or --replay FILE"),
            (vec!["t.jsonl"], "a trace is rendered (--headless F) or audited (--audit); no window watches one"),
            (vec!["--audit-content"], "give one of a trace file (several with --audit), --match FILE or --replay FILE"),
            (vec!["--match", "m.toml", "--keys", "1:a"], "--keys holds buttons in a match file's set played headless (--match FILE --headless F)"),
            (vec!["--match", "m.toml", "--side", "left"], "--side goes with --replay"),
            (vec!["a.jsonl", "b.jsonl", "--headless", "1"], "one trace at a time (several with --audit)"),
            (vec!["--match", "m.toml", "--objects"], "--objects, --mark and --png-scale go with --headless"),
        ] {
            assert_eq!(args(&options).err().as_deref(), Some(why), "{options:?}");
        }
    }
}
