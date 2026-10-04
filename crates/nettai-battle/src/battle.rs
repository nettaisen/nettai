//! The battle: round state, the per-tick order of operations, and the flow
//! state machines (intro, banner, custom screen, fighting, results, end).
//! See docs/engine/battle-flow.md.

use nettai_content_api::SystemHook;
use crate::content::RootId;
use crate::actor::{ActorId, Actors};
use crate::collision::Collision;
use crate::behavior::Behaviors;
use crate::custom::{CustomScreens, Recorded};
use crate::field::Field;
use crate::hand::ChipHand;
use crate::hud::{Banner, BannerStatus, CustomGauge};
use crate::input::{InputRecord, PlayerTick, keys};
use crate::link::{Link, Packet};
use crate::object::{ObjectRef, Objects};
use crate::console::Console;
use crate::rng::Rng;
use crate::content::{BannerId, BannerRole, Content, FormData, MusicRole, NaviData, SoundRole};
use crate::setup::{BattleSettings, NaviStats, RoundSetup, SetScore, effects};
use crate::transform::{TransformRequest, TransformSequencer};
use crate::sound::SoundCue;
use nettai_content_api::ChipHandle;
use std::sync::Arc;

/// Battle flag bits.
pub mod battle_flags {
    /// The fight has started (collision is live).
    pub const FIGHTING: u16 = 0x01;
    /// The custom gauge is full.
    pub const GAUGE_FULL: u16 = 0x02;
    /// Dimming (dimming chips).
    pub const DIMMED: u16 = 0x04;
    /// A player asked to open the custom screen.
    pub const CUSTOM_REQUESTED: u16 = 0x10;
    /// The cameras shake even while the battle is paused
    /// (`battle_isTimeStopPauseOrBattleFlags0x20_800a0a4`): BN5's
    /// TomahawkSoul's change sets it (0x08012138), every soul change's end
    /// clears it (0x080121B6); nothing in BN6 sets it.
    pub const SHAKE_THROUGH_PAUSE: u16 = 0x20;
    /// Per-player custom gauges and chip counters (`sub_802E112`). Never set
    /// in netbattles (battle mode 1) or random battles.
    pub const PER_PLAYER_GAUGES: u16 = 0x40;
}

/// Top-level battle states (the game's jump-table offsets).
pub mod top {
    pub const INIT: u8 = 0x0;
    pub const RUNNING: u8 = 0x4;
    pub const END: u8 = 0x8;
}

/// Battle-mode handler states (netbattle).
pub mod mode {
    pub const INTRO: u8 = 0x00;
    pub const BANNER: u8 = 0x04;
    pub const CUSTOM: u8 = 0x08;
    pub const FIGHTING: u8 = 0x0C;
    pub const FADE_OUT: u8 = 0x14;
}

/// Fighting-phase states.
pub mod fight {
    pub const SETUP: u8 = 0x00;
    pub const START_BANNER: u8 = 0x04;
    pub const FIGHTING: u8 = 0x08;
    pub const WIN: u8 = 0x0C;
    pub const LOSE: u8 = 0x10;
    pub const DRAW: u8 = 0x14;
    pub const JUDGE: u8 = 0x18;
    pub const PAUSE: u8 = 0x1C;
    pub const CUSTOM_REVERT: u8 = 0x20;
    pub const CUSTOM_SEQUENCE: u8 = 0x24;
}

/// Round-level state (the game's BattleState).
#[derive(Clone, Debug, Default, Hash)]
pub struct RoundState {
    pub top: u8,
    pub mode: u8,
    pub sub: u8,
    pub init: u8,
    /// Actors counted per side (drops when an actor is removed).
    pub actor_count: [u8; 2],
    /// Custom screens opened so far (the turn number).
    pub turn: u8,
    pub name_counts: [u8; 2],
    pub running: u8,
    pub time_up: u8,
    pub local_side: u8,
    /// Cycles mod 20 and mod 180 (grass healing).
    pub cycle20: u8,
    pub cycle180: u8,
    pub mode_copy: u8,
    pub winner: u8,
    /// Alive navis per side.
    pub alive: [u8; 2],
    /// Both players' status bits as the link delivers them (bit 2: that
    /// player's custom screen is open).
    pub remote_status: [u8; 2],
    pub has_regular: u8,
    pub wins: u8,
    pub losses: u8,
    pub round: u8,
    pub max_combo: u8,
    pub combo: u8,
    pub combo_window: u8,
    pub busting_level: u8,
    pub result: u8,
    /// Low-HP music latch per side (`sub_8009158`).
    pub low_hp_music: [bool; 2],
    /// Small countdown used by banners and the end state.
    pub delay: i16,
    pub flags: u16,
    /// The local navi's HP when the battle ended (`sub_800FAE0`).
    pub exit_hp: u16,
    pub escape: u16,
    /// Ticks of fighting (capped).
    pub battle_time: u32,
    pub has_tags: u8,
    pub tag_index: u8,
    /// What each side's navis are taken for (their identities; four
    /// each).
    pub identities: [[Option<nettai_content_api::IdentityHandle>; 4]; 2],
    /// Intro progress bits (0x10 fade started, 0x01 fade done, 0x02 all
    /// navis in); 0x04/0x08 chips enabled per side.
    pub intro_bits: u8,
    pub frames: u32,
    pub ticks: u32,
    /// Alive actors, four slots per side.
    pub alive_actors: [[Option<ObjectRef>; 4]; 2],
    /// The actor list as spawned.
    pub spawned_actors: [[Option<ObjectRef>; 4]; 2],
}

/// The fighting-phase machine.
#[derive(Clone, Debug, Default, Hash)]
pub struct FightMachine {
    pub state: u8,
    pub sub: u8,
    pub init: u8,
    /// 0 none, 1 local win, 2 local loss, 3 draw, 6 open the custom screen.
    pub result: u8,
    pub pausing_player: u8,
    pub timer: i16,
    pub turn_timer: u16,
    /// The damage judge after a time-up (`dword_203EAD0`).
    pub judge: Judge,
}

/// The ticks between a console's low-HP sounds.
const LOW_HP_SOUND_TICKS: u8 = 0x2D;

/// What opening the custom screen costs a side in the battle flag 0x40
/// mode (`sub_800A29A`).
const GAUGE_CUSTOM_COST: u16 = 0x2900;

/// The damage judge (`sub_802CB38` sets it up, `sub_802CB78` runs it): the
/// judge's banner with both navis' damage taken, 59 ticks of rolling
/// digits (one RNG draw each), the real values for 120 ticks, then 30 more.
/// Less damage taken wins; equal damage is a draw.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Judge {
    /// +0: 0 starting, 4 running, 8 done.
    pub state: u8,
    /// +1, +2, +3: the running state's step, sub-step, and whether the
    /// sub-step's entry ran.
    pub step: u8,
    pub sub: u8,
    pub sub_init: bool,
    /// +5.
    pub timer: u8,
    /// +7: 1 the local side wins, 2 it loses, 3 a draw.
    pub outcome: u8,
    /// +8, +0xA: the damage side 1 and side 0 took.
    pub damage: [u16; 2],
    /// +0xC, +0xE: the rolling digits shown (presentation).
    pub rolled: [u16; 2],
}

/// The screen fades a battle starts (`SetScreenFade`'s modes): which
/// colors, which way the level steps (`off_8005FB4`) and where it stops
/// (`off_8006040`'s last byte, times 16).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FadeMode {
    /// 0: the intro of a set's first battle clears from white.
    IntroFromWhite = 0x00,
    /// 4: the end of a round, to white (`sub_80094DA`'s other case).
    EndToWhite = 0x04,
    /// 8: the intro of a later battle clears from black.
    IntroFromBlack = 0x08,
    /// 0xC: the end of a round, to black.
    EndToBlack = 0x0C,
    /// 0x38: a dimming's end (`object_undimScreen`).
    Undim = 0x38,
    /// 0x3C: a dimming (`object_dimScreen`): a quarter of the way.
    Dim = 0x3C,
    /// 0x40: the transformation sequencer's fade back in.
    TransformIn = 0x40,
    /// 0x44: the transformation sequencer's fade out.
    TransformOut = 0x44,
    /// 0x30: BN5's soul button's flash fades back (its custom screen's
    /// state 9).
    SoulFlashBack = 0x30,
    /// 0x34: ... and its flash, to full.
    SoulFlash = 0x34,
    /// 0x10: the custom screen's Program Advance animation fades back in.
    ProgramAdvanceBack = 0x10,
    /// 0x14: ... and out, a quarter of the way.
    ProgramAdvance = 0x14,
    /// 0x50: the custom screen's cursor leaves a dark chip.
    DarkChipBack = 0x50,
    /// 0x54: the cursor rests on a dark chip: five sixteenths of the way.
    DarkChip = 0x54,
    /// 0x58: the second fade record's (`loc_8006274`): the custom screen's
    /// window and sprites back in when the cursor leaves a dark chip.
    DarkChipWindowBack = 0x58,
    /// 0x5C: ... and darkened while it rests on one, three sixteenths.
    DarkChipWindow = 0x5C,
    /// 0x60: the custom screen's Beast Out fades back in.
    BeastOutBack = 0x60,
    /// 0x64: ... and out, half the way.
    BeastOut = 0x64,
    /// 0x6C: battle mode 1's fade back in after a transformation.
    Mode1TransformIn = 0x6C,
    /// 0x70: battle mode 1's fade out for a transformation.
    Mode1TransformOut = 0x70,
    /// 0x84: back from 0x88 (`sub_800BCF6`).
    BlackOutBack = 0x84,
    /// 0x88: the Gregar and Falzar chips' controllers darken what a dimming
    /// darkens all the way, to black.
    BlackOut = 0x88,
}

impl FadeMode {
    /// Toward full (`sub_800647C`) rather than toward clear
    /// (`sub_8006366`), and the level it stops at.
    fn course(self) -> (bool, u16) {
        match self {
            FadeMode::IntroFromWhite | FadeMode::IntroFromBlack => (false, 0),
            FadeMode::EndToWhite | FadeMode::EndToBlack => (true, 0x100),
            FadeMode::Undim => (false, 0),
            FadeMode::Dim => (true, 0x40),
            FadeMode::TransformIn | FadeMode::Mode1TransformIn => (false, 0),
            FadeMode::TransformOut | FadeMode::Mode1TransformOut => (true, 0x100),
            FadeMode::ProgramAdvanceBack | FadeMode::DarkChipBack | FadeMode::DarkChipWindowBack | FadeMode::BeastOutBack => {
                (false, 0)
            }
            FadeMode::SoulFlashBack => (false, 0),
            FadeMode::SoulFlash => (true, 0x100),
            FadeMode::DarkChipWindow => (true, 0x30),
            FadeMode::ProgramAdvance => (true, 0x40),
            FadeMode::DarkChip => (true, 0x50),
            FadeMode::BeastOut => (true, 0x80),
            FadeMode::BlackOutBack => (false, 0),
            FadeMode::BlackOut => (true, 0x100),
        }
    }
}

/// The screen fade (`eScreenFade`, the first of its two records): a level
/// from 0 (clear) to 0x100 (fully faded) that the running fade steps
/// toward its target once per frame, outside the battle tick
/// (`subsystem_triggerTransition_800630A`). The level outlives a fade: the
/// next one starts wherever the last one left it (a counter cut-in's dim
/// starts from the dimmed screen and is done after one step).
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Fade {
    /// +1: the running (or last) fade.
    pub mode: FadeMode,
    /// +6: the level.
    pub level: u16,
    /// +4: the level's change per step.
    pub speed: u16,
    /// +0xA: where the fade stops.
    pub target: u16,
    /// +3: the fade is running (`IsScreenFadeActive`).
    pub active: bool,
    /// +2: a fade toward clear took its first step (which holds the
    /// level).
    pub stepped: bool,
}

