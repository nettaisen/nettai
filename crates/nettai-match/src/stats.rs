//! A round's navi stats by name, for a tool to show (the editor's stats
//! pane): each stat's name, what it means, and its value in the stats a
//! round starts with (`check::round_stats`: what the side's rules built on
//! the navi's fresh stats); the engine's ([`FIELDS`]), then the game's own
//! ([`game_fields`], by their schema's names). A side states none of them: what a save brings
//! to them is its game's facts (EXE6's `hp`, `reg_up`, `sun`), and the rest
//! is derived as the round is set up.
//!
//! Weapons, barriers, shot programs and forms are names in the match's
//! game (`none` for no weapon, barrier or program); `gauge` is `normal`,
//! `fast` or `slow`; `supports` lists `rush`, `beat` and `tango`, or is
//! `bug` (the NaviCust's support bug: none, and none can be set).

use nettai_battle::content::Content;
use nettai_battle::setup::{GaugeSpeed, NaviStats, Supports};
use nettai_content_api::{FormHandle, RecordHandle, WeaponHandle};

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
}

macro_rules! int {
    ($name:literal, $max:expr, $about:literal, |$s:ident| $place:expr) => {
        Field {
            name: $name,
            kind: Kind::Int($max),
            about: $about,
            get: |$s| Value::Int($place as u32),
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
        }
    };
}

/// Every stat, in the order a tool shows them.
pub const FIELDS: &[Field] = &[
    int!("base_hp", 9999, "the base HP (MegaMan's, the HP memories'; a link navi's, its story progress's)", |s| s.max_base_hp),
    int!("max_hp", 9999, "the maximum HP (the base and the NaviCust's HP programs)", |s| s.max_hp),
    int!("current_hp", 9999, "the HP in the block (a link battle starts at the maximum)", |s| s.hp),
    int!("attack", 255, "the buster's Attack level (0 is level 1)", |s| s.attack),
    int!("rapid", 255, "the buster's Rapid level (0 is level 1)", |s| s.rapid),
    int!("charge", 255, "the buster's Charge level (0 is level 1)", |s| s.charge),
    int!("custom_level", 255, "chips the custom screen deals (5 to 10)", |s| s.custom_level),
    int!("mega_level", 255, "Mega chips the folder can hold", |s| s.mega_level),
    int!("giga_level", 255, "Giga chips the folder can hold", |s| s.giga_level),
    int!("regular_memory", 255, "the Regular chip's MB at most", |s| s.reg_up),
    int!("mood", 255, "the emotion (0 worn out, 0x80 normal, 0xFF Full Synchro)", |s| s.mood),
    int!("element", 255, "the base form's element byte", |s| s.element),
    flag!("super_armor", "SuprArmr: no flinching", |s| s.super_armor),
    flag!("float_shoes", "FlotShoe: panels don't act on the navi", |s| s.float_shoes),
    flag!("air_shoes", "AirShoes: the navi stands over holes", |s| s.air_shoes),
    flag!("undershirt", "UnderSht: a hit that would delete leaves 1 HP", |s| s.undershirt),
    flag!("status_guard", "statuses don't take", |s| s.bugs.status_immunity),
    record!("first_barrier", "barrier", "the barrier the navi enters with", |s| s.first_barrier),
    Field {
        name: "gauge",
        kind: Kind::Gauge,
        about: "the custom gauge's speed",
        get: |s| Value::Gauge(s.gauge_speed),
    },
    Field {
        name: "supports",
        kind: Kind::Supports,
        about: "the supports (Rush, Beat, Tango), or the support bug",
        get: |s| Value::Supports(s.support),
    },
    int!("chip_recovery", 0xFFFF, "HP healed per chip used", |s| s.chip_recovery),
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

/// The field with this name.
pub fn field(name: &str) -> Option<&'static Field> {
    FIELDS.iter().find(|f| f.name == name)
}

/// The stats of the game's own (its rules' `stats`, `NaviStats::game`), by
/// their names in the schema's order, after [`FIELDS`] (EXE6's Beast Out
/// turns and ChpShufl, EXE5's Hub Style, ...): each a number or a flag.
pub fn game_fields(content: &Content, s: &NaviStats) -> Vec<(String, Value)> {
    let Some(rules) = content.defs.rules() else { return Vec::new() };
    let schema = content.defs.schema(rules.stats);
    if s.game.id() != rules.stats {
        return Vec::new();
    }
    use nettai_content_api::FieldValue as F;
    (0..schema.fields().len())
        .filter_map(|i| {
            let v = match s.game.get(schema, i) {
                F::Bool(b) => Value::Bool(b),
                F::U8(n) => Value::Int(n as u32),
                F::I8(n) => Value::Int(n as u8 as u32),
                F::U16(n) => Value::Int(n as u32),
                F::U32(n) => Value::Int(n),
                _ => return None,
            };
            Some((schema.field(i).name.clone(), v))
        })
        .collect()
}

/// A value as a match file writes it.
pub fn to_toml(content: &Content, v: Value) -> toml::Value {
    match v {
        Value::Int(x) => toml::Value::Integer(x as i64),
        Value::Bool(x) => toml::Value::Boolean(x),
        Value::Weapon(w) => toml::Value::String(w.map_or("none".into(), |h| crate::ids::local(&content.defs.weapon(h).key).into())),
        Value::Record(r) => toml::Value::String(r.map_or("none".into(), |h| crate::ids::local(&content.defs.records[h.index()].key).into())),
        Value::Form(f) => toml::Value::String(crate::ids::local(&content.defs.form(f).key).into()),
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
