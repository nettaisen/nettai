//! The names a game defines, as content gives them: values of their kind's
//! table (`PanelType.cracked`, `Family.sword`), which the define phase reads
//! back as `Data::Name` and a record reads (serde's form) as a table of one
//! field, its kind's tag (`{ "$PanelType": "cracked" }`): a string where a
//! name belongs is refused. Also the game's names of each kind, which its
//! value tables hold ([`game_names`]).

use std::fmt;

use nettai_content_api::{GameNames, NameKind};
use serde::de::{self, Deserializer, IgnoredAny, MapAccess, Visitor};

/// The tag a name of `kind` reads as in a record (`$PanelType`).
pub(crate) fn tag(kind: NameKind) -> String {
    format!("${}", kind.table())
}

/// A name as a record reads it: a name of its kind, or (where the engine
/// keeps one) the number it stands for.
pub(crate) enum NameOrNumber {
    Name(String),
    Number(u64),
}

struct NameVisitor {
    kind: NameKind,
    numbers: bool,
}

impl<'de> Visitor<'de> for NameVisitor {
    type Value = NameOrNumber;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a {} (`{}.x`)", self.kind.what(), self.kind.table())
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<NameOrNumber, E> {
        if self.numbers { Ok(NameOrNumber::Number(v)) } else { Err(E::invalid_type(de::Unexpected::Unsigned(v), &self)) }
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<NameOrNumber, E> {
        match u64::try_from(v) {
            Ok(v) => self.visit_u64(v),
            Err(_) => Err(E::invalid_type(de::Unexpected::Signed(v), &self)),
        }
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<NameOrNumber, E> {
        Err(E::custom(format!("{v:?} is a string: a {} is a value of its table (`{}.{v}`)", self.kind.what(), self.kind.table())))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<NameOrNumber, A::Error> {
        let tag = tag(self.kind);
        let Some(key) = map.next_key::<String>()? else { return Err(de::Error::invalid_length(0, &self)) };
        if key != tag {
            return Err(de::Error::custom(match key.strip_prefix('$') {
                Some(other) => format!("a {} where a {} belongs (`{}.x`)", other, self.kind.table(), self.kind.table()),
                None => format!("a table where a {} belongs (`{}.x`)", self.kind.what(), self.kind.table()),
            }));
        }
        let name: String = map.next_value()?;
        if map.next_key::<IgnoredAny>()?.is_some() {
            return Err(de::Error::custom(format!("a table where a {} belongs", self.kind.what())));
        }
        Ok(NameOrNumber::Name(name))
    }
}

/// A name of `kind` (`{ "$Family": "sword" }`), or with `numbers`, a number.
pub(crate) fn name_or_number<'de, D: Deserializer<'de>>(d: D, kind: NameKind, numbers: bool) -> Result<NameOrNumber, D::Error> {
    d.deserialize_any(NameVisitor { kind, numbers })
}

/// A name of `kind`, as a record reads it.
pub(crate) fn name<'de, D: Deserializer<'de>>(d: D, kind: NameKind) -> Result<String, D::Error> {
    match name_or_number(d, kind, false)? {
        NameOrNumber::Name(n) => Ok(n),
        NameOrNumber::Number(_) => unreachable!("no numbers"),
    }
}

macro_rules! name_fields {
    ($($kind:ident: $one:ident, $optional:ident;)*) => {$(
        /// A field holding a name of its kind (`deserialize_with`).
        #[allow(dead_code)]
        pub(crate) fn $one<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
            name(d, NameKind::$kind)
        }

        /// An optional field holding a name of its kind (`deserialize_with`,
        /// with `default`).
        #[allow(dead_code)]
        pub(crate) fn $optional<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
            let v: Option<serde_json::Value> = serde::Deserialize::deserialize(d)?;
            match v {
                None | Some(serde_json::Value::Null) => Ok(None),
                Some(v) => name(v, NameKind::$kind).map(Some).map_err(de::Error::custom),
            }
        }
    )*};
}

name_fields! {
    PanelType: panel_type, panel_type_opt;
    Family: family, family_opt;
    Emotion: emotion, emotion_opt;
    Version: version, version_opt;
}

/// The names `content`'s game defines, of each kind: its rules' panel
/// types, chip families, emotions, buttons and windows, its builds'
/// versions (`versions`) and its root's collections.
pub(crate) fn game_names(content: &super::Content, versions: Vec<String>) -> GameNames {
    let mut names = GameNames::default();
    if let Some(r) = content.rules.as_ref() {
        names.set(NameKind::PanelType, r.panels.names.clone());
        names.set(NameKind::Family, r.chip_families.names().into_iter().map(str::to_string).collect());
        names.set(NameKind::Emotion, r.emotion.names.clone());
    }
    if let Some(r) = content.defs.rules() {
        names.set(NameKind::Button, r.buttons.iter().map(|&b| content.defs.button(b).name.clone()).collect());
        names.set(NameKind::Window, r.windows.iter().map(|&w| content.defs.window(w).name.clone()).collect());
    }
    names.set(NameKind::Version, versions);
    // (A collection's entries are its record type's.)
    let mut collections: Vec<String> =
        content.defs.definitions.of(nettai_content_api::Registry::Entry).iter().filter_map(|d| d.record_type.clone()).collect();
    collections.sort();
    collections.dedup();
    names.set(NameKind::Collection, collections);
    names
}