impl Default for Fade {
    /// A battle starts on a fully faded screen, which its intro clears.
    fn default() -> Self {
        Fade { mode: FadeMode::EndToBlack, level: 0x100, speed: 0, target: 0x100, active: false, stepped: false }
    }
}

impl Fade {
    /// The intro's fade takes this many ticks (from a fully faded screen,
    /// at the speed battles use).
    pub const TICKS: u8 = 17;

    /// `SetScreenFade(mode, speed)` (a speed of 0xFF steps by 0x100). The
    /// level is left where it is.
    pub fn start(&mut self, mode: FadeMode, speed: u8) {
        let (_, target) = mode.course();
        self.mode = mode;
        self.target = target;
        self.speed = if speed == 0xFF { 0x100 } else { speed as u16 };
        self.active = true;
        self.stepped = false;
    }

    /// `IsScreenFadeActive`.
    pub fn active(&self) -> bool {
        self.active
    }

    /// One step of the running fade (`off_8005FB4[mode]`), once per frame.
    pub fn step(&mut self) {
        if !self.active {
            return;
        }
        let (up, target) = self.mode.course();
        if up {
            // sub_800647C
            let level = self.level as i32 + self.speed as i32;
            if level >= target as i32 {
                self.active = false;
                self.level = target;
            } else {
                self.level = level as u16;
            }
        } else {
            // sub_8006366: the first step holds the level.
            let mut level = self.level as i32;
            if self.stepped {
                level = (level - self.speed as i32).max(0);
            }
            self.stepped = true;
            self.level = level as u16;
            if level <= target as i32 {
                self.active = false;
            }
        }
    }

    /// Steps left before the running fade is done (0 when it isn't
    /// running).
    pub fn remaining(&self) -> u8 {
        if !self.active || self.speed == 0 {
            return 0;
        }
        let (up, target) = self.mode.course();
        let steps = |d: u16| d.div_ceil(self.speed);
        let left = if up {
            steps(target.saturating_sub(self.level))
        } else {
            steps(self.level.saturating_sub(target)) + u16::from(!self.stepped)
        };
        left.min(0xFF) as u8
    }
}

/// A player's custom-screen result, as it goes over the link when their
/// window has slid out.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CustomResult {
    /// The chosen hand (None = no chips chosen: the previous hand stays).
    pub hand: Option<ChipHand>,
    pub navi_stats: NaviStats,
    /// Cross/Beast transformation request.
    pub transform: TransformRequest,
}

/// Events from outside the simulation that happen on a tick.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TickEvents {
    /// After the round, the link session the end state asked to close has
    /// closed.
    pub link_closed: bool,
    /// For checking against recordings that lack a player's folder: what
    /// the recording says that player's custom screen sends (see
    /// `custom::PlayerSetup::folder`). None for simulated players.
    pub recorded: [Option<Recorded>; 2],
}

#[derive(Clone, Debug)]
pub struct Battle {
    /// The content the battle runs on (read-only and shared: snapshots
    /// share it, and it is not part of the digest; `setup.content` is its
    /// identity).
    pub content: Arc<Content>,
    /// Whose data the battle reads (docs/design/rules-in-luau.md §2.3).
    pub games: BattleGames,
    pub setup: RoundSetup,
    pub stats: [NaviStats; 2],
    /// Each side's other navi's stats for a navi switch
    /// (`eBattleNaviStats2034A60`): a copy of the side's stats at the
    /// battle's start; a change keeps the navi it leaves here when it is
    /// this one, and takes the navi it goes to from here when it is that
    /// one (`sub_802D7A0`); a switch knockout takes it back (`sub_802D9B0`).
    pub reserves: [NaviStats; 2],
    pub rng: Rng,
    /// Each player's console: its own RNG (RNG1), which ChpShufl's re-deal
    /// draws from, and what advances it (`console`).
    pub consoles: [Console; 2],
    pub round: RoundState,
    pub fight: FightMachine,
    pub gauge: CustomGauge,
    pub banner: Banner,
    /// The chip each side last used, while the other player's console
    /// names it (presentation only; left out of the digest).
    pub used_chips: [Option<crate::hud::UsedChip>; 2],
    /// What each side's console shows of its own navi's chips
    /// (presentation only; left out of the digest).
    pub chip_hud: [crate::hud::ChipHud; 2],
    /// The HUD parts a chip's effect hid (presentation only; left out of the
    /// digest).
    pub hud_hidden: crate::hud::HudHidden,
    /// The message the HUD shows (presentation only; left out of the
    /// digest).
    pub message: Option<crate::hud::MessageLine>,
    /// The warning markers each console's HUD shows this tick
    /// (presentation only; left out of the digest).
    pub warnings: [Vec<crate::hud::Warning>; 2],
    /// The HP numbers each console's HUD shows under objects, by place
    /// (presentation only; left out of the digest).
    pub hp_numbers: [[Option<crate::hud::HpNumber>; crate::hud::HpNumber::PLACES]; 2],
    pub paused: bool,
    pub inputs: [InputRecord; 2],
    pub hands: [ChipHand; 2],
    /// Both players' transformation requests from the last custom screen.
    pub transform_requests: [TransformRequest; 2],
    /// The requests as this turn started (`unk_203A980`): what each navi
    /// changes into.
    pub turn_transforms: [TransformRequest; 2],
    /// The transformation sequencer run at the start of each turn.
    pub transform_seq: TransformSequencer,
    /// What a mid-battle custom-screen request waits for first.
    pub custom_reversion: crate::transform::CustomReversion,
    /// Per side: the bug frags the player brought (`dword_203F7E0`, from
    /// the save through the init exchange); a dark chip spends one.
    pub bug_frags: [u32; 2],
    /// Per side: the level of the navi code the save received
    /// (`dword_203CFA0`, from the save through the init exchange; 0xFF
    /// none: `PlayerSetup::navi_level`), which picks a link navi's chip
    /// bonus.
    pub navi_levels: [u8; 2],
    pub objects: Objects,
    pub actors: Actors,
    pub collision: Collision,
    pub field: Field,
    pub fade: Fade,
    /// Navis waiting to fade in at the intro.
    pub fadein_queue: [Option<ObjectRef>; 8],
    /// Per-side damage-carry records (`dword_203CFB0`).
    pub damage_carry: [DamageCarry; 2],
    /// Both players' custom screens.
    pub custom: CustomScreens,
    /// The link: what each player sends reaches the fight `delay` ticks
    /// later.
    pub link: Link,
    /// Per-side extra battle state (`sub_802E070`), used by the battle-flag
    /// 0x40 mode.
    pub sides: [SideState; 2],
    /// Per-side statistics counters (`byte_203EAE0`, `sub_800AB46`).
    pub side_stats: [[u8; 16]; 2],
    /// Per side: BN5's ColonelSoul army, armed or not, and its soldiers'
    /// damage words (`kinds::obstacle::Soldiers`).
    pub obstacle_soldiers: [crate::kinds::obstacle::Soldiers; 2],
    /// The first four counters of BN5's per-player battle record
    /// (`sub_802D064`'s, 0x0802AEA6): the counter hits and inflicted bugs
    /// that land on the other side's navis no player controls, at most 10
    /// each.
    pub navi_hit_counts: [[u8; 4]; 2],
    /// Each player's tactics (`crate::tactics`), as the computer navis'
    /// AI turns them: their setups' at the round's start.
    pub tactics: [crate::tactics::Tactics; 2],
    /// Per-side registry of defensive chips and their linked objects
    /// (0x10 bytes per side at 0x02036720).
    pub linked: [LinkedRecord; 2],
    /// Per side: its dimming (`byte_203CF00`).
    pub dimming: [crate::dimming::DimmingRecord; 2],
    /// The last navi chip used, of either side (`byte_203C960`, BN5's
    /// 0x0203C430; cleared as the battle starts, `sub_800B75A`): BN5's
    /// DethPhnx brings its navi again; nothing in BN6 reads it.
    pub last_navi_chip: Option<crate::kinds::navi_chip::LastNaviChip>,
    /// Per side: the player's rules, its ruleset and its systems' state
    /// (docs/design/rules-in-luau.md).
    pub rules: [crate::rules::SideRules; 2],
    /// Sound calls made this tick, as each side's player hears them
    /// (output only; see `sound`).
    pub(crate) sound: [Vec<SoundCue>; 2],
    /// How the round ended, once the end state is through.
    pub(crate) outcome: Option<RoundEnd>,
    /// A folder a tool is having checked (`Battle::check_folder`), and what
    /// it breaks: no part of the simulation.
    pub(crate) folder_check: Option<crate::rules::FolderCheck>,
}

/// How a round ended (`sub_8007CA0`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RoundEnd {
    /// The set goes on. The next round starts with its own init (link
    /// sync, the navi stats and RNG exchange) from these settings and the
    /// score so far.
    NextRound { settings: BattleSettings, score: SetScore },
    /// The battle is over.
    Over(BattleResult),
    /// The engine stopped the battle: a tick failed (`Battle::fail`), on a
    /// content error or a state the original can't go on from (it would read
    /// past a table or through a null pointer). The message says what
    /// happened. Two battles that step the same state on the same inputs
    /// stop alike, so under netplay both peers end the match here.
    Error(String),
}

/// The battle's result from the local side's perspective (BattleState
/// +0x1F, which the game hands back to the menu that started the battle).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BattleResult {
    Won = 1,
    Lost = 2,
    Drawn = 3,
    /// The player ran away (single-player battles).
    Escaped = 4,
    /// The link broke (`sub_8007EB8`).
    CommError = 5,
    /// Result codes 9 and 0xA: the battle was cut short. (Their setters
    /// weren't found; `sub_8007CA0` treats both alike.)
    Terminated = 9,
    TerminatedA = 0xA,
}

/// Where a set stands after a round (`sub_800AF50`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SetStanding {
    Undecided,
    Decided(BattleResult),
}

/// A side's extra battle state (0x1D0 bytes at `sub_802E070(side)`); only
/// the fields the engine reads or writes are modeled (the rest are listed
/// in docs/engine/field-names.md). The per-player gauges' mode (battle
/// flag 0x40) uses it; outside it, only SloGauge and FstGauge write it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SideState {
    pub active: u8,
    pub panel_x: u8,
    /// A per-side gauge (a SELECT special needs 0x1500; counters add it).
    pub gauge: u16,
    /// +0x50: the SELECT special runs (`sub_802E4E4`).
    pub select_special: u8,
    /// +0x54: a system's takeover of the side's navi runs (BN6's Cross
    /// special, DarkInvs' auto-battle): idle asks the side's systems
    /// (`takeover`) instead of reading the buttons.
    pub takeover: u8,
    /// +2: ticks the SELECT special holds the navi (0xB4 when reset,
    /// `sub_802E07C`; `sub_802F068`).
    pub select_ticks: u8,
    /// +0x30: ticks left of the takeover (BN6's Cross special starts it at
    /// 0x1E0), counted down in the navi's stage B (`sub_802E1D8`).
    pub takeover_ticks: u16,
    /// +0x3C / +0x3A: ticks the side's gauge stays slow / fast (SloGauge,
    /// FstGauge), counted down by `sub_80107D4`.
    pub slow_gauge_ticks: u16,
    pub fast_gauge_ticks: u16,
    /// +0x12: the swing a variable sword makes for a navi no buttons drive
    /// (BN5's computer navi draws it before VarSwrd or NeoVari, 0x0802A330).
    pub sword_pick: u8,
    /// Presentation: the emotion window shows the second set of the base
    /// form's faces (BN5's Hub Style, NaviStats +0x4C: 0x0801AF8E adds 11 to
    /// the face), as the side's rules set it (`battle.set_face_variant`;
    /// `kinds::player::shows_face_variant`).
    pub face_variant: bool,
    /// +0x44: the target the side tracks (an actor of the other side), which
    /// an obstacle leaving hands on (`sub_802EF74`).
    pub tracked: Option<ObjectRef>,
    /// +0x34: the special chip the side's SELECT uses (`sub_800EE26`); none
    /// for the zeroed field, which reads as the zeroed chip
    /// (`roles.chips.zeroed`).
    pub special_chip: Option<ChipHandle>,
    /// +0x36 / +0x38: bonuses stored for the special chip, spent with it
    /// (on a damaging chip, on a navi chip).
    pub special_attack_bonus: u16,
    pub special_navi_bonus: u16,
}

