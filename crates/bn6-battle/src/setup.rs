//! What a round starts from: the battle settings, both navis' stats, the
//! shared RNG seed, and the set score carried between rounds.

/// Battle settings (the game's 16-byte BattleSettings record).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BattleSettings {
    /// Panel layout index.
    pub layout: u8,
    pub unk_01: u8,
    pub music: u8,
    /// Battle mode (0 = netbattle).
    pub mode: u8,
    pub background: u8,
    pub battle_number: u8,
    /// Panel column pattern (which columns belong to which side).
    pub panel_pattern: u8,
    pub unk_07: u8,
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
    /// actor list.
    pub fn netbattle_from_bytes(b: &[u8]) -> BattleSettings {
        let source = u32::from_le_bytes(b[12..16].try_into().unwrap());
        let actors = ActorList::find(source)
            .unwrap_or_else(|| panic!("battle settings name an unknown actor list {source:#010x}"));
        BattleSettings {
            layout: b[0],
            unk_01: b[1],
            music: b[2],
            mode: b[3],
            background: b[4],
            battle_number: b[5],
            panel_pattern: b[6],
            unk_07: b[7],
            effects: u32::from_le_bytes(b[8..12].try_into().unwrap()),
            actors,
        }
    }
}

/// A navi's in-battle stats (the game's 0x64-byte NaviStats block, which
/// battle code reads and writes by index).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NaviStats(pub [u8; 0x64]);

impl std::fmt::Debug for NaviStats {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "NaviStats(hp {}/{}, form {:#x})", self.hp(), self.max_hp(), self.form())
    }
}

impl NaviStats {
    pub fn byte(&self, i: usize) -> u8 {
        self.0[i]
    }
    pub fn set_byte(&mut self, i: usize, v: u8) {
        self.0[i] = v;
    }
    pub fn u16(&self, i: usize) -> u16 {
        u16::from_le_bytes([self.0[i], self.0[i + 1]])
    }
    pub fn set_u16(&mut self, i: usize, v: u16) {
        self.0[i..i + 2].copy_from_slice(&v.to_le_bytes());
    }

    pub fn attack(&self) -> u8 {
        self.0[0x01]
    }
    pub fn speed(&self) -> u8 {
        self.0[0x02]
    }
    pub fn charge(&self) -> u8 {
        self.0[0x03]
    }
    pub fn buster_routine(&self) -> u8 {
        self.0[0x04]
    }
    pub fn charge_routine(&self) -> u8 {
        self.0[0x05]
    }
    pub fn back_special(&self) -> u8 {
        self.0[0x07]
    }
    /// Custom gauge speed class: 0 normal, 1 fast, 2 slow.
    pub fn gauge_speed(&self) -> u8 {
        self.0[0x08]
    }
    pub fn custom_level(&self) -> u8 {
        self.0[0x0A]
    }
    /// NaviCust HP-drain bug level (while in the custom screen).
    pub fn custom_drain_bug(&self) -> u8 {
        self.0[0x19]
    }
    pub fn beast_out_counter(&self) -> u8 {
        self.0[0x21]
    }
    pub fn navi(&self) -> u8 {
        self.0[0x29]
    }
    /// Current Cross/Beast form (0 = none).
    pub fn form(&self) -> u8 {
        self.0[0x2C]
    }
    pub fn max_base_hp(&self) -> u16 {
        self.u16(0x3E)
    }
    pub fn hp(&self) -> u16 {
        self.u16(0x40)
    }
    pub fn max_hp(&self) -> u16 {
        self.u16(0x42)
    }
    /// Custom-HP bug damage at custom-screen open.
    pub fn custom_hp_bug(&self) -> u16 {
        self.u16(0x54)
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
