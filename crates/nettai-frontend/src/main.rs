//! nettai-frontend: watch a golden trace replayed through the engine, or play.
//! See docs/frontend.md.

use nettai_frontend::driver::{LivePlayer, TracePlayer};
use nettai_frontend::textlayer::TextMode;
use nettai_frontend::vfont::{TextRenderer, VectorFont};
use nettai_frontend::{Renderer, Session, TickHook, app, headless, session};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

struct Args {
    /// The packs (one a game: docs/design/rules-in-luau.md §7.4).
    packs: Vec<PathBuf>,
    content: Option<PathBuf>,
    mute: bool,
    trace: Option<PathBuf>,
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

/// Where `bn6-extract content <falzar-us> <gregar-us> <falzar-jp> <gregar-jp> <dir>` puts the BN6 pack by default.
const DEFAULT_PACK: &str = "data/content/bn6";

const USAGE: &str = "\
usage: nettai-frontend [OPTIONS] TRACE.jsonl     watch a trace's rounds
       nettai-frontend [OPTIONS] --play          play live (you are the left navi)
       nettai-frontend [OPTIONS] --match FILE    play a match file (you are its left side)
       nettai-frontend [OPTIONS] --play --host PORT        play another player over the
       nettai-frontend [OPTIONS] --play --join ADDR:PORT   network: host, or join the host
       nettai-frontend [OPTIONS] TRACE.jsonl --headless FRAMES [--out DIR] [--png-scale N]
       nettai-frontend [OPTIONS] TRACE.jsonl --audit

