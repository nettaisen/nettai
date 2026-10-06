//! The ruleset's tables: data no single entity owns.

use super::{BannerId, ChipFamily, CustomScreenLayout, PanelCondition, PanelOffset, SecondaryElements};
use crate::field::PanelType;
use super::flags::serde_flags;
use serde::{Deserialize, Serialize};

/// The rule section `flow` (docs/design/exe5-map.md §15.3 items 7 and 13,
/// §15.4): the battle's flow where games differ, the arena's game's. A
/// game states every one (no field has a default: the engine has no
/// game's flow of its own).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowRules {
    /// The words a custom screen's result takes on the link, one a tick
    /// (`sub_800B3A2`'s count: EXE6's 50; EXE5's 49, its NaviStats 0x60
    /// bytes to EXE6's 0x64), read of the sending side's game: the screens
    /// close on the tick after both results are in (`sub_8026A28`; EXE5's
    /// Team Battle screen's state 8, 0x08025FEC, after 0x080266FA).
    pub result_words: u8,
    /// Before the custom screen opens, the transformation sequencer runs
    /// once more after the reversions (EXE6's state 0x24, `sub_8008492`);
    /// EXE5 opens it straight after them.
    pub sequencer_before_custom: bool,
    /// The fight checks for an escape (EXE6's `sub_800AAD6`; EXE5 has none).
    pub escape_check: bool,
    /// Ticks the result's banner holds before the round ends, at least
    /// (`sub_80081A4`, `sub_800825A`: EXE6 102; 94 in a special battle,
    /// effect 2, EXE5 65).
    pub result_wait: ResultWait,
    /// Presentation, read of a console's own game (the frontend's; the
    /// simulation reads neither): the custom screen's close starts the
    /// HUD's chip window too, so the next chip's name shows through the
    /// turn's banner (EXE5's 0x080230CC calls `sub_801E012`'s counterpart,
    /// as a Japanese EXE6 console's `sub_8026DC4` does; a US EXE6 console's
    /// waits for the navi's first decision).
    pub chip_window_at_close: bool,
    /// Presentation, as above: the intro fades in from black on a set's
    /// first battle too (EXE5's `sub_80E0684` counterpart, 0x080E0698, reads
    /// the byte it tests through a flags value rather than the battle
    /// state: one the open bus gives, never 1 or less; EXE6's first battle
    /// fades in from white).
    pub intro_from_black: bool,
    /// Sound, read of the arena's game: the low-HP music switch (EXE6's
    /// `sub_8009158`: while a console's navi is at a quarter of its HP or
    /// less, the music plays a semitone higher and 282/256 as fast). EXE5
    /// has none: its ROM has no tempo or pitch control (EXE6's
    /// `sub_800065A` and `sound_8000672` have no counterpart), and none of
    /// its lab's sound recordings calls one.
    pub low_hp_music: bool,
    /// The battles whose win shows the winner's navi's banner (the navi's
    /// `banners.win`); any other win shows the role `win`'s, or
    /// `win_judged`'s on the judge's ruling. Each game states its own.
    pub navi_win_banner: NaviWinBanner,
}

/// The rule section `link_pick`: what a link battle picks at random with
/// its settings (EXE6's `sub_81209DC`, EXE5's 0x08129F2C: two numbers a
/// round, the first RNG's for the stage and the second's for the
/// background). For whoever makes a random match: the engine picks none (a
/// round's settings state its stage and background). Every ruleset states
/// it; one whose `stages` is empty has no random pick.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct LinkPick {
    /// The stages, by the pick's index: entry `i` is the stage of the
    /// settings record index `i` reaches (EXE6: `PosRNG1() % 0x60` into
    /// `BattleSettingsList1`, 96 stages each once; EXE5 picks the same way
    /// and then takes 76 off an index from 76 to 87, so its first twelve
    /// records are there twice and the twelve from 76 never). A stage is
    /// there as often as an index reaches a record that is it. (The
    /// section states stages, which `sections::link` resolves once they
    /// have their handles.)
    pub stages: Vec<nettai_content_api::StageHandle>,
    /// How many of `stages`, from the first, a set's first round is picked
    /// among: the pick's count for the match a random match is (a triple
    /// battle's practice, the effects `nettai_match::MATCH_EFFECTS`), which
    /// the comm menu passes for the first round. EXE6's goes by the match
    /// type (a triple battle's 0x60: all of them), EXE5's by practice or not
    /// (a practice's 0x44: the first 68). The rounds after pick among all of
    /// `stages` (both games pass the count 0x60 for them).
    pub first_round_stages: usize,
    /// The backgrounds, by the pick's index (EXE6's `byte_8120A20`, 21
    /// entries, some there twice and so twice as likely; EXE5's 27, each
    /// once). None: a link battle shows its stage's own.
    pub backgrounds: Vec<super::BackgroundId>,
}

