//! Collision: hurtboxes and hitboxes registered on panels.
//!
//! There is no global collision pass. Each collidable object, during its
//! own update, first *removes* its registration (resolving hits against
//! whatever is registered on its panels right now), acts on the results,
//! then *presents* itself again. Results land in accumulators both sides
//! read on their next update. See docs/engine/field-collision-damage.md §3.

use crate::battle::Battle;
use crate::content::{Content, DamageWord, Region, RegionRole, SecondaryElements, SparkRole};
use crate::field;
use crate::object::{ObjectRef, PanelPos};
use nettai_content_api::{CollisionHandle, RegionHandle, SparkHandle, StatusHandle};

pub const SLOTS: usize = 32;

/// A handle to a collision slot. Like the game's pointers, a handle can
/// outlive the slot's allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CollisionId(pub u8);

/// Status flags (`f1`).
pub mod f1 {
    pub const GUARD: u32 = 1 << 0;
    pub const INVISIBLE: u32 = 1 << 1;
    /// Submerged: collides only with collision types that have
    /// `types::REACHES_SUBMERGED` or `types::BREAKS_GUARD`, in both
    /// directions (`sub_3007218`). DiveMan's AI sets it
    /// while he dives (`sub_80FDEFC`). The timed form (`timer::SUBMERGED`)
    /// is held while its timer runs and no action is in use (`sub_8010162`);
    /// its only starter, actor #0x5D variant 1 (`sub_80C49E4`), EXE6 never
    /// spawns, EXE4's PopUp does (0x080CD0E2). See
    /// field-collision-damage.md §4.10.1.
    pub const SUBMERGED: u32 = 1 << 2;
    pub const INVULNERABLE: u32 = 1 << 3;
    pub const AIRSHOE: u32 = 1 << 4;
    pub const FLOATSHOE: u32 = 1 << 5;
    pub const MOVING: u32 = 1 << 6;
    pub const DEAD: u32 = 1 << 8;
    pub const FLASHING: u32 = 1 << 9;
    pub const FLINCHING: u32 = 1 << 10;
    pub const PARALYZED: u32 = 1 << 11;
    pub const SLIDING: u32 = 1 << 12;
    pub const BLIND: u32 = 1 << 13;
    pub const IMMOBILIZED: u32 = 1 << 14;
    pub const CONFUSED: u32 = 1 << 15;
    pub const FROZEN: u32 = 1 << 16;
    pub const SUPERARMOR: u32 = 1 << 17;
    pub const UNDERSHIRT: u32 = 1 << 18;
    pub const MOVE_COMPLETE: u32 = 1 << 19;
    pub const DRAG: u32 = 1 << 20;
    pub const ANGER: u32 = 1 << 21;
    pub const USING_ACTION: u32 = 1 << 22;
    /// Only swords and guard breakers (`types::SWORD`, `types::BREAKS_GUARD`)
    /// reach it, and it reaches only bodies and those (`sub_3007218`).
    /// Nothing sets it.
    pub const SWORD_ONLY: u32 = 1 << 23;
    pub const AFFECTED_BY_ICE: u32 = 1 << 25;
    /// Untouchable: `sub_3007218` drops every pair in which either side
    /// has it, so no hit reaches it and it hits nothing; poison panels
    /// don't hurt it either (`sub_801A186`). Falzar Beast Over's form flags
    /// set it (`sub_8014674`), which is why nothing can hit that form.
    pub const UNTOUCHABLE: u32 = 1 << 27;
    pub const BUBBLED: u32 = 1 << 31;
    /// Hits still reach it while the battle is dimmed (`sub_3007218`; else
    /// only hitters set up while dimmed do): ElemTrap's trap sets it.
    pub const HIT_WHILE_DIMMED: u32 = 1 << 24;
}

/// Collision type bits (docs/engine/field-collision-damage.md §3.3): what
/// an object is (`self_flags`) and what it reacts to (`target_flags`), as
/// content's collision types give them (by name: [`member_bits`]). A hit
/// leaves the hitter's in the receiver's `hit_flags` and `raw_hit_flags`;
/// panels' flags carry the high half of what is registered on them. Bits
/// the engine doesn't test matter only through `target & self`.
pub mod types {
    pub const ATTACK_SIDE0: u32 = 1 << 31;
    pub const ATTACK_SIDE1: u32 = 1 << 30;
    pub const OBJECT_SIDE0: u32 = 1 << 29;
    pub const OBJECT_SIDE1: u32 = 1 << 28;
    pub const BODY_SIDE0: u32 = 1 << 27;
    pub const BODY_SIDE1: u32 = 1 << 26;
    pub const OTHER_BODY_SIDE0: u32 = 1 << 25;
    pub const OTHER_BODY_SIDE1: u32 = 1 << 24;
    pub const NEUTRAL: u32 = 1 << 23;
    /// A side's player navi (its body's types have it with `BODY_SIDE0`
    /// or `BODY_SIDE1`): the field regions 0x84 and 0x85 are the panels
    /// that hold it (field-collision-damage.md §3.4).
    pub const NAVI_SIDE0: u32 = 1 << 22;
    pub const NAVI_SIDE1: u32 = 1 << 21;
    pub const FLOATING: u32 = 1 << 20;
    pub const BLOCKER: u32 = 1 << 19;
    /// Reaches the objects a chip sets down that any side's hit sets off:
    /// their target type has it (row 0x0D: a FlashBomb set down, RlngLog's
    /// log, ColArmy's soldiers, a VDoll, SideBub), so an attack with it
    /// reaches those of its own side too. Mines, the generic hitbox (row
    /// 0x48) and the attacks of both sides carry it. No routine tests it.
    pub const REACHES_SET_OBJECTS: u32 = 1 << 18;
    /// Hits while the battle is dimmed: setup gives it to what is set up
    /// while dimmed (`sub_3007218` lets only these, and receivers with
    /// `f1::HIT_WHILE_DIMMED`, hit then).
    pub const HITS_WHILE_DIMMED: u32 = 1 << 16;
    /// Doesn't reach a floating body, nor is it reached by a floating
    /// hitter (`sub_3007218`).
    pub const GROUND_ONLY: u32 = 1 << 15;
    /// A guard that holds against it shows no guard spark; in EXE6 it also
    /// lets `BREAKS_GUARD` break the guard (`sub_3007218`).
    pub const NO_GUARD_SPARK: u32 = 1 << 14;
    /// Counts as a sword against a sword weakness and AntiSword, and
    /// reaches `f1::SWORD_ONLY`.
    pub const SWORD: u32 = 1 << 13;
    /// Breaks a guard (EXE6's only with `NO_GUARD_SPARK`), reaches
    /// submerged and `f1::SWORD_ONLY` bodies, doesn't push an obstacle
    /// that any hit pushes, and keeps a `PIERCING` hit from ending a flash.
    pub const BREAKS_GUARD: u32 = 1 << 12;
    /// A pushing attack (AirShot's shot, a tornado): pops a barrier.
    pub const PUSHING: u32 = 1 << 11;
    /// What the dark chips' attacks carry (EXE5's DarkWide, DrkSword,
    /// DrkLance, BlakWing and the dark meteors, thunder and circle; EXE4's
    /// ElemDark), and a few others (DrilArm's second drill, a bugged
    /// tornado). No type reaches it and the engine doesn't test it; EXE5's
    /// rules do (a dark hit's NaviCust hit bug spares a light navi:
    /// content/exe5/rules/light_dark).
    pub const DARK: u32 = 1 << 10;
    /// A pull (a gust, DustBrk's): pops a barrier.
    pub const PULL: u32 = 1 << 9;
    /// Counts as a drain on the receiver.
    pub const DRAIN: u32 = 1 << 8;
    /// Reaches `f1::FLOATSHOE` bodies, and is reached by them, where the
    /// game's hit test asks for it (`float_shoe_needs_self_bit`).
    pub const REACHES_FLOATSHOE: u32 = 1 << 7;
    /// Wind: pops a barrier (EXE4's takes it away).
    pub const WIND: u32 = 1 << 5;
    /// Destroys the chip the navi it hits holds (`sub_801A2CC`).
    pub const CHIP_DESTRUCTION: u32 = 1 << 4;
    /// Reaches submerged bodies.
    pub const REACHES_SUBMERGED: u32 = 1 << 3;
    /// Pierces invisibility and flashing, and ends a flash.
    pub const PIERCING: u32 = 1 << 2;
    /// Breaks: a guard (by the game's `guard_breaks_to`), ice (thaws it
    /// for one more multiplier) and obstacles.
    pub const BREAK: u32 = 1 << 1;

