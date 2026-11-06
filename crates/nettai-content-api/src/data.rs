//! [`Data`]: plain data trees that cross between a runtime and the engine:
//! the definitions content makes, as the define phase reads them back
//! ([`crate::definitions`]).

use crate::assets::AssetKind;
use crate::registry::Registry;

/// A value of a definition's spec.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Data {
    Nil,
    Bool(bool),
    Int(i64),
    Str(String),
    /// A sequence (1-based in Luau).
    List(Vec<Data>),
    /// A table by keys (ids, names), in key order.
    Map(Vec<(Key, Data)>),
    /// Another definition, by registry and key (in definitions only).
    Ref(Registry, String),
    /// An asset, by kind and name (`asset.sprite("bomb")`; in definitions
    /// only).
    Asset(AssetKind, String),
    /// A function: the definition's function slot at this place (in
    /// definitions only; the runtime keeps the function itself).
    Function,
    /// A field's type, as a state's declaration gives it (`schema.u16`, a
    /// record type, `schema.list(T, n)`; in definitions only).
    Type(crate::state::TypeData),
}

/// A table key.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Key {
    Int(i64),
    Str(String),
}

impl From<&str> for Key {
    fn from(s: &str) -> Key {
        Key::Str(s.to_string())
    }
}

impl From<i64> for Key {
    fn from(i: i64) -> Key {
        Key::Int(i)
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Key::Int(i) => write!(f, "{i}"),
            Key::Str(s) => f.write_str(s),
        }
    }
}

impl Data {
    /// A table from (key, value) pairs.
    pub fn map<K: Into<Key>>(entries: impl IntoIterator<Item = (K, Data)>) -> Data {
        Data::Map(entries.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    /// The field `name` of a table (Nil if absent or not a table).
    pub fn field(&self, name: &str) -> &Data {
        match self {
            Data::Map(entries) => entries
                .iter()
                .find(|(k, _)| matches!(k, Key::Str(s) if s == name))
                .map_or(&Data::Nil, |(_, v)| v),
            _ => &Data::Nil,
        }
    }

    /// Item `i` (from 1, as in Luau) of a list (Nil if absent or not a
    /// list).
    pub fn item(&self, i: usize) -> &Data {
        match self {
            Data::List(items) => i.checked_sub(1).and_then(|i| items.get(i)).unwrap_or(&Data::Nil),
            _ => &Data::Nil,
        }
    }

    /// The string, if this is one.
    pub fn str(&self) -> Option<&str> {
        match self {
            Data::Str(s) => Some(s),
            _ => None,
        }
    }

    /// The integer, if this is one.
    pub fn int(&self) -> Option<i64> {
        match self {
            Data::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn is_nil(&self) -> bool {
        matches!(self, Data::Nil)
    }

    /// A set of names, as content states one: a table of booleans keyed by
    /// member (`{ solid = true, holy = true }`), a member set to false the
    /// same as one left out. Its members (those set to true), in name order;
    /// `{}` none. A key that is no name, or a value that is no boolean, is an
    /// error (what is wrong, for the caller to place).
    pub fn set(&self) -> Result<Vec<&str>, String> {
        let (holds, _) = self.condition()?;
        Ok(holds)
    }

    /// A condition on named members, as content states one: a table of
    /// booleans keyed by member, true where the member must hold and false
    /// where it must not (`{ solid = true, occupied = false }`). Its members
    /// that must hold, and those that must not, each in name order; `{}`
    /// none. A key that is no name, or a value that is no boolean, is an
    /// error.
    pub fn condition(&self) -> Result<(Vec<&str>, Vec<&str>), String> {
        let entries = match self {
            Data::Map(entries) => entries.as_slice(),
            // (An empty table reads as an empty list.)
            Data::List(items) if items.is_empty() => &[],
            Data::List(_) => return Err("is a table of booleans keyed by name (`{ name = true }`), not a list".into()),
            other => return Err(format!("is a table of booleans keyed by name (`{{ name = true }}`), not {}", other.kind())),
        };
        let (mut holds, mut not) = (Vec::new(), Vec::new());
        for (k, v) in entries {
            let Key::Str(name) = k else {
                return Err(format!("has the key {k}, no name: a table of booleans keyed by name (`{{ name = true }}`)"));
            };
            match v {
                Data::Bool(true) => holds.push(name.as_str()),
                Data::Bool(false) => not.push(name.as_str()),
                other => return Err(format!("`{name}` is {}, not true or false", other.kind())),
            }
        }
        Ok((holds, not))
    }

    /// What kind of value this is, for messages.
    pub fn kind(&self) -> &'static str {
        match self {
            Data::Nil => "nil",
            Data::Bool(_) => "a boolean",
            Data::Int(_) => "a number",
            Data::Str(_) => "a string",
            Data::List(_) => "a list",
            Data::Map(_) => "a table",
            Data::Ref(..) => "a definition",
            Data::Asset(..) => "an asset",
            Data::Function => "a function",
            Data::Type(_) => "a field's type",
        }
    }
}