/// The battles whose win shows the winner's navi's banner
/// (`FlowRules::navi_win_banner`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NaviWinBanner {
    /// Every link battle (EXE6's `sub_80081A4`: `sub_800A8D4`'s table by
    /// the navi, whatever the result).
    LinkBattle,
    /// The link battles that are operation battles, battle flag 0x40
    /// (EXE5's 0x080074D2: 0x080090E8, then 0x080090C0's table by the
    /// navi): a Team Battle's and a NetBattle's win shows the roles'.
    OperationBattle,
}

/// The result's wait, in ticks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultWait {
    pub normal: u16,
    pub special: u16,
}

/// The rule section `chip-use` (docs/design/exe5-map.md §15.3 items 10 and
/// 11), read from the section of the chip's own root. A game states every
/// one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChipUseRules {
    /// The dimming handler (action 0x15) and the instant chips' action
    /// leave the action on the frame they run (EXE5's 0x080EC318 and
    /// 0x080EC6F6), not on the next update after the dimming (EXE6's
    /// `sub_80EBD9C`) nor, for a weapon's own effect, 8 ticks later
    /// (`sub_80EC39C`).
    pub leave_on_use: bool,
    /// Where AntiNavi's sparkle and AntiRecv's mark show (`sub_800ABC6`),
    /// in pixels from the panel's center (the navi chip's, the healer's):
    /// down the field, and up.
    pub anti_navi_sparkle: SparkleOffset,
    /// The hand's modifier bits 0x08, 0x10 and 0x20 count at a chip's use
    /// (EXE5's capsules: 0x08010368 turns 0x08 and 0x20 into the damage
    /// word's 0x1000 and 0x0800, and 0x0800FFF6 heals the user a tenth of
    /// its HP on 0x10); EXE6's `sub_8012C34` knows bits 0x02 and 0x04 alone.
    pub mixed_modifiers: bool,
}

/// The rule section `effects` (docs/design/exe5-map.md §15.3 items 15 and
/// 16), the arena's: the battle's shared effects where games' touch the
/// simulation differently. A game states every one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectsRules {
    /// What the camera shake draws its jitter from.
    pub shake: ShakeRule,
    /// A hit spark steps its sprite once as it starts (EXE6's
    /// `sub_80E0864`); EXE5's (0x080E0870) doesn't, so it lasts a tick
    /// longer.
    pub spark_steps_at_start: bool,
    /// How an object's collision types are set again (`sub_801A082`).
    pub retype: RetypeRule,
    /// How a damage word's flag bits decode (`sub_8019F44`).
    pub damage_word: DamageWordRule,
    /// An obstacle's reaction has EXE5's step for ColonelSoul's army
    /// (0x080CAB02 from its four reactions, docs/design/exe5-map.md §15.11:
    /// `kinds::obstacle::Soldiers`): one standing where an armed side can
    /// use it turns into that side's soldier (the role
    /// `kinds.obstacle_soldier`).
    pub obstacle_soldiers: bool,
    /// What holds a screen palette flash (effect object #0x0A,
    /// `kinds::palette_flash`) by its mode.
    pub palette_flash: PaletteFlashRule,
    /// Where that flash sits among the palette transforms a frame
    /// applies, which a dimming's fade is one of. Presentation: the
    /// renderer's (`Fade::Flash`); the simulation reads none of it.
    pub palette_flash_order: PaletteFlashOrder,
    /// An afterimage (`sub_80E33FA`) and a form overlay (`sub_80C4530`'s
    /// spawner) run while the battle is paused: EXE6's spawners set their
    /// header flag 0x04; EXE5's (0x080E35F4, and its overlays', whose flags
    /// its lab records without it) don't.
    pub overlays_run_while_paused: bool,
    /// An afterimage that copies its owner (`sub_80E32D8` with no sprite of
    /// its own) takes the owner's NameID and wears what that record's init
    /// hook puts on, taken off as it goes: EXE6's. EXE5's afterimage
    /// (0x080E3550) has no such mode: its spawners give it the owner's
    /// battle sprite (0x0800DA72) and it wears nothing, so an afterimage of
    /// a navi that wears something (KnightMan's ball, NumberMan's face) is
    /// the navi alone.
    pub afterimages_wear_overlays: bool,
    /// Loading an animation's frame (`sprite_loadAnimationData`) takes the
    /// palette offset of the frame's first part, which a sprite is drawn
    /// with (EXE6's `sub_3006730`); EXE5's (0x03006898) leaves it to the
    /// sprite's next step, so a sprite drawn before it steps again keeps
    /// its last step's offset, or 0 when just loaded (EXE5's hit spark,
    /// which doesn't step as it starts, shows its first frame in its
    /// palette 0). Presentation: `Look::part_palette`.
    pub load_sets_part_palette: bool,
    /// How the game's obstacles number their action tables.
    pub obstacle_actions: ObstacleActions,
    /// The Full Synchro aura where games differ.
    pub full_synchro_aura: AuraRules,
}

