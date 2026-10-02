//! Battle objects: actors (navis), attacks and effects.
//!
//! Objects live in three fixed pools of 32 slots, one per kind of object.
//! Which slot an object gets and where it sits in the update order are both
//! observable (they decide who runs first and who collides with whom), so
//! the rules are the game's:
//!
//! - a new object takes the lowest free slot of its pool;
//! - an object spawned while another is updating is inserted right after
//!   it and runs later in the same tick; otherwise it goes to the end;
//! - freeing unlinks an object but leaves its own links intact, and the
//!   update loop follows the current object's `next` after it returns.
//!
//! Each object's behavior is chosen by its pool and `index` (see `kinds`).

pub mod sprite;

use crate::collision::CollisionId;
use crate::actor::ActorId;
use nettai_content_api::{ChipHandle, KindHandle};
use sprite::Sprite;

pub use nettai_content_api::{ObjectRef, PanelPos, Pool, Vec3};

/// Where a pool's slots start in the slot arrays.
fn pool_base(pool: Pool) -> usize {
    pool as usize * SLOTS
}

/// Header flags every new object in a pool starts with.
fn spawn_flags(pool: Pool) -> u8 {
    match pool {
        Pool::Attack => flags::ACTIVE | flags::NO_SPRITE_UPDATE,
        _ => flags::ACTIVE | flags::NO_SPRITE_UPDATE | flags::RUN_WHILE_DIMMED,
    }
}

pub const SLOTS: usize = 32;

/// An object's slot index across all pools.
fn slot_index(r: ObjectRef) -> usize {
    pool_base(r.pool) + r.slot as usize
}

/// An object's node in the update list.
fn node_of(r: ObjectRef) -> Node {
    Node(FIRST_OBJECT_NODE + slot_index(r) as u8)
}

/// Header flag bits.
pub mod flags {
    /// The slot is in use.
    pub const ACTIVE: u8 = 0x01;
    /// Drawn this frame.
    pub const VISIBLE: u8 = 0x02;
    /// Keeps updating while the battle is paused.
    pub const RUN_WHILE_PAUSED: u8 = 0x04;
    /// The sprite doesn't animate (set at spawn; loading a sprite clears it).
    pub const NO_SPRITE_UPDATE: u8 = 0x08;
    /// Keeps updating while dimmed.
    pub const RUN_WHILE_DIMMED: u8 = 0x10;
    /// Holds a panel reservation.
    pub const HOLDS_RESERVATION: u8 = 0x20;
}

/// Who sees an object where the viewers differ: what a rule the original
/// keeps per console decided for each viewer (a blind player doesn't see
/// the other side's objects; a few markers show only on their owner's
/// console). Presentation, like the `VISIBLE` flag: the simulation never
/// reads it and the digest leaves it out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sight {
    /// Every viewer sees what the `VISIBLE` flag says.
    #[default]
    Shared,
    /// Whether each viewer (by side) sees it. The `VISIBLE` flag is the
    /// local side's (`RoundSetup::local_side`), as the original's console
    /// has it.
    ByViewer([bool; 2]),
}

/// Object lifecycle states (the game's jump-table offsets).
pub mod state {
    pub const INIT: u8 = 0;
    pub const UPDATE: u8 = 4;
    pub const DESTROY: u8 = 8;
    /// The fourth state of kinds that have one after DESTROY (for them
    /// DESTROY is a last running phase).
    pub const FINISH: u8 = 0x0C;
}

/// An object's lifecycle position: state, action, phase and whether the
/// phase's entry ran.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct StateWord {
    pub state: u8,
    pub action: u8,
    pub phase: u8,
    pub phase_init: u8,
}

/// The drag reaction's steps (the game's values 0, 4, 8).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DragStep {
    /// Set up the push (`sub_80178D4`).
    #[default]
    Start,
    /// Sliding toward the destination panel (`sub_8017992`).
    Slide,
    /// The recovery wait (`sub_8017A38`).
    Recover,
}

/// Where a pushed obstacle may slide (`sub_8017E44`'s +0x0C: 0, 1, 2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SlideBounds {
    /// Any solid panel no other obstacle or body holds.
    #[default]
    Anywhere,
    /// Only the given side's panels: pulled back toward whoever pushed it,
    /// it stays out of their area.
    Area(u8),
}

/// A new object: its kind (and the pool the kind's objects live in), its
/// kind's initial state, and where and with what it starts
/// (`crate::kinds::spawn` makes one from a kind).
pub struct New {
    pub pool: Pool,
    pub kind: KindHandle,
    pub vars: crate::kinds::Vars,
    pub pos: Vec3,
    pub params: [u8; 4],
}

