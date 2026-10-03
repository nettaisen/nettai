//! What a side brings for its ruleset's systems besides what the match's
//! own fields say: each system's player setup (its `setup` fields:
//! docs/design/rules-in-luau.md §2.2), by the system's id and the field's
//! name, as the match file writes it (`[left.setup."bn5:light-dark"]`,
//! `value = 100`). Only the fields given are kept; the rest are the
//! system's own defaults (its `setup_defaults`, else zero: BN5's light and
//! dark value a fresh save's 500). Nothing here knows a game: any system's
//! setup is written the same way.
//!
//! A field holds a boolean, an integer (it must fit the field's type), an
//! enum's variant by name, a definition by key (a form, a chip...), or an
//! array of those. The setups are written into the round's player setup
//! after the side's own fields (the game, the Crosses...), which are facts
//! some systems take: a setup given here for the same field is the one the
//! round starts with.

use crate::Side;
use nettai_battle::content::Content;
use nettai_battle::custom::PlayerSetup;
use nettai_content_api::{FieldType, FieldValue, Registry, Schema, Value};
use std::collections::BTreeMap;

/// The setup fields a side's own fields write (its game, Crosses and
/// Beast Out: bn6-compat's `Unlocks::write`), which a tool offers there
/// rather than here.
pub const SIDE_FIELDS: &[&str] = &["version", "crosses", "beast_out", "cross_list"];

/// A setup field's value as a match file writes it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SetupValue {
    Bool(bool),
    Int(i64),
    /// An enum's variant, or a definition's key.
    Name(String),
    List(Vec<SetupValue>),
}

/// A side's setups: by system id, then by field name, the fields given.
pub type Setups = BTreeMap<String, BTreeMap<String, SetupValue>>;

impl SetupValue {
    /// A value from a match file's TOML.
    pub fn from_toml(v: &toml::Value) -> Result<SetupValue, String> {
        Ok(match v {
            toml::Value::Boolean(b) => SetupValue::Bool(*b),
            toml::Value::Integer(i) => SetupValue::Int(*i),
            toml::Value::String(s) => SetupValue::Name(s.clone()),
            toml::Value::Array(a) => SetupValue::List(a.iter().map(SetupValue::from_toml).collect::<Result<_, _>>()?),
            other => return Err(format!("{other} is no setup value (a boolean, an integer, a name or a list of those)")),
        })
    }

    pub fn to_toml(&self) -> toml::Value {
        match self {
            SetupValue::Bool(b) => toml::Value::Boolean(*b),
            SetupValue::Int(i) => toml::Value::Integer(*i),
            SetupValue::Name(s) => toml::Value::String(s.clone()),
            SetupValue::List(l) => toml::Value::Array(l.iter().map(SetupValue::to_toml).collect()),
        }
    }
}

/// A definition of `registry` by key, as a setup field names it.
fn definition(content: &Content, registry: Registry, key: &str) -> Option<u16> {
    let d = &content.defs;
    Some(match registry {
        Registry::Form => d.form_by_key(key)?.0,
        Registry::Chip => d.chip_by_key(key)?.0,
        Registry::Navi => d.navi_by_key(key)?.0,
        Registry::Weapon => d.weapon_by_key(key)?.0,
        Registry::Record => d.record(key)?.0,
        _ => return None,
    })
}

