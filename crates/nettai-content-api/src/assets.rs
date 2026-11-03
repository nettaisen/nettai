//! Assets by name (docs/design/content-model-v2.md §6.3, rules-in-luau.md
//! §7.4): content names a sprite, a sound, a banner, a background or a
//! mugshot, and the define phase interns the names to asset handles, each
//! its place among its kind's names in byte order ([`AssetNames`]). The
//! names are the content's alone: a sprite is the one its module defines
//! (`asset.sprite("bomb", { { 3, 3, 3 } })`: its name, and the timing of
//! its animations, [`SpriteDef`]), any other asset the one a module names
//! (`asset.sound("throw")`). So a battle needs no assets: which assets
//! there are, their handles and the sprites' timing are all content.
//!
//! A name is also what the game's assets, extracted from its ROMs, have the
//! asset under (a match plays one game, so a name is its game's:
//! docs/design/content-model-v2.md §4.0). What the ROMs number each (a
//! sprite's category and index, a sound's song-table entry) is the assets'
//! index ([`AssetIndex`]), and [`AssetNumbers`] gives each of a content's
//! handles its number there, for whoever draws or plays them: the frontends
//! and the audio. The engine knows an asset by its handle alone.

use std::collections::BTreeMap;

use crate::keys;
use crate::types::SpriteNumber;

/// A kind of asset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AssetKind {
    Sprite,
    Sound,
    Banner,
    Background,
    Mugshot,
}

impl AssetKind {
    pub const ALL: [AssetKind; 5] =
        [AssetKind::Sprite, AssetKind::Sound, AssetKind::Banner, AssetKind::Background, AssetKind::Mugshot];

    /// Its resolver's name (`asset.sprite`) and the state field type that
    /// holds one (`"sprite"`).
    pub fn name(self) -> &'static str {
        match self {
            AssetKind::Sprite => "sprite",
            AssetKind::Sound => "sound",
            AssetKind::Banner => "banner",
            AssetKind::Background => "background",
            AssetKind::Mugshot => "mugshot",
        }
    }

    pub fn from_name(name: &str) -> Option<AssetKind> {
        AssetKind::ALL.into_iter().find(|k| k.name() == name)
    }
}

impl std::fmt::Display for AssetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// The assets a content names, by kind: each kind's names in byte order,
/// each once. A handle is a name's place among its kind's. The define phase
/// makes it from what the game's modules name, and the backgrounds from what
/// its `backgrounds.toml` names ([`Presentation`]).
///
/// Its hash leaves the backgrounds out: no tick reads one, and a game's
/// backgrounds are presentation (docs/design/asset-formats.md §8).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssetNames {
    pub sprites: Vec<String>,
    pub sounds: Vec<String>,
    pub banners: Vec<String>,
    pub backgrounds: Vec<String>,
    pub mugshots: Vec<String>,
}

impl std::hash::Hash for AssetNames {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        let AssetNames { sprites, sounds, banners, backgrounds: _, mugshots } = self;
        (sprites, sounds, banners, mugshots).hash(h);
    }
}

impl AssetNames {
    /// The names `named` gives, each kind's in byte order, each once.
    pub fn of(named: impl IntoIterator<Item = (AssetKind, String)>) -> AssetNames {
        let mut a = AssetNames::default();
        for (kind, name) in named {
            a.list_mut(kind).push(name);
        }
        for kind in AssetKind::ALL {
            let list = a.list_mut(kind);
            list.sort();
            list.dedup();
        }
        a
    }

    /// The names of `kind`, in handle order.
    pub fn list(&self, kind: AssetKind) -> &[String] {
        match kind {
            AssetKind::Sprite => &self.sprites,
            AssetKind::Sound => &self.sounds,
            AssetKind::Banner => &self.banners,
            AssetKind::Background => &self.backgrounds,
            AssetKind::Mugshot => &self.mugshots,
        }
    }

    fn list_mut(&mut self, kind: AssetKind) -> &mut Vec<String> {
        match kind {
            AssetKind::Sprite => &mut self.sprites,
            AssetKind::Sound => &mut self.sounds,
            AssetKind::Banner => &mut self.banners,
            AssetKind::Background => &mut self.backgrounds,
            AssetKind::Mugshot => &mut self.mugshots,
        }
    }

    /// The names of `kind`, in handle order.
    pub fn names(&self, kind: AssetKind) -> Vec<&str> {
        self.list(kind).iter().map(String::as_str).collect()
    }

    /// The handle of `kind` asset `name`.
    pub fn handle(&self, kind: AssetKind, name: &str) -> Option<u16> {
        self.list(kind).binary_search_by(|n| n.as_str().cmp(name)).ok().map(|i| i as u16)
    }

