//! `bn6-content`: turn the extracted graphics bundle and sound bank into a
//! content pack of open formats, check a pack, build the binary data from
//! it, and verify that it reads back exactly.
//!
//!     bn6-content export <pack> [--graphics <bn6-assets.bin>] [--sound <bank>]
//!     bn6-content check <pack>
//!     bn6-content build <pack> [--graphics-out <file>] [--sound-out <file>]
//!     bn6-content verify <pack> [--graphics <bn6-assets.bin>] [--sound <bank>] [--seconds N]
//!
//! The pack holds the game's own graphics and recordings when exported from
//! a ROM's data: keep it out of version control (data/ is ignored).

use bn6_assets::Bundle;
use bn6_content::report::{Level, Report};
use bn6_content::{pack, timing, verify};
use m4a::{SongId, SoundBank};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

struct Args {
    command: String,
    pack: PathBuf,
    graphics: Option<PathBuf>,
    sound: Option<PathBuf>,
    graphics_out: Option<PathBuf>,
    sound_out: Option<PathBuf>,
    seconds: f64,
}

const USAGE: &str = "usage:
  bn6-content export <pack> [--graphics <bn6-assets.bin>] [--sound <bank>]
  bn6-content check <pack>
  bn6-content build <pack> [--graphics-out <file>] [--sound-out <file>]
  bn6-content verify <pack> [--graphics <bn6-assets.bin>] [--sound <bank>] [--seconds N]";

fn parse() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let command = it.next().ok_or(USAGE)?;
    let pack = PathBuf::from(it.next().ok_or(USAGE)?);
    let mut a = Args { command, pack, graphics: None, sound: None, graphics_out: None, sound_out: None, seconds: 60.0 };
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--graphics" => a.graphics = Some(value()?.into()),
            "--sound" => a.sound = Some(value()?.into()),
            "--graphics-out" => a.graphics_out = Some(value()?.into()),
            "--sound-out" => a.sound_out = Some(value()?.into()),
            "--seconds" => a.seconds = value()?.parse().map_err(|_| "--seconds takes a number")?,
            f => return Err(format!("unknown option {f}\n{USAGE}")),
        }
    }
    Ok(a)
}

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

fn load_bundle(p: &PathBuf) -> Bundle {
    Bundle::load(p).unwrap_or_else(|e| fail(format!("{}: {e}", p.display())))
}

fn load_bank(p: &PathBuf) -> SoundBank {
    let bytes = std::fs::read(p).unwrap_or_else(|e| fail(format!("{}: {e}", p.display())));
    SoundBank::from_bytes(&bytes).unwrap_or_else(|e| fail(format!("{}: {e}", p.display())))
}

fn print_report(r: &Report, verbose: bool) {
    for i in &r.issues {
        if verbose || i.level != Level::Note {
            eprintln!("{i}");
        }
    }
    let notes = r.count(Level::Note);
    eprintln!(
        "{} errors, {} warnings, {notes} notes",
        r.count(Level::Error),
        r.count(Level::Warning)
    );
}

