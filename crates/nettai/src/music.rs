//! The battle's music (docs/app.md §1, "The battle's music"): the game's own, or a battle theme the player picked,
//! of any game whose pack is there. The user: "you should also allow for selectable battle themes (bn4 virus
//! battle, bn4 boss battle, bn4 tournament battle, bn4 vs duo, bn5 virus battle, bn5 boss battle, bn5 vs nebula
//! gray, bn6 virus battle, bn6 boss battle, bn6 vs cybeast, bn6 crossover battle)", picked "beside the arena, each
//! their own".
//!
//! What the themes are is each game's content: its rules' `battle_themes`, songs of its pack, and their names in
//! its locales (`battle_themes`, by the song's asset name); this names no song and no game. A theme is offered
//! once its game's parts are read (its content and its sound, `Games::read`: no runtime is needed), whichever
//! game is selected. The pick is one, the player's own, Practice's for one visit (`practice::Visit`: the user,
//! "going into practice should reset all the arena settings"): its entry and its menu show and change that one,
//! and its own sessions play it; a match, a replay and its video play the game's own. It is presentation: the
//! battle, its cues, a replay and its rollback are the same whatever plays (`nettai_audio::with_theme`).
//!
//! Practice's entry plays the music its session will start ([`preview`]: the user, "the practice mode menu
//! should also play the battle music and when it transitions it should play through the transition and not
//! restart"), on the battle's own sound, which the session's battle then takes over (`Sound::take_preview`,
//! `Player::carry_sound`): the song goes on through the cards into the battle. Back at the entry (EXIT, or the
//! battle stopped), the entry takes the battle's sound over the other way ([`preview_after`]: the user, "also
//! exiting practice mode back into the practice menu should continue the music"), its song going on where it is.

use crate::games::{Games, Ready};
use crate::shell::Shell;
use crate::sound::PreviewSong;
use nettai_battle::sound::SoundId;
use nettai_frontend::preload::SoundParts;
use slint::ComponentHandle;
use std::cell::RefCell;
use std::collections::HashMap;

/// A battle's music.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Music {
    /// The battle's game's own: what it plays without a pick.
    #[default]
    GameOwn,
    /// A battle theme: game `game`'s theme of song `song` (its asset name).
    Theme { game: String, song: String },
}

/// What a list of the battle's music offers: the game's own first, then the themes of each game whose parts
/// are read (`Games::read`), the games in the app's order and each game's in its own. A game whose pack isn't
/// there offers none.
pub fn offered(games: &Games) -> Vec<Music> {
    let themes = games.list.iter().filter_map(|g| games.read(&g.id).map(|ready| (g.id.clone(), ready)));
    std::iter::once(Music::GameOwn)
        .chain(themes.flat_map(|(game, ready)| {
            ready.loaded.battle_themes().into_iter().map(move |song| Music::Theme { game: game.clone(), song: song.to_string() }).collect::<Vec<_>>()
        }))
        .collect()
}

/// Where `music` is in `offered`: a pick not offered (its game not read yet) shows as the game's own, the
/// first.
pub fn place(offered: &[Music], music: &Music) -> usize {
    offered.iter().position(|m| m == music).unwrap_or(0)
}

thread_local! {
    /// The themes' names in the language shown, by game and song: [`name_themes`]'.
    static NAMES: RefCell<HashMap<(String, String), String>> = RefCell::new(HashMap::new());
}

/// Read the battle themes' names of `games` in `lang`, for [`name`]: from the content's locales alone, at
/// start-up and when the language changes (`Shell::read_game_names`).
pub fn name_themes<'a>(games: impl IntoIterator<Item = &'a str>, lang: &str) {
    let dir = nettai_content::index::content();
    let mut names = HashMap::new();
    for game in games {
        for (song, name) in nettai_content::locale::text_table(&dir, game, lang, nettai_content::locale::BATTLE_THEMES) {
            names.insert((game.to_string(), song), name);
        }
    }
    NAMES.with(|n| *n.borrow_mut() = names);
}

