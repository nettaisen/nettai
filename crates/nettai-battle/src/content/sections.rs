//! The rule sections (`define.rules("panels", { ... })`, rules/*.luau) as
//! the ruleset's typed tables (`Rules`). Content without them (the
//! engine's test content, whose tables are Rust) keeps its tables: each
//! section is built only when the content defines it.

use std::collections::BTreeMap;

use nettai_content_api::{ContentError, Definitions, Registry, keys};
use serde::Deserialize;
use serde_json::Value as Json;

use super::reader::SpecReader;
use super::*;
use crate::field::PanelType;

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
    /// A sound asset, as the pack identifies it.
    #[serde(default)]
    trail_sound: Option<u16>,
    #[serde(default)]
    expires: Option<u16>,
    #[serde(default)]
    burn: Option<u16>,
    /// An element by name.
    #[serde(default)]
    drains: Option<String>,
    #[serde(default)]
    holds: Option<u16>,
    #[serde(default)]
    submerges: bool,
    /// An element by name.
    #[serde(default)]
    cleared_by: Option<String>,
    /// By the direction of the move, the steps tried in turn.
    #[serde(default)]
    slide: Option<Vec<Vec<SlideStep>>>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct SlideStep {
    dx: i8,
    dy: i8,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct MendSection {
    normal: u16,
    battle_mode_1: u16,
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
    mend: MendSection,
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
    #[serde(default)]
    push_reading: super::rules::PushReading,
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

/// Root `root`'s rule sections into `rules` (which starts as the base):
/// each only if the root defines it.
fn sections(rules: &mut Rules, root: &str, r: &SpecReader, definitions: &Definitions) -> Result<(), ContentError> {
    for d in definitions.of(Registry::Rules) {
        if keys::root_of(&d.key).unwrap_or("") != root {
            continue;
        }
        let at = format!("{}.luau: rules {}", d.module, d.key);
        let e = |m: String| ContentError::new(m);
        let spec = &d.spec;
        match keys::local(&d.key) {
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
                let mut families = [SecondaryElements::default(); ChipFamily::ALL.len()];
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
                // The types the game has (docs/design/bn5-map.md §15.3 item
                // 1): BN6 names its 13, BN5 its 11; the others are another
                // loaded game's (`fill_panel_types`).
                let mut types = vec![PanelTypeRule::default(); PanelType::ALL.len()];
                for t in PanelType::ALL {
                    let name = serde_name(&t);
                    let Some(rule) = s.types.get(&name) else { continue };
                    let element = |field: &str, v: &Option<String>| -> Result<Option<u8>, ContentError> {
                        match v {
                            Some(element) => Ok(Some(
                                ELEMENT_NAMES
                                    .iter()
                                    .position(|n| n == element)
                                    .ok_or_else(|| e(format!("{at}: types.{name}.{field}: {element:?} is not an element")))?
                                    as u8,
                            )),
                            None => Ok(None),
                        }
                    };
                    let drains = element("drains", &rule.drains)?;
                    let cleared_by = element("cleared_by", &rule.cleared_by)?;
                    let slide = match &rule.slide {
                        Some(by_direction) => {
                            if by_direction.len() != 6 || by_direction.iter().any(|tries| tries.len() > 4) {
                                return Err(e(format!(
                                    "{at}: types.{name}.slide: six directions (none, up, down, back, forward, other), four steps or fewer each"
                                )));
                            }
                            let mut s = PanelSlide::default();
                            for (d, tries) in by_direction.iter().enumerate() {
                                for (i, step) in tries.iter().enumerate() {
                                    s.tries[d][i] = Some((step.dx, step.dy));
                                }
                            }
                            Some(s)
                        }
                        None => None,
                    };
                    types[t as usize] = PanelTypeRule {
                        flags: rule.flags,
                        road_slide: rule.road_slide,
                        trail_sound: rule.trail_sound.map(crate::sound::SoundId),
                        expires: rule.expires,
                        burn: rule.burn,
                        drains,
                        holds: rule.holds,
                        submerges: rule.submerges,
                        slide,
                        cleared_by,
                        named: true,
                    };
                }
                if let Some(unknown) = s.types.keys().find(|k| !PanelType::ALL.iter().any(|t| serde_name(t) == **k)) {
                    return Err(e(format!("{at}: types names {unknown:?}, a panel type the engine doesn't have")));
                }
                rules.panels = PanelRules {
                    types,
                    start_visible: s.start_visible,
                    front_edges: s.front_edges,
                    step: s.step.rules(),
                    dash_step: s.dash_step.rules(),
                    any_side_step: s.any_side_step.rules(),
                    mend: s.mend.normal,
                    mend_in_battle_mode_1: s.mend.battle_mode_1,
                };
            }
            "reactions" => {
                let s: ReactionsSection = r.read(spec, &at).map_err(e)?;
                (rules.push_vectors, rules.ice_vectors, rules.bubble_bob) = (s.push, s.ice, s.bubble_bob);
                rules.push_reading = s.push_reading;
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
            "pools" => {
                let s: PoolSizes = r.read(spec, &at).map_err(e)?;
                if s.slots().iter().any(|&n| n == 0 || n as usize > crate::object::SLOTS) {
                    return Err(e(format!("{at}: a pool holds 1 to {} objects", crate::object::SLOTS)));
                }
                rules.pools = s;
            }
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
            "chip-use" => rules.chip_use = r.read::<super::rules::ChipUseRules>(spec, &at).map_err(e)?,
            "flow" => rules.flow = r.read::<super::rules::FlowRules>(spec, &at).map_err(e)?,
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
            other => return Err(e(format!("{at}: the engine has no rule section `{other}`"))),
        }
    }
    Ok(())
}

/// The rule sections the content defines, into `content.rules`.
pub fn build(content: &mut Content, definitions: &Definitions) -> Result<(), ContentError> {
    let r = SpecReader::new(&content.assets, definitions);
    let mut all = Vec::new();
    for root in content.scripts.root_names() {
        let mut rules = content.base_rules.clone();
        sections(&mut rules, &root, &r, definitions)?;
        all.push(rules);
    }
    fill_panel_types(&mut all);
    content.rules = all;
    Ok(())
}

/// A panel type a game's section doesn't name is the first other loaded
/// game's that does, in root order (docs/design/rules-in-luau.md §7.4: a
/// BN6 chip's road in a BN5 arena runs by BN6's rule; never a panic). One
/// no loaded game names keeps an empty rule.
pub(crate) fn fill_panel_types(all: &mut [Rules]) {
    for rules in all.iter_mut() {
        rules.panels.types.resize(PanelType::ALL.len(), PanelTypeRule::default());
    }
    for t in 0..PanelType::ALL.len() {
        let Some(named) = all.iter().find(|r| r.panels.types[t].named).map(|r| r.panels.types[t]) else { continue };
        for rules in all.iter_mut() {
            if !rules.panels.types[t].named {
                rules.panels.types[t] = PanelTypeRule { named: false, ..named };
            }
        }
    }
}
