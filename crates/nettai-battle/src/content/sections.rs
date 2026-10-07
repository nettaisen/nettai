//! The rule sections as the game's typed tables (`Rules`): the fields of
//! its rules (rules/init.luau: `panels = require("@self/panels")`,
//! each a plain table its module returns; docs/design/content-model-v2.md
//! §3.8), each read against its schema when the content is defined (a
//! message names the place: `rules: panels.types.grass.flags`).
//!
//! **The engine has no game's rules of its own.** The rules state every
//! rule that has no neutral value: the sections [`REQUIRED`], and in them
//! every field but those that are none for a game without the feature (a
//! missing one is a load error that names it: `rules: flow: missing
//! field `escape_check``). What may be left out
//! reads as nothing for every game: a feature's section a game hasn't
//! (`lockon`, `banners`) and a table
//! that is empty without it (`elements`, `buster`, `math`,
//! `custom_screen`); in a section, a list or an attribute of one entry
//! that is none unless stated (a panel type's `burn`).
//! Content whose rules are Rust tables (`Content::base_rules`: a tool's
//! decode of a ROM, a test's content of a few modules) states them there,
//! and its rules' sections replace those. (The engine's test content
//! states its rules in its rules, as a game does.)

use std::collections::BTreeMap;

use nettai_content_api::{ContentError, Data, DataKey, Definitions};
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
    /// The game's panel types by its own numbers (names).
    numbers: Vec<String>,
    step: StepSection,
    dash_step: StepSection,
    any_side_step: StepSection,
    reservations: super::rules::Reservations,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReactionsSection {
    push: [SlideVector; 10],
    push_reading: super::rules::PushReading,
    hit_test: super::rules::HitTest,
    obstacle_slide_bounds: bool,
    ice: [SlideVector; 6],
    bubble_bob: [i8; 32],
    slide_speed: super::rules::SlideSpeed,
    overlay_restart: super::rules::OverlayRestart,
    stance_counter: super::rules::StanceCounter,
    request_clears: super::rules::RequestClears,
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
    /// (None listed: a re-deal keeps none, whatever the hand.)
    #[serde(default)]
    redeal_kept: Vec<u8>,
    emblem_at_window_return: bool,
    chatbox_commands_wait_for_text: bool,
    talking_characters: super::custom::TalkingCharacters,
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
    form_tick: bool,
    flash_hides_on_clear: bool,
    /// The status word a navi without collision data reads as.
    missing_collision_status: u32,
    reactions: super::rules::Reactions,
    bugs_before_drain: bool,
    no_charge_drive: bool,
    hp_loss: super::rules::HpLoss,
    emotion: super::rules::EmotionRules,
    form_break: super::FormBreak,
    weakness_hit_breaks_form: bool,
    weakness_mark: super::rules::WeaknessMark,
}

/// The `fresh_stats` section but its weapon (`mode9_a`, a definition) and
/// the game's own stats (by the rules' `stats`): the rules' [`link`] gives
/// those.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FreshStatsSection {
    reg_up: u8,
    custom_level: u8,
    mood: u8,
}