/// How a game's obstacles number their action tables (`kinds::obstacle`):
/// the framework's entries first, then the kind's own (its idle, then the
/// rest).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObstacleActions {
    /// The actors' frozen and bubbled entries at 6 and 7, the kind's own
    /// from 8 (EXE6's).
    #[serde(rename = "own_from_8")]
    OwnFrom8,
    /// Neither frozen nor bubbled, the kind's own from 6 (EXE5's, as its
    /// navis' state table, 0x080EAE08).
    #[serde(rename = "own_from_6")]
    OwnFrom6,
}

/// How the collision setup (`object_setupCollisionData`'s and
/// `sub_801A082`'s call of `sub_8019F44`) decodes the flag bits of an
/// object's damage word: the damage is its low 11 bits, doubled with
/// 0x8000.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DamageWordRule {
    /// A paralysis and two bug codes (EXE6's `sub_8019F44`): 0x4000 the
    /// role `statuses.damage_word_paralysis` with hit modifier 1 (a flinch
    /// too); then 0x2000 bug code 0xF8 (and no more), else 0x1000 bug code
    /// 0xF7, each code's high byte what the caller left in r1.
    ParalysisAndBugs,
    /// Three statuses and a bug code (EXE5's 0x080165EC): 0x4000 the
    /// paralysis (status byte 0x10) with hit modifier 0 (no flinch), and no
    /// more; else 0x2000 the role `statuses.damage_word_confusion` (0x20),
    /// and no more; else 0x1000 `statuses.damage_word_blindness` (0x30);
    /// then 0x800 bug code 0x18 with high byte 0x11.
    StatusesAndBug,
}

/// What holds a screen palette flash (effect object #0x0A) while the battle
/// is paused or dimmed, by its mode (Param3: bit 0 keeps it flashing while
/// dimmed, bit 1 while paused).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaletteFlashRule {
    /// The mode's bit 1 keeps it going through a pause, and through
    /// dimming (EXE6's `sub_80E10C0`, `sub_80E114C`); else a pause holds
    /// it, and dimming does unless the variant's bit is set (variant 0
    /// tests bit 0, variant 1 bit 1).
    ModeRunsThroughPause,
    /// A pause holds either variant whatever its mode (EXE5's 0x080E104C,
    /// 0x080E10D0); dimming holds it only with a mode of 0.
    PauseHolds,
}

/// Where a screen palette flash (effect object #0x0A) sits among the
/// palette transforms. Each frame the game copies its palettes and applies
/// the transforms set in a table of slots, in the slots' order (EXE6's
/// `sub_80023E0`'s table, EXE5's 0x08002350's); the fade system's level
/// fades take slots 18 and 19 for its first record, a dimming's and a
/// transformation's, and 20 and 21 for its second (EXE6's 0x08006396,
/// EXE5's 0x08005B42: the record's number plus 18 and 19). A flash fills
/// the stage's palettes with its white through a slot of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaletteFlashOrder {
    /// Before the fades: EXE5's flash takes slot 9 (0x080E104C; its
    /// two-layer one 9 and 10, 0x080E10D0), so a dimming darkens the white
    /// it put there with the rest: a flash under a dimming is white a
    /// quarter down (Blinder's, Colonel's, OmegaRkt's, LeadRaid's).
    BeforeFades,
    /// After the first record's: EXE6's takes slot 20 (`sub_80E10C0`; its
    /// two-layer one 20 and 21, `sub_80E114C`), so its white stands over a
    /// dimming (Colonel's, DeltaRay's, CrossDiv's, the navi advances').
    AfterFades,
}

/// How `sub_801A082` (an object's damage, hit modifier and collision types
/// set again: `reset_collision_types`) goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetypeRule {
    /// What it is and what it hits are set, marked as made while dimmed
    /// when the battle is (EXE6's `sub_801A082`).
    IsAndHits,
    /// What it is alone is set, never marked (EXE5's 0x08016B9E): its
    /// store of what it hits goes to the row number plus 0x34, a BIOS
    /// address no write reaches, so it keeps hitting what it did. A bug
    /// code's garbage high byte is the target lookup's offset, as the
    /// setup's.
    IsAlone,
}

/// How the camera shakes (`camera_doShakeEffect_80301e8`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShakeRule {
    /// Each console's camera draws from its own RNG1, on two channels (the
    /// primary and the secondary: EXE6's, `sub_80302B6`).
    ConsoleRng,
    /// One channel, its jitter two draws from the battle's RNG2 each
    /// shaking tick, the shake held (and nothing drawn) while the battle
    /// is paused without dimming (EXE5's, 0x08030D78).
    BattleRng,
}