  --pack DIR       the content pack to play (graphics and sound), from
                   `bn6-extract content <falzar-us> <gregar-us> <falzar-jp> <gregar-jp> <dir>` (default: $BN6_PACK, else
                   data/content/bn6); again for another game's pack, loaded
                   beside it: each asset draws and sounds from its own pack
  --content DIR    the battle content: the definitions that name the pack's
                   assets (default: $BN6_CONTENT, else this repository's
                   content/bn6)
  --mute           no sound (headless rendering never plays any)
  --round N        the trace round to start with (default 1; later rounds follow)
  --seed N         live play's seed: the field, the folders, the Crosses
                   offered and the battle's RNG are drawn from it (default:
                   from the clock); each start prints it
  --stage NAME     live play on this link battle stage (its key, e.g.
                   netbattle-43) instead of a random one
  --cards KEYS     live play: your patch cards (the Japanese games'
                   Modification Cards), their keys comma-separated in the
                   order they apply; -KEY installs one switched off (e.g.
                   canodumb,-shadow)
  --their-cards KEYS  the right navi's patch cards, likewise
  --match FILE     play the match this file sets up (docs/frontend.md §6: the
                   arena, each side's ruleset, navi, game, folder, Crosses,
                   patch cards and stats, by content key; nettai-editor makes
                   them), instead of a random one; you are its left side. With
                   --host or --join the left side is what you bring, and the
                   host's arena is the match's
  --save-match FILE  write the match played (live play's random draw, or the
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
  --audit          draw every frame and play every sound cue into nothing, no
                   window, and list what they named that the pack doesn't
                   have (a sprite, an animation, a palette, a chip's icon or
                   name glyph, a banner, a song); exits 1 if there was any
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
                   engine and content (the handshake checks); each brings
                   their own folder, game and Crosses (drawn from their
                   --seed) and patch cards (--cards); the host's --stage
                   picks the stage; the field and the battle's RNG come
                   from both players' seeds
  --delay N        netplay's input delay in frames (default 2): more delay,
                   fewer rollbacks
  --wait SECONDS   how long the host waits for a player, or the joiner for
                   the host (default 300 and 30)";

fn parse() -> Result<Args, String> {
    let mut a = Args {
        packs: Vec::new(),
        content: None,
        mute: false,
        trace: None,
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
            s => a.trace = Some(s.into()),
        }
    }
    // A match file is played live.
    a.play |= a.match_file.is_some();
    if a.trace.is_none() && !a.play {
        return Err("give a trace file, --play or --match FILE".into());
    }
    if a.match_file.is_some() {
        if a.trace.is_some() {
            return Err("--match plays a match file, not a trace".into());
        }
        if a.stage.is_some() || a.cards.iter().any(Option::is_some) {
            return Err("the match file names the stage and the patch cards (edit it, or leave out --match)".into());
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

/// Load one part of the pack, or stop with what is wrong with it.
fn load<T>(pack: &Path, what: &str, f: impl Fn(&Path) -> Result<(T, nettai_content::report::Report), nettai_content::report::Report>) -> T {
    let t = Instant::now();
    match f(pack) {
        Ok((v, r)) => {
            show(&r);
            if std::env::var_os("NETTAI_LOAD_TIMES").is_some() {
                eprintln!("loaded the {what} in {:.1?}", t.elapsed());
            }
            v
        }
        Err(r) => {
            show(&r);
            fail(format!(
                "can't load the {what} of the content pack {}\n(write it with `cargo run -p bn6-extract -- content <falzar-us> <gregar-us> <falzar-jp> <gregar-jp> {}`)",
                pack.display(),
                pack.display()
            ))
        }
    }
}

/// The battle's display text in `lang`: the pack's lettering in it (fonts,
/// HUD lines, pictures with text) and the content root's strings table, if
/// the language isn't the content's own.
fn language(assets: nettai_assets::Bundle, root: &Path, lang: &str) -> (nettai_assets::Bundle, Option<nettai_content::locale::Strings>) {
    let own = nettai_content::locale::OWN;
    let strings = if lang == own { None } else { nettai_content::locale::load_all(root, lang).unwrap_or_else(|e| fail(e)) };
    if strings.is_none() && lang != own {
        let have = nettai_content::locale::languages(root).join(", ");
        fail(format!("the content ({}) has no strings in {lang:?} (it has {have})", root.display()));
    }
    let assets = assets.in_language(lang).unwrap_or_else(|e| fail(format!("{e} (extract the pack again with the Japanese ROMs)")));
    (assets, strings)
}

/// Sound: hand each tick's cues to the audio output.
fn audio_hook(banks: Vec<Arc<m4a::SoundBank>>, songs: nettai_audio::Songs) -> Box<dyn TickHook> {
    let mut out = nettai_audio::AudioOut::with_banks(banks, songs).unwrap_or_else(|e| fail(format!("no audio output: {e}")));
    Box::new(move |s: &Session| {
        match &s.sound {
            // Netplay: what the player's tracker made of the frame (plays,
            // and cancels of cues played on a wrong prediction).
            Some(actions) => out.handle_actions(actions.iter().copied()),
            None => out.handle(s.battle.sound_cues()),
        }
        out.tick();
    })
}

/// A match file, read and checked against the content.
fn read_match(content: &Arc<nettai_battle::Content>, path: &Path) -> nettai_match::Match {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| fail(format!("can't read {}: {e}", path.display())));
    nettai_match::parse(content, &text).unwrap_or_else(|problems| {
        fail(format!("{} can't be played:\n  {}", path.display(), problems.join("\n  ")))
    })
}

/// Write the match played, with the seed it was played from.
fn save_match(content: &nettai_battle::Content, m: &nettai_match::Match, seed: u32, path: &Path) {
    let m = nettai_match::Match { seed: Some(seed), ..m.clone() };
    std::fs::write(path, nettai_match::write(content, &m)).unwrap_or_else(|e| fail(format!("can't write {}: {e}", path.display())));
    eprintln!("wrote the match to {}", path.display());
}

/// A netplay match: connect (host or join), shake hands, and agree the
/// round; each player brings their side, a match file's left side or one
/// drawn from their own `seed` with their patch cards, and the host its
/// arena or stage.
fn netplay(args: &Args, content: &Arc<nettai_battle::Content>, seed: u32, file: Option<nettai_match::Match>) -> Session {
    use nettai_frontend::netplay::{NetOptions, NetPlayer, Offer, agree, hello};
    use nettai_match::{Draws, Side, link_stage, patch_cards};
    use nettai_netplay::transport::{Connection, Role, Udp};
    let offer = match file {
        Some(m) => {
            let [side, _] = m.sides;
            Offer { side, stage: None, arena: args.host.is_some().then_some(m.arena) }
        }
        None => {
            let mut side = Side::drawn(content, &mut Draws::new(seed)).unwrap_or_else(|e| fail(e));
            if let Some(list) = &args.cards[0] {
                side.cards = patch_cards(content, list).unwrap_or_else(|e| fail(e));
            }
            let stage = args.stage.as_deref().map(|key| link_stage(content, key).unwrap_or_else(|e| fail(e)));
            Offer { side, stage, arena: None }
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
    let (offers, setup, m) = agree(content, &conn, &offer).unwrap_or_else(|e| fail(format!("netplay: {e}")));
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
    let folders = offers.map(|o| o.side.folder);
    Session::new(Box::new(NetPlayer::new(content.clone(), conn, setup, folders, options)))
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
    let all_packs = match args.packs.is_empty() {
        true => vec![std::env::var_os("BN6_PACK").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(DEFAULT_PACK))],
        false => args.packs.clone(),
    };
    let root = args.content.clone().unwrap_or_else(nettai_content::root::bn6);
    let content = Arc::new(load(&all_packs[0], "battle content", |_| nettai_content::pack::load_battle_packs(&root, &all_packs)));
    // Each pack's graphics, by the content's pack order (`PackId`); the
    // content's own pack's in the player's language.
    let by_pack = nettai_content::pack::pack_paths(&content, &all_packs);
    let home = content.scripts.roots.first().map(|r| r.assets()).and_then(|g| content.assets.pack(g));
    let own = home.unwrap_or_else(|| fail("the content's own pack is not loaded"));
    let mut bundles: Vec<nettai_assets::Bundle> = Vec::new();
    let mut strings = None;
    for (i, path) in by_pack.iter().enumerate() {
        let b = load(path, "graphics", nettai_content::pack::load_graphics);
        if i == own.index() {
            let (b, s) = language(b, &root, &args.lang);
            strings = s;
            bundles.push(b);
        } else {
            bundles.push(b);
        }
    }
    session::quiet_engine_panics();
    let mut renderer = Renderer::with_packs(nettai_frontend::packs::Packs::new(bundles.iter().collect(), own));
    renderer.set_strings(strings.map(Arc::new));
    // The font mode's font, shared by the renderer (which strings it has)
    // and the text layer's drawing.
    let font = (args.text == TextMode::Font).then(|| {
        Arc::new(match &args.font {
            Some(path) => VectorFont::load(path).unwrap_or_else(|e| fail(e)),
            None => VectorFont::bundled(),
        })
    });
    renderer.set_text(args.text, font.clone());
    let mut text = font.map(TextRenderer::new);

    let mut sessions: Vec<Session> = Vec::new();
    if args.play {
        let file = args.match_file.as_deref().map(|path| read_match(&content, path));
        let seed = args.seed.or(file.as_ref().and_then(|m| m.seed)).unwrap_or_else(|| {
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1)
        });
        if args.host.is_some() || args.join.is_some() {
            sessions.push(netplay(&args, &content, seed, file));
        } else {
            let m = match file {
                Some(m) => m,
                None => {
                    let stage = args.stage.as_deref().map(|key| nettai_match::link_stage(&content, key).unwrap_or_else(|e| fail(e)));
                    let mut m = nettai_match::draw::live(&content, seed, stage).unwrap_or_else(|e| fail(e));
                    for (side, list) in args.cards.iter().enumerate() {
                        if let Some(list) = list {
                            m.sides[side].cards = nettai_match::patch_cards(&content, list).unwrap_or_else(|e| fail(e));
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
            sessions.push(Session::new(Box::new(LivePlayer::new(m.round(&content, seed), content.clone()))));
        }
    } else if let Some(path) = &args.trace {
        let rounds =
            TracePlayer::load(path, &content).unwrap_or_else(|e| fail(format!("can't read {}: {e}", path.display())));
        for r in rounds.into_iter().skip(args.round.saturating_sub(1)) {
            if let Some((a, b)) = r.frame_range() {
                eprintln!("round {}: frames {a}..={b}", r.round_number);
            }
            sessions.push(Session::new(Box::new(r)));
        }
    }
    if sessions.is_empty() {
        fail("nothing to play");
    }

    if args.audit {
        let sound = (!args.mute).then(|| by_pack.iter().map(|p| Arc::new(load(p, "sound", nettai_content::pack::load_sound))).collect());
        let found = headless::audit(&mut renderer, sessions, sound);
        for s in &found.stopped {
            eprintln!("{s}");
        }
        for line in found.problems.lines() {
            println!("{line}");
        }
        eprintln!("audit: {} frames, {} sound cues, {} problems", found.frames, found.cues, found.problems.len());
        if !found.problems.is_empty() {
            std::process::exit(1);
        }
        return;
    }
    if let Some(list) = &args.headless {
        let wanted = headless::parse_frames(list).unwrap_or_else(|e| fail(e));
        let keys = headless::KeyScript::parse(args.keys.as_deref().unwrap_or("")).unwrap_or_else(|e| fail(e));
        let mut log = |s: &str| eprintln!("{s}");
        let rendered =
            headless::render_frames_with(
                &mut renderer,
                sessions,
                &wanted,
                &args.out,
                args.png_scale,
                args.objects,
                &keys,
                text.as_mut(),
                &mut log,
            );
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

    let mut hooks: Vec<Box<dyn TickHook>> = Vec::new();
    if !args.mute {
        let banks = by_pack.iter().map(|p| Arc::new(load(p, "sound", nettai_content::pack::load_sound))).collect();
        hooks.push(audio_hook(banks, nettai_audio::Songs::of(&content.assets)));
    }
    eprintln!("{}", app::HELP);
    let opts = app::Options { scale: args.scale, start_paused: args.paused, quit_after: args.quit_after };
    if let Err(e) = app::run(&mut renderer, sessions, &mut hooks, &opts, text.as_mut()) {
        fail(format!("window: {e}"));
    }
}
