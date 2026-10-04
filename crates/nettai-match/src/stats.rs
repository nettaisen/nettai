//! A side's stats block: the navi's stats by name, over what a save gives
//! it (`Side::save_base`: its fresh stats, a link navi's at its level). A
//! match file's `[left.stats]` sets what differs; writing one, the fields
//! that differ are written. Every stat a round starts from is a field, so a
//! written block gives back the same stats.
//!
//! The fields, in the order they apply: `hp` sets the base HP, the maximum
//! and the HP the round starts with together; `max_hp` and `current_hp`
//! set those apart. Weapons, barriers, shot programs and forms are content
//! keys (`none` for no weapon, barrier or program); `gauge` is `normal`,
//! `fast` or `slow`; `supports` lists `rush`, `beat` and `tango`, or is
//! `bug` (the NaviCust's support bug: none, and none can be set).

use nettai_battle::content::Content;
use nettai_battle::setup::{GaugeSpeed, NaviStats, Supports};
use nettai_content_api::{FormHandle, RecordHandle, WeaponHandle};
use std::collections::BTreeMap;

/// One stat's value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value {
    Int(u32),
    Bool(bool),
    Weapon(Option<WeaponHandle>),
    Record(Option<RecordHandle>),
    Form(FormHandle),
    Gauge(GaugeSpeed),
    Supports(Option<Supports>),
}

/// What kind of value a stat takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A whole number up to this.
    Int(u32),
    Bool,
    Weapon,
    /// A record of this type (`barrier`, `projectile-variant`).
    Record(&'static str),
    Form,
    Gauge,
    Supports,
}

/// A stat: its name, what it takes, and where it is.
pub struct Field {
    pub name: &'static str,
    pub kind: Kind,
    /// What it means, for the editor.
    pub about: &'static str,
    pub get: fn(&NaviStats) -> Value,
    pub set: fn(&mut NaviStats, Value),
}

macro_rules! int {
    ($name:literal, $max:expr, $about:literal, |$s:ident| $place:expr) => {
        Field {
            name: $name,
            kind: Kind::Int($max),
            about: $about,
            get: |$s| Value::Int($place as u32),
            set: |$s, v| {
                if let Value::Int(x) = v {
                    $place = x as _;
                }
            },
        }
    };
}

macro_rules! flag {
    ($name:literal, $about:literal, |$s:ident| $place:expr) => {
        Field {
            name: $name,
            kind: Kind::Bool,
            about: $about,
            get: |$s| Value::Bool($place),
            set: |$s, v| {
                if let Value::Bool(x) = v {
                    $place = x;
                }
            },
        }
    };
}

macro_rules! weapon {
    ($name:literal, $about:literal, |$s:ident| $place:expr) => {
        Field {
            name: $name,
            kind: Kind::Weapon,
            about: $about,
            get: |$s| Value::Weapon($place),
            set: |$s, v| {
                if let Value::Weapon(x) = v {
                    $place = x;
                }
            },
        }
    };
}

macro_rules! record {
    ($name:literal, $ty:literal, $about:literal, |$s:ident| $place:expr) => {
        Field {
            name: $name,
            kind: Kind::Record($ty),
            about: $about,
            get: |$s| Value::Record($place),
            set: |$s, v| {
                if let Value::Record(x) = v {
                    $place = x;
                }
            },
        }
    };
}

macro_rules! form {
    ($name:literal, $about:literal, |$s:ident| $place:expr) => {
        Field {
            name: $name,
            kind: Kind::Form,
            about: $about,
            get: |$s| Value::Form($place),
            set: |$s, v| {
                if let Value::Form(x) = v {
                    $place = x;
                }
            },
        }
    };
}

