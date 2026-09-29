//! Per-actor data for navis: input state, action requests, charge state
//! and the state of the attack in progress. Eight slots, allocated lowest-free first.
//!
//! This is the game's AIData block (0x100 bytes per slot). Only the fields
//! ported code reads are modeled; each names its AIData offset once. The
//! full old-name mapping is in docs/engine/field-names.md.

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
    /// Revert to base form while paused (`sub_8015994`: the turn-start
    /// check found the Beast Out used up; the mid-battle custom screen
    /// asks too): pause-time action 0x1C with `status::REVERTING_FORM`.
    pub const REVERT_FORM: u32 = 0x40;
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
    /// Starts action 0x49 from idle (`sub_80EEB4C`): a slash at every
    /// opposing navi that is paralyzed (variant 0) or stands on a panel
    /// with flags 0x1C00 (variant 1). No setter was found; the form
    /// changes set the *state* bit 0x80000 (`status::FORM_CHANGE_SPRITE_HELD`),
    /// not this request.
    pub const STUN_STRIKE: u32 = 0x80000;
    pub const SELECT_SPECIAL: u32 = 0x0200_0000;
    /// Change Cross while paused (`sub_802DCDE`, from the transformation
    /// sequencer): pause-time action 0x1C with `status::CHANGING_CROSS`.
    pub const CROSS_CHANGE: u32 = 0x0400_0000;
    /// Cross death (action 0x4C) outside the pause; pause-time request for
    /// action 0x1C inside it. Both set `status::CROSS_KNOCKOUT`.
    pub const CROSS_DEATH: u32 = 0x0800_0000;
    /// Battle mode 9 A press.
    pub const MODE9_A: u32 = 0x1000_0000;
    pub const CROSS_SPECIAL: u32 = 0x2000_0000;
    /// Starts action 0x30 (`sub_80ED55C`, with `status::VOLLEY`): a
    /// volley of shots, the count per variant. No setter was found.
    pub const VOLLEY: u32 = 0x4000_0000;
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
    /// Pause handler: form change in progress (`sub_8014A38`).
    pub const FORM_CHANGE: u32 = 0x80;
    /// Pause handler: reverting to base form (`sub_8015614`).
    pub const REVERTING_FORM: u32 = 0x100;
    pub const NO_CHARGE: u32 = 0x200;
    pub const CAN_TURN: u32 = 0x400;
    /// Anti-damage trap armed (acts like chip 0xBB).
    pub const TRAP_ARMED: u32 = 0x800;
    /// Pause handler: changing Cross (`sub_802D714`).
    pub const CHANGING_CROSS: u32 = 0x1000;
    /// Knocked out of a Cross instead of deleted (action 0x4C, or the
    /// pause handler's `sub_802D926`). Takes over the action dispatch.
    pub const CROSS_KNOCKOUT: u32 = 0x2000;
    /// A Cross change took effect (set when `sub_802D714` ends). A link
    /// navi with it falls back instead of being deleted (`sub_802DD2A`).
    pub const CROSSED: u32 = 0x4000;
    /// The volley (action 0x30) runs. Takes over the action dispatch.
    pub const VOLLEY: u32 = 0x1_0000;
    /// Takes over the action dispatch like the two above; no setter was
    /// found.
    pub const UNINTERRUPTIBLE: u32 = 0x2_0000;
    /// A weakness hit is breaking the Cross (`sub_8015766` runs instead
    /// of the action).
    pub const CROSS_BREAKING: u32 = 0x4_0000;
    /// A form change holds the navi's sprite still (it is off the field).
    pub const FORM_CHANGE_SPRITE_HELD: u32 = 0x8_0000;
    /// Anti-damage trap for heat attacks.
    pub const HEAT_TRAP: u32 = 0x20_0000;
}

