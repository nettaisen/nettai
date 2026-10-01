//! The tables the ruleset and v1 modules still read by number, built from
//! what the content defines (docs/design/content-model-v2.md §12, step 5):
//! the pack's chips, navis and forms by number, weapons' charge times,
//! the rule sections, collision types, statuses and lock-on modes, and the numbered tables (effects,
//! sparks, regions, the object kinds' rows).
//!
//! Where a definition still carries what only registration by number reads
//! (a chip's number, action, subtype and v1 module; a navi's number and
//! NameID; a table's original numbering), it sits in a `legacy { ... }`
//! marker, which goes when its family converts (§12, phase B) or the
//! ruleset stops asking numbers (phase C).
//!
//! Content without these definitions (the engine's test content, whose
//! tables are Rust) keeps its tables: each part is built only when the
//! content defines it.

use std::collections::{BTreeMap, HashMap};

use bn6_content_api::{AssetKind, AssetNames, ContentError, Data, DataKey, Definition, Definitions, Registry};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value as Json};

use super::*;
use crate::field::PanelType;

/// What registration by number reads of the chips, navis, forms and
/// weapons the content defines, by their definitions' keys.
#[derive(Clone, Debug, Default)]
pub struct Legacy {
    /// The pack's chips: each definition's record, with its number.
    pub chips: BTreeMap<String, ChipData>,
    pub navis: BTreeMap<String, NaviData>,
    pub forms: BTreeMap<String, FormData>,
    pub weapons: BTreeMap<String, LegacyWeapon>,
    /// Charge times of the weapon routine numbers no weapon names.
    pub routine_charges: BTreeMap<u8, Vec<u16>>,
}

/// A weapon registration by number reads: its routine numbers, the v1
/// module that implements it and what else that module implements.
#[derive(Clone, Debug, Default)]
pub struct LegacyWeapon {
    pub name: String,
    pub routines: Vec<u8>,
    pub script: Option<String>,
    pub action: Option<u8>,
    pub instant_chip: Option<u8>,
    pub charge_ticks: Vec<u16>,
}

fn err(d: &Definition, e: impl std::fmt::Display) -> ContentError {
    ContentError::new(format!("{}.luau: {} {}: {e}", d.module, d.registry, d.key))
}

/// How the definitions' values read as the tables' data: assets as the
/// engine identifies them, references as the original numbers of what they
/// name.
pub struct Resolver<'a> {
    assets: &'a AssetNames,
    numbers: HashMap<(Registry, String), i64>,
}

impl<'a> Resolver<'a> {
    /// The numbers of the chips, navis, forms, weapons, collision types,
    /// statuses and lock-on modes the definitions give (their `legacy`
    /// markers, a collision type's row).
    pub fn new(assets: &'a AssetNames, definitions: &Definitions) -> Resolver<'a> {
        let mut numbers = HashMap::new();
        let mut put = |d: &Definition, n: Option<i64>| {
            if let Some(n) = n {
                numbers.insert((d.registry, d.key.clone()), n);
            }
        };
        for d in definitions.of(Registry::Chip).iter().chain(definitions.of(Registry::Navi)).chain(definitions.of(Registry::Form)) {
            put(d, d.spec.field("legacy").field("number").int());
        }
        for d in definitions.of(Registry::Weapon) {
            put(d, d.spec.field("legacy").field("routines").item(1).int());
        }
        // A lock-on mode reads as its handle (its place among the
        // definitions, which are in key order).
        for (i, d) in definitions.of(Registry::Lockon).iter().enumerate() {
            put(d, Some(i as i64));
        }
        for d in definitions.of(Registry::Collision) {
            put(d, d.spec.field("row_offset").int().map(|o| o / 8));
        }
        Resolver { assets, numbers }
    }

    /// The number of the definition `key` of `registry`.
    pub fn number(&self, registry: Registry, key: &str) -> Option<i64> {
        self.numbers.get(&(registry, key.to_string())).copied()
    }

    /// A sprite asset's identity.
    pub fn sprite(&self, name: &str) -> Option<SpriteId> {
        self.assets.sprites.get(name).copied()
    }

    /// `d` as the data a table's record reads (serde's form).
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
            Data::Ref(registry, key) => match self.number(*registry, key) {
                Some(n) => Json::from(n),
                None => return Err(format!("{at}: {registry} {key:?} has no number the tables can hold")),
            },
            Data::Asset(AssetKind::Sprite, name) => match self.sprite(name) {
                Some(s) => Json::String(s.to_string()),
                None => return Err(format!("{at}: the pack has no sprite {name:?}")),
            },
            Data::Asset(kind, name) => {
                let h = self.assets.handle(*kind, name).ok_or_else(|| format!("{at}: the pack has no {kind} {name:?}"))?;
                Json::from(self.assets.number(*kind, h).expect("a handle's asset"))
            }
            Data::Function => return Err(format!("{at}: a function isn't data")),
        })
    }

    /// `d` read as `T`.
    pub fn read<T: DeserializeOwned>(&self, d: &Data, at: &str) -> Result<T, String> {
        let j = self.json(d, at)?;
        serde_json::from_value(j).map_err(|e| format!("{at}: {e}"))
    }
}

