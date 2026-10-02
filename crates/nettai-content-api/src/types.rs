//! Value types shared by the core and content: object handles, fixed-point
//! positions, panels, sprite ids.

/// Which pool an object lives in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Pool {
    /// Actors: navis, and large chip objects (the game's type 1).
    Actor,
    /// Attacks and hit regions (type 3).
    Attack,
    /// Effects and helpers (type 4).
    Effect,
}

impl Pool {
    pub const ALL: [Pool; 3] = [Pool::Actor, Pool::Attack, Pool::Effect];

    /// The pool's name in content ("actor", "attack", "effect").
    pub fn name(self) -> &'static str {
        match self {
            Pool::Actor => "actor",
            Pool::Attack => "attack",
            Pool::Effect => "effect",
        }
    }

    pub fn from_name(name: &str) -> Option<Pool> {
        Pool::ALL.into_iter().find(|p| p.name() == name)
    }
}

/// A handle to an object slot. Like the game's pointers, a handle can
/// outlive the object it named.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectRef {
    pub pool: Pool,
    pub slot: u8,
}

/// A 16.16 fixed-point position or velocity, relative to the field's center.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Vec3 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Vec3 {
    /// A position in whole pixels.
    pub const fn px(x: i32, y: i32, z: i32) -> Vec3 {
        Vec3 { x: x << 16, y: y << 16, z: z << 16 }
    }

    /// Component-wise wrapping sum (the game's 32-bit adds).
    pub fn wrapping_add(self, o: Vec3) -> Vec3 {
        Vec3 { x: self.x.wrapping_add(o.x), y: self.y.wrapping_add(o.y), z: self.z.wrapping_add(o.z) }
    }

    /// Component-wise wrapping difference.
    pub fn wrapping_sub(self, o: Vec3) -> Vec3 {
        Vec3 { x: self.x.wrapping_sub(o.x), y: self.y.wrapping_sub(o.y), z: self.z.wrapping_sub(o.z) }
    }
}

/// A panel coordinate: x 1..=6, y 1..=3 on the field (0 and 7/4 are the
/// border).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PanelPos {
    pub x: u8,
    pub y: u8,
}

/// A sprite, as the engine names it: a handle, its place among the loaded
/// packs' sprite names (`AssetNames::sprites`, qualified by pack, in byte
/// order; docs/design/rules-in-luau.md §7.4). What a pack calls it (its
/// category and index) is [`PackSprite`], which loaders, frontends and
/// compat read through the asset names; the simulation never does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct SpriteId(pub u16);

impl SpriteId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A sprite as its pack holds it: (category, index) into the game's
/// sprite table. Written `"CC-II"` in hex, as its `sprite.json` in a
/// content pack holds it (its folder there is its name).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackSprite {
    pub category: u8,
    pub index: u8,
}

impl std::fmt::Display for PackSprite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02x}-{:02x}", self.category, self.index)
    }
}

impl std::str::FromStr for PackSprite {
    type Err = String;
    fn from_str(s: &str) -> Result<PackSprite, String> {
        let bad = || format!("{s:?} is not a sprite id (CC-II in hex)");
        let (c, i) = s.split_once('-').ok_or_else(bad)?;
        if c.len() != 2 || i.len() != 2 {
            return Err(bad());
        }
        let hex = |t: &str| u8::from_str_radix(t, 16).map_err(|_| bad());
        Ok(PackSprite { category: hex(c)?, index: hex(i)? })
    }
}

impl serde::Serialize for PackSprite {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for PackSprite {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<PackSprite, D::Error> {
        let s = <String as serde::Deserialize>::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// A loaded asset pack, by its place among the content's packs (their
/// games in byte order: `AssetNames::packs`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackId(pub u8);

impl PackId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// An asset as its pack holds it: the pack, and the pack's own number for
/// it (a sprite's category and index, a sound's song-table entry, a
/// banner's, a background's, a mugshot's number).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InPack<T> {
    pub pack: PackId,
    pub id: T,
}