/// State of the attack action in progress, shared by whatever action is
/// running (AIData+0xA0, the game's AIAttackVars). Action-specific state
/// gets named fields as actions are ported.
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
    pub special_source: u8,
    /// Which `set_attack` slot started the action.
    pub kind: u8,
    pub beast_lockon: u8,
    /// +0x30: a marker: the move's "direction changed", or a heat trap
    /// swallowing a hit.
    pub marker: u32,
    /// The running action's own state (timers, destinations).
    pub action: crate::kinds::player::actions::ActionVars,
    /// +0x1E..+0x27: the Beast Out rush around the action, when
    /// `beast_lockon` is 1.
    pub rush: crate::kinds::player::actions::beast_rush::Vars,
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
    /// Weapon routines: battle-mode-9 A press, A-charge type, buster,
    /// charged shot, B+Back special.
    pub mode9_a: u8,
    pub a_charge: u8,
    pub buster: u8,
    pub charge_shot: u8,
    pub back_special: u8,
    /// AIData+0x09: ticks toward the next HP lost to the fight-time HP bug
    /// (`sub_8010230`; `sub_801026A` for actors without navi stats).
    pub hp_drain_counter: u8,
    /// AIData+0x0A: ticks toward the next HP lost to the custom-screen HP
    /// drain bug (`sub_80102AC`).
    pub drain_counter: u8,
    /// AIData+0x0F: the turn-start Beast Out check (`sub_80159C6`) runs
    /// only while this is 0, and then sets it to 2. Closing the custom
    /// screen sets it to 1 (`sub_8009338`); a mid-battle custom-screen
    /// request counts it down (`sub_8015A16`, MegaMan only; 0xFF is left
    /// alone). Intent uncertain: in netbattles the close always leaves 1
    /// before the next check.
    pub beast_out_check_delay: u8,
    /// AIData+0x10: drain hits this navi landed on the opponent, turned
    /// into healing (MaxHP/10 each) on its own next hit collection
    /// (`sub_801A308`, `sub_801A324`).
    pub drain_heal_credits: u8,
    /// Alternative A-charge type (form chips).
    pub alt_a_charge: u8,
    /// AIData+0x13: ticks left to press Back after B for the B+Back
    /// special (`sub_8012FC8`: 8 on a B press).
    pub back_special_window: u8,
    /// AIData+0x15: ticks before the B+Back special can be input again:
    /// the special's lockout, set when a `set_attack` kind-3 action (the
    /// B+Back special) ends (`sub_801171C`), counted down by
    /// `sub_80107D4`.
    pub back_special_cooldown: u8,
    /// Input lockout after a chip, in ticks.
    pub lockout: u8,
    /// Buffered auto-move.
    pub buffered_move: u8,
    /// Charge counter, level (0 none, 1 charging, 2 full) and source
    /// (0 none, 1 A, 2 B).
    pub charge_counter: u8,
    /// AIData+0x1C: the NaviCust on-hit bug already fired during this
    /// hit sequence (`sub_8013F1E`; cleared while `prevent_anim` is 0).
    pub hit_bug_latched: bool,
    pub charge_level: u8,
    pub charge_source: u8,
    pub total_damage_taken: u16,
    pub pad: Pad,
    /// Mirror of `pad` maintained during time stop.
    pub timestop_pad: Pad,
    /// AIData+0x32: the Beast Out counter is spent (the game stores
    /// 0xFFFF): set at init with a zero counter (`sub_8013892`), by the
    /// turn-start check (`sub_80159C6`), when a Beast Out reverts
    /// (`sub_80158CC`), and by the NaviCust emotion-swing bug
    /// (`sub_8013DA0`); cleared by `sub_8014446`. Gives emotion 1 and
    /// blocks mood changes (`sub_8015BEC`) and anger (`sub_80143CE`).
    pub beast_out_spent: bool,
    pub anger: u16,
    /// AIData+0x36: exhausted after Beast Over (`sub_80158CC` →
    /// `sub_8014466` stores 0x3C0, which nothing counts down): emotion 5,
    /// mood changes blocked, and 1 HP lost per tick for the rest of the
    /// battle, never the last one (`sub_8014498`).
    pub beast_over_exhausted: bool,
    /// AIData+0x38: ticks before a road panel can start another slide
    /// (5 after a road slide, `sub_80166D0`/`sub_8016730`; counted down
    /// by `sub_801A36A`).
    pub road_cooldown: u16,
    /// AIData+0x3C: the height (Z, whole pixels) a bubble bobs around and
    /// restores when it pops (`sub_8016B72`, `sub_801A2B0`). Viruses
    /// record it every tick (`sub_8108F74`); nothing sets it for players.
    pub bubble_base_z: i16,
    /// AIData+0x40: the Beast Out lock-on marker (effect #0xF,
    /// `sub_80E1620`), which `sub_80E1662` unfreezes.
    pub lockon_marker: Option<ObjectRef>,
    /// Action requests from input (`request::*`).
    pub requests: u32,
    /// Actor state bits (`status::*`).
    pub status: u32,
    /// AIData+0x4C: consecutive ticks spent flinching or paralyzed
    /// (`sub_80143FC`); 120 of them make MegaMan angry (`sub_80142DC`).
    pub stun_ticks: u32,
    /// AIData+0x50: an object tied to the navi that the full status reset
    /// ends (`sub_801390C` → `sub_80E5410`: state 8, first extra var
    /// cleared). Which object stores itself here was not found.
    pub reset_linked_object: Option<ObjectRef>,
    /// The charge-glow effect object.
    pub charge_glow: Option<ObjectRef>,
    /// AIData+0x5C: the Full Synchro aura (actor #0x5E, spawned by
    /// `sub_80139C4` → `sub_80C4C12`); form changes end it, deletion
    /// forgets it (`sub_801746E`).
    pub full_synchro_aura: Option<ObjectRef>,
    /// A sprite overlay attached for the current chip.
    pub overlay: Option<ObjectRef>,
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
