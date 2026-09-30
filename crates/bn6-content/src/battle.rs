//! The battle data of a pack: what the engine's [`Content`] holds, as
//! TOML files laid out by owner (docs/design/content-pack.md).
//!
//! ```text
//! chips/NNN-name/chip.toml             a chip, with the data only its action reads
//! navis/NN-name/navi.toml              a navi
//! navis/00-megaman/forms/NN-name/form.toml   one of MegaMan's forms
//! navis/00-megaman/weapons/NN-name/weapon.toml   a weapon routine a script implements
//! objects/KIND/object.toml             an object kind's data (rocks, overlays...) and, for a kind
//!                                      a script implements, its [kind] (pool, index, script)
//! rules/*.toml                         rules no entity owns (collision, panels, stages...)
//! registries/*.toml                    things many entities name by id (effects, regions...)
//! graphics/sprites/CC-II/animations.json   sprite timing (see crate::sprite)
//! **/*.luau                            the scripts, next to the data they implement
//! ```
//!
//! An entity names its script in its file (`script = "chip.luau"`, a path
//! relative to the file's folder); `Content::registrations` says what that
//! makes the script implement (docs/design/scripting.md).
//!
//! Every record carries its original id; [`load`] builds the engine's
//! dense id-indexed tables and reports duplicate ids, gaps and dangling
//! references by file. [`export`] writes the files, and reading them back
//! gives the same `Content` (checked by the extractor on every export).

use crate::pack::Files;
use crate::report::Report;
use bn6_battle::content::*;
use bn6_battle::field::PanelType;
use bn6_battle::setup::{ActorEntry, ActorKind, ActorList, ActorListId, BattleSettings};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Where MegaMan's forms are.
const FORMS_OF: &str = "navis/00-megaman/forms";

/// Attach points a sprite has in the original.
const ATTACH_POINTS: usize = 34;

