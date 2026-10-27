//! Assets by name (docs/design/content-model-v2.md §6.3, rules-in-luau.md
//! §7.4): content names a sprite, a sound, a banner, a background or a
//! mugshot, and the define phase interns the names to asset handles, each
//! its place among its kind's names in byte order ([`AssetNames`]). The
//! names are the content's alone: a sprite is the one its module defines
//! (`asset.sprite("bomb", { { 3, 3, 3 } })`: its name, and the timing of
//! its animations, [`SpriteDef`]), any other asset the one a module names
//! (`asset.sound("throw")`). So a battle needs no asset pack: which assets
//! there are, their handles and the sprites' timing are all content.
//!
//! A name is also what the game's asset pack holds the asset under (a
//! match plays one game, so a name is its game's: docs/design/
//! content-model-v2.md §4.0). What the pack calls each (a sprite's
//! category and index, a sound's song-table entry) is the pack's index
//! ([`AssetIndex`], its `assets.toml`), and [`AssetNumbers`] gives each of a
//! content's handles its number in a pack, for whoever draws or plays them:
//! the frontends and the audio. The engine knows an asset by its handle
//! alone.

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
/// makes it from what the game's modules name.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AssetNames {
    pub sprites: Vec<String>,
    pub sounds: Vec<String>,
    pub banners: Vec<String>,
    pub backgrounds: Vec<String>,
    pub mugshots: Vec<String>,
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

/// A sprite as content defines it (`asset.sprite(name, animations,
/// { parts = ... })`): its animations as the battle times them, and where
/// the parts content reads sit.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SpriteDef {
    pub animations: Vec<SpriteAnimation>,
    /// The parts content reads (`sprite:part_offset(n)`), by part number
    /// (from 0): where the part sits in each frame (by animation, then
    /// frame), in pixels from the object, unflipped. A sprite states those
    /// alone; a part it doesn't state is at (0, 0).
    pub parts: BTreeMap<u8, Vec<Vec<(i8, i8)>>>,
}

/// An animation: its frames' durations, in ticks and in order (its last
/// frame ends it), and whether it loops (plays again from its first frame
/// once its last is over) or holds its last frame.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SpriteAnimation {
    pub frames: Vec<u8>,
    pub looping: bool,
}

/// What the define phase made of the assets content names: their names
/// (the handles), and each sprite's definition, by handle.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DefinedAssets {
    pub names: AssetNames,
    pub sprites: Vec<SpriteDef>,
}

/// One pack's own index: its assets by unqualified name (as its
/// `assets.toml` holds them), each with the pack's number for it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssetIndex {
    pub sprites: BTreeMap<String, SpriteNumber>,
    pub sounds: BTreeMap<String, u16>,
    pub banners: BTreeMap<String, u8>,
    pub backgrounds: BTreeMap<String, u8>,
    pub mugshots: BTreeMap<String, u8>,
}

impl AssetIndex {
    /// The pack's number for `kind` asset `name` (a sprite's category and
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

/// A content's assets as a pack holds them: each handle's number in the
/// pack, by kind (a sprite's category and index, a sound's song-table
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
    /// Content `names`' assets in the pack whose index is `index`; the
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

    /// Sprite handle `h`'s sprite in the pack.
    pub fn sprite(&self, h: u16) -> Option<SpriteNumber> {
        self.sprites.get(h as usize).copied()
    }

    /// Sound handle `h`'s song-table entry.
    pub fn sound(&self, h: u16) -> Option<u16> {
        self.sounds.get(h as usize).copied()
    }

    /// `kind` asset `h`'s number in the pack (a sprite's category and index
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
    /// once; a pack gives each its number, and a name it hasn't is said.
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
        let pack = AssetNumbers::new(&names, &index).unwrap();
        assert_eq!(pack.sprite(1), Some(SpriteNumber { category: 0x0C, index: 2 }));
        assert_eq!(pack.number(AssetKind::Sprite, 0), Some(0x1001));
        assert_eq!(pack.sound(0), Some(0xB2));
    }
}
