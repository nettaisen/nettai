//! A game's backgrounds, music and sprites of its own (docs/design/asset-formats.md §8): `backgrounds.toml`,
//! `music.toml` and `sprites.toml` in its pack's folder, read with its content as its locales are ([`read`]) and
//! made the content's in its define phase (`nettai_battle::Content::presentation`).
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
//! ```toml
//! # sprites.toml
//! [sprites.my-bomb]                                # one made from a picture of the pack's own
//! file = "sprites/my_bomb.png"
//! frame = [32, 32]                                 # a cell's size (none: one row of as many cells as frames)
//! origin = [16, 28]                                # where in a cell the object stands (none: the bottom's middle)
//! shadow = "sprites/my_bomb_shadow.png"
//! animations = [[6, 4, 3, 2], { frames = [3, 2], loop = true }]   # each frame's steps, in its cells' order
//!
//! [sprites.explosion]                              # a ROM's sprite, timed otherwise
//! animations = [[2, 2, 2, 2, 2, 2], [4, 4]]
//! ```
//!
//! A file's path is the pack's (`backgrounds/my_stage.png`, from its folder). No module states any of this and
//! no tick reads it, so it is out of the content's hash: content that differs in its backgrounds, music or sprites'
//! looks alone is the same content. (How long an action takes is its module's: the battle counts a sprite's steps,
//! and drawing finds the frame they reach.)
use nettai_content_api::{BackgroundMusic, CustomBackground, CustomInstrument, CustomSound, CustomSprite, Presentation, SpriteAnimation, packs};
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

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct SpritesFile {
    #[serde(default)]
    sprites: BTreeMap<String, SpriteEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpriteEntry {
    file: Option<String>,
    frame: Option<[u16; 2]>,
    origin: Option<[i16; 2]>,
    shadow: Option<String>,
    animations: Vec<AnimationEntry>,
}

/// An animation: its frames' durations, or a table of them that may loop.
#[derive(Deserialize)]
#[serde(untagged)]
enum AnimationEntry {
    Frames(Vec<u8>),
    Table {
        frames: Vec<u8>,
        #[serde(rename = "loop", default)]
        looping: bool,
    },
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

/// Game `game`'s backgrounds, music and sprites of its own, from its pack's folder `pack` (none there: none). An
/// error says which file and what is wrong with it.
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
    let sprites = text(packs::SPRITES_FILE)?
        .map_or(Ok(SpritesFile::default()), |t| toml::from_str::<SpritesFile>(&t).map_err(|e| format!("{game}/{}: {e}", packs::SPRITES_FILE)))?;
    let mut p = parse(game, backgrounds, music)?;
    p.sprites = parse_sprites(game, sprites)?;
    Ok(p)
}

/// What `sprites.toml` states, checked: each sprite's entry and the files it names.
fn parse_sprites(game: &str, s: SpritesFile) -> Result<BTreeMap<String, CustomSprite>, String> {
    let source = format!("{game}/{}", packs::SPRITES_FILE);
    let mut out = BTreeMap::new();
    for (name, e) in s.sprites {
        let at = format!("{source}: sprite {name:?}");
        let mut made = CustomSprite { source: source.clone(), ..Default::default() };
        for (i, a) in e.animations.into_iter().enumerate() {
            let (frames, looping) = match a {
                AnimationEntry::Frames(f) => (f, false),
                AnimationEntry::Table { frames, looping } => (frames, looping),
            };
            if frames.is_empty() || frames.contains(&0) {
                return Err(format!("{at}: animation {i} is its frames' durations, each a number of steps from 1"));
            }
            made.animations.push(SpriteAnimation { frames, looping });
        }
        if made.animations.is_empty() {
            return Err(format!("{at}: a sprite states its `animations` (each its frames' durations)"));
        }
        match e.file {
            Some(file) => {
                made.file = Some(pack_file(game, &at, &file, "png")?);
                made.frame = e.frame.map(|[w, h]| (w, h));
                if made.frame.is_some_and(|(w, h)| w == 0 || h == 0) {
                    return Err(format!("{at}: `frame` is a cell's width and height in pixels, each from 1"));
                }
                made.origin = e.origin.map(|[x, y]| (x, y));
                made.shadow = e.shadow.map(|f| pack_file(game, &format!("{at}: shadow"), &f, "png")).transpose()?;
            }
            None if e.frame.is_some() || e.origin.is_some() || e.shadow.is_some() => {
                return Err(format!("{at}: `frame`, `origin` and `shadow` are a sprite's made from a picture: it names its `file` (an indexed PNG)"));
            }
            None => {}
        }
        out.insert(name, made);
    }
    Ok(out)
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

    fn sprites_str(s: &str) -> Result<BTreeMap<String, CustomSprite>, String> {
        parse_sprites("g", toml::from_str::<SpritesFile>(s).map_err(|e| e.to_string())?)
    }

    /// A sprite of the pack's own, its picture and its animations' timing, and a ROM's timed otherwise.
    #[test]
    fn sprites_toml_states_pictures_and_timing() {
        let s = sprites_str(
            r#"
[sprites.drone]
file = "sprites/drone.png"
frame = [32, 24]
origin = [16, 20]
shadow = "sprites/drone_shadow.png"
animations = [[3, 3], { frames = [2, 2, 4], loop = true }]
[sprites.explosion]
animations = [[1, 1]]
"#,
        )
        .unwrap();
        let anim = |frames: &[u8], looping| SpriteAnimation { frames: frames.to_vec(), looping };
        assert_eq!(
            s["drone"],
            CustomSprite {
                source: "g/sprites.toml".into(),
                file: Some("g:sprites/drone.png".into()),
                frame: Some((32, 24)),
                origin: Some((16, 20)),
                shadow: Some("g:sprites/drone_shadow.png".into()),
                animations: vec![anim(&[3, 3], false), anim(&[2, 2, 4], true)],
            }
        );
        assert_eq!((s["explosion"].file.as_ref(), &s["explosion"].animations[..]), (None, &[anim(&[1, 1], false)][..]));
        let err = |t: &str| sprites_str(t).unwrap_err();
        assert!(err("[sprites.x]
animations = [[0]]").contains("from 1"));
        assert!(err("[sprites.x]
animations = []").contains("states its `animations`"));
        assert!(err("[sprites.x]
frame = [8, 8]
animations = [[1]]").contains("it names its `file`"));
        assert!(err("[sprites.x]
file = \"x.gif\"
animations = [[1]]").contains("is a .png file"));
    }
}