/// The engine's value of `v` for a field (or an array's element) of type
/// `ty`, or why it is none.
fn scalar(content: &Content, ty: &FieldType, v: &SetupValue) -> Result<Value, String> {
    match (ty, v) {
        (FieldType::Bool, SetupValue::Bool(b)) => Ok(Value::Bool(*b)),
        (FieldType::U8 | FieldType::U16 | FieldType::U32 | FieldType::I8 | FieldType::I16 | FieldType::I32, SetupValue::Int(i)) => {
            let fits = match ty {
                FieldType::U8 => (0..=u8::MAX as i64).contains(i),
                FieldType::U16 => (0..=u16::MAX as i64).contains(i),
                FieldType::U32 => (0..=u32::MAX as i64).contains(i),
                FieldType::I8 => (i8::MIN as i64..=i8::MAX as i64).contains(i),
                FieldType::I16 => (i16::MIN as i64..=i16::MAX as i64).contains(i),
                _ => (i32::MIN as i64..=i32::MAX as i64).contains(i),
            };
            if fits { Ok(Value::Int(*i)) } else { Err(format!("{i} doesn't fit a {ty:?} field")) }
        }
        (FieldType::Enum(names), SetupValue::Name(n)) => names
            .iter()
            .position(|x| x == n)
            .map(|i| Value::Int(i as i64))
            .ok_or_else(|| format!("{n:?} is none of {}", names.join(", "))),
        (FieldType::Ref(registry, _), SetupValue::Name(key)) => definition(content, *registry, key)
            .map(|h| Value::Def(*registry, h))
            .ok_or_else(|| format!("{key:?} names no {registry} of the content")),
        (ty, v) => Err(format!("{v:?} is no value of a {ty:?} field")),
    }
}

/// What a value is, written back as a match file names it (a definition
/// by key, an enum's variant by name).
fn written(content: &Content, ty: &FieldType, v: FieldValue) -> Option<SetupValue> {
    Some(match (ty, v) {
        (FieldType::Bool, FieldValue::Bool(b)) => SetupValue::Bool(b),
        (FieldType::Enum(names), FieldValue::Enum(i)) => SetupValue::Name(names.get(i as usize)?.clone()),
        (FieldType::Ref(..), FieldValue::Ref(Some((registry, h)))) => SetupValue::Name(match registry {
            Registry::Form => content.defs.form(nettai_content_api::FormHandle(h)).key.clone(),
            Registry::Chip => content.defs.chip(nettai_content_api::ChipHandle(h)).key.clone(),
            Registry::Navi => content.defs.navi(nettai_content_api::NaviHandle(h)).key.clone(),
            _ => return None,
        }),
        (_, v) => SetupValue::Int(v.load().int()?),
    })
}

/// The systems of `side`'s ruleset that have a setup, with its layout, in
/// the ruleset's order.
pub fn systems<'c>(content: &'c Content, side: &Side) -> Vec<(&'c str, &'c Schema)> {
    let Some(r) = side.ruleset_or_stock(content) else { return Vec::new() };
    content
        .defs
        .ruleset(r)
        .systems
        .iter()
        .map(|&h| content.defs.system(h))
        .map(|s| (s.key.as_str(), content.defs.schema(s.setup)))
        .filter(|(_, schema)| !schema.fields().is_empty())
        .collect()
}

/// What is wrong with `side`'s setups: a system its ruleset hasn't, a field
/// its setup hasn't, a value that isn't one of the field's.
pub fn check(content: &Content, side: &Side) -> Vec<String> {
    let mut out = Vec::new();
    let systems = systems(content, side);
    for (system, fields) in &side.setups {
        let Some(&(_, schema)) = systems.iter().find(|(k, _)| k == system) else {
            out.push(format!("setup: {system} is no system of the side's ruleset with a setup"));
            continue;
        };
        for (field, v) in fields {
            let Some(i) = schema.index_of(field) else {
                out.push(format!("setup: {system} has no setup field `{field}`"));
                continue;
            };
            let ty = &schema.field(i).ty;
            let bad = match (ty, v) {
                (FieldType::Array(elem, n), SetupValue::List(items)) => {
                    if items.len() > *n as usize {
                        Some(format!("{} values; the field holds {n}", items.len()))
                    } else {
                        items.iter().find_map(|x| scalar(content, elem, x).err())
                    }
                }
                (ty, v) => scalar(content, ty, v).err(),
            };
            if let Some(why) = bad {
                out.push(format!("setup: {system}.{field}: {why}"));
            }
        }
    }
    out
}

