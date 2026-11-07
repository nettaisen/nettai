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
    /// A name its game defines, by kind (`PanelType.cracked`, `Family.sword`;
    /// in definitions only): one of the game's, which the engine reads.
    Name(NameKind, String),
}

/// The kinds of names a game defines (docs/design/scripting.md §2.1), each
/// a value table content names them by (`PanelType.cracked`), built from
/// the game loaded: what its rules, its builds and its root say it has.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NameKind {
    /// A panel type: one of its rules' `panels.numbers`.
    PanelType,
    /// A chip family: one of its rules' `elements.families`.
    Family,
    /// An emotion: one of its rules' status section's `emotion` order.
    Emotion,
    /// A version: one of its builds' `groups`.
    Version,
    /// A custom-screen window of the rules': one of their `windows`.
    Window,
    /// A custom-screen button of the rules': one of their `buttons`.
    Button,
    /// A collection: one of its root's (`patch_cards`).
    Collection,
}

impl NameKind {
    pub const ALL: [NameKind; 7] =
        [NameKind::PanelType, NameKind::Family, NameKind::Emotion, NameKind::Version, NameKind::Window, NameKind::Button, NameKind::Collection];

    /// The value table's name (`PanelType`).
    pub fn table(self) -> &'static str {
        match self {
            NameKind::PanelType => "PanelType",
            NameKind::Family => "Family",
            NameKind::Emotion => "Emotion",
            NameKind::Version => "Version",
            NameKind::Window => "Window",
            NameKind::Button => "Button",
            NameKind::Collection => "Collection",
        }
    }

    /// What a name of this kind is, for messages.
    pub fn what(self) -> &'static str {
        match self {
            NameKind::PanelType => "panel type",
            NameKind::Family => "chip family",
            NameKind::Emotion => "emotion",
            NameKind::Version => "version",
            NameKind::Window => "window of the rules'",
            NameKind::Button => "button of the rules'",
            NameKind::Collection => "collection",
        }
    }
}

/// The names a game defines, of each kind, in its order (a panel type's
/// place is its number): what the value tables hold.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct GameNames(pub Vec<(NameKind, Vec<String>)>);

impl GameNames {
    /// The names of `kind` (none: the game has none).
    pub fn of(&self, kind: NameKind) -> &[String] {
        self.0.iter().find(|(k, _)| *k == kind).map_or(&[], |(_, n)| n.as_slice())
    }

    /// Whether `name` is one of the game's of `kind`.
    pub fn has(&self, kind: NameKind, name: &str) -> bool {
        self.of(kind).iter().any(|n| n == name)
    }

    /// Set the names of `kind`.
    pub fn set(&mut self, kind: NameKind, names: Vec<String>) {
        self.0.retain(|(k, _)| *k != kind);
        self.0.push((kind, names));
        self.0.sort_by_key(|(k, _)| *k);
    }
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
            Data::Name(..) => "a game's name",
        }
    }
}
