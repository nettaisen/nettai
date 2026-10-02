//! Assets by name (docs/design/content-model-v2.md §6.3): content names a
//! sprite, a sound, a banner, a background or a mugshot
//! (`asset.sprite("bomb")`), and the name resolves while content loads to
//! an asset handle, its place among its kind's names in byte-wise order.
//! The engine keeps each name's asset as it identifies it today (a sprite's
//! category and index, a sound's number); that becomes an asset handle of
//! its own when the pack stores assets by name (§12, step 6).

use std::collections::BTreeMap;

use crate::types::SpriteId;

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

/// The assets content can name, by kind and name, with the engine's
/// identity for each.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AssetNames {
    pub sprites: BTreeMap<String, SpriteId>,
    pub sounds: BTreeMap<String, u16>,
    pub banners: BTreeMap<String, u8>,
    pub backgrounds: BTreeMap<String, u8>,
    pub mugshots: BTreeMap<String, u8>,
}

impl AssetNames {
    /// The names of `kind`, in handle order.
    pub fn names(&self, kind: AssetKind) -> Vec<&str> {
        fn keys<V>(m: &BTreeMap<String, V>) -> Vec<&str> {
            m.keys().map(String::as_str).collect()
        }
        match kind {
            AssetKind::Sprite => keys(&self.sprites),
            AssetKind::Sound => keys(&self.sounds),
            AssetKind::Banner => keys(&self.banners),
            AssetKind::Background => keys(&self.backgrounds),
            AssetKind::Mugshot => keys(&self.mugshots),
        }
    }

    /// The handle of `kind` asset `name`.
    pub fn handle(&self, kind: AssetKind, name: &str) -> Option<u16> {
        fn place<V>(m: &BTreeMap<String, V>, name: &str) -> Option<u16> {
            m.keys().position(|k| k == name).map(|i| i as u16)
        }
        match kind {
            AssetKind::Sprite => place(&self.sprites, name),
            AssetKind::Sound => place(&self.sounds, name),
            AssetKind::Banner => place(&self.banners, name),
            AssetKind::Background => place(&self.backgrounds, name),
            AssetKind::Mugshot => place(&self.mugshots, name),
        }
    }

    /// Sprite handle `h`'s sprite.
    pub fn sprite(&self, h: u16) -> Option<SpriteId> {
        self.sprites.values().nth(h as usize).copied()
    }

    /// Sound handle `h`'s sound number.
    pub fn sound(&self, h: u16) -> Option<u16> {
        self.sounds.values().nth(h as usize).copied()
    }

    /// Banner, background or mugshot handle `h`'s number.
    pub fn number(&self, kind: AssetKind, h: u16) -> Option<u16> {
        let m = match kind {
            AssetKind::Sprite => return self.sprite(h).map(|s| u16::from_be_bytes([s.category, s.index])),
            AssetKind::Sound => return self.sound(h),
            AssetKind::Banner => &self.banners,
            AssetKind::Background => &self.backgrounds,
            AssetKind::Mugshot => &self.mugshots,
        };
        m.values().nth(h as usize).map(|&n| n as u16)
    }

    /// Whether `name` is a numbered placeholder (`sprite-0c-01`,
    /// `sound-101`): an asset nobody has named yet, which content may not
    /// use (§6.3).
    pub fn is_placeholder(kind: AssetKind, name: &str) -> bool {
        name.strip_prefix(kind.name())
            .and_then(|rest| rest.strip_prefix('-'))
            .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-'))
    }
}
