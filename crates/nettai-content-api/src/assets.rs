//! Assets by name (docs/design/content-model-v2.md §6.3, rules-in-luau.md
//! §7.4): content names a sprite, a sound, a banner, a background or a
//! mugshot (`asset.sprite("bomb")`), and the name resolves while content
//! loads to an asset handle, its place among its kind's names in byte-wise
//! order. Content loads from several packs at once (one a game): every name
//! is qualified with its pack's game (`bn6:bomb`), as the loader qualifies
//! definitions' keys, so the handles cover every loaded pack. The engine
//! knows an asset by its handle alone; what the pack calls it (a sprite's
//! category and index, a sound's song-table entry) is here, for loaders,
//! frontends, the audio and compat.

use std::collections::BTreeMap;

use crate::keys;
use crate::types::{InPack, PackId, PackSprite};

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

/// The assets content can name, by kind and qualified name (`bn6:bomb`),
/// each with its pack and the pack's own number for it. A kind's handles
/// are its names' places in byte order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AssetNames {
    /// The loaded packs' games, by [`PackId`] (in byte order).
    pub packs: Vec<String>,
    pub sprites: BTreeMap<String, InPack<PackSprite>>,
    pub sounds: BTreeMap<String, InPack<u16>>,
    pub banners: BTreeMap<String, InPack<u8>>,
    pub backgrounds: BTreeMap<String, InPack<u8>>,
    pub mugshots: BTreeMap<String, InPack<u8>>,
}

/// One pack's own index: its assets by unqualified name (as its
/// `assets.toml` holds them).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PackIndex {
    pub sprites: BTreeMap<String, PackSprite>,
    pub sounds: BTreeMap<String, u16>,
    pub banners: BTreeMap<String, u8>,
    pub backgrounds: BTreeMap<String, u8>,
    pub mugshots: BTreeMap<String, u8>,
}

impl AssetNames {
    /// The assets of `packs` (each its game and its index), by their names
    /// in their packs (a match plays one game: one pack, whose names meet no
    /// other's).
    pub fn of_packs(packs: Vec<(String, PackIndex)>) -> AssetNames {
        let mut packs = packs;
        packs.sort_by(|a, b| a.0.cmp(&b.0));
        let mut a = AssetNames { packs: packs.iter().map(|(g, _)| g.clone()).collect(), ..Default::default() };
        for (i, (game, index)) in packs.into_iter().enumerate() {
            let pack = PackId(i as u8);
            fn add<T>(game: &str, pack: PackId, from: BTreeMap<String, T>, to: &mut BTreeMap<String, InPack<T>>) {
                for (name, id) in from {
                    let _ = game;
                    to.insert(name, InPack { pack, id });
                }
            }
            add(&game, pack, index.sprites, &mut a.sprites);
            add(&game, pack, index.sounds, &mut a.sounds);
            add(&game, pack, index.banners, &mut a.banners);
            add(&game, pack, index.backgrounds, &mut a.backgrounds);
            add(&game, pack, index.mugshots, &mut a.mugshots);
        }
        a
    }

    /// One pack's assets (a game's alone).
    pub fn of_pack(game: &str, index: PackIndex) -> AssetNames {
        AssetNames::of_packs(vec![(game.to_string(), index)])
    }

    /// The pack whose game is `game`.
    pub fn pack(&self, game: &str) -> Option<PackId> {
        self.packs.iter().position(|g| g == game).map(|i| PackId(i as u8))
    }

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

    /// Sprite handle `h`'s pack and sprite there.
    pub fn sprite(&self, h: u16) -> Option<InPack<PackSprite>> {
        self.sprites.values().nth(h as usize).copied()
    }

    /// Sound handle `h`'s pack and song-table entry there.
    pub fn sound(&self, h: u16) -> Option<InPack<u16>> {
        self.sounds.values().nth(h as usize).copied()
    }

    /// Banner, background or mugshot handle `h`'s pack and number there.
    pub fn number(&self, kind: AssetKind, h: u16) -> Option<InPack<u16>> {
        let wide = |a: &InPack<u8>| InPack { pack: a.pack, id: a.id as u16 };
        match kind {
            AssetKind::Sprite => self.sprite(h).map(|s| InPack { pack: s.pack, id: u16::from_be_bytes([s.id.category, s.id.index]) }),
            AssetKind::Sound => self.sound(h),
            AssetKind::Banner => self.banners.values().nth(h as usize).map(wide),
            AssetKind::Background => self.backgrounds.values().nth(h as usize).map(wide),
            AssetKind::Mugshot => self.mugshots.values().nth(h as usize).map(wide),
        }
    }

    /// The handle of pack `pack`'s sprite `id` (the first name it has).
    pub fn sprite_handle(&self, pack: PackId, id: PackSprite) -> Option<u16> {
        self.sprites.values().position(|a| a.pack == pack && a.id == id).map(|i| i as u16)
    }

    /// The handle of pack `pack`'s sound `number`.
    pub fn sound_handle(&self, pack: PackId, number: u16) -> Option<u16> {
        self.sounds.values().position(|a| a.pack == pack && a.id == number).map(|i| i as u16)
    }

    /// The handle of pack `pack`'s banner, background or mugshot `number`.
    pub fn number_handle(&self, kind: AssetKind, pack: PackId, number: u16) -> Option<u16> {
        let find = |m: &BTreeMap<String, InPack<u8>>| m.values().position(|a| a.pack == pack && a.id as u16 == number).map(|i| i as u16);
        match kind {
            AssetKind::Sprite => self.sprite_handle(pack, PackSprite { category: (number >> 8) as u8, index: number as u8 }),
            AssetKind::Sound => self.sound_handle(pack, number),
            AssetKind::Banner => find(&self.banners),
            AssetKind::Background => find(&self.backgrounds),
            AssetKind::Mugshot => find(&self.mugshots),
        }
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