/// The state every object shares. Behavior-specific state lives in `vars`.
#[derive(Clone, Debug, Default)]
pub struct Object {
    pub flags: u8,
    /// Its kind. The original's object slot (pool and index), which the
    /// traces compare, is the validator's to know: the engine's own kinds'
    /// and number-registered kinds' slots are in their definitions
    /// (`KindDef::slot`), and compat has every kind's.
    pub kind: KindHandle,
    /// Spawn parameters (behavior-specific).
    pub params: [u8; 4],
    /// Lifecycle state (`state::INIT/UPDATE/DESTROY`).
    pub state: u8,
    /// Current action; meaning is behavior-specific.
    pub action: u8,
    /// Phase within the action.
    pub phase: u8,
    /// Whether the current phase's entry code has run (the game stores
    /// 0, then 4 or 1). An obstacle's slide keeps its panels left here.
    pub phase_init: u8,
    /// BattleObject+0x0C: where a pushed obstacle may slide (`sub_8017E44`).
    pub slide_bounds: SlideBounds,
    /// BattleObject+0x0D: the drag reaction's step. Actors' stage B resets
    /// it every tick they aren't dragged (`sub_801AF44`); an obstacle's
    /// slide steps through it too (`sub_8017E26`, `sub_8017CC0`).
    pub drag_step: DragStep,
    /// Low nibble: primary element; high nibble: secondary element bits.
    pub element: u8,
    pub slide_type: u8,
    /// Requested animation; `anim_loaded` is the one the sprite has.
    pub anim: u8,
    pub anim_loaded: u8,
    pub panel: PanelPos,
    /// Destination panel while moving.
    pub future_panel: PanelPos,
    /// 0 = left side, 1 = right side.
    pub alliance: u8,
    /// Facing is `alliance ^ flip`.
    pub flip: u8,
    pub prevent_anim: u8,
    /// BattleObject+0x19: ticks left of the dimming shake after a hit
    /// (`sub_8017AB4`: 30 per damaging hit). Other object kinds use the
    /// byte for other things.
    pub shake_timer: u8,
    /// Players: chips left in the hand.
    pub chips_held: u8,
    pub slide_tiles: u8,
    pub slide_dx: u8,
    pub slide_dy: u8,
    pub slide_timer: u8,
    pub slide_state: u8,
    pub timer: u16,
    pub timer2: u16,
    pub hp: u16,
    pub max_hp: u16,
    /// What it is taken for (the original's NameID); none: a virus.
    pub identity: Option<nettai_content_api::IdentityHandle>,
    /// Players: the next chip in the hand (none: the game's 0xFFFF). Other
    /// objects keep the zeroed field, which the chip use reads as the
    /// zeroed chip (`roles.chips.zeroed`).
    pub chip: Option<ChipHandle>,
    /// Attack power plus flag bits (double, paralyze, uninstall...).
    pub damage: u16,
    pub stamina: u16,
    /// BattleObject+0x30 / +0x32: the whole-pixel X and Z an actor shakes
    /// around while dimmed, saved when the dimming handler first runs
    /// (`sub_8017AB4`). Other object kinds use these halfwords for other
    /// things (e.g. a dimming chip's id and bonus).
    pub shake_origin_x: i16,
    pub shake_origin_z: i16,
    pub pos: Vec3,
    pub vel: Vec3,
    pub related: [Option<ObjectRef>; 2],
    /// ExtraVars[0] of an object whose NameID's init hook puts on two
    /// overlays (AI index 14, `sub_8010FD8`): the second one.
    pub second_overlay: Option<ObjectRef>,
    pub collision: Option<CollisionId>,
    pub actor: Option<ActorId>,
    /// A lifecycle position saved by status reactions (the game's +0x5C
    /// word; None = nothing saved).
    pub saved_state: Option<StateWord>,
    /// Behavior-private state.
    pub vars: crate::kinds::Vars,
    /// A dimming controller's chip and bonus, which its telop shows
    /// (BattleObject+0x30 / +0x32 of a controller). Presentation only:
    /// nothing in the simulation reads it, and it is left out of the
    /// digest. None: no one told the engine which chip the controller's
    /// telop names (a dimming content starts itself).
    pub telop_chip: Option<crate::hud::TelopChip>,
    /// Who sees it, where the viewers differ (presentation, see [`Sight`]):
    /// `Battle::visible_to` reads it with the `VISIBLE` flag.
    pub sight: Sight,
}

