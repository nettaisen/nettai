//! The ruleset's tables: data no single entity owns.

use super::{BannerId, ChipFamily, CustomScreenLayout, PanelCondition, PanelOffset, SecondaryElements};
use crate::field::PanelType;
use serde::{Deserialize, Serialize};

/// The rule section `flow` (docs/design/bn5-map.md §15.3 items 7 and 13,
/// §15.4): where the arena's game's battle flow differs from BN6's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowRules {
    /// The custom screen closes on the tick both results are in (BN5's
    /// Team Battle screen, 0x08025EF2), not on the next (BN6's
    /// `sub_8026A28`: `CustomScreens::committed`).
    #[serde(default)]
    pub custom_closes_with_results: bool,
    /// Before the custom screen opens, the transformation sequencer runs
    /// once more after the reversions (BN6's state 0x24, `sub_8008492`);
    /// BN5 opens it straight after them.
    #[serde(default = "yes")]
    pub sequencer_before_custom: bool,
    /// The fight checks for an escape (BN6's `sub_800AAD6`; BN5 has none).
    #[serde(default = "yes")]
    pub escape_check: bool,
    /// Ticks the result's banner holds before the round ends, at least
    /// (`sub_80081A4`, `sub_800825A`: BN6 102; 94 in a special battle,
    /// effect 2, BN5 65).
    pub result_wait: ResultWait,
    /// Presentation, read of a console's own game (the frontend's; the
    /// simulation reads neither): the custom screen's close starts the
    /// HUD's chip window too, so the next chip's name shows through the
    /// turn's banner (BN5's 0x080230CC calls `sub_801E012`'s counterpart,
    /// as a Japanese BN6 console's `sub_8026DC4` does; a US BN6 console's
    /// waits for the navi's first decision).
    #[serde(default)]
    pub chip_window_at_close: bool,
    /// Presentation, as above: the intro fades in from black on a set's
    /// first battle too (BN5's `sub_80E0684` counterpart, 0x080E0698, reads
    /// the byte it tests through a flags value rather than the battle
    /// state: one the open bus gives, never 1 or less; BN6's first battle
    /// fades in from white).
    #[serde(default)]
    pub intro_from_black: bool,
}

fn yes() -> bool {
    true
}

/// The result's wait, in ticks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultWait {
    pub normal: u16,
    pub special: u16,
}

impl Default for FlowRules {
    /// BN6's flow.
    fn default() -> FlowRules {
        FlowRules {
            custom_closes_with_results: false,
            sequencer_before_custom: true,
            escape_check: true,
            result_wait: ResultWait { normal: 0x66, special: 0x5E },
            chip_window_at_close: false,
            intro_from_black: false,
        }
    }
}

/// The rule section `chip-use` (docs/design/bn5-map.md §15.3 items 10 and
/// 11), read from the section of the chip's own root.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChipUseRules {
    /// The dimming handler (action 0x15) and the instant chips' action
    /// leave the action on the frame they run (BN5's 0x080EC318 and
    /// 0x080EC6F6), not on the next update after the dimming (BN6's
    /// `sub_80EBD9C`) nor, for a weapon's own effect, 8 ticks later
    /// (`sub_80EC39C`).
    #[serde(default)]
    pub leave_on_use: bool,
    /// Where AntiNavi's sparkle shows (`sub_800ABC6`), in pixels from the
    /// navi chip's panel's center: down the field, and up.
    pub anti_navi_sparkle: SparkleOffset,
}

