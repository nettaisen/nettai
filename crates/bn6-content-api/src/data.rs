//! [`Data`]: the content pack's data as scripts read it (the `data`
//! global), a plain tree the engine builds from the pack and a runtime
//! turns into its own read-only values.

/// A value of the pack's data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Data {
    Nil,
    Bool(bool),
    Int(i64),
    Str(String),
    /// A sequence (1-based in Luau).
    List(Vec<Data>),
    /// A table by keys (ids, names).
    Map(Vec<(Key, Data)>),
}

/// A table key.
#[derive(Clone, Debug, PartialEq, Eq)]
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

impl Data {
    /// A table from (key, value) pairs.
    pub fn map<K: Into<Key>>(entries: impl IntoIterator<Item = (K, Data)>) -> Data {
        Data::Map(entries.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }
}