    /// Each kind of thing, of either side.
    pub const ATTACKS: u32 = ATTACK_SIDE0 | ATTACK_SIDE1;
    pub const OBJECTS: u32 = OBJECT_SIDE0 | OBJECT_SIDE1;
    pub const BODIES: u32 = BODY_SIDE0 | BODY_SIDE1;
    pub const OTHER_BODIES: u32 = OTHER_BODY_SIDE0 | OTHER_BODY_SIDE1;
    pub const NAVIS: u32 = NAVI_SIDE0 | NAVI_SIDE1;
}

/// A collision type member's bits (content names them: `new.collision`'s
/// `flags`, docs/design/scripting.md): side 0's and side 1's, each the
/// other's mirror image (a member of the object's own side, `own_`, or the
/// other's, `enemy_`; the rest are the same bit for both).
pub fn member_bits(m: nettai_content_api::CollisionMember) -> [u32; 2] {
    use nettai_content_api::CollisionMember as M;
    use types::*;
    let same = |b: u32| [b, b];
    match m {
        M::OwnAttack => [ATTACK_SIDE0, ATTACK_SIDE1],
        M::EnemyAttack => [ATTACK_SIDE1, ATTACK_SIDE0],
        M::OwnObject => [OBJECT_SIDE0, OBJECT_SIDE1],
        M::EnemyObject => [OBJECT_SIDE1, OBJECT_SIDE0],
        M::OwnBody => [BODY_SIDE0, BODY_SIDE1],
        M::EnemyBody => [BODY_SIDE1, BODY_SIDE0],
        M::OwnOtherBody => [OTHER_BODY_SIDE0, OTHER_BODY_SIDE1],
        M::EnemyOtherBody => [OTHER_BODY_SIDE1, OTHER_BODY_SIDE0],
        M::OwnNavi => [NAVI_SIDE0, NAVI_SIDE1],
        M::EnemyNavi => [NAVI_SIDE1, NAVI_SIDE0],
        M::Neutral => same(NEUTRAL),
        M::Floating => same(FLOATING),
        M::Blocker => same(BLOCKER),
        M::ReachesSetObjects => same(REACHES_SET_OBJECTS),
        M::HitsWhileDimmed => same(HITS_WHILE_DIMMED),
        M::GroundOnly => same(GROUND_ONLY),
        M::NoGuardSpark => same(NO_GUARD_SPARK),
        M::Sword => same(SWORD),
        M::BreaksGuard => same(BREAKS_GUARD),
        M::Pushing => same(PUSHING),
        M::Dark => same(DARK),
        M::Pull => same(PULL),
        M::Drain => same(DRAIN),
        M::ReachesFloatShoe => same(REACHES_FLOATSHOE),
        M::Wind => same(WIND),
        M::ChipDestruction => same(CHIP_DESTRUCTION),
        M::ReachesSubmerged => same(REACHES_SUBMERGED),
        M::Piercing => same(PIERCING),
        M::Break => same(BREAK),
    }
}

/// The bits a name in a hit test stands for, for an object of `side`
/// (`Collision:hit_any`, the rules' `anti_traps.sword_spares`): a
/// collision type member ([`member_bits`]) or one of the [`hit`] bits.
/// A side-relative member without a side is refused.
pub fn hit_name_bits(name: &str, side: Option<u8>) -> Result<u32, String> {
    use nettai_content_api::{CollisionMember, HitBit};
    if let Some(b) = HitBit::from_name(name) {
        return Ok(match b {
            HitBit::Guarded => hit::GUARDED,
            HitBit::Countered => hit::COUNTERED,
            HitBit::GuardSpark => hit::GUARD_SPARK,
        });
    }
    let Some(m) = CollisionMember::from_name(name) else {
        let names: Vec<&str> = CollisionMember::ALL.iter().map(|m| m.name()).chain(HitBit::ALL.iter().map(|b| b.name())).collect();
        return Err(format!("`{name}` is no hit's name (the collision types' members and the hit bits: {})", names.join(", ")));
    };
    let [w0, w1] = member_bits(m);
    match side {
        _ if w0 == w1 => Ok(w0),
        Some(s) => Ok(if s & 1 == 0 { w0 } else { w1 }),
        None => Err(format!("`{name}` is of a side, and nothing gives one here")),
    }
}

/// A collision type's two words from its members' names (`new.collision`'s
/// `flags`), side 0's and side 1's.
pub fn type_words<'a>(names: impl IntoIterator<Item = &'a str>) -> Result<[u32; 2], String> {
    use nettai_content_api::CollisionMember;
    let mut words = [0u32; 2];
    for name in names {
        let m = CollisionMember::from_name(name).ok_or_else(|| {
            let names: Vec<&str> = CollisionMember::ALL.iter().map(|m| m.name()).collect();
            format!("`{name}` is no collision type member (they are {})", names.join(", "))
        })?;
        let bits = member_bits(m);
        if words[0] & bits[0] != 0 {
            return Err(format!("`{name}` is named twice"));
        }
        words[0] |= bits[0];
        words[1] |= bits[1];
    }
    Ok(words)
}