fn main() {
    let a = parse().unwrap_or_else(|e| fail(e));
    match a.command.as_str() {
        "export" => export(&a),
        "check" => {
            let mut r = Report::default();
            let m = pack::read_manifest(&a.pack, &mut r).unwrap_or_else(|| fail(&r));
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
        "build" => {
            let mut r = Report::default();
            if let Some(out) = &a.graphics_out {
                let b = pack::import_graphics(&a.pack, &mut r).unwrap_or_else(|| fail(&r));
                std::fs::write(out, b.to_bytes()).unwrap_or_else(|e| fail(e));
                eprintln!("wrote {}", out.display());
            }
            if let Some(out) = &a.sound_out {
                let b = pack::import_sound(&a.pack, &mut r).unwrap_or_else(|| fail(&r));
                std::fs::write(out, b.to_bytes()).unwrap_or_else(|e| fail(e));
                eprintln!("wrote {}", out.display());
            }
            print_report(&r, false);
        }
        "verify" => verify_pack(&a),
        _ => fail(USAGE),
    }
}

fn export(a: &Args) {
    let t = Instant::now();
    let bundle = a.graphics.as_ref().map(load_bundle);
    let bank = a.sound.as_ref().map(load_bank);
    if bundle.is_none() && bank.is_none() {
        fail("nothing to export: give --graphics and/or --sound");
    }
    let mut files = vec![pack::manifest("BN6 (US Falzar) battle content", bundle.as_ref(), bank.is_some())];
    if let Some(b) = &bundle {
        files.extend(pack::export_graphics(b));
    }
    if let Some(b) = &bank {
        let (f, failures) = pack::export_sound(b);
        files.extend(f);
        for (id, e) in failures {
            eprintln!("song {:#05x} left out: {e}", id.0);
        }
    }
    pack::write_files(&a.pack, &files).unwrap_or_else(|e| fail(e));
    let bytes: usize = files.iter().map(|f| f.1.len()).sum();
    eprintln!("wrote {} files ({} KiB) to {} in {:.1?}", files.len(), bytes / 1024, a.pack.display(), t.elapsed());
}

/// BN6's battle music and the effects the battle engine plays.
const BATTLE_MUSIC: [u16; 4] = [0x15, 0x19, 0x1A, 0x1F];
const BATTLE_EFFECTS: [u16; 18] = [
    0x94, 0x6B, 0x6D, 0x86, 0x6C, 0x118, 0x12D, 0x124, 0x8E, 0x6E, 0x8A, 0x69, 0x71, 0x72, 0x8F, 0x97, 0x85, 0xC0,
];

fn verify_pack(a: &Args) {
    let mut failed = false;
    if let Some(g) = &a.graphics {
        let t = Instant::now();
        let original = load_bundle(g);
        let mut r = Report::default();
        let back = pack::import_graphics(&a.pack, &mut r);
        print_report(&r, false);
        match back {
            None => {
                eprintln!("graphics: the pack doesn't import");
                failed = true;
            }
            Some(b) => {
                let diffs = verify::compare_graphics(&original, &b);
                if diffs.is_empty() {
                    eprintln!(
                        "graphics: identical ({} sprites, {} backgrounds, field, HUD; {} bytes) in {:.1?}",
                        b.sprites.len(),
                        b.backgrounds.iter().flatten().count(),
                        b.to_bytes().len(),
                        t.elapsed()
                    );
                } else {
                    failed = true;
                    for d in diffs.iter().take(20) {
                        eprintln!("graphics: {d}");
                    }
                    eprintln!("graphics: {} differences", diffs.len());
                }
            }
        }
        let mut r = Report::default();
        match timing::load(&a.pack, &mut r) {
            Some(t) => {
                let d = verify::compare_timing(&t, &original);
                let frames: usize = t.sprites.values().flatten().map(Vec::len).sum();
                if d.is_empty() {
                    eprintln!("timing: identical ({} sprites, {frames} frames), read without images", t.sprites.len());
                } else {
                    failed = true;
                    eprintln!("timing: {}", d.join("; "));
                }
            }
            None => {
                failed = true;
                print_report(&r, false);
            }
        }
    }
    if let Some(s) = &a.sound {
        let t = Instant::now();
        let original = Arc::new(load_bank(s));
        let mut r = Report::default();
        let back = pack::import_sound(&a.pack, &mut r);
        print_report(&r, false);
        let Some(back) = back else {
            fail("sound: the pack doesn't import");
        };
        let back = Arc::new(back);
        let same_instruments = original.voicegroups == back.voicegroups
            && original.samples == back.samples
            && original.waves == back.waves
            && original.key_maps == back.key_maps
            && original.mixer == back.mixer
            && original.players == back.players
            && original.songs.len() == back.songs.len();
        eprintln!(
            "sound: instruments, samples, mixer and players {}",
            if same_instruments { "identical" } else { "DIFFER" }
        );
        failed |= !same_instruments;
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
                    eprintln!("song {id:#05x}: present in only one bank");
                }
            }
        }
        eprintln!(
            "songs: {same} with identical timelines ({structural} also command for command), {differ} differ"
        );
        failed |= differ > 0;
        // Render every song alone, and the battle's music and effects mixed.
        let frames = (a.seconds * m4a::FPS) as usize;
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
            "PCM: {} of {} songs render bit-identical over {:.0} s each ({} stereo samples), battle songs and effects: {} of {}",
            results.len() - bad.len(),
            results.len(),
            a.seconds,
            samples,
            battle.len() - battle_bad,
            battle.len()
        );
        failed |= !bad.is_empty();
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
                failed = true;
                eprintln!("PCM: the battle mix differs from sample {i}");
            }
        }
        eprintln!("sound verified in {:.1?}", t.elapsed());
    }
    if failed {
        std::process::exit(1);
    }
}