/// A theme's name as a list shows it, in the language shown: its game's name, then the theme's ("BN4 Tournament
/// Battle", "EXE4 トーナメントバトル"); a theme its locales don't name is its song's asset name. The game's own
/// is `own`, the UI's words for it.
pub fn name(music: &Music, own: &str) -> String {
    match music {
        Music::GameOwn => own.to_string(),
        Music::Theme { game, song } => {
            let theme = NAMES.with(|n| n.borrow().get(&(game.clone(), song.clone())).cloned()).unwrap_or_else(|| song.clone());
            format!("{} {theme}", Shell::game_name(game))
        }
    }
}

// ---- A screen's Music row (Practice's entry and its menu) ---------------------------------------------------

/// What the select's list offers, and the place of the pick in it.
fn list(shell: &Shell) -> (Vec<Music>, usize) {
    let offered = offered(&shell.games);
    let at = place(&offered, &shell.battle_music());
    (offered, at)
}

/// The UI's words for the game's own music: the list's first choice.
fn own_words(shell: &Shell) -> String {
    shell.ui().global::<nettai_ui::Shell>().get_game_own_music().to_string()
}

/// The pick as a select shows it (one not offered now shows as the game's own, which it plays as).
pub fn shown(shell: &Shell) -> String {
    let (offered, at) = list(shell);
    name(&offered[at], &own_words(shell))
}

/// Open the select's list under its control `id`, `index` (the screen hears the choice as `choose`).
pub fn open(shell: &mut Shell, id: &str, index: i32) {
    let (offered, at) = list(shell);
    let own = own_words(shell);
    let names = offered.iter().map(|m| name(m, &own)).collect();
    shell.pick(id, index, names, at);
}

/// Step the pick through the list by `by`, around from its end.
pub fn step(shell: &mut Shell, by: i32) {
    let (offered, at) = list(shell);
    let next = (at as i32 + by).rem_euclid(offered.len() as i32) as usize;
    shell.set_battle_music(&offered[next]);
}

/// The list's choice `choice` is the pick.
pub fn choose(shell: &mut Shell, choice: i32) {
    let (offered, _) = list(shell);
    if let Some(music) = offered.get(choice.max(0) as usize) {
        shell.set_battle_music(music);
    }
}

/// Have the menus play, in place of the title music, the music a battle of the selected game with the pick
/// starts as it opens: `song`, the round's music (`BattleSettings::music`), as the pick plays it (the game's
/// own, or a theme: [`sound`]'s parts). None: the title music. (Practice's entry: the user, "the practice mode
/// menu should also play the battle music".) Crossfading as it changes (`Sound::set_preview`); the battle that
/// starts with the same music takes it over (`Shell::carried_music`).
pub fn preview(shell: &mut Shell, song: Option<SoundId>) {
    preview_after(shell, song, None);
}

/// [`preview`], as a session's battle ends back at the entry (EXIT, or the battle stopped): `battle` is its sound
/// (`Player::take_sound`), and where its music is the song the entry plays, still playing, the entry plays on
/// from there with no restart and no fade (`Sound::set_preview_after`: the user, "also exiting practice mode
/// back into the practice menu should continue the music"). The battle plays the pick, as the entry does (the
/// practice menu's Music changes the battle's: `retheme`). Its music stopped (a round's end) or another (the
/// result's), the entry's starts from its beginning.
pub fn preview_after(shell: &mut Shell, song: Option<SoundId>, battle: Option<nettai_audio::BattleAudio>) {
    let preview = song.and_then(|song| {
        let (game, ready) = shell.ready()?;
        let music = shell.battle_music();
        let parts = sound(&shell.games, &ready, &music)?;
        Some(PreviewSong { key: key(&game, &music, song), parts, music: song })
    });
    shell.sound.set_preview_after(preview, battle);
}

/// What a battle's music is told by, a preview's and the battle's that takes it over: the battle's game, the
/// sound its round starts as its music, and the music picked.
pub fn key(game: &str, music: &Music, song: SoundId) -> String {
    match music {
        Music::GameOwn => format!("{game}/{}", song.0),
        Music::Theme { game: of, song: theme } => format!("{game}/{}/{of}:{theme}", song.0),
    }
}