    /// The name of `kind` asset `h`.
    pub fn name(&self, kind: AssetKind, h: u16) -> Option<&str> {
        self.list(kind).get(h as usize).map(String::as_str)
    }

    /// Every name, by kind.
    pub fn all(&self) -> impl Iterator<Item = (AssetKind, &str)> {
        AssetKind::ALL.into_iter().flat_map(move |k| self.list(k).iter().map(move |n| (k, n.as_str())))
    }

    /// Whether `name` is a numbered placeholder (`sprite-0c-01`,
    /// `sound-101`): an asset nobody has named yet, which content may not
    /// use (§6.3).
    pub fn is_placeholder(kind: AssetKind, name: &str) -> bool {
        keys::local(name)
            .strip_prefix(kind.name())
            .and_then(|rest| rest.strip_prefix('-'))
            .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-'))
    }
}

/// An animation of a sprite of the content's own (its `sprites.toml`): its
/// frames' durations, in steps and in order (its last frame ends it), and
/// whether it loops (plays again from its first frame once its last is
/// over) or holds its last frame. Drawing's alone: the battle counts a
/// sprite's steps.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SpriteAnimation {
    pub frames: Vec<u8>,
    pub looping: bool,
}

/// What the define phase made of the assets content names: their names
/// (the handles).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DefinedAssets {
    pub names: AssetNames,
}

/// Each background's music (a game's `backgrounds.toml`: `acdc-area = {
/// music = "virus-battle" }`; docs/app.md §1, "The battle's music"): the
/// song a round drawn on it plays where no other is picked, by the
/// background's name and the sound's.
///
/// Presentation: no tick reads it, so it is left out of a content's hash,
/// as the custom assets are.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BackgroundMusic(pub BTreeMap<String, String>);

impl std::hash::Hash for BackgroundMusic {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

/// What a game's backgrounds and music are, as its `backgrounds.toml` and
/// `music.toml` state them (docs/design/asset-formats.md §8): read with its
/// content, from its pack's folder, and made the content's in its define
/// phase (the backgrounds' names, the custom backgrounds and songs, each
/// background's music).
///
/// Presentation: no tick reads any of it (a round's background and its
/// music are drawn and played alone), so it is left out of a content's
/// hash, and content that differs in its backgrounds or music alone plays
/// as the same.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Presentation {
    /// The background a link battle's stage is drawn on (every stage
    /// record's, the same one in each game): a round's where it states none
    /// and none is picked, and where its own isn't here.
    pub stage_background: Option<String>,
    /// The backgrounds a link battle picks one of at random (the original's
    /// table, a background there twice twice as likely).
    pub link_battle: Vec<String>,
    /// Every background the file defines, in its order: its music and, for
    /// one of the content's own, its picture.
    pub backgrounds: Vec<String>,
    /// Each background's music.
    pub music: BackgroundMusic,
    /// The backgrounds made from pictures of the content's own.
    pub custom_backgrounds: BTreeMap<String, CustomBackground>,
    /// The songs a player may pick as a battle's music, by name, in the
    /// order a list shows them.
    pub battle_themes: Vec<String>,
    /// The songs made from MIDI files of the content's own.
    pub songs: BTreeMap<String, CustomSound>,
    /// The sprites made from pictures of the content's own, and the ROMs'
    /// sprites it times otherwise (its `sprites.toml`), by name.
    pub sprites: BTreeMap<String, CustomSprite>,
}

impl std::hash::Hash for Presentation {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

/// The assets content makes from files of its own, not the ROMs'
/// (docs/design/asset-formats.md §7, "Custom assets"): each defined once,
/// with its file (a background by the game's `backgrounds.toml`, a song by
/// its `music.toml`, a sprite by its `sprites.toml`), and added to the
/// game's assets as they load, after its ROMs' (`nettai_content::custom`).
/// By name.
///
/// Presentation: no tick reads one, so it is left out of a content's hash.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CustomAssets {
    pub backgrounds: BTreeMap<String, CustomBackground>,
    pub sprites: BTreeMap<String, CustomSprite>,
    pub sounds: BTreeMap<String, CustomSound>,
}

