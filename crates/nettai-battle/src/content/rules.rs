//! The rules' tables: data no single entity owns.

use super::{BannerId, ChipFamily, CustomScreenLayout, PanelCondition, SecondaryElements};
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
    /// simulation doesn't read it): the intro fades in from black on a set's
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
    /// The intro's first tick runs its first step too (the HUD's setup):
    /// EXE4's intro (0x08007464) goes on from its init on the same tick,
    /// where EXE6's (`sub_80091F0`) and EXE5's return after it.
    pub intro_steps_on_init: bool,
    /// A turn starts with the transformation sequencer (EXE6's fighting
    /// state 0, `sub_800840C`, twice, which EXE5 has too): the turn's
    /// banner a tick after it's through. EXE4 has none: its fighting state
    /// 0 (0x08007064) is the turn's banner, from the turn's first tick.
    pub sequencer_at_turn_start: bool,
    /// How a player asks for the custom screen with a full gauge.
    pub custom_request: CustomRequest,
}

/// A link battle's navi body damage (the rules' `status.link_body_damage`):
/// the navi's damage as it is set up (`sub_80142B0`, EXE6's and EXE5's
/// 0x080119AC: 10), which its body's collision takes, and whether its
/// status reset sets the collision's again (`sub_80142C2`: EXE6's; EXE5's
/// reset, 0x08011B3C, has none).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkBodyDamage {
    pub damage: u16,
    pub again_at_reset: bool,
}

/// How a player asks for the custom screen with a full gauge, L or R.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustomRequest {
    /// The navi's input decode marks the request (EXE6's `sub_8012FC8`
    /// sets battle flag 0x10), which the fight reads on the next tick
    /// (`sub_800A1D0`), and the navis' reversions run before the screen.
    NaviInput,
    /// The fight reads both players' keys itself (EXE4's 0x08007A2E: L or
    /// R pressed, the gauge full, not dimmed, the battle not over, not the
    /// late turns) and asks for the screen on that tick (0x0800718E: no
    /// reversions); the navi's decode doesn't.
    Joypads,
}

/// The rule section `link_pick`: what a link battle picks at random with
/// its settings (EXE6's `sub_81209DC`, EXE5's 0x08129F2C: two numbers a
/// round, the first RNG's for the stage and the second's for the
/// background). For whoever makes a random match: the engine picks none (a
/// round's settings state its stage and background). Every game's rules
/// state it; rules whose `stages` is empty have no random pick.
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
    /// The stages a match may name: the game's link battle stages, whether
    /// a random pick reaches them or not (EXE6's and EXE5's: each a
    /// settings record with the link effect that isn't the random
    /// battle's), in handle order. (The section states stages, which
    /// `sections::link` resolves.)
    pub match_stages: Vec<nettai_content_api::StageHandle>,
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
    /// None (EXE4's 0x08007252 shows the navi's, 0x08008534, under event
    /// flag 0x1187 alone, which its main subsystem 0x080406B0 sets and a
    /// netbattle's doesn't): every link battle's win shows the roles'.
    Never,
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
    /// The opponent's NaviCust support Beat lets a dark chip go: EXE4's
    /// (0x0800C86C tests the record's flag 0x20) takes a Mega or Giga chip
    /// only if it isn't dark; EXE6's `sub_80106C0` and EXE5's take any.
    pub beat_spares_dark: bool,
    /// A dimming's start hides the chip window on every console: EXE4's
    /// (0x08008BD8 calls `sub_801DACC(0x10)` whoever started it); EXE6's
    /// and EXE5's `object_timefreezeBegin` only on its starter's
    /// (`battle_networkInvert`). Presentation: the HUD's (`ChipHud`).
    pub dimming_hides_every_window: bool,
    /// A charged use of a Null-family chip runs the form's alternative
    /// A-charge routine, the attack's chip cleared (EXE6's `sub_800FB54`:
    /// the Beast forms'); EXE5's (0x0800D9CC) and EXE4's (0x0800B72C) use
    /// any charged chip with its charge, Null or not.
    pub charged_null_alt_routine: bool,
}

/// The rule section `effects` (docs/design/exe5-map.md §15.3 items 15 and
/// 16), the arena's: the battle's shared effects where games' touch the
/// simulation differently. A game states every one.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize)]
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
    /// How an object's damage word decodes (`sub_8019F44`).
    pub damage_word: DamageWord,
    /// What holds a screen palette flash (effect object #0x0A,
    /// `kinds::palette_flash`) by its mode.
    pub palette_flash: PaletteFlashRule,
    /// Where that flash sits among the palette transforms a frame
    /// applies, which a dimming's fade and a background's palette shift
    /// are among. Presentation: the renderer's (`Stage::new`'s `flashed`);
    /// the simulation reads none of it.
    pub palette_flash_order: PaletteFlashOrder,
    /// An afterimage (`sub_80E33FA`) and a form overlay (`sub_80C4530`'s
    /// spawner) run while the battle is paused: EXE6's spawners set their
    /// header flag 0x04; EXE5's (0x080E35F4, and its overlays', whose flags
    /// its lab records without it) don't.
    pub overlays_run_while_paused: bool,
    /// How a form overlay (actor object #0x57, `kinds::form_overlay`)
    /// starts and follows its owner.
    pub form_overlay: FormOverlayRules,
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
    /// A sprite updated through the dimming (`object_updateSpriteTimestop`)
    /// steps on the tick it loads a newly requested animation (EXE6's
    /// `sub_801BBF4`, EXE5's); EXE4's (0x08014446) only loads it, stepping
    /// from the next tick (its `object_updateSprite`, 0x080143FC, steps as
    /// EXE6's does).
    pub dimmed_update_steps_on_load: bool,
    /// An immobilized navi blinks black (its tail's `sub_801690A`, EXE5's
    /// 0x08013768: shader 0xFFFF two ticks of the battle time in four);
    /// EXE4's tail (0x08013BEC) has no such step. Presentation.
    pub immobilized_blinks: bool,
    /// How the game's obstacles number their action tables.
    pub obstacle_actions: ObstacleActions,
    /// The Full Synchro aura where games differ.
    pub full_synchro_aura: AuraRules,
    /// When a navi's charge glow comes.
    pub charge_glow: ChargeGlow,
    /// How a navi's buttons charge, and ask for the buster, the charged
    /// shot and chips.
    pub charge: ChargeControls,
    /// How a navi's held direction keys pick its step.
    pub steps: StepControls,
    /// When a screen fade toward clear ends (`Fade::step`).
    pub fade_clear: FadeClear,
    /// A banner's steps (`hud::Banner`).
    pub banner: BannerSteps,
    /// Each console's RNG1 advances once a frame, after the battle's (EXE6's
    /// main loop, `main_`'s `GetRNG1` after the subsystem; EXE5's). EXE4's
    /// main loop (0x080002B0) draws none: RNG1 moves only where the battle
    /// draws it.
    pub rng1_per_frame: bool,
    /// Where and which chip icons the HUD stacks over a navi.
    /// Presentation: the renderer's.
    pub chip_icons: ChipIcons,
    /// The ticks the other player's console names a chip a player used,
    /// the tick it starts on counted (`Battle::used_chip_for`: EXE6's
    /// `sub_801EB18`, a second, 0x3C; EXE5's; EXE4's 0x080164B4, a banner
    /// of the second block that shows without sliding, 33).
    pub used_chip_ticks: u8,
    /// A dimming chip's telop, as it starts, ends the used chips' names on
    /// every console (EXE6's `sub_800BA8A` and `sub_800BBA8`:
    /// `sub_801BED6(0x10000)`, `sub_801DACC(0x10000)`; EXE5's 0x0800A0FC
    /// and 0x0800A218). EXE4's (0x08008CF6, 0x08008DE0) lays the telop on
    /// the banner block and leaves the second block's name be, beside it.
    /// Presentation: the HUD's (`Battle::used_chip_for`).
    pub telop_ends_used_chips: bool,
    /// Each console's emotion window checks its navi's NaviCust bugs and
    /// flickers a bugged navi's face, an RNG1 draw a flicker (EXE6's
    /// `sub_801CC94`, EXE5's 0x08019780). EXE4's has no such check: its
    /// RNG1 never moves in a bugged navi's fight.
    pub bug_flicker: bool,
    /// When a player's step leaves its stats' panel trail on the panel it
    /// leaves (`bugs.panel_trail_kind`).
    pub panel_trail: PanelTrail,
    /// The status visual over a navi (effect object #6): where it sits
    /// and whether it casts a shadow.
    pub status_visual: StatusVisualRules,
    /// The order the objects of a pool are drawn in. Presentation: the
    /// renderer's.
    pub draw_order: DrawOrder,
}