/// A side's defensive-chip record (0x10 bytes per side at 0x02036720):
/// the chip, its damage word and bonus (for the counterattack), the navi
/// that used it, and the object that implements it, if any.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LinkedRecord {
    /// +0: the chip; none for an empty record (the game's 0).
    pub chip: Option<ChipHandle>,
    /// +2: the Atk+ / cross bonus.
    pub bonus: u16,
    /// +4: the damage word.
    pub damage: u32,
    /// +8: the navi that used the chip; its deletion clears the record.
    pub owner: Option<ObjectRef>,
    /// +0xC.
    pub object: Option<ObjectRef>,
}

impl Battle {
    /// `sub_800AB46`: bump a statistics counter (saturating at 0xFF).
    pub fn bump_side_stat(&mut self, side: u8, index: usize, n: u8) {
        let v = &mut self.side_stats[side as usize][index];
        *v = v.saturating_add(n);
    }

    /// `sub_802CEA6`: clear a side's defensive-chip record. Its object
    /// (ElemTrap's trap) ends when it sees that the record no longer names
    /// it; the original tells it through its second parameter.
    pub fn clear_linked(&mut self, side: u8) {
        self.linked[side as usize] = Default::default();
    }

    /// The stage's panel column pattern (which columns belong to which
    /// side).
    pub fn panel_pattern(&self) -> u8 {
        self.content.stage(self.setup.settings.stage).panel_pattern
    }

    /// A side's form.
    pub fn form(&self, side: usize) -> &FormData {
        self.content.form(self.stats[side].form)
    }

    /// A side's navi.
    pub fn navi(&self, side: usize) -> &NaviData {
        self.content.navi(self.stats[side].navi)
    }

    /// `battle_networkInvert`: whether `alliance` is not the local side.
    pub fn is_remote(&self, alliance: u8) -> bool {
        alliance ^ self.round.local_side != 0
    }

