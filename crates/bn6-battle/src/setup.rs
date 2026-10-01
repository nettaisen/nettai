//! What a round starts from: the battle settings, both navis' stats, the
//! shared RNG seed, and the set score carried between rounds.

use crate::content::{Content, ContentHash};
use bn6_content_api::{FormHandle, NaviHandle, StageHandle, WeaponHandle};

/// A round's battle settings: its stage, and what the round sets over the
/// stage's record (the background a set's later rounds draw, the effects
/// the battle runs with).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BattleSettings {
    pub stage: StageHandle,
    pub background: u8,
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

/// A stage's battle settings record (the game's 16-byte BattleSettings,
/// `BattleSettingsList1`'s entries).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StageSettings {
    /// Panel layout (`Content::panel_layout`).
    pub layout: u8,
    pub music: u8,
    /// Battle mode (0 = netbattle).
    pub mode: u8,
    pub background: u8,
    pub battle_number: u8,
    /// Panel column pattern (which columns belong to which side).
    pub panel_pattern: u8,
    /// `effects` bits (see `effects`).
    pub effects: u32,
    /// Who and what spawns where.
    pub actors: ActorListId,
}

/// Battle effects bits.
pub mod effects {
    /// Link battle between two players.
    pub const LINK: u32 = 0x8;
    /// Multi-round set.
    pub const SET: u32 = 0x400;
    /// Random battle.
    pub const RANDOM: u32 = 0x20_0000;
}

/// An entry of a battle's actor list: something placed on the field when
/// the round starts (`sub_8007368`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ActorEntry {
    /// What to spawn.
    pub kind: ActorKind,
    /// The side, for navis. Rocks take the side of their panel instead.
    pub alliance: u8,
    /// Panel.
    pub x: u8,
    pub y: u8,
}

/// What an actor-list entry spawns. These are the kinds the game's lists
/// use; the spawn loop (`off_80073A0`) knows a few more.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActorKind {
    /// A player navi (`sub_80073CC`).
    Navi,
    /// A rock (attack object #0x59, `sub_80074FA`), placed at the start
    /// (by content: objects/rock).
    Rock {
        /// Which rock (`ObjectData::rocks`).
        variant: u8,
    },
    /// A boulder (attack object #0x6E, `sub_8007450`), kept in the
    /// field-object registry's stage slots (by content: objects/boulder).
    Object6E,
    /// A Guardian statue (attack object #0x7D, `sub_800751C`), placed at
    /// the start (by content: chips/guardian). Its `variant` is the
    /// entry's argument, which only reaches the position its spawner
    /// leaves before its init.
    Object7D { variant: u8 },
}

impl ActorKind {
    /// The entry's type: its spawn routine in `off_80073A0`.
    pub fn entry_type(self) -> u8 {
        match self {
            ActorKind::Navi => 0,
            ActorKind::Object6E => 3,
            ActorKind::Rock { .. } => 8,
            ActorKind::Object7D { .. } => 9,
        }
    }

    /// The entry's argument (a rock's variant).
    pub fn variant(self) -> u8 {
        match self {
            ActorKind::Rock { variant } | ActorKind::Object7D { variant } => variant,
            ActorKind::Navi | ActorKind::Object6E => 0,
        }
    }
}

/// A battle's actor list (`Stages::actor_lists`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActorList {
    /// The address the original's battle settings records name the list
    /// by: what link data and traces carry.
    pub original_address: u32,
    pub entries: Vec<ActorEntry>,
}

/// An actor list: its index in the content's `Stages::actor_lists`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ActorListId(pub u8);

impl<'a> IntoIterator for &'a ActorList {
    type Item = &'a ActorEntry;
    type IntoIter = std::slice::Iter<'a, ActorEntry>;
    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

/// A navi (NaviStats+0x29): MegaMan, or one of the link navis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Navi(pub u8);

impl Navi {
    pub const MEGAMAN: Navi = Navi(0);

    /// Index into per-navi tables.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// MegaMan's Cross / Beast form (NaviStats+0x2C, "Transformation").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Form(pub u8);

impl Form {
    pub const NONE: Form = Form(0);
    /// Crosses are 1..=10: Gregar's HeatCross, ElecCross, SlashCross,
    /// EraseCross and ChargeCross, then Falzar's SpoutCross, TomahawkCross,
    /// TenguCross, GroundCross and DustCross.
    pub const CHARGE_CROSS: Form = Form(5);
    pub const DUST_CROSS: Form = Form(0x0A);
    pub const GREGAR_BEAST: Form = Form(0x0B);
    pub const FALZAR_BEAST: Form = Form(0x0C);
    /// Cross + Beast forms are 0x0D..=0x16.
    pub const GREGAR_BEAST_OVER: Form = Form(0x17);
    pub const FALZAR_BEAST_OVER: Form = Form(0x18);