/// The order a pool's objects are drawn in, each pool in turn (actors,
/// attacks, effects): it decides which of two parts in the same depth
/// bucket is in front (the one drawn later).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DrawOrder {
    /// By the update list (EXE6's `sub_8003E18`, `sub_8004218` and
    /// `sub_8004510` walk the lists `RunBattleObjectLogic` builds; EXE5's).
    UpdateList,
    /// By slot (EXE4's 0x08003BA0, 0x08003ED4 and 0x08004180 walk each
    /// pool's slots from the first).
    Slots,
}

/// The chip icons the HUD stacks over a navi that holds chips (the
/// renderer's `icon_parts`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChipIcons {
    /// Over its sprite's attach point 3, the next chip's icon once for
    /// every chip held (six at most), each next one two pixels up and
    /// two away from where the console faces, in front of the field's
    /// objects (EXE6's `sub_801C082`, EXE5's).
    AttachPoint,
    /// At its own offset from its place on the screen (its identity's
    /// `chip_icons_at`: EXE4's table 0x0800B9E4, by navi number, which
    /// 0x08015B24 keeps), each chip it holds from the next one on by its
    /// own icon, each next one two pixels up and two left, at priority 1
    /// in depth buckets from the icons' count down (EXE4's 0x08014860
    /// and 0x08015000: the local navi's alone).
    NaviOffset,
}

/// A banner's steps, in ticks (`hud::Banner::tick`): it slides in, holds,
/// slides out; and how a banner that holds until let go is let go. Sliding
/// in, it unsquashes in a line from its first tick to its last, and
/// squashes so sliding out (the frontend's: EXE6's `sub_801CE28` by 0x20 a
/// tick over 5, EXE4's 0x08014994 by 0x10 over 9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BannerSteps {
    pub slide_in: u8,
    pub hold: u8,
    pub slide_out: u8,
    pub release: BannerRelease,
    /// It bounces as its hold starts and as it ends, squashed a little
    /// for two ticks each (EXE6's `sub_801CE28`; EXE4's holds still).
    pub bounces: bool,
}

/// How a holding banner is let go (`hud::Banner::release`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BannerRelease {
    /// Its timer stops at 5 while it holds; let go, it holds three ticks
    /// more, then slides out (EXE6's `sub_801CE28`, `sub_801E780`).
    HoldsThreeMore,
    /// Its hold doesn't count while it holds; let go, it slides out at once
    /// (EXE4's 0x080149CE, 0x0801616C).
    SlidesOut,
}

/// When a screen fade toward clear (the intro's, a dimming's end) ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FadeClear {
    /// On the step whose level reaches its target, kept at least 0 (EXE6's
    /// `sub_8006366`, EXE5's).
    AtTarget,
    /// On the step whose level would go under its target, which isn't kept:
    /// one step later (EXE4's 0x08005BDE; its intro's fade from white
    /// takes 18 steps, EXE6's 17).
    PastTarget,
}

/// What a navi does while the battle is paused (`Rules::paused_navi`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PausedNavi {
    /// It runs through the pause (its header flag 0x04 stays), and its
    /// status block's tail runs its pause handler in place of its action
    /// (but in its entry): the form changes and reversions, the navi
    /// switches (EXE6's `loc_801B142`, EXE5's).
    PauseHandler,
    /// It runs through the pause until it takes control, which clears its
    /// header flag 0x04 (EXE4's 0x08010A88), so that the object loop skips
    /// it then; its status block has no pause test (0x08013A48: the top
    /// block runs whenever the navi does, its take-control tick too) and
    /// its tail no pause handler, running its action (0x08013C2A).
    StopsAtControl,
}

/// How a navi's buttons charge, and ask for the buster, the charged shot and
/// chips (`EffectsRules::charge`; `kinds::player::input`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChargeControls {
    /// EXE6's (`sub_8012FC8`, `sub_8012EBC`; EXE5's): the held button
    /// raises a hold request (A first, then B; pressing the other switches),
    /// its charge counts to a full charge and stays there; the buster fires
    /// on B's release (its press without a charged shot); L and R turn, or
    /// with a full gauge ask for the custom screen; B then back (by the
    /// navi's facing) with B held asks for the B+Back special.
    HoldFlags,
    /// EXE4's (0x0800BDE0, 0x0800BBA4, 0x0800BB50): no hold requests; B
    /// held charges B, A held with a chip in hand charges A (counting while
    /// the chip charges), the other's press switching, and a count goes on
    /// past a full charge (to 510); the buster fires on B's release, unless
    /// a buster or charged shot is asked already; the navi's buttons
    /// neither turn it nor ask for the custom screen; B then Left
    /// (whichever way it faces) within 8 ticks asks for the B+Left special.
    PerButton,
}

/// How a navi's held direction keys pick its step (`EffectsRules::steps`;
/// `kinds::player::idle::held_direction`, the idle's step).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepControls {
    /// The keys read, first held first (EXE6's `sub_800FA54`: up, down,
    /// right, left; EXE4's 0x0800B4B0: right, left, up, down). Right is
    /// toward the other side, left away, on either side's console.
    pub keys: Vec<StepKey>,
    /// What each key steps toward while the navi is confused (EXE6's
    /// `byte_800FAA4`: up and down swapped, right and left; EXE4's
    /// 0x0800B550 the same).
    pub confused: ConfusedKeys,
    /// The idle starts a step only toward a panel the navi may step to
    /// (EXE4's idle, 0x080EEC82: 0x0800B4B0 tests the panel); else (EXE6's
    /// `sub_80F0354`) a held direction starts the step, which, blocked,
    /// leaves for idle at once (its phase from the start).
    pub idle_checks_target: bool,
    /// A move bug: a stat of the rules' (`stats`) that the NaviCust's and
    /// the patch cards' bugs write, and what it does (EXE4's NaviStats
    /// +0x0D). EXE6's and EXE5's have none (theirs, the processing bug,
    /// is the engine's own).
    #[serde(default)]
    pub bug: Option<StepBug>,
}

/// A move bug (`StepControls::bug`): the rules' stat `stat` (a `u8`). At
/// 0xFF the navi is confused for `confused` ticks as the round starts
/// (EXE4's init, 0x0800D8B0); else its high nibble, in the keys' bits
/// (0x10 right, 0x20 left, 0x40 up, 0x80 down), is held while the navi
/// holds no direction key or slides, in the keys' order and never turned
/// by a confusion (0x0800B4DA): the navi steps on its own.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepBug {
    pub stat: String,
    pub confused: u16,
}

/// A direction key (`StepControls`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepKey {
    Up,
    Down,
    Left,
    Right,
}

/// A confused navi's step for each key (`StepControls::confused`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfusedKeys {
    pub up: StepKey,
    pub down: StepKey,
    pub left: StepKey,
    pub right: StepKey,
}