/// The rule section `effects` (docs/design/bn5-map.md §15.3 items 15 and
/// 16), the arena's: the battle's shared effects where a game's touch the
/// simulation otherwise than BN6's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectsRules {
    /// What the camera shake draws its jitter from.
    #[serde(default)]
    pub shake: ShakeRule,
    /// A hit spark steps its sprite once as it starts (BN6's
    /// `sub_80E0864`); BN5's (0x080E0870) doesn't, so it lasts a tick
    /// longer.
    #[serde(default = "yes")]
    pub spark_steps_at_start: bool,
    /// How an object's collision types are set again (`sub_801A082`).
    #[serde(default)]
    pub retype: RetypeRule,
    /// Which pairs of collisions a hit passes between, and what a guard
    /// blocks (`sub_3007218`).
    #[serde(default)]
    pub resolve: ResolveRule,
    /// An obstacle's reaction has BN5's step for ColonelSoul's army
    /// (0x080CAB02 from its four reactions, docs/design/bn5-map.md §15.11:
    /// `kinds::obstacle::Soldiers`): one standing where an armed side can
    /// use it turns into that side's soldier (the role
    /// `kinds.obstacle_soldier`). Read of the obstacle's own game's rules
    /// (its kind's), not the arena's.
    #[serde(default)]
    pub obstacle_soldiers: bool,
}

impl Default for EffectsRules {
    /// BN6's.
    fn default() -> EffectsRules {
        EffectsRules {
            shake: ShakeRule::default(),
            spark_steps_at_start: true,
            retype: RetypeRule::default(),
            resolve: ResolveRule::default(),
            obstacle_soldiers: false,
        }
    }
}

/// How a hit resolves between a receiver and a hitter (`sub_3007218`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolveRule {
    /// BN6's: a submerged side meets only types 0x1008; a FloatShoe side
    /// only types 0x80; a guard is broken by types 0x2 (0x1002 when the
    /// hitter has 0x4000) and blocks with its spark unless the hitter has
    /// types 0x0C005000.
    #[default]
    Bn6,
    /// BN5's (0x0801691C): a submerged or bubbled side (0x80000004) meets
    /// only types 0x1008 or the Elec element; no FloatShoe test (BN5's
    /// types have no 0x80); a guard is broken by types 0x1002 and blocks
    /// with its spark unless the hitter has types 0x0C004000.
    Bn5,
}

/// How `sub_801A082` (an object's damage, hit modifier and collision types
/// set again: `reset_collision_types`) goes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetypeRule {
    /// BN6's: what it is and what it hits, marked as made while dimmed
    /// when the battle is.
    #[default]
    Bn6,
    /// BN5's (0x08016B9E): what it is alone, never marked; its store of
    /// what it hits goes to the row number plus 0x34, a BIOS address no
    /// write reaches, so it keeps hitting what it did. A bug code's
    /// garbage high byte is the target lookup's offset, as the setup's.
    Bn5,
}

/// How the camera shakes (`camera_doShakeEffect_80301e8`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShakeRule {
    /// BN6's: each console's camera draws from its own RNG1, on two
    /// channels (the primary and the secondary, `sub_80302B6`).
    #[default]
    Console,
    /// BN5's (0x08030D78): one channel, its jitter two draws from the
    /// battle's RNG2 each shaking tick, the shake held (and nothing drawn)
    /// while the battle is paused without dimming.
    Battle,
}

/// The speed of a navi's slides (ice, roads, BN5's metal: `sub_8016730`)
/// and drags (a push: `sub_80178D4`), 16.16 pixels a tick across and in
/// depth.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlideSpeed {
    pub x: i32,
    pub y: i32,
}

impl Default for SlideSpeed {
    /// BN6's: 10 pixels across, 6 in depth (BN5's 8: 0x0801361E and
    /// 0x080143A8).
    fn default() -> SlideSpeed {
        SlideSpeed { x: 0xA_0000, y: 0x6_0000 }
    }
}

/// A sparkle's place from a panel's center, in pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SparkleOffset {
    pub dy: i16,
    pub z: i16,
}

/// How a navi's hooks restart what it wears after an animation change, a
/// flinch or a drag (`sub_8011450`, `sub_80F06CE`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayRestart {
    /// BN6's `sub_80C44D2`: the overlay reloads its animation and steps
    /// its sprite at once.
    #[default]
    Step,
    /// BN5's 0x080C374E: it reloads its animation at its next step.
    Reload,
}

/// When the counter a stance's caught hit starts (`sub_80105F2`) runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StanceCounter {
    /// BN6's `sub_80105F2`: from the next tick.
    #[default]
    NextTick,
    /// BN5's 0x0800E340: its first step at once, as after a trap's catch.
    AtOnce,
}

