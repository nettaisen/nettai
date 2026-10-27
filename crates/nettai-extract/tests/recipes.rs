//! The recipes held to the Rust extraction they replace (docs/design/extraction-recipes.md §6.3): each game's
//! full set extracted both ways in one build, in memory (`Extraction::files`, no writes), and compared: the file
//! lists, then each file's bytes (`extraction.txt`, the warnings, among them), and the ROM reads, as ranges per ROM
//! after merging. And, with `NETTAI_EXTRACT_BASELINE` (a folder of the verification workspace's
//! `data/extract-baseline/<commit>/`), each path's files with the baseline's digests and its reads with the
//! baseline's: what holds a recipe once its game's Rust is gone (§6.5). Each path's time is said.
//!
//! The ROMs come from a folder, recognized by their headers (`NETTAI_EXTRACT_ROM_DIR`, any file names); the games
//! are `NETTAI_EXTRACT_GAMES`'s, else all three (a game with no recipe yet is compared with the baseline alone):
//!
//!     NETTAI_EXTRACT_ROM_DIR=~/Documents/Tango/roms NETTAI_EXTRACT_GAMES=exe6 \
//!     NETTAI_EXTRACT_BASELINE=../bn6battle-verify/data/extract-baseline/fac5b936a \
//!     cargo test --release -p nettai-extract --test recipes -- --ignored --nocapture

mod common;

use common::{full_set, games, tables};
use nettai_extract::{Extraction, RomRead, RomSet};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

/// A pack's files, by path.
type Files = BTreeMap<String, Vec<u8>>;

fn files(e: &Extraction) -> Files {
    e.files().unwrap_or_else(|err| panic!("{}: {err}", e.game)).into_iter().collect()
}

/// Where two packs' files first differ, if they do: a path one has alone, or the first file whose bytes differ
/// (for a text file, its first differing line).
fn first_difference(a: &Files, b: &Files) -> Option<String> {
    for path in a.keys().chain(b.keys()) {
        let (x, y) = (a.get(path), b.get(path));
        match (x, y) {
            (Some(_), None) => return Some(format!("{path}: only the first has it")),
            (None, Some(_)) => return Some(format!("{path}: only the second has it")),
            (Some(x), Some(y)) if x != y => {
                if let (Ok(tx), Ok(ty)) = (std::str::from_utf8(x), std::str::from_utf8(y)) {
                    let line = tx.lines().zip(ty.lines()).position(|(l, m)| l != m);
                    return Some(match line {
                        Some(n) => format!("{path}: line {}:\n  {}\n  {}", n + 1, tx.lines().nth(n).unwrap(), ty.lines().nth(n).unwrap()),
                        None => format!("{path}: {} lines against {}", tx.lines().count(), ty.lines().count()),
                    });
                }
                let at = x.iter().zip(y).position(|(p, q)| p != q).unwrap_or(x.len().min(y.len()));
                return Some(format!("{path}: byte {at:#x} of {} and {}", x.len(), y.len()));
            }
            _ => {}
        }
    }
    None
}

/// The ranges read of each ROM, merged where they meet or overlap.
fn ranges(reads: impl IntoIterator<Item = (String, u32, u32)>) -> BTreeMap<String, Vec<(u32, u32)>> {
    let mut by_rom: BTreeMap<String, Vec<(u32, u32)>> = BTreeMap::new();
    for (rom, address, len) in reads {
        by_rom.entry(rom).or_default().push((address, address + len));
    }
    for list in by_rom.values_mut() {
        list.sort();
        let mut merged: Vec<(u32, u32)> = Vec::new();
        for &(start, end) in list.iter() {
            match merged.last_mut() {
                Some(last) if start <= last.1 => last.1 = last.1.max(end),
                _ => merged.push((start, end)),
            }
        }
        *list = merged;
    }
    by_rom
}

fn read_ranges(reads: &[RomRead]) -> BTreeMap<String, Vec<(u32, u32)>> {
    ranges(reads.iter().map(|r| (r.rom.clone(), r.address, r.len)))
}

/// Where two sets of ranges first differ.
fn ranges_difference(a: &BTreeMap<String, Vec<(u32, u32)>>, b: &BTreeMap<String, Vec<(u32, u32)>>) -> Option<String> {
    for rom in a.keys().chain(b.keys()) {
        let (x, y) = (a.get(rom).cloned().unwrap_or_default(), b.get(rom).cloned().unwrap_or_default());
        if x != y {
            let at = x.iter().zip(&y).position(|(p, q)| p != q).unwrap_or(x.len().min(y.len()));
            let show = |l: &[(u32, u32)]| l.get(at).map_or("none".to_string(), |(s, e)| format!("{s:08X}..{e:08X}"));
            return Some(format!("{rom}: range {at} of {} and {}: {} against {}", x.len(), y.len(), show(&x), show(&y)));
        }
    }
    None
}