/// When a navi's charge glow (effect #8, `kinds::charge_glow`) comes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChargeGlow {
    /// With the navi: its init spawns it, and it lives as long (EXE6's
    /// `sub_80E0F02`, EXE5's).
    WithNavi,
    /// With a charge: the navi's init spawns none (EXE4's 0x0801079C); the
    /// charge brings its own glow (0x0800BD88's effect 5, which goes when
    /// the charge does).
    WithCharge,
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
/// `sub_801A082`'s call of `sub_8019F44`) decodes an object's damage word:
/// the damage is its bits under `damage` (EXE6's and EXE5's low 11, 0x7FF;
/// EXE4's low 14, 0x3FFF: 0x08012860), doubled with 0x8000; then its flag
/// bits, read in the order the game's routine reads them (`flags`). EXE6's:
/// 0x4000 a paralysis with hit modifier 1, then 0x2000 bug code 0xF8 (and
/// no more), else 0x1000 bug code 0xF7, each code's high byte the target
/// lookup's row offset (what the caller left in r1). EXE5's (0x080165EC):
/// 0x4000 a paralysis with hit modifier 0 (no flinch), and no more; else
/// 0x2000 a confusion, and no more; else 0x1000 a blindness; then 0x800 bug
/// code 0x18 with high byte 0x11. EXE4's: 0x4000 its status 0x12 with hit
/// modifier 0, nothing else.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageWord {
    pub damage: u16,
    pub flags: Vec<DamageFlag>,
}

/// A flag bit of a damage word (`DamageWord::flags`): what its hits carry
/// when the word has it, and whether the reading stops there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageFlag {
    pub bit: u16,
    /// The status its hits carry (the role naming it, `statuses`).
    #[serde(default)]
    pub status: Option<super::StatusRole>,
    /// The hit modifier it sets (1 a flinch, 0 none).
    #[serde(default)]
    pub hit_modifier: Option<u8>,
    /// The bug code its hits carry; its high byte, where `bug_high_row`,
    /// the target lookup's row offset (EXE6's: what the lookup left in r1).
    #[serde(default)]
    pub bug: Option<u16>,
    #[serde(default)]
    pub bug_high_row: bool,
    /// The reading ends at this bit when the word has it.
    #[serde(default)]
    pub stop: bool,
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
    /// EXE4's takes slot 0 (0x080E2A34; its two-layer one 0 and 1,
    /// 0x080E2AAC), before a background's palette shift too (slot 3,
    /// darksoul's: SparkMan's flash shows its white shifted); EXE5's
    /// backgrounds shift no palettes.
    BeforeFades,
    /// After the first record's: EXE6's takes slot 20 (`sub_80E10C0`; its
    /// two-layer one 20 and 21, `sub_80E114C`), so its white stands over a
    /// dimming (Colonel's, DeltaRay's, CrossDiv's, the navi advances').
    AfterFades,
}

/// How a form overlay (actor object #0x57: EXE6's `sub_80C4530`, EXE5's
/// 0x080C379C, EXE4's 0x080CC3D8) starts, and what it takes from its owner
/// each tick besides its position, visibility, color shader, white flash
/// and mosaic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormOverlayRules {
    /// How it starts following its owner.
    pub start: OverlayStart,
    /// Each tick it restarts its sprite on its owner's animation (plus its
    /// offset) when that changed (EXE6's `sub_80C458C`); else it only
    /// records the animation, which its sprite's update loads.
    pub reloads_animation: bool,
    /// Each tick it takes its palette again (EXE6's `sub_80C46CC`); else it
    /// keeps the one its init took.
    pub palette_each_tick: bool,
    /// Each tick it takes its owner's facing (EXE6's `sub_80C458C`); else
    /// it keeps the one its init took, its own side's.
    pub facing_each_tick: bool,
    /// While it waits for the navis it steps its sprite as it would
    /// following (EXE4's 0x080CC4D4); else its sprite holds.
    pub waiting_steps: bool,
}

/// How a form overlay starts following its owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayStart {
    /// It waits (its action 0) until every navi is in, then follows (its
    /// action 4), stepping its sprite at once (EXE6's `sub_80C461C`).
    AfterNavisIn,
    /// With the fight on, it follows (its action 8) from its first update.
    /// Otherwise it runs while paused and waits (its action 0) until every
    /// navi is in, then stops running while paused and follows. (EXE4's
    /// 0x080CC440, whose action 4, a random battle's intro, comes instead of
    /// the wait in a random battle, which no link battle is.)
    AtOnceInFight,
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

/// The speed of a navi's slides (ice, roads, EXE5's magnet: `sub_8016730`)
/// and drags (a push: `sub_80178D4`), 16.16 pixels a tick across and in
/// depth (EXE6's 10 pixels across and 6 in depth; EXE5's 10 and 8,
/// 0x0801361E and 0x080143A8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlideSpeed {
    pub x: i32,
    pub y: i32,
}

/// How a move's direction goes into the collision record
/// (`object_updateCollisionPanels`: the reactions section's
/// `move_direction`), which an ice slide or push, EXE5's magnet slide and
/// content reading the record's direction read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveDirection {
    /// EXE6's `sub_800E994` (EXE5's 0x0800CC92, the same code): 0 none, 1
    /// up, 2 down, 3 back and 4 forward by the side, 5 other. Only a move
    /// of two panels or more right or down is other (the routine tests
    /// `>= 2` and nothing below -1), and a diagonal one: any move left or up
    /// along one axis counts by its sign, so a navi warped two panels back
    /// (side 0) or forward (side 1) has moved back or forward, and slides on
    /// ice.
    BySide,
    /// EXE4's 0x0800AF90: 0 none, 1 up, 2 down, 3 left and 4 right whatever
    /// the side, across before up and down (a diagonal move by its x), and no
    /// other.
    Absolute,
}

impl MoveDirection {
    /// The direction of a move from `old` to `new` by a body of side
    /// `alliance`.
    pub fn of(self, old: crate::object::PanelPos, new: crate::object::PanelPos, alliance: u8) -> u8 {
        let dx = new.x as i8 - old.x as i8;
        let dy = new.y as i8 - old.y as i8;
        match self {
            MoveDirection::BySide => {
                if dx >= 2 || dy >= 2 {
                    return 5;
                }
                let (back, forward) = if alliance == 0 { (3, 4) } else { (4, 3) };
                match (dx.signum(), dy.signum()) {
                    (0, 0) => 0,
                    (0, -1) => 1,
                    (0, 1) => 2,
                    (-1, 0) => back,
                    (1, 0) => forward,
                    _ => 5,
                }
            }
            MoveDirection::Absolute => match (dx.signum(), dy.signum()) {
                (1, _) => 4,
                (-1, _) => 3,
                (_, 1) => 2,
                (_, -1) => 1,
                _ => 0,
            },
        }
    }
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
    /// The game's own stats (its rules' `stats`), fresh: the section's
    /// values of them by name, the rest zero (EXE6's Beast Out turns, 3).
    pub stats: nettai_content_api::SmallBlock,
    /// +0x44: the weapon of the A button in battle mode 9 (EXE6's zeroed
    /// byte names weapon routine 0, MegaMan's buster). None stated: none
    /// (EXE5's block has the light/dark value there, its light and dark
    /// part's).
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
/// anti-sword trigger, the mode-9 A press, the rules' own B charge's release
/// (EXE5's Chaos Unison releases).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RequestSet(pub u32);

impl RequestSet {
    pub const NAMES: &'static [(u32, &'static str)] = &[
        (crate::actor::request::ANTI_SWORD_TRIGGERED, "anti_sword_triggered"),
        (crate::actor::request::MODE9_A, "mode9_a"),
        (crate::actor::request::RULES_RELEASE, "rules_release"),
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

/// Collision flags a status end clears, in a content file a list of their
/// names (`StatusFlag`'s): the statuses'.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct StatusFlags(pub u32);

impl StatusFlags {
    pub const NAMES: &'static [(u32, &'static str)] = &[
        (crate::collision::f1::PARALYZED, "paralyzed"),
        (crate::collision::f1::BLIND, "blind"),
        (crate::collision::f1::IMMOBILIZED, "immobilized"),
        (crate::collision::f1::CONFUSED, "confused"),
        (crate::collision::f1::FROZEN, "frozen"),
        (crate::collision::f1::BUBBLED, "bubbled"),
    ];
}

serde_flags!(StatusFlags, u32);

/// Status timers a status end zeroes, a bit each by the timer's index
/// (`collision::timer`), in a content file a list of their names
/// (`StatusTimer`'s).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct StatusTimers(pub u16);