/// What a hit leaves in `Accumulators::hit_flags` besides the hitters'
/// collision types (bits no type has).
pub mod hit {
    /// A guard turned its hit aside (the hitter's).
    pub const GUARDED: u32 = 1 << 0;
    /// A counter hit (the receiver's).
    pub const COUNTERED: u32 = 1 << 6;
    /// Its guard turned aside a hit that shows a guard spark (the
    /// receiver's).
    pub const GUARD_SPARK: u32 = 1 << 17;
}

/// The collision type bits of an attack of each side (side 0's, side
/// 1's).
pub const ATTACK_OF: [u32; 2] = [types::ATTACK_SIDE0, types::ATTACK_SIDE1];

/// The panels each side's attacks were presented on during a tick (a bit a
/// panel at `y * 8 + x`, as `Collision::masks` index them), by side: what
/// [`crate::Battle::attacked_panels`] reads. Kept for a reader of the
/// battle (an AI's view of it), as the panels' highlight is: nothing in
/// the simulation reads it, and the digest leaves it out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AttackLog {
    pub panels: [u64; 2],
}

/// Per-registration-window hit results (zeroed on present).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Accumulators {
    /// Slots already paired with since the last present.
    pub pair_tested: u32,
    /// Everything that touched us, unfiltered.
    pub raw_hit_flags: u32,
    /// What hit us (filtered: the hitters' `types`), plus the `hit` bits.
    pub hit_flags: u32,
    /// The same by the other collision's flip (EXE5's +0x6C and +0x70: the
    /// hit registration, 0x080169C8 to 0x08016A68, keeps them only so; an
    /// unflipped hitter's, of either side, and a flipped one's), which
    /// EXE5's obstacle push on any hit reads (0x08017AD8); EXE6 keeps them
    /// unread.
    pub hit_flags_by_flip: [u32; 2],
    pub exclamation: u8,
    pub damage_multiplier: u8,
    pub damage_elements: u8,
    pub raw_elements: u8,
    pub elec_damage: u16,
    pub hit_by: u32,
    pub final_damage: u16,
    /// Damage by element (null, heat, aqua, elec, wood, element 5/poison).
    pub element_damage: [u16; 6],
    pub mood_damage: u16,
    pub counter: u16,
    pub drain_hits: u16,
    pub raw_element_damage: [u16; 6],
    pub inflicted_bugs: u16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CollisionData {
    pub enabled: u8,
    /// The region it covers around its panel (or the whole field's panels
    /// that meet a condition), or none.
    pub region: Option<RegionHandle>,
    pub element: u8,
    /// Directions a guard blocked from.
    pub guard_dirs: u8,
    pub alliance: u8,
    pub flip: u8,
    pub barrier: u8,
    /// Counter strength (bits 0-6); bit 7 = can't counter.
    pub counter_byte: u8,
    /// What a panel type counts while the body stands on it (poison's
    /// drain, `sub_801A186`).
    pub standing_count: u8,
    /// The spark its hits show, or none.
    pub hit_effect: Option<SparkHandle>,
    /// Region anchor.
    pub panel: PanelPos,
    /// Last move direction: 0 none, 1 up, 2 down, 3 back, 4 forward, 5 other.
    pub direction: u8,
    pub counter_timer: u8,
    pub hit_mod_base: u8,
    pub hit_mod_final: u8,
    /// The hit modifiers of the hits it took by the hitter's flip (EXE5's
    /// +0x18 and +0x19, 0x08016AA6: an unflipped hitter's, of either side,
    /// and a flipped one's), which a push that reads by the hitter's flip
    /// takes (`PushSource::ByHitterFlip`, EXE5's); EXE6's keeps them unread.
    pub hit_mod_by_side: [u8; 2],
    /// The status its hits carry, and the one the hits it took landed.
    pub status_base: Option<StatusHandle>,
    pub status_final: Option<StatusHandle>,
    /// Bug code (low byte) and argument (high byte).
    pub bugs: u16,
    pub barrier_weak: u8,
    pub barrier_saved_hmf: u8,
    pub barrier_hp: u8,
    pub barrier_threshold: u8,
    pub secondary_weakness: u8,
    pub secondary_element: u8,
    pub barrier_timer: u16,
    /// Status timers: paralyze, confuse, blind, immobilize, flash,
    /// submerged, invulnerable, freeze, bubble.
    pub status_timers: [u16; 9],
    pub self_damage: u16,
    /// What I am (collision type flags).
    pub self_flags: u32,
    /// What I react to.
    pub target_flags: u32,
    pub parent: Option<ObjectRef>,
    pub f1: u32,
    pub f2: u32,
    /// This slot's bit (`0x80000000 >> slot`).
    pub bit: u32,
    /// Status visual objects and other links (0x48..0x67).
    /// EXE5's +0x2C: how long a body stays under the sea's surface (0xFFFF
    /// while it dives on a panel that submerges: 0x08017030).
    pub dive_timer: u16,
    pub links: [Option<ObjectRef>; 5],
    pub acc: Accumulators,
}

/// Indices into `links`: the status visual objects.
pub mod link {
    /// +0x48: confusion stars.
    pub const CONFUSE: usize = 0;
    /// +0x4C: blindness.
    pub const BLIND: usize = 1;
    /// +0x58: ice block.
    pub const FREEZE: usize = 2;
    /// +0x60: bubble.
    pub const BUBBLE: usize = 3;
    /// +0x50 (EXE4's +0x7C): the mark over a body that is under
    /// (`sub_801012C`: EXE5's ripple, EXE4's hole).
    pub const RIPPLE: usize = 4;
}

/// Indices into `status_timers`.
pub mod timer {
    pub const PARALYZE: usize = 0;
    pub const CONFUSE: usize = 1;
    pub const BLIND: usize = 2;
    pub const IMMOBILIZE: usize = 3;
    pub const FLASH: usize = 4;
    /// CollisionData+0x26: holds `f1::SUBMERGED` (0xFFFF =
    /// indefinitely); started by `sub_80101AE`, ended by any hit
    /// (`sub_8010198`) or by `sub_80101C4`.
    pub const SUBMERGED: usize = 5;
    pub const INVULNERABLE: usize = 6;
    pub const FREEZE: usize = 7;
    pub const BUBBLE: usize = 8;
}

#[derive(Clone, Debug, Hash)]
pub struct Collision {
    slots: [CollisionData; SLOTS],
    in_use: u32,
    /// Per-panel registration masks, index y*8+x.
    pub masks: [u32; 40],
}

impl Collision {
    /// The pool at battle start (`sub_801986C`).
    pub fn new() -> Collision {
        let mut slots = [CollisionData::default(); SLOTS];
        for (k, s) in slots.iter_mut().enumerate() {
            s.bit = 0x8000_0000 >> k;
        }
        Collision { slots, in_use: 0, masks: [0; 40] }
    }

    pub fn get(&self, id: CollisionId) -> &CollisionData {
        &self.slots[id.0 as usize]
    }

    pub fn get_mut(&mut self, id: CollisionId) -> &mut CollisionData {
        &mut self.slots[id.0 as usize]
    }