// ---- File records ------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObjectFile<T> {
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    variant: Vec<T>,
    /// The object kind a script implements (see `KindFile`).
    #[serde(default, rename = "kind", skip_serializing_if = "Option::is_none")]
    script_kind: Option<ObjectKind>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AbsorbedFile {
    /// By obstacle kind.
    obstacle: Vec<IdSprite>,
    #[serde(default, rename = "kind", skip_serializing_if = "Option::is_none")]
    script_kind: Option<ObjectKind>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SunBeamFile {
    look: Vec<IdSprite>,
    #[serde(default, rename = "kind", skip_serializing_if = "Option::is_none")]
    script_kind: Option<ObjectKind>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShockWaveFile {
    /// By variant, the wave's first parameter.
    wave: Vec<ShockWave>,
    #[serde(default, rename = "kind", skip_serializing_if = "Option::is_none")]
    script_kind: Option<ObjectKind>,
}

/// An object kind a script implements, in its folder's `object.toml`
/// (`[kind]`: pool, index, script).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KindFile {
    kind: ObjectKind,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct IdSprite {
    id: u8,
    sprite: SpriteId,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AttachmentFile {
    attachment: Vec<AttachmentKind>,
    #[serde(default, rename = "kind", skip_serializing_if = "Option::is_none")]
    script_kind: Option<ObjectKind>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ElementsFile {
    /// Extra damage multiplier by receiver element, then hitter element.
    weakness: BTreeMap<String, [u8; 6]>,
    /// Secondary elements by chip family (families not listed add none).
    family_elements: BTreeMap<String, SecondaryElements>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CollisionFile {
    #[serde(rename = "type")]
    types: Vec<CollisionTypeRecord>,
    field_region: Vec<FieldRegionRecord>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CollisionTypeRecord {
    id: u8,
    side0: u32,
    side1: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FieldRegionRecord {
    /// The region number (0x80 and up).
    id: u8,
    require: u32,
    forbid: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PanelsFile {
    /// `#` where a panel shows at the start of a round, rows y = 0..=4,
    /// columns x = 0..=7.
    start_visible: [String; 5],
    /// `#` where a panel draws its front edge.
    front_edges: [String; 5],
    #[serde(rename = "type")]
    types: Vec<PanelTypeRecord>,
    step: StepRecord,
    dash_step: StepRecord,
    any_side_step: StepRecord,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PanelTypeRecord {
    kind: PanelType,
    flags: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    road_slide: Option<SlideVector>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct StepRecord {
    grounded: SidesRecord,
    floor_free: SidesRecord,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct SidesRecord {
    side0: PanelCondition,
    side1: PanelCondition,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StagesFile {
    settings: Vec<SettingsRecord>,
    actor_list: Vec<ActorListRecord>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsRecord {
    id: u8,
    layout: u8,
    music: u8,
    mode: u8,
    background: u8,
    battle_number: u8,
    panel_pattern: u8,
    effects: u32,
    actor_list: u8,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActorListRecord {
    id: u8,
    original_address: u32,
    actors: Vec<ActorRecord>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActorRecord {
    kind: ActorKindName,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    variant: Option<u8>,
    side: u8,
    x: u8,
    y: u8,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ActorKindName {
    Navi,
    Rock,
    Object6e,
    Object7d,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BannersFile {
    holding: Vec<BannerId>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusFile {
    hp_bug_periods: [u8; 8],
    status: Vec<StatusRecord>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusRecord {
    /// The status byte.
    id: u8,
    requests: u32,
    duration: u16,
    timer: StatusTimer,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WeaponsFile {
    buster_recovery: Vec<[u8; 6]>,
    weapon: Vec<WeaponRecord>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WeaponRecord {
    id: u8,
    charge_ticks: [u16; 5],
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReactionsFile {
    push: [SlideVector; 10],
    ice: [SlideVector; 6],
    bubble_bob: [i8; 32],
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LockonFile {
    column_shifts: Vec<i8>,
    search: Vec<LockonSearch>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CustomScreenFile {
    slot: Vec<SlotRecord>,
    left_scan_top: Vec<u8>,
    left_scan_bottom: Vec<u8>,
    right_scan_top: Vec<u8>,
    right_scan_bottom: Vec<u8>,
    left_scan_start: [u8; 12],
    right_scan_start: [u8; 12],
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SlotRecord {
    id: u8,
    kind: TemplateSlot,
    vertical: u8,
    left: u8,
    right: u8,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MathFile {
    sine: Vec<i16>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SpChipsFile {
    /// Deletion times, `[h:]m:ss.cc`.
    deletion_times: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EffectsFile {
    effect: Vec<EffectRecord>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SparksFile {
    spark: Vec<EffectRecord>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EffectRecord {
    id: u8,
    sprite: SpriteId,
    anim: u8,
    palette: u8,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegionsFile {
    region: Vec<RegionRecord>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegionRecord {
    id: u8,
    panels: Vec<PanelOffset>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LayoutsFile {
    layout: Vec<LayoutRecord>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LayoutRecord {
    id: u8,
    /// Panel types, rows y = 1..=3, columns x = 1..=6.
    rows: [[PanelType; 6]; 3],
}

// ---- Names ---------------------------------------------------------------------------

/// The receiver/hitter element names of the weakness table.
const ELEMENT_NAMES: [&str; 6] = ["null", "fire", "aqua", "elec", "wood", "drain"];

fn family_name(f: ChipFamily) -> String {
    toml_value_name(&f)
}

/// A serde enum's name, as written.
fn toml_value_name<T: Serialize>(v: &T) -> String {
    match toml::Value::try_from(v) {
        Ok(toml::Value::String(s)) => s,
        _ => unreachable!("a unit enum variant"),
    }
}

/// A folder-name slug: lower-case letters and digits, other runs as `-`.
pub fn slug(name: &str) -> String {
    let mut s = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c.to_ascii_lowercase());
        } else if !s.ends_with('-') {
            s.push('-');
        }
    }
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { "unnamed".into() } else { s }
}

/// `m:ss.cc` for a BCD deletion time (hours:minutes:seconds.hundredths).
fn bcd_time(t: u32) -> String {
    let d = |v: u32| (v >> 4) * 10 + (v & 0xF);
    let (h, m, s, c) = (d(t >> 24), d((t >> 16) & 0xFF), d((t >> 8) & 0xFF), d(t & 0xFF));
    if h != 0 { format!("{h}:{m:02}:{s:02}.{c:02}") } else { format!("{m}:{s:02}.{c:02}") }
}

fn parse_bcd_time(s: &str) -> Option<u32> {
    let (rest, c) = s.split_once('.')?;
    let parts: Vec<&str> = rest.split(':').collect();
    let num = |t: &str| t.parse::<u32>().ok().filter(|&v| v < 100);
    let (h, m, sec) = match parts.as_slice() {
        [sec] => (0, 0, num(sec)?),
        [m, sec] => (0, num(m)?, num(sec)?),
        [h, m, sec] => (num(h)?, num(m)?, num(sec)?),
        _ => return None,
    };
    let c = num(c).filter(|_| c.len() == 2)?;
    let bcd = |v: u32| (v / 10) << 4 | v % 10;
    Some(bcd(h) << 24 | bcd(m) << 16 | bcd(sec) << 8 | bcd(c))
}

fn grid(g: &[[bool; 8]; 5]) -> [String; 5] {
    std::array::from_fn(|y| g[y].iter().map(|&b| if b { '#' } else { '.' }).collect())
}

fn parse_grid(rows: &[String; 5], file: &str, what: &str, report: &mut Report) -> [[bool; 8]; 5] {
    let mut g = [[false; 8]; 5];
    for (y, row) in rows.iter().enumerate() {
        if row.chars().count() != 8 || row.chars().any(|c| c != '#' && c != '.') {
            report.error(file, format!("{what} row {y} must be 8 characters of '#' and '.', not {row:?}"));
            continue;
        }
        for (x, c) in row.chars().enumerate() {
            g[y][x] = c == '#';
        }
    }
    g
}

// ---- Writing ---------------------------------------------------------------------------

/// Keys whose integers are written in hex, and how many digits (0: as
/// few as fit, at least two).
const HEX_KEYS: &[(&str, usize)] = &[
    ("id", 0),
    ("index", 0),
    ("action", 0),
    ("music", 0),
    ("layout", 0),
    ("panel_pattern", 0),
    ("win_banner", 0),
    ("lose_banner", 0),
    ("holding", 0),
    ("break_sound", 0),
    ("name_id", 0),
    ("mode9_a", 0),
    ("a_charge", 0),
    ("buster", 0),
    ("charge_shot", 0),
    ("back_special", 0),
    ("alt_a_charge", 0),
    ("chip", 0),
    ("sequence", 0),
    ("original_address", 8),
    ("effects", 8),
    ("side0", 8),
    ("side1", 8),
    ("flags", 8),
    ("require", 8),
    ("forbid", 8),
    ("requests", 8),
];

fn hex(v: i64, digits: usize) -> toml_edit::Value {
    let text = if digits == 0 { format!("{v:#04x}") } else { format!("{v:#0w$x}", w = digits + 2) };
    text.parse().expect("a hex integer is a TOML value")
}

/// Rewrite the integers of hex keys in hex, keeping their decoration.
fn hexify_value(v: &mut toml_edit::Value, digits: usize) {
    match v {
        toml_edit::Value::Integer(i) => {
            let decor = i.decor().clone();
            let mut n = hex(*i.value(), digits);
            *n.decor_mut() = decor;
            *v = n;
        }
        toml_edit::Value::Array(a) => a.iter_mut().for_each(|x| hexify_value(x, digits)),
        _ => {}
    }
}

fn hexify_table(t: &mut dyn toml_edit::TableLike) {
    for (key, item) in t.iter_mut() {
        let digits = HEX_KEYS.iter().find(|(k, _)| *k == key.get()).map(|&(_, d)| d);
        match item {
            toml_edit::Item::Value(toml_edit::Value::InlineTable(it)) => hexify_table(it),
            toml_edit::Item::Value(toml_edit::Value::Array(a)) => {
                for v in a.iter_mut() {
                    match v {
                        toml_edit::Value::InlineTable(it) => hexify_table(it),
                        v => {
                            if let Some(d) = digits {
                                hexify_value(v, d)
                            }
                        }
                    }
                }
            }
            toml_edit::Item::Value(v) => {
                if let Some(d) = digits {
                    hexify_value(v, d)
                }
            }
            toml_edit::Item::Table(t) => hexify_table(t),
            toml_edit::Item::ArrayOfTables(a) => a.iter_mut().for_each(|t| hexify_table(t)),
            toml_edit::Item::None => {}
        }
    }
}

/// A file's text: a comment, then the record as TOML with hex ids and
/// flag words.
fn toml_file<T: Serialize>(comment: &str, value: &T) -> Vec<u8> {
    let text = toml::to_string(value).expect("content serializes as TOML");
    let mut doc: toml_edit::DocumentMut = text.parse().expect("TOML the serializer wrote parses");
    hexify_table(doc.as_table_mut());
    let mut out = String::new();
    for line in comment.lines() {
        out += if line.is_empty() { "#".into() } else { format!("# {line}") }.as_str();
        out.push('\n');
    }
    out.push('\n');
    out += &doc.to_string();
    out.into_bytes()
}

/// A navi's folder.
pub fn navi_folder(n: &NaviData) -> String {
    format!("navis/{:02x}-{}", n.id, slug(&n.name))
}

/// A chip's folder.
pub fn chip_folder(c: &ChipData) -> String {
    format!("chips/{:03x}-{}", c.id, slug(&c.name))
}

/// A form's folder.
pub fn form_folder(f: &FormData) -> String {
    format!("{FORMS_OF}/{:02x}-{}", f.id, slug(&f.name))
}

/// Where MegaMan's weapon routines are.
const WEAPONS_OF: &str = "navis/00-megaman/weapons";

/// A weapon routine's folder.
pub fn weapon_folder(w: &WeaponData) -> String {
    format!("{WEAPONS_OF}/{:02x}-{}", w.id, slug(&w.name))
}

/// The object kinds whose folders hold data of their own (a kind of these
/// a script implements keeps its `[kind]` in the same file).
const DATA_OBJECTS: [&str; 6] = ["rock", "absorbed-obstacle", "body-overlay", "sun-beam", "attachment", "shock-wave"];

/// A script as an entity's file names it: `module` (a path in the pack
/// without `.luau`) relative to `folder`, with `.luau`.
pub fn script_file(folder: &str, module: &str) -> String {
    let from: Vec<&str> = folder.split('/').collect();
    let to: Vec<&str> = module.split('/').collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<&str> = vec![".."; from.len() - common];
    parts.extend(&to[common..]);
    format!("{}.luau", parts.join("/"))
}

/// The module an entity's `script` names (a path relative to `folder`).
pub fn script_module(folder: &str, file: &str) -> Result<String, String> {
    let rel = file.strip_suffix(".luau").ok_or_else(|| format!("script {file:?} is not a .luau file"))?;
    let mut parts: Vec<&str> = folder.split('/').collect();
    for seg in rel.split('/') {
        match seg {
            "." | "" => {}
            ".." => {
                parts.pop().ok_or_else(|| format!("script {file:?} leaves the pack"))?;
            }
            s => parts.push(s),
        }
    }
    Ok(parts.join("/"))
}

/// `k` with its script as its folder names it.
fn kind_in_file(k: &ObjectKind) -> ObjectKind {
    ObjectKind { script: script_file(&format!("objects/{}", k.name), &k.script), ..k.clone() }
}

/// The battle data's files (not the sprite timing: that is written with
/// the sprites, `animations.json`).
pub fn export(c: &Content) -> Files {
    let mut files = Files::new();
    let mut put = |path: String, bytes: Vec<u8>| files.push((path, bytes));
    for chip in &c.chips {
        let comment = format!("Chip {:#05x}, {}. See docs/design/content-pack.md for what each field means.", chip.id, chip.name);
        let folder = chip_folder(chip);
        let chip = ChipData { script: chip.script.as_ref().map(|m| script_file(&folder, m)), ..chip.clone() };
        put(format!("{folder}/chip.toml"), toml_file(&comment, &chip));
    }
    for n in &c.navis {
        let comment = format!("Navi {}, {}.", n.id, n.name);
        put(format!("{}/navi.toml", navi_folder(n)), toml_file(&comment, n));
    }
    for f in &c.forms {
        let comment = format!("MegaMan's form {:#04x}, {}.", f.id, f.name);
        put(format!("{}/form.toml", form_folder(f)), toml_file(&comment, f));
    }
    for w in &c.weapons {
        let comment = format!("MegaMan's weapon routine {:#04x}, {}: a script implements it.", w.id, w.name);
        let folder = weapon_folder(w);
        let w = WeaponData { script: script_file(&folder, &w.script), ..w.clone() };
        put(format!("{folder}/weapon.toml"), toml_file(&comment, &w));
    }
    // Object kinds.
    let o = &c.objects;
    let script_kind = |name: &str| o.kinds.iter().find(|k| k.name == name).map(kind_in_file);
    put(
        "objects/rock/object.toml".into(),
        toml_file(
            "Rocks (attack object #0x59) by variant, the rock's first parameter.",
            &ObjectFile { variant: o.rocks.clone(), script_kind: script_kind("rock") },
        ),
    );
    let absorbed = o.absorbed_sprites.iter().enumerate().map(|(i, &sprite)| IdSprite { id: i as u8, sprite }).collect();
    put(
        "objects/absorbed-obstacle/object.toml".into(),
        toml_file(
            "The sprite an absorbed obstacle (effect object #0x39) flies with, by obstacle kind.",
            &AbsorbedFile { obstacle: absorbed, script_kind: script_kind("absorbed-obstacle") },
        ),
    );
    put(
        "objects/body-overlay/object.toml".into(), toml_file(
            "Body overlays (actor object #0x56) by variant: a second sprite on a navi, in front of it in the\nanimations `in_front` marks.",
            &ObjectFile { variant: o.body_overlays.clone(), script_kind: script_kind("body-overlay") },
        ),
    );
    let looks = o.sun_beam_looks.iter().enumerate().map(|(i, &sprite)| IdSprite { id: i as u8, sprite }).collect();
    put(
        "objects/sun-beam/object.toml".into(),
        toml_file(
            "The sun beam's (effect object #0x48) sprites by look, its first parameter.",
            &SunBeamFile { look: looks, script_kind: script_kind("sun-beam") },
        ),
    );
    put(
        "objects/shock-wave/object.toml".into(),
        toml_file(
            "Shock waves (attack object #0x16) by variant, its first parameter: sprite, animation, ticks on a\npanel before the next wave rolls on, and the panel type it leaves (`cracked` cracks the panel,\n`broken` breaks it or cracks it under something).",
            &ShockWaveFile { wave: o.shock_waves.clone(), script_kind: script_kind("shock-wave") },
        ),
    );
    let owned: Vec<u8> = c.chips.iter().filter_map(|c| Some(c.gun_del_sol.as_ref()?.gun.id)).collect();
    let rest = o.attachments.iter().filter(|a| !owned.contains(&a.id)).copied().collect();
    put(
        "objects/attachment/object.toml".into(), toml_file(
            "Attachments (actor object #5) by number, its first parameter: the ones no chip folder declares.",
            &AttachmentFile { attachment: rest, script_kind: script_kind("attachment") },
        ),
    );
    for k in o.kinds.iter().filter(|k| !DATA_OBJECTS.contains(&k.name.as_str())) {
        let comment = format!("{} {} object {:#04x}: a script implements it.", k.name, k.pool.name(), k.index);
        put(format!("objects/{}/object.toml", k.name), toml_file(&comment, &KindFile { kind: kind_in_file(k) }));
    }
    for (module, source) in &c.scripts.modules {
        put(format!("{module}.luau"), source.clone().into_bytes());
    }
    // Rules.
    let r = &c.rules;
    let weakness = (0..6).map(|i| (ELEMENT_NAMES[i].to_string(), r.element_weakness[i])).collect();
    let family_elements = ChipFamily::ALL
        .iter()
        .filter(|&&f| r.family_elements[f as usize].0 != 0)
        .map(|&f| (family_name(f), r.family_elements[f as usize]))
        .collect();
    put(
        "rules/elements.toml".into(), toml_file(
            "weakness.RECEIVER[hitter]: extra damage multiplier by the receiver's element, then the\nhitter's (null, fire, aqua, elec, wood, drain).\nfamily_elements: the secondary elements a chip family adds to its attacks.",
            &ElementsFile { weakness, family_elements },
        ),
    );
    let types = r.collision_types.iter().enumerate().map(|(i, t)| CollisionTypeRecord { id: i as u8, side0: t[0], side1: t[1] }).collect();
    let field_region = r
        .field_regions
        .iter()
        .enumerate()
        .map(|(i, c)| FieldRegionRecord { id: 0x80 + i as u8, require: c.require, forbid: c.forbid })
        .collect();
    put(
        "rules/collision.toml".into(), toml_file(
            "Collision types by number, for side 0 and side 1: A reacts to B when A's target type\nmeets B's self type (docs/engine/field-collision-damage.md §3.3).\nfield_region: hit region 0x80 and up covers every panel that has all of `require` and none\nof `forbid`.",
            &CollisionFile { types, field_region },
        ),
    );
    let p = &r.panels;
    let sides = |s: &StepRuleSet| StepRecord {
        grounded: SidesRecord { side0: s.grounded[0], side1: s.grounded[1] },
        floor_free: SidesRecord { side0: s.floor_free[0], side1: s.floor_free[1] },
    };
    let panel_types = PanelType::ALL
        .iter()
        .zip(&p.types)
        .map(|(&kind, t)| PanelTypeRecord { kind, flags: t.flags, road_slide: t.road_slide })
        .collect();
    put(
        "rules/panels.toml".into(), toml_file(
            "Panels: what each panel type adds to a panel's flags word (and where roads carry a navi),\nwhich panels show and draw their front edge at the start of a round, and what a panel must be\nto step onto: grounded or floor-free (AirShoes), by side.",
            &PanelsFile {
                start_visible: grid(&p.start_visible),
                front_edges: grid(&p.front_edges),
                types: panel_types,
                step: sides(&p.step),
                dash_step: sides(&p.dash_step),
                any_side_step: sides(&p.any_side_step),
            },
        ),
    );
    let s = &r.stages;
    let settings = s
        .settings
        .iter()
        .enumerate()
        .map(|(i, b)| SettingsRecord {
            id: i as u8,
            layout: b.layout,
            music: b.music,
            mode: b.mode,
            background: b.background,
            battle_number: b.battle_number,
            panel_pattern: b.panel_pattern,
            effects: b.effects,
            actor_list: b.actors.0,
        })
        .collect();
    let actor_list = s
        .actor_lists
        .iter()
        .enumerate()
        .map(|(i, l)| ActorListRecord {
            id: i as u8,
            original_address: l.original_address,
            actors: l.entries.iter().map(actor_record).collect(),
        })
        .collect();
    put(
        "rules/stages.toml".into(), toml_file(
            "Battle settings (a set's later rounds are drawn from them) and the actor lists they spawn.\nAn actor list's original_address is how the original's battle settings name it (what link\ndata and traces carry).",
            &StagesFile { settings, actor_list },
        ),
    );
    put(
        "rules/banners.toml".into(), toml_file("Banners (by banner id) that stay up until removed.", &BannersFile { holding: r.holding_banners.clone() }),
    );
    let status = r
        .status_effects
        .iter()
        .enumerate()
        .flat_map(|(g, row)| {
            row.iter().enumerate().map(move |(k, e)| StatusRecord {
                id: ((g as u8 + 1) << 4) | k as u8,
                requests: e.requests,
                duration: e.duration,
                timer: e.timer,
            })
        })
        .collect();
    put(
        "rules/status.toml".into(), toml_file(
            "Status effects by status byte: the requests they raise, how long, and which timer.\nStatuses past a group's real entries read what follows, as in the game ({ other = N }\nnames another collision field).\nhp_bug_periods: the HP bug's drain period by bug level.",
            &StatusFile { hp_bug_periods: r.hp_bug_periods, status },
        ),
    );
    let weapon = r.weapons.iter().enumerate().map(|(i, w)| WeaponRecord { id: i as u8, charge_ticks: w.charge_ticks }).collect();
    put(
        "rules/weapons.toml".into(), toml_file(
            "Weapon routines by number: ticks to a full charge by Charge stat (0..4; a Charge past 4 reads\nthe next routine's). buster_recovery: ticks after a buster shot by Rapid stat, then by open\npanels ahead (0..5).",
            &WeaponsFile { buster_recovery: r.buster_recovery.clone(), weapon },
        ),
    );
    put(
        "rules/reactions.toml".into(), toml_file(
            "push: slides by hit-modifier bit (+5 with 0x80); ice: slides by the direction the navi last\nmoved (none, up, down, back, forward, other); panels = 6 slides until blocked.\nbubble_bob: a bubbled navi's height by bubble timer.",
            &ReactionsFile { push: r.push_vectors, ice: r.ice_vectors, bubble_bob: r.bubble_bob },
        ),
    );
    put(
        "rules/lockon.toml".into(), toml_file(
            "The Beast Out lock-on: for the chips' lock-on modes that search, the panels next to the\ntarget tried (dx toward the user's front), whether the middle row is taken afterwards, and the\ncolumn shifts tried when nothing fits.",
            &LockonFile { column_shifts: r.lockon.column_shifts.clone(), search: r.lockon.searches.clone() },
        ),
    );
    let cs = &r.custom_screen;
    put(
        "rules/custom-screen.toml".into(),
        toml_file(
            "The custom screen's slot grid (slots 0-4 the top row, 5-9 the bottom row, 10 OK, 11 the button
under it): what each slot starts as and its neighbours (vertical is both UP and DOWN). A missing
neighbour is looked for along the scan lists, each slot starting at its *_scan_start.",
            &CustomScreenFile {
                slot: cs
                    .slots
                    .iter()
                    .enumerate()
                    .map(|(i, s)| SlotRecord { id: i as u8, kind: s.kind, vertical: s.vertical, left: s.left, right: s.right })
                    .collect(),
                left_scan_top: cs.left_scan_top.clone(),
                left_scan_bottom: cs.left_scan_bottom.clone(),
                right_scan_top: cs.right_scan_top.clone(),
                right_scan_bottom: cs.right_scan_bottom.clone(),
                left_scan_start: cs.left_scan_start,
                right_scan_start: cs.right_scan_start,
            },
        ),
    );
    put(
        "rules/sp-chips.toml".into(), toml_file(
            "The deletion times at which an SP navi chip's damage steps down (chips' sp_damage).",
            &SpChipsFile { deletion_times: r.sp_deletion_times.iter().map(|&t| bcd_time(t)).collect() },
        ),
    );
    put(
        "rules/math.toml".into(), toml_file(
            "The sine table (math_sinTable, which math_cosTable continues): 256 steps a turn, 1.0 = 0x100,\nover a turn and a half (the cosine of step a is entry a + 64).",
            &MathFile { sine: r.sine.clone() },
        ),
    );
    // Registries.
    let effect = |i: usize, e: &EffectSprite| EffectRecord { id: i as u8, sprite: e.sprite, anim: e.anim, palette: e.palette };
    put(
        "registries/effects.toml".into(), toml_file(
            "Generic one-shot effects (effect object #0) by effect id: the sprite animation each plays.",
            &EffectsFile { effect: c.effects.iter().enumerate().map(|(i, e)| effect(i, e)).collect() },
        ),
    );
    put(
        "registries/sparks.toml".into(), toml_file(
            "Hit sparks by hit-effect id.",
            &SparksFile { spark: c.sparks.iter().enumerate().map(|(i, e)| effect(i, e)).collect() },
        ),
    );
    put(
        "registries/regions.toml".into(), toml_file(
            "Hit regions by region number: the panels a hit covers, [dx, dy] from its panel with dx toward\nthe facing side, in the order hits are processed.",
            &RegionsFile {
                region: c.regions.iter().enumerate().map(|(i, p)| RegionRecord { id: i as u8, panels: p.clone() }).collect(),
            },
        ),
    );
    put(
        "registries/panel-layouts.toml".into(), toml_file(
            "Panel layouts by layout number (battle settings' layout): panel types, rows y = 1..3,\ncolumns x = 1..6.",
            &LayoutsFile {
                layout: c.panel_layouts.iter().enumerate().map(|(i, l)| LayoutRecord { id: i as u8, rows: l.rows }).collect(),
            },
        ),
    );
    files
}

/// Sprite timing alone, for a pack without graphics: each sprite's
/// `animations.json` with only its frames' ticks and flags (a pack with
/// graphics writes these files with the sprites, `crate::sprite::export`).
pub fn export_timing(c: &Content) -> Files {
    c.animations
        .sprites
        .iter()
        .map(|(id, anims)| {
            let doc = crate::sprite::AnimationsDoc {
                format: crate::sprite::ANIMATIONS_FORMAT.into(),
                version: crate::sprite::VERSION,
                sprite: [id.category, id.index],
                animations: anims
                    .iter()
                    .map(|a| {
                        a.iter()
                            .map(|f| crate::sprite::FrameDoc {
                                ticks: f.duration,
                                flags: crate::sprite::flags_doc(f.flags),
                                tileset: 0,
                                layout: 0,
                                palettes: 0,
                            })
                            .collect()
                    })
                    .collect(),
            };
            let path = format!("graphics/sprites/{}/animations.json", crate::sprite::folder_name(id.category, id.index));
            (path, serde_json::to_vec(&doc).expect("animations serialize"))
        })
        .collect()
}

fn actor_record(e: &ActorEntry) -> ActorRecord {
    let (kind, variant) = match e.kind {
        ActorKind::Navi => (ActorKindName::Navi, None),
        ActorKind::Rock { variant } => (ActorKindName::Rock, Some(variant)),
        ActorKind::Object6E => (ActorKindName::Object6e, None),
        ActorKind::Object7D { variant } => (ActorKindName::Object7d, Some(variant)),
    };
    ActorRecord { kind, variant, side: e.alliance, x: e.x, y: e.y }
}

// ---- Reading -----------------------------------------------------------------------------

/// Read a TOML file of the pack.
fn read_toml<T: DeserializeOwned>(root: &Path, file: &str, report: &mut Report) -> Option<T> {
    let text = match std::fs::read_to_string(root.join(file)) {
        Ok(t) => t,
        Err(e) => {
            report.error(file, format!("can't read: {e}"));
            return None;
        }
    };
    match toml::from_str(&text) {
        Ok(v) => Some(v),
        Err(e) => {
            report.error(file, format!("invalid: {e}"));
            None
        }
    }
}

/// The folders under `dir` (sorted), each with the file `name` in it.
fn entity_files(root: &Path, dir: &str, name: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(root.join(dir))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join(name).is_file())
        .map(|e| format!("{dir}/{}/{name}", e.file_name().to_string_lossy()))
        .collect();
    v.sort();
    v
}

/// Records with ids into a dense table 0..n: duplicates and gaps are
/// errors naming the file.
fn dense<T>(items: Vec<(usize, T, String)>, what: &str, report: &mut Report) -> Vec<T> {
    let mut slots: BTreeMap<usize, (T, String)> = BTreeMap::new();
    for (id, v, file) in items {
        if let Some((_, first)) = slots.get(&id) {
            report.error(&file, format!("{what} {id:#x} is also in {first}"));
            continue;
        }
        slots.insert(id, (v, file));
    }
    let mut out = Vec::with_capacity(slots.len());
    for (expect, (id, (v, file))) in slots.into_iter().enumerate() {
        if id != expect {
            report.error(&file, format!("{what} {id:#x} leaves a gap: {what}s must be numbered from 0 without gaps ({expect:#x} is missing)"));
            return out;
        }
        out.push(v);
    }
    out
}

/// Read the battle data of a pack (the sprite timing included).
pub fn load(root: &Path, report: &mut Report) -> Option<Content> {
    let errors_before = report.count(crate::report::Level::Error);
    // Chips.
    let mut chips = Vec::new();
    for file in entity_files(root, "chips", "chip.toml") {
        if let Some(mut c) = read_toml::<ChipData>(root, &file, report) {
            let folder = file.trim_end_matches("/chip.toml");
            if let Some(s) = &c.script {
                match script_module(folder, s) {
                    Ok(m) => c.script = Some(m),
                    Err(e) => report.error(&file, e),
                }
            }
            chips.push((c.id as usize, c, file));
        }
    }
    let chips = dense(chips, "chip", report);
    // Navis and forms.
    let mut navis = Vec::new();
    for file in entity_files(root, "navis", "navi.toml") {
        if let Some(n) = read_toml::<NaviData>(root, &file, report) {
            navis.push((n.id as usize, n, file));
        }
    }
    let navis = dense(navis, "navi", report);
    let mut forms = Vec::new();
    for file in entity_files(root, FORMS_OF, "form.toml") {
        if let Some(f) = read_toml::<FormData>(root, &file, report) {
            forms.push((f.id as usize, f, file));
        }
    }
    let forms = dense(forms, "form", report);
    let mut names = BTreeMap::new();
    for (id, file) in navis
        .iter()
        .filter_map(|n| Some((n.name_record.as_ref()?.id, navi_folder(n))))
        .chain(forms.iter().filter_map(|f| Some((f.name_record.as_ref()?.id, form_folder(f)))))
    {
        if let Some(first) = names.insert(id, file.clone()) {
            report.error(file, format!("NameID {id:#x} is also {first}'s"));
        }
    }
    // Objects.
    let objects = load_objects(root, &chips, report)?;
    let rules = load_rules(root, report)?;
    // Registries.
    let effects: EffectsFile = read_toml(root, "registries/effects.toml", report)?;
    let effects = dense(
        effects.effect.into_iter().map(|e| (e.id as usize, EffectSprite { sprite: e.sprite, anim: e.anim, palette: e.palette }, "registries/effects.toml".into())).collect(),
        "effect",
        report,
    );
    let sparks: SparksFile = read_toml(root, "registries/sparks.toml", report)?;
    let sparks = dense(
        sparks.spark.into_iter().map(|e| (e.id as usize, EffectSprite { sprite: e.sprite, anim: e.anim, palette: e.palette }, "registries/sparks.toml".into())).collect(),
        "spark",
        report,
    );
    let regions: RegionsFile = read_toml(root, "registries/regions.toml", report)?;
    let regions = dense(regions.region.into_iter().map(|r| (r.id as usize, r.panels, "registries/regions.toml".into())).collect(), "region", report);
    let layouts: LayoutsFile = read_toml(root, "registries/panel-layouts.toml", report)?;
    let panel_layouts = dense(
        layouts.layout.into_iter().map(|l| (l.id as usize, PanelLayout { rows: l.rows }, "registries/panel-layouts.toml".into())).collect(),
        "panel layout",
        report,
    );
    let animations = load_animations(root, report)?;
    let weapons = load_weapons(root, report);
    let scripts = load_scripts(root, report);
    let content =
        Content { chips, navis, forms, rules, objects, effects, sparks, regions, panel_layouts, animations, weapons, scripts };
    check_references(&content, report);
    (report.count(crate::report::Level::Error) == errors_before).then_some(content)
}

fn load_objects(root: &Path, chips: &[ChipData], report: &mut Report) -> Option<ObjectData> {
    let mut kinds = Vec::new();
    let mut add_kind = |name: &str, k: Option<ObjectKind>, report: &mut Report| {
        let Some(k) = k else { return };
        match script_module(&format!("objects/{name}"), &k.script) {
            Ok(script) => kinds.push(ObjectKind { name: name.to_string(), script, ..k }),
            Err(e) => report.error(format!("objects/{name}/object.toml"), e),
        }
    };
    let rocks: ObjectFile<RockKind> = read_toml(root, "objects/rock/object.toml", report)?;
    add_kind("rock", rocks.script_kind, report);
    let file = "objects/rock/object.toml";
    let rocks = dense(rocks.variant.into_iter().map(|r| (r.id as usize, r, file.into())).collect(), "rock variant", report);
    let absorbed: AbsorbedFile = read_toml(root, "objects/absorbed-obstacle/object.toml", report)?;
    add_kind("absorbed-obstacle", absorbed.script_kind, report);
    let file = "objects/absorbed-obstacle/object.toml";
    let absorbed_sprites = dense(absorbed.obstacle.into_iter().map(|k| (k.id as usize, k.sprite, file.into())).collect(), "obstacle kind", report);
    let overlays: ObjectFile<BodyOverlay> = read_toml(root, "objects/body-overlay/object.toml", report)?;
    add_kind("body-overlay", overlays.script_kind, report);
    let file = "objects/body-overlay/object.toml";
    let body_overlays = dense(overlays.variant.into_iter().map(|o| (o.id as usize, o, file.into())).collect(), "body overlay", report);
    let beams: SunBeamFile = read_toml(root, "objects/sun-beam/object.toml", report)?;
    add_kind("sun-beam", beams.script_kind, report);
    let file = "objects/sun-beam/object.toml";
    let sun_beam_looks = dense(beams.look.into_iter().map(|l| (l.id as usize, l.sprite, file.into())).collect(), "sun beam look", report);
    let waves: ShockWaveFile = read_toml(root, "objects/shock-wave/object.toml", report)?;
    add_kind("shock-wave", waves.script_kind, report);
    let file = "objects/shock-wave/object.toml";
    let shock_waves = dense(waves.wave.into_iter().map(|w| (w.id as usize, w, file.into())).collect(), "shock wave", report);
    // Attachments: the chips' own and the rest. A chip may share another's
    // (the same row), but not change it.
    let rest: AttachmentFile = read_toml(root, "objects/attachment/object.toml", report)?;
    add_kind("attachment", rest.script_kind, report);
    // The kinds without data of their own.
    for file in entity_files(root, "objects", "object.toml") {
        let name = file.trim_start_matches("objects/").trim_end_matches("/object.toml").to_string();
        if DATA_OBJECTS.contains(&name.as_str()) {
            continue;
        }
        if let Some(k) = read_toml::<KindFile>(root, &file, report) {
            add_kind(&name, Some(k.kind), report);
        }
    }
    let mut all: BTreeMap<u8, (AttachmentKind, String)> = BTreeMap::new();
    let declared = rest
        .attachment
        .into_iter()
        .map(|a| (a, "objects/attachment/object.toml".to_string()))
        .chain(chips.iter().filter_map(|c| Some((c.gun_del_sol.as_ref()?.gun, format!("{}/chip.toml", chip_folder(c))))));
    for (a, file) in declared {
        match all.get(&a.id) {
            Some((b, first)) if *b != a => {
                report.error(&file, format!("attachment {:#04x} differs from the one {first} declares", a.id))
            }
            Some(_) => {}
            None => {
                all.insert(a.id, (a, file));
            }
        }
    }
    let attachments = dense(all.into_iter().map(|(id, (a, file))| (id as usize, a, file)).collect(), "attachment", report);
    kinds.sort_by(|a, b| a.name.cmp(&b.name));
    Some(ObjectData { attachments, rocks, absorbed_sprites, body_overlays, sun_beam_looks, kinds, shock_waves })
}

fn load_rules(root: &Path, report: &mut Report) -> Option<Rules> {
    // Elements.
    let file = "rules/elements.toml";
    let e: ElementsFile = read_toml(root, file, report)?;
    let mut element_weakness = [[0u8; 6]; 6];
    for (name, row) in &e.weakness {
        match ELEMENT_NAMES.iter().position(|n| n == name) {
            Some(i) => element_weakness[i] = *row,
            None => report.error(file, format!("weakness.{name}: not an element ({})", ELEMENT_NAMES.join(", "))),
        }
    }
    let mut family_elements = [SecondaryElements::default(); 13];
    for (name, bits) in &e.family_elements {
        match ChipFamily::ALL.iter().find(|&&f| family_name(f) == *name) {
            Some(&f) => family_elements[f as usize] = *bits,
            None => report.error(file, format!("family_elements.{name}: not a chip family")),
        }
    }
    // Collision.
    let file = "rules/collision.toml";
    let c: CollisionFile = read_toml(root, file, report)?;
    let collision_types = dense(c.types.into_iter().map(|t| (t.id as usize, [t.side0, t.side1], file.into())).collect(), "collision type", report);
    let mut regions = Vec::new();
    for r in c.field_region {
        match r.id.checked_sub(0x80) {
            Some(i) => regions.push((i as usize, PanelCondition { require: r.require, forbid: r.forbid }, file.to_string())),
            None => report.error(file, format!("field region {:#04x}: field regions are numbered from 0x80", r.id)),
        }
    }
    let field_regions = dense(regions, "field region", report);
    // Panels.
    let file = "rules/panels.toml";
    let p: PanelsFile = read_toml(root, file, report)?;
    let mut types = vec![PanelTypeRule::default(); PanelType::ALL.len()];
    let mut seen = [false; 13];
    for t in &p.types {
        let i = t.kind as usize;
        if std::mem::replace(&mut seen[i], true) {
            report.error(file, format!("panel type {} is listed twice", toml_value_name(&t.kind)));
        }
        types[i] = PanelTypeRule { flags: t.flags, road_slide: t.road_slide };
    }
    if let Some(i) = seen.iter().position(|s| !s) {
        report.error(file, format!("panel type {} is missing", toml_value_name(&PanelType::ALL[i])));
    }
    let rule_set = |s: &StepRecord| StepRuleSet {
        grounded: [s.grounded.side0, s.grounded.side1],
        floor_free: [s.floor_free.side0, s.floor_free.side1],
    };
    let panels = PanelRules {
        types,
        start_visible: parse_grid(&p.start_visible, file, "start_visible", report),
        front_edges: parse_grid(&p.front_edges, file, "front_edges", report),
        step: rule_set(&p.step),
        dash_step: rule_set(&p.dash_step),
        any_side_step: rule_set(&p.any_side_step),
    };
    // Stages.
    let file = "rules/stages.toml";
    let s: StagesFile = read_toml(root, file, report)?;
    let mut lists = Vec::new();
    for l in s.actor_list {
        let mut entries = Vec::new();
        for a in &l.actors {
            let kind = match (a.kind, a.variant) {
                (ActorKindName::Navi, None) => ActorKind::Navi,
                (ActorKindName::Object6e, None) => ActorKind::Object6E,
                (ActorKindName::Rock, Some(variant)) => ActorKind::Rock { variant },
                (ActorKindName::Object7d, Some(variant)) => ActorKind::Object7D { variant },
                (k, _) => {
                    let has = if a.variant.is_some() { "has" } else { "needs" };
                    report.error(file, format!("actor list {}: a {} {has} a variant", l.id, toml_value_name(&k)));
                    continue;
                }
            };
            entries.push(ActorEntry { kind, alliance: a.side, x: a.x, y: a.y });
        }
        lists.push((l.id as usize, ActorList { original_address: l.original_address, entries }, file.to_string()));
    }
    let actor_lists = dense(lists, "actor list", report);
    let settings = dense(
        s.settings
            .into_iter()
            .map(|b| {
                let settings = BattleSettings {
                    layout: b.layout,
                    music: b.music,
                    mode: b.mode,
                    background: b.background,
                    battle_number: b.battle_number,
                    panel_pattern: b.panel_pattern,
                    effects: b.effects,
                    actors: ActorListId(b.actor_list),
                };
                (b.id as usize, settings, file.to_string())
            })
            .collect(),
        "battle settings",
        report,
    );
    let stages = Stages { settings, actor_lists };
    // Banners, statuses, weapons, reactions, lock-on, SP chips.
    let banners: BannersFile = read_toml(root, "rules/banners.toml", report)?;
    let file = "rules/status.toml";
    let st: StatusFile = read_toml(root, file, report)?;
    let mut groups: BTreeMap<u8, [Option<StatusEffect>; 16]> = BTreeMap::new();
    for s in st.status {
        let e = StatusEffect { requests: s.requests, duration: s.duration, timer: s.timer };
        let Some(g) = (s.id >> 4).checked_sub(1) else {
            report.error(file, format!("status {:#04x}: statuses start at 0x10", s.id));
            continue;
        };
        let slot = &mut groups.entry(g).or_default()[(s.id & 0xF) as usize];
        if slot.replace(e).is_some() {
            report.error(file, format!("status {:#04x} is listed twice", s.id));
        }
    }
    let mut status_effects = Vec::new();
    for (expect, (g, row)) in groups.into_iter().enumerate() {
        if g as usize != expect || row.iter().any(Option::is_none) {
            report.error(file, format!("status group {:#04x}: statuses must fill whole groups of 16 from 0x10", (g + 1) << 4));
            break;
        }
        status_effects.push(row.map(|e| e.expect("checked")));
    }
    let file = "rules/weapons.toml";
    let w: WeaponsFile = read_toml(root, file, report)?;
    let weapons = dense(w.weapon.into_iter().map(|w| (w.id as usize, WeaponRoutine { charge_ticks: w.charge_ticks }, file.into())).collect(), "weapon routine", report);
    let reactions: ReactionsFile = read_toml(root, "rules/reactions.toml", report)?;
    let lockon: LockonFile = read_toml(root, "rules/lockon.toml", report)?;
    let math: MathFile = read_toml(root, "rules/math.toml", report)?;
    let file = "rules/sp-chips.toml";
    let sp: SpChipsFile = read_toml(root, file, report)?;
    let mut sp_deletion_times = Vec::new();
    for t in &sp.deletion_times {
        match parse_bcd_time(t) {
            Some(v) => sp_deletion_times.push(v),
            None => report.error(file, format!("deletion time {t:?} is not [h:]m:ss.cc")),
        }
    }
    let file = "rules/custom-screen.toml";
    let cs: CustomScreenFile = read_toml(root, file, report)?;
    let slots = dense(
        cs.slot.into_iter().map(|s| (s.id as usize, SlotLayout { kind: s.kind, vertical: s.vertical, left: s.left, right: s.right }, file.into())).collect(),
        "slot",
        report,
    );
    let slots: [SlotLayout; 12] = match slots.try_into() {
        Ok(s) => s,
        Err(v) => {
            report.error(file, format!("{} slots; the screen has 12", Vec::len(&v)));
            return None;
        }
    };
    let custom_screen = CustomScreenLayout {
        slots,
        left_scan_top: cs.left_scan_top,
        left_scan_bottom: cs.left_scan_bottom,
        right_scan_top: cs.right_scan_top,
        right_scan_bottom: cs.right_scan_bottom,
        left_scan_start: cs.left_scan_start,
        right_scan_start: cs.right_scan_start,
    };
    Some(Rules {
        custom_screen,
        element_weakness,
        family_elements,
        collision_types,
        field_regions,
        panels,
        stages,
        holding_banners: banners.holding,
        status_effects,
        hp_bug_periods: st.hp_bug_periods,
        weapons,
        buster_recovery: w.buster_recovery,
        sp_deletion_times,
        sine: math.sine,
        push_vectors: reactions.push,
        ice_vectors: reactions.ice,
        bubble_bob: reactions.bubble_bob,
        lockon: Lockon { searches: lockon.search, column_shifts: lockon.column_shifts },
    })
}

/// Every sprite's animation timing, from `animations.json`.
fn load_animations(root: &Path, report: &mut Report) -> Option<Animations> {
    let t = crate::timing::load(root, report)?;
    let sprites = t
        .sprites
        .into_iter()
        .map(|((category, index), anims)| {
            let anims = anims.into_iter().map(|a| a.into_iter().map(|f| AnimFrame { duration: f.ticks, flags: f.flags }).collect()).collect();
            (SpriteId { category, index }, anims)
        })
        .collect();
    Some(Animations { sprites })
}

/// MegaMan's weapon routines that scripts implement.
fn load_weapons(root: &Path, report: &mut Report) -> Vec<WeaponData> {
    let mut out: Vec<WeaponData> = Vec::new();
    for file in entity_files(root, WEAPONS_OF, "weapon.toml") {
        let Some(mut w) = read_toml::<WeaponData>(root, &file, report) else { continue };
        if let Some(first) = out.iter().find(|o| o.id == w.id) {
            report.error(&file, format!("weapon routine {:#04x} is also {}'s", w.id, weapon_folder(first)));
            continue;
        }
        match script_module(file.trim_end_matches("/weapon.toml"), &w.script) {
            Ok(m) => w.script = m,
            Err(e) => report.error(&file, e),
        }
        out.push(w);
    }
    out.sort_by_key(|w| w.id);
    out
}

/// The pack's scripts: every `.luau` file but the definition files
/// (`.d.luau`), by path without `.luau`. (Graphics and sound hold none.)
pub fn load_scripts(root: &Path, report: &mut Report) -> Scripts {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>, report: &mut Report) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        paths.sort();
        for path in paths {
            let rel = path.strip_prefix(root).expect("walked under the root");
            let key = rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
            if path.is_dir() {
                if key != "graphics" && key != "sound" {
                    walk(root, &path, out, report);
                }
                continue;
            }
            let Some(module) = key.strip_suffix(".luau") else { continue };
            if module.ends_with(".d") {
                continue;
            }
            match std::fs::read_to_string(&path) {
                Ok(source) => {
                    out.insert(module.to_string(), source);
                }
                Err(e) => report.error(&key, format!("can't read: {e}")),
            }
        }
    }
    let mut modules = BTreeMap::new();
    walk(root, root, &mut modules, report);
    Scripts { modules }
}

/// References between records that must resolve.
fn check_references(c: &Content, report: &mut Report) {
    if let Err(e) = c.registrations() {
        report.error("scripts", e);
    }
    for (i, s) in c.rules.stages.settings.iter().enumerate() {
        if s.actors.0 as usize >= c.rules.stages.actor_lists.len() {
            report.error("rules/stages.toml", format!("battle settings {i:#04x}: actor list {} doesn't exist", s.actors.0));
        }
        if s.layout as usize >= c.panel_layouts.len() {
            report.error("rules/stages.toml", format!("battle settings {i:#04x}: panel layout {:#04x} doesn't exist", s.layout));
        }
    }
    for chip in &c.chips {
        let file = format!("{}/chip.toml", chip_folder(chip));
        if let Some(g) = &chip.gun_del_sol {
            for look in [g.beam.look, g.beam_in_sun.look] {
                if look as usize >= c.objects.sun_beam_looks.len() {
                    report.error(&file, format!("sun beam look {look} isn't in objects/sun-beam/object.toml"));
                }
            }
        }
        if chip.damage > 1000 && chip.damage <= 1000 + 18 && chip.sp_damage.is_none() {
            report.error(&file, "an SP navi chip (damage formula 1..=18) needs sp_damage");
        }
        if (1024..=1044).contains(&chip.damage) && chip.navi_damage.is_none() {
            report.error(&file, "a link navi's chip (damage formula 24..=44) needs navi_damage");
        }
        if let Some(d) = &chip.sp_damage
            && d.len() != c.rules.sp_deletion_times.len() + 1
        {
            report.error(
                &file,
                format!("sp_damage has {} entries; one more than rules/sp-chips.toml's {} deletion times", d.len(), c.rules.sp_deletion_times.len()),
            );
        }
    }
    for n in c.navis.iter().filter_map(|n| n.name_record.as_ref()).chain(c.forms.iter().filter_map(|f| f.name_record.as_ref())) {
        if n.attach_points.len() < ATTACH_POINTS {
            report.warn(format!("NameID {:#x}", n.id), format!("{} attach points; the game has 34", n.attach_points.len()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deletion_times_read_back() {
        for t in [0x1000, 0x2800, 0x0159_5999, 0x0001_0000] {
            assert_eq!(parse_bcd_time(&bcd_time(t)), Some(t), "{}", bcd_time(t));
        }
        assert_eq!(bcd_time(0x1000), "0:10.00");
        assert_eq!(parse_bcd_time("10.5"), None);
    }

    #[test]
    fn slugs_are_folder_names() {
        assert_eq!(slug("HeatMan[SP]"), "heatman-sp");
        assert_eq!(slug("M-Cannon"), "m-cannon");
        assert_eq!(slug("??"), "unnamed");
    }
}