impl StatusTimers {
    pub const NAMES: &'static [(u32, &'static str)] = &[
        (1 << crate::collision::timer::PARALYZE, "paralyze"),
        (1 << crate::collision::timer::CONFUSE, "confuse"),
        (1 << crate::collision::timer::BLIND, "blind"),
        (1 << crate::collision::timer::IMMOBILIZE, "immobilize"),
        (1 << crate::collision::timer::FREEZE, "freeze"),
        (1 << crate::collision::timer::BUBBLE, "bubble"),
    ];
}

serde_flags!(StatusTimers, u16);

/// What a navi's statuses' end clears (EXE6's `sub_801A264`, EXE4's
/// 0x08013218): its statuses' flags, its requests (the collision's second
/// word: the statuses' own, the status table's `requests`) and its timers.
/// EXE6's: the six statuses' flags (0x8001E800), the requests 0x300E8 and
/// the six timers; EXE4's, a game with no freeze and no bubble, the four
/// others' (0xE800), the requests 0x8068 and the four timers (its flag
/// 0x10000 holds a navi changing into a soul).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusEnd {
    pub flags: StatusFlags,
    pub requests: u32,
    pub timers: StatusTimers,
}

/// What a deleted player's object does in its destroy state
/// (`kinds::player`'s `destroy`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeadPlayer {
    /// EXE6's `sub_8016C4E` (EXE5's alike): its reservations and collision
    /// data let go and the side's actor count one less, once; the object
    /// kept in its slot (freed for an actor record that isn't counted).
    Kept,
    /// EXE4's 0x0801052C: its collision data let go, the object freed at
    /// once and the side's actor count one less; its reservations as they
    /// are.
    Freed,
}

/// When the counter a stance's caught hit starts (`sub_80105F2`), or a
/// trap's (`sub_801056A`), runs.
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
/// and its push on any hit): from which hit modifier byte, the first of how
/// many of its bits from bit 2 (none set: the row past them), and an
/// obstacle's rows. EXE6's (`sub_800E548`, `sub_800F598`): the final
/// modifier's bits 2 to 5, 0x80 moving a navi's pick five rows on, an
/// obstacle's four rows (`byte_800F604`; none set reads past them, from the
/// BIOS). EXE5's (0x0800C9D8; an obstacle's 0x0800D4B0 and 0x08017AD8): the
/// unflipped hitters' bits 2 to 5, else the flipped ones' with the
/// direction reversed, an obstacle's five rows. EXE4's (0x0800ACAA,
/// 0x0800B1C0): the final modifier's bits 2 to 7, a navi's and an
/// obstacle's six rows the same (back, forward, a panel back, a panel
/// forward, up, down), then none.
///
/// A hit's push (one of those bits set) is a drag, rather than a slide,
/// with the modifier's `drag_bit` set too (the hit intake, EXE6's
/// `sub_801AEB0`: 0x40; EXE4's, 0x08013858: 0x01, the flinch bit), and an
/// obstacle's push is that (EXE6's `sub_801AD9E` tests the bit alone, which
/// no hit has without a push; EXE4's 0x0801393E both). Whatever starts a
/// navi's slide or drag, a game whose pushes are its only slides reads the
/// push (EXE4's 0x08010294 and 0x08010ABC call its reading straight): the
/// engine's slide type 1, which the intake sets with the request.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PushReading {
    pub reads: PushSource,
    pub bits: u8,
    pub drag_bit: u8,
    #[serde(default)]
    pub shift: Option<PushShift>,
    /// An obstacle's rows, turned toward the pusher's side.
    pub obstacle_rows: Vec<SlideVector>,
    /// An obstacle's intake that keeps a push's damage takes the end of
    /// its moves as a navi's does: MOVE_COMPLETE consumed, then the panel
    /// under it has its say (its type's `move_end`: EXE4's 0x0801393E
    /// calls 0x0801335A, its ice's push). EXE6's `sub_801AD12` and EXE5's
    /// don't.
    #[serde(default)]
    pub obstacle_move_end: bool,
}

/// Which hit modifier a push reads (`PushReading::reads`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PushSource {
    /// The hits' final modifier, toward the navi's front.
    Final,
    /// The unflipped hitters' modifier, else the flipped ones' with the
    /// direction reversed (`CollisionData::hit_mod_by_side`).
    ByHitterFlip,
}

/// A modifier bit that moves a navi's pick of a push row `rows` on
/// (`PushReading::shift`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PushShift {
    pub bit: u8,
    pub rows: u8,
}

impl PushReading {
    /// The row the first set bit of `modifier`'s `bits` from bit 2 picks;
    /// none set: the row past them.
    pub fn first(&self, modifier: u8) -> usize {
        (0..self.bits as usize).find(|&i| (modifier >> 2) & (1 << i) != 0).unwrap_or(self.bits as usize)
    }

    /// The modifier's push bits: `bits` of them from bit 2 (EXE6's 0x3C,
    /// EXE4's 0xFC).
    pub fn mask(&self) -> u8 {
        (((1u16 << self.bits) - 1) << 2) as u8
    }

    /// Whether `modifier` pushes as a drag: a push bit and the drag bit.
    pub fn drags(&self, modifier: u8) -> bool {
        modifier & self.mask() != 0 && modifier & self.drag_bit != 0
    }
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
    /// An untouchable receiver (flag 0x08000000) is tested with the
    /// invulnerable one, after its guard and the air/ground test, so its
    /// guard still turns hits aside (EXE4's 0x08012BDC: 0x08000008); else
    /// with the other states before the guard (EXE6's and EXE5's).
    pub guard_before_untouchable: bool,
    /// A hit a guard turns aside marks the receiver's guard byte with the
    /// hitter's direction (1 << its flip: EXE6's, EXE5's); else with 1,
    /// whatever the direction (EXE4's 0x08012BA6).
    pub guard_marks_direction: bool,
}

/// How often a NaviCust bug drains a point of HP (the status section's
/// `hp_drain` and `custom_drain`): a period by the bug's stat (its level,
/// 0 none), or the stat itself the period. In a content file, a list of
/// eight periods or `"stat"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(untagged)]
pub enum DrainPeriods {
    /// By level (EXE6's `byte_80102A4` and `byte_80102F8`, EXE5's the same).
    ByLevel([u8; 8]),
    /// The stat is the period (EXE4's 0x0800C164 and 0x0800C194).
    Stat(StatIsPeriod),
}

/// `"stat"`: the bug's stat is its drain's period (`DrainPeriods::Stat`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatIsPeriod {
    Stat,
}

impl DrainPeriods {
    /// The period of a bug whose stat is `stat` (0: no drain).
    pub fn period(&self, stat: u8) -> u8 {
        match self {
            DrainPeriods::ByLevel(table) => *table.get(stat as usize).expect("a drain bug's level"),
            DrainPeriods::Stat(_) => stat,
        }
    }
}

/// The NaviCust's HP bug (`sub_8010230`, EXE4's 0x0800C164): a point of HP
/// every period, never below 1, while the battle isn't dimmed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HpDrainRule {
    pub periods: DrainPeriods,
    /// It waits while the battle is paused (EXE6's and EXE5's); EXE4's
    /// drains through a pause.
    pub stops_while_paused: bool,
}

/// The custom screen's HP bug (`sub_80102AC`, EXE5's 0x0800E034, EXE4's
/// 0x0800C194): a point of HP every period while the side's status (the
/// link's status byte, BattleState +0x14) has one of `status`'s bits: bit
/// 2 its screen is up, bit 0 its selection runs (EXE6's and EXE5's 5,
/// EXE4's 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomDrainRule {
    pub periods: DrainPeriods,
    pub status: u8,
}

