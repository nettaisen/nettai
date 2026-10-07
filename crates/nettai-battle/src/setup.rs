//! What a round starts from: the battle settings, both navis' stats, the
//! shared RNG seed, and the set score carried between rounds.

use crate::content::{Content, ContentHash};
use nettai_content_api::{FormHandle, NaviHandle, RecordHandle, StageHandle, WeaponHandle};

/// A round's battle settings: its stage, and what the round sets over the
/// stage's record (the background a set's later rounds draw, the effects
/// the battle runs with).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BattleSettings {
    pub stage: StageHandle,
    pub background: crate::content::BackgroundId,
    /// `effects` bits (see `effects`).
    pub effects: u32,
}

impl BattleSettings {
    /// A round on `stage` as its record gives it (its background and
    /// effects).
    pub fn on(content: &Content, stage: StageHandle) -> BattleSettings {
        let s = content.stage(stage);
        BattleSettings { stage, background: s.background, effects: s.effects }
    }
}

/// Battle effects bits.
pub mod effects {
    /// A ranked boss (`BATTLE_EFFECT_BOSS_RANK`).
    pub const BOSS_RANK: u32 = 0x1;
    /// Link battle between two players.
    pub const LINK: u32 = 0x8;
    /// Multi-round set.
    pub const SET: u32 = 0x400;
    /// Random battle.
    pub const RANDOM: u32 = 0x20_0000;
}

/// Custom gauge speed (NaviStats+0x08).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GaugeSpeed {
    #[default]
    Normal = 0,
    Fast = 1,
    Slow = 2,
}

/// The NaviCust supports, which act once in link battles (NaviStats+0x0D
/// bits; the byte is 0xFF when there are none).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Supports {
    /// Bit 0: Rush cancels one chip with `flags2 & 2`.
    pub rush: bool,
    /// Bit 1: Beat cancels one Mega or Giga chip.
    pub beat: bool,
    /// Bit 2: Tango heals once when HP drops to a quarter.
    pub tango: bool,
}

/// The navi's weapons (the original's bytes name `off_80117D4`'s routines;
/// 0xFF is none).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct NaviWeapons {
    /// +0x04: the B-button buster.
    pub buster: Option<WeaponHandle>,
    /// +0x05: the charged shot.
    pub charge_shot: Option<WeaponHandle>,
    /// +0x07: the B+Back special.
    pub back_special: Option<WeaponHandle>,
    /// +0x39: the A-button charge (charged chips).
    pub a_charge: Option<WeaponHandle>,
    /// +0x44: the A button in battle mode 9.
    pub mode9_a: Option<WeaponHandle>,
    /// +0x4D: the buster shot's program: the projectile variant a shot is
    /// on a lucky draw (none: the byte 0).
    pub buster_shot: Option<RecordHandle>,
    /// +0x4F: the charged shot's program, likewise (none: the byte 0, the
    /// plain charged shot).
    pub charge_shot_kind: Option<RecordHandle>,
    /// +0x48: the damage a B+Back special takes from the navi's stats
    /// (ProtoMan's reflecting guard, weapon routine 0x30; from the navi's
    /// starting row).
    pub back_special_damage: u16,
}

/// NaviCust bugs and program side effects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct NaviCustBugs {
    /// +0x11: random repeat steps after a move.
    pub auto_step: u8,
    /// The keys the navi steps by when none is held, or while sliding
    /// (EXE4's NaviStats +0x0D, which DrkSword's and DarkBomb's costs and a
    /// Mod Card's bug set: the joypad's direction bits, 0x10 to 0x80, read
    /// in the rules' order of keys and never turned by confusion; 0 and
    /// 0xFF none).
    pub idle_step_keys: u8,
    /// +0x12: what a step leaves on the panel behind (1 break, 3 crack...).
    pub panel_trail_kind: u8,
    /// +0x13: how often a step leaves it (0 = never).
    pub panel_trail_level: u8,
    /// +0x14: buster slots (of 16) that fire a blank shot.
    pub buster_blanks: u8,
    /// +0x15: buster slots (of 16) that fire a charged shot.
    pub buster_charged: u8,
    /// +0x16: status taken on a hit (1 blind, 2 confusion, 3 HP bug + 1).
    pub hit_status: u8,
    /// +0x18: HP drain during the fight (level 0..7).
    pub hp_drain: u8,
    /// +0x19: HP drain while the custom screen is open (level 0..7).
    pub custom_drain: u8,
    /// +0x1A: a battle-start hook (9 and 10 pick a random variant).
    pub battle_start: u8,
    /// +0x24: emotion swings (with the Beast Out counter).
    pub emotion: u8,
    /// +0x31: steps go astray.
    pub processing: u8,
    /// +0x3D: damage at battle start (never kills).
    pub starting_damage: u8,
    /// +0x52: statuses are not applied.
    pub status_immunity: bool,
    /// +0x54: damage at custom-screen open (never kills).
    pub custom_damage: u16,
    /// +0x63: from this custom screen of a round on, each deals one
    /// chip fewer per screen (0 = never).
    pub hand_shrink_turn: u8,
}

