//! A game's backgrounds and music (docs/design/asset-formats.md §8): `backgrounds.toml` and `music.toml` in its
//! pack's folder, read with its content as its locales are ([`read`]) and made the content's in its define phase
//! (`nettai_battle::Content::presentation`).
//!
//! ```toml
//! # backgrounds.toml
//! stage = "comp"                                   # the background every link battle stage is drawn on
//! link_battle = ["lans-hp", "acdc-hp", "comp"]     # those a link battle picks one of at random
//!
//! [backgrounds]
//! acdc-area = { music = "virus-battle" }           # each background's music
//!
//! [backgrounds.my-stage]                           # one made from a picture of the pack's own
//! file = "backgrounds/my_stage.png"
//! scroll = [16, 8]
//! speed_up = 30
//! frames = [8, 8, 8]
//! loop = true
//! music = "my-theme"
//! ```
//!
//! ```toml
//! # music.toml
//! themes = ["virus-battle", "boss-battle", "my-theme"]   # the songs a player may pick as a battle's music
//!
//! [songs.my-theme]                                 # one made from a MIDI file of the pack's own
//! file = "music/my_theme.mid"
//! voices = "virus-battle"                          # the ROMs' song whose instruments it plays on
//!
//! [songs.my-theme.instruments.41]                  # an instrument of its own, by program
//! file = "music/strings.wav"
//! key = 69
//! loop = 1024
//! envelope = { release = 200 }
//! ```
//!
//! A file's path is the pack's (`backgrounds/my_stage.png`, from its folder). No module states any of this and
//! no tick reads it, so it is out of the content's hash: content that differs in its backgrounds or music alone is
//! the same content.
use nettai_content_api::{BackgroundMusic, CustomBackground, CustomInstrument, CustomSound, Presentation, packs};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct BackgroundsFile {
    stage: Option<String>,
    #[serde(default)]
    link_battle: Vec<String>,
    #[serde(default)]
    backgrounds: toml::Table,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BackgroundEntry {
    music: Option<String>,
    file: Option<String>,
    scroll: Option<[i16; 2]>,
    speed_up: Option<u16>,
    frames: Option<Vec<u16>>,
    #[serde(rename = "loop")]
    looping: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct MusicFile {
    #[serde(default)]
    themes: Vec<String>,
    #[serde(default)]
    songs: BTreeMap<String, SongEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SongEntry {
    file: String,
    voices: String,
    #[serde(default)]
    instruments: BTreeMap<String, InstrumentEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InstrumentEntry {
    file: String,
    key: Option<u8>,
    #[serde(rename = "loop")]
    loop_start: Option<u32>,
    envelope: Option<EnvelopeEntry>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct EnvelopeEntry {
    attack: Option<u8>,
    decay: Option<u8>,
    sustain: Option<u8>,
    release: Option<u8>,
}

/// Game `game`'s backgrounds and music, from its pack's folder `pack` (none there: none). An error says which file
/// and what is wrong with it.
pub fn read(pack: &Path, game: &str) -> Result<Presentation, String> {
    let text = |file: &str| -> Result<Option<String>, String> {
        let path = pack.join(file);
        if !nettai_files::is_file(&path) {
            return Ok(None);
        }
        nettai_files::read_to_string(&path).map(Some).map_err(|e| format!("reading {}: {e}", path.display()))
    };
    let backgrounds = text(packs::BACKGROUNDS_FILE)?.map_or(Ok(BackgroundsFile::default()), |t| {
        toml::from_str::<BackgroundsFile>(&t).map_err(|e| format!("{game}/{}: {e}", packs::BACKGROUNDS_FILE))
    })?;
    let music = text(packs::MUSIC_FILE)?
        .map_or(Ok(MusicFile::default()), |t| toml::from_str::<MusicFile>(&t).map_err(|e| format!("{game}/{}: {e}", packs::MUSIC_FILE)))?;
    parse(game, backgrounds, music)
}

/// What the two files state, checked: each background's entry, each song's, the files they name.
fn parse(game: &str, b: BackgroundsFile, m: MusicFile) -> Result<Presentation, String> {
    let bg_file = format!("{game}/{}", packs::BACKGROUNDS_FILE);
    let music_file = format!("{game}/{}", packs::MUSIC_FILE);
    let mut p = Presentation { stage_background: b.stage, link_battle: b.link_battle, ..Default::default() };
    let mut music = BTreeMap::new();
    for (name, entry) in b.backgrounds {
        let at = format!("{bg_file}: background {name:?}");
        let entry: BackgroundEntry = entry.try_into().map_err(|e| format!("{at}: {e}"))?;
        if let Some(song) = entry.music {
            music.insert(name.clone(), song);
        }
        match entry.file {
            Some(file) => {
                let mut made = CustomBackground { source: bg_file.clone(), file: pack_file(game, &at, &file, "png")?, ..Default::default() };
                let scroll = entry.scroll.unwrap_or([0, 0]);
                made.scroll = (i32::from(scroll[0]), i32::from(scroll[1]));
                if let Some(n) = entry.speed_up {
                    if n == 0 {
                        return Err(format!("{at}: `speed_up` is a number of frames, from 1"));
                    }
                    if made.scroll == (0, 0) {
                        return Err(format!("{at}: `speed_up` is how many frames it takes to reach its `scroll`, and it has none"));
                    }
                    made.speed_up = Some(n);
                }
                if let Some(frames) = entry.frames {
                    if frames.is_empty() || frames.contains(&0) {
                        return Err(format!("{at}: `frames` is its pictures' durations, each a number of frames from 1"));
                    }
                    made.frames = frames;
                }
                made.looping = entry.looping.unwrap_or(false);
                if made.looping && made.frames.is_empty() {
                    return Err(format!("{at}: `loop` plays its `frames` again, and it has none"));
                }
                p.custom_backgrounds.insert(name.clone(), made);
            }
            None if entry.scroll.is_some() || entry.speed_up.is_some() || entry.frames.is_some() || entry.looping.is_some() => {
                return Err(format!("{at}: `scroll`, `speed_up`, `frames` and `loop` are a background's made from a picture: it names its `file` (an indexed PNG)"));
            }
            None if !music.contains_key(&name) => {
                return Err(format!("{at}: a background names its `music`, and one made from a picture its `file` (an indexed PNG)"));
            }
            None => {}
        }
        p.backgrounds.push(name);
    }
    p.music = BackgroundMusic(music);
    let mut themes: Vec<String> = Vec::with_capacity(m.themes.len());
    for theme in m.themes {
        if themes.contains(&theme) {
            return Err(format!("{music_file}: `themes` names {theme:?} twice"));
        }
        themes.push(theme);
    }
    p.battle_themes = themes;
    for (name, s) in m.songs {
        let at = format!("{music_file}: song {name:?}");
        let mut made = CustomSound { source: music_file.clone(), file: pack_file(game, &at, &s.file, "mid")?, voices: s.voices, ..Default::default() };
        for (program, i) in s.instruments {
            let at = format!("{at}: instrument {program}");
            let program: u8 = program.parse().ok().filter(|p| *p <= 127).ok_or_else(|| format!("{at}: an instrument is keyed by its program, 0 to 127"))?;
            let mut inst = CustomInstrument { file: pack_file(game, &at, &i.file, "wav")?, loop_start: i.loop_start, ..Default::default() };
            if let Some(k) = i.key {
                if k > 127 {
                    return Err(format!("{at}: `key` is a MIDI key, 0 to 127"));
                }
                inst.key = k;
            }
            if let Some(e) = i.envelope {
                for (slot, v) in [e.attack, e.decay, e.sustain, e.release].into_iter().enumerate() {
                    if let Some(v) = v {
                        inst.envelope[slot] = v;
                    }
                }
            }
            made.instruments.insert(program, inst);
        }
        p.songs.insert(name, made);
    }
    Ok(p)
}

/// The pack's file `path` (from its folder), as a custom asset's file is named: its pack, then its path in it
/// (`exe6:backgrounds/my_stage.png`), its extension `extension`.
fn pack_file(game: &str, at: &str, path: &str, extension: &str) -> Result<String, String> {
    if !path.rsplit_once('.').is_some_and(|(_, ext)| ext == extension) {
        return Err(format!("{at}: `file` {path:?} is a .{extension} file"));
    }
    if path.starts_with('/') || path.split('/').any(|part| part.is_empty() || part == "." || part == "..") {
        return Err(format!("{at}: `file` {path:?} is a path in the pack, from its folder (`backgrounds/my_stage.png`)"));
    }
    Ok(format!("{game}:{path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_str(backgrounds: &str, music: &str) -> Result<Presentation, String> {
        let b = toml::from_str::<BackgroundsFile>(backgrounds).map_err(|e| e.to_string())?;
        let m = toml::from_str::<MusicFile>(music).map_err(|e| e.to_string())?;
        parse("g", b, m)
    }

    /// The files' backgrounds, music, themes and songs, each where the content takes it.
    #[test]
    fn the_files_state_the_backgrounds_and_music() {
        let p = read_str(
            r#"
stage = "comp"
link_battle = ["comp", "lans-hp", "comp"]
[backgrounds]
comp = { music = "virus-battle" }
lans-hp = { music = "boss-battle" }
[backgrounds.sea]
file = "backgrounds/sea.png"
scroll = [0, 16]
speed_up = 30
frames = [8, 8]
loop = true
music = "sea-theme"
"#,
            r#"
themes = ["virus-battle", "boss-battle", "sea-theme"]
[songs.sea-theme]
file = "music/sea.mid"
voices = "virus-battle"
[songs.sea-theme.instruments.41]
file = "music/strings.wav"
key = 69
loop = 1024
envelope = { release = 200 }
"#,
        )
        .unwrap();
        assert_eq!((p.stage_background.as_deref(), &p.link_battle[..]), (Some("comp"), &["comp".to_string(), "lans-hp".into(), "comp".into()][..]));
        assert_eq!(p.backgrounds, ["comp", "lans-hp", "sea"]);
        assert_eq!(p.music.0.get("sea").map(String::as_str), Some("sea-theme"));
        let sea = &p.custom_backgrounds["sea"];
        assert_eq!((sea.file.as_str(), sea.scroll, sea.speed_up, &sea.frames[..], sea.looping), ("g:backgrounds/sea.png", (0, 16), Some(30), &[8u16, 8][..], true));
        assert_eq!(p.battle_themes, ["virus-battle", "boss-battle", "sea-theme"]);
        let song = &p.songs["sea-theme"];
        assert_eq!((song.file.as_str(), song.voices.as_str(), song.source.as_str()), ("g:music/sea.mid", "virus-battle", "g/music.toml"));
        let inst = &song.instruments[&41];
        assert_eq!((inst.file.as_str(), inst.key, inst.loop_start, inst.envelope), ("g:music/strings.wav", 69, Some(1024), [255, 0, 255, 200]));
    }

    /// What the files can't be is said, with the file and the entry.
    #[test]
    fn the_files_say_what_they_cant_be() {
        let err = |b: &str, m: &str| read_str(b, m).unwrap_err();
        assert!(err("[backgrounds]\ncomp = {}", "").contains("background \"comp\": a background names its `music`"));
        assert!(err("[backgrounds.sea]\nscroll = [1, 2]\nmusic = \"x\"", "").contains("it names its `file`"));
        assert!(err("[backgrounds.sea]\nfile = \"sea.gif\"", "").contains("is a .png file"));
        assert!(err("[backgrounds.sea]\nfile = \"../sea.png\"", "").contains("a path in the pack"));
        assert!(err("[backgrounds.sea]\nfile = \"sea.png\"\nspeed_up = 4", "").contains("it has none"));
        assert!(err("", "themes = [\"a\", \"a\"]").contains("names \"a\" twice"));
        assert!(err("", "[songs.x]\nfile = \"x.mid\"\nvoices = \"v\"\n[songs.x.instruments.128]\nfile = \"s.wav\"").contains("0 to 127"));
        assert!(read_str("colour = 1", "").unwrap_err().contains("unknown field"));
    }
}
