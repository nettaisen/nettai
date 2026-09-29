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
use sprite::Sprite;

/// Which pool an object lives in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Pool {
    /// Actors: navis, and large chip objects (the game's type 1).
    Actor,
    /// Attacks and hit regions (type 3).
    Attack,
    /// Effects and helpers (type 4).
    Effect,
}

impl Pool {
    pub const ALL: [Pool; 3] = [Pool::Actor, Pool::Attack, Pool::Effect];

    /// The game's type number (1, 3, 4).
    pub fn type_number(self) -> u8 {
        match self {
            Pool::Actor => 1,
            Pool::Attack => 3,
            Pool::Effect => 4,
        }
    }

    fn base(self) -> usize {
        self as usize * SLOTS
    }

    /// Header flags every new object in this pool starts with.
    fn spawn_flags(self) -> u8 {
        match self {
            Pool::Attack => flags::ACTIVE | flags::NO_SPRITE_UPDATE,
            _ => flags::ACTIVE | flags::NO_SPRITE_UPDATE | flags::RUN_IN_TIME_STOP,
        }
    }
}

pub const SLOTS: usize = 32;

/// A handle to an object slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectRef {
    pub pool: Pool,
    pub slot: u8,
}

impl ObjectRef {
    fn node(self) -> Node {
        Node(FIRST_OBJECT_NODE + (self.pool.base() + self.slot as usize) as u8)
    }
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
    /// Keeps updating during time stop.
    pub const RUN_IN_TIME_STOP: u8 = 0x10;
    /// Holds a panel reservation.
    pub const HOLDS_RESERVATION: u8 = 0x20;
}

/// Object lifecycle states (the game's jump-table offsets).
pub mod state {
    pub const INIT: u8 = 0;
    pub const UPDATE: u8 = 4;
    pub const DESTROY: u8 = 8;
}

/// An object's lifecycle position: state, action, phase and whether the
/// phase's entry ran.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StateWord {
    pub state: u8,
    pub action: u8,
    pub phase: u8,
    pub phase_init: u8,
}

/// A 16.16 fixed-point position or velocity, relative to the field's center.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Vec3 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// A panel coordinate: x 1..=6, y 1..=3 on the field (0 and 7/4 are the
/// border).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PanelPos {
    pub x: u8,
    pub y: u8,
}

/// The state every object shares. Behavior-specific state lives in `vars`.
#[derive(Clone, Debug, Default)]
pub struct Object {
    pub flags: u8,
    /// Which behavior within the pool.
    pub index: u8,
    /// Spawn parameters (behavior-specific).
    pub params: [u8; 4],
    /// Lifecycle state (`state::INIT/UPDATE/DESTROY`).
    pub state: u8,
    /// Current action; meaning is behavior-specific.
    pub action: u8,
    /// Phase within the action.
    pub phase: u8,
    /// Whether the current phase's entry code has run (the game stores
    /// 0, then 4 or 1).
    pub phase_init: u8,
    pub unk_0c: u8,
    pub unk_0d: u8,
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
    pub unk_19: u8,
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
    pub name_id: u16,
    /// Players: the next chip in the hand (0xFFFF = none).
    pub chip: u16,
    /// Attack power plus flag bits (double, paralyze, uninstall...).
    pub damage: u16,
    pub stamina: u16,
    pub unk_30: u16,
    pub unk_32: u16,
    pub pos: Vec3,
    pub vel: Vec3,
    pub related: [Option<ObjectRef>; 2],
    pub collision: Option<CollisionId>,
    pub actor: Option<ActorId>,
    /// A lifecycle position saved by status reactions (the game's +0x5C
    /// word; None = nothing saved).
    pub saved_state: Option<StateWord>,
    /// Behavior-private state.
    pub vars: crate::kinds::Vars,
}

/// A position in the update list: the head, the tail sentinel, or an
/// object slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Node(u8);

const HEAD: Node = Node(0);
const SENTINEL: Node = Node(1);
const FIRST_OBJECT_NODE: u8 = 2;
const NODES: usize = FIRST_OBJECT_NODE as usize + 3 * SLOTS;

#[derive(Clone, Copy, Debug, Default)]
struct Links {
    prev: Option<Node>,
    next: Option<Node>,
}

/// All battle objects and their update order.
#[derive(Clone)]
pub struct Objects {
    slots: Vec<Object>,
    /// Per-slot sprite state. Kept apart from `Object` because the game
    /// doesn't clear it when a slot is reused.
    sprites: Vec<Sprite>,
    in_use: [u32; 3],
    links: [Links; NODES],
    /// The object whose update is running, if any.
    current: Option<Node>,
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
        &self.slots[r.pool.base() + r.slot as usize]
    }

    pub fn get_mut(&mut self, r: ObjectRef) -> &mut Object {
        &mut self.slots[r.pool.base() + r.slot as usize]
    }

    pub fn sprite(&self, r: ObjectRef) -> &Sprite {
        &self.sprites[r.pool.base() + r.slot as usize]
    }

    pub fn sprite_mut(&mut self, r: ObjectRef) -> &mut Sprite {
        &mut self.sprites[r.pool.base() + r.slot as usize]
    }

    pub fn is_allocated(&self, r: ObjectRef) -> bool {
        self.in_use[r.pool as usize] & (0x8000_0000 >> r.slot) != 0
    }

    /// Allocate the lowest free slot of `pool` and initialize the object.
    /// Returns None if the pool is full. The caller links it.
    fn allocate(&mut self, pool: Pool, index: u8, pos: Vec3, params: [u8; 4]) -> Option<ObjectRef> {
        let bits = &mut self.in_use[pool as usize];
        let slot = (0..SLOTS as u8).find(|&i| *bits & (0x8000_0000 >> i) == 0)?;
        *bits |= 0x8000_0000 >> slot;
        let r = ObjectRef { pool, slot };
        *self.get_mut(r) = Object {
            flags: pool.spawn_flags(),
            index,
            params,
            pos,
            vars: crate::kinds::Vars::for_kind(pool, index),
            ..Object::default()
        };
        Some(r)
    }

    /// Spawn an object. It runs in the current tick: right after the
    /// object that spawned it, or at the end of the list when spawned from
    /// outside the update loop.
    pub fn spawn(&mut self, pool: Pool, index: u8, pos: Vec3, params: [u8; 4]) -> Option<ObjectRef> {
        let r = self.allocate(pool, index, pos, params)?;
        let new = r.node();
        match self.current {
            Some(cur) if cur != new => self.insert_after(cur, new),
            _ => self.append(new),
        }
        Some(r)
    }

    /// Spawn an object at the end of the update list.
    pub fn spawn_at_end(&mut self, pool: Pool, index: u8, pos: Vec3, params: [u8; 4]) -> Option<ObjectRef> {
        let r = self.allocate(pool, index, pos, params)?;
        self.append(r.node());
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
        let n = r.node();
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
    pub(crate) fn loop_first(&mut self) -> Option<ObjectRef> {
        let first = self.links[HEAD.0 as usize].next?;
        self.enter(first)
    }

    /// Advance the update loop past the current object. The successor is
    /// read now, after the object's update ran.
    pub(crate) fn loop_next(&mut self) -> Option<ObjectRef> {
        let cur = self.current?;
        let next = self.links[cur.0 as usize].next?;
        self.enter(next)
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