impl Object {
    /// Shown or hidden for every viewer: the `VISIBLE` flag. (Rules that
    /// decide it per viewer are `Battle::hide_from_blind` and its kin.)
    pub fn set_visible(&mut self, on: bool) {
        if on {
            self.flags |= flags::VISIBLE;
        } else {
            self.flags &= !flags::VISIBLE;
        }
        self.sight = Sight::Shared;
    }
}

/// A position in the update list: the head, the tail sentinel, or an
/// object slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Node(u8);

const HEAD: Node = Node(0);
const SENTINEL: Node = Node(1);
const FIRST_OBJECT_NODE: u8 = 2;
const NODES: usize = FIRST_OBJECT_NODE as usize + 3 * SLOTS;

#[derive(Clone, Copy, Debug, Default, Hash)]
struct Links {
    prev: Option<Node>,
    next: Option<Node>,
}

/// All battle objects and their update order.
#[derive(Clone, Debug, Hash)]
pub struct Objects {
    slots: Vec<Object>,
    /// Per-slot sprite state. Kept apart from `Object` because the game
    /// doesn't clear it when a slot is reused.
    sprites: Vec<Sprite>,
    in_use: [u32; 3],
    links: [Links; NODES],
    /// The object whose update is running, if any.
    current: Option<Node>,
    /// The update loop's bookkeeping (`object_800372A`): per pool, the
    /// objects it passed this tick, and the r3 it leaves for the next
    /// object's update (see `loop_register`).
    loop_passed: [u8; 3],
    loop_register: u32,
}

impl Default for Objects {
    fn default() -> Objects {
        Objects::new()
    }
}

impl Objects {
    pub fn new() -> Objects {
        let mut o = Objects {
            slots: vec![Object::default(); 3 * SLOTS],
            sprites: vec![Sprite::default(); 3 * SLOTS],
            in_use: [0; 3],
            links: [Links::default(); NODES],
            current: None,
            loop_passed: [0; 3],
            loop_register: 0,
        };
        o.reset_list();
        o
    }

    /// Empty the update list (the pools are left as they are).
    pub fn reset_list(&mut self) {
        self.links[HEAD.0 as usize] = Links { prev: None, next: Some(SENTINEL) };
        self.links[SENTINEL.0 as usize] = Links { prev: Some(HEAD), next: None };
        self.current = None;
    }

    /// Clear every pool and the list, as at the start of a battle.
    pub fn reset(&mut self) {
        *self = Objects::new();
    }

    pub fn get(&self, r: ObjectRef) -> &Object {
        &self.slots[slot_index(r)]
    }

    pub fn get_mut(&mut self, r: ObjectRef) -> &mut Object {
        &mut self.slots[slot_index(r)]
    }

    pub fn sprite(&self, r: ObjectRef) -> &Sprite {
        &self.sprites[slot_index(r)]
    }

    pub fn sprite_mut(&mut self, r: ObjectRef) -> &mut Sprite {
        &mut self.sprites[slot_index(r)]
    }

    /// Whether `pool` has a free slot.
    pub fn has_room(&self, pool: Pool) -> bool {
        self.in_use[pool as usize].count_ones() < SLOTS as u32
    }

    pub fn is_allocated(&self, r: ObjectRef) -> bool {
        self.in_use[r.pool as usize] & (0x8000_0000 >> r.slot) != 0
    }

    /// Allocate the lowest free slot of `pool` and initialize the object.
    /// Returns None if the pool is full. The caller links it.
    fn allocate(&mut self, new: New) -> Option<ObjectRef> {
        let New { pool, kind, vars, pos, params } = new;
        let bits = &mut self.in_use[pool as usize];
        let slot = (0..SLOTS as u8).find(|&i| *bits & (0x8000_0000 >> i) == 0)?;
        *bits |= 0x8000_0000 >> slot;
        let r = ObjectRef { pool, slot };
        *self.get_mut(r) = Object {
            flags: spawn_flags(pool),
            kind,
            params,
            pos,
            vars,
            ..Object::default()
        };
        Some(r)
    }

    /// Spawn an object. It runs in the current tick: right after the
    /// object that spawned it, or at the end of the list when spawned from
    /// outside the update loop.
    pub fn spawn(&mut self, new: New) -> Option<ObjectRef> {
        let r = self.allocate(new)?;
        let new = node_of(r);
        match self.current {
            Some(cur) if cur != new => self.insert_after(cur, new),
            _ => self.append(new),
        }
        Some(r)
    }

