//! The tables the ruleset and v1 modules still read by number, built from
//! what the content defines (docs/design/content-model-v2.md §12, step 5):
//! the rule sections, collision types, statuses and lock-on modes, and the numbered tables (effects,
//! sparks, regions, the object kinds' rows).
//!
//! Where a definition still carries what only registration by number reads
//! (a table's original numbering), it sits in a `legacy { ... }` marker,
//! which goes when its family converts (§12, phase B) or the ruleset stops
//! asking numbers (phase C).
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

pub(crate) fn err(d: &Definition, e: impl std::fmt::Display) -> ContentError {
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
    /// The numbers the tables hold of what the definitions name: the
    /// lock-on modes' handles.
    pub fn new(assets: &'a AssetNames, definitions: &Definitions) -> Resolver<'a> {
        let mut numbers = HashMap::new();
        let mut put = |d: &Definition, n: Option<i64>| {
            if let Some(n) = n {
                numbers.insert((d.registry, d.key.clone()), n);
            }
        };
        // A lock-on mode reads as its handle (its place among the
        // definitions, which are in key order).
        for (i, d) in definitions.of(Registry::Lockon).iter().enumerate() {
            put(d, Some(i as i64));
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
            // A chip by its key: the registry resolves it to a handle
            // (no chip has a number the tables hold).
            Data::Ref(Registry::Chip, key) => Json::String(key.clone()),
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
    /// The SP navis whose deletion times a setup carries, in its order.
    #[serde(default)]
    slots: Vec<String>,
}

/// One of the Cross special's chips: a chip, or `{ chip, damage_of }`.
#[derive(Deserialize)]
#[serde(untagged)]
enum SpecialChipEntry {
    Chip(String),
    With(SpecialChip),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CrossSpecialSection {
    /// A row of chips by the hundreds of the navi's base max HP.
    rows: Vec<Vec<SpecialChipEntry>>,
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
        let expect = i as i64;
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
            "sp-chips" => {
                let s: SpChipsSection = r.read(spec, &at).map_err(e)?;
                (rules.sp_deletion_times, rules.sp_slots) = (s.deletion_times, s.slots);
            }
            "cross-special" => {
                let s: CrossSpecialSection = r.read(spec, &at).map_err(e)?;
                rules.cross_special = s
                    .rows
                    .into_iter()
                    .map(|row| {
                        row.into_iter()
                            .map(|c| match c {
                                SpecialChipEntry::Chip(chip) => SpecialChip { chip, damage_of: None },
                                SpecialChipEntry::With(c) => c,
                            })
                            .collect()
                    })
                    .collect();
            }
            // The object kinds' tables by number, each while something
            // still reads it (the rocks', the absorbed obstacles' and the
            // sun beam's are their kinds' own definitions now).
            "sword-waves" => content.objects.sword_waves = numbered(r, spec, &at, Some("id")).map_err(e)?,
            "boomerangs" => content.objects.boomerangs = numbered(r, spec, &at, Some("id")).map_err(e)?,
            "shock-waves" => content.objects.shock_waves = numbered(r, spec, &at, Some("id")).map_err(e)?,
            "projectiles" => content.objects.projectiles = numbered(r, spec, &at, Some("id")).map_err(e)?,
            "flying-shots" => content.objects.flying_shots = numbered(r, spec, &at, Some("id")).map_err(e)?,
            other => return Err(e(format!("{at}: the engine has no rule section `{other}`"))),
        }
    }
    Ok(())
}

/// Remove fields from a table.
fn strip(spec: &mut Data, fields: &[&str]) {
    if let Data::Map(entries) = spec {
        entries.retain(|(k, _)| !matches!(k, DataKey::Str(s) if fields.contains(&s.as_str())));
    }
}

/// The fields of a spec, as a record's data: all but those named.
pub(crate) fn fields(d: &Definition, r: &Resolver, skip: &[&str]) -> Result<Map<String, Json>, ContentError> {
    let mut spec = d.spec.clone();
    strip(&mut spec, skip);
    match r.json(&spec, &format!("{} {}", d.registry, d.key)).map_err(|m| err(d, m))? {
        Json::Object(o) => Ok(o),
        Json::Array(a) if a.is_empty() => Ok(Map::new()),
        _ => Err(err(d, "is a table")),
    }
}

/// Everything registration by number reads of what the content defines:
/// the tables into `content`.
pub fn build(content: &mut Content, definitions: &Definitions) -> Result<(), ContentError> {
    let assets = content.assets.clone();
    let r = Resolver::new(&assets, definitions);
    sections(content, &r, definitions)
}
