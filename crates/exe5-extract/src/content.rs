//! `exe5-extract content <protoman-us> <colonel-us> <protoman-jp> <colonel-jp> <pack-dir>`: EXE5's battle
//! assets as a content pack of open formats (nettai-content): the graphics
//! (indexed PNG, JSON, Tiled maps, the sprites' animation timing), all of
//! the game's sound (MIDI, TOML, WAV), and the asset index (`assets.toml`).
//! The graphics and the index are read back from the written pack and
//! compared with what was extracted.

use crate::rom::{self, Version};
use nettai_content::report::Report;
use std::path::Path;

pub const USAGE: &str = "usage: exe5-extract content <protoman-us> <colonel-us> <protoman-jp> <colonel-jp> <pack-dir>\n\
     (all four ROMs, in this order: the US Team ProtoMan ROM, BRBE; the US Team Colonel ROM, BRKE;\n\
     the Japanese Team of Blues ROM, BRBJ; the Japanese Team of Colonel ROM, BRKJ)";

/// The names EXE5's compat (content/exe5/compat, built into exe5-compat) gives
/// the assets: sprites, songs and banners by EXE6's names for what is EXE6's,
/// the faces by EXE5's (its [mugshots]), chip icons by the chips' ids
/// (`cannon`); and what the fonts' glyphs draw (its text.toml).
fn asset_names() -> nettai_content::names::AssetNames {
    let c = exe5_compat::Compat::exe5();
    nettai_content::names::AssetNames {
        sprites: c.sprite_names(),
        songs: c.assets.sounds.iter().map(|(k, &v)| (v, k.clone())).collect(),
        banners: c.assets.banners.iter().map(|(k, &v)| (v, k.clone())).collect(),
        backgrounds: c.assets.backgrounds.iter().map(|(k, &v)| (v, k.clone())).collect(),
        mugshots: c.assets.mugshots.iter().map(|(k, &v)| (v, k.clone())).collect(),
        chips: c.chip_keys.iter().map(|(&id, k)| (id, nettai_content_api::keys::local(k).to_string())).collect(),
        glyphs: c.text.glyphs.clone(),
        dialogue_glyphs: c.text.dialogue_glyphs.clone(),
        // The Japanese ROMs' encoding is the pack's Japanese lettering's.
        language_glyphs: [(crate::lettering::LANGUAGE.to_string(), (c.text.jp.glyphs.clone(), c.text.jp.dialogue_glyphs.clone()))].into(),
        ..Default::default()
    }
}

/// The game an EXE5 pack is (its manifest's `game`).
pub const GAME: &str = "exe5";

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
    // The names content/exe5/compat gives the assets (EXE6's for what is EXE6's);
    // the rest under their placeholders.
    let names = asset_names();
    let bundle = crate::graphics::bundle(&roms, &names);
    let version_chips = crate::graphics::version_chips(&roms);
    let (mut bank, failures) = m4a::rom::extract(&roms.protoman.0).unwrap_or_else(|e| panic!("reading the sound data: {e}"));
    for (song, e) in &failures {
        eprintln!("song {:#05x} left out (it uses a command the driver port doesn't play): {e}", song.0);
    }
    // Team Colonel's own songs, if any, with what they play with added to the bank.
    let (colonel, _) = m4a::rom::extract(&roms.us(Version::Colonel).0).unwrap_or_else(|e| panic!("reading Team Colonel's sound: {e}"));
    let versions = crate::sound::colonel_songs(&mut bank, &colonel);
    let jp = crate::graphics::japanese_differences(&roms);
    if !jp.is_empty() {
        let ids: Vec<String> = jp.iter().map(|(c, i)| format!("{c:02x}-{i:02x}")).collect();
        eprintln!("note: the Japanese ROMs draw sprites {} otherwise (no content draws them); the pack has the US's", ids.join(", "));
    }
    let jp = crate::graphics::japanese_backgrounds(&roms);
    if !jp.is_empty() {
        let ids: Vec<String> = jp.iter().map(|&id| format!("{id:#04x} ({})", names.background(id))).collect();
        eprintln!("note: the Japanese ROMs have another background {} (no netbattle shows it); the pack has the US's", ids.join(", "));
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
    if colonel_own.is_empty() {
        eprintln!("Team Colonel's ROM plays every song as Team ProtoMan's does: no version's own");
    } else {
        eprintln!("Team Colonel's own songs: {}", colonel_own.join(", "));
    }
    let bytes: usize = files.iter().map(|f| f.1.len()).sum();
    eprintln!(
        "wrote {out} (game {GAME}): {} sprites, {} backgrounds, {} songs, {} samples, {} chips' pictures ({} version chips' from their own version's ROM), {} files, {} KiB in {:.1?}",
        bundle.sprites.len(),
        bundle.backgrounds.iter().flatten().count(),
        bank.songs.iter().flatten().count() - left_out.len(),
        bank.samples.len(),
        bundle.custom.chip_art.len(),
        version_chips.len(),
        files.len(),
        bytes / 1024,
        t.elapsed(),
    );
}

/// The pack's manifest: nettai-content's, with the game it is.
fn manifest(bundle: &nettai_assets::Bundle) -> (String, Vec<u8>) {
    nettai_content::pack::manifest("EXE5 (US Team ProtoMan and Team Colonel) battle assets", GAME, Some(bundle), true)
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
        let (x, y) = (&a.hud, &b.hud);
        let fields = [
            ("tiles", x.tiles == y.tiles),
            ("gauge", x.gauge_tiles == y.gauge_tiles),
            ("palettes", x.hp_palettes == y.hp_palettes && x.gauge_palette == y.gauge_palette && x.icon_palette == y.icon_palette),
            ("maps", x.hp_box == y.hp_box && x.gauge_frame == y.gauge_frame),
            ("font", x.font == y.font && x.font_chars == y.font_chars),
            ("enemy digits", x.enemy_digits == y.enemy_digits && x.enemy_palette == y.enemy_palette),
            ("chip icons", x.chip_icons == y.chip_icons && x.hidden_icon == y.hidden_icon),
            ("mugshots", x.mugshots == y.mugshots),
            ("counts", x.counts == y.counts && x.count_box == y.count_box),
            ("mugshot boxes", x.mugshot_boxes == y.mugshot_boxes),
            ("navi mugshots", x.navi_mugshots == y.navi_mugshots && x.navi_box == y.navi_box),
            ("pause", x.pause == y.pause),
            ("texts", x.texts == y.texts),
            ("banners", x.banners == y.banners && x.banner_digits == y.banner_digits && x.banner_palette == y.banner_palette),
            ("waiting", x.waiting == y.waiting && x.waiting_palette == y.waiting_palette),
            ("warning", x.warning == y.warning && x.warning_palette == y.warning_palette),
            ("dialogue font", x.dialogue_font == y.dialogue_font),
            ("chatbox", x.chatbox == y.chatbox),
        ];
        let differ: Vec<&str> = fields.iter().filter(|(_, same)| !same).map(|(n, _)| *n).collect();
        return format!("the HUD's ({})", differ.join(", "));
    }
    if a.field != b.field {
        return "the field's".into();
    }
    "the backgrounds'".into()
}