impl std::hash::Hash for CustomAssets {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

impl CustomAssets {
    pub fn is_empty(&self) -> bool {
        self.backgrounds.is_empty() && self.sprites.is_empty() && self.sounds.is_empty()
    }
}

/// A background made from a picture of the content's own (a game's
/// `backgrounds.toml`: `[backgrounds.my-stage]`, `file = "backgrounds/my_stage.png"`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CustomBackground {
    /// The file that defines it, for what is said of it (`exe6/backgrounds.toml`).
    pub source: String,
    /// Its picture: an indexed PNG, by its pack and its path in the pack
    /// (`exe6:backgrounds/my_stage.png`; `packs::Dirs::asset_file`).
    pub file: String,
    /// How far it scrolls each frame, in 1/16 pixel (a ROM background's
    /// `scroll`).
    pub scroll: (i32, i32),
    /// How many frames its scroll takes to speed up to `scroll` from
    /// standing still (a ROM background's `speeding`); none: it scrolls at
    /// `scroll` from the first.
    pub speed_up: Option<u16>,
    /// Its pictures' durations in frames, in turn (its picture is as many,
    /// one below another); none: one picture, still.
    pub frames: Vec<u16>,
    /// Whether its pictures play again from the first after the last (else
    /// the last stays).
    pub looping: bool,
}

/// A sprite of the content's own (a game's `sprites.toml`: `[sprites.my-bomb]`,
/// `file = "sprites/my_bomb.png"`, `animations = [[6, 4, 3, 2]]`): its
/// frames, in its animations' order, each a cell of its picture, and how
/// long each shows. One without a picture is a ROM's sprite timed otherwise
/// (its animations as many, each of as many frames, as the ROM's).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CustomSprite {
    /// The file that defines it, for what is said of it (`exe6/sprites.toml`).
    pub source: String,
    /// Its frames: an indexed PNG of cells, by its pack and its path in the
    /// pack (as [`CustomBackground::file`]); none: the ROM's sprite of its
    /// name, timed by `animations`.
    pub file: Option<String>,
    /// A cell's width and height in pixels; none: the picture is one row of
    /// as many cells as the animations have frames.
    pub frame: Option<(u16, u16)>,
    /// Where in a cell the object stands, in pixels from its top left;
    /// none: the middle of its bottom edge.
    pub origin: Option<(i16, i16)>,
    /// Its shadow (every frame's first part, which an object may draw on
    /// the ground or hide): an indexed PNG, by its pack and its path, drawn
    /// centered on the object's place; none: no shadow.
    pub shadow: Option<String>,
    /// Its animations: how many frames each has, and how long each shows.
    pub animations: Vec<SpriteAnimation>,
}

/// A song made from a MIDI file of the content's own (a game's
/// `music.toml`: `[songs.my-theme]`, `file = "music/my_theme.mid"`, `voices =
/// "virus-battle"`), played on the instruments of a song the ROMs have.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CustomSound {
    /// The file that defines it, for what is said of it (`exe6/music.toml`).
    pub source: String,
    /// Its notes: a standard MIDI file, by its pack and its path in the
    /// pack (as [`CustomBackground::file`]).
    pub file: String,
    /// The ROMs' sound whose instruments it plays on, by name: its
    /// voicegroup, its music player, its priority and its reverb.
    pub voices: String,
    /// Instruments of its own, by program (0 to 127), in place of those
    /// programs of `voices`' voicegroup.
    pub instruments: BTreeMap<u8, CustomInstrument>,
}

/// An instrument made from a recording of the content's own: a Direct
/// Sound voice of a sample.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomInstrument {
    /// Its sample: a WAV file, by its pack and its path in the pack (as
    /// [`CustomBackground::file`]).
    pub file: String,
    /// The key (MIDI's numbers, 60 middle C) the recording sounds at.
    pub key: u8,
    /// Where it loops from to its end, in samples; none: it plays once.
    pub loop_start: Option<u32>,
    /// Its envelope as the driver takes a Direct Sound voice's: attack,
    /// decay, sustain and release, 0 to 255 each.
    pub envelope: [u8; 4],
}

impl Default for CustomInstrument {
    /// Middle C, no loop, and the envelope that plays the recording as it is and stops it at the note's end.
    fn default() -> CustomInstrument {
        CustomInstrument { file: String::new(), key: 60, loop_start: None, envelope: [255, 0, 255, 0] }
    }
}

/// A game's assets by unqualified name, each with its number (the extracted
/// assets' index: every asset there is, named or not).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssetIndex {
    pub sprites: BTreeMap<String, SpriteNumber>,
    pub sounds: BTreeMap<String, u16>,
    pub banners: BTreeMap<String, u8>,
    pub backgrounds: BTreeMap<String, u8>,
    pub mugshots: BTreeMap<String, u8>,
}

impl AssetIndex {
    /// The number of `kind` asset `name` (a sprite's category and
    /// index as one, `0xCCII`).
    pub fn number(&self, kind: AssetKind, name: &str) -> Option<u16> {
        match kind {
            AssetKind::Sprite => self.sprites.get(name).map(|s| u16::from_be_bytes([s.category, s.index])),
            AssetKind::Sound => self.sounds.get(name).copied(),
            AssetKind::Banner => self.banners.get(name).map(|&n| n as u16),
            AssetKind::Background => self.backgrounds.get(name).map(|&n| n as u16),
            AssetKind::Mugshot => self.mugshots.get(name).map(|&n| n as u16),
        }
    }
}

