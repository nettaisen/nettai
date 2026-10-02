//! `nettai-content`: check a content pack, compare it with another, and edit
//! its sprites in Aseprite.
//!
//!     nettai-content check <pack> [--content DIR]
//!     nettai-content verify <pack> <reference-pack> [--seconds N]
//!     nettai-content aseprite-export <pack> [NAME ...]
//!     nettai-content aseprite-import <pack> [NAME ...]
//!
//! A pack exported from a ROM (`bn6-extract content`) holds the game's own
//! data: keep it out of version control (data/ is ignored).

use nettai_content::report::{Level, Report};
use nettai_content::{pack, root, timing, verify};
use m4a::SongId;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

struct Args {
    command: String,
    pack: PathBuf,
    reference: Option<PathBuf>,
    /// The content root whose definitions `check` loads with the pack.
    content: PathBuf,
    seconds: f64,
    /// Sprite folders to work on (aseprite commands; none: all).
    only: Vec<String>,
}

const USAGE: &str = "usage:
  nettai-content check <pack> [--content DIR]         read every file and report what it finds, and
                                                   define the content root's definitions with the
                                                   pack's assets (default: $BN6_CONTENT, else this
                                                   repository's content/bn6)
  nettai-content verify <pack> <reference-pack> [--seconds N]
                                                   check that two packs load as the same content
                                                   (graphics, sprite timing, sound as timelines and
                                                   as N seconds of PCM per song)
  nettai-content aseprite-export <pack> [NAME ...]   write sprites' Aseprite views
  nettai-content aseprite-import <pack> [NAME ...]   read the views back into the sprites' files";

fn parse() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let command = it.next().ok_or(USAGE)?;
    let pack = PathBuf::from(it.next().ok_or(USAGE)?);
    let mut a = Args { command, pack, reference: None, content: root::bn6(), seconds: 60.0, only: Vec::new() };
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--content" => a.content = it.next().ok_or("--content needs a directory")?.into(),
            "--seconds" => {
                let v = it.next().ok_or("--seconds needs a value")?;
                a.seconds = v.parse().map_err(|_| "--seconds takes a number")?;
            }
            f if f.starts_with("--") => return Err(format!("unknown option {f}\n{USAGE}")),
            f if a.command == "verify" && a.reference.is_none() => a.reference = Some(f.into()),
            f => a.only.push(f.to_string()),
        }
    }
    Ok(a)
}

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

fn print_report(r: &Report, verbose: bool) {
    for i in &r.issues {
        if verbose || i.level != Level::Note {
            eprintln!("{i}");
        }
    }
    let notes = r.count(Level::Note);
    eprintln!("{} errors, {} warnings, {notes} notes", r.count(Level::Error), r.count(Level::Warning));
}

fn main() {
    let a = parse().unwrap_or_else(|e| fail(e));
    match a.command.as_str() {
        "check" => {
            let mut r = Report::default();
            let m = pack::read_manifest(&a.pack, &mut r).unwrap_or_else(|| fail(&r));
            // The define phase: every module loads, and what the modules
            // define fits together, with the pack's assets.
            let t = std::time::Instant::now();
            match pack::load_battle(&a.content, &a.pack) {
                Ok((c, loaded)) => {
                    r.issues.extend(loaded.issues);
                    r.note(
                        a.content.display().to_string(),
                        format!(
                            "{} modules define {} definitions ({:.1?})",
                            c.scripts.modules.len(),
                            c.defs.definitions.defs.len(),
                            t.elapsed()
                        ),
                    );
                    nettai_content::lint::definitions(&c, &mut r);
                    // Each root's tables, against its definitions.
                    for dir in nettai_content::root::dirs(&a.content).unwrap_or_default() {
                        nettai_content::locale::check_root(&dir, &c, &mut r);
                    }
                }
                Err(failed) => r.issues.extend(failed.issues),
            }
            if m.graphics.is_some() {
                pack::import_graphics(&a.pack, &mut r);
            }
            if m.sound.is_some() {
                pack::import_sound(&a.pack, &mut r);
            }
            print_report(&r, true);
            if r.has_errors() {
                std::process::exit(1);
            }
        }
        "verify" => verify_packs(&a),
        "aseprite-export" | "aseprite-import" => aseprite(&a),
        _ => fail(USAGE),
    }
}