/// Play the pick in the battle the shell runs from now on, in place of the music it had (the user: "also you
/// should be able to change the bgm in the practice menu"): its music, if the battle's own is playing, starts
/// again at once as the new song (`Player::set_sound_parts`); the result's music, or a round's end, stays, and
/// the next round starts with it. Presentation alone, as the pick is.
pub fn retheme(shell: &mut Shell) {
    let Some(game) = shell.battle.as_ref().map(|b| b.game.clone()) else { return };
    let Some(ready) = shell.games.read(&game) else { return };
    let Some(parts) = sound(&shell.games, &ready, &shell.battle_music()) else { return };
    if let Some(b) = &mut shell.battle {
        b.stage.player.set_sound_parts(parts);
    }
}

/// The sound of a battle of game `ready` with `music` (as the battle's preload takes it): its theme's song,
/// with its own game's bank, in place of the game's own music where the theme's game is read and has it; else
/// the game's own. None: the game has no sound loaded.
pub fn sound(games: &Games, ready: &Ready, music: &Music) -> Option<SoundParts> {
    let sound = ready.loaded.sound.as_ref()?;
    let theme = match music {
        Music::GameOwn => None,
        Music::Theme { game, song } => games.read(game).and_then(|of| of.loaded.theme(song)),
    };
    Some(sound.parts(ready.content(), theme.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pick no longer offered shows as the game's own.
    #[test]
    fn a_pick_not_offered_is_the_games_own() {
        let offered = [Music::GameOwn, Music::Theme { game: "exe6".into(), song: "boss-battle".into() }];
        assert_eq!(place(&offered, &offered[1]), 1);
        assert_eq!(place(&offered, &Music::Theme { game: "exe4".into(), song: "virus-battle".into() }), 0);
    }

    use nettai_battle::Content;
    use nettai_frontend::game::{Found, Game, Loaded, Sound, TextMode};
    use std::sync::Arc;

    /// Game `content` as it is read (content/'s, its assets made up): its sound a bank with a song at each of
    /// its sounds' numbers (the library's testing bank's songs), its battle's music and its themes held notes on
    /// the music player, every other a short note on another.
    fn loaded(content: Arc<Content>) -> Loaded {
        let mut bank = (*nettai_frontend::testing::bank(&content)).clone();
        // (Sound 0's song is a held note on the music player, sound 1's a short one on player 1.)
        let (held, short) = (bank.songs[0].clone(), bank.songs[1].clone());
        let music: Vec<u16> = content.battle_music().into_iter().chain(content.battle_themes().iter().copied()).map(|m| m.0).collect();
        let songs = nettai_audio::Songs::of(&content.assets);
        let numbers = songs.0.iter().map(|s| s.id as usize + 1).max().unwrap_or(0);
        bank.songs = vec![short; numbers];
        for (handle, at) in songs.0.iter().enumerate() {
            if music.contains(&(handle as u16)) {
                bank.songs[at.id as usize] = held.clone();
            }
        }
        Loaded {
            found: Found { dir: Default::default(), packs: Vec::new(), report: Default::default() },
            game: Game {
                name: content.game().to_string(),
                content: content.clone(),
                dir: nettai_content::index::content(),
                packs: Vec::new(),
                own: Default::default(),
                others: Vec::new(),
                report: Default::default(),
            },
            graphics: nettai_frontend::testing::graphics(&content, 0x7FFF),
            text: TextMode::Original,
            font: None,
            sound: Some(Sound { banks: vec![Arc::new(bank)], songs, report: Default::default() }),
        }
    }

    fn games(ids: &[&str]) -> Games {
        let content = |id: &str| match id {
            "exe4" => nettai_match::testing::exe4_content(),
            "exe5" => nettai_match::testing::exe5_content(),
            _ => nettai_match::testing::exe6_content(),
        };
        Games::read_of(ids.iter().map(|&id| (id, loaded(content(id)))).collect())
    }

    /// The list is the game's own, then each read game's themes as its content states them, by the user's names:
    /// the game's name as shown in the language, then the theme's. A game not read offers none.
    #[test]
    fn the_themes_are_the_games_in_their_order() {
        let all = games(&["exe4", "exe5", "exe6"]);
        crate::shell::Shell::read_game_names(&["exe4", "exe5", "exe6"], "en");
        let names: Vec<String> = offered(&all).iter().map(|m| name(m, "Game's own")).collect();
        assert_eq!(
            names,
            [
                "Game's own",
                "BN4 Virus Battle",
                "BN4 Boss Battle",
                "BN4 Tournament Battle",
                "BN4 vs Duo",
                "BN5 Virus Battle",
                "BN5 Boss Battle",
                "BN5 vs Nebula Gray",
                "BN6 Virus Battle",
                "BN6 Boss Battle",
                "BN6 vs Cybeast",
                "BN6 Crossover Battle"
            ]
        );
        crate::shell::Shell::read_game_names(&["exe4", "exe5", "exe6"], "ja");
        let ja: Vec<String> = offered(&all).iter().skip(1).map(|m| name(m, "")).collect();
        assert_eq!(ja[3], "EXE4 VS デューオ");
        assert_eq!(ja[10], "EXE6 クロスオーバーバトル");
        let some = games(&["exe6"]);
        assert_eq!(offered(&some).len(), 1 + 4, "BN4's and BN5's aren't read");
        crate::shell::Shell::read_game_names(&["exe4", "exe5", "exe6"], "en");
    }

    /// What a theme plays in place of: the songs each game's round starts, a link battle's (EXE4's a single
    /// battle's and a set's) and its stages' own.
    #[test]
    fn the_battles_own_music() {
        let names = |content: Arc<Content>| {
            let sounds = content.assets.names(nettai_content_api::AssetKind::Sound).into_iter().map(String::from).collect::<Vec<_>>();
            content.battle_music().iter().map(|m| sounds[m.0 as usize].clone()).collect::<Vec<_>>()
        };
        assert_eq!(names(nettai_match::testing::exe4_content()), ["boss-battle", "virus-battle"]);
        assert_eq!(names(nettai_match::testing::exe5_content()), ["virus-battle"]);
        assert_eq!(names(nettai_match::testing::exe6_content()), ["virus-battle"]);
    }

    /// A battle plays the picked theme in place of its game's own music: a BN4 theme in a BN6 battle on a
    /// driver of BN4's bank (its song, as BN4's pack numbers it), BN6's own theme on BN6's; a pick whose game
    /// isn't read, the game's own. The battle is the same tick for tick whatever plays.
    #[test]
    fn a_battle_plays_the_picked_theme() {
        use nettai_audio::{MUSIC_PLAYER, m4a::SongId};
        use nettai_battle::content::PackId;
        let all = games(&["exe4", "exe5", "exe6"]);
        let six = all.read("exe6").unwrap();
        let content = six.content().clone();
        let m = nettai_match::pick::live(&content, "exe6", 11, None).unwrap();
        // The music each driver plays once the round has started, and the battle's digest.
        let play = |music: &Music| {
            let (banks, songs) = sound(&all, &six, music).unwrap();
            let packs = banks.len();
            let folders = [0, 1].map(|side| m.sides[side].folder(&content).saved().unwrap());
            let set = nettai_match::Set::new(content.clone(), m.round(&content, 11), folders);
            let driver = nettai_frontend::driver::LivePlayer::against(set, Default::default(), 11);
            let renderer = nettai_frontend::testing::graphics(&content, 0x7FFF).renderer(TextMode::Original, None);
            let mut player = nettai_frontend::Player::with(renderer, None, Some(nettai_audio::BattleAudio::with_banks(banks, songs)), Box::new(driver));
            for _ in 0..30 {
                player.tick(0);
            }
            let audio = player.audio().unwrap();
            let playing: Vec<Option<SongId>> = (0..packs)
                .map(|p| audio.driver_of(PackId(p as u8)).unwrap().player(MUSIC_PLAYER).filter(|m| m.is_playing()).and_then(|m| m.song()))
                .collect();
            (playing, player.battle().digest())
        };
        // (A song by its pack's number.)
        let number = |game: &str, song: &str| {
            let c = all.read(game).unwrap().content().clone();
            SongId(c.assets.sound(c.assets.handle(nettai_content_api::AssetKind::Sound, song).unwrap()).unwrap().id)
        };
        let own = SongId(content.assets.sound(content.defs.roles().link_battle_music(true).0).unwrap().id);
        let (playing, digest) = play(&Music::GameOwn);
        assert_eq!(playing, [Some(own)], "BN6's own");
        let theme = |game: &str, song: &str| Music::Theme { game: game.into(), song: song.into() };
        let (playing, with_theme) = play(&theme("exe4", "tournament-battle"));
        assert_eq!(playing, [None, Some(number("exe4", "tournament-battle"))], "BN4's, on its own bank's driver");
        assert_eq!(with_theme, digest, "the battle is the same");
        let (playing, _) = play(&theme("exe6", "final-battle"));
        assert_eq!(playing, [Some(number("exe6", "final-battle"))], "BN6's own theme, on BN6's driver");
        let (playing, _) = play(&theme("exe6", "winner-1"));
        assert_eq!(playing, [Some(own)], "no theme of that song: the game's own");
        let some = games(&["exe6"]);
        let six = some.read("exe6").unwrap();
        let (banks, songs) = sound(&some, &six, &theme("exe4", "tournament-battle")).unwrap();
        assert_eq!((banks.len(), songs), (1, six.loaded.sound.as_ref().unwrap().songs.clone()), "BN4 not read: the game's own");
    }

    /// The practice menu's Music changes the battle's music as it runs (`retheme`'s `Player::set_sound_parts`):
    /// the new song at once, on its own pack's driver; the game's own again on the battle's; the battle the same.
    #[test]
    fn a_battle_changes_its_music_as_it_runs() {
        use nettai_audio::{MUSIC_PLAYER, m4a::SongId};
        use nettai_battle::content::PackId;
        let all = games(&["exe4", "exe5", "exe6"]);
        let six = all.read("exe6").unwrap();
        let content = six.content().clone();
        let m = nettai_match::pick::live(&content, "exe6", 11, None).unwrap();
        let player = |music: &Music| {
            let (banks, songs) = sound(&all, &six, music).unwrap();
            let folders = [0, 1].map(|side| m.sides[side].folder(&content).saved().unwrap());
            let set = nettai_match::Set::new(content.clone(), m.round(&content, 11), folders);
            let driver = nettai_frontend::driver::LivePlayer::against(set, Default::default(), 11);
            let renderer = nettai_frontend::testing::graphics(&content, 0x7FFF).renderer(TextMode::Original, None);
            nettai_frontend::Player::with(renderer, None, Some(nettai_audio::BattleAudio::with_banks(banks, songs)), Box::new(driver))
        };
        let playing = |p: &nettai_frontend::Player| -> Vec<Option<SongId>> {
            let audio = p.audio().unwrap();
            (0..3).map_while(|n| audio.driver_of(PackId(n))).map(|d| d.player(MUSIC_PLAYER).filter(|m| m.is_playing()).and_then(|m| m.song())).collect()
        };
        let number = |game: &str, song: &str| {
            let c = all.read(game).unwrap().content().clone();
            SongId(c.assets.sound(c.assets.handle(nettai_content_api::AssetKind::Sound, song).unwrap()).unwrap().id)
        };
        let own = number("exe6", "virus-battle");
        let theme = Music::Theme { game: "exe5".into(), song: "final-battle".into() };
        let (mut changed, mut straight) = (player(&Music::GameOwn), player(&Music::GameOwn));
        for _ in 0..30 {
            changed.tick(0);
            straight.tick(0);
        }
        assert_eq!(playing(&changed), [Some(own)]);
        changed.set_sound_parts(sound(&all, &six, &theme).unwrap());
        for _ in 0..2 {
            changed.tick(0);
            straight.tick(0);
        }
        assert_eq!(playing(&changed), [None, Some(number("exe5", "final-battle"))], "BN5's, at once");
        changed.set_sound_parts(sound(&all, &six, &Music::GameOwn).unwrap());
        for _ in 0..2 {
            changed.tick(0);
            straight.tick(0);
        }
        assert_eq!(playing(&changed), [Some(own), None], "the game's own again, from its start");
        assert_eq!(changed.battle().digest(), straight.battle().digest(), "the battle the same");
    }

    /// A practice set of game `game` (its first round: the music it starts), between two new sides.
    fn practice_set(all: &Games, game: &str, seed: u32) -> nettai_match::Set {
        let content = all.read(game).unwrap().content().clone();
        let m = nettai_match::pick::live(&content, game, seed, None).unwrap();
        let folders = [0, 1].map(|side| m.sides[side].folder(&content).saved().unwrap());
        nettai_match::Set::new(content.clone(), m.round(&content, seed), folders)
    }

    /// What Practice's entry plays before `set` with `music` picked, as `preview` makes it: the battle's own sound's
    /// parts, started by the song its first round starts.
    fn entry_music(all: &Games, set: &nettai_match::Set, music: &Music) -> PreviewSong {
        let game = set.content().game().to_string();
        let ready = all.read(&game).unwrap();
        let song = set.first().settings.music(set.content()).expect("a link battle's music");
        PreviewSong { key: key(&game, music, song), parts: sound(all, &ready, music).unwrap(), music: song }
    }

    /// The menus' sound, the selected game `game`'s, wanting its music, and a clock of frames for it.
    fn menus(all: &Games, game: &str) -> (crate::sound::Sound, impl FnMut(&mut crate::sound::Sound, u64)) {
        let mut sound = crate::sound::Sound::silent();
        sound.use_game(game, &all.read(game).unwrap().loaded);
        sound.want_music(true);
        let (start, mut at) = (std::time::Instant::now(), 0);
        let frames = move |sound: &mut crate::sound::Sound, n: u64| {
            for _ in 0..n {
                at += 17;
                sound.frame(start + std::time::Duration::from_millis(at), None, &mut Vec::new());
            }
        };
        (sound, frames)
    }

    /// Practice's entry plays the music its session will start in place of the title music (`preview`'s
    /// `Sound::set_preview`): the game's own battle music on the game's bank, a theme on its own pack's; another in
    /// its place; none, the title music; and it fades out where the menus' music isn't wanted.
    #[test]
    fn the_entry_plays_the_battles_music() {
        use nettai_audio::m4a::SongId;
        let all = games(&["exe4", "exe6"]);
        let set = practice_set(&all, "exe6", 11);
        let (mut sound, mut frames) = menus(&all, "exe6");
        let number = |game: &str, song: &str| {
            let c = all.read(game).unwrap().content().clone();
            SongId(c.assets.sound(c.assets.handle(nettai_content_api::AssetKind::Sound, song).unwrap()).unwrap().id)
        };
        let theme = |song: &str| Music::Theme { game: "exe4".into(), song: song.into() };
        sound.set_preview(Some(entry_music(&all, &set, &Music::GameOwn)));
        frames(&mut sound, 3);
        assert_eq!(sound.music_song(), Some((true, number("exe6", "virus-battle"))), "the game's own battle music, not the title music");
        sound.set_preview(Some(entry_music(&all, &set, &theme("tournament-battle"))));
        frames(&mut sound, 3);
        assert_eq!(sound.music_song(), Some((true, number("exe4", "tournament-battle"))), "a theme, on its own pack's driver");
        let (_, _, at) = sound.music_at().unwrap();
        sound.set_preview(Some(entry_music(&all, &set, &theme("tournament-battle"))));
        frames(&mut sound, 3);
        assert!(sound.music_at().unwrap().2 > at, "the same one again goes on");
        sound.set_preview(Some(entry_music(&all, &set, &theme("final-battle"))));
        frames(&mut sound, 3);
        assert_eq!(sound.music_song(), Some((true, number("exe4", "final-battle"))), "another in its place");
        sound.want_music(false);
        frames(&mut sound, 90);
        assert_eq!(sound.music_song(), None, "faded out and stopped where the menus' music isn't wanted");
        sound.want_music(true);
        frames(&mut sound, 3);
        assert_eq!(sound.music_song(), Some((true, number("exe4", "final-battle"))), "again from its start");
        sound.set_preview(None);
        frames(&mut sound, 3);
        // (content/'s EXE6 with its assets made up names no title music: the title music is none here.)
        assert_eq!(sound.music_song(), None, "the title music again");
    }

    /// The battle started from the entry takes over the entry's music (`Sound::take_preview`,
    /// `Player::carry_sound`): its round's start finds its music playing, so the song goes on from where the entry
    /// had it (its clock past the entry's, where a battle of its own starts it from 0), the game's own and a theme
    /// alike. The battle is the same tick for tick.
    #[test]
    fn the_battle_goes_on_with_the_entrys_music() {
        use nettai_audio::MUSIC_PLAYER;
        use nettai_battle::content::PackId;
        let all = games(&["exe4", "exe6"]);
        let set = practice_set(&all, "exe6", 11);
        let content = set.content().clone();
        let player = |parts: SoundParts| {
            let driver = nettai_frontend::driver::LivePlayer::against(set.clone(), Default::default(), 11);
            let renderer = nettai_frontend::testing::graphics(&content, 0x7FFF).renderer(TextMode::Original, None);
            nettai_frontend::Player::with(renderer, None, Some(nettai_audio::BattleAudio::with_banks(parts.0, parts.1)), Box::new(driver))
        };
        // (The battle's music: its song, and how far into it, on whichever pack's driver plays it.)
        let music = |p: &nettai_frontend::Player| {
            let audio = p.audio().unwrap();
            (0..3).map_while(|n| audio.driver_of(PackId(n))).find_map(|d| d.player(MUSIC_PLAYER).filter(|m| m.is_playing()).and_then(|m| Some((m.song()?, m.clock()))))
        };
        for picked in [Music::GameOwn, Music::Theme { game: "exe4".into(), song: "tournament-battle".into() }] {
            let entry = entry_music(&all, &set, &picked);
            let (key, parts) = (entry.key.clone(), entry.parts.clone());
            let (mut sound, mut frames) = menus(&all, "exe6");
            sound.set_preview(Some(entry));
            // (The entry a while, then the cards: the menus' music goes on.)
            frames(&mut sound, 240);
            let (_, song, at) = sound.music_at().unwrap();
            assert!(at > 0, "{picked:?}: the entry's music has played");
            let audio = sound.take_preview(&key).expect("the entry's music, to take over");
            assert_eq!(sound.music_song(), None, "{picked:?}: the menus play it no more");
            let (mut carried, mut own) = (player(parts.clone()), player(parts));
            carried.carry_sound(audio);
            for _ in 0..30 {
                carried.tick(0);
                own.tick(0);
            }
            let (carried_song, carried_at) = music(&carried).expect("the battle's music");
            let (own_song, own_at) = music(&own).expect("the battle's music");
            assert_eq!((carried_song, own_song), (song, song), "{picked:?}: the same song");
            assert!(carried_at > at && carried_at > own_at + at / 2, "{picked:?}: it went on ({carried_at} past the entry's {at}; a battle's own is at {own_at})");
            assert_eq!(carried.battle().digest(), own.battle().digest(), "{picked:?}: the battle the same");
        }
    }

    /// A practice battle of `set` on the sound of `music`'s parts, its levels the menus' `music` and `effects`.
    fn battle_on(all: &Games, set: &nettai_match::Set, music: &Music, levels: (f32, f32)) -> nettai_frontend::Player {
        let content = set.content().clone();
        let ready = all.read(content.game()).unwrap();
        let (banks, songs) = sound(all, &ready, music).unwrap();
        let driver = nettai_frontend::driver::LivePlayer::against(set.clone(), Default::default(), 11);
        let renderer = nettai_frontend::testing::graphics(&content, 0x7FFF).renderer(TextMode::Original, None);
        let mut player = nettai_frontend::Player::with(renderer, None, Some(nettai_audio::BattleAudio::with_banks(banks, songs)), Box::new(driver));
        player.set_sound_levels(levels.0, levels.1);
        player
    }

    /// What `audio`'s music player plays, and how far into it, on whichever pack's driver plays it; and whether a
    /// player of another than the music's plays (a sound effect).
    fn heard(audio: &nettai_audio::BattleAudio) -> (Option<(nettai_audio::m4a::SongId, u32)>, bool) {
        use nettai_audio::MUSIC_PLAYER;
        let drivers = || (0..=u8::MAX).map_while(|n| audio.driver_of(nettai_battle::content::PackId(n)));
        let music = drivers().find_map(|d| d.player(MUSIC_PLAYER).filter(|m| m.is_playing()).and_then(|m| Some((m.song()?, m.clock()))));
        let effects = drivers().any(|d| d.players().iter().enumerate().any(|(p, m)| p != MUSIC_PLAYER.0 as usize && m.is_playing()));
        (music, effects)
    }

    /// The entry the session's battle ends back at (EXIT, or the battle stopped) takes over the battle's music the
    /// other way (`Player::take_sound`, `Sound::set_preview_after`): the song goes on from where the battle had it
    /// (its clock past the battle's, not from 0), at once at the music's level with no fade-in; the battle's sound
    /// effects stop, and its levels are the menus'. The game's own and a theme alike, and a theme the practice
    /// menu changed the battle's music to (`retheme`), the entry's pick being that one.
    #[test]
    fn the_entry_goes_on_with_the_battles_music() {
        use nettai_audio::{MUSIC_PLAYER, SoundCue};
        use nettai_battle::content::{PackId, SoundRole};
        let all = games(&["exe4", "exe6"]);
        let set = practice_set(&all, "exe6", 11);
        let effect = SoundCue::Effect(set.content().defs.roles().sounds[&SoundRole::CustomCursor]);
        let theme = Music::Theme { game: "exe4".into(), song: "tournament-battle".into() };
        // (The music the battle started with, and the pick as it ends.)
        for (started, picked) in [(Music::GameOwn, Music::GameOwn), (theme.clone(), theme.clone()), (Music::GameOwn, theme.clone())] {
            let mut player = battle_on(&all, &set, &started, (0.5, 1.0));
            for _ in 0..240 {
                player.tick(0);
            }
            if started != picked {
                player.set_sound_parts(sound(&all, &all.read("exe6").unwrap(), &picked).unwrap());
                for _ in 0..60 {
                    player.tick(0);
                }
            }
            let mut audio = player.take_sound().expect("the battle's sound");
            // (A sound effect playing as it ends, and another queued.)
            audio.handle(&[effect]);
            audio.tick(&mut Vec::new());
            audio.handle(&[effect]);
            let (Some((song, at)), effects) = heard(&audio) else { panic!("{picked:?}: the battle's music plays") };
            assert!(at > 0 && effects, "{picked:?}: the battle's music has played, and an effect plays");
            let (mut sound, mut frames) = menus(&all, "exe6");
            let entry = entry_music(&all, &set, &picked);
            sound.set_preview_after(Some(entry), Some(audio));
            frames(&mut sound, 3);
            let (preview, now, later) = sound.music_at().expect("the entry's music");
            assert_eq!((preview, now), (true, song), "{picked:?}: the entry plays the battle's song");
            assert!(later > at, "{picked:?}: it went on ({later} past the battle's {at})");
            let (audio, fade) = sound.preview_sound().unwrap();
            assert_eq!(fade, 1.0, "{picked:?}: at the music's level at once");
            assert!(!heard(audio).1, "{picked:?}: the battle's effects stopped");
            assert!((0..3).map_while(|n| audio.driver_of(PackId(n))).all(|d| d.level(MUSIC_PLAYER) == 256), "{picked:?}: the menus' levels");
        }
    }

    /// Where the battle's music had stopped as the session ends (a round's end), or another plays (the result's),
    /// the entry's starts from its beginning, fading in.
    #[test]
    fn the_entry_starts_its_music_where_the_battles_stopped() {
        use nettai_audio::SoundCue;
        use nettai_battle::content::MusicRole;
        let all = games(&["exe6"]);
        let set = practice_set(&all, "exe6", 11);
        let winner = set.content().defs.roles().music[&MusicRole::Winner];
        for end in [SoundCue::StopMusic, SoundCue::Music(winner)] {
            let mut player = battle_on(&all, &set, &Music::GameOwn, (1.0, 1.0));
            for _ in 0..240 {
                player.tick(0);
            }
            let mut audio = player.take_sound().unwrap();
            audio.handle(&[end]);
            audio.tick(&mut Vec::new());
            let (mut sound, mut frames) = menus(&all, "exe6");
            let entry = entry_music(&all, &set, &Music::GameOwn);
            let song = entry.music;
            sound.set_preview_after(Some(entry), Some(audio));
            frames(&mut sound, 3);
            let (preview, _, at) = sound.music_at().expect("the entry's music");
            assert!(preview && at < 10, "{end:?}: from its beginning ({at})");
            let (audio, fade) = sound.preview_sound().unwrap();
            assert!(fade < 1.0, "{end:?}: fading in");
            assert_eq!(audio.music_heard(), Some(song), "{end:?}: the entry's own song");
        }
    }
}
