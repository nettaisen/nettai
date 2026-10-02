//! `bn5-extract content <protoman-us> <colonel-us> <protoman-jp> <colonel-jp> <pack-dir>`: BN5's battle
//! assets as a content pack of open formats (nettai-content): the graphics
//! (indexed PNG, JSON, Tiled maps, the sprites' animation timing), all of
//! the game's sound (MIDI, TOML, WAV), and the asset index (`assets.toml`).
//! The graphics and the index are read back from the written pack and
//! compared with what was extracted.

use crate::rom::{self, Version};
use nettai_content::report::Report;
use std::path::Path;

pub const USAGE: &str = "usage: bn5-extract content <protoman-us> <colonel-us> <protoman-jp> <colonel-jp> <pack-dir>\n\
     (all four ROMs, in this order: the US Team ProtoMan ROM, BRBE; the US Team Colonel ROM, BRKE;\n\
     the Japanese Team of Blues ROM, BRBJ; the Japanese Team of Colonel ROM, BRKJ)";

/// The names BN5's compat (content/bn5/compat, built into bn5-compat) gives
/// the assets: sprites, songs and banners by BN6's names for what is BN6's,
/// chip icons by the chips' keys.
fn asset_names() -> nettai_content::names::AssetNames {
    let c = bn5_compat::Compat::bn5();
    nettai_content::names::AssetNames {
        sprites: c.sprite_names(),
        songs: c.assets.sounds.iter().map(|(k, &v)| (v, k.clone())).collect(),
        banners: c.assets.banners.iter().map(|(k, &v)| (v, k.clone())).collect(),
        backgrounds: c.assets.backgrounds.iter().map(|(k, &v)| (v, k.clone())).collect(),
        chips: c.chip_keys.clone(),
        ..Default::default()
    }
}

/// The game a BN5 pack is (its manifest's `game`): the namespace its names
/// take when it loads beside another game's pack.
pub const GAME: &str = "bn5";

pub fn main(args: &[String]) {
    let [protoman, colonel, protoman_jp, colonel_jp, out] = args else {
        let missing = ["the US Team ProtoMan ROM", "the US Team Colonel ROM", "the Japanese Team of Blues ROM", "the Japanese Team of Colonel ROM", "the pack's directory"];
        if args.len() < 5 {
            eprintln!("missing: {} (given {} of the five)\n{USAGE}", missing[args.len()..].join(", "), args.len());
        } else {
            eprintln!("too many arguments ({}; the four ROMs, then the pack's directory)\n{USAGE}", args.len());
        }
        std::process::exit(2);
    };
    let roms = rom::load([protoman.as_str(), colonel.as_str(), protoman_jp.as_str(), colonel_jp.as_str()]).unwrap_or_else(|e| {
        eprintln!("{e}\n{USAGE}");
        std::process::exit(2);
    });
    let t = std::time::Instant::now();
    // The names content/bn5/compat gives the assets (BN6's for what is BN6's);
    // the rest under their placeholders.
    let names = asset_names();
    let bundle = crate::graphics::bundle(&roms, &names);
    let versioned = crate::graphics::versioned_chips(&roms);
    let (mut bank, failures) = m4a::rom::extract(&roms.protoman.0).unwrap_or_else(|e| panic!("reading the sound data: {e}"));
    for (song, e) in &failures {
        eprintln!("song {:#05x} left out (it uses a command the driver port doesn't play): {e}", song.0);
    }
    // Team Colonel's own songs, with what they play with added to the bank.
    let (colonel, _) = m4a::rom::extract(&roms.us(Version::Colonel).0).unwrap_or_else(|e| panic!("reading Team Colonel's sound: {e}"));
    let versions = crate::sound::colonel_songs(&mut bank, &colonel);
    let jp = crate::graphics::japanese_differences(&roms);
    if !jp.is_empty() {
        let ids: Vec<String> = jp.iter().map(|(c, i)| format!("{c:02x}-{i:02x}")).collect();
        eprintln!("note: the Japanese ROMs draw sprites {} otherwise; the pack has the US's", ids.join(", "));
    }

    let mut files = vec![manifest(&bundle)];
    files.extend(nettai_content::pack::export_graphics(&bundle, &names));
    let (sound, left_out) = nettai_content::pack::export_sound_versions(&bank, &versions, &names);
    files.extend(sound);
    for (song, e) in &left_out {
        eprintln!("song {:#05x} left out (no MIDI mapping yet): {e}", song.0);
    }
    let songs = bank.songs.iter().enumerate().filter(|(_, s)| s.is_some()).map(|(i, _)| i as u16);
    let songs = songs.chain(failures.iter().map(|(id, _)| id.0)).collect();
    let index = names.index(&bundle, &songs);
    files.push(nettai_content::names::index_file(&index));
    let root = Path::new(out);
    nettai_content::pack::write_files(root, &files).unwrap_or_else(|e| panic!("writing {out}: {e}"));
    // The asset index and the graphics must read back exactly.
    let mut report = Report::default();
    match nettai_content::names::read_index(root, &mut report) {
        Some(back) if back == index => {}
        Some(_) => panic!("the pack's asset index reads back differently"),
        None => panic!("the pack's asset index doesn't load:\n{report}"),
    }
    let mut report = Report::default();
    match nettai_content::pack::import_graphics(root, &mut report) {
        Some(back) if back == bundle => {}
        Some(back) => panic!("the pack's graphics read back differently: {}", difference(&bundle, &back)),
        None => panic!("the pack's graphics don't load:\n{report}"),
    }
    // So must the sound's versions (the songs as the driver plays them: a MIDI
    // round trip keeps the timing, not the command bytes).
    let mut report = Report::default();
    match nettai_content::pack::import_sound_versions(root, &mut report) {
        Some((back, back_versions)) => {
            let ids = |v: &nettai_content::sound::SongVersions| -> Vec<(String, Vec<u16>)> {
                v.versions.iter().map(|(n, s)| (n.clone(), s.keys().copied().collect())).collect()
            };
            assert_eq!(ids(&back_versions), ids(&versions), "the pack's versions' songs read back differently");
            assert_eq!(back_versions.base_version, versions.base_version);
            assert_eq!(back.voicegroups.len(), bank.voicegroups.len(), "the pack's voicegroups read back differently");
        }
        None => panic!("the pack's sound doesn't load:\n{report}"),
    }
    let colonel_own: Vec<String> = versions.versions.iter().flat_map(|(_, s)| s.keys().map(|id| format!("{id:#05x}"))).collect();
    eprintln!("Team Colonel's own songs: {}", colonel_own.join(", "));
    let bytes: usize = files.iter().map(|f| f.1.len()).sum();
    eprintln!(
        "wrote {out} (game {GAME}): {} sprites, {} backgrounds, {} songs, {} samples, {} chips' pictures ({} each version's own), {} files, {} KiB in {:.1?}",
        bundle.sprites.len(),
        bundle.backgrounds.iter().flatten().count(),
        bank.songs.iter().flatten().count() - left_out.len(),
        bank.samples.len(),
        crate::graphics::CHIP_COUNT,
        versioned.len(),
        files.len(),
        bytes / 1024,
        t.elapsed(),
    );
}

