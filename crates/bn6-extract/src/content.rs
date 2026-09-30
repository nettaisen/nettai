//! `bn6-extract content <rom> <pack-dir> [--overlay <dir>]`: the game's
//! battle content as a content pack of open formats (see the bn6-content
//! crate): the battle data (TOML, by owner), the graphics (indexed PNG,
//! JSON, Tiled maps) and all of the game's sound (MIDI, TOML, WAV), with
//! the hand-written scripts that implement BN6's content: the source
//! overlay (`bn6_content::overlay`), this repository's content/bn6 unless
//! `--overlay` names another.
//!
//! The battle data is read back from the written pack and compared with
//! what was extracted (and the overlay added), so a pack that wouldn't load
//! as the same content is never left behind silently.
//!
//! The pack holds the game's own data: write it outside version control
//! (data/content/ is ignored).

use bn6_content::report::{Level, Report};
use std::path::Path;

/// The source overlay in this repository (content/bn6).
const OVERLAY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/bn6");

pub fn main(args: &[String]) {
    let usage = "usage: bn6-extract content <rom> <pack-dir> [--overlay <dir>]";
    let (Some(rom), Some(out)) = (args.first(), args.get(1)) else {
        eprintln!("{usage}");
        std::process::exit(2);
    };
    let overlay_dir = match &args[2..] {
        [] => OVERLAY.to_string(),
        [flag, dir] if flag == "--overlay" => dir.clone(),
        _ => {
            eprintln!("{usage}");
            std::process::exit(2);
        }
    };
    let rom_bytes = crate::load_rom(rom);
    let t = std::time::Instant::now();
    let bundle = crate::graphics::bundle(&rom_bytes);
    let mut battle = crate::battle::content(&rom_bytes);
    check_timing(&battle, &bundle);
    let mut report = Report::default();
    let overlay = bn6_content::overlay::read(Path::new(&overlay_dir), &mut report);
    let overlay = overlay.unwrap_or_else(|| panic!("the overlay {overlay_dir} doesn't read:\n{report}"));
    overlay.apply(&mut battle, &mut report);
    if report.has_errors() {
        panic!("the overlay {overlay_dir} doesn't apply:\n{report}");
    }
    let (bank, failures) = m4a::rom::extract(&rom_bytes.0).unwrap_or_else(|e| panic!("reading the sound data: {e}"));
    for (song, e) in &failures {
        eprintln!("song {:#05x} left out (it uses a command the driver port doesn't play): {e}", song.0);
    }
    let mut files = vec![bn6_content::pack::manifest("BN6 (US Falzar) battle content", Some(&bundle), true, true)];
    files.extend(bn6_content::battle::export(&battle));
    files.extend(overlay.files().iter().cloned());
    files.extend(bn6_content::pack::export_graphics(&bundle));
    let (sound, left_out) = bn6_content::pack::export_sound(&bank);
    files.extend(sound);
    for (song, e) in &left_out {
        eprintln!("song {:#05x} left out (no MIDI mapping yet): {e}", song.0);
    }
    let root = Path::new(out);
    bn6_content::pack::write_files(root, &files).unwrap_or_else(|e| panic!("writing {out}: {e}"));
    // The battle data must read back exactly.
    let mut report = Report::default();
    let back = bn6_content::battle::load(root, &mut report);
    for i in report.issues.iter().filter(|i| i.level != Level::Note) {
        eprintln!("{i}");
    }
    match back {
        Some(back) if back == battle => {}
        Some(back) => panic!("the pack's battle data reads back differently: {:?}", bn6_content::verify::compare_battle(&battle, &back)),
        None => panic!("the pack's battle data doesn't load"),
    }
    let bytes: usize = files.iter().map(|f| f.1.len()).sum();
    eprintln!(
        "wrote {out}: {} chips, {} navis, {} forms, {} scripts, {} sprites, {} backgrounds, {} songs, {} samples; {} files, {} KiB in {:.1?} (content {})",
        battle.chips.len(),
        battle.navis.len(),
        battle.forms.len(),
        battle.scripts.modules.len(),
        bundle.sprites.len(),
        bundle.backgrounds.iter().flatten().count(),
        bank.songs.iter().flatten().count() - left_out.len(),
        bank.samples.len(),
        files.len(),
        bytes / 1024,
        t.elapsed(),
        battle.hash()
    );
}

/// The engine's sprite timing and the graphics' animations are the same
/// data read twice; they must agree.
fn check_timing(battle: &bn6_battle::Content, bundle: &bn6_assets::Bundle) {
    let from_graphics: std::collections::BTreeMap<_, _> = bundle
        .sprites
        .iter()
        .map(|s| {
            let anims: Vec<Vec<bn6_battle::content::AnimFrame>> = s
                .animations
                .iter()
                .map(|a| a.iter().map(|f| bn6_battle::content::AnimFrame { duration: f.duration, flags: f.flags }).collect())
                .collect();
            (bn6_battle::content::SpriteId { category: s.category, index: s.index }, anims)
        })
        .collect();
    assert!(from_graphics == battle.animations.sprites, "the sprites' animation timing differs from the graphics' animations");
}