/// How a navi takes a hit's NaviCust bug, where games differ (`sub_801AC6C`,
/// `sub_80139F6`): the status section's, each stated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IntakeRules {
    /// The bug is taken before the HP bug drains (EXE5's hit intake,
    /// 0x080178EC, calls 0x0801103E before 0x0800DFEC); EXE6's after, so a
    /// drain bug's first drain comes a tick later.
    pub bugs_before_drain: bool,
    /// EXE5's no-charge drive (DarkInvs, 0x080E2318): a navi with the
    /// no-charge state counts its drive's ticks down at the intake's end
    /// (0x0800DBE0) and asks for the stun strike when they run out (EXE5's
    /// action 0x49 ends the drive); its idle hands the step it would take
    /// to the side's rules' `controller` (0x080F03E4: the auto battle
    /// AI, 0x0802B4AC, or the reset of its state); and its last 180 ticks
    /// it flickers gray (0x080136E0). EXE6 has none of it.
    pub no_charge_drive: bool,
    /// How a navi loses HP (`object_subtractHP`, `applyDamageToPlayer`).
    pub hp_loss: HpLoss,
    /// What a navi's hit sounds like, on each console.
    pub hit_sound: HitSound,
    /// How a navi's barrier takes the tick's hits (`sub_801A802`).
    pub barrier: BarrierTick,
    /// What a side's defensive chip catches (`sub_802CEF4`).
    pub anti_traps: AntiTraps,
}

/// What a side's defensive chip catches (`IntakeRules::anti_traps`: EXE6's
/// `sub_802CEF4`, EXE5's 0x08029A60, EXE4's 0x08023048).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AntiTraps {
    /// AntiDmg catches a hit of at least this much damage (EXE6's 10;
    /// EXE5's and EXE4's any, 1: they test the elements' damage for none).
    pub damage_min: u16,
    /// With the trap the navi's own status arms, a hit under the least is
    /// swallowed all the same (EXE6's; EXE5's arms AntiDmg's own test).
    pub armed_swallows_below: bool,
    /// A sword's hit (hit flags 0x2000) with any of these flags isn't
    /// AntiSwrd's (EXE6's 0x20000; EXE5's and EXE4's none).
    pub sword_spares: u32,
}

/// How a navi's barrier takes the tick's hits (`IntakeRules::barrier`:
/// EXE6's `sub_801A802`, EXE5's, EXE4's 0x08012DF8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BarrierTick {
    /// It stands still while the battle is paused (EXE6's, EXE5's). EXE4's
    /// runs whenever its navi does: in the intro's pause, until the navi
    /// takes control (`status.paused_navi`), its timer counts.
    pub stops_while_paused: bool,
    /// What wind does to it.
    pub wind: BarrierWind,
    /// A `regrowing` barrier (EXE4's type 4): worn down, it stays up with
    /// no HP, letting hits through, and after this many ticks (counted
    /// while not dimmed) it is back with this much HP; wind leaves it be
    /// meanwhile, and it never times out (EXE4's 0x08012E44: 180 ticks,
    /// 150 HP). None in a game that has none.
    #[serde(default)]
    pub regrowing: Option<BarrierRegrowth>,
}

/// How a regrowing barrier comes back (`BarrierTick::regrowing`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BarrierRegrowth {
    /// The ticks it is down.
    pub after: u16,
    /// Its HP when back.
    pub hp: u8,
}

/// What wind does to a barrier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BarrierWind {
    /// Pops it (EXE6's, EXE5's), wind being the raw elements' 0x20 or the
    /// raw hit flags' 0xA20: popped, it absorbs every hit until its visual
    /// clears it.
    Pops,
    /// Takes it away at once (EXE4's 0x08012E0C: no popped barrier), wind
    /// being the raw hit flags' 0x20 (its raw channel, 0x08012D1E, keeps
    /// the hitters' self words and no elements); the hit is absorbed all
    /// the same.
    TakesAway,
}

/// What a navi's hit sounds like, on each console (`IntakeRules::hit_sound`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HitSound {
    /// A player hears the role `own_hit` when their own navi is hit, `hit`
    /// when another is (EXE6's `applyDamageToPlayer_801ba12`, EXE5's).
    ByConsole,
    /// Every console hears the role `hit`, or `auto_battle_hit` for a navi
    /// in auto battle (EXE4's hit intake, 0x08013A8C: sound 0x6D where the
    /// navi's NaviStats +0x26 is 1, else 0x6B).
    ByNavi,
}

/// How a navi loses HP (`IntakeRules::hp_loss`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HpLoss {
    /// The HP alone goes (EXE6's `object_subtractHP`), and a hit sounds
    /// before the HP left is looked at (`applyDamageToPlayer_801ba12`).
    HpAlone,
    /// A player's loss also drains its side's gauge (EXE5's 0x0800C6E0), a
    /// loss that brings a navi to 0 asks the side's rules (`hp_emptied`:
    /// EXE5's last stand, 0x0802C16C), and a hit shows (white, its sounds)
    /// only by their answer there (0x080185A2); one that doesn't goes
    /// straight to the deletion's test, without the element-5 damage.
    Gauge,
    /// EXE4's: a loss takes the HP alone (`object_subtractHP`, 0x0800AB92),
    /// but a player's hit (its status block's final damage, 0x08013A48)
    /// drains its side's gauge too (0x0800AB9E: by the loss ×128) and shows
    /// (white, its sound) only with HP left; at 0 HP, from the hit or the
    /// element-5 damage, the side's rules are asked after (`hp_emptied`:
    /// 0x0800EBC8, EXE4's dark MegaMan's last stand, which holds the navi at
    /// 1 HP itself), and the navi falls unless they keep it.
    HitDrainsGauge,
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
    MarkedForms,
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

/// A side's emotions, each game's own (the status section's `emotion`):
/// their names, in the order the game reads them off its navi (the first
/// case that holds is the side's emotion: EXE6's `sub_8015B54`, EXE5's
/// 0x0801270C), over facts of the framework's ([`EmotionWhen`]); what each
/// is to the framework ([`EmotionRole`]); what holds a mood and how anger
/// leaves it. (A derivation, read on each read: nothing is kept in step,
/// rules-in-luau.md §4.6.)
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(try_from = "EmotionSection")]
pub struct EmotionRules {
    /// What holds a side's mood against the setter (`sub_8015BEC`, EXE5's
    /// 0x080127D6).
    pub mood_held: MoodHeld,
    /// How the end of anger leaves the mood (`sub_80143A6`, EXE5's
    /// 0x08011A94).
    pub anger_end: AngerEnd,
    /// The emotions' names, by [`Emotion`]: the order's last case's first
    /// (`Emotion(0)`, the default: what no other case's holds), then as the
    /// order first names them.
    pub names: Vec<String>,
    /// The order: each case's emotion, and when it holds (any of its
    /// alternatives; none: always, as the last case does).
    pub order: Vec<EmotionCase>,
    /// What each emotion is to the framework, by [`Emotion`].
    pub roles: Vec<Option<EmotionRole>>,
    /// What a hit's counter byte does to the moods.
    pub hit_mood: HitMood,
    /// The mood a Full Synchro boost leaves, through the setter (EXE6's and
    /// EXE5's 0x80; EXE4's 0x99, 0x0800D568).
    pub full_synchro_spent: u8,
    /// Anger's boost plays the boost's sound, as Full Synchro's does (EXE6's,
    /// EXE5's); else none (EXE4's 0x0800D57C ends the anger alone).
    pub anger_boost_sound: bool,
    /// A tick's damage that asks for anger, this much or more (EXE6's
    /// `sub_80142DC` and EXE5's 0x080119C4: half of it against 0x96, 300);
    /// none, only 120 ticks stunned do (EXE4's 0x0800C540).
    pub anger_damage: Option<u16>,
}

/// What a hit's counter byte (the hitter's collision +5's low bits) does
/// to the moods (`EmotionRules::hit_mood`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HitMood {
    /// The byte wears the receiver's mood; a counter hit (in the
    /// receiver's counter window) marks the counter (0x8000) and wears
    /// none, and the counterer's side's rules hear it (`countered`): EXE6's
    /// hit kernel and `sub_801A200`, EXE5's.
    CounterMark,
    /// The bytes the receiver takes in a tick raise the other side's mood
    /// (to 0xFF; a mood of 0 stays), a counter hit's counting 0xFF, and
    /// wear the receiver's (a counter hit's 0x7F): EXE4's hit kernel
    /// (0x08012C10) and status routine (0x080131E4). No `countered`.
    HitterGains,
}