/// How a navi's push (slide type 1) reads the hits it took.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PushReading {
    /// BN6's `sub_800E548`: the first of bits 2 to 5 of the hits' modifier,
    /// toward the navi's front; the 0x80 bit picks the last five rows.
    #[default]
    Bn6,
    /// BN5's 0x0800C9D8: the first of bits 2 to 5 of the unflipped
    /// hitters' modifier, else of the flipped ones' with the direction
    /// reversed.
    Bn5,
}

/// How a navi takes a hit's NaviCust bug where a game's differs from BN6's
/// (`sub_801AC6C`, `sub_80139F6`; the default).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct IntakeRules {
    /// The bug is taken before the HP bug drains (BN5's hit intake,
    /// 0x080178EC, calls 0x0801103E before 0x0800DFEC); BN6's after, so a
    /// drain bug's first drain comes a tick later.
    pub bugs_before_drain: bool,
    /// A drain bug's argument (codes 0x18 and 0x19) goes by its flags
    /// (BN5's 0x0801103E): with bit 4 it adds its low four bits (to at most
    /// 7), with bit 5 it subtracts them (to at least 0), else it raises the
    /// level to them (no lower level changes, and nothing is reloaded).
    /// BN6's adds the argument (to at most 7).
    pub drain_bug_flags: bool,
}

/// How the weakness request breaks a form (`Rules::form_break`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FormBreak {
    #[default]
    Bn6,
    Bn5,
}

/// Whose emotions a side's navi has (`Rules::emotions`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Emotions {
    #[default]
    Bn6,
    Bn5,
}