/// The baseline's digests of a game's pack (`<game>.sha256`: `digest  path` a line).
fn baseline_digests(dir: &PathBuf, game: &str) -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(dir.join(format!("{game}.sha256"))).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    text.lines().filter_map(|l| l.split_once("  ")).map(|(d, p)| (p.to_string(), d.to_string())).collect()
}

/// The baseline's reads of a game (`<game>.reads`, the CLI's `--reads`).
fn baseline_reads(dir: &PathBuf, game: &str) -> BTreeMap<String, Vec<(u32, u32)>> {
    let text = std::fs::read_to_string(dir.join(format!("{game}.reads"))).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    ranges(text.lines().map(|l| {
        let mut f = l.split('\t');
        let rom = f.next().unwrap().to_string();
        let address = u32::from_str_radix(f.next().unwrap(), 16).unwrap();
        let len = u32::from_str_radix(f.next().unwrap(), 16).unwrap();
        (rom, address, len)
    }))
}

fn digests(files: &Files) -> BTreeMap<String, String> {
    use sha2::{Digest, Sha256};
    files.iter().map(|(p, b)| (p.clone(), Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect())).collect()
}

fn digests_difference(a: &BTreeMap<String, String>, baseline: &BTreeMap<String, String>) -> Option<String> {
    for path in a.keys().chain(baseline.keys()) {
        match (a.get(path), baseline.get(path)) {
            (Some(_), None) => return Some(format!("{path}: not in the baseline")),
            (None, Some(_)) => return Some(format!("{path}: only the baseline has it")),
            (Some(x), Some(y)) if x != y => return Some(format!("{path}: another digest than the baseline's")),
            _ => {}
        }
    }
    None
}

/// An extraction of a game's full set, its time, and its reads (a second run, recorded).
fn run(what: &str, f: &dyn Fn() -> Result<Extraction, nettai_extract::Error>, roms: &RomSet) -> (Files, Vec<RomRead>, f64) {
    let t = Instant::now();
    let e = f().unwrap_or_else(|err| panic!("{what}: {err}"));
    let time = t.elapsed().as_secs_f64();
    let files = files(&e);
    let (_, reads) = nettai_extract::recording_reads(roms, f).unwrap_or_else(|err| panic!("{what}: {err}"));
    (files, reads, time)
}

#[test]
#[ignore = "requires local ROMs via NETTAI_EXTRACT_ROM_DIR"]
fn the_recipes_make_the_rust_extractions_packs() {
    let baseline = std::env::var_os("NETTAI_EXTRACT_BASELINE").map(PathBuf::from);
    let mut wrong = Vec::new();
    for game in games() {
        let tables = tables(game);
        let roms = full_set(game);
        let id = game.id();
        let luau = tables.recipe().map(|recipe| run(&format!("{id}'s recipe"), &|| recipe.extract(&roms), &roms));
        let rust = nettai_extract::has_rust_extraction(game).then(|| run(&format!("{id} in Rust"), &|| nettai_extract::extract_in_rust(&tables, &roms), &roms));
        if let Some((_, _, t)) = &luau {
            eprintln!("{id}: the recipe extracted in {t:.3} s");
        }
        if let Some((_, _, t)) = &rust {
            eprintln!("{id}: the Rust extracted in {t:.3} s");
        }
        if let (Some((lf, lr, _)), Some((rf, rr, _))) = (&luau, &rust) {
            match first_difference(lf, rf) {
                Some(d) => wrong.push(format!("{id}: the recipe's pack isn't the Rust's: {d}")),
                None => eprintln!("{id}: the recipe's pack is the Rust's, byte for byte ({} files)", lf.len()),
            }
            match ranges_difference(&read_ranges(lr), &read_ranges(rr)) {
                Some(d) => wrong.push(format!("{id}: the recipe reads otherwise than the Rust: {d}")),
                None => {
                    let same_order = lr.len() == rr.len() && lr.iter().zip(rr).all(|(a, b)| (&a.rom, a.address, a.len, a.pointer, a.open) == (&b.rom, b.address, b.len, b.pointer, b.open));
                    eprintln!("{id}: the recipe reads what the Rust reads ({} reads, {} in the same order)", lr.len(), if same_order { "all" } else { "not all" });
                }
            }
        }
        if let Some(dir) = &baseline {
            let want = baseline_digests(dir, id);
            let want_reads = baseline_reads(dir, id);
            for (what, path) in [("the recipe", &luau), ("the Rust", &rust)] {
                let Some((f, r, _)) = path else { continue };
                match digests_difference(&digests(f), &want) {
                    Some(d) => wrong.push(format!("{id}: {what}'s pack isn't the baseline's: {d}")),
                    None => eprintln!("{id}: {what}'s pack is the baseline's ({} files)", want.len()),
                }
                match ranges_difference(&read_ranges(r), &want_reads) {
                    Some(d) => wrong.push(format!("{id}: {what} reads otherwise than the baseline: {d}")),
                    None => eprintln!("{id}: {what} reads what the baseline reads"),
                }
            }
        }
        if luau.is_none() && baseline.is_none() {
            eprintln!("{id}: no recipe yet, and no baseline to compare the Rust with");
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