    /// Spawn an object at the head of the update list (`sub_80033E4`): it
    /// first runs next tick, before everything else.
    pub fn spawn_at_front(&mut self, new: New) -> Option<ObjectRef> {
        let r = self.allocate(new)?;
        let new = node_of(r);
        let first = self.links[HEAD.0 as usize].next.expect("list head has a successor");
        self.links[new.0 as usize] = Links { prev: Some(HEAD), next: Some(first) };
        self.links[HEAD.0 as usize].next = Some(new);
        self.links[first.0 as usize].prev = Some(new);
        Some(r)
    }

    /// Spawn an object at the end of the update list.
    pub fn spawn_at_end(&mut self, new: New) -> Option<ObjectRef> {
        let r = self.allocate(new)?;
        self.append(node_of(r));
        Some(r)
    }

    fn append(&mut self, new: Node) {
        let last = self.links[SENTINEL.0 as usize].prev.expect("list sentinel has a predecessor");
        self.links[last.0 as usize].next = Some(new);
        self.links[new.0 as usize] = Links { prev: Some(last), next: Some(SENTINEL) };
        self.links[SENTINEL.0 as usize].prev = Some(new);
    }

    fn insert_after(&mut self, cur: Node, new: Node) {
        let after = self.links[cur.0 as usize].next.expect("updating object has a successor");
        self.links[new.0 as usize] = Links { prev: Some(cur), next: Some(after) };
        self.links[cur.0 as usize].next = Some(new);
        self.links[after.0 as usize].prev = Some(new);
    }

    /// Free an object: clear its flags and slot bit and unlink it. Its own
    /// links are left as they were, as in the game.
    pub fn free(&mut self, r: ObjectRef) {
        self.get_mut(r).flags = 0;
        self.in_use[r.pool as usize] &= !(0x8000_0000 >> r.slot);
        let n = node_of(r);
        let Links { prev, next } = self.links[n.0 as usize];
        if let Some(p) = prev {
            self.links[p.0 as usize].next = next;
        }
        if let Some(nx) = next {
            self.links[nx.0 as usize].prev = prev;
        }
    }

    /// Free every allocated object, in slot order (end of a round).
    pub fn free_all(&mut self) {
        for pool in Pool::ALL {
            for slot in 0..SLOTS as u8 {
                let r = ObjectRef { pool, slot };
                if self.is_allocated(r) {
                    self.free(r);
                }
            }
        }
    }

    fn node_ref(n: Node) -> Option<ObjectRef> {
        let i = n.0.checked_sub(FIRST_OBJECT_NODE)? as usize;
        let pool = Pool::ALL[i / SLOTS];
        Some(ObjectRef { pool, slot: (i % SLOTS) as u8 })
    }

    /// Objects in update order.
    pub fn in_order(&self) -> impl Iterator<Item = ObjectRef> + '_ {
        let mut n = self.links[HEAD.0 as usize].next;
        std::iter::from_fn(move || {
            let node = n?;
            let r = Self::node_ref(node)?;
            n = self.links[node.0 as usize].next;
            Some(r)
        })
    }

    /// Start the update loop: returns the first object to run.
    pub fn loop_first(&mut self) -> Option<ObjectRef> {
        self.loop_passed = [0; 3];
        self.loop_register = 0;
        let first = self.links[HEAD.0 as usize].next?;
        self.enter(first)
    }

    /// Advance the update loop past the current object. The successor is
    /// read now, after the object's update ran.
    pub fn loop_next(&mut self) -> Option<ObjectRef> {
        let cur = self.current?;
        // object_800372A: the object joins its pool's list for the tick,
        // leaving r3 at its place in it, times 4.
        if let Some(r) = Self::node_ref(cur) {
            let passed = &mut self.loop_passed[r.pool as usize];
            self.loop_register = 4 * *passed as u32;
            *passed = passed.wrapping_add(1);
        }
        let next = self.links[cur.0 as usize].next?;
        self.enter(next)
    }

    /// The r3 the update loop leaves for the object updating now
    /// (`object_800372A`, after the object before it): 4 × how many objects
    /// of that previous object's pool the loop passed before it this tick
    /// (0 for the first object: the loop's caller's r3, which no spawn
    /// reads). Code that never sets r3 spawns with it as a position.
    pub fn loop_register(&self) -> u32 {
        self.loop_register
    }

    fn enter(&mut self, n: Node) -> Option<ObjectRef> {
        if n == SENTINEL {
            self.current = None;
            return None;
        }
        self.current = Some(n);
        Self::node_ref(n)
    }

    /// The object whose update is running.
    pub fn current(&self) -> Option<ObjectRef> {
        self.current.and_then(Self::node_ref)
    }
}
