//! What a round starts from: the battle settings, both navis' stats, the
//! shared RNG seed, and the set score carried between rounds.

/// Battle settings (the game's 16-byte BattleSettings record).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BattleSettings {
    /// Panel layout index.
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
    pub actors: &'static ActorList,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorKind {
    /// A player navi (`sub_80073CC`).
    Navi,
    /// A rock (attack object #0x59, `sub_80074FA`), placed at the start.
    Rock {
        /// Which rock (`data::ROCKS`).
        variant: u8,
    },
    /// Attack object #0x6E, kept in the field-object registry's stage
    /// slots (`sub_8007450`).
    Object6E,
    /// Attack object #0x7D (`sub_800751C`).
    Object7D { variant: u8 },
}

/// A battle's actor list, as found in the game's battle settings table.
#[derive(Debug, PartialEq, Eq)]
pub struct ActorList {
    /// Identifies the list: the game's address for it, which battle
    /// settings (and so traces and replays) carry.
    pub source: u32,
    pub entries: &'static [ActorEntry],
}

impl ActorList {
    /// The actor list a battle settings record refers to.
    pub fn find(source: u32) -> Option<&'static ActorList> {
        crate::data::ACTOR_LISTS.iter().find(|l| l.source == source)
    }
}

impl<'a> IntoIterator for &'a ActorList {
    type Item = &'a ActorEntry;
    type IntoIter = std::slice::Iter<'a, ActorEntry>;
    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

impl BattleSettings {
    /// Settings from their 16-byte encoding. Bytes 12..16 identify the
    /// actor list. Byte 1 (read by `GetBattleSettingsUnk01`, outside the
    /// battle simulation) and byte 7 (no reader found) are not kept.
    pub fn netbattle_from_bytes(b: &[u8]) -> BattleSettings {
        let source = u32::from_le_bytes(b[12..16].try_into().unwrap());
        let actors = ActorList::find(source)
            .unwrap_or_else(|| panic!("battle settings name an unknown actor list {source:#010x}"));
        BattleSettings {
            layout: b[0],
            music: b[2],
            mode: b[3],
            background: b[4],
            battle_number: b[5],
            panel_pattern: b[6],
            effects: u32::from_le_bytes(b[8..12].try_into().unwrap()),
            actors,
        }
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
    /// Crosses are 1..=10.
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GaugeSpeed {
    #[default]
    Normal = 0,
    Fast = 1,
    Slow = 2,
}

/// Support navis that act once in link battles (NaviStats+0x0D bits; the
/// byte is 0xFF when there are none).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SupportNavis {
    /// Bit 0: cancels one chip with `flags2 & 2`.
    pub rush: bool,
    /// Bit 1: cancels one Mega or Giga chip.
    pub beat: bool,
    /// Bit 2: acts once when HP drops to a quarter.
    pub tango: bool,
}

/// The weapon routine indices (`off_80117D4`; 0xFF = none).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NaviWeapons {
    /// +0x04: the B-button buster.
    pub buster: u8,
    /// +0x05: the charged shot.
    pub charge_shot: u8,
    /// +0x07: the B+Back special.
    pub back_special: u8,
    /// +0x39: the A-button charge (charged chips).
    pub a_charge: u8,
    /// +0x44: the A button in battle mode 9.
    pub mode9_a: u8,
    /// +0x4D: the buster shot's projectile config (applied on an RNG roll).
    pub buster_shot: u8,
    /// +0x4F: the charged shot's projectile config (0 = default 6).
    pub charge_shot_kind: u8,
}

/// NaviCust bugs and program side effects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
}

/// A navi's in-battle stats (the game's 0x64-byte NaviStats block). Only
/// the bytes the engine uses are modeled; `from_bytes`/`to_bytes` are the
/// one place that knows the layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
    pub support: Option<SupportNavis>,
    /// +0x0E: 0 worn out, 0x80 normal, 0xFF Full Synchro; hits wear it
    /// down.
    pub mood: u8,
    /// +0x10: MegaMan's base element byte (primary | secondary bits).
    pub element: u8,
    /// +0x17: the form at battle start (copied to `form` at init).
    pub starting_form: Form,
    /// +0x1B / +0x1C / +0x1D / +0x23
    pub float_shoes: bool,
    pub air_shoes: bool,
    pub undershirt: bool,
    pub super_armor: bool,
    /// +0x21
    pub beast_out_counter: u8,
    /// +0x22: fighting outdoors in the sun (some chips hit harder).
    pub sun: bool,
    /// +0x29
    pub navi: Navi,
    /// +0x2B: a per-navi variant (selects its move lag).
    pub navi_variant: u8,
    /// +0x2C
    pub form: Form,
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
}