/// A navi's in-battle stats (the game's 0x64-byte NaviStats block). Only
/// the bytes the engine uses are modeled, and the game's own beside them
/// (`game`); the compat crates' codecs know the block's layout (the field
/// comments give each one's offset).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct NaviStats {
    /// +0x01: buster attack level.
    pub attack: u8,
    /// +0x02: buster rapid level.
    pub rapid: u8,
    /// +0x03: buster charge level.
    pub charge: u8,
    /// +0x06: the barrier the navi starts with (a `barrier` record of
    /// lib/barriers; the byte 0: none).
    pub first_barrier: Option<RecordHandle>,
    /// +0x08
    pub gauge_speed: GaugeSpeed,
    /// +0x09
    pub reg_up: u8,
    /// +0x0A
    pub custom_level: u8,
    /// +0x0B / +0x0C
    pub mega_level: u8,
    pub giga_level: u8,
    /// +0x0D: None when the byte is 0xFF.
    pub support: Option<Supports>,
    /// +0x0E: 0 worn out, 0x80 normal, 0xFF Full Synchro; hits wear it
    /// down.
    pub mood: u8,
    /// +0x10: MegaMan's base element byte (primary | secondary bits).
    pub element: u8,
    /// +0x17: the form at battle start (copied to `form` at init).
    pub starting_form: FormHandle,
    /// +0x1B / +0x1C / +0x1D / +0x23
    pub float_shoes: bool,
    pub air_shoes: bool,
    pub undershirt: bool,
    pub super_armor: bool,
    /// +0x29
    pub navi: NaviHandle,
    /// +0x2B: a per-navi variant (selects its move lag).
    pub navi_variant: u8,
    /// +0x2C
    pub form: FormHandle,
    /// +0x2D: the folder the navi brings (0-2).
    pub folder: u8,
    /// +0x2E / +0x2F: the folders' regular chips.
    pub folder_reg: [u8; 2],
    /// +0x3E / +0x40 / +0x42
    pub max_base_hp: u16,
    pub hp: u16,
    pub max_hp: u16,
    /// +0x50: HP healed per chip used.
    pub chip_recovery: u16,
    /// +0x56..+0x59: the folders' tag chips.
    pub folder_tags: [[u8; 2]; 2],
    pub weapons: NaviWeapons,
    pub bugs: NaviCustBugs,
    /// The stats of the navi's game's own (its rules' `stats`: EXE6's
    /// Beast Out turns and NaviCust ChpShufl, EXE5's Hub Style, ...), by
    /// the rules' schema: what the rules read and write by name, and the
    /// compat crates map to the block's bytes. The engine reads none of them
    /// but by a role ([`crate::content::StatRole`]).
    pub game: nettai_content_api::SmallBlock,
}

impl NaviStats {
    /// `init_8013B64` (EXE5's 0x080111AA): `navi`'s stats, fresh: what the
    /// game's routine writes for every navi
    /// (`initNaviStats_WithDefaultStatsMaybe_8013438`, EXE5's 0x08010C00:
    /// the game's rule section `fresh_stats`, and the block's empty values)
    /// with what the navi comes with (`byte_80210DD`'s row: the navi's
    /// `fresh` and `weapons`). What a navi switch brings a link navi with,
    /// and what MegaMan's NaviCust starts from (`sub_8136C24`). None for a
    /// navi without a row. No value here is a game's: each is the game's
    /// rules' or the navi's, or none.
    pub fn fresh(navi: NaviHandle, content: &Content) -> Option<NaviStats> {
        let data = content.navi(navi);
        let fresh = data.fresh?;
        let game = content.rules().fresh_stats;
        let defaults = NaviStats::default();
        let base = content.base_form_for(navi);
        Some(NaviStats {
            reg_up: game.reg_up,
            custom_level: game.custom_level,
            mood: game.mood,
            // (The game's own: what its `fresh_stats` gives them, the rest
            // zero.)
            game: game.stats,
            // The block's empty values: the support byte there with no
            // support on (0xFF is no byte), the first folder, no Regular
            // chip and no tag chips.
            support: Some(Default::default()),
            form: base,
            starting_form: base,
            folder: 0,
            folder_reg: [0xFF; 2],
            folder_tags: [[0xFF; 2]; 2],
            navi,
            max_base_hp: fresh.hp,
            hp: fresh.hp,
            max_hp: fresh.hp,
            super_armor: fresh.super_armor,
            float_shoes: fresh.float_shoes,
            air_shoes: fresh.air_shoes,
            undershirt: fresh.undershirt,
            first_barrier: fresh.first_barrier,
            mega_level: fresh.mega_level,
            giga_level: fresh.giga_level,
            weapons: NaviWeapons {
                buster: data.weapons.buster,
                charge_shot: data.weapons.charge_shot,
                back_special: data.weapons.back_special,
                a_charge: data.weapons.a_charge,
                back_special_damage: fresh.back_special_damage,
                mode9_a: game.mode9_a,
                // (No shot programs: the bytes 0.)
                ..defaults.weapons
            },
            // (No panel trail: the byte 0xFF.)
            bugs: NaviCustBugs { panel_trail_kind: 0xFF, ..defaults.bugs },
            ..defaults
        })
    }

