//! `bn6-extract content <rom> <pack-dir>`: the battle graphics and all of
//! the game's sound as a content pack of open formats (indexed PNG, JSON,
//! Tiled maps, MIDI, TOML, WAV; see the bn6-content crate), which the
//! frontend loads directly.
//!
//! The pack holds the game's own graphics and recordings: write it outside
//! version control (data/content/ is ignored).

use m4a::SoundBank;
use std::path::Path;

pub fn main(args: &[String]) {
    let (Some(rom), Some(out)) = (args.first(), args.get(1)) else {
        eprintln!("usage: bn6-extract content <rom> <pack-dir>");
        std::process::exit(2);
    };
    let rom_bytes = crate::load_rom(rom);
    let t = std::time::Instant::now();
    let bundle = crate::graphics::bundle(&rom_bytes);
    let (bank, failures) = m4a::rom::extract(&rom_bytes.0).unwrap_or_else(|e| panic!("reading the sound data: {e}"));
    for (song, e) in &failures {
        eprintln!("song {:#05x} left out (it uses a command the driver port doesn't play): {e}", song.0);
    }
    debug_assert_eq!(SoundBank::from_bytes(&bank.to_bytes()).as_ref(), Ok(&bank));
    let mut files = vec![bn6_content::pack::manifest("BN6 (US Falzar) battle content", Some(&bundle), true)];
    files.extend(bn6_content::pack::export_graphics(&bundle));
    let (sound, left_out) = bn6_content::pack::export_sound(&bank);
    files.extend(sound);
    for (song, e) in &left_out {
        eprintln!("song {:#05x} left out (no MIDI mapping yet): {e}", song.0);
    }
    bn6_content::pack::write_files(Path::new(out), &files).unwrap_or_else(|e| panic!("writing {out}: {e}"));
    let bytes: usize = files.iter().map(|f| f.1.len()).sum();
    eprintln!(
        "wrote {out}: {} sprites, {} backgrounds, {} songs, {} samples; {} files, {} KiB in {:.1?}",
        bundle.sprites.len(),
        bundle.backgrounds.iter().flatten().count(),
        bank.songs.iter().flatten().count() - left_out.len(),
        bank.samples.len(),
        files.len(),
        bytes / 1024,
        t.elapsed()
    );
}