/// The status visual over a navi (effect object #6, `sub_80E08FC`): the
/// confusion's stars and the blindness's mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusVisualRules {
    /// Where it sits on the navi, and the hit marker with it.
    pub place: StatusVisualPlace,
}

/// Where the status visual sits, and the hit marker (`sub_80E8124`'s
/// offset: EXE6's and EXE5's attach point 5, EXE4's actor record's +6 and
/// +7, 0x080133E8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusVisualPlace {
    /// At the navi's sprite attach point 5, hidden off the field (EXE6's
    /// `sub_80E0954`, EXE5's).
    AttachPoint,
    /// At its identity's `status_mark` from its position, x toward the
    /// enemy side, shown anywhere (EXE4's 0x080E235A: its actor record's
    /// +6 and +7, 0x08011878).
    StatusMark,
}

/// When a player's step leaves its panel trail (the rule section `effects`'
/// `panel_trail`), on the panel it steps off unless that is missing or
/// broken: kind 1 breaks it; any other turns it to the type of the game's
/// number, with the type's trail sound when its type changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelTrail {
    /// At a chance of the stats' level in 8 (an RNG2 draw), and kind 3
    /// cracks the panel (EXE6's `sub_8013CC4`, EXE5's).
    ByChance,
    /// Every step, unless the kind is 0xFF; kind 3 is a type set like any
    /// other (EXE4's 0x080EB264, which has no level: its sound for the
    /// crack tests the setter's return, the panel's object bits, which are
    /// never 3).
    Always,
}

/// A game's chip families (rule section `elements`: `families`, each name
/// with its number, and `non_elemental`): the numbers its chip records hold
/// and its pack's custom-screen icons are by (EXE6's own; EXE5's as its
/// pack has the icons, EXE6's numbers with its recovery and invisible
/// after them; EXE4's own, its record's +0x07).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ChipFamilies {
    /// Each family's name and number, by number.
    pub families: Vec<(String, ChipFamily)>,
    /// The family of a chip that names none, and the one the rules on
    /// non-elemental chips mean (EXE6's Beast and Cross forms: a Cross's
    /// erasing, a Beast's bonus and Beast Over's doubling of the family's
    /// damaging chips, the empty hand's family byte, the Beast forms'
    /// other A charge).
    pub non_elemental: ChipFamily,
}

impl ChipFamilies {
    /// The family named `name`.
    pub fn by_name(&self, name: &str) -> Option<ChipFamily> {
        self.families.iter().find(|(n, _)| n == name).map(|&(_, f)| f)
    }

    /// The families' names, by number.
    pub fn names(&self) -> Vec<&str> {
        self.families.iter().map(|(n, _)| n.as_str()).collect()
    }

    /// Family `family`'s name, if the game has it.
    pub fn name(&self, family: ChipFamily) -> Option<&str> {
        self.families.iter().find(|&&(_, f)| f == family).map(|(n, _)| n.as_str())
    }
}

/// A side's emotion: one of its game's ([`EmotionRules::names`]); the
/// default, `Emotion(0)`, the one when nothing else holds (EXE6's and
/// EXE5's normal).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Emotion(pub u8);

/// A case of the emotions' order.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EmotionCase {
    pub emotion: Emotion,
    pub when: Vec<EmotionWhen>,
}

/// An alternative of a case: facts of the framework's, each of which must
/// hold (those left out: whatever).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmotionWhen {
    /// The side's mood is this (0xFF: Full Synchro's), or under this.
    #[serde(default)]
    pub mood: Option<u8>,
    #[serde(default)]
    pub mood_below: Option<u8>,
    /// Its navi is angry (its anger's ticks run), held tired, exhausted
    /// (after Beast Over), or out of its base form.
    #[serde(default)]
    pub angry: Option<bool>,
    #[serde(default)]
    pub tired: Option<bool>,
    #[serde(default)]
    pub exhausted: Option<bool>,
    #[serde(default)]
    pub in_form: Option<bool>,
    /// The battle's mode is this.
    #[serde(default)]
    pub battle_mode: Option<u8>,
}

/// The facts the emotions' order reads of a side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmotionFacts {
    pub mood: u8,
    pub angry: bool,
    pub tired: bool,
    pub exhausted: bool,
    pub in_form: bool,
    pub battle_mode: u8,
}

impl EmotionWhen {
    fn holds(&self, f: &EmotionFacts) -> bool {
        let is = |want: Option<bool>, fact: bool| want.is_none_or(|w| w == fact);
        self.mood.is_none_or(|m| f.mood == m)
            && self.mood_below.is_none_or(|m| f.mood < m)
            && is(self.angry, f.angry)
            && is(self.tired, f.tired)
            && is(self.exhausted, f.exhausted)
            && is(self.in_form, f.in_form)
            && self.battle_mode.is_none_or(|m| f.battle_mode == m)
    }
}

/// What an emotion is to the framework's behaviors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmotionRole {
    /// Full Synchro: the next damaging chip doubles (and is spent), the
    /// navi takes the mood's palette and its aura, and the opponent's
    /// counter window flashes on a console that sees it.
    FullSynchro,
    /// Anger: the next damaging chip doubles.
    Angry,
    /// Worn out: the buster deals 1, and anger doesn't start.
    WornOut,
    /// Tired: anger doesn't start.
    Tired,
}

impl EmotionRules {
    /// The side's emotion for `facts`: the first case that holds.
    pub fn of(&self, facts: &EmotionFacts) -> Emotion {
        self.order.iter().find(|c| c.when.is_empty() || c.when.iter().any(|w| w.holds(facts))).map_or(self.fallback(), |c| c.emotion)
    }

    /// The emotion when nothing else holds (the order's last case's).
    pub fn fallback(&self) -> Emotion {
        Emotion(0)
    }

    pub fn name(&self, e: Emotion) -> &str {
        self.names.get(e.0 as usize).map_or("", String::as_str)
    }

    pub fn by_name(&self, name: &str) -> Option<Emotion> {
        self.names.iter().position(|n| n == name).map(|i| Emotion(i as u8))
    }