    /// Index into per-form tables.
    pub fn index(self) -> usize {
        self.0 as usize
    }

    /// Beast Out, with or without a cross, or Beast Over.
    pub fn is_beast(self) -> bool {
        (0x0B..=0x18).contains(&self.0)
    }

    /// Beast Over (the navi acts on its own).
    pub fn is_beast_over(self) -> bool {
        self.0 >= 0x17
    }
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

/// The navi's weapons (`off_80117D4`'s routines; the byte 0xFF is none).
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
    /// +0x4D: the buster shot's projectile config (applied on an RNG roll).
    pub buster_shot: u8,
    /// +0x4F: the charged shot's projectile config (0 = default 6).
    pub charge_shot_kind: u8,
}

/// NaviCust bugs and program side effects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct NaviCustBugs {
    /// +0x11: random repeat steps after a move.
    pub auto_step: u8,
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
/// the bytes the engine uses are modeled; bn6-compat's codec knows the
/// block's layout (the field comments give each one's offset).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct NaviStats {
    /// +0x01: buster attack level.
    pub attack: u8,
    /// +0x02: buster rapid level.
    pub rapid: u8,
    /// +0x03: buster charge level.
    pub charge: u8,
    /// +0x06: the barrier the navi starts with (0 = none).
    pub first_barrier: u8,
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
    /// +0x20: the navi's game (0 Gregar, 1 Falzar): MstrCros picks its
    /// Crosses by it.
    pub version: u8,
    /// +0x21
    pub beast_out_counter: u8,
    /// +0x22: fighting outdoors in the sun (some chips hit harder).
    pub sun: bool,
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
    /// +0x60: NaviCust ChpShufl (a custom-screen button re-deals).
    pub chip_shuffle: bool,
    /// +0x61: NaviCust NumbrOpn (the custom screen deals 10 chips).
    pub number_open: bool,
    pub weapons: NaviWeapons,
    pub bugs: NaviCustBugs,
}

impl NaviStats {
    /// A hit's bug code can name any stat byte below 0x64 by its offset
    /// and set it (`sub_80139F6`): the field at that offset takes the
    /// byte (a halfword's low or high byte; a flag is set by any nonzero
    /// byte; a navi, form or weapon byte names the pack's by number).
    /// Offsets the engine doesn't model are not supported.
    pub fn set_byte_by_bug_code(&mut self, offset: u8, value: u8, content: &Content) {
        let weapon = |v: u8| (v != 0xFF).then(|| content.weapon_numbered(v));
        let flag = value != 0;
        let low = |w: &mut u16| *w = (*w & 0xFF00) | value as u16;
        let high = |w: &mut u16| *w = (*w & 0x00FF) | (value as u16) << 8;
        let w = &mut self.weapons;
        let g = &mut self.bugs;
        match offset {
            0x01 => self.attack = value,
            0x02 => self.rapid = value,
            0x03 => self.charge = value,
            0x04 => w.buster = weapon(value),
            0x05 => w.charge_shot = weapon(value),
            0x06 => self.first_barrier = value,
            0x07 => w.back_special = weapon(value),
            0x08 => {
                self.gauge_speed = match value {
                    0 => GaugeSpeed::Normal,
                    1 => GaugeSpeed::Fast,
                    2 => GaugeSpeed::Slow,
                    v => panic!("bug code sets the gauge speed to {v}, which is not modeled"),
                }
            }
            0x09 => self.reg_up = value,
            0x0A => self.custom_level = value,
            0x0B => self.mega_level = value,
            0x0C => self.giga_level = value,
            0x0D => {
                self.support = (value != 0xFF).then_some(Supports {
                    rush: value & 1 != 0,
                    beat: value & 2 != 0,
                    tango: value & 4 != 0,
                })
            }
            0x0E => self.mood = value,
            0x10 => self.element = value,
            0x11 => g.auto_step = value,
            0x12 => g.panel_trail_kind = value,
            0x13 => g.panel_trail_level = value,
            0x14 => g.buster_blanks = value,
            0x15 => g.buster_charged = value,
            0x16 => g.hit_status = value,
            0x17 => self.starting_form = content.form_numbered(Form(value)),
            0x18 => g.hp_drain = value,
            0x19 => g.custom_drain = value,
            0x1A => g.battle_start = value,
            0x1B => self.float_shoes = flag,
            0x1C => self.air_shoes = flag,
            0x1D => self.undershirt = flag,
            0x20 => self.version = value,
            0x21 => self.beast_out_counter = value,
            0x22 => self.sun = flag,
            0x23 => self.super_armor = flag,
            0x24 => g.emotion = value,
            0x29 => self.navi = content.navi_numbered(Navi(value)),
            0x2B => self.navi_variant = value,
            0x2C => self.form = content.form_numbered(Form(value)),
            0x2D => self.folder = value,
            0x2E => self.folder_reg[0] = value,
            0x2F => self.folder_reg[1] = value,
            0x31 => g.processing = value,
            0x39 => w.a_charge = weapon(value),
            0x3D => g.starting_damage = value,
            0x3E => low(&mut self.max_base_hp),
            0x3F => high(&mut self.max_base_hp),
            0x40 => low(&mut self.hp),
            0x41 => high(&mut self.hp),
            0x42 => low(&mut self.max_hp),
            0x43 => high(&mut self.max_hp),
            0x44 => w.mode9_a = weapon(value),
            0x4D => w.buster_shot = value,
            0x4F => w.charge_shot_kind = value,
            0x50 => low(&mut self.chip_recovery),
            0x51 => high(&mut self.chip_recovery),
            0x52 => g.status_immunity = flag,
            0x54 => low(&mut g.custom_damage),
            0x55 => high(&mut g.custom_damage),
            0x56 => self.folder_tags[0][0] = value,
            0x57 => self.folder_tags[0][1] = value,
            0x58 => self.folder_tags[1][0] = value,
            0x59 => self.folder_tags[1][1] = value,
            0x60 => self.chip_shuffle = flag,
            // The game tests the byte for 1.
            0x61 => self.number_open = value == 1,
            0x63 => g.hand_shrink_turn = value,
            _ => panic!("bug code writes NaviStats+{offset:#x}, which is not modeled"),
        }
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

/// Where a later round of a set is fought: a stage and the background to
/// show (one pair of `byte_203CA50`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Stage {
    pub stage: StageHandle,
    pub background: u8,
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
    /// The stages of the set's next rounds, as player 0 drew them for
    /// this round's init exchange: when round `n` ends and the set goes
    /// on, round `n + 1` is fought on `later_stages[n - 1]`.
    pub later_stages: [Stage; 2],
    /// The local side's low-HP music latch starts set. The round's init counts the
    /// frames it waits for the link in the halfword the latch later uses
    /// (BattleState+0x20), so a round whose init had to wait starts with
    /// it set, and its first tick plays no pinch cue.
    pub low_hp_music_latched: bool,
    /// Per side, from the save via the init exchange: how fast each SP
    /// navi was deleted (`byte_203EB00`). The SP navi chips' damage goes
    /// by it.
    pub sp_times: [SpTimes; 2],
    /// Per side: the battle folder and what the save unlocks on the
    /// custom screen.
    pub players: [crate::custom::PlayerSetup; 2],
    /// Ticks from a player's packet going out to its arriving (the link's
    /// latency; `link::Link::RECORDED_DELAY` in the recorded sessions).
    pub link_delay: u8,
}

/// How fast (in frames) a player deleted each SP navi (20 halfwords).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpTimes(pub [u16; 20]);

impl Default for SpTimes {
    /// Every SP navi deleted in no time (the best damage).
    fn default() -> SpTimes {
        SpTimes([0; 20])
    }
}

impl SpTimes {
    /// The frames SP navi chip `n` (formula `n + 1`) took to delete.
    pub fn frames(&self, n: usize) -> u16 {
        self.0[n]
    }
}

impl RoundSetup {
    /// The settings of the set's next round, fought on `stage` after this
    /// one (`battleSettings_802D2B2`): that table entry, with this
    /// round's effects and the stage's background.
    pub fn next_settings(&self, stage: Stage) -> BattleSettings {
        BattleSettings { stage: stage.stage, background: stage.background, effects: self.settings.effects }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bug_code_writes_a_named_stat() {
        let content = crate::content::testing::content();
        let mut s = NaviStats { max_hp: 1000, ..Default::default() };
        s.set_byte_by_bug_code(0x1D, 1, &content);
        assert!(s.undershirt);
        s.set_byte_by_bug_code(0x16, 2, &content);
        assert_eq!(s.bugs.hit_status, 2);
        s.set_byte_by_bug_code(0x43, 0x01, &content);
        assert_eq!(s.max_hp, 0x01E8);
        s.set_byte_by_bug_code(0x05, 0xFF, &content);
        assert_eq!(s.weapons.charge_shot, None);
    }
}