/// The speed of a navi's slides (ice, roads, EXE5's metal: `sub_8016730`)
/// and drags (a push: `sub_80178D4`), 16.16 pixels a tick across and in
/// depth (EXE6's 10 pixels across and 6 in depth; EXE5's 10 and 8,
/// 0x0801361E and 0x080143A8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlideSpeed {
    pub x: i32,
    pub y: i32,
}

/// The rule section `fresh_stats`: what a navi's stats hold when they are
/// made fresh (`NaviStats::fresh`), beyond what the navi's own row states
/// (its `fresh` and `weapons`): what the game's routine writes for every
/// navi (EXE6's `initNaviStats_WithDefaultStatsMaybe_8013438`, EXE5's
/// 0x08010C00). A game states the first three; the engine has none of its
/// own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FreshStatsRules {
    /// NaviStats +0x09: the Regular memory.
    pub reg_up: u8,
    /// +0x0A: the custom level.
    pub custom_level: u8,
    /// +0x0E: the mood.
    pub mood: u8,
    /// +0x21: EXE6's Beast Out turns (3). None stated: none (a game
    /// without Beast Out).
    pub beast_out_counter: u8,
    /// +0x44: the weapon of the A button in battle mode 9 (EXE6's zeroed
    /// byte names weapon routine 0, MegaMan's buster). None stated: none
    /// (EXE5's block has the light/dark value there, its light and dark
    /// system's).
    pub mode9_a: Option<nettai_content_api::WeaponHandle>,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayRestart {
    /// EXE6's `sub_80C44D2`: the overlay reloads its animation and steps
    /// its sprite at once.
    Step,
    /// EXE5's 0x080C374E: it reloads its animation at its next step.
    Reload,
}

/// Requests an end clears, in a content file a list of their names: the
/// anti-sword trigger, the mode-9 A press, the Chaos Unison releases.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RequestSet(pub u32);

impl RequestSet {
    pub const NAMES: &'static [(u32, &'static str)] = &[
        (crate::actor::request::ANTI_SWORD_TRIGGERED, "anti_sword_triggered"),
        (crate::actor::request::MODE9_A, "mode9_a"),
        (crate::actor::request::CHAOS_SUCCESS, "chaos_success"),
        (crate::actor::request::CHAOS_FAILURE, "chaos_failure"),
    ];
}

serde_flags!(RequestSet, u32);

/// What the ends of a navi's actions clear of its requests besides the six
/// attack requests (`request::ATTACKS`): the original's masks, which
/// differ by game. EXE6's are 0x1000003F at an attack's end (`sub_801171C`)
/// and a paralysis's, freeze's or bubble's, and 0x1000043F at a flinch's
/// and a drag's: the mode-9 A press, and the anti-sword trigger. EXE5's are
/// 0x1803F at an attack's end (0x0800F2DA) and 0x1843F at a paralysis's, a
/// flinch's and a drag's (0x080142A6, 0x080141D0, 0x080144E0): the Chaos
/// Unison releases, and the anti-sword trigger.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestClears {
    pub attack: RequestSet,
    /// A paralysis's end, and a freeze's and a bubble's.
    pub paralysis: RequestSet,
    pub flinch: RequestSet,
    pub drag: RequestSet,
}

/// When the counter a stance's caught hit starts (`sub_80105F2`) runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StanceCounter {
    /// EXE6's `sub_80105F2`: from the next tick.
    NextTick,
    /// EXE5's 0x0800E340: its first step at once, as after a trap's catch.
    AtOnce,
}

/// How a navi's push (slide type 1) reads the hits it took, and an
/// obstacle's push the same way (`kinds::obstacle`: an obstacle's vector,
/// and its push on any hit).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PushReading {
    /// The first of bits 2 to 5 of the hits' modifier, toward the navi's
    /// front; the 0x80 bit picks the last five rows (EXE6's
    /// `sub_800E548`).
    TowardFront,
    /// The first of bits 2 to 5 of the unflipped hitters' modifier, else
    /// of the flipped ones' with the direction reversed (EXE5's
    /// 0x0800C9D8; an obstacle's 0x0800D4B0 and 0x08017AD8).
    ByHitterFlip,
}

/// The hit test where games differ (EXE6's `sub_3007218`, EXE5's
/// 0x0801691C): which pairs of collisions a hit can't join. In both, a
/// submerged body (flag 0x4) meets only collision types with 0x8 or
/// 0x1000, and a guard turns aside the types it doesn't break to that
/// have none of 0x0C005000 (EXE5's own mask, 0x0C004000, differs only for
/// a type with 0x1000, which its guard always breaks to).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HitTest {
    /// A FloatShoe body (flag 0x20) meets only collision types with the
    /// 0x80 self bit (EXE6's); else it meets all (EXE5's types have no
    /// self bit, and neither its test nor its unfiltered channel,
    /// 0x08017494, looks).
    pub float_shoe_needs_self_bit: bool,
    /// The collision flag 0x80000000 is a body under the sea's surface,
    /// tested as a submerged one (EXE5's flags 0x80000004); else it is a
    /// bubble, on which an elec hit counts once more (EXE6's).
    pub bubbled_as_submerged: bool,
    /// An elec hit reaches a submerged body whatever its type (EXE5's).
    pub elec_reaches_submerged: bool,
    /// The type bits a guard breaks to (EXE6's 0x2, EXE5's 0x1002); a type
    /// with 0x4000 breaks one by 0x1002 in either.
    pub guard_breaks_to: u32,
    /// An elec hit counts once more as null damage on a body standing on a
    /// sea panel, as a fire hit does on grass (EXE5's 0x08016AF6).
    pub elec_bonus_on_sea: bool,
}