/// The engine's fields of the `fresh_stats` section; the rest are the
/// game's own stats.
const FRESH_STATS: [&str; 4] = ["reg_up", "custom_level", "mood", "mode9_a"];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkPickSection {
    first_round_stages: usize,
    backgrounds: Vec<super::BackgroundId>,
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

/// The rule sections rules may name, by field (the engine's
/// schemas).
pub(crate) const SECTIONS: &[&str] = &[
    "banners",
    "buster",
    "chip_use",
    "custom_screen",
    "effects",
    "elements",
    "flow",
    "fresh_stats",
    "link_pick",
    "math",
    "panels",
    "pools",
    "reactions",
    "status",
];

/// The rule sections the rules state, whatever else they do: those with a
/// rule that has no neutral value (a choice between games' behaviors, a
/// size, a speed). The engine has no game's to fall back on.
pub(crate) const REQUIRED: &[&str] = &["chip_use", "effects", "flow", "fresh_stats", "link_pick", "panels", "pools", "reactions", "status"];

/// What is stated of the rules, by section: a rules' sections, over
/// the content's Rust tables when it has them.
#[derive(Default)]
struct Stated {
    elements: Option<([[u8; 6]; 6], [SecondaryElements; 15])>,
    panels: Option<PanelRules>,
    reactions: Option<ReactionsSection>,
    sine: Option<Vec<i16>>,
    pools: Option<PoolSizes>,
    custom_screen: Option<CustomScreenLayout>,
    buster: Option<BusterSection>,
    holding_banners: Option<Vec<BannerId>>,
    status: Option<StatusSection>,
    chip_use: Option<super::rules::ChipUseRules>,
    flow: Option<super::rules::FlowRules>,
    effects: Option<super::rules::EffectsRules>,
    fresh_stats: Option<super::rules::FreshStatsRules>,
    link_pick: Option<LinkPickSection>,
}

impl Stated {
    /// Every section, as Rust tables state it.
    fn of(r: &Rules) -> Stated {
        Stated {
            elements: Some((r.element_weakness, r.family_elements)),
            panels: Some(r.panels.clone()),
            reactions: Some(ReactionsSection {
                push: r.push_vectors,
                push_reading: r.push_reading,
                hit_test: r.hit_test,
                obstacle_slide_bounds: r.obstacle_slide_bounds,
                ice: r.ice_vectors,
                bubble_bob: r.bubble_bob,
                slide_speed: r.slide_speed,
                overlay_restart: r.overlay_restart,
                stance_counter: r.stance_counter,
                request_clears: r.request_clears,
            }),
            sine: Some(r.sine.clone()),
            pools: Some(r.pools),
            custom_screen: Some(r.custom_screen.clone()),
            buster: Some(BusterSection { recovery: r.buster_recovery.clone(), empty_hand: r.empty_hand }),
            holding_banners: Some(r.holding_banners.clone()),
            status: Some(StatusSection {
                hp_bug_periods: r.hp_bug_periods,
                form_tick: r.form_tick,
                flash_hides_on_clear: r.flash_hides_on_clear,
                missing_collision_status: r.missing_collision_status.0,
                reactions: r.reactions,
                bugs_before_drain: r.intake.bugs_before_drain,
                no_charge_drive: r.intake.no_charge_drive,
                hp_loss: r.intake.hp_loss,
                emotion: r.emotion.clone(),
                form_break: r.form_break,
                weakness_hit_breaks_form: r.weakness_hit_breaks_form,
                weakness_mark: r.weakness_mark,
            }),
            chip_use: Some(r.chip_use),
            flow: Some(r.flow),
            link_pick: Some(LinkPickSection { first_round_stages: r.link_pick.first_round_stages, backgrounds: r.link_pick.backgrounds.clone() }),
            effects: Some(r.effects.clone()),
            fresh_stats: Some(r.fresh_stats),
        }
    }

    /// The rules: every section of [`REQUIRED`] stated (else an error at
    /// `at` that names the first one that isn't), the others' tables empty
    /// where nothing states them.
    fn rules(self, at: &str) -> Result<Rules, ContentError> {
        let missing = |name: &str| {
            ContentError::new(format!(
                "{at}: it states no `{name}` section: the rules state every rule of their game ({}), and the engine has no game's rules of its own",
                REQUIRED.join(", ")
            ))
        };
        let chip_use = self.chip_use.ok_or_else(|| missing("chip_use"))?;
        let effects = self.effects.ok_or_else(|| missing("effects"))?;
        let flow = self.flow.ok_or_else(|| missing("flow"))?;
        let link_pick = self.link_pick.ok_or_else(|| missing("link_pick"))?;
        let fresh_stats = self.fresh_stats.ok_or_else(|| missing("fresh_stats"))?;
        let mut panels = self.panels.ok_or_else(|| missing("panels"))?;
        let pools = self.pools.ok_or_else(|| missing("pools"))?;
        let reactions = self.reactions.ok_or_else(|| missing("reactions"))?;
        let status = self.status.ok_or_else(|| missing("status"))?;
        panels.types.resize(PanelType::ALL.len(), PanelTypeRule::default());
        let (element_weakness, family_elements) = self.elements.unwrap_or_default();
        let buster = self.buster.unwrap_or(BusterSection { recovery: Vec::new(), empty_hand: EmptyHandChip::default() });
        Ok(Rules {
            element_weakness,
            family_elements,
            panels,
            holding_banners: self.holding_banners.unwrap_or_default(),
            hp_bug_periods: status.hp_bug_periods,
            form_tick: status.form_tick,
            flash_hides_on_clear: status.flash_hides_on_clear,
            missing_collision_status: super::rules::MissingCollisionStatus(status.missing_collision_status),
            reactions: status.reactions,
            emotion: status.emotion,
            form_break: status.form_break,
            weakness_hit_breaks_form: status.weakness_hit_breaks_form,
            weakness_mark: status.weakness_mark,
            intake: super::rules::IntakeRules {
                bugs_before_drain: status.bugs_before_drain,
                no_charge_drive: status.no_charge_drive,
                hp_loss: status.hp_loss,
            },
            empty_hand: buster.empty_hand,
            buster_recovery: buster.recovery,
            // (Its stages are `link`'s to resolve.)
            link_pick: super::rules::LinkPick {
                stages: Vec::new(),
                first_round_stages: link_pick.first_round_stages,
                backgrounds: link_pick.backgrounds,
                match_stages: Vec::new(),
            },
            sine: self.sine.unwrap_or_default(),
            push_vectors: reactions.push,
            push_reading: reactions.push_reading,
            hit_test: reactions.hit_test,
            obstacle_slide_bounds: reactions.obstacle_slide_bounds,
            ice_vectors: reactions.ice,
            slide_speed: reactions.slide_speed,
            overlay_restart: reactions.overlay_restart,
            stance_counter: reactions.stance_counter,
            request_clears: reactions.request_clears,
            bubble_bob: reactions.bubble_bob,
            flow,
            effects,
            chip_use,
            fresh_stats,
            custom_screen: self.custom_screen.unwrap_or_default(),
            pools,
        })
    }
}

/// The content's rules: what its rules state over its Rust tables
/// (`base`), if it has either. Rules that leaves out a section of
/// [`REQUIRED`] that no Rust table states, or a field of one, is an error
/// naming it. Without rules, the Rust tables alone; without either, a
/// game pack (`game`) is an error too, and modules of no game have no
/// rules (a test's: no battle can be made of them).
fn rules(base: Option<&Rules>, game: Option<&str>, r: &SpecReader, definitions: &Definitions) -> Result<Option<Rules>, ContentError> {
    let Some(d) = super::defs::rules_definition(definitions) else {
        return match (base, game) {
            (None, Some(game)) => Err(ContentError::new(format!(
                "{game}/{}.luau: game pack {game} defines no rules (its root's `rules`, its rules/init.luau): a game states its rules, and the engine has no game's rules of its own",
                nettai_content_api::packs::INIT
            ))),
            _ => Ok(base.cloned()),
        };
    };
    let mut stated = base.map(Stated::of).unwrap_or_default();
    let at = format!("{}.luau: rules", nettai_content_api::keys::module_path(&d.module));
    if let Data::Map(fields) = &d.spec {
        for (field, spec) in fields {
            let DataKey::Str(name) = field else { continue };
            if SECTIONS.contains(&name.as_str()) {
                section(&mut stated, name, spec, &format!("{at}: {name}"), r)?;
            }
        }
    }
    stated.rules(&at).map(Some)
}

/// Section `name` (`spec`, at `at`) into what is stated.
fn section(stated: &mut Stated, name: &str, spec: &Data, at: &str, r: &SpecReader) -> Result<(), ContentError> {
    {
        let at = at.to_string();
        let e = |m: String| ContentError::new(m);
        match name {
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
                stated.elements = Some((weakness, families));
            }
            "panels" => {
                let s: PanelsSection = r.read(spec, &at).map_err(e)?;
                // The types the game has (docs/design/exe5-map.md §15.3 item
                // 1): EXE6 names its 13, EXE5 its 11; the others keep an
                // empty rule (no panel of the game is one).
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
                                    .ok_or_else(|| e(format!("{at}.types.{name}.{field}: {element:?} is not an element")))?
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
                                    "{at}.types.{name}.slide: six directions (none, up, down, back, forward, other), four steps or fewer each"
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
                let numbers = s
                    .numbers
                    .iter()
                    .map(|n| {
                        PanelType::ALL
                            .iter()
                            .copied()
                            .find(|t| serde_name(t) == *n)
                            .ok_or_else(|| e(format!("{at}: numbers names {n:?}, a panel type the engine doesn't have")))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                stated.panels = Some(PanelRules {
                    types,
                    start_visible: s.start_visible,
                    front_edges: s.front_edges,
                    step: s.step.rules(),
                    dash_step: s.dash_step.rules(),
                    any_side_step: s.any_side_step.rules(),
                    mend: s.mend.normal,
                    mend_in_battle_mode_1: s.mend.battle_mode_1,
                    numbers,
                    reservations: s.reservations,
                });
            }
            "reactions" => stated.reactions = Some(r.read(spec, &at).map_err(e)?),
            "math" => stated.sine = Some(r.read::<MathSection>(spec, &at).map_err(e)?.sine),
            "pools" => {
                let s: PoolSizes = r.read(spec, &at).map_err(e)?;
                if s.slots().iter().any(|&n| n == 0 || n as usize > crate::object::SLOTS) {
                    return Err(e(format!("{at}: a pool holds 1 to {} objects", crate::object::SLOTS)));
                }
                stated.pools = Some(s);
            }
            "custom_screen" => {
                let s: CustomScreenSection = r.read(spec, &at).map_err(e)?;
                if s.talking_characters.only.is_some() == s.talking_characters.all_but.is_some() {
                    return Err(e(format!("{at}: talking_characters states `only` or `all_but`, one of the two")));
                }
                stated.custom_screen = Some(CustomScreenLayout {
                    slots: s.slots,
                    left_scan_top: s.left_scan_top,
                    left_scan_bottom: s.left_scan_bottom,
                    right_scan_top: s.right_scan_top,
                    right_scan_bottom: s.right_scan_bottom,
                    left_scan_start: s.left_scan_start,
                    right_scan_start: s.right_scan_start,
                    redeal_kept: s.redeal_kept,
                    emblem_at_window_return: s.emblem_at_window_return,
                    chatbox_commands_wait_for_text: s.chatbox_commands_wait_for_text,
                    talking_characters: s.talking_characters,
                });
            }
            "buster" => stated.buster = Some(r.read(spec, &at).map_err(e)?),
            "banners" => stated.holding_banners = Some(r.read::<BannersSection>(spec, &at).map_err(e)?.holding),
            "status" => stated.status = Some(r.read(spec, &at).map_err(e)?),
            "chip_use" => stated.chip_use = Some(r.read(spec, &at).map_err(e)?),
            "flow" => stated.flow = Some(r.read(spec, &at).map_err(e)?),
            "link_pick" => {
                // (Its stages are definitions, which `link` resolves once
                // they have their handles; stated all the same. Both
                // fields are asked for here: a section with one alone
                // reads as an empty table once the stages are out.)
                for field in ["stages", "first_round_stages", "backgrounds", "match_stages"] {
                    if matches!(spec.field(field), Data::Nil) {
                        return Err(e(format!("{at}: missing field `{field}`")));
                    }
                }
                let mut data = spec.clone();
                super::reader::strip(&mut data, &["stages", "match_stages"]);
                stated.link_pick = Some(r.read(&data, &at).map_err(e)?);
            }
            "effects" => stated.effects = Some(r.read(spec, &at).map_err(e)?),
            "fresh_stats" => {
                // (Its weapon is a definition, which `link` resolves once
                // the definitions have their handles.)
                let mut data = spec.clone();
                let own: Vec<String> = match spec {
                    Data::Map(entries) => entries.iter().map(|(k, _)| k.to_string()).filter(|k| !FRESH_STATS.contains(&k.as_str())).collect(),
                    _ => Vec::new(),
                };
                let own: Vec<&str> = own.iter().map(String::as_str).collect();
                super::reader::strip(&mut data, &own);
                super::reader::strip(&mut data, &["mode9_a"]);
                let s: FreshStatsSection = r.read(&data, &at).map_err(e)?;
                stated.fresh_stats = Some(super::rules::FreshStatsRules {
                    reg_up: s.reg_up,
                    custom_level: s.custom_level,
                    mood: s.mood,
                    stats: Default::default(),
                    mode9_a: None,
                });
            }
            other => return Err(e(format!("{at}: the engine has no rule section `{other}` ({})", SECTIONS.join(", ")))),
        }
    }
    Ok(())
}

/// The content's rules (`Content::rules`): what its rules state, over
/// its Rust tables when it has them.
pub fn build(content: &mut Content, definitions: &Definitions) -> Result<(), ContentError> {
    let r = SpecReader::new(&content.assets, definitions);
    let games = content.scripts.games();
    let rules = rules(content.base_rules.as_ref(), games.first().map(String::as_str), &r, definitions)?;
    content.rules = rules;
    Ok(())
}

/// The rules' references to definitions, once those have their handles
/// (`Defs::build`): the fresh stats' `mode9_a`, a weapon, and the link
/// pick's `stages`. Rules that states no `fresh_stats` section (a
/// test's, whose rules are Rust tables) keeps what the tables have.
pub fn link(content: &mut Content) -> Result<(), ContentError> {
    let Some(d) = super::defs::rules_definition(&content.defs.definitions) else { return Ok(()) };
    let path = nettai_content_api::keys::module_path(&d.module);
    // A section's list of stages (`link_pick.stages`, `link_pick.match_stages`),
    // resolved: none where it states none.
    let stages_of = |content: &Content, field: &str| -> Result<Option<Vec<nettai_content_api::StageHandle>>, ContentError> {
        let at = format!("{path}.luau: rules: link_pick.{field}");
        let items: &[Data] = match d.spec.field("link_pick").field(field) {
            Data::Nil => return Ok(None),
            Data::List(items) => items,
            // (An empty table is an empty list.)
            Data::Map(entries) if entries.is_empty() => &[],
            other => return Err(ContentError::new(format!("{at}: a list of stages, not {other:?}"))),
        };
        let mut stages = Vec::with_capacity(items.len());
        for (i, item) in items.iter().enumerate() {
            let Data::Ref(nettai_content_api::Registry::Stage, key) = item else {
                return Err(ContentError::new(format!("{at}[{}]: a stage (one of the root's `stages`), not {item:?}", i + 1)));
            };
            stages.push(content.defs.stage_by_key(key).ok_or_else(|| ContentError::new(format!("{at}[{}]: the content has no stage {key:?}", i + 1)))?);
        }
        Ok(Some(stages))
    };
    if let Some(mut named) = stages_of(content, "match_stages")? {
        named.sort();
        named.dedup();
        if let Some(rules) = content.rules.as_mut() {
            rules.link_pick.match_stages = named;
        }
    }
    match stages_of(content, "stages")? {
        None => {}
        Some(stages) => {
            if let Some(rules) = content.rules.as_mut() {
                // The first round's are some of them, and one at least where
                // there are any.
                let first = rules.link_pick.first_round_stages;
                if first > stages.len() || (first == 0) != stages.is_empty() {
                    return Err(ContentError::new(format!(
                        "{path}.luau: rules: link_pick.first_round_stages is {first}: how many of its {} stages, from the first, a set's first round picks among",
                        stages.len()
                    )));
                }
                rules.link_pick.stages = stages;
            }
        }
    }
    let section = d.spec.field("fresh_stats");
    // The game's own stats, fresh: a block of the rules' `stats`, with the
    // section's values of them.
    if let Some(rules_def) = content.defs.rules() {
        let id = rules_def.stats;
        let schema = content.defs.schema(id);
        let path = nettai_content_api::keys::module_path(&d.module);
        let mut block = nettai_content_api::SmallBlock::new(id, schema).ok_or_else(|| {
            ContentError::new(format!(
                "{path}.luau: rules: their `stats` take {} bytes; a navi's stats keep {} of the game's own",
                schema.size(),
                nettai_content_api::SMALL_BLOCK
            ))
        })?;
        if let Data::Map(entries) = section {
            for (k, v) in entries {
                let name = k.to_string();
                if FRESH_STATS.contains(&name.as_str()) {
                    continue;
                }
                let at = format!("{path}.luau: rules: fresh_stats.{name}");
                let i = schema.index_of(&name).ok_or_else(|| ContentError::new(format!("{at}: neither the engine's nor the rules' `stats`")))?;
                let value = match v {
                    Data::Bool(b) => nettai_content_api::Value::Bool(*b),
                    Data::Int(n) => nettai_content_api::Value::Int(*n),
                    other => return Err(ContentError::new(format!("{at}: a number or a flag, not {other:?}"))),
                };
                block.set(schema, i, value).map_err(|e| ContentError::new(format!("{at}: {e}")))?;
            }
        }
        if let Some(rules) = content.rules.as_mut() {
            rules.fresh_stats.stats = block;
        }
    }
    if matches!(section, Data::Nil) {
        return Ok(());
    }
    let at = format!("{}.luau: rules: fresh_stats.mode9_a", nettai_content_api::keys::module_path(&d.module));
    let weapon = match section.field("mode9_a") {
        Data::Nil => None,
        Data::Ref(nettai_content_api::Registry::Weapon, key) => {
            Some(content.defs.weapon_by_key(key).ok_or_else(|| ContentError::new(format!("{at}: the content has no weapon {key:?}")))?)
        }
        other => return Err(ContentError::new(format!("{at}: a weapon (a `new.weapon`), not {other:?}"))),
    };
    if let Some(rules) = content.rules.as_mut() {
        rules.fresh_stats.mode9_a = weapon;
    }
    Ok(())
}
