//! Flag bytes in content files: a list of the set bits' names, with any
//! bit that has no name as its number (`["has_damage", 32]`). Nothing is
//! derived: the value is exactly the bits the list names.

use serde::de::{self, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserializer, Serializer};

/// A named bit or a raw one, as listed in a file.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum Bit {
    Name(String),
    Number(u32),
}

pub(crate) fn serialize<S: Serializer>(bits: u32, names: &[(u32, &str)], s: S) -> Result<S::Ok, S::Error> {
    let named: u32 = names.iter().map(|&(b, _)| b).fold(0, |a, b| a | b);
    let mut seq = s.serialize_seq(None)?;
    for &(bit, name) in names {
        if bits & bit != 0 {
            seq.serialize_element(name)?;
        }
    }
    for k in 0..32 {
        let bit = 1u32 << k;
        if bits & bit != 0 && named & bit == 0 {
            seq.serialize_element(&bit)?;
        }
    }
    seq.end()
}

pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D, names: &'static [(u32, &'static str)], max: u32) -> Result<u32, D::Error> {
    struct V {
        names: &'static [(u32, &'static str)],
        max: u32,
    }
    impl<'de> Visitor<'de> for V {
        type Value = u32;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            let names: Vec<&str> = self.names.iter().map(|n| n.1).collect();
            write!(f, "a list of flag names ({}) or bit values", names.join(", "))
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<u32, A::Error> {
            let mut v = 0u32;
            while let Some(bit) = seq.next_element::<Bit>()? {
                v |= match bit {
                    Bit::Name(n) => match self.names.iter().find(|x| x.1 == n) {
                        Some(&(b, _)) => b,
                        None => return Err(de::Error::custom(format!("unknown flag {n:?}"))),
                    },
                    Bit::Number(b) if b.count_ones() == 1 && b <= self.max => b,
                    Bit::Number(b) => return Err(de::Error::custom(format!("{b:#x} is not a single bit that fits"))),
                };
            }
            Ok(v)
        }
    }
    d.deserialize_seq(V { names, max })
}

/// `impl Serialize, Deserialize` for a flags newtype `T(pub int)` with a
/// `NAMES` table.
macro_rules! serde_flags {
    ($t:ty, $int:ty) => {
        impl serde::Serialize for $t {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                crate::content::flags::serialize(self.0 as u32, <$t>::NAMES, s)
            }
        }
        impl<'de> serde::Deserialize<'de> for $t {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<$t, D::Error> {
                crate::content::flags::deserialize(d, <$t>::NAMES, <$int>::MAX as u32).map(|v| Self(v as $int))
            }
        }
    };
}

pub(crate) use serde_flags;
