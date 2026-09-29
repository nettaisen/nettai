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

    /// The game's type number (1, 3, 4).
    pub fn type_number(self) -> u8 {
        match self {
            Pool::Actor => 1,
            Pool::Attack => 3,
            Pool::Effect => 4,
        }
    }

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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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

/// A sprite: (category, index) into the game's sprite table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpriteId {
    pub category: u8,
    pub index: u8,
}