    fn allocate(&mut self) -> Option<CollisionId> {
        let k = (0..SLOTS).find(|&k| self.in_use & (0x8000_0000 >> k) == 0)?;
        self.in_use |= 0x8000_0000 >> k;
        let bit = self.slots[k].bit;
        self.slots[k] = CollisionData { bit, enabled: 1, ..CollisionData::default() };
        Some(CollisionId(k as u8))
    }

    /// Whether slot `id` is allocated.
    pub fn in_use(&self, id: CollisionId) -> bool {
        self.in_use & self.slots[id.0 as usize].bit != 0
    }

    /// `object_freeCollisionData`. Mask bits and fields are left alone.
    pub fn free(&mut self, id: CollisionId) {
        let s = &mut self.slots[id.0 as usize];
        s.enabled = 0;
        self.in_use &= !s.bit;
    }

    /// The union of the high halves of the collision types registered on a
    /// panel (freed slots with stale bits included).
    pub fn panel_self_flags(&self, x: u8, y: u8) -> u32 {
        let mut m = self.masks[(y * 8 + x) as usize];
        let mut acc = 0;
        let mut slot = 0;
        loop {
            let c = m >> 31;
            m <<= 1;
            if c != 0 {
                acc |= self.slots[slot].self_flags & 0xFFFF_0000;
            } else if m == 0 {
                break;
            }
            slot += 1;
        }
        acc
    }

    /// The panels a registration covers, in processing order.
    fn region_panels(&self, content: &Content, field: &field::Field, id: CollisionId) -> Vec<(u8, u8)> {
        let s = &self.slots[id.0 as usize];
        match s.region.map(|h| content.region(h)) {
            None => Vec::new(),
            Some(Region::Panels(offsets)) => {
                let dir: i8 = if s.alliance ^ s.flip == 0 { 1 } else { -1 };
                offsets
                    .iter()
                    .map(|o| ((s.panel.x as i8 + o.dx * dir) as u8, (s.panel.y as i8 + o.dy) as u8))
                    .filter(|&(x, y)| field::is_valid(x, y))
                    .collect()
            }
            Some(Region::Field(cond)) => {
                let mut v = Vec::new();
                for y in 1..=3 {
                    for x in 1..=6 {
                        if field.check(x, y, cond.require, cond.forbid) {
                            v.push((x, y));
                        }
                    }
                }
                v
            }
        }
    }

    /// Whether a registration's region is a whole-field one.
    fn covers_whole_field(&self, content: &Content, id: CollisionId) -> bool {
        matches!(self.slots[id.0 as usize].region.map(|h| content.region(h)), Some(Region::Field(_)))
    }
}

/// Records let go of while still in use: an object freed without freeing
/// its collision record (`object_freeCollisionData` before
/// `object_freeMemory`) keeps the record taken for the rest of the round,
/// and with all [`SLOTS`] taken nothing new can collide. For the replays
/// against the original, which keep no record of its collision pool: what
/// each tick lets go of. (The original lets go of one itself, EXE6's
/// TenguMan's, `sub_80B9F00`; nettai frees it.)
#[derive(Clone, Debug, Default)]
pub struct StrayWatch {
    /// The records already let go of, as of the last tick looked at.
    strays: u32,
}

impl StrayWatch {
    /// The kinds of the objects that let go of a record still in use since
    /// the last call (each record said once, as its holder is freed or
    /// takes another).
    pub fn let_go(&mut self, b: &Battle) -> Vec<String> {
        let mut strays = 0;
        let mut out = Vec::new();
        for k in 0..SLOTS as u8 {
            let id = CollisionId(k);
            if !b.collision.in_use(id) {
                continue;
            }
            let Some(holder) = b.collision.get(id).parent else { continue };
            if b.objects.is_allocated(holder) && b.objects.get(holder).collision == Some(id) {
                continue;
            }
            let bit = b.collision.get(id).bit;
            strays |= bit;
            if self.strays & bit == 0 {
                // (A freed object keeps its kind until its slot is taken again.)
                out.push(b.content.defs.kind(b.objects.get(holder).kind).key.to_string());
            }
        }
        self.strays = strays;
        out
    }

    /// A replay's difference for this tick: a record let go of while the
    /// battle still runs, by a kind other than `the_originals` (those the
    /// original's own code lets go of). At the battle's end every object
    /// goes, and nothing is said.
    pub fn difference(&mut self, b: &Battle, the_originals: &[&str]) -> Option<String> {
        let kinds = self.let_go(b);
        if b.is_battle_over() {
            return None;
        }
        let kind = kinds.into_iter().find(|k| !the_originals.contains(&k.as_str()))?;
        Some(format!("collision: {kind} let go of its collision record still in use (freed without object_freeCollisionData)"))
    }
}

impl Default for Collision {
    fn default() -> Collision {
        Collision::new()
    }
}

impl Battle {
    /// `object_createCollisionData`: give `obj` a collision slot. A new
    /// slot is zeroed: no region, and the plain hit spark (the original's
    /// hit effect 0).
    pub fn create_collision(&mut self, obj: ObjectRef) -> Option<CollisionId> {
        let id = self.collision.allocate();
        if let Some(id) = id {
            self.collision.get_mut(id).hit_effect = Some(self.roles().spark(SparkRole::Plain));
        }
        self.objects.get_mut(obj).collision = id;
        id
    }

    /// The registration's own panel as a region (`object_setCollisionRegion`
    /// with 1, what a setup gives every registration).
    pub fn anchor_region(&self) -> Option<RegionHandle> {
        Some(self.roles().region(RegionRole::Anchor))
    }

    /// `object_setupCollisionData`.
    pub fn setup_collision(&mut self, obj: ObjectRef, self_type: CollisionHandle, target_type: CollisionHandle, hit_mod: u8) {
        let o = self.objects.get(obj).clone();
        let Some(id) = o.collision else { return };
        let dimmed = self.is_dimmed();
        let anchor = self.roles().region(RegionRole::Anchor);
        let word = self.game_rules().effects.damage_word.clone();
        let s = self.collision.get_mut(id);
        s.parent = Some(obj);
        s.hit_mod_base = hit_mod;
        s.element = o.element & 0xF;
        s.secondary_element = o.element & 0xF0;
        s.alliance = o.alliance;
        s.flip = o.flip;
        s.panel = o.panel;
        s.region = Some(anchor);
        s.counter_byte = o.stamina as u8;
        s.self_damage = o.damage;
        s.self_flags = self.content.collision_type(self_type, o.alliance).0 | if dimmed { types::HITS_WHILE_DIMMED } else { 0 };
        let (target_flags, row_offset) = self.content.collision_type(target_type, o.alliance);
        s.target_flags = target_flags;
        // The garbage high byte of any bug code: the table offset the
        // target lookup left in r1.
        let r1 = row_offset + o.alliance as u16 * 4;
        decode_damage_word(s, r1, &word, self.content.defs.roles());
    }

