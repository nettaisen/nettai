//! An entry's effects as its game's card screen lists them (a patch card's
//! `effects`): each effect record's line from the strings' text table
//! `patch_card_effects`, keyed by the record's `kind` and the name of its
//! one choice where it has one (a string field's value, or a definition's
//! id: `body.fire`, `charged_shot.patch-cards/airman/charge`), its numbers
//! put in by their field names (`HP+{amount}`, `{pair.slot}` a field of a
//! table it holds); whether the card shows it as a bug is the record's
//! `bug`, and the group the screen lists it in its `group` (the groups'
//! names are the text table `patch_card_groups`). An effect that holds
//! effects of its own (a table with an `effects` list: EXE4's two-part
//! card's half) has its line and then theirs. An effect without a line is
//! one the screen doesn't show, and isn't shown here either. An effect that
//! says its `version` (a card each version's ROM gives effects of its own:
//! the Bass-Cross MegaMan cards) is that version's alone, shown for a build of
//! it and not another's. Read by the data's names alone, as a tool reads any
//! collection's entries.

use crate::games::Names;
use nettai_content_api::{Data, DataKey, EntryHandle, Registry};

/// The strings' text table of the lines.
pub const TABLE: &str = "patch_card_effects";
/// The strings' text table of the groups' names, by the effects' `group`.
pub const GROUPS: &str = "patch_card_groups";
/// The entries' data field that lists their effects.
pub const FIELD: &str = "effects";
/// The effects' field that names the one version whose card has the effect.
pub const VERSION: &str = "version";

/// One effect as a line: its text, whether the card shows it as a bug,
/// and the group the card screen lists it in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub bug: bool,
    pub group: Option<String>,
}

/// What puts a value a line names into it as the screen shows it, by the
/// name's path (`{pair.card}`, a definition's id: its number; `{pair.slot}`:
/// the slot's name); none, and a number is put as it is, anything else
/// keeps the name.
pub type Named<'a> = &'a dyn Fn(&str, &Data) -> Option<String>;

/// The lines of entry `h`'s effects as a build of version `version` has
/// them (those of no version's and of its: [`of_version`]), in its order, or
/// none when its data list no effects (an entry of another kind of
/// collection).
pub fn lines(names: &Names, h: EntryHandle, version: Option<&str>, named: Named) -> Option<Vec<Line>> {
    let c = names.content;
    let d = c.defs.definitions.get(Registry::Entry, &c.defs.entry(h).key)?;
    let Data::List(effects) = d.spec.field(FIELD) else { return None };
    let effects: Vec<Data> = effects.iter().filter(|e| of_version(e, version)).cloned().collect();
    let mut out = Vec::new();
    of_effects(&effects, &|key| names.text(TABLE, key), named, &mut out);
    Some(out)
}

/// Whether effect record `r` is one a build of version `version` has: it
/// names no version, or that one (none: a build of no version, which has
/// only the effects of none).
pub fn of_version(r: &Data, version: Option<&str>) -> bool {
    match r.field(VERSION) {
        Data::Str(v) => Some(v.as_str()) == version,
        _ => true,
    }
}

/// The lines of `effects` into `out`: each one's, then those of the effects
/// a table of it holds.
fn of_effects(effects: &[Data], text: &dyn Fn(&str) -> Option<String>, named: Named, out: &mut Vec<Line>) {
    for r in effects {
        out.extend(line(r, text, named));
        if let Data::Map(fields) = r {
            for (_, v) in fields {
                if let Data::List(inner) = v.field(FIELD) {
                    of_effects(inner, text, named, out);
                }
            }
        }
    }
}

/// What the names of a card's line are as its screen shows them: a slot by
/// its name (the game's builds data's slots, by the field of an entry's
/// data that names one), a card by its own number (the game's compat
/// tables': `import::card_number`).
pub fn named_by<'a>(kit: &'a crate::builds::Kit) -> impl Fn(&str, &Data) -> Option<String> + 'a {
    move |path, value| {
        let last = path.rsplit('.').next().unwrap_or(path);
        match value {
            Data::Int(n) => kit.builds.slots.as_ref().filter(|s| s.slot_field() == last).and_then(|s| s.name(usize::try_from(*n).ok()?)).map(str::to_string),
            Data::Str(id) => crate::builds::import::card_number(&kit.game, id).map(|n| format!("{n:03}")),
            _ => None,
        }
    }
}