/// Write `side`'s setups into `player`'s (a field of an array its values
/// in order, the rest zero), after the side's other facts.
pub fn write(content: &Content, side: &Side, player: &mut PlayerSetup) -> Result<(), String> {
    let systems = systems(content, side);
    for (system, fields) in &side.setups {
        let schema = systems.iter().find(|(k, _)| k == system).map(|(_, s)| *s).ok_or_else(|| format!("no system {system}"))?;
        for (field, v) in fields {
            let i = schema.index_of(field).ok_or_else(|| format!("{system} has no setup field `{field}`"))?;
            match (&schema.field(i).ty, v) {
                (FieldType::Array(elem, n), SetupValue::List(items)) => {
                    for k in 0..*n as usize {
                        let value = match items.get(k) {
                            Some(x) => scalar(content, elem, x)?,
                            None if **elem == FieldType::Bool => Value::Bool(false),
                            None if matches!(**elem, FieldType::Ref(..)) => Value::Nil,
                            None => Value::Int(0),
                        };
                        player.set_rule_elem(content, system, field, k, value)?;
                    }
                }
                (ty, v) => player.set_rule(content, system, field, scalar(content, ty, v)?)?,
            }
        }
    }
    Ok(())
}

/// Field `field` of system `system` as `side`'s round starts with it: the
/// side's setup's, else the system's default (none: the side's ruleset has
/// no such system or field, or it is an array). For a tool that shows it.
pub fn value(content: &Content, side: &Side, system: &str, field: &str) -> Option<SetupValue> {
    if let Some(v) = side.setups.get(system).and_then(|f| f.get(field)) {
        return Some(v.clone());
    }
    let player = PlayerSetup { ruleset: side.ruleset_or_stock(content), ..PlayerSetup::default() };
    let (schema, block) = player.rule_block(content, system)?;
    let i = schema.index_of(field)?;
    let ty = &schema.field(i).ty;
    if let FieldType::Array(..) = ty {
        return None;
    }
    written(content, ty, block.get(schema, i))
}

/// Keep only the setups of systems `side`'s ruleset has (after its ruleset
/// changed).
pub fn retain_own(content: &Content, side: &mut Side) {
    let own: Vec<String> = systems(content, side).iter().map(|(k, _)| k.to_string()).collect();
    side.setups.retain(|k, _| own.contains(k));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::every_game;

    /// A BN5 side's light/dark value: the system's default (a fresh save's
    /// 500) until the side gives one; a value the field can't hold, a field
    /// or system the ruleset hasn't, are said.
    #[test]
    fn setups_default_and_check() {
        let content = every_game();
        let mut m = crate::Match::empty(&content).unwrap();
        let s = &mut m.sides[0];
        s.ruleset = content.defs.ruleset_by_key("bn5:stock");
        s.navi = content.defs.navi_by_key("bn5:megaman").unwrap();
        assert_eq!(value(&content, s, "bn5:light-dark", "value"), Some(SetupValue::Int(500)));
        s.setups.entry("bn5:light-dark".into()).or_default().insert("value".into(), SetupValue::Int(100));
        assert_eq!(value(&content, s, "bn5:light-dark", "value"), Some(SetupValue::Int(100)));
        assert_eq!(check(&content, s), Vec::<String>::new());
        let mut player = PlayerSetup { ruleset: s.ruleset, ..PlayerSetup::default() };
        write(&content, s, &mut player).unwrap();
        let (schema, block) = player.rule_block(&content, "bn5:light-dark").unwrap();
        assert_eq!(block.get(schema, schema.index_of("value").unwrap()), FieldValue::U16(100));
        // What doesn't fit, and what isn't there.
        s.setups.get_mut("bn5:light-dark").unwrap().insert("value".into(), SetupValue::Int(70_000));
        s.setups.get_mut("bn5:light-dark").unwrap().insert("hub_style".into(), SetupValue::Int(1));
        s.setups.entry("bn6:cross".into()).or_default().insert("version".into(), SetupValue::Name("falzar".into()));
        let said = check(&content, s);
        for p in [
            "setup: bn5:light-dark.value: 70000 doesn't fit a U16 field",
            "setup: bn5:light-dark has no setup field `hub_style`",
            "setup: bn6:cross is no system of the side's ruleset with a setup",
        ] {
            assert!(said.iter().any(|x| x == p), "{p:?} not in {said:?}");
        }
        // A ruleset change keeps its own systems' setups alone.
        s.ruleset = content.defs.ruleset_by_key("bn6:stock");
        retain_own(&content, s);
        assert_eq!(s.setups.keys().collect::<Vec<_>>(), ["bn6:cross"]);
    }
}
