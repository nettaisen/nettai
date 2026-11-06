//! Sets of names in content files: a table of booleans keyed by member
//! (`{ dimming = true, has_damage = true }`), a member set to false the same
//! as one left out; and conditions, the same table where false means the
//! member must not hold (`{ solid = true, occupied = false }`). A flag word
//! whose every bit content may set has a name reads as the bits its set
//! names; nothing is derived. A key that is no name the set has, or a value
//! that is no boolean, is an error.

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serializer};

pub(crate) fn serialize<S: Serializer>(bits: u32, names: &[(u32, &str)], s: S) -> Result<S::Ok, S::Error> {
    let mut map = s.serialize_map(None)?;
    for &(bit, name) in names {
        if bits & bit != 0 {
            map.serialize_entry(name, &true)?;
        }
    }
    map.end()
}

/// A member's value in a set: true or false.
struct Member(bool);

impl<'de> Deserialize<'de> for Member {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Member, D::Error> {
        struct V;
        impl Visitor<'_> for V {
            type Value = Member;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("true or false")
            }
            fn visit_bool<E: de::Error>(self, b: bool) -> Result<Member, E> {
                Ok(Member(b))
            }
        }
        d.deserialize_bool(V)
    }
}

/// A set's or a condition's members: (those set to true, those set to
/// false), each in the order the table gives them.
fn members<'de, D: Deserializer<'de>>(d: D) -> Result<(Vec<String>, Vec<String>), D::Error> {
    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = (Vec<String>, Vec<String>);
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a table of booleans keyed by name (`{ name = true }`)")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let (mut holds, mut not) = (Vec::new(), Vec::new());
            while let Some(name) = map.next_key::<String>()? {
                let Member(on) = map.next_value()?;
                if on { holds.push(name) } else { not.push(name) }
            }
            Ok((holds, not))
        }
        // (Luau's `{}` reads as an empty list.)
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            if seq.next_element::<de::IgnoredAny>()?.is_some() {
                return Err(de::Error::custom("a set is a table of booleans keyed by name (`{ name = true }`), not a list"));
            }
            Ok((Vec::new(), Vec::new()))
        }
    }
    d.deserialize_any(V)
}

/// A set of names (`{ name = true }`): its members.
pub(crate) fn set<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    Ok(members(d)?.0)
}

/// A condition (`{ solid = true, occupied = false }`): the members that
/// must hold, and those that must not.
pub(crate) fn condition<'de, D: Deserializer<'de>>(d: D) -> Result<(Vec<String>, Vec<String>), D::Error> {
    members(d)
}

pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D, names: &'static [(u32, &'static str)]) -> Result<u32, D::Error> {
    let mut v = 0u32;
    for n in set(d)? {
        v |= match names.iter().find(|x| x.1 == n) {
            Some(&(b, _)) => b,
            None => {
                let names: Vec<&str> = names.iter().map(|n| n.1).collect();
                return Err(de::Error::custom(format!("unknown flag {n:?} (the flags: {})", names.join(", "))));
            }
        };
    }
    Ok(v)
}

/// `impl Serialize, Deserialize` for a flag type with a `NAMES` table: a set
/// of its names (`{ name = true }`). The type is a `bitflags` type
/// (`serde_flags!(T)`), or a newtype of one that names a part of its bits
/// (`serde_flags!(T(Inner))`).
macro_rules! serde_flags {
    ($t:ident) => {
        impl serde::Serialize for $t {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                crate::content::flags::serialize(self.bits().into(), <$t>::NAMES, s)
            }
        }
        impl<'de> serde::Deserialize<'de> for $t {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<$t, D::Error> {
                // (The names' bits are the type's own: they fit.)
                crate::content::flags::deserialize(d, <$t>::NAMES).map(|v| <$t>::from_bits_retain(v as _))
            }
        }
    };
    ($t:ty, $int:ty) => {
        impl serde::Serialize for $t {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                crate::content::flags::serialize(self.0 as u32, <$t>::NAMES, s)
            }
        }
        impl<'de> serde::Deserialize<'de> for $t {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<$t, D::Error> {
                crate::content::flags::deserialize(d, <$t>::NAMES).map(|v| Self(v as $int))
            }
        }
    };
    ($t:ident($inner:ty)) => {
        impl serde::Serialize for $t {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                crate::content::flags::serialize(self.0.bits().into(), <$t>::NAMES, s)
            }
        }
        impl<'de> serde::Deserialize<'de> for $t {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<$t, D::Error> {
                crate::content::flags::deserialize(d, <$t>::NAMES).map(|v| $t(<$inner>::from_bits_retain(v as _)))
            }
        }
    };
}

pub(crate) use serde_flags;