/// A number field of an entry's data (a card's `mb`), if it has one.
pub fn number(names: &Names, h: EntryHandle, field: &str) -> Option<i64> {
    let c = names.content;
    c.defs.definitions.get(Registry::Entry, &c.defs.entry(h).key).and_then(|d| d.spec.field(field).int())
}

/// The table's key of effect record `r` (`kind`, or `kind.choice`; its `group` and its `version` are no choice).
pub fn key(r: &Data) -> Option<String> {
    let Data::Map(fields) = r else { return None };
    let Data::Str(kind) = r.field("kind") else { return None };
    let choice = fields.iter().filter(|(k, _)| !matches!(k, DataKey::Str(s) if s == "kind" || s == "group" || s == VERSION)).find_map(|(_, v)| match v {
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
pub fn line(r: &Data, text: impl Fn(&str) -> Option<String>, named: Named) -> Option<Line> {
    let template = text(&key(r)?)?;
    let group = match r.field("group") {
        Data::Str(g) => Some(g.clone()),
        _ => None,
    };
    Some(Line { text: fill(&template, r, named), bug: matches!(r.field("bug"), Data::Bool(true)), group })
}

/// `template` with each `{name}` what `named` makes of the record's value
/// `name` (`{a.b}`: the value `b` of its table `a`), else the number it is;
/// a name neither has stays as it is.
pub fn fill(template: &str, r: &Data, named: Named) -> String {
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
        let value = name.split('.').fold(r, |d, part| d.field(part));
        match (named(name, value), value) {
            (Some(s), _) => out.push_str(&s),
            (None, Data::Int(n)) => out.push_str(&n.to_string()),
            (None, _) => out.push_str(&rest[i..i + j + 2]),
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
    /// the line by name, a table's by its path; one the table hasn't isn't
    /// shown.
    #[test]
    fn a_record_finds_its_line() {
        let table = |key: &str| -> Option<String> {
            match key {
                "hp_add" => Some("HP+{amount}".into()),
                "body.fire" => Some("FireBody".into()),
                "air_shoes" => Some("AirShoe".into()),
                "pair" => Some("With {pair.card} in {pair.slot}".into()),
                "all_guard" => Some("All Guard".into()),
                _ => None,
            }
        };
        let none: Named = &|_, _| None;
        let hp = record(&[("kind", Data::Str("hp_add".into())), ("amount", Data::Int(150)), ("group", Data::Str("parameter".into()))]);
        assert_eq!(line(&hp, table, none), Some(Line { text: "HP+150".into(), bug: false, group: Some("parameter".into()) }));
        let body = record(&[("kind", Data::Str("body".into())), ("element", Data::Str("fire".into()))]);
        assert_eq!(key(&body).as_deref(), Some("body.fire"));
        let off = record(&[("kind", Data::Str("air_shoes".into())), ("on", Data::Bool(false)), ("bug", Data::Bool(true))]);
        assert_eq!(line(&off, table, none), Some(Line { text: "AirShoe".into(), bug: true, group: None }));
        let hub = record(&[("kind", Data::Str("hub_style".into())), ("amount", Data::Int(1))]);
        assert_eq!(line(&hub, table, none), None);
        assert_eq!(fill("{x} and {amount}%", &hp, none), "{x} and 150%");
        // A two-part card's half: its line names its other half and the slot it must be in, then its own effects'.
        let guard = record(&[("kind", Data::Str("all_guard".into()))]);
        let half = record(&[("card", Data::Str("077-all-guard-2-2".into())), ("slot", Data::Int(3)), ("effects", Data::List(vec![guard]))]);
        let pair = record(&[("kind", Data::Str("pair".into())), ("pair", half)]);
        let named: Named = &|name, v| match (name, v) {
            ("pair.card", Data::Str(s)) => Some(s[..3].to_string()),
            _ => None,
        };
        let mut out = Vec::new();
        of_effects(&[pair], &table, named, &mut out);
        let texts: Vec<&str> = out.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["With 077 in 3", "All Guard"]);
    }
}
