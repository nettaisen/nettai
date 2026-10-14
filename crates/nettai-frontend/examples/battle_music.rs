//! The battle themes, heard without listening, on real packs: for each game's battle, with each theme of every
//! game (and the game's own music), a live match is played a few seconds with the sound made, and what plays is
//! checked: the music player of the theme's own pack's driver plays the theme's song (its number in that pack's
//! song table, which its asset name gives), the battle's own pack's music player plays none (another game's
//! theme) or the theme (its own), and the samples aren't silence; and the battle is the same, tick for tick, as
//! with its own music. Then each theme is changed to while the battle runs (as Practice's menu changes it:
//! `Player::set_sound_parts`) and back to the game's own, each checked the same way. It prints a line for each.
//!
//! ```text
//! cargo run -p nettai-frontend --example battle_music
//! ```
//!
//! The packs are found as the tool finds them (`$NETTAI_PACKS`); a game whose pack isn't there is left out. It
//! fails (status 1) on a theme that doesn't play as it should.

use nettai_audio::m4a::SongId;
use nettai_audio::{BattleAudio, MUSIC_PLAYER, Theme};
use nettai_battle::content::PackId;
use nettai_frontend::Player;
use nettai_frontend::driver::LivePlayer;
use nettai_frontend::game::{Loaded, Options, GraphicsMode};

/// Ticks of each battle played: five seconds, or a change in the middle of them.
const TICKS: u32 = 300;

/// What a battle of `game` sounds like played as `parts` say, each a stretch of ticks with its music (none: the
/// game's own), the first from its start and each other changed to as it runs: each pack's music player's song
/// at the end, the last two seconds' loudness (RMS), and the battle's digest.
fn play(game: &Loaded, parts: &[(u32, Option<&Theme>)]) -> Result<(Vec<Option<SongId>>, f32, u64), String> {
    let content = game.game.content.clone();
    let sound = game.sound.as_ref().ok_or("no sound")?;
    let (banks, songs) = sound.parts(&content, parts.first().and_then(|p| p.1));
    let m = nettai_match::pick::live(&content, &game.game.name, 11, None)?;
    let set = nettai_match::Set::of(&content, &m, 11);
    let renderer = game.graphics.renderer(GraphicsMode::Original, None);
    let mut player = Player::with(renderer, None, Some(BattleAudio::with_banks(banks, songs)), Box::new(LivePlayer::new(set)));
    let mut samples = Vec::new();
    for (at, &(ticks, theme)) in parts.iter().enumerate() {
        if at > 0 {
            player.set_sound_parts(sound.parts(&content, theme));
        }
        for _ in 0..ticks {
            if !player.tick(0) {
                return Err(format!("the match stopped: {}", player.stopped().unwrap_or("?")));
            }
            player.take_samples(&mut samples);
        }
    }
    let audio = player.audio().ok_or("no audio")?;
    let playing = (0..).map_while(|p| audio.driver_of(PackId(p))).map(|d| d.player(MUSIC_PLAYER).filter(|m| m.is_playing()).and_then(|m| m.song())).collect();
    // (The last two seconds: the music, past the round's opening.)
    let tail = &samples[samples.len().saturating_sub(2 * nettai_audio::SAMPLE_RATE as usize)..];
    let rms = (tail.iter().map(|[l, r]| l * l + r * r).sum::<f32>() / (2 * tail.len().max(1)) as f32).sqrt();
    Ok((playing, rms, player.battle().digest()))
}

fn run() -> Result<bool, String> {
    let mut games = Vec::new();
    for name in ["exe4", "exe5", "exe6"] {
        match nettai_frontend::game::load(name, &Options::default()) {
            Ok(game) => games.push(game),
            Err(e) => println!("{name}: not loaded ({e})"),
        }
    }
    let mut good = true;
    let mut say = |ok: bool, line: String| {
        good &= ok;
        println!("{line} {}", if ok { "ok" } else { "WRONG" });
    };
    for game in &games {
        let content = &game.game.content;
        let own_name = &game.game.name;
        let (own, own_rms, digest) = play(game, &[(TICKS, None)])?;
        // The song the round starts as the game's own, by its pack's number.
        let own_song = content.battle_music().iter().filter_map(|h| content.assets.sound(h.0)).map(|s| SongId(s.id)).collect::<Vec<_>>();
        let ok = own.len() == 1 && own[0].is_some_and(|s| own_song.contains(&s)) && own_rms > 0.001;
        say(ok, format!("{own_name} battle, its own music: {own:?} (rms {own_rms:.4})"));
        for of in &games {
            for song in of.battle_themes() {
                let theme = of.theme(song).ok_or(format!("{}'s theme {song} has no song", of.game.name))?;
                let same_game = std::ptr::eq(of, game);
                let expected: Vec<Option<SongId>> = if same_game { vec![Some(SongId(theme.song))] } else { vec![None, Some(SongId(theme.song))] };
                let named = format!("{own_name} battle, {}:{song} (song {:#05x} of {}'s pack)", of.game.name, theme.song, of.game.name);
                let same = |d: u64| if d == digest { "the same" } else { "DIFFERS" };
                // From the start.
                let (playing, rms, d) = play(game, &[(TICKS, Some(&theme))])?;
                say(playing == expected && rms > 0.001 && d == digest, format!("{named}: music players {playing:?}, rms {rms:.4}, battle {}", same(d)));
                // Changed to as the battle runs, and back to the game's own.
                let (playing, rms, d) = play(game, &[(TICKS / 2, None), (TICKS / 2, Some(&theme))])?;
                say(playing == expected && rms > 0.001 && d == digest, format!("{named} changed to: music players {playing:?}, rms {rms:.4}, battle {}", same(d)));
                let back: Vec<Option<SongId>> = if same_game { own.clone() } else { vec![own[0], None] };
                let (playing, rms, d) = play(game, &[(TICKS / 2, Some(&theme)), (TICKS / 2, None)])?;
                say(playing == back && rms > 0.001 && d == digest, format!("{named} changed back from: music players {playing:?}, rms {rms:.4}, battle {}", same(d)));
            }
        }
    }
    Ok(good)
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(why) => {
            eprintln!("{why}");
            std::process::exit(1);
        }
    }
}