/// How a navi takes a hit's NaviCust bug, where games differ (`sub_801AC6C`,
/// `sub_80139F6`): the status section's, each stated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IntakeRules {
    /// The bug is taken before the HP bug drains (EXE5's hit intake,
    /// 0x080178EC, calls 0x0801103E before 0x0800DFEC); EXE6's after, so a
    /// drain bug's first drain comes a tick later.
    pub bugs_before_drain: bool,
    /// A drain bug's argument (codes 0x18 and 0x19) goes by its flags
    /// (EXE5's 0x0801103E): with bit 4 it adds its low four bits (to at most
    /// 7), with bit 5 it subtracts them (to at least 0), else it raises the
    /// level to them (no lower level changes, and nothing is reloaded).
    /// EXE6's adds the argument (to at most 7).
    pub drain_bug_flags: bool,
    /// EXE5's no-charge drive (DarkInvs, 0x080E2318): a navi with the
    /// no-charge state counts its drive's ticks down at the intake's end
    /// (0x0800DBE0) and asks for the stun strike when they run out (EXE5's
    /// action 0x49 ends the drive); its idle hands the step it would take
    /// to the side's systems' `controller` (0x080F03E4: the auto battle
    /// AI, 0x0802B4AC, or the reset of its state); and its last 180 ticks
    /// it flickers gray (0x080136E0). EXE6 has none of it.
    pub no_charge_drive: bool,
    /// How a navi loses HP (`object_subtractHP`, `applyDamageToPlayer`).
    pub hp_loss: HpLoss,
}

/// How a navi loses HP (`IntakeRules::hp_loss`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HpLoss {
    /// The HP alone goes (EXE6's `object_subtractHP`), and a hit sounds
    /// before the HP left is looked at (`applyDamageToPlayer_801ba12`).
    HpAlone,
    /// A player's loss also drains its side's gauge, and a last stand
    /// holds a navi at 1 HP (EXE5's 0x0800C6E0, 0x080185A2): a player
    /// MegaMan of emotion 5 (a dark MegaMan's) whom the loss brings to 0
    /// holds at 1 HP, once a battle, and asks for the volley (0x0802C16C:
    /// EXE5's action 0x30); and a hit shows (white, its sounds) only by
    /// the register that check leaves at 0 HP (`kinds::player::LastStand`).
    GaugeAndLastStand,
}

/// What a navi's status word (its collision data's flags 1) reads as while
/// the navi has no collision data, its init not yet run
/// (`Rules::missing_collision_status`): the game reads it through the null
/// pointer, from the BIOS, which gives the opcode the BIOS last fetched
/// (open bus). It happens on a round's first tick, when the other side's
/// navi inits first and its first barrier's visual asks whether the local
/// navi is blind (`sub_800EB6C`). EXE6's is the object spawn's fill's
/// (`ZeroFillByWord`'s CpuSet: 0xE3A02004), which has the blind bit; EXE5's
/// an interrupt's (0xE55EC002, read on the console at 0x0800CE18), which
/// hasn't. Each game states its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MissingCollisionStatus(pub u32);

/// How the weakness request breaks a form (`Rules::form_break`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormBreak {
    /// A form of the classes Cross, Beast and Cross Beast breaks, to what
    /// it breaks to (EXE6's `sub_8015766`: a Cross Beast to its Beast,
    /// else the base form), with animation 2, its overlay refreshed and
    /// kept stepping, and the collision region taken off and put back.
    CrossOrBeast,
    /// Any form breaks, to the base form (EXE5's 0x080122C8), with none of
    /// those, and fewer flags cleared.
    AnyForm,
}

/// What a navi's status routine tests to show the weakness mark
/// (`Rules::weakness_mark`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaknessMark {
    /// The multiplier of the last hit to reach the navi this tick, on a
    /// tick with final damage (EXE6's `sub_801A42E`: the byte its hit
    /// kernel stores at +0x74 of each hit, and the final damage at +0x80):
    /// a weakness of either kind, a thaw, elec on a bubble. A hit after it
    /// that nothing multiplies takes the mark away.
    LastHitMultiplier,
    /// The damage this tick's hits left of the element the navi's is weak
    /// to (EXE5's 0x08017254, whose hit kernel stores no multiplier: the
    /// accumulator the table 0x08017284 names by the navi's element, none
    /// for null; the engine reads the element off the weakness table): any
    /// hit of that element, whichever came last, and no test of the final
    /// damage.
    WeakElementDamage,
}