/// The pack's manifest: nettai-content's, with the game it is. (The loader
/// reads `game` once packs of several games load together; until then it is
/// a line the manifest's reader passes over.)
fn manifest(bundle: &nettai_assets::Bundle) -> (String, Vec<u8>) {
    let (path, bytes) = nettai_content::pack::manifest("BN5 (US Team ProtoMan and Team Colonel) battle assets", Some(bundle), true);
    let text = String::from_utf8(bytes).expect("the manifest is text");
    let mut out = String::new();
    let mut placed = false;
    for line in text.lines() {
        out.push_str(line);
        out.push('\n');
        if !placed && line.starts_with("name = ") {
            out.push_str(&format!("game = \"{GAME}\"\n"));
            placed = true;
        }
    }
    assert!(placed, "the manifest has a name");
    (path, out.into_bytes())
}

/// Where two graphics bundles differ, roughly.
fn difference(a: &nettai_assets::Bundle, b: &nettai_assets::Bundle) -> String {
    if a.sprites.len() != b.sprites.len() {
        return format!("{} sprites written, {} read", a.sprites.len(), b.sprites.len());
    }
    for (x, y) in a.sprites.iter().zip(&b.sprites) {
        if x != y {
            return format!("sprite {:02x}-{:02x}", x.category, x.index);
        }
    }
    if a.custom != b.custom {
        for (x, y) in a.custom.chip_art.iter().zip(&b.custom.chip_art) {
            if x != y {
                let what = if x.picture.tiles != y.picture.tiles { "tiles" } else if x.picture.palette != y.picture.palette { "palette" } else { "version or region" };
                return format!("chip {}'s picture ({what}: {:?} against {:?})", x.key, x.picture.palette, y.picture.palette);
            }
        }
        return "the custom screen's".into();
    }
    if a.hud != b.hud {
        return "the HUD's".into();
    }
    if a.field != b.field {
        return "the field's".into();
    }
    "the backgrounds'".into()
}