impl NaviStats {
    /// Decode the game's 0x64-byte block.
    pub fn from_bytes(b: &[u8; 0x64]) -> NaviStats {
        let u16at = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
        let flag = |i: usize| b[i] != 0;
        NaviStats {
            attack: b[0x01],
            rapid: b[0x02],
            charge: b[0x03],
            first_barrier: b[0x06],
            gauge_speed: match b[0x08] {
                0 => GaugeSpeed::Normal,
                1 => GaugeSpeed::Fast,
                2 => GaugeSpeed::Slow,
                v => panic!("gauge speed {v}"),
            },
            reg_up: b[0x09],
            custom_level: b[0x0A],
            mega_level: b[0x0B],
            giga_level: b[0x0C],
            support: (b[0x0D] != 0xFF).then(|| SupportNavis {
                rush: b[0x0D] & 1 != 0,
                beat: b[0x0D] & 2 != 0,
                tango: b[0x0D] & 4 != 0,
            }),
            mood: b[0x0E],
            element: b[0x10],
            starting_form: Form(b[0x17]),
            float_shoes: flag(0x1B),
            air_shoes: flag(0x1C),
            undershirt: flag(0x1D),
            super_armor: flag(0x23),
            beast_out_counter: b[0x21],
            sun: flag(0x22),
            navi: Navi(b[0x29]),
            navi_variant: b[0x2B],
            form: Form(b[0x2C]),
            folder_reg: [b[0x2E], b[0x2F]],
            max_base_hp: u16at(0x3E),
            hp: u16at(0x40),
            max_hp: u16at(0x42),
            chip_recovery: u16at(0x50),
            folder_tags: [[b[0x56], b[0x57]], [b[0x58], b[0x59]]],
            weapons: NaviWeapons {
                buster: b[0x04],
                charge_shot: b[0x05],
                back_special: b[0x07],
                a_charge: b[0x39],
                mode9_a: b[0x44],
                buster_shot: b[0x4D],
                charge_shot_kind: b[0x4F],
            },
            bugs: NaviCustBugs {
                auto_step: b[0x11],
                panel_trail_kind: b[0x12],
                panel_trail_level: b[0x13],
                buster_blanks: b[0x14],
                buster_charged: b[0x15],
                hit_status: b[0x16],
                hp_drain: b[0x18],
                custom_drain: b[0x19],
                battle_start: b[0x1A],
                emotion: b[0x24],
                processing: b[0x31],
                starting_damage: b[0x3D],
                status_immunity: flag(0x52),
                custom_damage: u16at(0x54),
            },
        }
    }

    /// Encode the modeled fields (other bytes are zero).
    pub fn to_bytes(&self) -> [u8; 0x64] {
        let mut b = [0u8; 0x64];
        let put16 =
            |b: &mut [u8; 0x64], i: usize, v: u16| b[i..i + 2].copy_from_slice(&v.to_le_bytes());
        b[0x01] = self.attack;
        b[0x02] = self.rapid;
        b[0x03] = self.charge;
        b[0x06] = self.first_barrier;
        b[0x08] = self.gauge_speed as u8;
        b[0x09] = self.reg_up;
        b[0x0A] = self.custom_level;
        b[0x0B] = self.mega_level;
        b[0x0C] = self.giga_level;
        b[0x0D] = match self.support {
            None => 0xFF,
            Some(s) => s.rush as u8 | (s.beat as u8) << 1 | (s.tango as u8) << 2,
        };
        b[0x0E] = self.mood;
        b[0x10] = self.element;
        b[0x17] = self.starting_form.0;
        b[0x1B] = self.float_shoes as u8;
        b[0x1C] = self.air_shoes as u8;
        b[0x1D] = self.undershirt as u8;
        b[0x23] = self.super_armor as u8;
        b[0x21] = self.beast_out_counter;
        b[0x22] = self.sun as u8;
        b[0x29] = self.navi.0;
        b[0x2B] = self.navi_variant;
        b[0x2C] = self.form.0;
        b[0x2E] = self.folder_reg[0];
        b[0x2F] = self.folder_reg[1];
        put16(&mut b, 0x3E, self.max_base_hp);
        put16(&mut b, 0x40, self.hp);
        put16(&mut b, 0x42, self.max_hp);
        put16(&mut b, 0x50, self.chip_recovery);
        b[0x56] = self.folder_tags[0][0];
        b[0x57] = self.folder_tags[0][1];
        b[0x58] = self.folder_tags[1][0];
        b[0x59] = self.folder_tags[1][1];
        let w = &self.weapons;
        b[0x04] = w.buster;
        b[0x05] = w.charge_shot;
        b[0x07] = w.back_special;
        b[0x39] = w.a_charge;
        b[0x44] = w.mode9_a;
        b[0x4D] = w.buster_shot;
        b[0x4F] = w.charge_shot_kind;
        let g = &self.bugs;
        b[0x11] = g.auto_step;
        b[0x12] = g.panel_trail_kind;
        b[0x13] = g.panel_trail_level;
        b[0x14] = g.buster_blanks;
        b[0x15] = g.buster_charged;
        b[0x16] = g.hit_status;
        b[0x18] = g.hp_drain;
        b[0x19] = g.custom_drain;
        b[0x1A] = g.battle_start;
        b[0x24] = g.emotion;
        b[0x31] = g.processing;
        b[0x3D] = g.starting_damage;
        b[0x52] = g.status_immunity as u8;
        put16(&mut b, 0x54, g.custom_damage);
        b
    }

