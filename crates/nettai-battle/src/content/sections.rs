//! The rule sections as the game's typed tables (`Rules`): the fields of
//! its ruleset (rules/init.luau: `panels = require("@self/panels")`,
//! each a plain table its module returns; docs/design/content-model-v2.md
//! §3.8), each read against its schema when the content is defined (a
//! message names the place: `ruleset: panels.types.grass.flags`).
//!
//! **The engine has no game's rules of its own.** A ruleset states every
//! rule that has no neutral value: the sections [`REQUIRED`], and in them
//! every field (a missing one is a load error that names it:
//! `ruleset: flow: missing field `escape_check``). What may be left out
//! reads as nothing for every game: a feature's section a game hasn't
//! (`berserk`, `lockon`, `navicust`, `sp_chips`, `banners`) and a table
//! that is empty without it (`elements`, `buster`, `math`,
//! `custom_screen`); in a section, a list or an attribute of one entry
//! that is none unless stated (a panel type's `burn`, `chaos_cycle`).
//! Content whose rules are Rust tables (`Content::base_rules`: the engine's
//! test content, a tool's decode of a ROM) states them there, and its
//! ruleset's sections replace those.

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
    /// (None listed: a re-deal keeps none, whatever the hand.)
    #[serde(default)]
    redeal_kept: Vec<u8>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BusterSection {
    recovery: Vec<[u8; 6]>,
    empty_hand: EmptyHandChip,
    /// (No rows: the game has no cycle.)
    #[serde(default)]
    chaos_cycle: Vec<[u8; 4]>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BannersSection {
    #[serde(default)]
    holding: Vec<BannerId>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NaviCustSection {
    boards: Vec<Vec<String>>,
    command_line: u8,
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
    drain_bug_flags: bool,
    no_charge_drive: bool,
    hp_loss: super::rules::HpLoss,
    emotions: super::Emotions,
    form_break: super::FormBreak,
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
    /// The SP navis whose deletion times a setup carries, in its order
    /// (none listed: a setup carries none).
    #[serde(default)]
    slots: Vec<String>,
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

/// The rule sections a ruleset may name, by field (the engine's
/// schemas).
pub(crate) const SECTIONS: &[&str] = &[
    "banners",
    "berserk",
    "buster",
    "chip_use",
    "custom_screen",
    "effects",
    "elements",
    "flow",
    "lockon",
    "math",
    "navicust",
    "panels",
    "pools",
    "reactions",
    "sp_chips",
    "status",
];

/// The rule sections a ruleset states, whatever else it does: those with a
/// rule that has no neutral value (a choice between games' behaviors, a
/// size, a speed). The engine has no game's to fall back on.
pub(crate) const REQUIRED: &[&str] = &["chip_use", "effects", "flow", "panels", "pools", "reactions", "status"];

/// What is stated of the rules, by section: a ruleset's sections, over
/// the content's Rust tables when it has them.
#[derive(Default)]
struct Stated {
    elements: Option<([[u8; 6]; 6], [SecondaryElements; 15])>,
    panels: Option<PanelRules>,
    reactions: Option<ReactionsSection>,
    berserk: Option<BerserkRules>,
    sine: Option<Vec<i16>>,
    pools: Option<PoolSizes>,
    custom_screen: Option<CustomScreenLayout>,
    buster: Option<BusterSection>,
    navicust: Option<NaviCustRules>,
    holding_banners: Option<Vec<BannerId>>,
    status: Option<StatusSection>,
    lockon: Option<Lockon>,
    chip_use: Option<super::rules::ChipUseRules>,
    flow: Option<super::rules::FlowRules>,
    effects: Option<super::rules::EffectsRules>,
    sp_chips: Option<SpChipsSection>,
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
            }),
            berserk: Some(r.berserk),
            sine: Some(r.sine.clone()),
            pools: Some(r.pools),
            custom_screen: Some(r.custom_screen.clone()),
            buster: Some(BusterSection { recovery: r.buster_recovery.clone(), empty_hand: r.empty_hand, chaos_cycle: r.chaos_cycle.clone() }),
            navicust: Some(r.navicust.clone()),
            holding_banners: Some(r.holding_banners.clone()),
            status: Some(StatusSection {
                hp_bug_periods: r.hp_bug_periods,
                form_tick: r.form_tick,
                flash_hides_on_clear: r.flash_hides_on_clear,
                missing_collision_status: r.missing_collision_status.0,
                reactions: r.reactions,
                bugs_before_drain: r.intake.bugs_before_drain,
                drain_bug_flags: r.intake.drain_bug_flags,
                no_charge_drive: r.intake.no_charge_drive,
                hp_loss: r.intake.hp_loss,
                emotions: r.emotions,
                form_break: r.form_break,
            }),
            lockon: Some(r.lockon.clone()),
            chip_use: Some(r.chip_use),
            flow: Some(r.flow),
            effects: Some(r.effects),
            sp_chips: Some(SpChipsSection { deletion_times: r.sp_deletion_times.clone(), slots: r.sp_slots.clone() }),
        }
    }

    /// The rules: every section of [`REQUIRED`] stated (else an error at
    /// `at` that names the first one that isn't), the others' tables empty
    /// where nothing states them.
    fn rules(self, at: &str) -> Result<Rules, ContentError> {
        let missing = |name: &str| {
            ContentError::new(format!(
                "{at}: it states no `{name}` section: a ruleset states every rule of its game ({}), and the engine has no game's rules of its own",
                REQUIRED.join(", ")
            ))
        };
        let chip_use = self.chip_use.ok_or_else(|| missing("chip_use"))?;
        let effects = self.effects.ok_or_else(|| missing("effects"))?;
        let flow = self.flow.ok_or_else(|| missing("flow"))?;
        let mut panels = self.panels.ok_or_else(|| missing("panels"))?;
        let pools = self.pools.ok_or_else(|| missing("pools"))?;
        let reactions = self.reactions.ok_or_else(|| missing("reactions"))?;
        let status = self.status.ok_or_else(|| missing("status"))?;
        panels.types.resize(PanelType::ALL.len(), PanelTypeRule::default());
        let (element_weakness, family_elements) = self.elements.unwrap_or_default();
        let buster = self.buster.unwrap_or(BusterSection { recovery: Vec::new(), empty_hand: EmptyHandChip::default(), chaos_cycle: Vec::new() });
        let sp_chips = self.sp_chips.unwrap_or(SpChipsSection { deletion_times: Vec::new(), slots: Vec::new() });
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
            emotions: status.emotions,
            form_break: status.form_break,
            intake: super::rules::IntakeRules {
                bugs_before_drain: status.bugs_before_drain,
                drain_bug_flags: status.drain_bug_flags,
                no_charge_drive: status.no_charge_drive,
                hp_loss: status.hp_loss,
            },
            empty_hand: buster.empty_hand,
            buster_recovery: buster.recovery,
            chaos_cycle: buster.chaos_cycle,
            sp_deletion_times: sp_chips.deletion_times,
            sp_slots: sp_chips.slots,
            sine: self.sine.unwrap_or_default(),
            push_vectors: reactions.push,
            push_reading: reactions.push_reading,
            hit_test: reactions.hit_test,
            obstacle_slide_bounds: reactions.obstacle_slide_bounds,
            ice_vectors: reactions.ice,
            slide_speed: reactions.slide_speed,
            overlay_restart: reactions.overlay_restart,
            stance_counter: reactions.stance_counter,
            bubble_bob: reactions.bubble_bob,
            lockon: self.lockon.unwrap_or_default(),
            berserk: self.berserk.unwrap_or_default(),
            flow,
            effects,
            chip_use,
            custom_screen: self.custom_screen.unwrap_or_default(),
            pools,
            navicust: self.navicust.unwrap_or_default(),
        })
    }
}