/// Every stat, in the order a block applies them.
pub const FIELDS: &[Field] = &[
    Field {
        name: "hp",
        kind: Kind::Int(9999),
        about: "base HP (the HP memories'); with no max_hp or current_hp, also the maximum and the HP the round starts with",
        get: |s| Value::Int(s.max_base_hp as u32),
        set: |s, v| {
            if let Value::Int(x) = v {
                (s.max_base_hp, s.max_hp, s.hp) = (x as u16, x as u16, x as u16);
            }
        },
    },
    int!("max_hp", 9999, "the maximum HP (the base and the NaviCust's HP programs)", |s| s.max_hp),
    int!("current_hp", 9999, "the HP the round starts with", |s| s.hp),
    int!("attack", 255, "the buster's Attack level (0 is level 1)", |s| s.attack),
    int!("rapid", 255, "the buster's Rapid level (0 is level 1)", |s| s.rapid),
    int!("charge", 255, "the buster's Charge level (0 is level 1)", |s| s.charge),
    int!("custom_level", 255, "chips the custom screen deals (5 to 10)", |s| s.custom_level),
    int!("mega_level", 255, "Mega chips the folder can hold", |s| s.mega_level),
    int!("giga_level", 255, "Giga chips the folder can hold", |s| s.giga_level),
    int!("regular_memory", 255, "the Regular chip's MB at most", |s| s.reg_up),
    int!("mood", 255, "the emotion (0 worn out, 0x80 normal, 0xFF Full Synchro)", |s| s.mood),
    int!("element", 255, "the base form's element byte", |s| s.element),
    int!("beast_out_counter", 255, "Beast Out turns", |s| s.beast_out_counter),
    flag!("sun", "fighting outdoors in the sun (some chips hit harder)", |s| s.sun),
    flag!("super_armor", "SuprArmr: no flinching", |s| s.super_armor),
    flag!("float_shoes", "FlotShoe: panels don't act on the navi", |s| s.float_shoes),
    flag!("air_shoes", "AirShoes: the navi stands over holes", |s| s.air_shoes),
    flag!("undershirt", "UnderSht: a hit that would delete leaves 1 HP", |s| s.undershirt),
    int!("hub_style", 2, "BN5's Hub Style (its patch card 111: 1 Team ProtoMan's, 2 Team Colonel's)", |s| s.hub_style),
    flag!("status_guard", "statuses don't take", |s| s.bugs.status_immunity),
    record!("first_barrier", "barrier", "the barrier the navi enters with", |s| s.first_barrier),
    Field {
        name: "gauge",
        kind: Kind::Gauge,
        about: "the custom gauge's speed",
        get: |s| Value::Gauge(s.gauge_speed),
        set: |s, v| {
            if let Value::Gauge(x) = v {
                s.gauge_speed = x;
            }
        },
    },
    Field {
        name: "supports",
        kind: Kind::Supports,
        about: "the supports (Rush, Beat, Tango), or the support bug",
        get: |s| Value::Supports(s.support),
        set: |s, v| {
            if let Value::Supports(x) = v {
                s.support = x;
            }
        },
    },
    int!("chip_recovery", 0xFFFF, "HP healed per chip used", |s| s.chip_recovery),
    flag!("chip_shuffle", "ChpShufl: the custom screen re-deals", |s| s.chip_shuffle),
    flag!("number_open", "NumbrOpn: the custom screen deals 10", |s| s.number_open),
    weapon!("buster", "the B button's weapon", |s| s.weapons.buster),
    weapon!("charged_shot", "the charged shot", |s| s.weapons.charge_shot),
    weapon!("back_special", "the B+Back special", |s| s.weapons.back_special),
    weapon!("a_charge", "the A button's charge (charged chips)", |s| s.weapons.a_charge),
    weapon!("mode9_a", "the A button in battle mode 9", |s| s.weapons.mode9_a),
    record!("buster_shot", "projectile-variant", "the buster shot's program", |s| s.weapons.buster_shot),
    record!("charged_shot_program", "projectile-variant", "the charged shot's program", |s| s.weapons.charge_shot_kind),
    int!("back_special_damage", 0xFFFF, "the B+Back special's damage", |s| s.weapons.back_special_damage),
    int!("navi_variant", 255, "the navi's variant (its move lag)", |s| s.navi_variant),
    form!("form", "the form the navi is in", |s| s.form),
    form!("starting_form", "the form the battle starts in", |s| s.starting_form),
    int!("chip_drops", 255, "the NaviCust's collector bug and Collect (no netbattle reads it)", |s| s.chip_drops),
    int!("encounters", 255, "the NaviCust's encounter bug (no netbattle reads it)", |s| s.encounters),
    int!("folder", 255, "the folder the navi brings (0-2)", |s| s.folder),
    int!("folder_1_regular", 255, "the first folder's Regular chip in the save (0xFF none)", |s| s.folder_reg[0]),
    int!("folder_2_regular", 255, "the second folder's Regular chip in the save (0xFF none)", |s| s.folder_reg[1]),
    int!("folder_1_tag_a", 255, "the first folder's tag chips in the save (0xFF none)", |s| s.folder_tags[0][0]),
    int!("folder_1_tag_b", 255, "", |s| s.folder_tags[0][1]),
    int!("folder_2_tag_a", 255, "the second folder's tag chips in the save (0xFF none)", |s| s.folder_tags[1][0]),
    int!("folder_2_tag_b", 255, "", |s| s.folder_tags[1][1]),
    // The NaviCust's bugs.
    int!("step_bug", 255, "bug: steps go astray (1)", |s| s.bugs.processing),
    int!("auto_step", 255, "bug: random steps after a move", |s| s.bugs.auto_step),
    int!("panel_trail", 255, "bug: what a step leaves on the panel behind (3 cracked; 0xFF none)", |s| s.bugs.panel_trail_kind),
    int!("panel_trail_level", 255, "bug: how often a step leaves it (0 never)", |s| s.bugs.panel_trail_level),
    int!("buster_blanks", 255, "bug: buster shots of 16 that fire blanks", |s| s.bugs.buster_blanks),
    int!("buster_charged", 255, "bug: buster shots of 16 that fire charged shots", |s| s.bugs.buster_charged),
    int!("hit_status", 255, "bug: the status a hit gives (1 blind, 2 confusion, 3 HP bug)", |s| s.bugs.hit_status),
    int!("hp_drain", 255, "bug: HP drain in the fight (level 0-7)", |s| s.bugs.hp_drain),
    int!("custom_drain", 255, "bug: HP drain while the custom screen is open (level 0-7)", |s| s.bugs.custom_drain),
    int!("battle_start_bug", 255, "bug: a status at the battle's start (9, 10)", |s| s.bugs.battle_start),
    int!("emotion_bug", 255, "bug: the emotion swings", |s| s.bugs.emotion),
    int!("starting_damage", 255, "bug: damage at the battle's start", |s| s.bugs.starting_damage),
    int!("custom_damage", 0xFFFF, "bug: damage when the custom screen opens", |s| s.bugs.custom_damage),
    int!("hand_shrink_turn", 255, "bug: from this custom screen on, one chip fewer each (0 never)", |s| s.bugs.hand_shrink_turn),
];