/// A side's emotions where games differ (the status section's `emotion`):
/// how the emotion is read off the navi (`kinds::player::emotion`: EXE6's
/// `sub_8015B54`, EXE5's 0x0801270C), what holds a mood and how anger
/// leaves it. Read in this order: battle mode 1's plain reading, a form's
/// normal, then anger and worn out (a mood of 0, or exhausted) in the
/// game's order, tired, Full Synchro (a mood of 0xFF), worried, normal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmotionRules {
    /// What holds a side's mood against the setter (`sub_8015BEC`, EXE5's
    /// 0x080127D6).
    pub mood_held: MoodHeld,
    /// How the end of anger leaves the mood (`sub_80143A6`, EXE5's
    /// 0x08011A94).
    pub anger_end: AngerEnd,
    /// In battle mode 1 a side is in Full Synchro (a mood of 0xFF) or
    /// normal, whatever else (EXE5's 0x080127C0).
    pub plain_in_battle_mode_1: bool,
    /// Out of its base form a navi reads as normal (EXE5's: in a soul, the
    /// soul's own face, which nothing doubles or ends).
    pub normal_in_a_form: bool,
    /// Anger is read before worn out (EXE5's: an angry navi at a mood of 0
    /// is angry); else worn out first (EXE6's).
    pub anger_before_worn_out: bool,
    /// The navi's held states are read: exhausted is worn out, and held
    /// tired its own emotion (EXE6's AIData +0x33 and +0x32); else neither
    /// (EXE5's routine reads no such byte).
    pub tired_and_exhausted: bool,
    /// A mood under this is worried (EXE5's 65); none stated: no mood is.
    #[serde(default)]
    pub worried_below: Option<u8>,
}

/// What holds a side's mood against the setter (`EmotionRules::mood_held`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoodHeld {
    /// Its navi held tired or exhausted (EXE6's `sub_8015BEC`).
    TiredOrExhausted,
    /// A mood of 0, which stays (EXE5's 0x080127D6: a dark MegaMan never
    /// reaches Full Synchro, and anger's end doesn't lift him).
    AtZero,
}

/// How the end of anger leaves the mood (`EmotionRules::anger_end`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AngerEnd {
    /// The mood goes to 0x80, whatever holds it (EXE6's `sub_80143A6`).
    ResetsMood,
    /// The mood goes to 0x80 through the setter: a held mood stays (EXE5's
    /// 0x08011A94).
    ThroughSetter,
}

/// The Full Synchro aura where games differ (the effects section's
/// `full_synchro_aura`; `kinds::full_synchro_aura`: EXE6's `sub_80C4B18`,
/// EXE5's actor object 0x5E, 0x080C45E0).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuraRules {
    /// Its animation follows its navi's identity each tick (EXE6's
    /// `sub_80C4B84`: a Cross's or a Beast's own as the navi changes form);
    /// else it keeps the one it started with (EXE5's 0x080C4648).
    pub follows_identity: bool,
    /// Its sprite steps while the battle is paused (EXE5's); else a pause
    /// stills it (EXE6's). Dimming stills it in both.
    pub steps_while_paused: bool,
    /// Once the fight is on it stops running while the battle is paused
    /// (EXE5's: its header flag goes); else it runs through every pause
    /// (EXE6's).
    pub stops_at_a_pause_in_the_fight: bool,
}