/// A content's assets as the game's assets number them: each handle's
/// number, by kind (a sprite's category and index, a sound's song-table
/// entry, a banner's, background's or mugshot's id). What a frontend draws
/// and the audio plays by; the simulation never reads it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssetNumbers {
    pub sprites: Vec<SpriteNumber>,
    pub sounds: Vec<u16>,
    pub banners: Vec<u8>,
    pub backgrounds: Vec<u8>,
    pub mugshots: Vec<u8>,
}

impl AssetNumbers {
    /// Content `names`' assets in the assets whose index is `index`; the
    /// names it hasn't, each as `kind name`, when there are any.
    pub fn new(names: &AssetNames, index: &AssetIndex) -> Result<AssetNumbers, Vec<String>> {
        let mut missing = Vec::new();
        let mut each = |kind: AssetKind| -> Vec<u16> {
            names
                .list(kind)
                .iter()
                .map(|n| {
                    index.number(kind, n).unwrap_or_else(|| {
                        missing.push(format!("{kind} {n}"));
                        0
                    })
                })
                .collect()
        };
        let sprites = each(AssetKind::Sprite).into_iter().map(|n| SpriteNumber { category: (n >> 8) as u8, index: n as u8 }).collect();
        let sounds = each(AssetKind::Sound);
        let narrow = |v: Vec<u16>| v.into_iter().map(|n| n as u8).collect();
        let banners = narrow(each(AssetKind::Banner));
        let backgrounds = narrow(each(AssetKind::Background));
        let mugshots = narrow(each(AssetKind::Mugshot));
        if !missing.is_empty() {
            return Err(missing);
        }
        Ok(AssetNumbers { sprites, sounds, banners, backgrounds, mugshots })
    }

    /// Sprite handle `h`'s sprite.
    pub fn sprite(&self, h: u16) -> Option<SpriteNumber> {
        self.sprites.get(h as usize).copied()
    }

    /// Sound handle `h`'s song-table entry.
    pub fn sound(&self, h: u16) -> Option<u16> {
        self.sounds.get(h as usize).copied()
    }

    /// `kind` asset `h`'s number (a sprite's category and index
    /// as one, `0xCCII`).
    pub fn number(&self, kind: AssetKind, h: u16) -> Option<u16> {
        let i = h as usize;
        match kind {
            AssetKind::Sprite => self.sprites.get(i).map(|s| u16::from_be_bytes([s.category, s.index])),
            AssetKind::Sound => self.sounds.get(i).copied(),
            AssetKind::Banner => self.banners.get(i).map(|&n| n as u16),
            AssetKind::Background => self.backgrounds.get(i).map(|&n| n as u16),
            AssetKind::Mugshot => self.mugshots.get(i).map(|&n| n as u16),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A kind's handles are its names' places in byte order, each name
    /// once; the assets' index gives each its number, and a name it hasn't is said.
    #[test]
    fn handles_are_the_names_places() {
        let names = AssetNames::of([
            (AssetKind::Sprite, "bomb".to_string()),
            (AssetKind::Sound, "throw".to_string()),
            (AssetKind::Sprite, "aqua-tower".to_string()),
            (AssetKind::Sprite, "bomb".to_string()),
        ]);
        assert_eq!(names.sprites, ["aqua-tower", "bomb"]);
        assert_eq!(names.handle(AssetKind::Sprite, "bomb"), Some(1));
        assert_eq!(names.handle(AssetKind::Sound, "bomb"), None);
        assert_eq!(names.name(AssetKind::Sound, 0), Some("throw"));
        let mut index = AssetIndex::default();
        index.sprites.insert("bomb".into(), SpriteNumber { category: 0x0C, index: 2 });
        index.sprites.insert("aqua-tower".into(), SpriteNumber { category: 0x10, index: 1 });
        index.sprites.insert("unused".into(), SpriteNumber { category: 0x10, index: 9 });
        assert_eq!(AssetNumbers::new(&names, &index), Err(vec!["sound throw".to_string()]));
        index.sounds.insert("throw".into(), 0xB2);
        let numbers = AssetNumbers::new(&names, &index).unwrap();
        assert_eq!(numbers.sprite(1), Some(SpriteNumber { category: 0x0C, index: 2 }));
        assert_eq!(numbers.number(AssetKind::Sprite, 0), Some(0x1001));
        assert_eq!(numbers.sound(0), Some(0xB2));
    }
}