/// Global rules: element weakness, collision types, panels, banners,
/// statuses and the Beast Out lock-on.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rules {
    /// Extra damage multiplier by the receiver's element, then the
    /// hitter's (0 null, 1 fire, 2 aqua, 3 elec, 4 wood, 5 the drain
    /// element).
    pub element_weakness: [[u8; 6]; 6],
    /// The secondary elements each chip family adds to its attacks, by
    /// family.
    pub family_elements: [SecondaryElements; 15],
    pub panels: PanelRules,
    /// Banners that stay up until removed.
    pub holding_banners: Vec<BannerId>,
    /// The HP bug's drain period by bug level.
    pub hp_bug_periods: [u8; 8],
    /// BN6's per-form tick runs (`off_80EA93C`: `sub_80F0608`, MegaMan's
    /// and ChargeMan's: the Fire chips' charge, a form's height); BN5's
    /// table (0x080EB1E8) has none of it (rule section `status`).
    pub form_tick: bool,
    /// Which ticks of the mercy flash show the navi (rule section `status`):
    /// BN6's hides it while the flash timer's bit 1 is set
    /// (`sub_8016934`), BN5's while it is clear (0x080137B6): the same
    /// blink, two ticks out of phase. Presentation: visibility is no part
    /// of the simulation.
    pub flash_hides_on_clear: bool,
    /// Whose emotions the side's navi has (rule section `status`): BN6's
    /// (`sub_8015B54`, `sub_8015BEC`) or BN5's (0x08012740: a soul first,
    /// then anger, a mood of 0 and Full Synchro, a mood under 65 worried;
    /// 0x080127D6's setter leaving a mood of 0; the anger tick passing
    /// over AI index 23).
    pub emotions: Emotions,
    /// How the weakness request breaks a form (rule section `status`):
    /// BN6's (`sub_8015766`: a Cross or a Beast, to what it breaks to) or
    /// BN5's (0x080122C8: any form, to the base form; no animation 2, no
    /// overlay's stepping kept, the collision region left, fewer flags).
    pub form_break: FormBreak,
    /// How a navi takes a hit's NaviCust bug (rule section `status`, the
    /// navi's game's).
    pub intake: IntakeRules,
    /// What the charge rules read for an empty hand's chip.
    pub empty_hand: EmptyHandChip,
    /// Ticks of recovery after a buster shot, by Rapid stat, then by open
    /// panels ahead (0..=5).
    pub buster_recovery: Vec<[u8; 6]>,
    /// BN5's Chaos Unison cycle (0x08010650, rule section `buster`), a row
    /// by the chaos level (at most 2): its period and three bounds. While
    /// the B charge is full a counter runs through the period; under the
    /// first bound a release succeeds (the window 2), then 1 under the
    /// second, 0 under the third, 1 past it. None: no game's cycle.
    pub chaos_cycle: Vec<[u8; 4]>,
    /// The deletion times (BCD hours:minutes:seconds.hundredths) at which
    /// an SP navi chip's damage steps down (`DamageFormula::SpNavi`).
    pub sp_deletion_times: Vec<u32>,
    /// The SP navis whose deletion times a round's setup carries, in its
    /// order (`RoundSetup::sp_times`): an SP navi chip's formula names its
    /// slot by these names.
    pub sp_slots: Vec<String>,
    /// The Cross special's chips (`sub_802D5A8`): a row by the hundreds of
    /// the navi's base max HP (the first row up to 199, the last from its
    /// place on), each chip by key.
    pub cross_special: Vec<Vec<SpecialChip>>,
    /// The sine table (`math_sinTable`, which `math_cosTable` continues):
    /// 256 steps a turn, 1.0 = 0x100, over a turn and a half, so that the
    /// cosine of step `a` is entry `a + 64`.
    pub sine: Vec<i16>,
    /// Pushes by hit-modifier bit (+5 with 0x80).
    pub push_vectors: [SlideVector; 10],
    /// How a push reads the hit modifiers (docs/design/bn5-map.md §15.3
    /// item 2).
    pub push_reading: PushReading,
    /// Ice slides by the direction the navi last moved.
    pub ice_vectors: [SlideVector; 6],
    /// How fast a navi slides and is dragged (the reactions section's).
    pub slide_speed: SlideSpeed,
    /// How a navi's hooks restart what it wears (the reactions section's).
    pub overlay_restart: OverlayRestart,
    /// When a stance's counter runs (the reactions section's).
    pub stance_counter: StanceCounter,
    /// A bubbled navi's height, by bubble timer.
    pub bubble_bob: [i8; 32],
    pub lockon: Lockon,
    pub berserk: BerserkRules,
    /// The battle's flow where a game's differs (rule section `flow`, the
    /// arena's game's).
    pub flow: FlowRules,
    /// The battle's shared effects where a game's differ (rule section
    /// `effects`, the arena's game's).
    pub effects: EffectsRules,
    /// How a chip's use runs, by the chip's game (rule section `chip-use`;
    /// docs/design/rules-in-luau.md §7.5: a chip runs as its game wrote
    /// it).
    pub chip_use: ChipUseRules,
    /// The custom screen's slot layout.
    pub custom_screen: CustomScreenLayout,
    /// The object pools' sizes (rule section `pools`): a capacity-only
    /// limit, which a battle takes as the larger of its two players' games'
    /// (docs/design/rules-in-luau.md §2.3).
    pub pools: PoolSizes,
    /// The NaviCust's board (rule section `navicust`; none: the game has no
    /// NaviCust): what a setup's programs may cover. What they do is the
    /// game's rules' (BN6's navicust system).
    pub navicust: NaviCustRules,
}

/// A cell of the NaviCust's grid, on a board.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BoardCell {
    /// No cell: no program can cover it.
    #[default]
    Off,
    /// The frame around the board: a program may jut onto it (BN6: and is
    /// bugged).
    Frame,
    /// The board.
    On,
}

/// The NaviCust's board: its 7x7 grid's cells by row, then column.
pub type Board = [[BoardCell; crate::navicust::SIZE]; crate::navicust::SIZE];

/// The NaviCust's boards, by how far it has been expanded (BN6's: 4x4, 5x4
/// and 5x5, by key item 0x71), and its command line (a row).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct NaviCustRules {
    pub boards: Vec<Board>,
    pub command_line: u8,
}