    /// `sub_800AA1A`: add `r` to the intro fade-in queue (unless present).
    pub fn fadein_enqueue(&mut self, r: ObjectRef) -> bool {
        for slot in self.fadein_queue.iter_mut() {
            match slot {
                Some(o) if *o == r => return false,
                None => {
                    *slot = Some(r);
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    /// `sub_800AA06`: whether `r` is first in the fade-in queue.
    pub fn fadein_is_head(&self, r: ObjectRef) -> bool {
        self.fadein_queue[0] == Some(r)
    }

    /// `sub_800AA40`: remove `r` from the fade-in queue, then compact it
    /// (`sub_800AA64`, which only looks at the first seven entries).
    pub fn fadein_dequeue(&mut self, r: ObjectRef) {
        for slot in self.fadein_queue.iter_mut() {
            if *slot == Some(r) {
                *slot = None;
            }
        }
        let kept: Vec<ObjectRef> = self.fadein_queue[..7].iter().flatten().copied().collect();
        for (i, slot) in self.fadein_queue[..7].iter_mut().enumerate() {
            *slot = kept.get(i).copied();
        }
    }
}

/// A side's damage-carry record: damage this tick and last tick, and the
/// objects it tracks.
#[derive(Clone, Copy, Debug, Default, Hash)]
pub struct DamageCarry {
    pub this_tick: u16,
    pub previous: u16,
    pub source: Option<ObjectRef>,
    pub target: Option<ObjectRef>,
}

/// Whose data a battle reads (docs/design/rules-in-luau.md §2.3): the
/// arena's game (the stage's root) for the battle's own (the flow, the
/// field, the hit kernel's tables, the music and banners), each side's game
/// (its ruleset's, `RulesetDef::game`) for the side's (its navi's roles, its
/// screen, the rule sections about one navi). In a battle of one game they
/// are all that game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BattleGames {
    pub arena: RootId,
    pub sides: [RootId; 2],
    /// Each side's ruleset (none: the stage's game's stock rules), whose own
    /// sections, a mix's, the side reads over its game's.
    pub rulesets: [Option<nettai_content_api::RulesetHandle>; 2],
}

impl BattleGames {
    /// A round's: its stage's game and its players' rulesets' games (a
    /// player with none plays the stage's game's stock rules).
    pub fn of(content: &Content, setup: &RoundSetup) -> BattleGames {
        let defs = &content.defs;
        let stage = &defs.stage(setup.settings.stage).key;
        let arena = defs.root_of(stage).unwrap_or_else(|| panic!("stage {stage}'s id names no game"));
        let rulesets = [0, 1].map(|p| setup.players[p].ruleset);
        BattleGames { arena, sides: rulesets.map(|r| defs.ruleset_game(r, arena)), rulesets }
    }

    /// The pools' capacities: each the larger of the two players' games'
    /// (the user's decision: a capacity-only limit takes the larger game).
    pub fn pool_capacity(&self, content: &Content) -> [u8; 3] {
        let [a, b] = self.sides.map(|g| content.rules_of(g).pools.slots());
        std::array::from_fn(|i| a[i].max(b[i]))
    }
}

impl Battle {
    /// Start a round on `content`: the state the game is in when its init
    /// finishes and the first battle tick is about to run, with the
    /// content's scripts running what they implement (a runtime this
    /// thread keeps: `behavior`). Panics if the setup names other content
    /// (`RoundSetup::content`), if the content isn't defined
    /// (`Content::define`), or if its scripts don't load (a content error,
    /// the same on every machine).
    pub fn new(mut setup: RoundSetup, content: Arc<Content>) -> Battle {
        let hash = content.hash();
        assert_eq!(setup.content, hash, "the round's setup names content {} but runs on content {hash}", setup.content);
        assert!(content.defs.defined, "a battle runs on defined content (Content::define)");
        Behaviors::for_content(&content).unwrap_or_else(|e| panic!("{e}"));
        let score = setup.score;
        let games = BattleGames::of(&content, &setup);
        let stage = content.stage(setup.settings.stage);
        let panels = &content.rules_of(games.arena).panels;
        let (field, mode) = (Field::new(panels, &stage.layout, stage.panel_pattern, stage.mode), stage.mode);
        let objects = Objects::with_capacity(games.pool_capacity(&content));
        let hands = [ChipHand::empty(&content, games.arena), ChipHand::empty(&content, games.arena)];
        let rules = [0, 1].map(|p| crate::rules::SideRules::for_player(&content, &mut setup.players[p], games.arena));
        let mut b = Battle {
            content,
            games,
            stats: setup.navi_stats,
            reserves: setup.navi_stats,
            rng: Rng::new(setup.rng),
            consoles: [Console::new(&setup.players[0].console), Console::new(&setup.players[1].console)],
            round: RoundState {
                running: 1,
                max_combo: score.max_combo.max(1),
                wins: score.wins,
                losses: score.losses,
                round: score.round,
                mode_copy: mode,
                local_side: setup.local_side,
                intro_bits: 0x0C,
                low_hp_music: std::array::from_fn(|side| side == setup.local_side as usize && setup.low_hp_music_latched),
                top: top::RUNNING,
                ..RoundState::default()
            },
            fight: FightMachine::default(),
            gauge: CustomGauge::new(),
            banner: Banner::default(),
            used_chips: [None; 2],
            chip_hud: Default::default(),
            hud_hidden: Default::default(),
            message: None,
            warnings: Default::default(),
            hp_numbers: [[None; crate::hud::HpNumber::PLACES]; 2],
            paused: false,
            inputs: [InputRecord::default(); 2],
            hands,
            transform_requests: [TransformRequest::NONE; 2],
            turn_transforms: [TransformRequest::NONE; 2],
            transform_seq: TransformSequencer::default(),
            custom_reversion: Default::default(),
            bug_frags: [setup.players[0].bug_frags, setup.players[1].bug_frags],
            navi_levels: setup.players.each_ref().map(|p| {
                // (A setup's checks refuse a level past the navi codes:
                // the tables a level reads stop there.)
                let level = p.navi_level.unwrap_or(0xFF);
                assert!(
                    p.navi_level.is_none_or(|l| l <= crate::custom::MAX_NAVI_LEVEL),
                    "a navi code's level is 0 to {}, not {level}",
                    crate::custom::MAX_NAVI_LEVEL
                );
                level
            }),
            objects,
            actors: Actors::default(),
            collision: Collision::new(),
            field,
            fade: Fade::default(),
            fadein_queue: [None; 8],
            damage_carry: [DamageCarry::default(); 2],
            custom: CustomScreens::new(&setup.players),
            link: Link::new(setup.link_delay),
            sides: [SideState::default(); 2],
            side_stats: [[0; 16]; 2],
            obstacle_soldiers: Default::default(),
            navi_hit_counts: [[0; 4]; 2],
            tactics: [setup.players[0].tactics.clone(), setup.players[1].tactics.clone()],
            linked: [LinkedRecord::default(); 2],
            dimming: Default::default(),
            last_navi_chip: None,
            rules,
            sound: [Vec::new(), Vec::new()],
            outcome: None,
            folder_check: None,
            setup,
        };
        // Each side's systems set the round up before anything reads the
        // side's stats (BN6's patch cards change them); the battle-start
        // copy of the stats (`reserves`) is of the stats after them.
        b.notify_systems(nettai_content_api::SystemHook::RoundSetup);
        b.reserves = b.stats;
        // Init's last steps: refresh every panel, then one unpaused panel
        // update.
        b.field.refresh_all(&b.content.rules_of(b.games.arena).panels, &b.collision);
        b.tick_panels();
        b
    }

    /// The battle's tables: the arena's game's (§2.3: the flow, the field,
    /// the hit kernel's).
    pub fn arena_rules(&self) -> &crate::content::Rules {
        self.content.rules_of(self.games.arena)
    }

    /// The battle's roles: the arena's game's (the flow's banners and
    /// music, the field's and the hit kernel's effects, sparks, statuses).
    pub fn arena_roles(&self) -> &crate::content::Roles {
        self.content.defs.roles(self.games.arena)
    }

    /// Side `side`'s tables (the rule sections about one navi): its
    /// ruleset's own, a mix's, else its game's.
    pub fn side_game_rules(&self, side: u8) -> &crate::content::Rules {
        let s = side as usize & 1;
        self.content.side_rules(self.games.rulesets[s], self.games.sides[s])
    }

    /// Side `side`'s game's roles (what the framework uses for the side's
    /// navi and objects: the actions its requests start, the sounds its
    /// player hears, its navi's effects and sprites).
    pub fn side_roles(&self, side: u8) -> &crate::content::Roles {
        self.content.defs.roles(self.games.sides[side as usize & 1])
    }

    /// The side object `r` is on, if it is on one (its alliance).
    pub fn side_of(&self, r: ObjectRef) -> Option<u8> {
        let a = self.objects.get(r).alliance;
        (a < 2).then_some(a)
    }

    /// The roles for object `r`: its side's game's, else the arena's.
    pub fn roles_for(&self, r: ObjectRef) -> &crate::content::Roles {
        match self.side_of(r) {
            Some(side) => self.side_roles(side),
            None => self.arena_roles(),
        }
    }

    /// The tables for object `r`: its side's game's, else the arena's.
    pub fn rules_for(&self, r: ObjectRef) -> &crate::content::Rules {
        match self.side_of(r) {
            Some(side) => self.side_game_rules(side),
            None => self.arena_rules(),
        }
    }

    /// The game of the definition with id `key`: its prefix's (the engine's
    /// own `engine/...`, the arena's).
    pub fn game_of(&self, key: &str) -> crate::content::RootId {
        self.content.defs.root_of(key).unwrap_or(self.games.arena)
    }

    /// The chip a zeroed chip field reads: the arena's game's zeroed chip
    /// (`Content::chip_or_zeroed`).
    pub fn chip_or_zeroed(&self, h: Option<nettai_content_api::ChipHandle>) -> nettai_content_api::ChipHandle {
        self.content.chip_or_zeroed(self.games.arena, h)
    }

    /// The record a chip field names, a zeroed one the arena's game's
    /// zeroed chip's.
    pub fn chip_field(&self, h: Option<nettai_content_api::ChipHandle>) -> &crate::content::ChipData {
        self.content.chip_field(self.games.arena, h)
    }

    /// The arena's game's zeroed chip.
    pub fn zeroed_chip(&self) -> Option<nettai_content_api::ChipHandle> {
        self.content.zeroed_chip(self.games.arena)
    }

    /// The rules of chip `chip`'s own game (docs/design/rules-in-luau.md
    /// §7.5: a chip runs as its game wrote it); no chip, the arena's.
    pub fn chip_rules(&self, chip: Option<nettai_content_api::ChipHandle>) -> &crate::content::Rules {
        match chip {
            Some(h) => self.content.rules_of(self.game_of(&self.content.defs.chip(h).key)),
            None => self.arena_rules(),
        }
    }

    /// Start a banner unless one is showing (`Banner::start`). Returns
    /// false if one was.
    pub fn start_banner(&mut self, id: BannerId) -> bool {
        self.banner.start(id, self.content.rules_of(self.games.arena).banner_holds(id))
    }

    pub fn is_dimmed(&self) -> bool {
        self.round.flags & battle_flags::DIMMED != 0
    }

    pub fn set_flags(&mut self, f: u16) {
        self.round.flags |= f;
    }

    pub fn clear_flags(&mut self, f: u16) {
        self.round.flags &= !f;
    }

    /// `battle_isBattleOver`: a side has no navis left, or time is up.
    pub fn is_battle_over(&self) -> bool {
        self.round.alive[0] == 0 || self.round.alive[1] == 0 || self.round.time_up != 0
    }

    /// `battle_isBattleOver` as seven callers use it: they test the CPU
    /// flag the function leaves, which reads "over" only for time-up. A KO
    /// looks like "not over" to them.
    pub fn is_battle_over_flag_quirk(&self) -> bool {
        self.round.time_up != 0 && self.round.alive[0] != 0 && self.round.alive[1] != 0
    }

    /// The key of `r`'s kind (`"bn6:bomb"`, `"engine/effect"`): how tools
    /// name what an object is.
    pub fn kind_key(&self, r: ObjectRef) -> &str {
        &self.content.defs.kind(self.objects.get(r).kind).key
    }

    /// The key of `r`'s kind in its root (`"bomb"`, `"engine/effect"`):
    /// how tests on one root name what an object is.
    pub fn local_kind_key(&self, r: ObjectRef) -> &str {
        nettai_content_api::keys::local(self.kind_key(r))
    }

    /// The player navi of a side (`sub_80103BC`).
    pub fn player(&self, side: u8) -> Option<ObjectRef> {
        let r = self.round.spawned_actors[side as usize][0]?;
        let a = self.objects.get(r).actor?;
        (self.actors.get(a).actor_type == crate::actor::ActorType::Player).then_some(r)
    }

    pub fn player_actor(&self, side: u8) -> Option<ActorId> {
        self.player(side).and_then(|r| self.objects.get(r).actor)
    }

    /// Report a sound call of the original (output only), heard on both
    /// sides.
    pub fn play_sound(&mut self, cue: impl Into<SoundCue>) {
        let cue = cue.into();
        for heard in &mut self.sound {
            heard.push(cue);
        }
    }

    /// Report a sound call that only `side`'s player hears (the original
    /// makes it on that player's console only).
    pub fn play_sound_for(&mut self, side: u8, cue: impl Into<SoundCue>) {
        self.sound[side as usize].push(cue.into());
    }

    /// Play the sound content gives `role`, heard on both sides; and to
    /// `side`'s player only.
    pub fn sound(&mut self, role: SoundRole) {
        let id = self.arena_roles().sound(role);
        self.play_sound(id);
    }

    pub fn sound_for(&mut self, side: u8, role: SoundRole) {
        let id = self.side_roles(side).sound(role);
        self.play_sound_for(side, id);
    }

    /// The sound calls of the last tick, in the order the game makes them,
    /// as the local side hears them.
    pub fn sound_cues(&self) -> &[SoundCue] {
        &self.sound[self.round.local_side as usize]
    }

    /// The sound calls of the last tick as `side`'s player hears them.
    pub fn sound_cues_for(&self, side: u8) -> &[SoundCue] {
        &self.sound[side as usize]
    }

    /// One battle tick (one frame of the running battle). A battle the
    /// engine stopped ([`RoundEnd::Error`]) doesn't tick: it stays as it is,
    /// silent.
    pub fn tick(&mut self, input: &[PlayerTick; 2], events: TickEvents) {
        for heard in &mut self.sound {
            heard.clear();
        }
        if self.is_stopped() {
            return;
        }
        for (shown, console) in self.warnings.iter_mut().zip(&mut self.consoles) {
            shown.clear();
            console.frames = console.frames.wrapping_add(1);
        }
        // Panel highlights and blinks last one frame: the game's field
        // renderer clears them after drawing.
        self.field.clear_one_frame_looks();
        match self.round.top {
            top::RUNNING => self.tick_running(input, events),
            top::END => self.tick_end(input, &events),
            _ => {}
        }
        self.end_console_frames();
        self.round.frames = self.round.frames.wrapping_add(1);
        self.fade.step();
    }

    /// The link's step at the start of a tick (`sub_801FF18`): both
    /// players' packets go out, the ones sent `delay` ticks ago arrive;
    /// and both joypads read this tick's buttons.
    fn exchange_packets(&mut self, input: &[PlayerTick; 2], events: &TickEvents) -> [Packet; 2] {
        let sent = std::array::from_fn(|p| Packet {
            held: input[p].held & 0x3FF,
            in_custom: match &events.recorded[p] {
                Some(r) => r.in_custom,
                None => self.custom.sides[p].in_custom,
            },
        });
        for (side, t) in self.custom.sides.iter_mut().zip(input) {
            side.joypad.update(t.held);
        }
        self.link.exchange(sent)
    }

    fn tick_running(&mut self, input: &[PlayerTick; 2], events: TickEvents) {
        // Apply both players' packets.
        let arrived = self.exchange_packets(input, &events);
        for (p, packet) in arrived.iter().enumerate() {
            self.inputs[p].update(packet.held | keys::PRESENT);
            self.round.remote_status[p] = if packet.in_custom { 4 } else { 0 };
        }

        self.run_mode_handler(&events);
        self.run_objects();
        self.update_cameras();
        if !self.paused && !self.is_dimmed() {
            self.tick_panels();
        }
        self.update_player_hands();
        self.run_hud_tasks();
        self.update_emotion_windows();
        self.update_linked_registry();
        self.refresh_variable_damage();
        if !self.paused {
            if !self.is_dimmed() {
                self.round.cycle20 = (self.round.cycle20 + 1) % 20;
                self.round.cycle180 = (self.round.cycle180 + 1) % 180;
            }
            self.shift_damage_carry();
        }
        self.custom_hp_drain(0);
        if self.setup.settings.effects & effects::LINK != 0 {
            self.custom_hp_drain(1);
        }
        self.round.ticks = self.round.ticks.wrapping_add(1);
    }

    /// Top state 8 (`sub_8007B80`): 11 ticks of objects still running,
    /// then close the link session (mode 0, `sub_8007B9C`). Once it has
    /// closed (mode 4) the round is over (`sub_8007CA0`).
    fn tick_end(&mut self, input: &[PlayerTick; 2], events: &TickEvents) {
        self.exchange_packets(input, events);
        if self.round.mode != 0 {
            if self.outcome.is_none() {
                self.finish_round();
            }
            return;
        }
        match self.round.sub {
            0 => {
                // sub_8007BD0
                if self.round.init == 0 {
                    self.round.delay = 10;
                    self.round.init = 4;
                } else {
                    self.round.delay -= 1;
                    if self.round.delay < 0 {
                        self.round.sub = 4;
                        self.round.init = 0;
                    }
                }
            }
            _ => {
                // sub_8007C14: ask the link to close, then wait for it.
                if self.round.init == 0 {
                    self.round.init = 4;
                } else if events.link_closed {
                    self.round.mode = 4;
                    self.round.sub = 0;
                    self.round.init = 0;
                }
            }
        }
        self.run_objects();
        if !self.paused && !self.is_dimmed() {
            self.tick_panels();
        }
    }

    /// How the round ended, once the end state is through.
    pub fn round_end(&self) -> Option<&RoundEnd> {
        self.outcome.as_ref()
    }

    /// Stop the battle on a failed tick: one that panicked, with `message`
    /// (a content error, or a state the original can't go on from). The round
    /// ends with [`RoundEnd::Error`]; the state stays as the failed tick left
    /// it, the failed tick makes no sound, and the battle doesn't tick again.
    /// The failed tick ran the same code on the same state up to the panic,
    /// so two battles that fail on the same inputs are left in the same
    /// state: under netplay, a tick that fails on confirmed inputs ends the
    /// match on both peers, and one that fails on predicted inputs is rolled
    /// back like any other.
    pub fn fail(&mut self, message: impl Into<String>) {
        for heard in &mut self.sound {
            heard.clear();
        }
        self.outcome = Some(RoundEnd::Error(message.into()));
    }

    /// The engine stopped the battle ([`Battle::fail`]).
    pub fn is_stopped(&self) -> bool {
        matches!(self.outcome, Some(RoundEnd::Error(_)))
    }

    /// `sub_8007CA0`: chain the set's next round, or end the battle.
    fn finish_round(&mut self) {
        self.play_sound(SoundCue::StopMusic);
        let code = self.round.result & 0xF;
        // A broken link or a cut-short battle ends the set.
        let cut_short = matches!(code, 5 | 9 | 0xA);
        if self.setup.settings.effects & effects::SET != 0 && !cut_short {
            match self.set_standing() {
                SetStanding::Undecided => return self.chain_next_round(),
                // setTwoStructs_800A840
                SetStanding::Decided(r) => self.round.result = r as u8,
            }
        }
        let result = match self.round.result & 0xF {
            1 => BattleResult::Won,
            2 => BattleResult::Lost,
            3 => BattleResult::Drawn,
            4 => BattleResult::Escaped,
            5 => BattleResult::CommError,
            9 => BattleResult::Terminated,
            0xA => BattleResult::TerminatedA,
            c => panic!("battle result code {c} is none sub_8007CA0 hands back"),
        };
        let link = self.setup.settings.effects & effects::LINK != 0;
        if result == BattleResult::CommError && !link {
            // Outside link battles the game may restart the battle
            // (sub_803F4EC, event flag 0x1733, loc_80071FE): the menus'.
            panic!("a single-player battle's communication error restarts it from the menus (sub_8007CA0)");
        }
        // (A win also counts toward a save-data statistic, dword_2000B30.)
        // sub_800FAE0: the local navi's HP, read from its object although
        // the fade-out freed it. A broken link skips it (loc_8007E38).
        if result != BattleResult::CommError
            && let Some(r) = self.player(self.round.local_side)
        {
            self.round.exit_hp = self.objects.get(r).hp;
        }
        // The rest updates the PET navi and rewards outside the battle and
        // hands the result back (loc_8007E38).
        self.paused = false;
        self.round.running = 0;
        self.outcome = Some(RoundEnd::Over(result));
    }

    /// `sub_800AF50`: a best-of-three set is decided once one side can no
    /// longer be caught, or after the third round.
    fn set_standing(&self) -> SetStanding {
        let r = &self.round;
        let left = 3 - r.round as i32;
        let (wins, losses) = (r.wins as i32, r.losses as i32);
        if wins > losses + left {
            SetStanding::Decided(BattleResult::Won)
        } else if losses > wins + left {
            SetStanding::Decided(BattleResult::Lost)
        } else if r.round >= 3 {
            SetStanding::Decided(BattleResult::Drawn)
        } else {
            SetStanding::Undecided
        }
    }

    /// The set goes on (`battleSettings_802D2B2`, `loc_8007204`): the next
    /// round is fought on the stage drawn for it and starts over with its
    /// init, keeping the score.
    fn chain_next_round(&mut self) {
        let stage = self.setup.later_stages[self.round.round as usize - 1];
        let settings = self.setup.next_settings(stage);
        let r = &self.round;
        let score = SetScore { wins: r.wins, losses: r.losses, round: r.round, max_combo: r.max_combo };
        self.round.top = top::INIT;
        self.round.mode = 0;
        self.round.sub = 0;
        self.round.init = 0;
        self.outcome = Some(RoundEnd::NextRound { settings, score });
    }

    /// Run every object's update in list order, with pause/dimming gating.
    pub fn run_objects(&mut self) {
        let mut cur = self.objects.loop_first();
        while let Some(r) = cur {
            let f = self.objects.get(r).flags;
            let mut run = true;
            if self.paused && f & crate::object::flags::RUN_WHILE_PAUSED == 0 {
                run = false;
            }
            if run && self.is_dimmed() && f & crate::object::flags::RUN_WHILE_DIMMED == 0 {
                run = false;
            }
            if run {
                crate::kinds::update(self, r);
            }
            cur = self.objects.loop_next();
        }
    }

    // ---- Battle-mode handler -------------------------------------------

    fn run_mode_handler(&mut self, events: &TickEvents) {
        match self.round.mode {
            mode::INTRO => self.mode_intro(),
            mode::BANNER => self.mode_banner(),
            mode::CUSTOM => self.mode_custom(&events.recorded),
            mode::FIGHTING => self.mode_fighting(),
            mode::FADE_OUT => self.mode_fade_out(),
            m => panic!("battle mode state {m:#x} not supported in netbattles"),
        }
        self.low_hp_music();
    }

    fn mode_intro(&mut self) {
        if self.round.init == 0 {
            let s = &self.setup.settings;
            if s.effects & effects::SET == 0 {
                self.round.round = self.content.stage(s.stage).battle_number;
            } else {
                self.round.round += 1;
            }
            crate::kinds::intro::spawn(self);
            self.spawn_actors();
            self.notify_systems(nettai_content_api::SystemHook::RoundStart);
            // Reward-chip pick: draws once; netbattle navis have no rewards.
            self.rng.next_positive();
            self.paused = true;
            self.gauge.rate = CustomGauge::rate_for(self.stats[0].gauge_speed, self.stats[1].gauge_speed);
            let link = self.setup.settings.effects & effects::LINK != 0;
            let music = if link {
                Some(self.arena_roles().music(MusicRole::LinkBattle))
            } else {
                self.content.stage(self.setup.settings.stage).music
            };
            if let Some(music) = music {
                self.play_sound(SoundCue::Music(music));
            }
            self.round.init = 4;
            return;
        }
        if self.round.sub == 0 {
            // sub_800927C: the HUD's setup.
            self.start_emotion_windows();
            self.round.sub = 4;
        }
        if self.round.intro_bits & 0x02 != 0 {
            self.enter_mode(mode::BANNER);
        }
    }

    fn enter_mode(&mut self, m: u8) {
        self.round.mode = m;
        self.round.sub = 0;
        self.round.init = 0;
    }

    /// `sub_8007368`: place what the stage names, in its order. Only navis
    /// join the alive/actor bookkeeping; rocks and other field objects
    /// don't. The field objects are content's: each is placed by its
    /// kind's `place` (the original's spawner for its entry type in
    /// `off_80073A0`: the rock's `sub_80074FA`, the boulder's
    /// `sub_8007450`, the Guardian statue's `sub_800751C`).
    pub fn spawn_actors(&mut self) {
        use crate::content::Place;
        use nettai_content_api::{HookCall, PanelPos, PlaceSpec};
        let content = self.content.clone();
        for entry in &content.stage(self.setup.settings.stage).actors {
            if let Place::Kind(kind) = entry.place {
                let hook = content.defs.kind(kind).place.expect("a stage places kinds with a `place` (checked at load)");
                let panel = PanelPos { x: entry.x, y: entry.y };
                let spec = PlaceSpec { panel, side: entry.side, variant: entry.variant, argument: entry.argument };
                crate::behavior::call_hook(self, hook, HookCall::Place { spec });
                continue;
            }
            let r = crate::kinds::player::spawn(self, entry);
            let side = entry.side as usize;
            if let Some(r) = r {
                let counted = self.objects.get(r).actor.map(|a| self.actors.get(a).not_counted != 1).unwrap_or(true);
                if let Some(slot) = self.round.alive_actors[side].iter_mut().find(|s| s.is_none()) {
                    *slot = Some(r);
                }
                if counted {
                    self.round.actor_count[side] += 1;
                }
                let n = self.round.name_counts[side] as usize;
                if n < 4 {
                    self.round.identities[side][n] = self.objects.get(r).identity;
                }
                self.round.name_counts[side] += 1;
            }
        }
        self.round.alive = self.round.actor_count;
        self.round.spawned_actors = self.round.alive_actors;
    }

    fn mode_banner(&mut self) {
        match self.round.sub {
            0 => {
                if self.round.round == 0 {
                    self.enter_mode(mode::CUSTOM);
                    return;
                }
                if self.round.init == 0 {
                    self.round.delay = 10;
                    self.round.init = 4;
                    return;
                }
                self.round.delay -= 1;
                if self.round.delay < 0 {
                    self.round.sub = 4;
                    self.round.init = 0;
                }
            }
            4 => {
                if self.round.init == 0 {
                    self.start_banner(self.arena_roles().banner(BannerRole::RoundStart));
                    self.round.init = 4;
                } else if self.banner.status() == BannerStatus::Done {
                    self.round.sub = 8;
                    self.round.init = 0;
                }
            }
            _ => {
                if self.round.init == 0 {
                    self.round.delay = 10;
                    self.round.init = 4;
                    return;
                }
                self.round.delay -= 1;
                if self.round.delay < 0 {
                    self.enter_mode(mode::CUSTOM);
                }
            }
        }
    }

    // ---- Custom screen ---------------------------------------------------

    /// Mode state 8 (`sub_8009338`): the custom screen. Both players'
    /// screens open on the first tick (`sub_8026840`) and run from the
    /// next; the tick after both results are in, the fight resumes
    /// (`sub_8026A6C`).
    fn mode_custom(&mut self, recorded: &[Option<Recorded>; 2]) {
        if self.round.init == 0 {
            self.round.init = 1;
            self.open_custom_screens();
            return;
        }
        if self.custom.committed {
            return self.close_custom_screens();
        }
        self.tick_custom_screens(recorded);
    }

    /// `sub_8026A6C`: the screens close and the fight resumes.
    fn close_custom_screens(&mut self) {
        self.restart_gauge();
        self.play_sound(SoundCue::RestoreVolume);
        // `sub_8009338`: each side's rules, for a side with its navi.
        for side in 0..2 {
            if self.player_actor(side).is_some() {
                self.notify_side(side, SystemHook::CustomClosed);
            }
        }
        self.custom.committed = false;
        self.enter_mode(mode::FIGHTING);
    }

    /// `sub_800B3D8`: both results are in: each hand with chips replaces
    /// that player's hand, both navis' stats are taken as sent, and the
    /// transformations wait for the turn to start.
    pub(crate) fn install_exchange(&mut self, results: [CustomResult; 2]) {
        for (side, r) in results.into_iter().enumerate() {
            if let Some(h) = r.hand {
                self.hands[side] = h;
            }
            self.stats[side] = r.navi_stats;
            self.transform_requests[side] = r.transform;
        }
    }

    /// `sub_8013FD0`: custom-HP bug damage at custom-screen open. Never kills.
    pub(crate) fn custom_hp_bug(&mut self, side: u8) {
        let v = self.stats[side as usize].bugs.custom_damage;
        if v == 0 {
            return;
        }
        if let Some(r) = self.player(side) {
            let hp = self.objects.get(r).hp;
            let d = v.min(hp.saturating_sub(1));
            crate::kinds::subtract_hp(self, r, d);
            self.sound(SoundRole::OwnHit);
        }
    }

    // ---- Fighting ----------------------------------------------------------

    fn mode_fighting(&mut self) {
        if self.round.init == 0 {
            self.fight = FightMachine::default();
            self.round.init = 4;
        }
        self.run_fight_machine();
        let r = self.fight.result;
        match r {
            0 => {}
            6 => self.enter_mode(mode::CUSTOM),
            _ => {
                match r {
                    1 | 2 => {
                        self.round.result = r;
                        self.round.busting_level = self.busting_level();
                    }
                    _ => {}
                }
                self.enter_mode(mode::FADE_OUT);
            }
        }
    }

    fn run_fight_machine(&mut self) {
        match self.fight.state {
            fight::SETUP => self.fight_setup(),
            fight::START_BANNER => self.fight_start_banner(),
            fight::FIGHTING => self.fight_fighting(),
            fight::WIN | fight::LOSE => self.fight_result(),
            fight::DRAW => self.fight_draw(),
            fight::JUDGE => self.fight_judge(),
            fight::PAUSE => self.fight_pause(),
            fight::CUSTOM_REVERT => self.fight_custom_revert(),
            fight::CUSTOM_SEQUENCE => self.fight_custom_sequence(),
            s => panic!("fighting state {s:#x} reads past its table (sub_80080D2)"),
        }
    }

    /// A word store of the fighting state: its sub-state and entry flag go
    /// back to 0.
    fn set_fight_state(&mut self, state: u8) {
        self.fight.state = state;
        self.fight.sub = 0;
        self.fight.init = 0;
    }

    /// Fighting state 0x14, a draw (`sub_80082DC`): the draw banner (0x1C);
    /// once it is done, in a set whose standing is decided (`sub_800AF50`)
    /// the set's winner's result state, otherwise the round is a draw.
    /// (Its 0x66-tick timer counts down, but its test reads the halfword
    /// unsigned and never holds the state.)
    fn fight_draw(&mut self) {
        if self.fight.init == 0 {
            // The HUD's parts hide, and its tasks stop (`sub_801BED6`): the
            // emotion windows' among them.
            self.stop_emotion_windows();
            self.fight.timer = 0x66;
            self.fight.init = 4;
            self.start_banner(self.arena_roles().banner(BannerRole::Draw));
        }
        self.fight.timer = self.fight.timer.wrapping_sub(1);
        if self.banner.status() != BannerStatus::Done {
            return;
        }
        if self.setup.settings.effects & effects::SET != 0 {
            match self.set_standing() {
                SetStanding::Decided(BattleResult::Won) => return self.set_fight_state(fight::WIN),
                SetStanding::Decided(BattleResult::Lost) => return self.set_fight_state(fight::LOSE),
                _ => {}
            }
        }
        // setTwoStructs_800A840(3) (and GameState+0x14 = 3).
        self.round.result = BattleResult::Drawn as u8;
        self.fight.result = BattleResult::Drawn as u8;
    }

    /// Fighting state 0x18, the damage judge after a time-up
    /// (`sub_800834A`): 60 ticks, then the judge (`sub_802CB38`,
    /// `sub_802CB78`); its outcome is a win (counted), a loss (counted) or a
    /// draw.
    fn fight_judge(&mut self) {
        if self.fight.sub == 0 {
            // sub_8008364
            if self.fight.init == 0 {
                self.fight.timer = 0;
                self.fight.init = 4;
            }
            self.fight.timer = self.fight.timer.wrapping_add(1);
            if self.fight.timer >= 0x3C {
                // (The HUD's time display hides.)
                self.fight.sub = 4;
                self.fight.init = 0;
            }
            return;
        }
        // sub_800838A
        if self.fight.init == 0 {
            let taken = |b: &Battle, side: u8| {
                let Some(a) = b.player_actor(side) else {
                    panic!("the damage judge reads a missing navi's damage (sub_801055E)");
                };
                b.actors.get(a).total_damage_taken
            };
            let (d1, d0) = (taken(self, 1), taken(self, 0));
            self.start_judge(d1, d0);
            self.fight.init = 4;
            return;
        }
        if self.step_judge() {
            return;
        }
        match self.fight.judge.outcome {
            1 => {
                self.round.wins += 1;
                self.set_fight_state(fight::WIN);
            }
            2 => {
                self.round.losses += 1;
                self.set_fight_state(fight::LOSE);
            }
            _ => self.set_fight_state(fight::DRAW),
        }
    }

    /// `sub_802CB38(damage of side 1, damage of side 0)`: the side that took
    /// more damage loses (none on equal damage; the fighting machine's +0x10
    /// keeps it, which nothing reads), and the outcome from the local side.
    fn start_judge(&mut self, d1: u16, d0: u16) {
        let loser = if d1 == d0 {
            None
        } else if d1 < d0 {
            Some(0)
        } else {
            Some(1)
        };
        let outcome = match loser {
            None => 3,
            Some(l) if l == self.round.local_side => 2,
            Some(_) => 1,
        };
        self.fight.judge = Judge { damage: [d1, d0], outcome, ..Judge::default() };
    }

    /// `sub_802CB78`: one tick of the judge; true while it runs.
    fn step_judge(&mut self) -> bool {
        let j = &mut self.fight.judge;
        match j.state {
            // sub_802CBA4
            0 => j.state = 4,
            // sub_802CBAC
            4 => match j.step {
                // sub_802CBCC: the judge's banner with both damages, the
                // local side's first.
                0 => {
                    j.step = 4;
                    j.sub = 0;
                    j.sub_init = false;
                    self.start_banner(self.arena_roles().banner(BannerRole::Judge));
                }
                // sub_802CBF2
                4 => match j.sub {
                    // sub_802CC10
                    0 => {
                        j.timer = 0x3C;
                        j.sub = 4;
                    }
                    // sub_802CC1A: rolling digits.
                    4 => {
                        j.timer = j.timer.wrapping_sub(1);
                        if j.timer == 0 {
                            j.sub = 8;
                            j.sub_init = false;
                        } else {
                            let v = self.rng.next_positive();
                            let j = &mut self.fight.judge;
                            j.rolled = [(v & 0xFFFF) as u16 % 0x270E, (v >> 16) as u16 % 0x270E];
                        }
                    }
                    // sub_802CC50: the real values for 120 ticks.
                    _ => {
                        if !j.sub_init {
                            j.timer = 0x78;
                            j.sub_init = true;
                        }
                        j.timer = j.timer.wrapping_sub(1);
                        if j.timer == 0 {
                            j.step = 8;
                            j.sub = 0;
                            j.sub_init = false;
                        }
                    }
                },
                // sub_802CC8C: the banner goes, 30 ticks.
                _ => {
                    if j.sub == 0 {
                        j.timer = 0x1E;
                        j.sub = 4;
                        self.banner.release();
                    }
                    let j = &mut self.fight.judge;
                    j.timer = j.timer.wrapping_sub(1);
                    if j.timer == 0 {
                        j.state = 8;
                        j.step = 0;
                        j.sub = 0;
                        j.sub_init = false;
                    }
                }
            },
            // sub_802CCAE
            _ => return false,
        }
        true
    }

    /// Fighting state 0x1C, paused (`sub_80083E4`): only the player who
    /// paused resumes, with a new START press (sound 0x9F); the battle
    /// unpauses at the top of the next fighting tick.
    fn fight_pause(&mut self) {
        let p = self.fight.pausing_player as usize & 1;
        if self.inputs[p].pressed & keys::START != 0 {
            self.sound(SoundRole::Pause);
            self.set_fight_state(fight::FIGHTING);
            // (The HUD's pause display hides.)
        }
    }

    /// Fighting state 0 (`sub_800840C`): run the transformation sequencer
    /// twice, then count down Beast Out.
    fn fight_setup(&mut self) {
        if self.fight.init == 0 {
            self.start_transform_sequencer();
            self.fight.init = 4;
        }
        if self.step_transform_sequencer() {
            return;
        }
        if self.fight.sub == 0 {
            self.transform_seq.restart();
            self.fight.sub = 4;
            return;
        }
        // The turn starts: each side's rules (BN6's beast system spends a
        // turn in Beast Out, `sub_8015A38`).
        for side in 0..2u8 {
            if self.player(side).is_some() {
                self.notify_side(side, SystemHook::TurnStarted);
            }
        }
        self.fight.state = fight::START_BANNER;
        self.fight.sub = 0;
        self.fight.init = 0;
    }

    fn apply_actor_inputs(&mut self) {
        for side in 0..2u8 {
            let Some(a) = self.player_actor(side) else { continue };
            let over = self.is_battle_over();
            // (A controlled form, BN6's Beast Over: the controller decides.)
            let berserk = self.form(side as usize).traits.has(crate::content::FormTraits::CONTROLLED);
            let held = self.inputs[side as usize].held;
            let dimmed = self.is_dimmed();
            let ad = self.actors.get_mut(a);
            if over {
                ad.pad = Default::default();
                continue;
            }
            if berserk {
                continue;
            }
            ad.pad.update(held);
            if !dimmed {
                ad.dimmed_pad = Default::default();
            } else {
                ad.dimmed_pad.update(held);
            }
        }
    }

    fn fight_start_banner(&mut self) {
        self.apply_actor_inputs();
        if self.fight.init == 0 {
            self.fight.timer = 0x1E;
            self.fight.init = 4;
            if self.late_turns() {
                self.fight.turn_timer = 0xA5 * 4 - 1;
                self.start_banner(self.arena_roles().banner(BannerRole::FinalTurn));
            } else if self.setup.settings.effects & effects::LINK != 0 {
                self.start_banner(self.arena_roles().banner(BannerRole::TurnStart));
            }
        }
        if self.banner.status() == BannerStatus::Done {
            self.fight.state = fight::FIGHTING;
            self.fight.sub = 0;
            self.fight.init = 0;
        }
    }

    /// From the 15th custom screen on, netbattles use a turn timer and the
    /// gauge stops arming.
    pub fn late_turns(&self) -> bool {
        self.setup.settings.effects & effects::LINK != 0 && self.round.turn >= 15
    }

    fn fight_fighting(&mut self) {
        self.apply_actor_inputs();
        self.paused = false;
        self.set_flags(battle_flags::FIGHTING);
        self.update_combo();
        self.update_battle_time();
        if self.late_turns() {
            self.update_turn_timer();
        }
        match self.round_result() {
            1 => {
                if self.round.escape != 0 && self.arena_rules().flow.escape_check {
                    // sub_800AAD6: an escape ends the battle as a loss
                    // (result code 4, then 2), straight to the fade-out.
                    self.round.result = BattleResult::Escaped as u8;
                    self.enter_mode(mode::FADE_OUT);
                    self.round.result = BattleResult::Lost as u8;
                    return;
                }
                self.round.wins += 1;
                self.fight.state = fight::WIN;
                return;
            }
            2 => {
                self.round.losses += 1;
                self.fight.state = fight::LOSE;
                return;
            }
            7 => {
                self.fight.state = fight::JUDGE;
                return;
            }
            _ => {}
        }
        if let Some(p) = self.pause_request() {
            self.fight.pausing_player = p;
            self.paused = true;
            self.set_fight_state(fight::PAUSE);
            // The HUD's pause display (`sub_801E15C`): "PAUSE" shows, with
            // its sound, and the opponent's used chip name goes.
            self.used_chips = [None; 2];
            self.sound(SoundRole::Pause);
            return;
        }
        let open = if self.round.flags & battle_flags::PER_PLAYER_GAUGES != 0 {
            // sub_800A244: in the battle flag 0x40 mode a side opens it with
            // L or R and a gauge of 0x2900, which it pays.
            let sides = self.gauge_custom_requests();
            for side in 0..2 {
                if sides & (1 << side) != 0 {
                    let g = &mut self.sides[side].gauge;
                    *g = g.wrapping_sub(GAUGE_CUSTOM_COST);
                }
            }
            sides != 0
        } else {
            self.custom_open_requested()
        };
        if open {
            self.paused = true;
            self.set_fight_state(fight::CUSTOM_REVERT);
        }
    }

    /// `sub_800A244`: the sides (bit per side; side 1 only in link
    /// battles) asking for the custom screen with L or R and a full enough
    /// gauge, unless dimmed, over, or a SELECT special runs.
    fn gauge_custom_requests(&self) -> u8 {
        if self.is_dimmed() || self.is_battle_over() {
            return 0;
        }
        if self.sides[0].select_special != 0 || self.sides[1].select_special != 0 {
            return 0;
        }
        // sub_800A29A
        let asks = |side: usize| {
            self.sides[side].gauge >= GAUGE_CUSTOM_COST && self.inputs[side].pressed & (keys::L | keys::R) != 0
        };
        let link = self.setup.settings.effects & effects::LINK != 0;
        u8::from(asks(0)) | (u8::from(link && asks(1)) << 1)
    }

    /// Whether a custom-screen request goes through the reversions and the
    /// sequencer (battle mode 5, or not the battle flag 0x40 mode; BN5's
    /// 0x08007774 tests the flag alone).
    fn custom_request_transforms(&self) -> bool {
        let mode_5 = self.round.mode_copy == 5 && self.arena_rules().flow.sequencer_before_custom;
        mode_5 || self.round.flags & battle_flags::PER_PLAYER_GAUGES == 0
    }

    /// Fighting state 0x20 (`sub_8008452`): a custom screen was asked for:
    /// wait for the navis' reversions, then state 0x24. BN5's (0x08007774,
    /// the flow without `sequencer_before_custom`) opens the screen from
    /// this state, on the tick the reversions are done, as state 0x24 does
    /// a tick later.
    fn fight_custom_revert(&mut self) {
        if self.custom_request_transforms() {
            if self.fight.init == 0 {
                self.start_custom_reversion();
                // Each side's rules, for a side with its navi (BN6's beast
                // system: a Beast Out check comes due, `sub_8015A16`).
                for side in 0..2u8 {
                    if self.player_actor(side).is_some() {
                        self.notify_side(side, SystemHook::CustomRequested);
                    }
                }
                self.fight.init = 4;
            }
            if self.step_custom_reversion() {
                return;
            }
        }
        if !self.arena_rules().flow.sequencer_before_custom {
            self.fight.result = 6;
            return;
        }
        self.fight.state = fight::CUSTOM_SEQUENCE;
        self.fight.sub = 0;
        self.fight.init = 0;
    }

    /// Fighting state 0x24 (`sub_8008492`): the transformation sequencer
    /// runs once more from the start, then the custom screen opens.
    fn fight_custom_sequence(&mut self) {
        // (BN5 opens the screen straight after the reversions: its flow has
        // no state 0x24.)
        if self.custom_request_transforms() && self.arena_rules().flow.sequencer_before_custom {
            if self.step_transform_sequencer() {
                return;
            }
            if self.fight.sub == 0 {
                self.transform_seq.restart();
                self.fight.sub = 4;
                return;
            }
        }
        self.fight.result = 6;
    }

    /// `sub_800A152`: the round's result from the local side's perspective.
    fn round_result(&self) -> u8 {
        if self.is_dimmed() {
            return 0;
        }
        let local = self.round.local_side;
        if self.round.actor_count[0] == 0 {
            return if local == 0 { 2 } else { 1 };
        }
        if self.round.actor_count[1] == 0 {
            return if local == 0 { 1 } else { 2 };
        }
        if self.round.time_up != 0 {
            return 7;
        }
        0
    }

    /// `sub_800A046`: a player pressed START.
    fn pause_request(&self) -> Option<u8> {
        if self.is_battle_over() || self.is_dimmed() {
            return None;
        }
        (0..2u8).find(|&p| self.inputs[p as usize].pressed & keys::START != 0)
    }

    /// `sub_800A1D0`.
    fn custom_open_requested(&self) -> bool {
        if self.is_dimmed() || self.is_battle_over() {
            return false;
        }
        // (A controlled navi, BN6's Beast Over, can't ask for it: a full
        // gauge opens it.)
        let berserk = |side: usize| self.form(side).traits.has(crate::content::FormTraits::CONTROLLED);
        ((berserk(0) || berserk(1)) && self.round.flags & battle_flags::GAUGE_FULL != 0)
            || self.round.flags & battle_flags::CUSTOM_REQUESTED != 0
    }

    fn update_combo(&mut self) {
        let r = &mut self.round;
        if r.combo > r.max_combo {
            r.max_combo = r.combo;
        }
        if r.combo_window != 0 {
            r.combo_window -= 1;
        } else {
            r.combo = 0;
        }
    }

    fn update_battle_time(&mut self) {
        if self.is_dimmed() || self.paused || self.round.flags & battle_flags::FIGHTING == 0 {
            return;
        }
        if self.is_battle_over_flag_quirk() {
            return;
        }
        if self.round.battle_time < 0x8C9F {
            self.round.battle_time += 1;
        }
    }

    fn update_turn_timer(&mut self) {
        if self.paused || self.is_dimmed() {
            return;
        }
        if self.is_battle_over() {
            return;
        }
        if self.fight.turn_timer >= 0x3C {
            self.fight.turn_timer -= 1;
        } else {
            self.round.time_up = 1;
        }
    }

    fn fight_result(&mut self) {
        if self.fight.init == 0 {
            // The HUD's tasks stop (`sub_801BED6(0xE4C53)`): the gauge's and
            // the emotion windows'; the chips' icons and window go
            // (`sub_801DACC`).
            self.chip_hud = Default::default();
            self.gauge.enabled = false;
            self.stop_emotion_windows();
            let win = self.fight.state == fight::WIN;
            self.round.winner = if win { self.round.local_side } else { self.round.local_side ^ 1 };
            // The winner's console plays the victory music; in link
            // battles the other one plays the defeat music.
            let special = self.setup.settings.effects & 2 != 0;
            let link = self.setup.settings.effects & effects::LINK != 0;
            // (Each console's music is its player's game's.)
            for side in 0..2 {
                let roles = self.side_roles(side);
                let winner = roles.music(if special { MusicRole::WinnerSpecial } else { MusicRole::Winner });
                let loser = roles.music(MusicRole::Loser);
                if side == self.round.winner {
                    self.play_sound_for(side, SoundCue::Music(winner));
                } else if link {
                    self.play_sound_for(side, SoundCue::Music(loser));
                }
            }
            self.fight.init = 4;
            // (A special battle's wait, and the win's in battle modes 4, 5
            // and 8, is the shorter: `sub_80081A4`, `sub_800825A`.)
            let wait = self.arena_rules().flow.result_wait;
            let short = special || (win && matches!(self.round.mode_copy, 4 | 5 | 8));
            self.fight.timer = if short { wait.special } else { wait.normal } as _;
            // Netbattle win/lose banners are the navi's; a round lost on
            // time (the judge's ruling) says "YOU LOSE" (`sub_800825A`).
            let navi = self.content.navi(self.stats[self.round.local_side as usize].navi);
            let id = match win {
                true => navi.win_banner,
                false if self.round_result() == 7 => BannerId(0x18),
                false => navi.lose_banner,
            };
            self.start_banner(id);
        }
        self.fight.timer -= 1;
        if self.banner.status() == BannerStatus::Done && self.fight.timer <= 0 {
            self.fight.result = if self.fight.state == fight::WIN { 1 } else { 2 };
        }
    }

    fn busting_level(&self) -> u8 {
        crate::kinds::busting_level(self)
    }

    // ---- End of round ------------------------------------------------------

    fn mode_fade_out(&mut self) {
        if self.round.init == 0 {
            // sub_80094DA: to white when the battle was won against one of
            // the Cybeasts (NameIDs 0x173..=0x17E; `sub_800A7A6` over side 1's actors,
            // `sub_800A832`'s result code 1), otherwise to black; either
            // takes 16 ticks.
            let bosses = self.round.alive_actors[1]
                .iter()
                .flatten()
                .filter(|&&r| self.content.identity(self.objects.get(r).identity).class.is_cybeast())
                .count();
            let white = bosses != 0 && self.round.result & 0xF == 1;
            self.fade.start(if white { FadeMode::EndToWhite } else { FadeMode::EndToBlack }, 0x10);
            self.round.init = 4;
            return;
        }
        if !self.fade.active() {
            self.play_sound(SoundCue::StopMusic);
            self.objects.free_all();
            self.round.top = top::END;
            self.round.mode = 0;
            self.round.sub = 0;
            self.round.init = 0;
        }
    }

    // ---- Per-tick helpers ----------------------------------------------------

    /// `sub_800FDC0`: expose each player's hand on their navi.
    fn update_player_hands(&mut self) {
        for side in 0..2 {
            for slot in 0..4 {
                let Some(r) = self.round.alive_actors[side][slot] else { continue };
                let Some(a) = self.objects.get(r).actor else { continue };
                if self.actors.get(a).actor_type != crate::actor::ActorType::Player {
                    continue;
                }
                let alliance = self.objects.get(r).alliance as usize;
                let hand = &self.hands[alliance];
                let (held, next) = (hand.remaining(), hand.next_chip());
                let o = self.objects.get_mut(r);
                o.chips_held = held;
                o.chip = next;
            }
        }
    }

    /// `sub_800AE90`: a warning marker on the HUD this tick, over the
    /// custom gauge or over the place `at` on the field, with `sound` on
    /// every 16th frame of the console's frame counter; on `console`'s HUD
    /// only, or on both.
    pub fn warn(&mut self, sound: impl Into<SoundCue>, at: Option<crate::object::Vec3>, console: Option<u8>) {
        let sound = sound.into();
        for side in 0..2u8 {
            if console.is_some_and(|c| c & 1 != side) {
                continue;
            }
            self.warnings[side as usize].push(crate::hud::Warning { at });
            if self.consoles[side as usize].frames & 0xF == 0 {
                self.play_sound_for(side, sound);
            }
        }
    }

    /// `sub_801DC7C(dx, dy)`: `console`'s HUD (or both) numbers `r`'s HP
    /// under it, `dx`, `dy` pixels from where it projects its position;
    /// `damage`: the damage it took instead, uncentered (see
    /// [`HpNumber`](crate::hud::HpNumber)). It takes the first free place;
    /// it gets none when one before the first free place has it already or
    /// when all four are taken. (A place freed before the one that has it
    /// is taken anew: the original stops at the first free place.)
    pub fn show_hp(&mut self, r: ObjectRef, dx: i8, dy: i8, damage: bool, console: Option<u8>) {
        let hp = self.objects.get(r).hp;
        for side in 0..2u8 {
            if console.is_some_and(|c| c & 1 != side) {
                continue;
            }
            for place in &mut self.hp_numbers[side as usize] {
                match place {
                    Some(n) if n.object == r => break,
                    Some(_) => {}
                    None => {
                        *place = Some(crate::hud::HpNumber { object: r, dx, dy, hp, damage });
                        break;
                    }
                }
            }
        }
    }

    /// `sub_801DD34`: no more HP number for `r` (the first place that has
    /// it, on each console).
    pub fn hide_hp(&mut self, r: ObjectRef) {
        for places in &mut self.hp_numbers {
            if let Some(place) = places.iter_mut().find(|p| p.is_some_and(|n| n.object == r)) {
                *place = None;
            }
        }
    }

    /// `sub_801E270`: the HUD says `message` for a second.
    pub(crate) fn show_message(&mut self, message: crate::hud::Message) {
        self.message = Some(crate::hud::MessageLine { message, ticks: crate::hud::MessageLine::SHOWN_TICKS });
    }

    /// `sub_801EB18(chip, damage, bonus)` on the other player's console:
    /// `side` used `chip`, whose name (with the attack's damage word and
    /// bonus, for a chip whose damage shows) that console shows for a
    /// second.
    pub(crate) fn show_used_chip(&mut self, side: u8, chip: ChipHandle, damage: u16, bonus: u16) {
        let shows_damage = self.content.chip(chip).flags.0 & crate::content::ChipFlags::HAS_DAMAGE != 0;
        let (damage, bonus) = if shows_damage { (damage, bonus) } else { (0, 0) };
        self.used_chips[side as usize & 1] = Some(crate::hud::UsedChip {
            chip,
            damage: damage & 0x7FF,
            doubled: damage & 0x8000 != 0,
            bonus: bonus & !0x7800,
            ticks: crate::hud::UsedChip::SHOWN_TICKS,
        });
    }

    fn run_hud_tasks(&mut self) {
        // sub_801C168: an HP number whose object's HP is 0 frees its place.
        // (An object freed with HP left would keep its place in the
        // original, which goes on reading the freed slot; here the place
        // goes with it. No netbattle object leaves so: LilBoiler takes its
        // number away as it goes.)
        for places in &mut self.hp_numbers {
            for place in places.iter_mut() {
                if place.is_some_and(|n| {
                    let o = self.objects.get(n.object);
                    o.hp == 0 || o.flags & crate::object::flags::ACTIVE == 0
                }) {
                    *place = None;
                }
            }
        }
        if self.gauge.enabled {
            self.fill_gauge();
        }
        self.low_hp_sound();
        // sub_801D1D8: the used chips' names run out.
        for used in &mut self.used_chips {
            if let Some(u) = used {
                u.ticks -= 1;
                if u.ticks == 0 {
                    *used = None;
                }
            }
        }
        // sub_801CA0C: so does the message.
        if let Some(m) = &mut self.message {
            m.ticks -= 1;
            if m.ticks == 0 {
                self.message = None;
            }
        }
        // While the custom screen is up the banner is the local player's
        // screen's (its Program Advance's), already stepped with it.
        let local = self.round.local_side as usize;
        if self.round.mode == mode::CUSTOM
            && let Some(s) = &self.custom.sides[local].screen
        {
            self.banner = s.hud;
            return;
        }
        self.banner.tick();
    }

    /// `sub_801C470`.
    fn fill_gauge(&mut self) {
        if self.paused || self.is_dimmed() || self.round.flags & battle_flags::GAUGE_FULL != 0 {
            return;
        }
        let v = self.gauge.value.wrapping_add(self.gauge.rate);
        self.gauge.value = v;
        if v >= CustomGauge::FULL {
            self.gauge.value = CustomGauge::FULL;
            if !self.late_turns() {
                self.set_flags(battle_flags::GAUGE_FULL);
                self.sound(SoundRole::GaugeFull);
            }
        }
    }

    fn update_linked_registry(&mut self) {
        crate::kinds::update_linked_registry(self);
    }

    /// `chip_800AEE8`: chips flagged for it recompute their damage every tick.
    fn refresh_variable_damage(&mut self) {
        for side in 0..2u8 {
            crate::hand::refresh_variable_damage(self, side);
        }
    }

    fn shift_damage_carry(&mut self) {
        crate::kinds::shift_damage_carry(self);
    }

    /// `sub_80102AC`: the NaviCust HP-drain bug, active only while that
    /// player's custom screen is open.
    fn custom_hp_drain(&mut self, side: u8) {
        if self.round.remote_status[side as usize] & 5 == 0 {
            return;
        }
        const PERIOD: [u8; 8] = [0, 40, 30, 20, 10, 5, 3, 2];
        let period = PERIOD[(self.stats[side as usize].bugs.custom_drain & 7) as usize];
        if period == 0 {
            return;
        }
        let Some(r) = self.player(side) else { return };
        if self.objects.get(r).hp <= 1 {
            return;
        }
        let Some(a) = self.objects.get(r).actor else { return };
        let ad = self.actors.get_mut(a);
        ad.drain_counter += 1;
        if ad.drain_counter >= period {
            ad.drain_counter = 0;
            crate::kinds::subtract_hp(self, r, 1);
        }
    }

    /// The HP box's alarm (`sub_801C840`, the HUD's task bit 7; sound
    /// only): every 45 ticks a console's own navi is at a quarter of its HP
    /// or less, while the battle is neither over nor paused, that console
    /// sounds 0x84. The count stops where the HP recovers.
    fn low_hp_sound(&mut self) {
        if self.is_battle_over() || self.paused {
            return;
        }
        for side in 0..2u8 {
            let Some(r) = self.player(side) else { continue };
            let o = self.objects.get(r);
            if o.hp > o.max_hp >> 2 {
                continue;
            }
            let ticks = &mut self.consoles[side as usize].low_hp_ticks;
            *ticks += 1;
            if *ticks >= LOW_HP_SOUND_TICKS {
                *ticks = 0;
                self.sound_for(side, SoundRole::LowHp);
            }
        }
    }

    /// `sub_8009158`: the low-HP music switch (sound only, but it keeps a
    /// latch in the round state). Each console switches for its own navi;
    /// the engine keeps both sides' latches.
    fn low_hp_music(&mut self) {
        if self.setup.settings.effects & effects::LINK == 0 {
            return;
        }
        for side in 0..2 {
            let Some(r) = self.player(side) else { continue };
            let o = self.objects.get(r);
            let low = o.hp <= o.max_hp / 4;
            let latch = &mut self.round.low_hp_music[side as usize];
            if low != *latch {
                *latch = low;
                self.play_sound_for(side, SoundCue::Pinch(low));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::testing;
    use crate::setup::Stage;

    /// Updates until a fade started now is done (the first update is the
    /// starter's own, before the frame's step).
    fn fade_updates(f: &mut Fade, mode: FadeMode, speed: u8) -> u32 {
        f.start(mode, speed);
        let mut n = 1;
        while f.active() {
            f.step();
            n += 1;
        }
        n
    }

    #[test]
    fn the_damage_judge_rolls_59_times_and_rules_on_damage_taken() {
        let mut b = Battle::new(testing::round_setup(testing::LINK_BATTLE, testing::stats(1000)), testing::content());
        b.round.local_side = 0;
        // Side 0 took more damage: the local side loses.
        b.start_judge(10, 20);
        assert_eq!(b.fight.judge.outcome, 2);
        let seed = b.rng;
        let mut ticks = 0;
        while b.step_judge() {
            ticks += 1;
        }
        // T+62 ..= T+274 run, T+275 reports.
        assert_eq!(ticks, 213);
        let mut expected = seed;
        for _ in 0..59 {
            expected.next();
        }
        assert_eq!(b.rng, expected);
        b.start_judge(20, 20);
        assert_eq!(b.fight.judge.outcome, 3);
        b.start_judge(20, 10);
        assert_eq!(b.fight.judge.outcome, 1);
    }

    /// docs/design/bn5-map.md §15.3 item 18: a custom screen asked for in
    /// the fight opens a tick sooner in BN5's flow (0x08007774 sets the
    /// result itself once the reversions are done) than in BN6's, which
    /// goes through state 0x24 first.
    #[test]
    fn bn5s_custom_request_opens_from_its_own_state() {
        let ticks = |sequencer_before_custom: bool| {
            let mut c: crate::content::Content = testing::build();
            c.define().unwrap_or_else(|e| panic!("{e}"));
            for rules in &mut c.rules {
                rules.flow.sequencer_before_custom = sequencer_before_custom;
            }
            let c = std::sync::Arc::new(c);
            let mut setup = testing::round_setup(testing::LINK_BATTLE, testing::megaman_on(&c));
            setup.content = c.hash();
            let mut b = Battle::new(setup, c);
            b.spawn_actors();
            b.run_objects();
            b.fight.state = fight::CUSTOM_REVERT;
            b.fight.init = 0;
            let mut n = 0;
            while b.fight.result != 6 {
                n += 1;
                assert!(n < 100, "the screen never opens");
                match b.fight.state {
                    fight::CUSTOM_REVERT => b.fight_custom_revert(),
                    fight::CUSTOM_SEQUENCE => b.fight_custom_sequence(),
                    s => panic!("state {s:#x}"),
                }
            }
            (n, b.fight.state)
        };
        let (bn6, bn6_state) = ticks(true);
        let (bn5, bn5_state) = ticks(false);
        assert_eq!((bn5_state, bn6_state), (fight::CUSTOM_REVERT, fight::CUSTOM_SEQUENCE));
        assert!(bn5 < bn6, "BN5 {bn5} ticks, BN6 {bn6}");
    }

    #[test]
    fn screen_fades_keep_their_level() {
        let mut f = Fade::default();
        // The intro clears the faded screen in 17 updates.
        assert_eq!(fade_updates(&mut f, FadeMode::IntroFromBlack, 0x10), 18);
        assert_eq!(f.level, 0);
        // A dim from a clear screen: 16 steps; a counter cut-in's dim from
        // the dimmed screen: 1.
        assert_eq!(fade_updates(&mut f, FadeMode::Dim, 4), 17);
        assert_eq!(f.level, 0x40);
        assert_eq!(fade_updates(&mut f, FadeMode::Dim, 4), 2);
        // The undim: its first step holds the level.
        f.start(FadeMode::Undim, 4);
        assert_eq!(f.remaining(), 17);
        assert_eq!(fade_updates(&mut f, FadeMode::Undim, 4), 18);
        assert_eq!(f.level, 0);
        f.start(FadeMode::TransformOut, 0x10);
        assert_eq!(f.remaining(), 16);
    }

    /// A best-of-three netbattle round about to leave its end state, with
    /// the set standing at `wins`-`losses` after `round` rounds.
    fn ending(wins: u8, losses: u8, round: u8) -> Battle {
        let mut setup = testing::round_setup(testing::LINK_BATTLE, testing::stats(1000));
        setup.settings.effects = 0xE8C;
        let content = testing::content();
        setup.later_stages = [
            Stage { stage: content.stage_by_key(testing::ROCK_BATTLE), background: crate::content::BackgroundId(3) },
            Stage { stage: content.stage_by_key(testing::LINK_BATTLE_SIDE0_FIRST), background: crate::content::BackgroundId(0x13) },
        ];
        let mut b = Battle::new(setup, testing::content());
        let r = &mut b.round;
        (r.top, r.mode, r.sub, r.init) = (top::END, 4, 0, 0);
        (r.wins, r.losses, r.round, r.max_combo) = (wins, losses, round, 1);
        r.result = if wins > losses { 1 } else { 2 };
        b.paused = true;
        b
    }

    fn tick(b: &mut Battle) {
        b.tick(&[PlayerTick::default(), PlayerTick::default()], TickEvents::default());
    }

    #[test]
    fn an_undecided_set_chains_its_next_round_on_the_drawn_stage() {
        let mut b = ending(1, 0, 1);
        tick(&mut b);
        let Some(RoundEnd::NextRound { settings, score }) = b.round_end() else { panic!("{:?}", b.round_end()) };
        // The drawn table entry, with this round's effects and the drawn
        // background.
        let drawn = testing::content().stage_by_key(testing::ROCK_BATTLE);
        assert_eq!(*settings, BattleSettings { stage: drawn, effects: 0xE8C, background: crate::content::BackgroundId(3) });
        assert_eq!(*score, SetScore { wins: 1, losses: 0, round: 1, max_combo: 1 });
        assert_eq!(b.round.top, top::INIT);
        assert_eq!(b.sound_cues(), [SoundCue::StopMusic]);
    }

    #[test]
    fn a_decided_set_ends_the_battle() {
        let mut b = ending(2, 0, 2);
        tick(&mut b);
        assert_eq!(b.round_end(), Some(&RoundEnd::Over(BattleResult::Won)));
        assert_eq!((b.round.top, b.round.mode, b.round.running, b.paused), (top::END, 4, 0, false));
        tick(&mut b);
        assert_eq!(b.sound_cues(), [], "the battle ends once");

        let mut b = ending(1, 2, 3);
        tick(&mut b);
        assert_eq!(b.round_end(), Some(&RoundEnd::Over(BattleResult::Lost)));
        let mut b = ending(1, 1, 3);
        b.round.result = 1;
        tick(&mut b);
        assert_eq!((b.round_end(), b.round.result), (Some(&RoundEnd::Over(BattleResult::Drawn)), 3));
    }

    #[test]
    fn a_latched_low_hp_switch_plays_no_pinch_cue_on_the_first_tick() {
        let mut setup = testing::round_setup(testing::LINK_BATTLE, testing::stats(500));
        setup.low_hp_music_latched = true;
        let mut b = Battle::new(setup, testing::content());
        tick(&mut b);
        assert_eq!(b.sound_cues(), [SoundCue::Music(b.arena_roles().music(MusicRole::LinkBattle))]);
        tick(&mut b);
        assert_eq!(b.sound_cues(), [SoundCue::Pinch(false)]);
    }

    #[test]
    fn a_warning_sounds_on_a_consoles_sixteenth_frames() {
        let mut b = Battle::new(testing::round_setup(testing::LINK_BATTLE, testing::stats(500)), testing::content());
        (b.consoles[0].frames, b.consoles[1].frames) = (15, 3);
        tick(&mut b);
        assert_eq!((b.consoles[0].frames, b.consoles[1].frames), (16, 4));
        // (Any sound: content names the marker's own.)
        let id = b.arena_roles().sound(SoundRole::Pause);
        let sound = SoundCue::from(id);
        let heard = |b: &Battle, side: u8| b.sound_cues_for(side).iter().filter(|&&c| c == sound).count();
        // Over the gauge, on both consoles: only the one on a 16th frame
        // sounds it.
        b.warn(id, None, None);
        assert_eq!((b.warnings[0].len(), b.warnings[1].len()), (1, 1));
        assert_eq!((heard(&b, 0), heard(&b, 1)), (1, 0));
        // Over a place, on one console: every call sounds on such a frame.
        let at = crate::object::Vec3 { x: 20 << 16, y: 12 << 16, z: 0 };
        b.warn(id, Some(at), Some(0));
        assert_eq!(b.warnings[0][1].at, Some(at));
        assert_eq!((b.warnings[0].len(), b.warnings[1].len(), heard(&b, 0)), (2, 1, 2));
        // The markers last the tick.
        tick(&mut b);
        assert!(b.warnings.iter().all(Vec::is_empty));
    }

    #[test]
    fn a_console_sounds_every_45_ticks_its_navi_is_low() {
        let mut b = Battle::new(testing::round_setup(testing::LINK_BATTLE, testing::stats(500)), testing::content());
        tick(&mut b);
        tick(&mut b);
        let navi = b.player(0).expect("side 0's navi");
        b.objects.get_mut(navi).hp = 125;
        b.paused = false;
        let alarm = SoundCue::from(b.arena_roles().sound(SoundRole::LowHp));
        let heard = |b: &Battle, side: u8| b.sound_cues_for(side).iter().filter(|&&c| c == alarm).count();
        for _ in 0..44 {
            b.low_hp_sound();
        }
        assert_eq!(heard(&b, 0), 0);
        // Not while paused; then on the 45th tick, on its own console only.
        b.paused = true;
        b.low_hp_sound();
        assert_eq!(heard(&b, 0), 0);
        b.paused = false;
        b.low_hp_sound();
        assert_eq!((heard(&b, 0), heard(&b, 1)), (1, 0));
        // Above a quarter the count waits.
        b.objects.get_mut(navi).hp = 126;
        for _ in 0..90 {
            b.low_hp_sound();
        }
        assert_eq!(heard(&b, 0), 1);
    }
}