    /// The game's own stat `name` (its rules' `stats`), if the game has it.
    pub fn game_stat(&self, content: &Content, name: &str) -> Option<nettai_content_api::FieldValue> {
        let schema = content.defs.schema(content.defs.rules()?.stats);
        (self.game.id() == content.defs.rules()?.stats).then_some(())?;
        Some(self.game.get(schema, schema.index_of(name)?))
    }

    /// Set the game's own stat `name` (its rules' `stats`); refused for a
    /// stat the game hasn't, or a value its type doesn't take.
    pub fn set_game_stat(&mut self, content: &Content, name: &str, v: nettai_content_api::Value) -> Result<(), String> {
        let rules = content.defs.rules().ok_or("the content has no rules")?;
        let schema = content.defs.schema(rules.stats);
        if self.game.id() != rules.stats {
            self.game = nettai_content_api::SmallBlock::new(rules.stats, schema).ok_or("the rules' stats are too large")?;
        }
        let i = schema.index_of(name).ok_or_else(|| format!("the game's stats have no `{name}`"))?;
        self.game.set(schema, i, v).map_err(|e| format!("{name}: {e}"))
    }
}

/// Wins/losses/round/max combo, carried between the rounds of a set
/// (from the local side's perspective).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SetScore {
    pub wins: u8,
    pub losses: u8,
    pub round: u8,
    pub max_combo: u8,
}

/// Where a round of a set is fought: a stage and the background to show
/// (a later round's is one pair of `byte_203CA50`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Stage {
    pub stage: StageHandle,
    pub background: crate::content::BackgroundId,
}

/// Everything a round starts from.
#[derive(Clone, Debug, Hash)]
pub struct RoundSetup {
    /// The content the round runs on ([`Content::hash`]). `Battle::new`
    /// checks it against the content it is given; netplay peers whose
    /// setups agree run the same content.
    pub content: ContentHash,
    pub settings: BattleSettings,
    /// Both navis' stats, by side.
    pub navi_stats: [NaviStats; 2],
    /// The simulation RNG's state (both consoles agree on it).
    pub rng: u32,
    /// The console the simulation reproduces (0 = left). The original
    /// keeps a little per-console state (who fades in at the intro, what
    /// a blinded player sees, the chip-name and result banners, some
    /// sounds, the result as won/lost), and the engine keeps it for this
    /// side. In netplay both peers must use the same value, whichever
    /// side they present: it is part of the shared setup, not the viewer
    /// (docs/design/rollback.md).
    pub local_side: u8,
    pub score: SetScore,
    /// The stages of the set's rounds after the first, in order (the
    /// original's, which player 0 drew for the round's init exchange, are
    /// two: a triple battle's): when round `n` ends and the set goes on,
    /// round `n + 1` is fought on `later_stages[n - 1]`. The set is of one
    /// round more than it lists ([`RoundSetup::rounds`]).
    pub later_stages: Vec<Stage>,
    /// The local side's low-HP music latch starts set. The round's init counts the
    /// frames it waits for the link in the halfword the latch later uses
    /// (BattleState+0x20), so a round whose init had to wait starts with
    /// it set, and its first tick plays no pinch cue.
    pub low_hp_music_latched: bool,
    /// Per side: the battle folder and what the save unlocks on the
    /// custom screen.
    pub players: [crate::custom::PlayerSetup; 2],
}

impl RoundSetup {
    /// The set's rounds: the first and those it lists after it (a set of
    /// one round lists none).
    pub fn rounds(&self) -> usize {
        1 + self.later_stages.len()
    }

    /// The settings of the set's next round, fought on `stage` after this
    /// one (`battleSettings_802D2B2`): that table entry, with this
    /// round's effects and the stage's background.
    pub fn next_settings(&self, stage: Stage) -> BattleSettings {
        BattleSettings { stage: stage.stage, background: stage.background, effects: self.settings.effects }
    }
}