/// Write each sprite's Aseprite view, or read the views back into the
/// sprites' files.
fn aseprite(a: &Args) {
    let root = a.pack.join("graphics/sprites");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap_or_else(|e| fail(format!("{}: {e}", root.display())))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("sprite.json").is_file())
        .filter(|p| a.only.is_empty() || a.only.iter().any(|o| p.file_name().is_some_and(|n| n == o.as_str())))
        .collect();
    dirs.sort();
    let mut r = Report::default();
    let mut done = 0;
    for dir in &dirs {
        let name = format!("graphics/sprites/{}", dir.file_name().unwrap().to_string_lossy());
        let Some(sheet) = nettai_content::sprite::import(dir, &name, &mut r) else { continue };
        let view = dir.join("sprite.aseprite");
        if a.command == "aseprite-export" {
            std::fs::write(&view, nettai_content::aseprite::export(&sheet)).unwrap_or_else(|e| fail(e));
            done += 1;
        } else if let Ok(bytes) = std::fs::read(&view) {
            let file = format!("{name}/sprite.aseprite");
            if let Some(new) = nettai_content::aseprite::import(&bytes, &sheet, &file, &mut r) {
                for (f, data) in nettai_content::sprite::export(&new) {
                    std::fs::write(dir.join(f), data).unwrap_or_else(|e| fail(e));
                }
                done += 1;
            }
        }
    }
    print_report(&r, false);
    eprintln!("{} {done} of {} sprites", if a.command == "aseprite-export" { "wrote the views of" } else { "read back" }, dirs.len());
}

/// BN6's battle music and the effects the battle engine plays.
const BATTLE_MUSIC: [u16; 4] = [0x15, 0x19, 0x1A, 0x1F];
const BATTLE_EFFECTS: [u16; 18] = [
    0x94, 0x6B, 0x6D, 0x86, 0x6C, 0x118, 0x12D, 0x124, 0x8E, 0x6E, 0x8A, 0x69, 0x71, 0x72, 0x8F, 0x97, 0x85, 0xC0,
];

/// Load one part of both packs, or say why not.
fn both<T>(what: &str, load: impl Fn(&std::path::Path) -> Result<(T, Report), Report>, a: &Args) -> Option<(T, T)> {
    let reference = a.reference.as_ref().expect("checked");
    let one = |p: &std::path::Path| match load(p) {
        Ok((v, _)) => Some(v),
        Err(r) => {
            print_report(&r, false);
            eprintln!("{what}: {} doesn't load", p.display());
            None
        }
    };
    Some((one(&a.pack)?, one(reference)?))
}

fn verify_packs(a: &Args) {
    let Some(reference) = &a.reference else { fail(USAGE) };
    let mut failed = false;
    let mut report = |what: &str, diffs: Vec<String>, same: String| {
        if diffs.is_empty() {
            eprintln!("{what}: identical ({same})");
        } else {
            failed = true;
            for d in diffs.iter().take(20) {
                eprintln!("{what}: {d}");
            }
            eprintln!("{what}: {} differences", diffs.len());
        }
    };
    let t = Instant::now();
    if let Some((x, y)) = both("graphics", pack::load_graphics, a) {
        let same = format!("{} sprites, {} backgrounds, field, HUD, in {:.1?}", x.sprites.len(), x.backgrounds.iter().flatten().count(), t.elapsed());
        report("graphics", verify::compare_graphics(&y, &x), same);
        // Sprite timing is read on its own too (without images); it must
        // match the sprites' animations.
        let mut r = Report::default();
        match timing::load(&a.pack, &mut r) {
            Some(t) => {
                let frames: usize = t.sprites.values().flatten().map(Vec::len).sum();
                report("timing", verify::compare_timing(&t, &x), format!("{} sprites, {frames} frames, read without images", t.sprites.len()));
            }
            None => {
                failed = true;
                print_report(&r, false);
            }
        }
    }
    if let Some((x, y)) = both("sound", pack::load_sound, a) {
        failed |= !verify_sound(Arc::new(y), Arc::new(x), a.seconds, &reference.display().to_string());
    }
    if failed {
        std::process::exit(1);
    }
}