impl NaviCustRules {
    /// The board of a NaviCust with `expansions`, if the game has one.
    pub fn board(&self, expansions: u8) -> Option<&Board> {
        self.boards.get(expansions as usize)
    }

    /// Whether a program of `shape` can be placed with its center at
    /// `(x, y)` on `board` (BN6's `sub_813BB00`): every cell it covers is a
    /// cell of the board or its frame, and not all of them the frame.
    pub fn fits(board: &Board, shape: &crate::navicust::Shape, x: u8, y: u8) -> bool {
        let n = crate::navicust::SIZE as i32;
        let mut on = false;
        for (cx, cy) in crate::navicust::cells(shape, x, y) {
            if !(0..n).contains(&cx) || !(0..n).contains(&cy) {
                return false;
            }
            match board[cy as usize][cx as usize] {
                BoardCell::Off => return false,
                BoardCell::Frame => {}
                BoardCell::On => on = true,
            }
        }
        on
    }
}

/// How many objects each pool holds (BN6's are 32 each; BN5's actor pool
/// 16). At most `object::SLOTS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoolSizes {
    pub actor: u8,
    pub attack: u8,
    pub effect: u8,
}

impl Default for PoolSizes {
    fn default() -> PoolSizes {
        let n = crate::object::SLOTS as u8;
        PoolSizes { actor: n, attack: n, effect: n }
    }
}

impl PoolSizes {
    /// The sizes in pool order (actor, attack, effect).
    pub fn slots(&self) -> [u8; 3] {
        [self.actor, self.attack, self.effect]
    }
}

/// One of the Cross special's chips (`sub_802D4F0`): the chip its
/// controller uses, by key, with another chip's damage where the original
/// takes it from one (the last row's LifeSrd strikes with VarSwrd's).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecialChip {
    pub chip: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub damage_of: Option<String>,
}

impl Rules {
    /// Whether a banner stays up until removed.
    pub fn banner_holds(&self, id: BannerId) -> bool {
        self.holding_banners.contains(&id)
    }

    /// The secondary elements a chip family adds.
    pub fn family_elements(&self, family: ChipFamily) -> SecondaryElements {
        self.family_elements[family as usize]
    }

    /// Ticks of recovery after a buster shot at a Rapid stat with `open`
    /// open panels ahead (counted up to 5). A Rapid past the table reads
    /// on into the next row, as in the game.
    pub fn buster_recovery(&self, rapid: u8, open: u8) -> u8 {
        let i = rapid as usize * 6 + open.min(5) as usize;
        self.buster_recovery[i / 6][i % 6]
    }
}

/// Panel rules.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PanelRules {
    /// By panel type (`PanelType as usize`).
    pub types: Vec<PanelTypeRule>,
    /// Whether each panel shows at the start of a round, `[y][x]`.
    pub start_visible: [[bool; 8]; 5],
    /// Whether each panel draws its front edge, `[y][x]`.
    pub front_edges: [[bool; 8]; 5],
    /// What a panel must be to step onto (`tbl_800E660`), in dash mode
    /// (`byte_8010388`), and ignoring ownership (`byte_800E6C8`).
    pub step: StepRuleSet,
    pub dash_step: StepRuleSet,
    pub any_side_step: StepRuleSet,
    /// Ticks a broken panel stays broken, and in battle mode 1 (BN6: 0x258
    /// and 0x1E0, `sub_800C4BC`; BN5: 600 in both, 0x0800A998).
    pub mend: u16,
    pub mend_in_battle_mode_1: u16,
}

impl PanelRules {
    /// The flag bits a panel type contributes to a panel's flags word
    /// (with the type itself in the low nibble).
    pub fn type_flags(&self, t: PanelType) -> u32 {
        t as u32 | self.types[t as usize].flags
    }

    /// Where a road panel carries a navi.
    pub fn road_slide(&self, t: PanelType) -> Option<SlideVector> {
        self.types[t as usize].road_slide
    }
}

