//! Collision: hurtboxes and hitboxes registered on panels.
//!
//! There is no global collision pass. Each collidable object, during its
//! own update, first *removes* its registration (resolving hits against
//! whatever is registered on its panels right now), acts on the results,
//! then *presents* itself again. Results land in accumulators both sides
//! read on their next update. See docs/engine/field-collision-damage.md §3.

use crate::battle::Battle;
use crate::content::{Content, DamageWord, Region, RegionRole, SparkRole, StatusRole};
use crate::field::{self, PanelType};
use crate::object::{ObjectRef, PanelPos};
use nettai_content_api::{CollisionHandle, RegionHandle, SparkHandle, StatusHandle};

pub const SLOTS: usize = 32;

/// A handle to a collision slot. Like the game's pointers, a handle can
/// outlive the slot's allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CollisionId(pub u8);

/// Status flags (`f1`).
pub mod f1 {
    pub const GUARD: u32 = 0x1;
    pub const INVISIBLE: u32 = 0x2;
    /// Submerged: collides only with collision types that have bit 0x8 or
    /// 0x1000, in both directions (`sub_3007218`). DiveMan's AI sets it
    /// while he dives (`sub_80FDEFC`). The timed form (`timer::SUBMERGED`)
    /// is held while its timer runs and no action is in use (`sub_8010162`);
    /// its only starter, actor #0x5D variant 1 (`sub_80C49E4`), is never
    /// spawned. See field-collision-damage.md §4.10.1.
    pub const SUBMERGED: u32 = 0x4;
    pub const INVULNERABLE: u32 = 0x8;
    pub const AIRSHOE: u32 = 0x10;
    pub const FLOATSHOE: u32 = 0x20;
    pub const MOVING: u32 = 0x40;
    pub const DEAD: u32 = 0x100;
    pub const FLASHING: u32 = 0x200;
    pub const FLINCHING: u32 = 0x400;
    pub const PARALYZED: u32 = 0x800;
    pub const SLIDING: u32 = 0x1000;
    pub const BLIND: u32 = 0x2000;
    pub const IMMOBILIZED: u32 = 0x4000;
    pub const CONFUSED: u32 = 0x8000;
    pub const FROZEN: u32 = 0x1_0000;
    pub const SUPERARMOR: u32 = 0x2_0000;
    pub const UNDERSHIRT: u32 = 0x4_0000;
    pub const MOVE_COMPLETE: u32 = 0x8_0000;
    pub const DRAG: u32 = 0x10_0000;
    pub const ANGER: u32 = 0x20_0000;
    pub const USING_ACTION: u32 = 0x40_0000;
    pub const AFFECTED_BY_ICE: u32 = 0x200_0000;
    /// Untouchable: `sub_3007218` drops every pair in which either side
    /// has it, so no hit reaches it and it hits nothing; poison panels
    /// don't hurt it either (`sub_801A186`). Falzar Beast Over's form flags
    /// set it (`sub_8014674`), which is why nothing can hit that form.
    pub const UNTOUCHABLE: u32 = 0x800_0000;
    pub const BUBBLED: u32 = 0x8000_0000;
    /// Hits still reach it while the battle is dimmed (`sub_3007218`; else
    /// only hitters set up while dimmed do): ElemTrap's trap sets it.
    pub const HIT_WHILE_DIMMED: u32 = 0x0100_0000;
    /// The status word read through an object's missing collision data
    /// (a null pointer): BIOS memory, which game code can't read, gives
    /// the opcode the BIOS last fetched (open bus). After a software
    /// interrupt, such as the object spawn's fill (`ZeroFillByWord`'s
    /// CpuSet), that is 0xE3A02004: EXE6's, the rule's default
    /// (`content::MissingCollisionStatus`). An interrupt in between leaves
    /// 0xE55EC002, which EXE5's console reads there.
    pub const NULL_READ: u32 = 0xE3A0_2004;
}

