//! Per-actor data for navis: input state, action requests, charge state
//! and the attack scratchpad. Eight slots, allocated lowest-free first.
//!
//! Field names follow what the fields are used for; ones not yet understood
//! keep the offset of the game's structure in their name (`unk_2c`).

use crate::object::ObjectRef;

pub const SLOTS: usize = 8;

/// A handle to an actor-data slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ActorId(pub u8);

/// What kind of actor this is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActorType {
    #[default]
    Virus = 0,
    Navi = 1,
    Player = 2,
}

/// Action-request bits (`requests`).
pub mod request {
    pub const BUSTER: u32 = 0x1;
    pub const CHARGED_SHOT: u32 = 0x2;
    pub const CHIP: u32 = 0x4;
    pub const CHARGED_CHIP: u32 = 0x8;
    pub const BACK_SPECIAL: u32 = 0x10;
    pub const TURN_L: u32 = 0x1000;
    pub const TURN_R: u32 = 0x2000;
    pub const ALT_CHIP: u32 = 0x10000;
    pub const A_HELD: u32 = 0x20000;
    pub const B_HELD: u32 = 0x40000;
    pub const SELECT_SPECIAL: u32 = 0x0200_0000;
}

/// Actor state bits (`status`).
pub mod status {
    pub const CONTROLLABLE: u32 = 0x10;
    pub const CHIP_IN_PROGRESS: u32 = 0x40;
    pub const NO_CHARGE: u32 = 0x200;
    pub const CAN_TURN: u32 = 0x400;
}

/// The attack scratchpad, shared by whatever action is running.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttackVars {
    /// Step within the action (0, 4, 8...), and whether its entry ran.
    pub step: u8,
    pub step_init: u8,
    /// Attack element byte (primary | secondary bits).
    pub element: u8,
    /// Variant (chip subtype, buster variant).
    pub variant: u8,
    pub charged: u8,
    /// Input lockout to apply when the attack ends.
    pub lockout: u8,
    pub extra: u16,
    pub damage: u16,
    /// Counter/stagger strength for the attack's hitbox.
    pub hit_param: u16,
    /// Action parameters (a chip's `params`).
    pub params: [u8; 4],
    pub timer: u16,
    pub recovery: u16,
    pub chip_id: u16,
    pub unk_16: u8,
    pub unk_17: u8,
    pub unk_18: u16,
    pub unk_1a: u8,
    pub special_source: u8,
    /// Which `set_attack` slot started the action.
    pub kind: u8,
    pub beast_lockon: u8,
    pub unk_1e: u16,
    /// Bytes 0x20..0x50: action-private.
    pub scratch: [u8; 0x30],
}

impl Default for AttackVars {
    fn default() -> AttackVars {
        AttackVars {
            step: 0,
            step_init: 0,
            element: 0,
            variant: 0,
            charged: 0,
            lockout: 0,
            extra: 0,
            damage: 0,
            hit_param: 0,
            params: [0; 4],
            timer: 0,
            recovery: 0,
            chip_id: 0,
            unk_16: 0,
            unk_17: 0,
            unk_18: 0,
            unk_1a: 0,
            special_source: 0,
            kind: 0,
            beast_lockon: 0,
            unk_1e: 0,
            scratch: [0; 0x30],
        }
    }
}

