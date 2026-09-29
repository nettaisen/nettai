//! Per-actor data for navis: input state, action requests, charge state
//! and the state of the attack in progress. Eight slots, allocated lowest-free first.
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
    /// Forces the charged-shot action (setter unknown).
    pub const FORCED_CHARGED_SHOT: u32 = 0x20;
    /// Pause-time request: action 0x1C with state bit 0x100.
    pub const PAUSE_40: u32 = 0x40;
    /// Reactive defense chips (anti-damage traps).
    pub const TRAP_200: u32 = 0x200;
    pub const TRAP_400: u32 = 0x400;
    /// Time-stop counter chip.
    pub const TIMESTOP_CHIP: u32 = 0x800;
    pub const TURN_L: u32 = 0x1000;
    pub const TURN_R: u32 = 0x2000;
    /// Pause-time request: form change (action 0x1C, state bit 0x80).
    pub const FORM_CHANGE: u32 = 0x4000;
    pub const TRAP_8000: u32 = 0x8000;
    pub const ALT_CHIP: u32 = 0x10000;
    pub const A_HELD: u32 = 0x20000;
    pub const B_HELD: u32 = 0x40000;
    /// Starts action 0x49 from idle.
    pub const ACTION_49: u32 = 0x80000;
    pub const SELECT_SPECIAL: u32 = 0x0200_0000;
    /// Pause-time request: action 0x1C with state bit 0x1000.
    pub const PAUSE_4000000: u32 = 0x0400_0000;
    /// Cross death (action 0x4C) outside the pause; pause-time request for
    /// action 0x1C with state bit 0x2000 inside it.
    pub const CROSS_DEATH: u32 = 0x0800_0000;
    /// Battle mode 9 A press.
    pub const MODE9_A: u32 = 0x1000_0000;
    pub const CROSS_SPECIAL: u32 = 0x2000_0000;
    /// Starts action 0x30.
    pub const ACTION_30: u32 = 0x4000_0000;
    /// Hit by an element this navi is weak to (ends crosses).
    pub const WEAKNESS_HIT: u32 = 0x8000_0000;
    /// Every attack request.
    pub const ATTACKS: u32 = 0x3F;
    /// The charge holds.
    pub const HOLDS: u32 = A_HELD | B_HELD;
}

/// Actor state bits (`status`).
pub mod status {
    /// Direction bits of the last move (right, left, down, up).
    pub const MOVE_DIRECTIONS: u32 = 0xF;
    pub const CONTROLLABLE: u32 = 0x10;
    pub const CHIP_IN_PROGRESS: u32 = 0x40;
    /// Pause handler: form change in progress.
    pub const FORM_CHANGE: u32 = 0x80;
    pub const NO_CHARGE: u32 = 0x200;
    pub const CAN_TURN: u32 = 0x400;
    /// Anti-damage trap armed (acts like chip 0xBB).
    pub const TRAP_ARMED: u32 = 0x800;
    /// Cross states that take over the action dispatch.
    pub const CROSS_2000: u32 = 0x2000;
    pub const CROSS_4000: u32 = 0x4000;
    pub const CROSS_10000: u32 = 0x1_0000;
    pub const CROSS_20000: u32 = 0x2_0000;
    pub const CROSS_40000: u32 = 0x4_0000;
    /// Anti-damage trap for heat attacks.
    pub const HEAT_TRAP: u32 = 0x20_0000;
}

/// State of the attack action in progress, shared by whatever action is
/// running. Action-specific state gets named fields as actions are ported.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
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
    /// +0x2C: the move's panel-trail argument (0 for input moves).
    pub move_arg: u32,
    /// +0x30: a marker: the move's "direction changed", or a heat trap
    /// swallowing a hit.
    pub marker: u32,
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
    pub attack: AttackVars,
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

    /// Allocate the lowest free slot, cleared. (The game leaves the last
    /// 0x10 bytes of a slot uncleared; nothing ported reads them.)
    pub fn allocate(&mut self) -> Option<ActorId> {
        let slot = (0..SLOTS as u8).find(|&i| self.in_use & (1 << i) == 0)?;
        self.in_use |= 1 << slot;
        self.slots[slot as usize] = ActorData::default();
        Some(ActorId(slot))
    }

    pub fn free(&mut self, id: ActorId) {
        self.in_use &= !(1 << id.0);
    }

    pub fn reset(&mut self) {
        self.in_use = 0;
    }
}