/// Global rules: element weakness, collision types, panels, banners,
/// statuses and the Beast Out lock-on. No default: a game's ruleset states
/// them (`content::sections`), and the engine has no game's of its own.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
    /// EXE6's per-form tick runs (`off_80EA93C`: `sub_80F0608`, MegaMan's
    /// and ChargeMan's: the Fire chips' charge, a form's height); EXE5's
    /// table (0x080EB1E8) has none of it (rule section `status`), its
    /// MegaMan's routine being the forms' own (`FormDef::tick`).
    pub form_tick: bool,
    /// Which ticks of the mercy flash show the navi (rule section `status`):
    /// EXE6's hides it while the flash timer's bit 1 is set
    /// (`sub_8016934`), EXE5's while it is clear (0x080137B6): the same
    /// blink, two ticks out of phase. Presentation: visibility is no part
    /// of the simulation.
    pub flash_hides_on_clear: bool,
    /// What a navi's status word reads as while it has no collision data
    /// (rule section `status`): see [`MissingCollisionStatus`].
    pub missing_collision_status: MissingCollisionStatus,
    /// How a navi's status block runs its reactions (rule section
    /// `status`).
    pub reactions: Reactions,
    /// A side's emotions where games differ (rule section `status`'s
    /// `emotion`): how one is read off the navi, what holds a mood, how
    /// anger leaves it.
    pub emotion: EmotionRules,
    /// How the weakness request breaks a form (rule section `status`):
    /// EXE6's (`sub_8015766`: a Cross or a Beast, to what it breaks to) or
    /// EXE5's (0x080122C8: any form, to the base form; no animation 2, no
    /// overlay's stepping kept, the collision region left, fewer flags).
    pub form_break: FormBreak,
    /// A damaging weakness hit asks for the form break (rule section
    /// `status`): a step of EXE6's status routine (`sub_801A506`). EXE5's
    /// routine has no such step (0x08017BF2: the mark, the counter's
    /// bookkeeping, the damage), and only a dark chip used in a soul asks
    /// (0x08010070): a weakness hit there shows its mark and breaks nothing.
    pub weakness_hit_breaks_form: bool,
    /// What the status routine tests to show the weakness mark (rule
    /// section `status`): EXE6's the last hit's multiplier, EXE5's the
    /// damage of the element the navi is weak to.
    pub weakness_mark: WeaknessMark,
    /// How a navi takes a hit's NaviCust bug (rule section `status`, the
    /// navi's game's).
    pub intake: IntakeRules,
    /// What the charge rules read for an empty hand's chip.
    pub empty_hand: EmptyHandChip,
    /// Ticks of recovery after a buster shot, by Rapid stat, then by open
    /// panels ahead (0..=5).
    pub buster_recovery: Vec<[u8; 6]>,
    /// EXE5's Chaos Unison cycle (0x08010650, rule section `buster`), a row
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
    /// What a link battle picks at random (the section `link_pick`; none
    /// stated: nothing).
    pub link_pick: LinkPick,
    /// The sine table (`math_sinTable`, which `math_cosTable` continues):
    /// 256 steps a turn, 1.0 = 0x100, over a turn and a half, so that the
    /// cosine of step `a` is entry `a + 64`.
    pub sine: Vec<i16>,
    /// Pushes by hit-modifier bit (+5 with 0x80).
    pub push_vectors: [SlideVector; 10],
    /// How a push reads the hit modifiers (docs/design/exe5-map.md §15.3
    /// item 2).
    pub push_reading: PushReading,
    /// The hit test where games differ (docs/design/exe5-map.md §15.3 item
    /// 12).
    pub hit_test: HitTest,
    /// A pulled obstacle's slide stays out of the puller's area (EXE6's
    /// `byte_8017F24`); else an obstacle slides anywhere open (EXE5's
    /// 0x08014894 keeps no bounds).
    pub obstacle_slide_bounds: bool,
    /// Ice slides by the direction the navi last moved.
    pub ice_vectors: [SlideVector; 6],
    /// How fast a navi slides and is dragged (the reactions section's).
    pub slide_speed: SlideSpeed,
    /// How a navi's hooks restart what it wears (the reactions section's).
    pub overlay_restart: OverlayRestart,
    /// When a stance's counter runs (the reactions section's).
    pub stance_counter: StanceCounter,
    /// What the ends of a navi's actions clear of its requests (the
    /// reactions section's).
    pub request_clears: RequestClears,
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
    /// What a navi's stats hold when made fresh, beyond its own row (rule
    /// section `fresh_stats`).
    pub fresh_stats: FreshStatsRules,
    /// The custom screen's slot layout.
    pub custom_screen: CustomScreenLayout,
    /// The object pools' sizes (rule section `pools`): a capacity-only
    /// limit, which a battle takes as the larger of its two players' games'
    /// (docs/design/rules-in-luau.md §2.3).
    pub pools: PoolSizes,
    /// The NaviCust's board (rule section `navicust`; none: the game has no
    /// NaviCust): what a setup's programs may cover. What they do is the
    /// game's rules' (EXE6's navicust system).
    pub navicust: NaviCustRules,
}

/// A cell of the NaviCust's grid, on a board.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BoardCell {
    /// No cell: no program can cover it.
    #[default]
    Off,
    /// The frame around the board: a program may jut onto it (EXE6: and is
    /// bugged).
    Frame,
    /// The board.
    On,
}

/// The NaviCust's board: its 7x7 grid's cells by row, then column.
pub type Board = [[BoardCell; crate::navicust::SIZE]; crate::navicust::SIZE];

/// The NaviCust's boards, by how far it has been expanded (EXE6's and EXE5's:
/// 4x4, 5x4 and 5x5, by key item 0x71 and EXE5's 0x61; EXE5's without a
/// frame), and its command line (a row).
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
    /// `(x, y)` on `board` (EXE6's `sub_813BB00`): every cell it covers is a
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

/// How many objects each pool holds (EXE6's are 32 each; EXE5's actor pool
/// 16). At most `object::SLOTS`. Each game states its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoolSizes {
    pub actor: u8,
    pub attack: u8,
    pub effect: u8,
}