/// What one panel type is, and what it does (docs/design/bn5-map.md
/// §15.2; the behaviors are the engine's, keyed by the panel type, their
/// numbers the game's).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PanelTypeRule {
    /// Flag bits the type adds to a panel's flags word.
    pub flags: u32,
    /// For roads: where they carry a navi.
    pub road_slide: Option<SlideVector>,
    /// The sound a NaviCust panel trail makes turning a panel into the
    /// type (`byte_8013D44`; none: silent).
    pub trail_sound: Option<crate::sound::SoundId>,
    /// Ticks the type lasts before the panel turns normal, blinking its
    /// last 60 (BN6's roads 0x708, `sub_800C380`; BN5's lava and sea 960,
    /// 0x0800A998).
    pub expires: Option<u16>,
    /// The fire damage a grounded body standing on it takes, shifted by
    /// its weakness to fire, as the panel turns normal (BN5's lava,
    /// 0x08016D80 and 0x08016E18).
    pub burn: Option<u16>,
    /// The element of the bodies it drains as poison drains any (BN5's
    /// sea: fire, 0x08016C7E).
    pub drains: Option<u8>,
    /// Ticks a body that ends a move on it is held there, with a splash
    /// (BN5's sea, 0x0801715E).
    pub holds: Option<u16>,
    /// A body that can dive (its AI's flag 0x20) is submerged while on it,
    /// and no body is submerged off it (BN5's sea, 0x08017030).
    pub submerges: bool,
    /// A move's end on it starts a slide (slide type 3), tried in turn by
    /// the direction of the move (BN5's metal, 0x08017216, 0x0800C8A8).
    pub slide: Option<PanelSlide>,
    /// The element of the hitboxes that turn it normal as they pass over
    /// it (`sub_3007708`: fire grass, aqua the volcano, wood roads; BN5's
    /// 0x08016D14: and aqua lava, wood metal).
    pub cleared_by: Option<u8>,
    /// Whether the game's own section names the type; one it doesn't is
    /// the first other loaded game's that does (docs/design/rules-in-luau.md
    /// §7.4).
    pub named: bool,
}

/// A panel's slide (BN5's metal): by the direction the body last moved
/// (`CollisionData::direction`: none, up, down, back, forward, other),
/// the steps tried in turn, `dx` toward the body's front; the first one
/// the body can slide to is the slide, a panel at a time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PanelSlide {
    pub tries: [[Option<(i8, i8)>; 4]; 6],
}

/// Step rules by whether the object is floor-free (AirShoes, or standing
/// off solid ground), then by alliance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct StepRuleSet {
    pub grounded: [PanelCondition; 2],
    pub floor_free: [PanelCondition; 2],
}

impl StepRuleSet {
    pub fn get(&self, floor_free: bool, alliance: u8) -> PanelCondition {
        let rules = if floor_free { &self.floor_free } else { &self.grounded };
        rules[alliance as usize & 1]
    }
}

/// The status timer a status effect sets (a collision field).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusTimer {
    Paralyze,
    Confuse,
    Blind,
    Immobilize,
    Flash,
    /// `collision::timer::SUBMERGED`.
    Submerged,
    Invulnerable,
    Freeze,
    Bubble,
    /// Table garbage (statuses past a group's end): the collision panel.
    CollisionPanel,
    /// Table garbage: another collision field, by the original's offset.
    Other(u8),
}

/// A status effect (`define.status`): the requests it raises, its duration
/// and its timer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusEffect {
    /// Request bits (`ObjectFlags2`).
    pub requests: u32,
    pub duration: u16,
    pub timer: StatusTimer,
    /// The hit that lands it doesn't flinch or flash its target: applying
    /// it drops those requests (`sub_801A554`: the freezing statuses,
    /// the original's bytes 0x50 to 0x55).
    #[serde(default)]
    pub cancels_flinch: bool,
    /// A counter hit that lands it keeps it rather than paralyzing
    /// (`sub_800EB26`: the bubbling statuses, the original's bytes 0x60 to
    /// 0x65).
    #[serde(default)]
    pub survives_counter: bool,
}