    /// `sub_801A082`: redo the damage and collision-type part of the setup
    /// (after a change of damage or of what the object is).
    pub fn reset_collision_types(&mut self, obj: ObjectRef, self_type: CollisionHandle, target_type: CollisionHandle, hit_mod: u8) {
        let o = self.objects.get(obj);
        let Some(id) = o.collision else { return };
        let (alliance, damage) = (o.alliance, o.damage);
        let dimmed = self.is_dimmed();
        // (`effects.retype`: what it is and what it hits, or what it is
        // alone, EXE5's 0x08016B9E.)
        let alone = self.game_rules().effects.retype == crate::content::RetypeRule::IsAlone;
        let word = self.game_rules().effects.damage_word.clone();
        let (target_flags, row_offset) = self.content.collision_type(target_type, alliance);
        let s = self.collision.get_mut(id);
        s.hit_mod_base = hit_mod;
        s.self_damage = damage;
        s.self_flags = self.content.collision_type(self_type, alliance).0 | if dimmed && !alone { types::HITS_WHILE_DIMMED } else { 0 };
        if !alone {
            s.target_flags = target_flags;
        }
        // A bug code's garbage high byte is what `battle_isTimeStop` left in
        // r1 (4, or 0x10000 while dimmed); EXE5's, what the target lookup
        // left.
        let r1 = if alone {
            row_offset + alliance as u16 * 4
        } else if dimmed {
            0
        } else {
            4
        };
        decode_damage_word(s, r1, &word, self.content.defs.roles());
    }

    /// `object_presentCollisionData`: clear the accumulators and register.
    pub fn present_collision(&mut self, id: CollisionId) {
        let dimmed = self.is_dimmed();
        let s = self.collision.get_mut(id);
        if !dimmed {
            s.hit_mod_final = 0;
            // (EXE5's clear, 0x08016B22, takes its two by-side bytes too.)
            s.hit_mod_by_side = [0; 2];
            s.guard_dirs = 0;
        }
        s.status_final = None;
        s.acc = Accumulators::default();
        let (bit, self_flags) = (s.bit, s.self_flags);
        let whole_field = self.collision.covers_whole_field(&self.content, id);
        for (x, y) in self.collision.region_panels(&self.content, &self.field, id) {
            self.collision.masks[(y * 8 + x) as usize] |= bit;
            // (The panels attacked this tick, for a reader of the battle.)
            for (side, attack) in ATTACK_OF.iter().enumerate() {
                if self_flags & attack != 0 {
                    self.attacked.panels[side] |= 1 << (y * 8 + x);
                }
            }
            // Whole-field registrations refresh the wrong panel (a no-op).
            if !whole_field {
                self.field.refresh(&self.content.rules().panels, &self.collision, x, y);
            }
        }
    }

    /// `object_removeCollisionData`: unregister and resolve hits.
    pub fn remove_collision(&mut self, id: CollisionId) {
        let bit = self.collision.get(id).bit;
        let whole_field = self.collision.covers_whole_field(&self.content, id);
        for (x, y) in self.removal_panels(id, whole_field) {
            let i = (y * 8 + x) as usize;
            let was = self.collision.masks[i] & bit != 0;
            self.collision.masks[i] &= !bit;
            if whole_field && !was {
                continue;
            }
            self.field.refresh(&self.content.rules().panels, &self.collision, x, y);
            self.pair_test(x, y, id);
            self.convert_panel(x, y, id);
        }
    }

    /// The panels `remove_collision` takes `id` off, in its order: a
    /// whole-field registration's every panel, else its region's.
    fn removal_panels(&self, id: CollisionId, whole_field: bool) -> Vec<(u8, u8)> {
        if whole_field {
            (1..=3u8).flat_map(|y| (1..=6u8).map(move |x| (x, y))).collect()
        } else {
            self.collision.region_panels(&self.content, &self.field, id)
        }
    }

    /// Whether `remove_collision` would test `id` against a registration
    /// on its panels that it hasn't been tested against since it was
    /// presented (one that came onto them after it): only such a test adds
    /// to its hits, so without one its hits are the same before the removal
    /// as after.
    pub(crate) fn removal_tests_another(&self, id: CollisionId) -> bool {
        let s = self.collision.get(id);
        let (bit, tested) = (s.bit, s.acc.pair_tested);
        let whole_field = self.collision.covers_whole_field(&self.content, id);
        self.removal_panels(id, whole_field).into_iter().any(|(x, y)| {
            let m = self.collision.masks[(y * 8 + x) as usize];
            (!whole_field || m & bit != 0) && m & !bit & !tested != 0
        })
    }

    /// `sub_30075FC`.
    fn pair_test(&mut self, x: u8, y: u8, me: CollisionId) {
        if self.paused {
            return;
        }
        let mut m = self.collision.masks[(y * 8 + x) as usize];
        let mut other = 0usize;
        loop {
            let c = m >> 31;
            m <<= 1;
            if c != 0 {
                let o = CollisionId(other as u8);
                let other_bit = self.collision.get(o).bit;
                let my_bit = self.collision.get(me).bit;
                if self.collision.get(me).acc.pair_tested & other_bit == 0 {
                    self.collision.get_mut(me).acc.pair_tested |= other_bit;
                    self.collision.get_mut(o).acc.pair_tested |= my_bit;
                    self.react(me, o);
                    self.react(o, me);
                }
            } else if m == 0 {
                return;
            }
            other += 1;
        }
    }

    /// `sub_3007650`: `r` reacts to `h` if it targets what `h` is.
    fn react(&mut self, r: CollisionId, h: CollisionId) {
        if self.collision.get(r).target_flags & self.collision.get(h).self_flags != 0 {
            self.resolve_hit(r, h);
            self.accumulate_raw(r, h);
        }
    }