/// The fields a save keeps when its NaviCust compiles (`sub_8136C24` keeps
/// them through its reset; the rest the NaviCust makes): what a side with a
/// NaviCust may set.
pub const SAVE_FIELDS: &[&str] = &[
    "hp",
    "regular_memory",
    "mood",
    "beast_out_counter",
    "sun",
    "form",
    "folder",
    "folder_1_regular",
    "folder_2_regular",
    "folder_1_tag_a",
    "folder_1_tag_b",
    "folder_2_tag_a",
    "folder_2_tag_b",
];

/// The field with this name.
pub fn field(name: &str) -> Option<&'static Field> {
    FIELDS.iter().find(|f| f.name == name)
}

/// A value as a match file writes it.
pub fn to_toml(content: &Content, v: Value) -> toml::Value {
    match v {
        Value::Int(x) => toml::Value::Integer(x as i64),
        Value::Bool(x) => toml::Value::Boolean(x),
        Value::Weapon(w) => toml::Value::String(w.map_or("none".into(), |h| content.defs.weapon(h).key.clone())),
        Value::Record(r) => toml::Value::String(r.map_or("none".into(), |h| content.defs.records[h.index()].key.clone())),
        Value::Form(f) => toml::Value::String(content.defs.form(f).key.clone()),
        Value::Gauge(g) => toml::Value::String(gauge_name(g).into()),
        Value::Supports(None) => toml::Value::String("bug".into()),
        Value::Supports(Some(n)) => toml::Value::Array(
            [(n.rush, "rush"), (n.beat, "beat"), (n.tango, "tango")]
                .iter()
                .filter(|(on, _)| *on)
                .map(|(_, name)| toml::Value::String(name.to_string()))
                .collect(),
        ),
    }
}

pub fn gauge_name(g: GaugeSpeed) -> &'static str {
    match g {
        GaugeSpeed::Normal => "normal",
        GaugeSpeed::Fast => "fast",
        GaugeSpeed::Slow => "slow",
    }
}