/// The content's rules: what its ruleset states over its Rust tables
/// (`base`), if it has either. A ruleset that leaves out a section of
/// [`REQUIRED`] that no Rust table states, or a field of one, is an error
/// naming it. Without a ruleset, the Rust tables alone; without either, a
/// game pack (`game`) is an error too, and modules of no game have no
/// rules (a test's: no battle can be made of them).
fn rules(base: Option<&Rules>, game: Option<&str>, r: &SpecReader, definitions: &Definitions) -> Result<Option<Rules>, ContentError> {
    let Some(d) = super::defs::ruleset(definitions) else {
        return match (base, game) {
            (None, Some(game)) => Err(ContentError::new(format!(
                "{game}/{}.luau: game pack {game} defines no ruleset (`define.ruleset`, its rules/init.luau): a game states its rules, and the engine has no game's rules of its own",
                nettai_content_api::packs::INIT
            ))),
            _ => Ok(base.cloned()),
        };
    };
    let mut stated = base.map(Stated::of).unwrap_or_default();
    let at = format!("{}.luau: ruleset", nettai_content_api::keys::module_path(&d.module));
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
            "berserk" => {
                let s: BerserkSection = r.read(spec, &at).map_err(e)?;
                stated.berserk =
                    Some(BerserkRules { step: s.step.rules(), opponent: s.opponent, blocking: s.blocking, opposing_player: s.opposing_player });
            }
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
                stated.custom_screen = Some(CustomScreenLayout {
                    slots: s.slots,
                    left_scan_top: s.left_scan_top,
                    left_scan_bottom: s.left_scan_bottom,
                    right_scan_top: s.right_scan_top,
                    right_scan_bottom: s.right_scan_bottom,
                    left_scan_start: s.left_scan_start,
                    right_scan_start: s.right_scan_start,
                    redeal_kept: s.redeal_kept,
                });
            }
            "buster" => stated.buster = Some(r.read(spec, &at).map_err(e)?),
            "navicust" => {
                let s: NaviCustSection = r.read(spec, &at).map_err(e)?;
                use crate::navicust::SIZE;
                let mut boards = Vec::with_capacity(s.boards.len());
                for (i, rows) in s.boards.iter().enumerate() {
                    let bad = || e(format!("{at}: board {} is {SIZE} rows of {SIZE} cells (`o` the board, `f` its frame, `.` none)", i + 1));
                    if rows.len() != SIZE {
                        return Err(bad());
                    }
                    let mut board = [[BoardCell::Off; SIZE]; SIZE];
                    for (y, row) in rows.iter().enumerate() {
                        if row.len() != SIZE {
                            return Err(bad());
                        }
                        for (x, c) in row.bytes().enumerate() {
                            board[y][x] = match c {
                                b'o' => BoardCell::On,
                                b'f' => BoardCell::Frame,
                                b'.' => BoardCell::Off,
                                _ => return Err(bad()),
                            };
                        }
                    }
                    boards.push(board);
                }
                if s.command_line as usize >= SIZE {
                    return Err(e(format!("{at}: the command line is a row of the grid (0 to {})", SIZE - 1)));
                }
                stated.navicust = Some(NaviCustRules { boards, command_line: s.command_line });
            }
            "banners" => stated.holding_banners = Some(r.read::<BannersSection>(spec, &at).map_err(e)?.holding),
            "status" => stated.status = Some(r.read(spec, &at).map_err(e)?),
            "lockon" => {
                let s: LockonSection = r.read(spec, &at).map_err(e)?;
                stated.lockon = Some(Lockon { column_shifts: s.column_shifts, clear_path: s.clear_path });
            }
            "chip_use" => stated.chip_use = Some(r.read(spec, &at).map_err(e)?),
            "flow" => stated.flow = Some(r.read(spec, &at).map_err(e)?),
            "effects" => stated.effects = Some(r.read(spec, &at).map_err(e)?),
            "sp_chips" => stated.sp_chips = Some(r.read(spec, &at).map_err(e)?),
            other => return Err(e(format!("{at}: the engine has no rule section `{other}` ({})", SECTIONS.join(", ")))),
        }
    }
    Ok(())
}

/// The content's rules (`Content::rules`): what its ruleset states, over
/// its Rust tables when it has them.
pub fn build(content: &mut Content, definitions: &Definitions) -> Result<(), ContentError> {
    let r = SpecReader::new(&content.assets, definitions);
    let games = content.scripts.games();
    let rules = rules(content.base_rules.as_ref(), games.first().map(String::as_str), &r, definitions)?;
    content.rules = rules;
    Ok(())
}