    /// `sub_3007218`: receiver `r` is hit by `h`.
    fn resolve_hit(&mut self, r: CollisionId, h: CollisionId) {
        let hd = *self.collision.get(h);
        let rd = *self.collision.get(r);
        if self.is_dimmed() && !(rd.f1 & f1::HIT_WHILE_DIMMED != 0 || hd.self_flags & types::HITS_WHILE_DIMMED != 0) {
            return;
        }
        // (The game's hit test, the reactions section's `hit_test`: EXE5's,
        // 0x0801691C, counts a bubbled body as submerged, elec reaching
        // either, has no FloatShoe test, and its guard breaks to 0x1002.)
        let test = self.content.rules().hit_test;
        let submerged = if test.bubbled_as_submerged { f1::SUBMERGED | f1::BUBBLED } else { f1::SUBMERGED };
        let elec = |element: u8| test.elec_reaches_submerged && element == 3;
        let self_bit = test.float_shoe_needs_self_bit;
        // The hitter's state against the receiver's type.
        let f = hd.f1;
        let rs = rd.self_flags;
        if (f & (f1::INVISIBLE | f1::FLASHING) != 0 && rs & types::PIERCING == 0)
            || (f & submerged != 0 && rs & (types::REACHES_SUBMERGED | types::BREAKS_GUARD) == 0 && !elec(rd.element))
            || (f & f1::SWORD_ONLY != 0 && rs & (types::BODIES | types::SWORD | types::BREAKS_GUARD) == 0)
            || f & f1::UNTOUCHABLE != 0
            || (self_bit && f & f1::FLOATSHOE != 0 && rs & types::REACHES_FLOATSHOE == 0)
        {
            return;
        }
        // The receiver's state against the hitter's type (EXE4's leaves its
        // untouchable state to the invulnerable test, after the guard).
        let f = rd.f1;
        let hs = hd.self_flags;
        if (f & (f1::INVISIBLE | f1::FLASHING) != 0 && hs & types::PIERCING == 0)
            || (f & submerged != 0 && hs & (types::REACHES_SUBMERGED | types::BREAKS_GUARD) == 0 && !elec(hd.element))
            || (f & f1::SWORD_ONLY != 0 && hs & (types::SWORD | types::BREAKS_GUARD) == 0)
            || (!test.guard_before_untouchable && f & f1::UNTOUCHABLE != 0)
            || (self_bit && f & f1::FLOATSHOE != 0 && hs & types::REACHES_FLOATSHOE == 0)
        {
            return;
        }
        // Guard.
        if rd.f1 & f1::GUARD != 0 {
            let brk = if hs & types::NO_GUARD_SPARK != 0 { types::BREAKS_GUARD | types::BREAK } else { test.guard_breaks_to };
            if hs & brk == 0 {
                let hm = self.collision.get_mut(h);
                hm.acc.hit_flags |= hit::GUARDED;
                hm.acc.hit_flags_by_flip[rd.flip as usize & 1] |= hit::GUARDED;
                let mut flags = hs & !types::CHIP_DESTRUCTION;
                // (EXE5's mask is 0x0C004000: the same here, where its
                // guard has broken to every type with 0x1000.)
                if flags & (types::BODIES | types::NO_GUARD_SPARK | types::BREAKS_GUARD) == 0 {
                    let mark = if test.guard_marks_direction { 1 << hd.flip } else { 1 };
                    self.collision.get_mut(r).guard_dirs |= mark;
                    flags |= hit::GUARD_SPARK;
                }
                let rm = self.collision.get_mut(r);
                rm.acc.hit_flags |= flags;
                rm.acc.hit_flags_by_flip[hd.flip as usize & 1] |= flags;
                return;
            }
        }
        // Air/ground.
        if (rs & types::FLOATING != 0 && hs & types::GROUND_ONLY != 0) || (rs & types::GROUND_ONLY != 0 && hs & types::FLOATING != 0) {
            return;
        }
        if rd.f1 & f1::INVULNERABLE != 0 || (test.guard_before_untouchable && rd.f1 & f1::UNTOUCHABLE != 0) {
            return;
        }
        let rm = self.collision.get_mut(r);
        rm.acc.hit_by |= hd.bit;
        rm.acc.hit_flags |= hs;
        rm.acc.hit_flags_by_flip[hd.flip as usize & 1] |= hs;
        rm.acc.damage_elements |= hd.secondary_element;
        if hd.status_base.is_some() {
            rm.status_final = hd.status_base;
        }
        // An object that isn't a body (an attack) touching a body that
        // isn't invulnerable: the type of the panel under the body does what
        // its `hit` says (EXE6's ice: an aqua attack freezes the body; EXE5
        // has none, docs/design/exe5-map.md §15.3 item 4).
        if hs & types::BODIES != 0
            && rs & types::BODIES == 0
            && hd.status_timers[timer::INVULNERABLE] == 0
            && rd.acc.hit_flags & hit::GUARDED == 0
            && let Some(kind) = self.field.panel(hd.panel.x, hd.panel.y).map(|p| p.kind)
            && let Some(body) = hd.parent
        {
            self.call_panel(kind, nettai_content_api::PanelCall::Hit { body, element: rd.element });
        }
        let c = hd.counter_byte;
        let rm = self.collision.get_mut(r);
        if rm.counter_timer != 0 && c & 0x7F != 0 && c & 0x80 == 0 {
            rm.acc.hit_flags |= hit::COUNTERED;
            rm.acc.hit_flags_by_flip[hd.flip as usize & 1] |= hit::COUNTERED;
        }
        // What the hit wears off the mood: the byte's low seven bits, but
        // on a counter hit what the counter puts in the register they're
        // read from: none past EXE6's mark (0x8000), 0x7F of EXE4's 0xFF
        // (the rules' `emotion.hit_mood`).
        let counter_counts = self.game_rules().emotion.hit_mood == crate::content::HitMood::CounterCounts;
        let rm = self.collision.get_mut(r);
        let mut mood_damage = (c & 0x7F) as u16;
        if c & 0x80 == 0 && c & 0x7F != 0 {
            if rm.counter_timer != 0 {
                if counter_counts {
                    // EXE4's 0x08012C22: the counter counts 0xFF.
                    rm.acc.counter = rm.acc.counter.wrapping_add(0xFF);
                    mood_damage = 0x7F;
                } else {
                    rm.acc.counter = 0x8000;
                    mood_damage = 0;
                }
            } else {
                rm.acc.counter = rm.acc.counter.wrapping_add((c & 0x7F) as u16);
            }
        }
        rm.acc.mood_damage = rm.acc.mood_damage.wrapping_add(mood_damage);
        if hs & types::DRAIN != 0 {
            rm.acc.drain_hits = rm.acc.drain_hits.wrapping_add(1);
        }
        rm.hit_mod_final |= hd.hit_mod_base;
        // EXE5 also keeps it by the hitter's flip (0x08016AA6: the hitter's
        // collision's +5, not its side's +4).
        rm.hit_mod_by_side[hd.flip as usize & 1] |= hd.hit_mod_base;
        if hd.bugs & 0xFF != 0 {
            rm.acc.inflicted_bugs = hd.bugs;
        }
        // Multiplier.
        // (The hit kernel's table.)
        let w1 = self
            .content
            .rules()
            .element_weakness
            .get(rd.element as usize)
            .and_then(|row| row.get(hd.element as usize))
            .copied()
            .unwrap_or(0);
        let w2 = (rd.secondary_weakness & hd.secondary_element != 0 || (rd.secondary_weakness == SecondaryElements::SWORD && hs & types::SWORD != 0))
            as u8;
        let mut m = 1 + w1 + w2;
        rm.acc.damage_multiplier = m - 1;
        let thaw = rd.f1 & f1::FROZEN != 0 && (rm.acc.hit_flags & types::BREAK != 0 || hd.secondary_element == SecondaryElements::BREAK);
        if thaw {
            m += 1;
        }
        // (EXE6's bubble; EXE5's kernel has none: its flag 0x80000000 is a
        // body under the sea's surface, `hit_test.bubbled_as_submerged`,
        // whose elec hits its panel doubles.)
        if !test.bubbled_as_submerged && rd.f1 & f1::BUBBLED != 0 && hd.element == 3 {
            m += 1;
        }
        rm.acc.exclamation = m - 1;
        if hd.element == 3 {
            rm.acc.elec_damage = rm.acc.elec_damage.wrapping_add(hd.self_damage);
        }
        let e = (hd.element as usize).min(5);
        rm.acc.element_damage[e] = rm.acc.element_damage[e].wrapping_add(hd.self_damage.wrapping_mul(m as u16));
        let bonus = self.panel_bonus(&rd, &hd);
        let rm = self.collision.get_mut(r);
        if bonus {
            rm.acc.element_damage[0] = rm.acc.element_damage[0].wrapping_add(hd.self_damage);
        }
        if thaw {
            if let Some(p) = rd.parent {
                crate::kinds::thaw(self, p);
            }
        }
    }