    /// A hit's bug code can name any stat byte below 0x64 by its offset
    /// and set it (`sub_80139F6`). Offsets the engine doesn't model are
    /// not supported.
    pub fn set_byte_by_bug_code(&mut self, offset: u8, value: u8) {
        let mut b = self.to_bytes();
        b[offset as usize] = value;
        let updated = NaviStats::from_bytes(&b);
        // An unmodeled byte reads back as 0 (flags read back as 1).
        let back = updated.to_bytes()[offset as usize];
        if back != value && !(back == 1 && value != 0) {
            panic!("bug code writes NaviStats+{offset:#x}, which is not modeled");
        }
        *self = updated;
    }
}

/// Wins/losses/round/max combo, carried between the rounds of a set
/// (from the local side's perspective).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SetScore {
    pub wins: u8,
    pub losses: u8,
    pub round: u8,
    pub max_combo: u8,
}

/// Everything a round starts from.
#[derive(Clone, Debug)]
pub struct RoundSetup {
    pub settings: BattleSettings,
    /// Both navis' stats, by side.
    pub navi_stats: [NaviStats; 2],
    /// The simulation RNG's state (both consoles agree on it).
    pub rng: u32,
    /// Which side this engine instance presents (0 = left). A few
    /// presentation-driven details depend on it (who fades in at the intro).
    pub local_side: u8,
    pub score: SetScore,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The left navi's stats at the start of the machgun replay.
    const MACHGUN_P0: &str = "08000000000100ff00320505010080000000ff00000000000000000101000001010301000000001f0000000a0000ffffff0000000000000000ff00000000e803e803e8030000010000000a0000000000000000000000ffffffffffff0000000000000000";

    fn bytes(hex: &str) -> [u8; 0x64] {
        let v: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        v.try_into().unwrap()
    }

    #[test]
    fn navi_stats_decode() {
        let s = NaviStats::from_bytes(&bytes(MACHGUN_P0));
        assert_eq!((s.hp, s.max_hp, s.max_base_hp), (1000, 1000, 1000));
        assert_eq!(s.navi, Navi::MEGAMAN);
        assert_eq!(s.form, Form::NONE);
        assert!(s.float_shoes && s.air_shoes && !s.undershirt && !s.super_armor);
        assert_eq!(s.mood, 0x80);
        assert_eq!(s.support, Some(SupportNavis::default()));
        assert_eq!(
            (
                s.weapons.buster,
                s.weapons.charge_shot,
                s.weapons.back_special
            ),
            (0, 1, 0xFF)
        );
        assert_eq!(s.beast_out_counter, 3);
    }

    #[test]
    fn navi_stats_encode_round_trips() {
        let s = NaviStats::from_bytes(&bytes(MACHGUN_P0));
        assert_eq!(NaviStats::from_bytes(&s.to_bytes()), s);
    }

    #[test]
    fn bug_code_writes_a_named_stat() {
        let mut s = NaviStats::from_bytes(&bytes(MACHGUN_P0));
        s.set_byte_by_bug_code(0x1D, 1);
        assert!(s.undershirt);
        s.set_byte_by_bug_code(0x16, 2);
        assert_eq!(s.bugs.hit_status, 2);
    }
}