/// The chip record an empty hand reads. A hand with no chip left holds
/// chip 0xFFFF, and some of the game's checks look it up in the chip table
/// without testing for that (`getChip8021DA8(0xFFFF)`), reading whatever
/// ROM data lies 0xFFFF records past the table. It is the same in both
/// versions; these are the bytes the engine's readers look at, decoded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmptyHandChip {
    /// Its family byte is the Null family's (`sub_8012F62`: in the Beast
    /// forms the alternative A-charge routine's threshold applies).
    pub null_family: bool,
    /// Its element byte is Fire's (`sub_80F0608`, ChargeCross).
    pub fire: bool,
    /// Its flags byte.
    pub flags: super::ChipFlags,
}

/// A slide or push: a direction and how many panels. In a content file,
/// `{ dx, dy, panels }`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlideVector {
    pub dx: i8,
    pub dy: i8,
    /// 0 = none; 6 = until blocked.
    #[serde(rename = "panels")]
    pub tiles: u8,
}

impl SlideVector {
    pub const NONE: SlideVector = SlideVector { dx: 0, dy: 0, tiles: 0 };
}

/// The panels Beast Over's berserk controller (`sub_802D322`) looks at,
/// each by alliance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BerserkRules {
    /// Where its steps may land (`byte_802D410`, and `byte_802D420` with
    /// AirShoe).
    pub step: StepRuleSet,
    /// A panel with an opponent on it (`off_8109784`, `sub_810971A`).
    pub opponent: [PanelCondition; 2],
    /// Panel flags that end the look behind an opponent (`byte_8015D78`,
    /// `sub_8015CC0`).
    pub blocking: [u32; 2],
    /// The panel flag of the opposing player (`byte_80E74C4`,
    /// `sub_80E7486`).
    pub opposing_player: [u32; 2],
}

/// The Beast Out lock-on: where the Beast rush attacks from (`ho_8026554`,
/// by the chip's lock-on mode, a definition: `Content::lockon`). What the
/// modes share.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Lockon {
    /// Column shifts toward the user tried, in order, when no panel next
    /// to the target fits (`byte_8026735`).
    pub column_shifts: Vec<i8>,
    /// What every panel between the chosen one and the target must be for
    /// the modes that need a clear path, by alliance (`byte_8026544`).
    pub clear_path: [PanelCondition; 2],
}

/// How a lock-on mode picks the panel to attack from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockonRule {
    /// Stay on the navi's own panel (`sub_802661C`, mode 0).
    #[default]
    Stay,
    /// Along the target's row, counting from the navi's own column
    /// (`sub_8026622`): `offsets` (or `same_row_offsets` when the target
    /// stands in the navi's row), then the same in the rows `row_shifts`
    /// away from the target's.
    Row,
    /// Next to the target (`sub_8026450`): the first panel of `offsets`
    /// the navi can stand on, then with the column shifts if
    /// `column_shifts` (`sub_80265D0`), and only with a clear path to the
    /// target if `clear_path` (`sub_80264A8`).
    Near,
}

/// A lock-on mode (`define.lockon`; an entry of the original's
/// `jt_8026584`). Offsets are relative to where the rule counts from, dx
/// toward the user's front; a panel past the target's column never fits.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockonMode {
    pub rule: LockonRule,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub offsets: Vec<PanelOffset>,
    /// `Row`: the offsets when the target is in the navi's row.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub same_row_offsets: Vec<PanelOffset>,
    /// `Row`: the rows tried after the target's, relative to it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub row_shifts: Vec<i8>,
    /// `Near`: the offsets when the target stands in the column farthest
    /// ahead of the user (`sub_80266BA`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub far_column_offsets: Option<Vec<PanelOffset>>,
    /// `Near`: try the column shifts too.
    #[serde(default)]
    pub column_shifts: bool,
    /// `Near`: every panel from the chosen one up to the target's column
    /// must meet `Lockon::clear_path`.
    #[serde(default)]
    pub clear_path: bool,
    /// Afterwards, the middle row of the chosen column is taken if the
    /// navi can stand there (`sub_80265FE`).
    #[serde(default)]
    pub prefers_middle_row: bool,
}