    /// Whether a hit of `hd`'s counts once more as null damage on `rd`'s
    /// panel: by the panel type's rule, `doubles` (EXE6's fire on grass,
    /// `applyHeatOnGrassDamage_300766c`; EXE5's 0x08016AF6 and elec on its
    /// sea; EXE4's 0x08012CF2 and elec on ice).
    fn panel_bonus(&self, rd: &CollisionData, hd: &CollisionData) -> bool {
        let Some(p) = self.field.panel(rd.panel.x, rd.panel.y) else { return false };
        self.content.rules().panels.rule(p.kind).doubles == Some(hd.element)
    }

    /// `sub_3007692`: the unfiltered channel barriers look at. (EXE5's,
    /// 0x08017494, has no FloatShoe test.)
    fn accumulate_raw(&mut self, r: CollisionId, h: CollisionId) {
        let hd = *self.collision.get(h);
        let rd = *self.collision.get(r);
        if self.is_dimmed() && !(rd.f1 & f1::HIT_WHILE_DIMMED != 0 || hd.self_flags & types::HITS_WHILE_DIMMED != 0) {
            return;
        }
        let self_bit = self.content.rules().hit_test.float_shoe_needs_self_bit;
        if self_bit && ((hd.f1 & f1::FLOATSHOE != 0 && rd.self_flags & types::REACHES_FLOATSHOE == 0)
                || (rd.f1 & f1::FLOATSHOE != 0 && hd.self_flags & types::REACHES_FLOATSHOE == 0)) {
            return;
        }
        let bonus = self.panel_bonus(&rd, &hd);
        let rm = self.collision.get_mut(r);
        rm.acc.raw_hit_flags |= hd.self_flags;
        rm.acc.raw_elements |= hd.secondary_element;
        let e = (hd.element as usize).min(5);
        rm.acc.raw_element_damage[e] = rm.acc.raw_element_damage[e].wrapping_add(hd.self_damage);
        if bonus {
            rm.acc.raw_element_damage[0] = rm.acc.raw_element_damage[0].wrapping_add(hd.self_damage);
        }
    }

    /// `sub_3007708`: hitboxes passing over panels convert them.
    fn convert_panel(&mut self, x: u8, y: u8, id: CollisionId) {
        if self.paused {
            return;
        }
        let s = self.collision.get(id);
        if s.self_flags & types::BODIES != 0 {
            return;
        }
        let e = s.element;
        let Some(p) = self.field.panel(x, y) else { return };
        // (EXE6: fire on grass, aqua on volcano, wood on roads; EXE5's
        // 0x08016D14 the same with lava and magnet.)
        let cleared_by = self.content.rules().panels.rule(p.kind).cleared_by;
        if cleared_by == Some(e) {
            self.set_panel_type(x, y, self.content.rules().panels.roles.normal);
        }
    }

    /// `object_updateCollisionPanels`: move the anchor to the object's panel
    /// and record the direction of the move, as the game puts it (the
    /// reactions section's `move_direction`: EXE6's `sub_800E994`, EXE4's
    /// 0x0800AF90).
    pub fn update_collision_panels(&mut self, obj: ObjectRef) {
        let o = self.objects.get(obj);
        let Some(id) = o.collision else { return };
        let (new, alliance) = (o.panel, o.alliance);
        let rule = self.content.rules().move_direction;
        let s = self.collision.get_mut(id);
        s.direction = rule.of(s.panel, new, alliance);
        s.panel = new;
    }
}