    pub fn role(&self, e: Emotion) -> Option<EmotionRole> {
        self.roles.get(e.0 as usize).copied().flatten()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EmotionSection {
    mood_held: MoodHeld,
    anger_end: AngerEnd,
    order: Vec<EmotionCaseSpec>,
    #[serde(default)]
    roles: std::collections::BTreeMap<String, EmotionRole>,
    hit_mood: HitMood,
    full_synchro_spent: u8,
    anger_boost_sound: bool,
    #[serde(default)]
    anger_damage: Option<u16>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EmotionCaseSpec {
    emotion: String,
    #[serde(default)]
    when: Vec<EmotionWhen>,
}

impl TryFrom<EmotionSection> for EmotionRules {
    type Error = String;

    fn try_from(s: EmotionSection) -> Result<EmotionRules, String> {
        let order = s.order.into_iter().map(|c| (c.emotion, c.when)).collect();
        let rules = EmotionRules::new(s.mood_held, s.anger_end, order, s.roles.into_iter().collect())?;
        Ok(EmotionRules {
            hit_mood: s.hit_mood,
            full_synchro_spent: s.full_synchro_spent,
            anger_boost_sound: s.anger_boost_sound,
            anger_damage: s.anger_damage,
            ..rules
        })
    }
}

impl EmotionRules {
    /// The rules of a section: its order (each case's emotion by name, and
    /// when it holds) and its roles, by name.
    pub fn new(
        mood_held: MoodHeld,
        anger_end: AngerEnd,
        cases: Vec<(String, Vec<EmotionWhen>)>,
        roles_by_name: Vec<(String, EmotionRole)>,
    ) -> Result<EmotionRules, String> {
        let mut names: Vec<String> = cases.last().map(|c| c.0.clone()).into_iter().collect();
        let mut order = Vec::new();
        for (emotion, when) in cases {
            let i = match names.iter().position(|n| *n == emotion) {
                Some(i) => i,
                None => {
                    names.push(emotion);
                    names.len() - 1
                }
            };
            order.push(EmotionCase { emotion: Emotion(i as u8), when });
        }
        match order.last() {
            None => return Err("the emotions' order names none".into()),
            Some(c) if !c.when.is_empty() => {
                return Err(format!("the emotions' order ends with `{}` when it holds: its last case holds always", names[c.emotion.0 as usize]));
            }
            Some(_) => {}
        }
        let mut roles = vec![None; names.len()];
        for (name, role) in roles_by_name {
            let Some(i) = names.iter().position(|n| *n == name) else {
                return Err(format!("roles: `{name}` is no emotion of the order's"));
            };
            roles[i] = Some(role);
        }
        // (The section states the rest; EXE6's until then.)
        Ok(EmotionRules {
            mood_held,
            anger_end,
            names,
            order,
            roles,
            hit_mood: HitMood::CounterMark,
            full_synchro_spent: 0x80,
            anger_boost_sound: true,
            anger_damage: Some(300),
        })
    }
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
    /// Its spawn has it run while the battle is paused (EXE6's
    /// `sub_80C4C12` sets the header flag 0x04); else it waits for the
    /// battle to run (EXE5's 0x080C46E2, EXE4's 0x080CD276).
    pub spawn_runs_while_paused: bool,
}

/// Global rules: element weakness, collision types, panels, banners,
/// statuses and the Beast Out lock-on. No default: a game's rules state
/// them (`content::sections`), and the engine has no game's of its own.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Rules {
    /// Extra damage multiplier by the receiver's element, then the
    /// hitter's (0 null, 1 fire, 2 aqua, 3 elec, 4 wood, 5 the drain
    /// element).
    pub element_weakness: [[u8; 6]; 6],
    /// The game's chip families (rule section `elements`).
    pub chip_families: ChipFamilies,
    /// The secondary elements each chip family adds to its attacks, by
    /// family (a family past them adds none).
    pub family_elements: Vec<SecondaryElements>,
    pub panels: PanelRules,
    /// Banners that stay up until removed.
    pub holding_banners: Vec<BannerId>,
    /// The NaviCust's HP bug's drain (rule section `status`).
    pub hp_drain: HpDrainRule,
    /// The custom screen's HP bug's drain (rule section `status`).
    pub custom_drain: CustomDrainRule,
    /// EXE6's per-form tick runs (`off_80EA93C`: `sub_80F0608`, MegaMan's
    /// and ChargeMan's: the Fire chips' charge, a form's height); EXE5's
    /// table (0x080EB1E8) has none of it (rule section `status`), its
    /// MegaMan's routine being the forms' own (`FormDef::tick`).
    pub form_tick: bool,
    /// A form's weapon load leaves the B+Left special as the status reset
    /// loaded it (rule section `status`): EXE4's (0x0800BD48 in a soul sets
    /// only AIData +0x0C to +0x0F; +0x00 is the navi's, 0x0800D8C2, or the
    /// soul's own routine's: ProtoSoul's). EXE6's and EXE5's set the form's.
    pub form_keeps_back_special: bool,
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
    /// How a navi's reaction actions run (rule section `status`).
    pub reaction_actions: ReactionActions,
    /// A drag's pose and its end (rule section `status`).
    pub drag: DragRule,
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
    /// What a navi does while the battle is paused (rule section
    /// `status`): see [`PausedNavi`].
    pub paused_navi: PausedNavi,
    /// A navi's status timers (paralysis and the rest, `sub_800E730`) count
    /// while the battle is paused (rule section `status`): EXE4's
    /// (0x0800AE58, no pause test); EXE6's and EXE5's (0x0800CB50) hold.
    pub status_timers_while_paused: bool,
    /// The idle stands its navi (animation 0) on each tick past its first
    /// phase, the 10 ticks after a reaction's end (rule section `status`):
    /// EXE4's (0x080EEB7C: 0x080EEBAC), so a pose a reaction keeps (EXE4's
    /// drag's, `DragEnding::KeepsPose`) holds those ticks and no more;
    /// EXE6's (`sub_80F0354`) and EXE5's (0x080F027A) leave the pose.
    pub idle_stands: bool,
    /// What a navi's body hits for in a link battle (rule section
    /// `status`): none for EXE4, which sets none (its body hits for its
    /// object's zeroed damage).
    pub link_body_damage: Option<LinkBodyDamage>,
    /// How a navi takes a hit's NaviCust bug (rule section `status`, the
    /// navi's game's).
    pub intake: IntakeRules,
    /// What the charge rules read for an empty hand's chip.
    pub empty_hand: EmptyHandChip,
    /// Ticks of recovery after a buster shot, by Rapid stat, then by open
    /// panels ahead (0..=5).
    pub buster_recovery: Vec<[u8; 6]>,
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
    /// The slide a panel's move end starts by rows (slide type 2, by the
    /// direction of the move: none, up, down, back, forward, other; `dx`
    /// toward the body's front): EXE6's `byte_800E4E8`, EXE5's 0x0800C988
    /// (ice's); none in a game whose ice pushes (EXE4's).
    pub slide_rows: Option<[SlideVector; 6]>,
    /// How a move's direction goes into the collision record (the
    /// reactions section's).
    pub move_direction: MoveDirection,
    /// How fast a navi slides and is dragged (the reactions section's).
    pub slide_speed: SlideSpeed,
    /// How a navi's hooks restart what it wears (the reactions section's).
    pub overlay_restart: OverlayRestart,
    /// When a stance's counter runs (the reactions section's).
    pub stance_counter: StanceCounter,
    /// When a trap's counter runs (the reactions section's): EXE6's and
    /// EXE5's `sub_801056A` runs its first step at once; EXE4's 0x0800C780,
    /// a stance's catch and a trap's alike, sets its action alone.
    pub trap_counter: StanceCounter,
    /// What a deleted player's object does (the reactions section's).
    pub dead_player: DeadPlayer,
    /// What an attack's end hands its lockout on to (the reactions
    /// section's): see [`AttackEndLockout`]. (An end a navi's action asks to
    /// keep the lockout, EXE4's 0x0800CA28, hands none on.)
    pub attack_end_lockout: AttackEndLockout,
    /// What the ends of a navi's actions clear of its requests (the
    /// reactions section's).
    pub request_clears: RequestClears,
    /// What a navi's statuses' end clears (the reactions section's).
    pub status_end: StatusEnd,
    /// A bubbled navi's height, by bubble timer.
    pub bubble_bob: [i8; 32],
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
        self.family_elements.get(family.0 as usize).copied().unwrap_or_default()
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
    /// The game's panel types, by number (`PanelType`): what each is.
    pub types: Vec<PanelTypeRule>,
    /// The field's cycle's period (EXE6's 0x8C, `sub_800BFC4`: its volcanos
    /// erupt by it); none, no cycle (EXE5's and EXE4's).
    pub cycle: Option<u32>,
    /// Each type's name, by number (the section's `numbers`).
    pub names: Vec<String>,
    /// The types the engine's own code needs (the section's `roles`).
    pub roles: PanelRoles,
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
    /// Whether a reservation marks its holder (`Reservations`).
    pub reservations: Reservations,
    /// The flags word's bits a panel's type owns (its number, solidity, its
    /// crack and the types' own flags), which a crack, a break or poison
    /// clears before it sets its own: EXE6's 0x3F5F (`object_crackPanel`
    /// and its kin), EXE5's and EXE4's 0x23F5F (their sea's and metal's
    /// 0x20000 too). A crack keeps the solidity and the crack bit.
    pub type_mask: u32,
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

/// How a navi's reaction actions run, flinch, paralysis (and freeze and
/// bubble) and drag (the status section's `reaction_actions`), where games
/// differ beyond their hooks and requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReactionActions {
    /// EXE6's (`sub_80174FE`, `sub_80175B8`, `sub_80178D4`, `sub_8017A38`;
    /// EXE5's alike, 0x08014132 on): each marks the action in use (flag
    /// 0x400000) and leaves it at its end; a flinch and a paralysis snap the
    /// body to its panel, on the ground, unless it slides; each counts a
    /// reaction (the side's stat 3) and lets go of the navi's overlay link;
    /// the drag puts the body on the panel's ground line (its pose and its
    /// end are the status section's `drag`).
    Marked,
    /// EXE4's (0x08010960, 0x080109FA, 0x08010ABC, 0x08010C16): none marks
    /// the action in use or lets go of the overlay link; the flinch keeps
    /// the body's height as it snaps it, and the paralysis snaps it, at its
    /// height, sliding or not, and counts no reaction; the drag keeps the
    /// height and counts no reaction (its pose and its end are the status
    /// section's `drag`).
    Plain,
}

/// What an attack's end hands its lockout (the attack's +5) on to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttackEndLockout {
    /// EXE6's and EXE5's (`sub_801171C`, 0x0800F2D0): by the attack's kind,
    /// a chip's (kind 2) to the chip lockout, the B+Back special's (kind 3)
    /// to its cooldown, any other's to none.
    ByKind,
    /// EXE4's (0x0800C9FC, its chips' and weapons' end): to the chip
    /// lockout (AIData +0x3A) whatever the kind, a 0 clearing it.
    ChipLockout,
}

/// A drag's pose and its end (the status section's `drag`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DragRule {
    pub poses: DragPoses,
    pub ending: DragEnding,
}