/// The pack's sound (`back`) against the reference's (`original`): the
/// instruments, every song as timelines, and every song rendered alone and
/// the battle's music and effects mixed. True if identical.
fn verify_sound(original: Arc<m4a::SoundBank>, back: Arc<m4a::SoundBank>, seconds: f64, reference: &str) -> bool {
    let t = Instant::now();
    let mut ok = true;
    let same_instruments = original.voicegroups == back.voicegroups
        && original.samples == back.samples
        && original.waves == back.waves
        && original.key_maps == back.key_maps
        && original.mixer == back.mixer
        && original.players == back.players
        && original.songs.len() == back.songs.len();
    eprintln!("sound: instruments, samples, mixer and players {} to {reference}'s", if same_instruments { "identical" } else { "DIFFER" });
    ok &= same_instruments;
    let (mut same, mut differ, mut structural) = (0, 0, 0);
    for (id, (x, y)) in original.songs.iter().zip(back.songs.iter()).enumerate() {
        match (x, y) {
            (None, None) => {}
            (Some(x), Some(y)) => {
                if x == y {
                    structural += 1;
                }
                match verify::compare_song(x, y) {
                    None => same += 1,
                    Some(e) => {
                        differ += 1;
                        eprintln!("song {id:#05x}: {e}");
                    }
                }
            }
            _ => {
                differ += 1;
                eprintln!("song {id:#05x}: present in only one pack");
            }
        }
    }
    eprintln!("songs: {same} with identical timelines ({structural} also command for command), {differ} differ");
    ok &= differ == 0;
    // Render every song alone, and the battle's music and effects mixed.
    let frames = (seconds * m4a::FPS) as usize;
    let ids: Vec<usize> = (0..original.songs.len()).filter(|&i| original.songs[i].is_some()).collect();
    let results: Vec<(usize, Option<usize>, usize)> = {
        let n = std::thread::available_parallelism().map_or(4, |n| n.get());
        let chunk = ids.len().div_ceil(n).max(1);
        std::thread::scope(|sc| {
            let hs: Vec<_> = ids
                .chunks(chunk)
                .map(|c| {
                    let (o, b) = (original.clone(), back.clone());
                    sc.spawn(move || {
                        c.iter()
                            .map(|&i| {
                                let x = verify::render(&o, SongId(i as u16), frames);
                                let y = verify::render(&b, SongId(i as u16), frames);
                                (i, verify::first_difference(&x, &y), x.len())
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
        })
    };
    let bad: Vec<_> = results.iter().filter(|r| r.1.is_some()).collect();
    let samples: usize = results.iter().map(|r| r.2).sum();
    for (i, at, _) in &bad {
        eprintln!("song {i:#05x}: PCM differs from sample {}", at.unwrap());
    }
    let battle: Vec<u16> = BATTLE_MUSIC.iter().chain(&BATTLE_EFFECTS).copied().collect();
    let battle_bad = bad.iter().filter(|r| battle.contains(&(r.0 as u16))).count();
    eprintln!(
        "PCM: {} of {} songs render bit-identical over {seconds:.0} s each ({samples} stereo samples), battle songs and effects: {} of {}",
        results.len() - bad.len(),
        results.len(),
        battle.len() - battle_bad,
        battle.len()
    );
    ok &= bad.is_empty();
    // The battle music with every battle effect over it, a few at once.
    let mut starts = vec![(0usize, SongId(0x15))];
    for (k, &e) in BATTLE_EFFECTS.iter().enumerate() {
        starts.push((60 + 37 * k, SongId(e)));
        starts.push((61 + 37 * k, SongId(BATTLE_EFFECTS[(k + 5) % BATTLE_EFFECTS.len()])));
    }
    let mix_frames = 60 + 37 * BATTLE_EFFECTS.len() + 600;
    let x = verify::render_mix(&original, &starts, mix_frames);
    let y = verify::render_mix(&back, &starts, mix_frames);
    match verify::first_difference(&x, &y) {
        None => eprintln!("PCM: battle music with all {} battle effects mixed in: bit-identical ({} samples)", BATTLE_EFFECTS.len(), x.len()),
        Some(i) => {
            ok = false;
            eprintln!("PCM: the battle mix differs from sample {i}");
        }
    }
    eprintln!("sound verified in {:.1?}", t.elapsed());
    ok
}