/// `sub_8019F44`: decode a damage word (the rule `effects.damage_word`):
/// the damage under its mask, doubled with 0x8000, then its flag bits in
/// the game's order; `r1` is what the caller left there, a row-offset bug
/// code's high byte.
fn decode_damage_word(s: &mut CollisionData, r1: u16, word: &DamageWord, roles: &crate::content::Roles) {
    let d = s.self_damage;
    s.self_damage = d & word.damage;
    if d & 0x8000 != 0 {
        s.self_damage = s.self_damage.wrapping_mul(2);
    }
    for f in &word.flags {
        if d & f.bit == 0 {
            continue;
        }
        if let Some(role) = f.status {
            s.status_base = Some(roles.status(role));
        }
        if let Some(m) = f.hit_modifier {
            s.hit_mod_base = m;
        }
        if let Some(code) = f.bug {
            s.bugs = if f.bug_high_row { (r1 << 8).wrapping_add(code) } else { code };
        }
        if f.stop {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::battle_flags;
    use crate::content::{HitTest, testing};
    use std::sync::Arc;

    /// The test content's hit test (EXE6's guard: it breaks to 0x2), and one
    /// whose guard breaks to 0x1002 (EXE5's).
    fn tests() -> [HitTest; 2] {
        let plain = testing::rules().hit_test;
        assert_eq!(plain.guard_breaks_to, types::BREAK);
        [plain, HitTest { guard_breaks_to: types::BREAKS_GUARD | types::BREAK, ..plain }]
    }

    /// A fight on the test content whose arena tests hits by `test`: the
    /// battle and its two navis' collisions (side 0's, side 1's).
    fn fight(test: HitTest) -> (Battle, [CollisionId; 2]) {
        let mut c: Content = testing::build();
        c.define().unwrap_or_else(|e| panic!("{e}"));
        {
            let rules = c.rules_mut();
            rules.hit_test = test;
        }
        let c = Arc::new(c);
        let mut setup = testing::round_setup(testing::LINK_BATTLE, testing::megaman_on(&c));
        crate::content::testing::on(&mut setup, &c);
        let mut b = Battle::new(setup, c);
        b.spawn_actors();
        b.run_objects();
        b.round.flags |= battle_flags::FIGHTING;
        let ids = [0, 1].map(|side| {
            let p = b.player(side).unwrap();
            b.objects.get(p).collision.unwrap()
        });
        (b, ids)
    }

    /// Side 1's guarding navi hit by side 0's navi as an attack of types
    /// `types`: whether the guard held (the hitter told it was guarded).
    fn guarded(test: HitTest, types: u32) -> bool {
        let (mut b, [h, r]) = fight(test);
        b.collision.get_mut(r).f1 |= f1::GUARD;
        b.collision.get_mut(h).self_flags = types;
        b.collision.get_mut(h).f1 = 0;
        b.resolve_hit(r, h);
        b.collision.get(h).acc.hit_flags & hit::GUARDED != 0
    }

    /// docs/design/exe5-map.md §15.11: EXE5's guard breaks on type 0x1000
    /// whatever 0x4000 (EXE6's only with it); with both, either breaks.
    /// (EXE5's guarded-direction mask, 0x0C004000 to EXE6's 0x0C005000,
    /// differs only for a hit of 0x1000, which EXE5's guard never holds.)
    #[test]
    fn exe5_guard_breaks_on_0x1000() {
        let [breaks_to_2, breaks_to_1002] = tests();
        assert!(guarded(breaks_to_2, types::ATTACK_SIDE0 | types::BREAKS_GUARD));
        assert!(!guarded(breaks_to_1002, types::ATTACK_SIDE0 | types::BREAKS_GUARD));
        assert!(!guarded(breaks_to_2, types::ATTACK_SIDE0 | types::NO_GUARD_SPARK | types::BREAKS_GUARD));
        assert!(!guarded(breaks_to_1002, types::ATTACK_SIDE0 | types::NO_GUARD_SPARK | types::BREAKS_GUARD));
        assert!(guarded(breaks_to_2, types::ATTACK_SIDE0));
        assert!(guarded(breaks_to_1002, types::ATTACK_SIDE0));
    }

    /// docs/design/exe4-map.md §18 item 5: EXE4's kernel tests an untouchable
    /// receiver with the invulnerable one, after its guard, which still turns
    /// a hit aside; EXE6's before (no hit at all). Neither damages it.
    #[test]
    fn an_untouchable_guard_turns_hits_aside_where_the_rules_say() {
        let outcome = |after: bool| {
            let (mut b, [h, r]) = fight(HitTest { guard_before_untouchable: after, ..tests()[1] });
            b.collision.get_mut(r).f1 |= f1::GUARD | f1::UNTOUCHABLE;
            b.collision.get_mut(h).self_flags = types::ATTACK_SIDE0;
            b.collision.get_mut(h).f1 = 0;
            b.resolve_hit(r, h);
            (b.collision.get(h).acc.hit_flags & hit::GUARDED != 0, b.collision.get(r).acc.hit_flags & hit::GUARD_SPARK != 0)
        };
        assert_eq!(outcome(false), (false, false));
        assert_eq!(outcome(true), (true, true));
        // Without a guard, untouchable either way.
        let (mut b, [h, r]) = fight(HitTest { guard_before_untouchable: true, ..tests()[1] });
        b.collision.get_mut(r).f1 |= f1::UNTOUCHABLE;
        b.collision.get_mut(h).self_flags = types::ATTACK_SIDE0;
        b.collision.get_mut(h).f1 = 0;
        b.resolve_hit(r, h);
        assert_eq!(b.collision.get(r).acc.hit_flags, 0);
    }

    /// docs/design/exe4-map.md §18 item 4: a hit counts once more on a body
    /// standing on a panel whose type doubles its element: the test
    /// content's grass fire (EXE6's), and ice elec as EXE4's states it.
    #[test]
    fn a_panel_doubles_its_types_element() {
        let (mut b, [h, r]) = fight(tests()[0]);
        let bonus = |b: &Battle, element: u8| {
            let mut hd = *b.collision.get(h);
            hd.element = element;
            b.panel_bonus(b.collision.get(r), &hd)
        };
        let p = b.collision.get(r).panel;
        b.set_panel_type(p.x, p.y, crate::content::testing::panel("grass"));
        assert_eq!((bonus(&b, 1), bonus(&b, 3)), (true, false));
        b.set_panel_type(p.x, p.y, crate::content::testing::panel("ice"));
        assert_eq!((bonus(&b, 1), bonus(&b, 3)), (false, false), "the test content's ice doubles nothing");
        let mut c = (*b.content).clone();
        c.rules_mut().panels.types[crate::content::testing::panel("ice").0 as usize].doubles = Some(3);
        b.content = Arc::new(c);
        assert_eq!((bonus(&b, 1), bonus(&b, 3)), (false, true));
    }

    /// docs/design/exe4-map.md §18 item 1: a move's direction as the game
    /// puts it. EXE6's and EXE5's by the side (back and forward swap for
    /// side 1; two panels ahead is other), EXE4's whatever the side (across
    /// before up and down, never other).
    #[test]
    fn a_moves_direction_is_the_games() {
        use crate::content::MoveDirection::{Absolute, BySide};
        let p = |x, y| PanelPos { x, y };
        let from = p(3, 2);
        let moves = [p(2, 2), p(4, 2), p(3, 1), p(3, 3), p(5, 2), p(1, 2), p(4, 3), p(3, 2)];
        let by = |rule: crate::content::MoveDirection, side| moves.map(|to| rule.of(from, to, side));
        // (The other side's mirrored: as side 0's (the canonical side's,
        // `CANONICAL_SIDE`), two panels left is ahead, other; two right
        // back.)
        if nettai_content_api::CANONICAL_SIDE == 0 {
            assert_eq!(by(BySide, 0), [3, 4, 1, 2, 5, 3, 5, 0]);
            assert_eq!(by(BySide, 1), [4, 3, 1, 2, 3, 5, 5, 0]);
        } else {
            assert_eq!(by(BySide, 1), [4, 3, 1, 2, 5, 4, 5, 0]);
            assert_eq!(by(BySide, 0), [3, 4, 1, 2, 4, 5, 5, 0]);
        }
        assert_eq!(by(Absolute, 0), [3, 4, 1, 2, 4, 3, 4, 0]);
        assert_eq!(by(Absolute, 1), [3, 4, 1, 2, 4, 3, 4, 0]);
    }

    /// A body off the field moves by the panels' whole difference, as the
    /// game subtracts two unsigned bytes in 32 bits (EXE4's 0x0800AF90 after
    /// the `ldrb`s at 0x08012D9E, EXE6's `sub_800E994` after 0x0801A050's).
    /// An EXE4 navi stepping from X 128 to 127 moves left; it overflowed a
    /// signed byte.
    #[test]
    fn an_off_field_move_is_the_bytes_difference() {
        use crate::content::MoveDirection::{Absolute, BySide};
        let p = |x| PanelPos { x, y: 2 };
        let c = nettai_content_api::CANONICAL_SIDE;
        let back = if c == 0 { 3 } else { 4 };
        assert_eq!(Absolute.of(p(128), p(127), 0), 3);
        assert_eq!(Absolute.of(p(0), p(200), 0), 4);
        assert_eq!(Absolute.of(p(200), p(0), 0), 3);
        assert_eq!(BySide.of(p(0), p(200), c), 5);
        assert_eq!(BySide.of(p(200), p(0), c), back);
    }
}
