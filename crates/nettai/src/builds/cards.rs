//! An entry's effects as its game's card screen lists them (a patch card's
//! `effects`): each effect record's line from the strings' text table
//! `patch_card_effects`, keyed by the record's `kind` and the name of its
//! one choice where it has one (a string field's value, or a definition's
//! id: `body.fire`, `charged_shot.patch-cards/airman/charge`), its numbers
//! put in by their field names (`HP+{amount}`); whether the card shows it
//! as a bug is the record's `bug`, and the group the screen lists it in its
//! `group` ("parameter" or "ability"). An effect without a line is one the
//! screen doesn't show, and isn't shown here either. Read by the data's
//! names alone, as a tool reads any collection's entries.

use crate::games::Names;
use nettai_content_api::{Data, DataKey, EntryHandle, Registry};

/// The strings' text table of the lines.
pub const TABLE: &str = "patch_card_effects";
/// The entries' data field that lists their effects.
pub const FIELD: &str = "effects";

/// One effect as a line: its text, whether the card shows it as a bug,
/// and the group the card screen lists it in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub bug: bool,
    pub group: Option<String>,
}

/// The lines of entry `h`'s effects, in its order, or none when its data
/// list no effects (an entry of another kind of collection).
pub fn lines(names: &Names, h: EntryHandle) -> Option<Vec<Line>> {
    let c = names.content;
    let d = c.defs.definitions.get(Registry::Entry, &c.defs.entry(h).key)?;
    let Data::List(effects) = d.spec.field(FIELD) else { return None };
    Some(effects.iter().filter_map(|r| line(r, |key| names.text(TABLE, key))).collect())
}

/// A number field of an entry's data (a card's `mb`), if it has one.
pub fn number(names: &Names, h: EntryHandle, field: &str) -> Option<i64> {
    let c = names.content;
    c.defs.definitions.get(Registry::Entry, &c.defs.entry(h).key).and_then(|d| d.spec.field(field).int())
}

/// The table's key of effect record `r` (`kind`, or `kind.choice`).
pub fn key(r: &Data) -> Option<String> {
    let Data::Map(fields) = r else { return None };
    let Data::Str(kind) = r.field("kind") else { return None };
    let choice = fields.iter().filter(|(k, _)| !matches!(k, DataKey::Str(s) if s == "kind" || s == "group")).find_map(|(_, v)| match v {
        Data::Str(s) => Some(s.clone()),
        Data::Ref(_, k) => Some(nettai_match::ids::local(k).to_string()),
        _ => None,
    });
    Some(match choice {
        Some(choice) => format!("{kind}.{choice}"),
        None => kind.clone(),
    })
}

/// Effect record `r` as a line, its text from `text` by its key; none
/// without one (an effect the card screen doesn't show).
pub fn line(r: &Data, text: impl Fn(&str) -> Option<String>) -> Option<Line> {
    let template = text(&key(r)?)?;
    let group = match r.field("group") {
        Data::Str(g) => Some(g.clone()),
        _ => None,
    };
    Some(Line { text: fill(&template, r), bug: matches!(r.field("bug"), Data::Bool(true)), group })
}

/// `template` with each `{name}` the record's number `name` (a name the
/// record has no number of stays as it is).
pub fn fill(template: &str, r: &Data) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let Some(j) = after.find('}') else {
            out.push_str(&rest[i..]);
            return out;
        };
        let name = &after[..j];
        match r.field(name) {
            Data::Int(n) => out.push_str(&n.to_string()),
            _ => out.push_str(&rest[i..i + j + 2]),
        }
        rest = &after[j + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(fields: &[(&str, Data)]) -> Data {
        Data::map(fields.iter().map(|(k, v)| (*k, v.clone())))
    }

    /// A record's key is its kind and its one choice; its numbers go into
    /// the line by name; one the table hasn't isn't shown.
    #[test]
    fn a_record_finds_its_line() {
        let table = |key: &str| -> Option<String> {
            match key {
                "hp_add" => Some("HP+{amount}".into()),
                "body.fire" => Some("FireBody".into()),
                "air_shoes" => Some("AirShoe".into()),
                _ => None,
            }
        };
        let hp = record(&[("kind", Data::Str("hp_add".into())), ("amount", Data::Int(150)), ("group", Data::Str("parameter".into()))]);
        assert_eq!(line(&hp, table), Some(Line { text: "HP+150".into(), bug: false, group: Some("parameter".into()) }));
        let body = record(&[("kind", Data::Str("body".into())), ("element", Data::Str("fire".into()))]);
        assert_eq!(key(&body).as_deref(), Some("body.fire"));
        let off = record(&[("kind", Data::Str("air_shoes".into())), ("on", Data::Bool(false)), ("bug", Data::Bool(true))]);
        assert_eq!(line(&off, table), Some(Line { text: "AirShoe".into(), bug: true, group: None }));
        let hub = record(&[("kind", Data::Str("hub_style".into())), ("amount", Data::Int(1))]);
        assert_eq!(line(&hub, table), None);
        assert_eq!(fill("{x} and {amount}%", &hp), "{x} and 150%");
    }
}
