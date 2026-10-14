//! The battle's music (docs/app.md §9, "The battle's music"): the game's own, or a battle theme the player picked,
//! of any game whose pack is there. The user: "you should also allow for selectable battle themes (bn4 virus
//! battle, bn4 boss battle, bn4 tournament battle, bn4 vs duo, bn5 virus battle, bn5 boss battle, bn5 vs nebula
//! gray, bn6 virus battle, bn6 boss battle, bn6 vs cybeast, bn6 crossover battle)", picked "beside the arena, each
//! their own".
//!
//! What the themes are is each game's content: its rules' `battle_themes`, songs of its pack, and their names in
//! its locales (`battle_themes`, by the song's asset name); this names no song and no game. A theme is offered
//! once its game's parts are read (its content and its sound, `Games::read`: no runtime is needed), whichever
//! game is selected. The pick is one, the player's own, remembered in the settings ([`REMEMBERED`]); the screens
//! that show it (Play's lobby, Practice's battle options) show and change that one. Every battle the shell runs
//! plays with it: a practice battle, an online match (each player hears their own: nothing of it goes on the
//! wire), a replay watched, and a replay's video. It is presentation: the battle, its cues, a replay and its
//! rollback are the same whatever plays (`nettai_audio::with_theme`).

use crate::games::{Games, Ready};
use crate::settings::Settings;
use crate::shell::Shell;
use nettai_frontend::preload::SoundParts;
use slint::ComponentHandle;
use std::cell::RefCell;
use std::collections::HashMap;

/// The key the pick is remembered by (`Settings::remembered`): `<game>:<song>`, the theme's game and its song's
/// asset name (`exe4:tournament-battle`); none, the game's own.
pub const REMEMBERED: &str = "battle.music";

/// A battle's music.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Music {
    /// The battle's game's own: what it plays without a pick.
    #[default]
    GameOwn,
    /// A battle theme: game `game`'s theme of song `song` (its asset name).
    Theme { game: String, song: String },
}

impl Music {
    /// The pick the settings remember (a value that names no theme is the game's own).
    pub fn remembered(settings: &Settings) -> Music {
        match settings.remembered.get(REMEMBERED).and_then(|v| v.split_once(':')) {
            Some((game, song)) if !game.is_empty() && !song.is_empty() => Music::Theme { game: game.into(), song: song.into() },
            _ => Music::GameOwn,
        }
    }

    /// Remember the pick in the settings (the caller keeps them).
    pub fn remember(&self, settings: &mut Settings) {
        match self {
            Music::GameOwn => settings.remembered.remove(REMEMBERED),
            Music::Theme { game, song } => settings.remembered.insert(REMEMBERED.into(), format!("{game}:{song}")),
        };
    }
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

// ---- A screen's Music row (Play's lobby, Practice's entry) ---------------------------------------------------

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

/// Open the select's list under its control `id` (the screen hears the choice as `choose`).
pub fn open(shell: &mut Shell, id: &str) {
    let (offered, at) = list(shell);
    let own = own_words(shell);
    let names = offered.iter().map(|m| name(m, &own)).collect();
    shell.pick(id, 0, names, at);
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

/// Have the menus play the pick in place of the title music where it is a theme (`on`: Practice's entry shows;
/// the user: "in the practice screen, if the bgm choice changes from game default you should start playing the
/// bgm"), else the title music: with its own pack's bank, at the music's level, crossfading as it changes
/// (`Sound::set_preview`). A theme whose game isn't read is the title music.
pub fn preview(shell: &mut Shell, on: bool) {
    let theme = match (on, shell.battle_music()) {
        (true, Music::Theme { game, song }) => shell.games.read(&game).and_then(|g| g.loaded.theme(&song)).map(|t| (format!("{game}:{song}"), t)),
        _ => None,
    };
    shell.sound.set_preview(theme);
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

    /// The pick is kept in the settings by its key, and read back; the game's own is no key; a value that
    /// names no theme reads as the game's own.
    #[test]
    fn the_pick_is_kept_and_read_back() {
        let mut s = Settings::default();
        assert_eq!(Music::remembered(&s), Music::GameOwn);
        let pick = Music::Theme { game: "exe4".into(), song: "tournament-battle".into() };
        pick.remember(&mut s);
        assert_eq!(s.remembered.get(REMEMBERED).map(String::as_str), Some("exe4:tournament-battle"));
        let text = toml::to_string(&s).unwrap();
        assert_eq!(Music::remembered(&toml::from_str(&text).unwrap()), pick, "through the file");
        Music::GameOwn.remember(&mut s);
        assert!(!s.remembered.contains_key(REMEMBERED));
        for odd in ["", "exe4", ":x", "exe4:"] {
            s.remembered.insert(REMEMBERED.into(), odd.into());
            assert_eq!(Music::remembered(&s), Music::GameOwn, "{odd:?}");
        }
    }

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

    /// Practice's entry plays the pick in place of the title music (`preview`'s `Sound::set_preview`): a theme on
    /// its own pack's bank; another theme in its place; none, the title music; and it fades out where the menus'
    /// music isn't wanted (a battle).
    #[test]
    fn the_entry_previews_the_pick() {
        use nettai_audio::m4a::SongId;
        let all = games(&["exe4", "exe6"]);
        let mut sound = crate::sound::Sound::silent();
        sound.use_game("exe6", &all.read("exe6").unwrap().loaded);
        sound.want_music(true);
        let start = std::time::Instant::now();
        let mut at = 0;
        let mut frames = |sound: &mut crate::sound::Sound, n: u64| {
            for _ in 0..n {
                at += 17;
                sound.frame(start + std::time::Duration::from_millis(at), None, &mut Vec::new());
            }
        };
        let theme = |song: &str| {
            let t = all.read("exe4").unwrap().loaded.theme(song).unwrap();
            (format!("exe4:{song}"), t)
        };
        let number = |song: &str| SongId(all.read("exe4").unwrap().loaded.theme(song).unwrap().song);
        sound.set_preview(Some(theme("tournament-battle")));
        frames(&mut sound, 3);
        assert_eq!(sound.music_song(), Some((true, number("tournament-battle"))));
        sound.set_preview(Some(theme("tournament-battle")));
        sound.set_preview(Some(theme("final-battle")));
        frames(&mut sound, 3);
        assert_eq!(sound.music_song(), Some((true, number("final-battle"))), "another in its place");
        sound.want_music(false);
        frames(&mut sound, 90);
        assert_eq!(sound.music_song(), None, "faded out and stopped under a battle");
        sound.want_music(true);
        frames(&mut sound, 3);
        assert_eq!(sound.music_song(), Some((true, number("final-battle"))), "again from its start");
        sound.set_preview(None);
        frames(&mut sound, 3);
        // (content/'s EXE6 with its assets made up names no title music: the title music is none here.)
        assert_eq!(sound.music_song(), None, "the title music again");
    }
}