/// The pose a drag starts in: the first that holds of a paralyzed navi's
/// and a SuperArmor one's (each a game's, or none), else `otherwise`.
/// EXE6's `sub_80178D4`: paralyzed 2, SuperArmor 0, else 1; EXE5's
/// 0x08014304: SuperArmor 0, else 1; EXE4's 0x08010ABC: 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DragPoses {
    #[serde(default)]
    pub paralyzed: Option<u8>,
    #[serde(default)]
    pub super_armor: Option<u8>,
    pub otherwise: u8,
}

impl DragPoses {
    /// The pose for a navi of status word `status`.
    pub fn pose(&self, status: u32) -> u8 {
        use crate::collision::f1;
        match (self.paralyzed, self.super_armor) {
            (Some(p), _) if status & f1::PARALYZED != 0 => p,
            (_, Some(s)) if status & f1::SUPERARMOR != 0 => s,
            _ => self.otherwise,
        }
    }
}

/// What a drag's end does once its ticks are up (each clears the drag, the
/// action's use where the reaction actions mark it, and the drag's
/// requests, then idles).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DragEnding {
    /// EXE6's (`sub_8017A38`): a paralysis that outlasts the drag goes on,
    /// as the paralysis action; else it also clears the slide and the
    /// paralysis, the heat trap and a slide request, the slide's state, the
    /// pose back to standing, the form's overlay refreshed.
    ResumesParalysis,
    /// EXE5's (0x080144CE): the pose back to standing, whatever the
    /// paralysis.
    Stands,
    /// EXE4's (0x08010C16): in the pose it has.
    KeepsPose,
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

/// The panel types the engine's own code needs, each one of the game's
/// (the panels section's `roles`): the missing panel (no type change
/// reaches it), what a break and a crack make, and the normal panel (what
/// a mend, an expiry, a clearing and a burn leave).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PanelRoles {
    pub missing: PanelType,
    pub broken: PanelType,
    pub cracked: PanelType,
    pub normal: PanelType,
}

impl PanelRules {
    /// The panel type the game numbers `n` (a panel trail's byte), if the
    /// game has one.
    pub fn numbered(&self, n: u8) -> Option<PanelType> {
        ((n as usize) < self.types.len()).then_some(PanelType(n))
    }

    /// What panel type `t` is.
    pub fn rule(&self, t: PanelType) -> &PanelTypeRule {
        self.types.get(t.0 as usize).unwrap_or_else(|| panic!("panel type {} is none of its game's ({})", t.0, self.names.join(", ")))
    }

    /// The type the game names `name`.
    pub fn named(&self, name: &str) -> Option<PanelType> {
        self.names.iter().position(|n| n == name).map(|i| PanelType(i as u8))
    }

    /// Type `t`'s name.
    pub fn name(&self, t: PanelType) -> &str {
        self.names.get(t.0 as usize).map_or("(none of the game's)", String::as_str)
    }

    /// The flag bits a panel type contributes to a panel's flags word,
    /// with the type itself in the low nibble: the game's number of it
    /// (EXE5's holy is its 9, EXE6's its 5), which is what the original's
    /// word holds and what content reading the word's low byte reads (EXE5's
    /// GyroMan's Airforce, 0x080F0AD0).
    pub fn type_flags(&self, t: PanelType) -> u32 {
        t.0 as u32 | self.rule(t).flags
    }

    /// Type `t` as the right-hand console draws it: a type carrying across
    /// the field is the one carrying the other way (EXE6's left and right
    /// roads swap), any other type itself.
    pub fn mirrored(&self, t: PanelType) -> PanelType {
        match self.rule(t).carries {
            Some(v) if v.dx != 0 => self
                .types
                .iter()
                .position(|r| r.carries.is_some_and(|w| w.dx == -v.dx && w.dy == v.dy))
                .map_or(t, |i| PanelType(i as u8)),
            _ => t,
        }
    }}

/// What one panel type is, and what it does (docs/design/exe5-map.md
/// §15.2; the behaviors are the engine's, keyed by the panel type, their
/// numbers the game's).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PanelTypeRule {
    /// Flag bits the type adds to a panel's flags word.
    pub flags: u32,
    /// Where it carries a navi's slide of type 3 (EXE6's roads), when its
    /// `slide` says "carry".
    pub carries: Option<SlideVector>,
    /// The sound a NaviCust panel trail makes turning a panel into the
    /// type (`byte_8013D44`; none: silent).
    pub trail_sound: Option<crate::sound::SoundId>,
    /// Ticks the type lasts before the panel turns normal, blinking its
    /// last 60 (EXE6's roads 0x708, `sub_800C380`; EXE5's lava and sea 960,
    /// 0x0800A998).
    pub expires: Option<u16>,
    /// A body that can dive (its AI's flag 0x20) is submerged while on it,
    /// and no body is submerged off it (EXE5's sea, 0x08017030).
    pub submerges: bool,
    /// Where it carries a navi's slide of type 3 by the direction of the
    /// move, tried in turn (EXE5's magnet, 0x0800C8A8), when it does.
    pub carries_by_move: Option<PanelSlide>,
    /// The element of the hitboxes that turn it normal as they pass over
    /// it (`sub_3007708`: fire grass, aqua the volcano, wood roads; EXE5's
    /// 0x08016D14: and aqua lava, wood magnet).
    pub cleared_by: Option<u8>,
    /// The element whose hits count once more, as null damage, on a body
    /// standing on it (the hit kernel's `applyHeatOnGrassDamage_300766c`:
    /// fire on grass; EXE5's 0x08016AF6 elec on its sea too, EXE4's
    /// 0x08012CF2 elec on ice).
    pub doubles: Option<u8>,
    /// The shift that divides the damage a body standing on it takes, by
    /// element, rounding up (holy's 1, halving: `object_calculateFinalDamage1`
    /// and `object_calculateFinalDamage2`; a barrier's absorbing too,
    /// `sub_801A802`).
    pub damage_shift: u8,
    /// Nothing cracks or breaks it (EXE4's metal: its flag 0x20000, which
    /// the panel routines refuse).
    pub unbreakable: bool,
    /// A body standing on it can't move unless it floats (EXE4's pitfall:
    /// its `object_canMove`, 0x0800AD2A, and its kin 0x0800AD54,
    /// 0x0800AD7E).
    pub traps: bool,
}

/// A panel's slide (EXE5's magnet): by the direction the body last moved
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

/// A status effect (`new.status`): the requests it raises, its duration
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


