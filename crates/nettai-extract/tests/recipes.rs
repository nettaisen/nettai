//! The recipes held to the packs the Rust extraction made before them (docs/design/extraction-recipes.md §6.5):
//! each game's full set extracted by its recipe, in memory (`Extraction::files`, no writes), its files compared with
//! a baseline's digests (`NETTAI_EXTRACT_BASELINE`: a folder of the verification workspace's
//! `data/extract-baseline/<commit>/`), `extraction.txt` among them, and its ROM reads with the baseline's, as ranges
//! per ROM after merging. A later change to a recipe, to exelib's code or to a primitive is held to them, until a
//! change meant to alter a pack records a new baseline. Each game's time is said.
//!
//! The ROMs come from a folder, recognized by their headers (`NETTAI_EXTRACT_ROM_DIR`, any file names); the games
//! are `NETTAI_EXTRACT_GAMES`'s, else every game with a recipe:
//!
//!     NETTAI_EXTRACT_ROM_DIR=~/Documents/Tango/roms NETTAI_EXTRACT_GAMES=exe6 \
//!     NETTAI_EXTRACT_BASELINE=../bn6battle-verify/data/extract-baseline/fac5b936a \
//!     cargo test --release -p nettai-extract --test recipes -- --ignored --nocapture

mod common;

use common::{full_set, games, recipe};
use nettai_extract::{Extraction, RomRead};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

/// A pack's files, by path.
type Files = BTreeMap<String, Vec<u8>>;

fn files(e: &Extraction) -> Files {
    e.files().unwrap_or_else(|err| panic!("{}: {err}", e.game)).into_iter().collect()
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

#[test]
#[ignore = "requires local ROMs via NETTAI_EXTRACT_ROM_DIR and a baseline via NETTAI_EXTRACT_BASELINE"]
fn the_recipes_make_the_baselines_packs() {
    let baseline = PathBuf::from(std::env::var_os("NETTAI_EXTRACT_BASELINE").expect("NETTAI_EXTRACT_BASELINE: a baseline's folder"));
    let mut wrong = Vec::new();
    for game in games() {
        let recipe = recipe(&game);
        let roms = full_set(&recipe);
        let t = Instant::now();
        let e = recipe.extract(&roms).unwrap_or_else(|err| panic!("{game}: {err}"));
        eprintln!("{game}: the recipe extracted in {:.3} s", t.elapsed().as_secs_f64());
        let files = files(&e);
        let (_, reads) = recipe.extract_recording_reads(&roms).unwrap_or_else(|err| panic!("{game}: {err}"));
        let want = baseline_digests(&baseline, &game);
        match digests_difference(&digests(&files), &want) {
            Some(d) => wrong.push(format!("{game}: the recipe's pack isn't the baseline's: {d}")),
            None => eprintln!("{game}: the recipe's pack is the baseline's ({} files)", want.len()),
        }
        match ranges_difference(&read_ranges(&reads), &baseline_reads(&baseline, &game)) {
            Some(d) => wrong.push(format!("{game}: the recipe reads otherwise than the baseline: {d}")),
            None => eprintln!("{game}: the recipe reads what the baseline reads"),
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