/// A value of `f` from a match file.
pub fn from_toml(content: &Content, f: &Field, v: &toml::Value) -> Result<Value, String> {
    let key = |v: &toml::Value| v.as_str().map(str::to_string).ok_or_else(|| format!("{} takes a key, not {v}", f.name));
    match f.kind {
        Kind::Int(max) => match v.as_integer() {
            Some(x) if (0..=max as i64).contains(&x) => Ok(Value::Int(x as u32)),
            _ => Err(format!("{} takes a whole number from 0 to {max}, not {v}", f.name)),
        },
        Kind::Bool => v.as_bool().map(Value::Bool).ok_or_else(|| format!("{} takes true or false, not {v}", f.name)),
        Kind::Weapon => {
            let k = key(v)?;
            if k == "none" {
                return Ok(Value::Weapon(None));
            }
            content.defs.weapon_by_key(&k).map(|h| Value::Weapon(Some(h))).ok_or_else(|| format!("{}: no weapon {k:?}", f.name))
        }
        Kind::Record(ty) => {
            let k = key(v)?;
            if k == "none" {
                return Ok(Value::Record(None));
            }
            match content.defs.record(&k) {
                Some(h) if content.defs.records[h.index()].record_type == ty => Ok(Value::Record(Some(h))),
                Some(_) => Err(format!("{}: {k:?} is no {ty}", f.name)),
                None => Err(format!("{}: no {ty} {k:?}", f.name)),
            }
        }
        Kind::Form => {
            let k = key(v)?;
            content.defs.form_by_key(&k).map(Value::Form).ok_or_else(|| format!("{}: no form {k:?}", f.name))
        }
        Kind::Gauge => match v.as_str() {
            Some("normal") => Ok(Value::Gauge(GaugeSpeed::Normal)),
            Some("fast") => Ok(Value::Gauge(GaugeSpeed::Fast)),
            Some("slow") => Ok(Value::Gauge(GaugeSpeed::Slow)),
            _ => Err(format!("gauge is normal, fast or slow, not {v}")),
        },
        Kind::Supports => match v {
            toml::Value::String(s) if s == "bug" => Ok(Value::Supports(None)),
            toml::Value::Array(list) => {
                let mut n = Supports::default();
                for item in list {
                    match item.as_str() {
                        Some("rush") => n.rush = true,
                        Some("beat") => n.beat = true,
                        Some("tango") => n.tango = true,
                        _ => return Err(format!("supports lists rush, beat and tango, not {item}")),
                    }
                }
                Ok(Value::Supports(Some(n)))
            }
            _ => Err(format!("supports is a list of rush, beat and tango, or \"bug\", not {v}")),
        },
    }
}

/// `block` applied over `stats`, in the fields' order; the problems with
/// it, each said.
pub fn apply(content: &Content, block: &BTreeMap<String, toml::Value>, stats: &mut NaviStats) -> Vec<String> {
    let mut problems = Vec::new();
    for name in block.keys() {
        if field(name).is_none() {
            problems.push(format!("no stat {name:?} (the stats are {})", FIELDS.iter().map(|f| f.name).collect::<Vec<_>>().join(", ")));
        }
    }
    for f in FIELDS {
        if let Some(v) = block.get(f.name) {
            match from_toml(content, f, v) {
                Ok(v) => (f.set)(stats, v),
                Err(e) => problems.push(e),
            }
        }
    }
    problems
}

/// The block that turns `base` into `stats`: each field that differs once
/// the fields before it are set.
pub fn diff(content: &Content, base: &NaviStats, stats: &NaviStats) -> BTreeMap<String, toml::Value> {
    let mut now = *base;
    let mut block = BTreeMap::new();
    for f in FIELDS {
        let want = (f.get)(stats);
        if (f.get)(&now) != want {
            (f.set)(&mut now, want);
            block.insert(f.name.to_string(), to_toml(content, want));
        }
    }
    block
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Live play's navi, written as a block over MegaMan's fresh stats, is
    /// itself again.
    #[test]
    fn a_block_gives_back_the_stats() {
        let content = crate::testing::bn6_content();
        let live = crate::draw::live_navi(&content);
        let base = crate::Side::base_stats(&content, live.navi, nettai_battle::custom::GameVersion::Falzar);
        let block = diff(&content, &base, &live);
        assert!(block.contains_key("hp"), "{block:?}");
        let mut back = base;
        assert_eq!(apply(&content, &block, &mut back), Vec::<String>::new());
        assert_eq!(back, live);
        // A name no stat has, a value out of range, a key of nothing.
        let mut bad = BTreeMap::new();
        bad.insert("atack".to_string(), toml::Value::Integer(1));
        bad.insert("rapid".to_string(), toml::Value::Integer(-1));
        bad.insert("buster".to_string(), toml::Value::String("nothing".into()));
        let problems = apply(&content, &bad, &mut back);
        assert_eq!(problems.len(), 3, "{problems:?}");
    }
}