// ---- Rule sections ------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ElementsSection {
    weakness: BTreeMap<String, [u8; 6]>,
    #[serde(default)]
    family_elements: BTreeMap<String, SecondaryElements>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PanelTypeSection {
    flags: u32,
    #[serde(default)]
    road_slide: Option<SlideVector>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct StepSection {
    grounded: [PanelCondition; 2],
    floor_free: [PanelCondition; 2],
}

impl StepSection {
    fn rules(self) -> StepRuleSet {
        StepRuleSet { grounded: self.grounded, floor_free: self.floor_free }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PanelsSection {
    types: BTreeMap<String, PanelTypeSection>,
    start_visible: [[bool; 8]; 5],
    front_edges: [[bool; 8]; 5],
    step: StepSection,
    dash_step: StepSection,
    any_side_step: StepSection,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReactionsSection {
    push: [SlideVector; 10],
    ice: [SlideVector; 6],
    bubble_bob: [i8; 32],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BerserkSection {
    step: StepSection,
    opponent: [PanelCondition; 2],
    blocking: [u32; 2],
    opposing_player: [u32; 2],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MathSection {
    sine: Vec<i16>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CustomScreenSection {
    slots: [SlotLayout; 12],
    left_scan_top: Vec<u8>,
    left_scan_bottom: Vec<u8>,
    right_scan_top: Vec<u8>,
    right_scan_bottom: Vec<u8>,
    left_scan_start: [u8; 12],
    right_scan_start: [u8; 12],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BusterSection {
    recovery: Vec<[u8; 6]>,
    empty_hand: EmptyHandChip,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BannersSection {
    #[serde(default)]
    holding: Vec<BannerId>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusSection {
    hp_bug_periods: [u8; 8],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LockonSection {
    column_shifts: Vec<i8>,
    clear_path: [PanelCondition; 2],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpChipsSection {
    /// BCD hours:minutes:seconds.hundredths.
    deletion_times: Vec<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActorRecordRow {
    version: u8,
    actor_type: crate::actor::ActorType,
    ai_index: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NameLookRow {
    #[serde(default)]
    sprite: Option<SpriteId>,
    anim: u8,
    palette: u8,
    shadow: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EffectRow {
    sprite: SpriteId,
    #[serde(default)]
    anim: u8,
    #[serde(default)]
    palette: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FieldRegionRow {
    require: u32,
    forbid: u32,
}

/// The names the elements section uses for the weakness table's rows.
const ELEMENT_NAMES: [&str; 6] = ["null", "fire", "aqua", "elec", "wood", "drain"];

/// A serde enum's written name.
fn serde_name<T: serde::Serialize>(v: &T) -> String {
    match serde_json::to_value(v) {
        Ok(Json::String(s)) => s,
        other => panic!("{other:?} is not a unit enum's name"),
    }
}

/// A table by number (`[0x05] = { ... }`) as rows from 0; a gap is an
/// error. `id`: the record's own field for its number.
fn numbered<T: DeserializeOwned>(r: &Resolver, d: &Data, at: &str, id: Option<&str>) -> Result<Vec<T>, String> {
    numbered_from(r, d, at, id, 0)
}

/// `numbered`, the rows from `from`.
fn numbered_from<T: DeserializeOwned>(r: &Resolver, d: &Data, at: &str, id: Option<&str>, from: i64) -> Result<Vec<T>, String> {
    let entries: Vec<(i64, &Data)> = match d {
        Data::Map(entries) => entries
            .iter()
            .map(|(k, v)| match k {
                DataKey::Int(i) => Ok((*i, v)),
                DataKey::Str(s) => Err(format!("{at}: `{s}` is not a number")),
            })
            .collect::<Result<_, _>>()?,
        // A table from 1 with no gaps reads as a list: its numbers from 1.
        Data::List(items) => items.iter().enumerate().map(|(i, v)| (i as i64 + 1, v)).collect(),
        _ => return Err(format!("{at}: a table by number")),
    };
    let mut out = Vec::with_capacity(entries.len());
    for (i, (n, v)) in entries.into_iter().enumerate() {
        let expect = i as i64 + from;
        if n != expect {
            return Err(format!("{at}: row {n:#x} leaves a gap (row {expect:#x} is missing)"));
        }
        let mut j = r.json(v, &format!("{at}[{n:#x}]"))?;
        if let (Some(field), Json::Object(o)) = (id, &mut j) {
            o.insert(field.to_string(), Json::from(n));
        }
        out.push(serde_json::from_value(j).map_err(|e| format!("{at}[{n:#x}]: {e}"))?);
    }
    Ok(out)
}

/// The rule sections into `rules` and the numbered tables into `content`:
/// each only if the content defines it.
fn sections(content: &mut Content, r: &Resolver, definitions: &Definitions) -> Result<(), ContentError> {
    for d in definitions.of(Registry::Rules) {
        let at = format!("{}.luau: rules {}", d.module, d.key);
        let e = |m: String| ContentError::new(m);
        let spec = &d.spec;
        let rules = &mut content.rules;
        match d.key.as_str() {
            "elements" => {
                let s: ElementsSection = r.read(spec, &at).map_err(e)?;
                let mut weakness = [[0u8; 6]; 6];
                for (name, row) in &s.weakness {
                    let i = ELEMENT_NAMES
                        .iter()
                        .position(|n| n == name)
                        .ok_or_else(|| e(format!("{at}: weakness.{name} is not an element")))?;
                    weakness[i] = *row;
                }
                let mut families = [SecondaryElements::default(); 13];
                for (name, bits) in &s.family_elements {
                    let f = ChipFamily::ALL
                        .iter()
                        .find(|&&f| serde_name(&f) == *name)
                        .ok_or_else(|| e(format!("{at}: family_elements.{name} is not a chip family")))?;
                    families[*f as usize] = *bits;
                }
                rules.element_weakness = weakness;
                rules.family_elements = families;
            }
            "panels" => {
                let s: PanelsSection = r.read(spec, &at).map_err(e)?;
                let mut types = vec![PanelTypeRule::default(); PanelType::ALL.len()];
                for t in PanelType::ALL {
                    let name = serde_name(&t);
                    let rule = s.types.get(&name).ok_or_else(|| e(format!("{at}: panel type {name} is missing")))?;
                    types[t as usize] = PanelTypeRule { flags: rule.flags, road_slide: rule.road_slide };
                }
                if s.types.len() != PanelType::ALL.len() {
                    return Err(e(format!("{at}: types names a panel type the engine doesn't have")));
                }
                rules.panels = PanelRules {
                    types,
                    start_visible: s.start_visible,
                    front_edges: s.front_edges,
                    step: s.step.rules(),
                    dash_step: s.dash_step.rules(),
                    any_side_step: s.any_side_step.rules(),
                };
            }
            "reactions" => {
                let s: ReactionsSection = r.read(spec, &at).map_err(e)?;
                (rules.push_vectors, rules.ice_vectors, rules.bubble_bob) = (s.push, s.ice, s.bubble_bob);
            }
            "berserk" => {
                let s: BerserkSection = r.read(spec, &at).map_err(e)?;
                rules.berserk = BerserkRules {
                    step: s.step.rules(),
                    opponent: s.opponent,
                    blocking: s.blocking,
                    opposing_player: s.opposing_player,
                };
            }
            "math" => rules.sine = r.read::<MathSection>(spec, &at).map_err(e)?.sine,
            "custom-screen" => {
                let s: CustomScreenSection = r.read(spec, &at).map_err(e)?;
                rules.custom_screen = CustomScreenLayout {
                    slots: s.slots,
                    left_scan_top: s.left_scan_top,
                    left_scan_bottom: s.left_scan_bottom,
                    right_scan_top: s.right_scan_top,
                    right_scan_bottom: s.right_scan_bottom,
                    left_scan_start: s.left_scan_start,
                    right_scan_start: s.right_scan_start,
                };
            }
            "buster" => {
                let s: BusterSection = r.read(spec, &at).map_err(e)?;
                (rules.buster_recovery, rules.empty_hand) = (s.recovery, s.empty_hand);
            }
            "banners" => rules.holding_banners = r.read::<BannersSection>(spec, &at).map_err(e)?.holding,
            "status" => rules.hp_bug_periods = r.read::<StatusSection>(spec, &at).map_err(e)?.hp_bug_periods,
            "lockon" => {
                let s: LockonSection = r.read(spec, &at).map_err(e)?;
                rules.lockon.column_shifts = s.column_shifts;
                rules.lockon.clear_path = s.clear_path;
            }
            "sp-chips" => rules.sp_deletion_times = r.read::<SpChipsSection>(spec, &at).map_err(e)?.deletion_times,
            "identities" => {
                let records: Vec<ActorRecordRow> = numbered(r, spec.field("actor_records"), &format!("{at}.actor_records"), None).map_err(e)?;
                rules.actor_records = records
                    .into_iter()
                    .map(|x| NaviRecord { version: x.version, actor_type: x.actor_type, ai_index: x.ai_index })
                    .collect();
                let Data::Map(looks) = spec.field("name_looks") else {
                    return Err(e(format!("{at}: name_looks is a table by NameID")));
                };
                let mut out = Vec::new();
                for (k, v) in looks {
                    let DataKey::Int(name_id) = k else { return Err(e(format!("{at}: name_looks is by NameID"))) };
                    let row: NameLookRow = r.read(v, &format!("{at}.name_looks[{name_id:#x}]")).map_err(e)?;
                    out.push(NameLook { name_id: *name_id as u16, sprite: row.sprite, anim: row.anim, palette: row.palette, shadow: row.shadow });
                }
                content.objects.name_looks = out;
            }
            "effects" => {
                let rows: Vec<EffectRow> = numbered(r, spec, &at, None).map_err(e)?;
                content.effects = rows.into_iter().map(|x| EffectSprite { sprite: x.sprite, anim: x.anim, palette: x.palette }).collect();
            }
            "sparks" => {
                let rows: Vec<EffectRow> = numbered(r, spec, &at, None).map_err(e)?;
                content.sparks = rows.into_iter().map(|x| EffectSprite { sprite: x.sprite, anim: x.anim, palette: x.palette }).collect();
            }
            "regions" => {
                // Region 0 is none (an empty list).
                let shapes: Vec<Vec<PanelOffset>> = numbered_from(r, spec.field("panels"), &format!("{at}.panels"), None, 1).map_err(e)?;
                content.regions = std::iter::once(Vec::new()).chain(shapes).collect();
                let field: Vec<FieldRegionRow> = numbered(r, spec.field("field"), &format!("{at}.field"), None).map_err(e)?;
                rules.field_regions = field.into_iter().map(|x| PanelCondition { require: x.require, forbid: x.forbid }).collect();
            }
            // The object kinds' tables by number, each while something
            // still reads it (the rocks', the absorbed obstacles' and the
            // sun beam's are their kinds' own definitions now).
            "attachments" => content.objects.attachments = numbered(r, spec, &at, Some("id")).map_err(e)?,
            "body-overlays" => content.objects.body_overlays = numbered(r, spec, &at, Some("id")).map_err(e)?,
            "sword-waves" => content.objects.sword_waves = numbered(r, spec, &at, Some("id")).map_err(e)?,
            "boomerangs" => content.objects.boomerangs = numbered(r, spec, &at, Some("id")).map_err(e)?,
            "shock-waves" => content.objects.shock_waves = numbered(r, spec, &at, Some("id")).map_err(e)?,
            "projectiles" => content.objects.projectiles = numbered(r, spec, &at, Some("id")).map_err(e)?,
            "flying-shots" => content.objects.flying_shots = numbered(r, spec, &at, Some("id")).map_err(e)?,
            // The weapons' charge times by routine are built with the weapons.
            "weapon-routines" => {}
            other => return Err(e(format!("{at}: the engine has no rule section `{other}`"))),
        }
    }
    Ok(())
}

// ---- Collision types ------------------------------------------------

fn registries(content: &mut Content, definitions: &Definitions) -> Result<(), ContentError> {
    // Collision types by row, when the content has no table of its own.
    let rows: Vec<&Definition> = definitions.of(Registry::Collision).iter().filter(|d| !d.spec.field("row_offset").is_nil()).collect();
    if content.rules.collision_types.is_empty() && !rows.is_empty() {
        let mut table: BTreeMap<i64, ([u32; 2], &Definition)> = BTreeMap::new();
        for d in rows {
            let word = |k: &str| d.spec.field(k).int().map(|i| i as u32).ok_or_else(|| err(d, format!("needs `{k}`")));
            let offset = d.spec.field("row_offset").int().expect("filtered");
            if offset % 8 != 0 {
                return Err(err(d, format!("row_offset {offset:#x} is not a row's (a multiple of 8)")));
            }
            // A row several modules define (each by its own name) must be
            // the same row.
            let flags = [word("side0")?, word("side1")?];
            if let Some((other, first)) = table.get(&(offset / 8))
                && *other != flags
            {
                return Err(err(d, format!("row {:#x} is also collision {}'s, with other flags", offset / 8, first.key)));
            }
            table.entry(offset / 8).or_insert((flags, d));
        }
        let mut out = Vec::new();
        for (expect, (row, (flags, d))) in table.into_iter().enumerate() {
            if row != expect as i64 {
                return Err(err(d, format!("row {row:#x} leaves a gap: collision types fill rows from 0 ({expect:#x} is missing)")));
            }
            out.push(flags);
        }
        content.rules.collision_types = out;
    }

    Ok(())
}

/// Remove fields from a table.
fn strip(spec: &mut Data, fields: &[&str]) {
    if let Data::Map(entries) = spec {
        entries.retain(|(k, _)| !matches!(k, DataKey::Str(s) if fields.contains(&s.as_str())));
    }
}

// ---- Chips, navis, forms, weapons -----------------------------------------------------

/// Whether a chip definition is a numbered record whose behaviour is still
/// a v1 module: its `legacy` marker gives its `number`.
fn numbered_chip(d: &Definition) -> bool {
    !d.spec.field("legacy").field("number").is_nil()
}

/// A numbered chip definition's record: `chip_record`'s fields, and its
/// `legacy = legacy { number, action, subtype, params, script, ... }`
/// marker's (the record's other fields, by their names). Its use is its
/// own (`action`, `dimming`, `navi` or `instant`), or the v1 module its
/// marker names (`script`), not both.
fn chip(d: &Definition, r: &Resolver) -> Result<ChipData, ContentError> {
    let own = ["action", "dimming", "navi", "instant"].iter().any(|f| !d.spec.field(f).is_nil());
    if own && !d.spec.field("legacy").field("script").is_nil() {
        return Err(err(d, "a chip's use is its own (`action`, `dimming`, `navi` or `instant`) or the v1 module its legacy marker names (`script`), not both"));
    }
    let record = super::defs::chip_record(d, r)?;
    let mut j = serde_json::to_value(&record).expect("a chip record serializes");
    let Json::Object(o) = &mut j else { unreachable!("a record is a table") };
    let mut marker = d.spec.field("legacy").clone();
    let number = marker.field("number").int().ok_or_else(|| err(d, "its legacy marker needs the chip's `number`"))?;
    strip(&mut marker, &["number"]);
    let Json::Object(extra) = r.json(&marker, &format!("chip {}.legacy", d.key)).map_err(|m| err(d, m))? else {
        return Err(err(d, "its legacy marker is a table"));
    };
    o.insert("id".into(), Json::from(number));
    for (k, v) in extra {
        o.insert(k, v);
    }
    serde_json::from_value(j).map_err(|m| err(d, format!("its legacy marker: {m}")))
}

/// A navi or form definition's identity (`identity`, with its NameID from
/// the legacy marker) as a record's `name_record`.
fn name_record(d: &Definition, r: &Resolver) -> Result<Option<Json>, ContentError> {
    let identity = d.spec.field("identity");
    if identity.is_nil() {
        return Ok(None);
    }
    let mut j = r.json(identity, &format!("{} {}.identity", d.registry, d.key)).map_err(|m| err(d, m))?;
    let name_id = d.spec.field("legacy").field("name_id").int().ok_or_else(|| err(d, "its identity needs a legacy `name_id`"))?;
    if let Json::Object(o) = &mut j {
        o.insert("id".into(), Json::from(name_id));
    }
    Ok(Some(j))
}

/// The fields of a spec, as the record's data: all but those named.
fn fields(d: &Definition, r: &Resolver, skip: &[&str]) -> Result<Map<String, Json>, ContentError> {
    let mut spec = d.spec.clone();
    strip(&mut spec, skip);
    match r.json(&spec, &format!("{} {}", d.registry, d.key)).map_err(|m| err(d, m))? {
        Json::Object(o) => Ok(o),
        Json::Array(a) if a.is_empty() => Ok(Map::new()),
        _ => Err(err(d, "is a table")),
    }
}

fn navi(d: &Definition, r: &Resolver) -> Result<NaviData, ContentError> {
    // The design's fields the engine doesn't read yet (phase C).
    let mut o = fields(d, r, &["id", "legacy", "identity", "banners", "own_chip", "actions", "traits", "mugshots"])?;
    let number = d.spec.field("legacy").field("number").int().ok_or_else(|| err(d, "needs a legacy `number`"))?;
    o.insert("id".into(), Json::from(number));
    let banners = d.spec.field("banners");
    for (field, which) in [("win_banner", "win"), ("lose_banner", "lose")] {
        let b = r.json(banners.field(which), &format!("navi {}.banners.{which}", d.key)).map_err(|m| err(d, m))?;
        o.insert(field.into(), b);
    }
    o.entry("weakness").or_insert(Json::Array(Vec::new()));
    o.entry("merge_height").or_insert(Json::from(0));
    let own = d.spec.field("own_chip");
    if !own.is_nil() {
        o.insert("own_chip".into(), r.json(own, &format!("navi {}.own_chip", d.key)).map_err(|m| err(d, m))?);
    }
    if let Some(n) = name_record(d, r)? {
        o.insert("name_record".into(), n);
    }
    serde_json::from_value(Json::Object(o)).map_err(|m| err(d, m))
}

/// A form's record, and its palette in a Cross (none: 0).
fn form(d: &Definition, r: &Resolver) -> Result<(FormData, Option<u8>), ContentError> {
    let mut o = fields(
        d,
        r,
        &["id", "legacy", "identity", "kind", "game", "cross_of", "beast", "palette", "mugshot", "overlay", "chip_bonus", "charged_chips", "status_reset", "traits", "weapons"],
    )?;
    let number = d.spec.field("legacy").field("number").int().ok_or_else(|| err(d, "needs a legacy `number`"))?;
    o.insert("id".into(), Json::from(number));
    o.entry("weakness").or_insert(Json::Array(Vec::new()));
    // Its weapon routines (0xFF: none), by slot: a weapon (its first
    // routine), or `legacy { routine = n }` for another of its routines.
    let weapons = d.spec.field("weapons");
    let mut w = Map::new();
    for slot in ["mode9_a", "a_charge", "buster", "charge_shot", "back_special", "alt_a_charge"] {
        let at = format!("form {}.weapons.{slot}", d.key);
        let n = match weapons.field(slot) {
            Data::Nil => 0xFF,
            v => match r.json(v, &at).map_err(|m| err(d, m))? {
                Json::Number(n) => n.as_i64().unwrap_or(0xFF),
                Json::Object(o) => o.get("routine").and_then(Json::as_i64).ok_or_else(|| err(d, format!("{at}: a weapon, or a legacy `routine`")))?,
                _ => return Err(err(d, format!("{at}: a weapon, or a legacy `routine`"))),
            },
        };
        w.insert(slot.into(), Json::from(n));
    }
    o.insert("weapons".into(), Json::Object(w));
    if let Some(n) = name_record(d, r)? {
        o.insert("name_record".into(), n);
    }
    let palette = d.spec.field("palette").int().map(|p| p as u8);
    Ok((serde_json::from_value(Json::Object(o)).map_err(|m| err(d, m))?, palette))
}

/// Whether a weapon definition's behaviour is still a v1 module: it has no
/// `setup` of its own, and its `legacy` marker names its routines (and the
/// module, if any implements them).
fn v1_weapon(d: &Definition) -> bool {
    d.spec.field("setup").is_nil() && matches!(d.spec.field("legacy"), Data::Map(_))
}

fn weapon(d: &Definition) -> Result<LegacyWeapon, ContentError> {
    let setup = d.spec.field("legacy");
    let routines: Vec<u8> = match setup.field("routines") {
        Data::List(items) => items.iter().map(|i| i.int().map(|n| n as u8)).collect::<Option<_>>(),
        _ => None,
    }
    .ok_or_else(|| err(d, "its legacy marker needs `routines`, the routine numbers"))?;
    let ticks = match d.spec.field("charge_ticks") {
        Data::List(items) => items.iter().map(|t| t.int().map(|n| n as u16)).collect::<Option<Vec<_>>>(),
        Data::Nil => Some(Vec::new()),
        _ => None,
    }
    .ok_or_else(|| err(d, "`charge_ticks` is a list of tick counts"))?;
    Ok(LegacyWeapon {
        name: d.spec.field("name").str().unwrap_or(&d.key).to_string(),
        routines,
        script: setup.field("script").str().map(str::to_string),
        action: setup.field("action").int().map(|n| n as u8),
        instant_chip: setup.field("instant_chip").int().map(|n| n as u8),
        charge_ticks: ticks,
    })
}

/// Everything registration by number reads of what the content defines:
/// the tables into `content`, the per-definition records returned.
pub fn build(content: &mut Content, definitions: &Definitions) -> Result<Legacy, ContentError> {
    let assets = content.assets.clone();
    let r = Resolver::new(&assets, definitions);
    let mut legacy = Legacy::default();

    sections(content, &r, definitions)?;
    registries(content, definitions)?;

    // Chips with a number in their legacy marker: the pack's, by number.
    for d in definitions.of(Registry::Chip) {
        if numbered_chip(d) {
            legacy.chips.insert(d.key.clone(), chip(d, &r)?);
        }
    }
    if !legacy.chips.is_empty() {
        let claimed: BTreeMap<ChipId, &str> = legacy.chips.iter().map(|(k, c)| (c.id.expect("numbered"), k.as_str())).collect();
        if claimed.len() != legacy.chips.len() {
            let mut seen = BTreeMap::new();
            for (k, c) in &legacy.chips {
                if let Some(first) = seen.insert(c.id, k) {
                    return Err(ContentError::new(format!("chips {first} and {k} are both chip {:#x}", c.id.unwrap_or_default())));
                }
            }
        }
        content.chips.retain(|c| c.id.is_none_or(|id| !claimed.contains_key(&id)));
        content.chips.extend(legacy.chips.values().cloned());
        content.chips.sort_by_key(|c| c.id);
    }

    // Navis and forms.
    for d in definitions.of(Registry::Navi) {
        legacy.navis.insert(d.key.clone(), navi(d, &r)?);
    }
    let mut palettes = BTreeMap::new();
    for d in definitions.of(Registry::Form) {
        let (f, palette) = form(d, &r)?;
        palettes.insert(f.id, palette.unwrap_or(0));
        legacy.forms.insert(d.key.clone(), f);
    }
    if !legacy.navis.is_empty() {
        content.navis = dense_by_number(legacy.navis.values().cloned().map(|n| (n.id, n)), "navi")?;
    }
    if !legacy.forms.is_empty() {
        content.forms = dense_by_number(legacy.forms.values().cloned().map(|f| (f.id, f)), "form")?;
        // The Cross palettes, by form up to the last form with one.
        let last = palettes.iter().filter(|(_, p)| **p != 0).map(|(f, _)| *f).max().unwrap_or(0);
        content.rules.cross_palettes = (0..=last).map(|f| palettes.get(&f).copied().unwrap_or(0)).collect();
    }

    // Weapons whose behaviour is a v1 module, and the routines no weapon
    // names.
    for d in definitions.of(Registry::Weapon) {
        if v1_weapon(d) {
            legacy.weapons.insert(d.key.clone(), weapon(d)?);
        }
    }
    if let Some(d) = definitions.get(Registry::Rules, "weapon-routines") {
        let rows: BTreeMap<u8, Vec<u16>> = r.read(&d.spec, "rules weapon-routines").map_err(|m| err(d, m))?;
        legacy.routine_charges = rows;
    }
    if !legacy.weapons.is_empty() {
        let mut out = Vec::new();
        for w in legacy.weapons.values() {
            let Some(script) = &w.script else { continue };
            for &id in &w.routines {
                out.push(WeaponData { id, name: w.name.clone(), action: w.action, instant_chip: w.instant_chip, script: script.clone() });
            }
        }
        out.sort_by_key(|w| w.id);
        content.weapons = out;
    }

    Ok(legacy)
}

/// Records by number, dense from 0.
fn dense_by_number<T>(items: impl Iterator<Item = (u8, T)>, what: &str) -> Result<Vec<T>, ContentError> {
    let mut by: BTreeMap<u8, T> = BTreeMap::new();
    for (n, x) in items {
        if by.insert(n, x).is_some() {
            return Err(ContentError::new(format!("two {what}s are {what} {n:#x}")));
        }
    }
    let mut out = Vec::new();
    for (expect, (n, x)) in by.into_iter().enumerate() {
        if n as usize != expect {
            return Err(ContentError::new(format!("{what} {n:#x} leaves a gap ({what} {expect:#x} is missing)")));
        }
        out.push(x);
    }
    Ok(out)
}