/// Per-registration-window hit results (zeroed on present).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Accumulators {
    /// Slots already paired with since the last present.
    pub pair_tested: u32,
    /// Everything that touched us, unfiltered.
    pub raw_hit_flags: u32,
    /// What hit us (filtered), plus: 0x1 we were guarded, 0x40 counter
    /// hit, 0x20000 we blocked a hit.
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
    pub poison_timer: u8,
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
    /// EXE5's +0x50: the ripple over a body under the sea's surface
    /// (0x0800DEB2).
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
        s.self_flags = self.content.collision_type(self_type, o.alliance).0 | if dimmed { 0x1_0000 } else { 0 };
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
        s.self_flags = self.content.collision_type(self_type, alliance).0 | if dimmed && !alone { 0x1_0000 } else { 0 };
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
        let bit = s.bit;
        let whole_field = self.collision.covers_whole_field(&self.content, id);
        for (x, y) in self.collision.region_panels(&self.content, &self.field, id) {
            self.collision.masks[(y * 8 + x) as usize] |= bit;
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
        let panels: Vec<(u8, u8)> = if whole_field {
            let mut v = Vec::new();
            for y in 1..=3u8 {
                for x in 1..=6u8 {
                    v.push((x, y));
                }
            }
            v
        } else {
            self.collision.region_panels(&self.content, &self.field, id)
        };
        for (x, y) in panels {
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
        if self.is_dimmed() && !(rd.f1 & f1::HIT_WHILE_DIMMED != 0 || hd.self_flags & 0x1_0000 != 0) {
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
        if (f & 0x202 != 0 && rs & 0x4 == 0)
            || (f & submerged != 0 && rs & 0x1008 == 0 && !elec(rd.element))
            || (f & 0x0080_0000 != 0 && rs & 0x0C00_3000 == 0)
            || f & f1::UNTOUCHABLE != 0
            || (self_bit && f & 0x20 != 0 && rs & 0x80 == 0)
        {
            return;
        }
        // The receiver's state against the hitter's type (EXE4's leaves its
        // untouchable state to the invulnerable test, after the guard).
        let f = rd.f1;
        let hs = hd.self_flags;
        if (f & 0x202 != 0 && hs & 0x4 == 0)
            || (f & submerged != 0 && hs & 0x1008 == 0 && !elec(hd.element))
            || (f & 0x0080_0000 != 0 && hs & 0x3000 == 0)
            || (!test.guard_before_untouchable && f & f1::UNTOUCHABLE != 0)
            || (self_bit && f & 0x20 != 0 && hs & 0x80 == 0)
        {
            return;
        }
        // Guard.
        if rd.f1 & f1::GUARD != 0 {
            let brk = if hs & 0x4000 != 0 { 0x1002 } else { test.guard_breaks_to };
            if hs & brk == 0 {
                let hm = self.collision.get_mut(h);
                hm.acc.hit_flags |= 1;
                hm.acc.hit_flags_by_flip[rd.flip as usize & 1] |= 1;
                let mut flags = hs & !0x10;
                // (EXE5's mask is 0x0C004000: the same here, where its
                // guard has broken to every type with 0x1000.)
                if flags & 0x0C00_5000 == 0 {
                    let mark = if test.guard_marks_direction { 1 << hd.flip } else { 1 };
                    self.collision.get_mut(r).guard_dirs |= mark;
                    flags |= 0x2_0000;
                }
                let rm = self.collision.get_mut(r);
                rm.acc.hit_flags |= flags;
                rm.acc.hit_flags_by_flip[hd.flip as usize & 1] |= flags;
                return;
            }
        }
        // Air/ground.
        if (rs & 0x0010_0000 != 0 && hs & 0x8000 != 0) || (rs & 0x8000 != 0 && hs & 0x0010_0000 != 0) {
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
        // Aqua on ice: freeze a body standing on ice (in a game that has
        // the freeze: the arena's role `statuses.ice_freeze`; EXE5 has none,
        // docs/design/exe5-map.md §15.3 item 4).
        if let Some(freeze) = self.roles().try_status(StatusRole::IceFreeze)
            && rd.element == 2
            && hs & 0x0C00_0000 != 0
            && rs & 0x0C00_0000 == 0
            && hd.status_timers[timer::INVULNERABLE] == 0
            && rd.acc.hit_flags & 1 == 0
            && self.field.panel(hd.panel.x, hd.panel.y).map(|p| p.kind) == Some(PanelType::Ice)
        {
            self.set_panel_type(hd.panel.x, hd.panel.y, PanelType::Normal);
            self.collision.get_mut(h).status_final = Some(freeze);
        }
        let c = hd.counter_byte;
        let rm = self.collision.get_mut(r);
        if rm.counter_timer != 0 && c & 0x7F != 0 && c & 0x80 == 0 {
            rm.acc.hit_flags |= 0x40;
            rm.acc.hit_flags_by_flip[hd.flip as usize & 1] |= 0x40;
        }
        // What the hit wears off the mood: the byte's low seven bits, but
        // on a counter hit what the counter puts in the register they're
        // read from: none past EXE6's mark (0x8000), 0x7F of EXE4's 0xFF
        // (the rules' `emotion.hit_mood`).
        let hitter_gains = self.game_rules().emotion.hit_mood == crate::content::HitMood::HitterGains;
        let rm = self.collision.get_mut(r);
        let mut mood_damage = (c & 0x7F) as u16;
        if c & 0x80 == 0 && c & 0x7F != 0 {
            if rm.counter_timer != 0 {
                if hitter_gains {
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
        if hs & 0x100 != 0 {
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
        let w2 = (rd.secondary_weakness & hd.secondary_element != 0 || (rd.secondary_weakness == 0x80 && hs & 0x2000 != 0))
            as u8;
        let mut m = 1 + w1 + w2;
        rm.acc.damage_multiplier = m - 1;
        let thaw = rd.f1 & f1::FROZEN != 0 && (rm.acc.hit_flags & 2 != 0 || hd.secondary_element == 0x10);
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
        self.content.rules().panels.types[p.kind as usize].doubles == Some(hd.element)
    }

    /// `sub_3007692`: the unfiltered channel barriers look at. (EXE5's,
    /// 0x08017494, has no FloatShoe test.)
    fn accumulate_raw(&mut self, r: CollisionId, h: CollisionId) {
        let hd = *self.collision.get(h);
        let rd = *self.collision.get(r);
        if self.is_dimmed() && !(rd.f1 & f1::HIT_WHILE_DIMMED != 0 || hd.self_flags & 0x1_0000 != 0) {
            return;
        }
        let self_bit = self.content.rules().hit_test.float_shoe_needs_self_bit;
        if self_bit && ((hd.f1 & 0x20 != 0 && rd.self_flags & 0x80 == 0) || (rd.f1 & 0x20 != 0 && hd.self_flags & 0x80 == 0)) {
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
        if s.self_flags & 0x0C00_0000 != 0 {
            return;
        }
        let e = s.element;
        let Some(p) = self.field.panel(x, y) else { return };
        // (EXE6: fire on grass, aqua on volcano, wood on roads; EXE5's
        // 0x08016D14 the same with lava and metal.)
        let cleared_by = self.content.rules().panels.types[p.kind as usize].cleared_by;
        if cleared_by == Some(e) {
            self.set_panel_type(x, y, PanelType::Normal);
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
        assert_eq!(plain.guard_breaks_to, 0x0002);
        [plain, HitTest { guard_breaks_to: 0x1002, ..plain }]
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
        b.collision.get(h).acc.hit_flags & 1 != 0
    }

    /// docs/design/exe5-map.md §15.11: EXE5's guard breaks on type 0x1000
    /// whatever 0x4000 (EXE6's only with it); with both, either breaks.
    /// (EXE5's guarded-direction mask, 0x0C004000 to EXE6's 0x0C005000,
    /// differs only for a hit of 0x1000, which EXE5's guard never holds.)
    #[test]
    fn exe5_guard_breaks_on_0x1000() {
        let [breaks_to_2, breaks_to_1002] = tests();
        assert!(guarded(breaks_to_2, 0x8000_1000));
        assert!(!guarded(breaks_to_1002, 0x8000_1000));
        assert!(!guarded(breaks_to_2, 0x8000_5000));
        assert!(!guarded(breaks_to_1002, 0x8000_5000));
        assert!(guarded(breaks_to_2, 0x8000_0000));
        assert!(guarded(breaks_to_1002, 0x8000_0000));
    }

    /// docs/design/exe4-map.md §18 item 5: EXE4's kernel tests an untouchable
    /// receiver with the invulnerable one, after its guard, which still turns
    /// a hit aside; EXE6's before (no hit at all). Neither damages it.
    #[test]
    fn an_untouchable_guard_turns_hits_aside_where_the_rules_say() {
        let outcome = |after: bool| {
            let (mut b, [h, r]) = fight(HitTest { guard_before_untouchable: after, ..tests()[1] });
            b.collision.get_mut(r).f1 |= f1::GUARD | f1::UNTOUCHABLE;
            b.collision.get_mut(h).self_flags = 0x8000_0000;
            b.collision.get_mut(h).f1 = 0;
            b.resolve_hit(r, h);
            (b.collision.get(h).acc.hit_flags & 1 != 0, b.collision.get(r).acc.hit_flags & 0x2_0000 != 0)
        };
        assert_eq!(outcome(false), (false, false));
        assert_eq!(outcome(true), (true, true));
        // Without a guard, untouchable either way.
        let (mut b, [h, r]) = fight(HitTest { guard_before_untouchable: true, ..tests()[1] });
        b.collision.get_mut(r).f1 |= f1::UNTOUCHABLE;
        b.collision.get_mut(h).self_flags = 0x8000_0000;
        b.collision.get_mut(h).f1 = 0;
        b.resolve_hit(r, h);
        assert_eq!(b.collision.get(r).acc.hit_flags, 0);
    }

    /// docs/design/exe4-map.md §18 item 15: EXE4's destroy unregisters the
    /// object's collision (`effects.destroy`), so its panel no longer
    /// carries its bit; EXE6's frees it as it is, the bit left stale.
    #[test]
    fn a_destroy_unregisters_where_the_rules_say() {
        use crate::content::DestroyRule;
        let run = |rule: DestroyRule| {
            let (mut b, [h, _]) = fight(tests()[0]);
            let mut c = (*b.content).clone();
            c.rules_mut().effects.destroy = rule;
            b.content = Arc::new(c);
            let r = b.player(0).unwrap();
            b.present_collision(h);
            let (p, bit) = (b.collision.get(h).panel, b.collision.get(h).bit);
            let at = (p.y * 8 + p.x) as usize;
            assert_ne!(b.collision.masks[at] & bit, 0, "registered");
            crate::kinds::generic_destroy(&mut b, r);
            b.collision.masks[at] & bit != 0
        };
        assert!(!run(DestroyRule::Unregisters));
        assert!(run(DestroyRule::Frees));
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
        b.set_panel_type(p.x, p.y, PanelType::Grass);
        assert_eq!((bonus(&b, 1), bonus(&b, 3)), (true, false));
        b.set_panel_type(p.x, p.y, PanelType::Ice);
        assert_eq!((bonus(&b, 1), bonus(&b, 3)), (false, false), "the test content's ice doubles nothing");
        let mut c = (*b.content).clone();
        c.rules_mut().panels.types[PanelType::Ice as usize].doubles = Some(3);
        b.content = Arc::new(c);
        assert_eq!((bonus(&b, 1), bonus(&b, 3)), (false, true));
    }

    /// docs/design/exe4-map.md §18 item 1: a move's direction as the game
    /// puts it. EXE6's and EXE5's by the side (back and forward swap for
    /// side 1; two panels right is other), EXE4's whatever the side (across
    /// before up and down, never other).
    #[test]
    fn a_moves_direction_is_the_games() {
        use crate::content::MoveDirection::{Absolute, BySide};
        let p = |x, y| PanelPos { x, y };
        let from = p(3, 2);
        let moves = [p(2, 2), p(4, 2), p(3, 1), p(3, 3), p(5, 2), p(1, 2), p(4, 3), p(3, 2)];
        let by = |rule: crate::content::MoveDirection, side| moves.map(|to| rule.of(from, to, side));
        assert_eq!(by(BySide, 0), [3, 4, 1, 2, 5, 3, 5, 0]);
        assert_eq!(by(BySide, 1), [4, 3, 1, 2, 5, 4, 5, 0]);
        assert_eq!(by(Absolute, 0), [3, 4, 1, 2, 4, 3, 4, 0]);
        assert_eq!(by(Absolute, 1), [3, 4, 1, 2, 4, 3, 4, 0]);
    }
}
