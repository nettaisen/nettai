//! Reading a definition's spec as typed data (serde's form): the record a
//! chip, navi or form definition makes, and the rule sections
//! (`sections`). Assets read as the engine identifies them, a lock-on mode
//! by its handle, a chip by its key (the registry resolves it to a handle).


use nettai_content_api::{AssetNames, ContentError, Data, DataKey, Definition, Definitions, Registry};
use serde::de::DeserializeOwned;
use nettai_content_api::{Choice, ChoiceKind};
use serde_json::{Map, Value as Json};


pub(crate) fn err(d: &Definition, e: impl std::fmt::Display) -> ContentError {
    ContentError::new(format!("{}.luau: {} {}: {e}", d.module, d.registry, d.key))
}

/// How the definitions' values read as typed data: assets as the engine
/// identifies them, a chip as its key.
pub struct SpecReader<'a> {
    assets: &'a AssetNames,
}

impl<'a> SpecReader<'a> {
    /// A reader for the values of `definitions`.
    pub fn new(assets: &'a AssetNames, _definitions: &Definitions) -> SpecReader<'a> {
        SpecReader { assets }
    }

    /// `d` as the data a record reads (serde's form).
    pub fn json(&self, d: &Data, at: &str) -> Result<Json, String> {
        Ok(match d {
            Data::Nil => Json::Null,
            Data::Bool(b) => Json::Bool(*b),
            Data::Int(i) => Json::from(*i),
            Data::Str(s) => Json::String(s.clone()),
            Data::List(items) => {
                let mut out = Vec::with_capacity(items.len());
                for (i, x) in items.iter().enumerate() {
                    out.push(self.json(x, &format!("{at}[{}]", i + 1))?);
                }
                Json::Array(out)
            }
            // An empty table is an empty list (Luau can't tell them apart).
            Data::Map(entries) if entries.is_empty() => Json::Array(Vec::new()),
            Data::Map(entries) => {
                let mut out = Map::new();
                for (k, v) in entries {
                    out.insert(k.to_string(), self.json(v, &format!("{at}.{k}"))?);
                }
                Json::Object(out)
            }
            // A chip by its key: the registry resolves it to a handle
            // (no chip has a number the tables hold).
            Data::Ref(Registry::Chip, key) => Json::String(key.clone()),
            Data::Ref(registry, key) => return Err(format!("{at}: a {registry} ({key:?}) is no value a record holds")),
            // An asset by its handle (every kind's id is one).
            Data::Asset(kind, name) => {
                Json::from(self.assets.handle(*kind, name).ok_or_else(|| format!("{at}: the content names no {kind} {name:?}"))?)
            }
            Data::Function => return Err(format!("{at}: a function isn't data")),
            Data::Type(_) => return Err(format!("{at}: a field's type (`schema`) isn't data")),
            // A choice by the engine's name of it, which the record's type
            // reads (a field that is one is read with [`SpecReader::choice`],
            // which refuses anything else).
            Data::Choice(c) => Json::String(c.name().to_string()),
        })
    }

    /// `d`, a field that takes a choice of `kind`, as the data a record
    /// reads: the engine's name of the member, or null for nil. Anything
    /// else (a string with the name among them) is an error.
    pub fn choice(&self, d: &Data, kind: ChoiceKind, at: &str) -> Result<Json, String> {
        match d {
            Data::Nil => Ok(Json::Null),
            Data::Choice(c) if c.kind == kind => Ok(Json::String(c.name().to_string())),
            other => Err(not_a_choice(other, kind, at)),
        }
    }

    /// `d` read as `T`. A message names the place in `d` that is wrong
    /// after `at` (`...: panels.types.grass.flags: invalid type`).
    pub fn read<T: DeserializeOwned>(&self, d: &Data, at: &str) -> Result<T, String> {
        let j = self.json(d, at)?;
        serde_path_to_error::deserialize(j).map_err(|e| {
            let path = e.path().to_string();
            if path == "." { format!("{at}: {}", e.inner()) } else { format!("{at}.{path}: {}", e.inner()) }
        })
    }
}

/// Remove fields from a table.
pub(crate) fn strip(spec: &mut Data, fields: &[&str]) {
    if let Data::Map(entries) = spec {
        entries.retain(|(k, _)| !matches!(k, DataKey::Str(s) if fields.contains(&s.as_str())));
    }
}

/// The fields of a spec, as a record's data: all but those named.
pub(crate) fn fields(d: &Definition, r: &SpecReader, skip: &[&str]) -> Result<Map<String, Json>, ContentError> {
    let mut spec = d.spec.clone();
    strip(&mut spec, skip);
    match r.json(&spec, &format!("{} {}", d.registry, d.key)).map_err(|m| err(d, m))? {
        Json::Object(o) => Ok(o),
        Json::Array(a) if a.is_empty() => Ok(Map::new()),
        _ => Err(err(d, "is a table")),
    }
}

/// The error of `d` given where a choice of `kind` is wanted.
pub(crate) fn not_a_choice(d: &Data, kind: ChoiceKind, at: &str) -> String {
    let k = kind.name();
    let first = Choice { kind, index: 0 }.member();
    match d {
        Data::Choice(c) => format!("{at}: expected a {k} (`{k}.{first}`), got {c}"),
        Data::Str(s) => format!("{at}: expected a {k} (`{k}.{first}`), got the string {s:?}"),
        other => format!("{at}: expected a {k} (`{k}.{first}`), got {}", other.kind()),
    }
}

/// The engine's name of `d`, a choice of `kind` (none for nil); anything
/// else is an error.
pub(crate) fn choice_name(d: &Data, kind: ChoiceKind, at: &str) -> Result<Option<&'static str>, String> {
    match d {
        Data::Nil => Ok(None),
        Data::Choice(c) if c.kind == kind => Ok(Some(c.name())),
        other => Err(not_a_choice(other, kind, at)),
    }
}
