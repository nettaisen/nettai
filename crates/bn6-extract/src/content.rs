//! `bn6-extract content <rom> <pack-dir> [--content <dir>]`: the game's
//! assets as a content pack of open formats (see the nettai-content crate):
//! the graphics (indexed PNG, JSON, Tiled maps, the sprites' animation
//! timing) and all of the game's sound (MIDI, TOML, WAV), under the names
//! the content's compat/assets.toml (and chips.toml, for the HUD's chip
//! icons) gives them, the rest under placeholders; and the pack's asset
//! index (`assets.toml`), which lists them all by those names.
//!
//! The battle content is not extracted: it is the content root's
//! definitions (this repository's content/bn6 unless `--content` names
//! another), which name these assets. The graphics and the index are read
//! back from the written pack and compared with what was extracted, and
//! the content root's definitions are defined against the pack, so a pack
//! that wouldn't load is never left behind silently.
//!
//! The pack holds the game's own data: write it outside version control
//! (data/content/ is ignored).

use nettai_content::report::{Level, Report};
use nettai_content_api::{AssetKind, AssetNames};
use std::path::Path;

pub fn main(args: &[String]) {
    let usage = "usage: bn6-extract content <rom> <pack-dir> [--gregar <rom>] [--content <dir>]";
    let (Some(rom), Some(out)) = (args.first(), args.get(1)) else {
        eprintln!("{usage}");
        std::process::exit(2);
    };
    let mut content_dir = nettai_content::root::bn6();
    let mut gregar = None;
    let mut rest = args[2..].iter();
    while let Some(flag) = rest.next() {
        match (flag.as_str(), rest.next()) {
            ("--content", Some(dir)) => content_dir = dir.into(),
            ("--gregar", Some(path)) => gregar = Some(crate::load_gregar_rom(path)),
            _ => {
                eprintln!("{usage}");
                std::process::exit(2);
            }
        }
    }
    if gregar.is_none() {
        eprintln!("no Gregar ROM (--gregar): a Gregar console's own pictures (its Crosses' names, its Beast) are Falzar's");
    }
    let names = asset_names(content_dir.join("compat").as_path());
    let rom_bytes = crate::load_rom(rom);
    let t = std::time::Instant::now();
    let bundle = crate::graphics::bundle(&rom_bytes, gregar.as_ref(), &names);
    let (bank, failures) = m4a::rom::extract(&rom_bytes.0).unwrap_or_else(|e| panic!("reading the sound data: {e}"));
    for (song, e) in &failures {
        eprintln!("song {:#05x} left out (it uses a command the driver port doesn't play): {e}", song.0);
    }
    let mut files = vec![nettai_content::pack::manifest("BN6 (US Falzar) battle assets", Some(&bundle), true)];
    files.extend(nettai_content::pack::export_graphics(&bundle, &names));
    let (sound, left_out) = nettai_content::pack::export_sound(&bank, &names);
    files.extend(sound);
    // The song-table entries: the songs the bank plays and those it left
    // out.
    let songs = bank.songs.iter().enumerate().filter(|(_, s)| s.is_some()).map(|(i, _)| i as u16);
    let songs = songs.chain(failures.iter().map(|(id, _)| id.0)).collect();
    let index = names.index(&bundle, &songs);
    files.push(nettai_content::names::index_file(&index));
    for (song, e) in &left_out {
        eprintln!("song {:#05x} left out (no MIDI mapping yet): {e}", song.0);
    }
    let root = Path::new(out);
    nettai_content::pack::write_files(root, &files).unwrap_or_else(|e| panic!("writing {out}: {e}"));
    // The asset index must read back exactly.
    let mut report = Report::default();
    match nettai_content::names::read_index(root, &mut report) {
        Some(back) if back == index => {}
        Some(_) => panic!("the pack's asset index reads back differently"),
        None => panic!("the pack's asset index doesn't load:\n{report}"),
    }
    // So must the graphics, under whatever names they were written.
    let mut report = Report::default();
    match nettai_content::pack::import_graphics(root, &mut report) {
        Some(back) if back == bundle => {}
        Some(_) => panic!("the pack's graphics read back differently"),
        None => panic!("the pack's graphics don't load:\n{report}"),
    }
    // And the content's definitions must define against it.
    let content = match nettai_content::pack::load_battle(&content_dir, root) {
        Ok((c, r)) => {
            for i in r.issues.iter().filter(|i| i.level != Level::Note) {
                eprintln!("{i}");
            }
            c
        }
        Err(r) => panic!("the content {} doesn't load with the pack:\n{r}", content_dir.display()),
    };
    let bytes: usize = files.iter().map(|f| f.1.len()).sum();
    eprintln!(
        "wrote {out}: {} sprites, {} backgrounds, {} songs, {} samples, {} named assets; {} files, {} KiB in {:.1?} (with {}: {} chips, content {})",
        bundle.sprites.len(),
        bundle.backgrounds.iter().flatten().count(),
        bank.songs.iter().flatten().count() - left_out.len(),
        bank.samples.len(),
        AssetKind::ALL.iter().map(|&k| index.names(k).iter().filter(|n| !AssetNames::is_placeholder(k, n)).count()).sum::<usize>(),
        files.len(),
        bytes / 1024,
        t.elapsed(),
        content_dir.display(),
        content.defs.chips.len(),
        content.hash()
    );
}

/// The names compat gives the assets (placeholders for all of them
/// without one).
fn asset_names(compat: &Path) -> nettai_content::names::AssetNames {
    let mut names = nettai_content::names::AssetNames::default();
    if !compat.is_dir() {
        eprintln!("no compat at {}: every asset is written under its placeholder", compat.display());
        return names;
    }
    let c = bn6_compat::Compat::read(compat).unwrap_or_else(|e| panic!("{e}"));
    for (name, id) in &c.assets.sprites {
        let parse = |s: &str| u8::from_str_radix(s, 16).ok();
        let Some((cat, index)) = id.split_once('-').and_then(|(a, b)| Some((parse(a)?, parse(b)?))) else {
            panic!("assets.toml: sprite {name} is {id:?}, not \"cc-ii\"");
        };
        names.sprites.insert((cat, index), name.clone());
    }
    names.songs = c.assets.sounds.iter().map(|(k, &v)| (v, k.clone())).collect();
    names.backgrounds = c.assets.backgrounds.iter().map(|(k, &v)| (v, k.clone())).collect();
    names.mugshots = c.assets.mugshots.iter().map(|(k, &v)| (v, k.clone())).collect();
    names.banners = c.assets.banners.iter().map(|(k, &v)| (v, k.clone())).collect();
    names.chips = c.chips.iter().map(|(k, e)| (e.id, k.clone())).collect();
    names.glyphs = c.text.glyphs.clone();
    names
}
