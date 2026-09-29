//! `bn6-extract assets <rom> <sound-bank>`: the game's sound (songs,
//! effects, instruments and samples) as an m4a sound bank for bn6-audio.
//!
//! The bank holds the game's own recordings, so it is written to a path the
//! user chooses and never committed (data/sound/ is gitignored).

use m4a::SoundBank;

pub fn main(args: &[String]) {
    let (Some(rom), Some(out)) = (args.first(), args.get(1)) else {
        eprintln!("usage: bn6-extract assets <rom> <sound-bank>");
        std::process::exit(2);
    };
    let data = std::fs::read(rom).unwrap_or_else(|e| panic!("reading {rom}: {e}"));
    assert_eq!(&data[0xA0..0xB0], b"MEGAMAN6_FXXBR6E", "expected US Falzar");
    let (bank, failures) = m4a::rom::extract(&data).unwrap_or_else(|e| panic!("reading the sound data: {e}"));
    for (song, e) in &failures {
        eprintln!("song {:#05x} left out: {e}", song.0);
    }
    let bytes = bank.to_bytes();
    // The bank must read back as it was written.
    assert_eq!(SoundBank::from_bytes(&bytes).as_ref(), Ok(&bank), "sound bank round trip");
    if let Some(dir) = std::path::Path::new(out).parent() {
        std::fs::create_dir_all(dir).unwrap_or_else(|e| panic!("creating {}: {e}", dir.display()));
    }
    std::fs::write(out, &bytes).unwrap_or_else(|e| panic!("writing {out}: {e}"));
    let songs = bank.songs.iter().flatten().count();
    let sample_bytes: usize = bank.samples.iter().map(|s| s.data.len()).sum();
    eprintln!(
        "wrote {out}: {songs} songs, {} players, {} voicegroups, {} samples ({} KiB), {} KiB in all",
        bank.players.len(),
        bank.voicegroups.len(),
        bank.samples.len(),
        sample_bytes / 1024,
        bytes.len() / 1024
    );
}