impl PoolSizes {
    /// The sizes in pool order (actor, attack, effect).
    pub fn slots(&self) -> [u8; 3] {
        [self.actor, self.attack, self.effect]
    }
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
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
    /// Ticks a broken panel stays broken, and in battle mode 1 (EXE6: 0x258
    /// and 0x1E0, `sub_800C4BC`; EXE5: 600 in both, 0x0800A998).
    pub mend: u16,
    pub mend_in_battle_mode_1: u16,
    /// The game's panel types by its own numbers, which a panel trail's
    /// byte (NaviStats+0x12) names: EXE6's 13 (5 its holy, 8 its volcano,
    /// then the roads), EXE5's 11 (5 its metal, 8 its lava, 9 its holy, 10
    /// its sea).
    pub numbers: Vec<PanelType>,
    /// Whether a reservation marks its holder (`Reservations`).
    pub reservations: Reservations,
}

/// How a navi's status block (`sub_801AF44`'s top block, from the
/// action requests to the status timers) runs its reactions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reactions {
    /// A drag, then the slides, a flinch, then the mercy flash's timer
    /// (EXE6's `sub_801A5EE`, only while the battle is fighting); a drag
    /// or a flinch resets the attack's links and ends a freeze or a
    /// bubble, and keeps a paralysis that a counter just made or that no
    /// flash request comes with.
    FlashTimerLast,
    /// The mercy flash's timer first (EXE5's 0x08017CC8 on: 0x080173C4,
    /// whatever the battle's flags), then the slides, a drag and a flinch;
    /// a drag or a flinch ends a paralysis unless a counter just made it
    /// (0x08017084 unless its flag2 bit, the engine's 0x4000), and resets
    /// nothing else.
    FlashTimerFirst,
}

/// What reserving a panel does to its holder (the panels section's
/// `reservations`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reservations {
    /// EXE6's: the holder's header flag 0x20 is set (`object_reservePanel`),
    /// and its destroy releases what it holds (`sub_801BB78`, from
    /// `object_genericDestroy` and the navis' end).
    Marked,
    /// EXE5's: the holder isn't marked (its reserve, 0x0801865C, sets only
    /// the panel's), and nothing releases a destroyed holder's (its
    /// destroys, 0x080138B6 and 0x080138F2, free the collision and the
    /// object alone).
    Unmarked,
}

impl PanelRules {
    /// The panel type the game numbers `n` (a panel trail's byte).
    pub fn numbered(&self, n: u8) -> Option<PanelType> {
        self.numbers.get(n as usize).copied()
    }

    /// The flag bits a panel type contributes to a panel's flags word,
    /// with the type itself in the low nibble: the game's number of it
    /// (`numbers`: EXE5's holy is its 9, EXE6's its 5), which is what the
    /// original's word holds and what content reading the word's low byte
    /// reads (EXE5's GyroMan's Airforce, 0x080F0AD0). A type the game
    /// doesn't number has the engine's own.
    pub fn type_flags(&self, t: PanelType) -> u32 {
        let number = self.numbers.iter().position(|n| *n == t).unwrap_or(t as usize);
        number as u32 | self.types[t as usize].flags
    }

    /// Where a road panel carries a navi.
    pub fn road_slide(&self, t: PanelType) -> Option<SlideVector> {
        self.types[t as usize].road_slide
    }
}

/// What one panel type is, and what it does (docs/design/exe5-map.md
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
    /// last 60 (EXE6's roads 0x708, `sub_800C380`; EXE5's lava and sea 960,
    /// 0x0800A998).
    pub expires: Option<u16>,
    /// The fire damage a grounded body standing on it takes, shifted by
    /// its weakness to fire, as the panel turns normal (EXE5's lava,
    /// 0x08016D80 and 0x08016E18).
    pub burn: Option<u16>,
    /// The element of the bodies it drains as poison drains any (EXE5's
    /// sea: fire, 0x08016C7E).
    pub drains: Option<u8>,
    /// Ticks a body that ends a move on it is held there, with a splash
    /// (EXE5's sea, 0x0801715E).
    pub holds: Option<u16>,
    /// A body that can dive (its AI's flag 0x20) is submerged while on it,
    /// and no body is submerged off it (EXE5's sea, 0x08017030).
    pub submerges: bool,
    /// A move's end on it starts a slide (slide type 3), tried in turn by
    /// the direction of the move (EXE5's metal, 0x08017216, 0x0800C8A8).
    pub slide: Option<PanelSlide>,
    /// The element of the hitboxes that turn it normal as they pass over
    /// it (`sub_3007708`: fire grass, aqua the volcano, wood roads; EXE5's
    /// 0x08016D14: and aqua lava, wood metal).
    pub cleared_by: Option<u8>,
    /// Whether the game's own section names the type; one it doesn't is
    /// the first other loaded game's that does (docs/design/rules-in-luau.md
    /// §7.4).
    pub named: bool,
}

/// A panel's slide (EXE5's metal): by the direction the body last moved
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