impl AttackVars {
    pub fn scratch_u16(&self, off: usize) -> u16 {
        u16::from_le_bytes([self.scratch[off], self.scratch[off + 1]])
    }
    pub fn set_scratch_u16(&mut self, off: usize, v: u16) {
        self.scratch[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }
    pub fn scratch_u32(&self, off: usize) -> u32 {
        u32::from_le_bytes(self.scratch[off..off + 4].try_into().unwrap())
    }
    pub fn set_scratch_u32(&mut self, off: usize, v: u32) {
        self.scratch[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }
}

/// Joypad state as an actor sees it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pad {
    pub held: u16,
    pub pressed: u16,
    pub released: u16,
    pub previous: u16,
}

impl Pad {
    pub fn update(&mut self, held: u16) {
        let old = self.held;
        self.previous = old;
        self.held = held;
        self.pressed = !old & held;
        self.released = old & !held;
    }
}

#[derive(Clone, Debug, Default)]
pub struct ActorData {
    pub actor_type: ActorType,
    /// Form or AI variant: selects per-form action tables.
    pub ai_index: u8,
    /// 1 = not counted as a combatant.
    pub not_counted: u8,
    pub unk_03: u8,
    /// Weapon routines: battle-mode-9 A press, A-charge type, buster,
    /// charged shot, B+Back special.
    pub mode9_a: u8,
    pub a_charge: u8,
    pub buster: u8,
    pub charge_shot: u8,
    pub back_special: u8,
    pub unk_09: u8,
    /// HP-drain bug counter.
    pub drain_counter: u8,
    pub unk_0b: u8,
    pub unk_0c: u8,
    pub unk_0d: u8,
    pub unk_0e: u8,
    pub unk_0f: u8,
    pub unk_10: u8,
    /// Alternative A-charge type (form chips).
    pub alt_a_charge: u8,
    pub unk_12: u8,
    pub unk_13: u8,
    pub unk_14: u8,
    pub unk_15: u8,
    pub unk_16: u8,
    pub unk_17: u8,
    pub unk_18: u8,
    /// Input lockout after a chip, in ticks.
    pub lockout: u8,
    /// Buffered auto-move.
    pub buffered_move: u8,
    /// Charge counter, level (0 none, 1 charging, 2 full) and source
    /// (0 none, 1 A, 2 B).
    pub charge_counter: u8,
    pub unk_1c: u8,
    pub charge_level: u8,
    pub charge_source: u8,
    pub unk_1f: u8,
    pub total_damage_taken: u16,
    pub pad: Pad,
    /// Mirror of `pad` maintained during time stop.
    pub timestop_pad: Pad,
    pub unk_32: u16,
    pub anger: u16,
    pub unk_36: u16,
    pub unk_38: u16,
    pub unk_3a: u16,
    pub unk_3c: u16,
    pub unk_3e: u16,
    /// Linked helper objects (+0x40: lock-on marker; others per use).
    pub unk_40: Option<ObjectRef>,
    /// Action requests from input (`request::*`).
    pub requests: u32,
    /// Actor state bits (`status::*`).
    pub status: u32,
    pub unk_4c: u32,
    pub unk_50: u32,
    pub unk_54: u32,
    /// The charge-glow effect object.
    pub charge_glow: Option<ObjectRef>,
    pub unk_5c: u32,
    pub unk_60: u32,
    pub unk_64: u32,
    /// A sprite overlay attached for the current chip.
    pub overlay: Option<ObjectRef>,
    pub unk_6c: u32,
    pub unk_70: u32,
    pub unk_74: u32,
    pub unk_78: u32,
    /// AI state machine (0x80..0xA0).
    pub ai_state: [u8; 0x20],
    pub attack: AttackVars,
    /// Bytes 0xF0..0x100: not cleared when the slot is reused.
    pub tail: [u8; 0x10],
    /// Obstacles the obstacle-absorbing chip pulled in, in arrival order
    /// (at most eight; the game keeps them at +0x6C with the count at
    /// +0x0D).
    pub absorbed: Vec<AbsorbedObstacle>,
}

/// An obstacle the obstacle-absorbing chip pulled in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AbsorbedObstacle {
    /// Obstacle kind (`data::ABSORBED_SPRITES`).
    pub kind: u8,
    /// Its animation when absorbed.
    pub anim: u8,
}

/// The actor-data pool.
#[derive(Clone, Debug, Default)]
pub struct Actors {
    slots: [ActorData; SLOTS],
    in_use: u8,
}

impl Actors {
    pub fn get(&self, id: ActorId) -> &ActorData {
        &self.slots[id.0 as usize]
    }

    pub fn get_mut(&mut self, id: ActorId) -> &mut ActorData {
        &mut self.slots[id.0 as usize]
    }

    /// Allocate the lowest free slot, cleared except for its tail bytes.
    pub fn allocate(&mut self) -> Option<ActorId> {
        let slot = (0..SLOTS as u8).find(|&i| self.in_use & (1 << i) == 0)?;
        self.in_use |= 1 << slot;
        let tail = self.slots[slot as usize].tail;
        self.slots[slot as usize] = ActorData { tail, ..ActorData::default() };
        Some(ActorId(slot))
    }

    pub fn free(&mut self, id: ActorId) {
        self.in_use &= !(1 << id.0);
    }

    pub fn reset(&mut self) {
        self.in_use = 0;
    }
}
