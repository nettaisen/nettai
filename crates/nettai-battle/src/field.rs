//! The field: an 8x5 grid of panels (the playable area is x 1..=6,
//! y 1..=3). See docs/engine/field-collision-damage.md §2.

use crate::battle::Battle;
use crate::collision::Collision;
use crate::content::{PanelCondition, SoundRole};
use crate::sound::SoundId;
use crate::object::{ObjectRef, PanelPos, Vec3};

/// A panel type: one of its game's, by the game's number of it, its place
/// in the rules' list (`PanelRules::types`, named by `PanelRules::names`):
/// the number a panel's flags word holds in its low nibble, the stages'
/// layouts and a panel trail's byte name. What a type does is its game's
/// rules'; the types the engine's own code needs (the missing panel, a
/// break's and a crack's, the normal one) are the rules' roles
/// (`PanelRoles`). In a content file, the type's name, which the game's
/// panel rules say the number of (a definition's names are read while its
/// game's content is defined: [`reading_panel_types`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PanelType(pub u8);

thread_local! {
    /// The panel types of the game whose definitions are being read (the
    /// rules' `panels.numbers`), which a type's name is one of.
    static READING: std::cell::RefCell<Option<Vec<String>>> = const { std::cell::RefCell::new(None) };
}

/// `f`, with a panel type's name read as one of `names` (the game's, by
/// number, as its content is defined).
pub fn reading_panel_types<T>(names: &[String], f: impl FnOnce() -> T) -> T {
    let before = READING.with(|r| r.replace(Some(names.to_vec())));
    let out = f();
    READING.with(|r| *r.borrow_mut() = before);
    out
}

/// The panel type `name` names among the game's being read.
pub(crate) fn reading_panel_type(name: &str) -> Result<PanelType, String> {
    READING.with(|r| match r.borrow().as_ref() {
        Some(names) => names
            .iter()
            .position(|n| n == name)
            .map(|i| PanelType(i as u8))
            .ok_or_else(|| format!("{name:?} is none of its game's panel types ({})", names.join(", "))),
        None => Err(format!("panel type {name:?}: no game's panel types are being read")),
    })
}

impl serde::Serialize for PanelType {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(self.0)
    }
}

impl<'de> serde::Deserialize<'de> for PanelType {
    /// A type's name (one of the game's being read), or its number.
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<PanelType, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Named {
            Number(u8),
            Name(String),
        }
        match Named::deserialize(d)? {
            Named::Number(n) => Ok(PanelType(n)),
            Named::Name(name) => reading_panel_type(&name).map_err(serde::de::Error::custom),
        }
    }
}

/// Bits of a panel's cached flags word.
pub mod pflags {
    /// The panel's type, by the game's number of it.
    pub const TYPE_MASK: u32 = 0x0F;
    pub const SOLID: u32 = 0x10;
    pub const ALLIANCE_1: u32 = 0x20;
    pub const CRACKED: u32 = 0x40;
    pub const RESERVED: u32 = 0x80;
    pub const BODY_SIDE0: u32 = 0x0800_0000;
    pub const BODY_SIDE1: u32 = 0x0400_0000;
    pub const FLOATING: u32 = 0x0010_0000;
    /// Bodies, neutral objects, blockers and reservations.
    pub const OCCUPIED: u32 = 0x0F88_0080;
    /// Any body.
    pub const BODY: u32 = 0x0F80_0000;
}

/// `Panel::crumble_timer`'s bit: waiting for a grounded body before it counts
/// (EXE4's panel +0x12 bit 15).
pub const CRUMBLE_ARMED: u16 = 0x8000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Panel {
    pub visible: bool,
    pub highlight: u8,
    pub kind: PanelType,
    /// Current owner.
    pub alliance: u8,
    /// Owner at the start of the round (for stolen-area return).
    pub home: u8,
    pub display_kind: PanelType,
    pub display_alliance: u8,
    /// `object_setPanelTypeBlink`: drawn this frame as this type and side
    /// instead (drawn only; the game's renderer shows it once and clears
    /// it).
    pub blink: Option<(PanelType, u8)>,
    pub x: u8,
    pub y: u8,
    pub front_edge: bool,
    /// Hole timer: counts down while broken.
    pub hole_timer: u16,
    pub return_blink: u16,
    /// Counts down while the panel is a type that expires (EXE6's roads,
    /// EXE5's lava and sea: `PanelTypeRule::expires`), set to the type's
    /// ticks as the panel becomes it.
    pub expire_timer: u16,
    /// A type that crumbles (EXE4's pitfall: `PanelTypeRule::crumbles`): the
    /// ticks it has left, counting while `CRUMBLE_ARMED` is clear; a stage's
    /// panel starts armed (counting from the tick a grounded body stands on
    /// it), one a type change makes counts at once (EXE4's panel +0x12).
    pub crumble_timer: u16,
    /// Cached flags: type bits, owner, reservation and the collision types
    /// of everything registered on the panel.
    pub flags: u32,
    /// Last tick's flags for cracked panels (0 otherwise).
    pub latch: u32,
    pub reserver: Option<ObjectRef>,
}

/// Per-column stolen-area bookkeeping.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Column {
    pub home: u8,
    pub return_ready: u8,
    pub stolen: u8,
    pub stolen_free: u8,
    pub stolen_mask: u8,
    pub free_mask: u8,
    pub timer: u16,
}

/// A run of same-owner columns, for stolen-area return (outermost first).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HomeRun {
    pub start: u8,
    pub dir: i8,
    pub count: u8,
    pub owner: u8,
}

#[derive(Clone, Debug, Hash)]
pub struct Field {
    /// Indexed [y][x].
    pub panels: [[Panel; 8]; 5],
    pub columns: [Column; 8],
    pub home_runs: Vec<HomeRun>,
    /// Volcano eruption cycle (period 0x8C).
    pub volcano_counter: u32,
    /// Ticks a broken panel stays broken.
    pub hole_ticks: u16,
    /// Obstacles on the field, per side.
    pub objects: FieldObjects,
    /// Each side's wind (Wind and Fan's fan, a navi's own).
    pub winds: [Wind; 2],
}

/// A side's wind (BattleState+0xC0 + 4·side, and its source at +0xC8):
/// the object blowing for the side, one at a time (`sub_80E541A`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Wind {
    pub object: Option<ObjectRef>,
    pub source: WindSource,
}

/// Who placed a side's wind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WindSource {
    /// Wind and Fan's fan (attack object #0x48; the game's 0).
    #[default]
    Obstacle,
    /// A navi's own wind (effect object #0x41; 1), which a fan's can't
    /// replace (`sub_80E532C`).
    Navi,
}

/// The field-object registry (BattleState+0xA0..+0xC0): the obstacles
/// (rocks, cubes...) each side owns, so that placing too many evicts the
/// oldest and chips can find them all.
///
/// Slots in the game's order: side 0 has two slots for class-0 obstacles
/// (oldest first) and one for class 1, then side 1 the same, then two
/// slots for stage objects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FieldObjects {
    pub slots: [Option<ObjectRef>; 8],
}

impl FieldObjects {
    /// Slots per side.
    const SIDE: usize = 3;
    /// The per-side slots (the first six) that obstacles register in.
    pub const OBSTACLE_SLOTS: usize = 6;

    /// `setFieldBattleObject_800F614` without the eviction's side effect:
    /// record `obj` for `side` in `class` (0 or 1). Returns the object it
    /// evicted, which the caller destroys.
    pub fn register(&mut self, obj: ObjectRef, side: u8, class: u8) -> Option<ObjectRef> {
        assert!(side <= 1 && class <= 1, "field object side {side} class {class}");
        let base = Self::SIDE * side as usize;
        if class == 1 {
            return self.slots[base + 2].replace(obj);
        }
        let (oldest, newer) = (base, base + 1);
        if self.slots[oldest].is_none() {
            self.slots[oldest] = Some(obj);
            None
        } else if self.slots[newer].is_none() {
            self.slots[newer] = Some(obj);
            None
        } else {
            let evicted = self.slots[oldest];
            self.slots[oldest] = self.slots[newer];
            self.slots[newer] = Some(obj);
            evicted
        }
    }

    /// `sub_800F656`: forget `obj` (the six obstacle slots only).
    pub fn unregister(&mut self, obj: ObjectRef) {
        for s in &mut self.slots[..Self::OBSTACLE_SLOTS] {
            if *s == Some(obj) {
                *s = None;
            }
        }
    }

    /// Whether one of the two stage slots is free (`sub_8007450`'s test
    /// before it places a stage object).
    pub fn stage_slot_free(&self) -> bool {
        self.slots[Self::OBSTACLE_SLOTS..].iter().any(|s| s.is_none())
    }

    /// `sub_8007450`'s store: `obj` takes the first free stage slot. False
    /// if both are taken.
    pub fn enter_stage(&mut self, obj: ObjectRef) -> bool {
        match self.slots[Self::OBSTACLE_SLOTS..].iter_mut().find(|s| s.is_none()) {
            Some(slot) => {
                *slot = Some(obj);
                true
            }
            None => false,
        }
    }

    /// A stage object leaves its slot (the original clears the slot whose
    /// address the object keeps).
    pub fn leave_stage(&mut self, obj: ObjectRef) {
        for s in &mut self.slots[Self::OBSTACLE_SLOTS..] {
            if *s == Some(obj) {
                *s = None;
            }
        }
    }

    /// `sub_800F806`: the class `obj` is registered in (None if it isn't).
    pub fn class_of(&self, obj: ObjectRef) -> Option<u8> {
        let i = self.slots[..Self::OBSTACLE_SLOTS].iter().position(|&s| s == Some(obj))?;
        Some(if i % Self::SIDE == 2 { 1 } else { 0 })
    }
}

pub fn is_valid(x: u8, y: u8) -> bool {
    (1..=6).contains(&x) && (1..=3).contains(&y)
}

impl Field {
    /// The field at the start of a round (`sub_800C4BC`).
    /// `rules` are the arena's game's panel rules (docs/design/
    /// rules-in-luau.md §2.3: the field is the battle's).
    pub fn new(rules: &crate::content::PanelRules, layout: &crate::content::PanelLayout, pattern: u8, battle_mode: u8) -> Field {
        // (EXE6: 0x258 ticks, 0x1E0 in battle mode 1; EXE5 600 in every
        // battle, 0x0800A998.)
        let hole_ticks = if battle_mode == 1 { rules.mend_in_battle_mode_1 } else { rules.mend };
        let rows = layout.rows;
        let mut columns = [Column::default(); 8];
        for (x, c) in columns.iter_mut().enumerate() {
            c.home = if (1..=6).contains(&x) { (pattern >> (x - 1)) & 1 } else { 0xFF };
        }
        let mut panels = [[Panel::default(); 8]; 5];
        for y in 0..5u8 {
            for x in 0..8u8 {
                let t = if is_valid(x, y) { rows[y as usize - 1][x as usize - 1] } else { rules.roles.missing };
                let home = columns[x as usize].home;
                panels[y as usize][x as usize] = Panel {
                    visible: rules.start_visible[y as usize][x as usize],
                    kind: t,
                    display_kind: t,
                    alliance: home,
                    home,
                    display_alliance: home,
                    front_edge: rules.front_edges[y as usize][x as usize],
                    x,
                    y,
                    hole_timer: hole_ticks,
                    expire_timer: rules.rule(t).expires.unwrap_or(0),
                    // (EXE4's field arms every panel's, 0x08009120: only a
                    // type that crumbles reads it.)
                    crumble_timer: rules.rule(t).crumbles.map_or(0, |n| CRUMBLE_ARMED | n),
                    ..Panel::default()
                };
            }
        }
        let mut f = Field {
            panels,
            columns,
            home_runs: Vec::new(),
            volcano_counter: 0x8C,
            hole_ticks,
            objects: FieldObjects::default(),
            winds: [Wind::default(); 2],
        };
        f.build_home_runs();
        f
    }

    /// Forget last frame's highlights and blinks (presentation only;
    /// `sub_800C5E0` clears each one it draws).
    pub fn clear_one_frame_looks(&mut self) {
        for p in self.panels.iter_mut().flatten() {
            p.highlight = 0;
            p.blink = None;
        }
    }

    pub fn panel(&self, x: u8, y: u8) -> Option<&Panel> {
        is_valid(x, y).then(|| &self.panels[y as usize][x as usize])
    }

    pub fn panel_mut(&mut self, x: u8, y: u8) -> Option<&mut Panel> {
        is_valid(x, y).then(|| &mut self.panels[y as usize][x as usize])
    }

    /// A panel's flags (0 off the field).
    pub fn flags(&self, x: u8, y: u8) -> u32 {
        self.panel(x, y).map(|p| p.flags).unwrap_or(0)
    }

    /// `object_checkPanelParameters`: on the field, flags nonzero, has
    /// every bit of `set` and none of `clear`.
    pub fn check(&self, x: u8, y: u8, set: u32, clear: u32) -> bool {
        let f = self.flags(x, y);
        f != 0 && f & clear == 0 && f & set == set
    }

    /// Whether panel (x, y) exists and meets `cond`.
    pub fn meets(&self, x: u8, y: u8, cond: PanelCondition) -> bool {
        self.check(x, y, cond.require, cond.forbid)
    }

    pub fn is_solid(&self, x: u8, y: u8) -> bool {
        self.flags(x, y) & pflags::SOLID != 0
    }

    /// Count consecutive columns from `start` in direction `dir` with the
    /// same home side.
    fn run_length(&self, start: u8, dir: i8) -> (u8, u8) {
        let owner = self.columns[start as usize].home;
        let mut n = 0;
        let mut c = start as i8;
        while (1..=6).contains(&c) && self.columns[c as usize].home == owner {
            n += 1;
            c += dir;
        }
        (n, owner)
    }

    /// `sub_800C67C`: runs of same-owner columns, outermost first.
    fn build_home_runs(&mut self) {
        let (mut l, mut r, mut d) = (1i8, 6i8, 1i8);
        self.home_runs.clear();
        while l < r {
            let start = if d > 0 { l } else { r };
            let (cnt, owner) = self.run_length(start as u8, d);
            if cnt == 0 {
                break;
            }
            self.home_runs.push(HomeRun { start: (start + (cnt as i8 - 1) * d) as u8, dir: -d, count: cnt, owner });
            if d > 0 {
                l += cnt as i8;
            } else {
                r -= cnt as i8;
            }
            d = -d;
        }
    }

    /// `_object_updatePanelParameters`: recompute a panel's flags.
    pub fn refresh(&mut self, rules: &crate::content::PanelRules, collision: &Collision, x: u8, y: u8) {
        if !is_valid(x, y) {
            return;
        }
        let occupants = collision.panel_self_flags(x, y);
        let p = &mut self.panels[y as usize][x as usize];
        p.display_kind = p.kind;
        p.display_alliance = p.alliance;
        p.flags = rules.type_flags(p.kind)
            | ((p.alliance as u32) << 5)
            | if p.reserver.is_some() { pflags::RESERVED } else { 0 }
            | occupants;
    }

    /// Refresh every panel (`sub_800C8F0`).
    pub fn refresh_all(&mut self, rules: &crate::content::PanelRules, collision: &Collision) {
        for y in (1..=4).rev() {
            for x in (1..=7).rev() {
                self.refresh(rules, collision, x, y);
            }
        }
    }
}

impl Battle {
    /// `sub_800BFC4`: the per-tick panel update (callers gate it on pause
    /// and dimming).
    pub fn tick_panels(&mut self) {
        self.return_stolen_area();
        self.field.volcano_counter = self.field.volcano_counter.wrapping_sub(1);
        if self.field.volcano_counter == 0 {
            self.field.volcano_counter = 0x8C;
        }
        for y in 1..=3u8 {
            for x in 1..=6u8 {
                self.tick_panel(x, y);
                let cracked = self.content.rules().panels.roles.cracked;
                let p = &mut self.field.panels[y as usize][x as usize];
                p.latch = if p.kind == cracked { p.flags } else { 0 };
            }
        }
    }

    /// `sub_800C380` (EXE5's 0x0800A998: its lava and sea expire as EXE6's
    /// roads do, each with its own timer, which a panel's new type sets).
    fn tick_panel(&mut self, x: u8, y: u8) {
        let h = self.field.hole_ticks;
        let p = self.field.panels[y as usize][x as usize];
        let rules = &self.content.rules().panels;
        let roles = rules.roles;
        let expires = rules.rule(p.kind).expires;
        let crumbles = rules.rule(p.kind).crumbles.is_some();
        let volcano = rules.is_named(p.kind, "volcano");
        if p.kind == roles.missing {
        } else if p.kind == roles.broken {
            let p = &mut self.field.panels[y as usize][x as usize];
            p.hole_timer = p.hole_timer.wrapping_sub(1);
            if p.hole_timer == 0 {
                p.kind = roles.normal;
                self.field.refresh(&self.content.rules().panels, &self.collision, x, y);
                self.field.panels[y as usize][x as usize].hole_timer = h;
                return;
            }
            // Blinks back to normal in its last second.
            let blink = p.hole_timer <= 60 && p.hole_timer & 2 != 0;
            p.display_kind = if blink { roles.normal } else { roles.broken };
        } else if p.kind == roles.cracked {
            let p = &mut self.field.panels[y as usize][x as usize];
            p.hole_timer = h;
            let latch = p.latch;
            if latch & pflags::BODY != 0 && latch & pflags::FLOATING == 0 && p.flags & pflags::OCCUPIED == 0 {
                p.kind = roles.broken;
                self.field.refresh(&self.content.rules().panels, &self.collision, x, y);
                self.field.panels[y as usize][x as usize].hole_timer = h;
                self.sound(SoundRole::PanelCrack);
            }
        } else if volcano {
            let p = &mut self.field.panels[y as usize][x as usize];
            p.hole_timer = h;
            let at = if p.x <= 3 { 0x8C } else { 0x46 };
            if self.field.volcano_counter == at {
                self.erupt(x, y);
            }
        } else if crumbles {
            // A type that crumbles (EXE4's pitfall, 0x0800980E): armed, it
            // waits for a grounded body standing on it (none floating);
            // then it counts down and turns normal, without a blink.
            let p = &mut self.field.panels[y as usize][x as usize];
            p.hole_timer = h;
            if p.crumble_timer & CRUMBLE_ARMED != 0 {
                if p.flags & pflags::FLOATING != 0 || p.flags & pflags::BODY == 0 {
                    return;
                }
                p.crumble_timer &= !CRUMBLE_ARMED;
            }
            if p.crumble_timer != 0 {
                p.crumble_timer -= 1;
                if p.crumble_timer > 0 {
                    return;
                }
            }
            p.kind = roles.normal;
            self.field.refresh(&self.content.rules().panels, &self.collision, x, y);
        } else if let Some(ticks) = expires {
            // A type that expires (EXE6's roads, EXE5's lava and sea): normal
            // when its ticks are up, blinking back in its last second.
            let kind = p.kind;
            let p = &mut self.field.panels[y as usize][x as usize];
            p.hole_timer = h;
            p.expire_timer = p.expire_timer.wrapping_sub(1);
            if p.expire_timer == 0 {
                p.kind = roles.normal;
                self.field.refresh(&self.content.rules().panels, &self.collision, x, y);
                self.field.panels[y as usize][x as usize].expire_timer = ticks;
                return;
            }
            let blink = p.expire_timer <= 60 && p.expire_timer & 2 != 0;
            p.display_kind = if blink { roles.normal } else { kind };
        } else {
            let p = &mut self.field.panels[y as usize][x as usize];
            p.hole_timer = h;
        }
    }

    /// A volcano panel erupts (`sub_80C5B76`): a 50-damage attack object.
    fn erupt(&mut self, x: u8, y: u8) {
        let pos = Vec3 { x: y as i32, y: 0, z: 0 };
        if let Some(r) = crate::kinds::spawn_engine(self, crate::kinds::EngineKind::Eruption, pos, [0x28, 0, 0, 0]) {
            let o = self.objects.get_mut(r);
            o.panel = PanelPos { x, y };
            o.element = 0;
            o.damage = 0x32;
            o.stamina = 0;
        }
    }

    /// `sub_800C746`: stolen columns return to their owner.
    fn return_stolen_area(&mut self) {
        for c in 1..=6u8 {
            let (mut n, mut nf, mut m, mut fm) = (0u8, 0u8, 0u8, 0u8);
            for y in 1..=3u8 {
                let p = &self.field.panels[y as usize][c as usize];
                if p.alliance != p.home {
                    n += 1;
                    m |= 1 << y;
                    if p.flags & pflags::OCCUPIED == 0 {
                        nf += 1;
                        fm |= 1 << y;
                    }
                }
            }
            let col = &mut self.field.columns[c as usize];
            col.stolen = n;
            col.stolen_free = nf;
            col.stolen_mask = m;
            col.free_mask = fm;
            if n == 0 {
                col.timer = 0;
            }
            if col.timer != 0 {
                col.timer -= 1;
                if col.timer == 0 {
                    col.return_ready = 1;
                }
            }
        }
        let runs = self.field.home_runs.clone();
        for run in runs {
            let mut c = run.start as i8;
            let mut ready: Vec<(u8, u8)> = Vec::new();
            for _ in 0..run.count {
                let col = self.field.columns[c as usize];
                if col.stolen_mask != 0 {
                    if col.return_ready != 0 && col.stolen_mask == col.free_mask {
                        ready.push((c as u8, col.free_mask));
                    } else {
                        ready.clear();
                    }
                }
                c += run.dir;
            }
            // The original's loop over the list restarts its index at 0 on
            // every pass, so it returns the list's first column once per
            // listed column and the others wait: a run's columns that come
            // due together return one a tick, the frontmost first.
            if let Some(&(c, mask)) = ready.first() {
                for _ in 0..ready.len() {
                    self.field.columns[c as usize].return_ready = 0;
                    for y in 1..=3u8 {
                        if mask & (1 << y) != 0 {
                            let p = &mut self.field.panels[y as usize][c as usize];
                            p.alliance = run.owner;
                            p.return_blink = (p.return_blink & 0xFF00) | 0x5A;
                            self.field.refresh(&self.content.rules().panels, &self.collision, c, y);
                        }
                    }
                }
            }
        }
        for y in 1..=3 {
            for x in 1..=6 {
                let p = &mut self.field.panels[y][x];
                p.display_alliance = p.alliance;
                if p.return_blink != 0 {
                    p.return_blink -= 1;
                    p.display_alliance ^= ((p.return_blink & 4) >> 2) as u8;
                }
            }
        }
    }

    /// `object_reservePanel`: reserve a panel for `obj` (marking it as a
    /// holder where the arena's rules say: `Reservations`).
    pub fn reserve_panel(&mut self, obj: ObjectRef, x: u8, y: u8) -> bool {
        let Some(p) = self.field.panel_mut(x, y) else { return false };
        if p.reserver.is_some() {
            return false;
        }
        p.reserver = Some(obj);
        p.flags |= pflags::RESERVED;
        if self.content.rules().panels.reservations == crate::content::Reservations::Marked {
            self.objects.get_mut(obj).flags |= crate::object::flags::HOLDS_RESERVATION;
        }
        true
    }

    /// `object_removePanelReserve`.
    pub fn unreserve_panel(&mut self, obj: ObjectRef, x: u8, y: u8) -> bool {
        let Some(p) = self.field.panel_mut(x, y) else { return false };
        if p.reserver != Some(obj) {
            return false;
        }
        p.reserver = None;
        p.flags &= !pflags::RESERVED;
        true
    }

    /// `sub_801BB78`: release every reservation `obj` holds (none where
    /// reservations don't mark their holder: EXE5's destroys release none).
    pub fn release_reservations(&mut self, obj: ObjectRef) {
        if self.objects.get(obj).flags & crate::object::flags::HOLDS_RESERVATION == 0 {
            return;
        }
        for y in (1..=4).rev() {
            for x in (1..=7).rev() {
                self.unreserve_panel(obj, x, y);
            }
        }
    }

    /// `_object_setPanelType`.
    pub fn set_panel_type(&mut self, x: u8, y: u8, t: PanelType) {
        let rules = &self.content.rules().panels;
        assert!((t.0 as usize) < rules.types.len(), "panel type {} is none of its game's ({})", t.0, rules.names.join(", "));
        let missing = rules.roles.missing;
        let Some(p) = self.field.panel_mut(x, y) else { return };
        if p.kind == missing {
            return;
        }
        p.kind = t;
        // (A type that expires starts its count: EXE6's roads,
        // `_object_setPanelType`; EXE5's lava and sea, 0x0800B2AE.)
        let rule = self.content.rules().panels.rule(t);
        if let Some(ticks) = rule.expires {
            p.expire_timer = ticks;
        }
        // (A type that crumbles counts at once: EXE4's 0x08009DC4.)
        if let Some(ticks) = rule.crumbles {
            p.crumble_timer = ticks;
        }
        self.field.refresh(&self.content.rules().panels, &self.collision, x, y);
    }

    /// `object_setPanelAlliance`.
    pub fn set_panel_alliance(&mut self, x: u8, y: u8, alliance: u8) {
        let missing = self.content.rules().panels.roles.missing;
        let Some(p) = self.field.panel_mut(x, y) else { return };
        if p.kind == missing {
            return;
        }
        p.alliance = alliance;
        p.return_blink = 0;
        self.field.refresh(&self.content.rules().panels, &self.collision, x, y);
    }

    /// Whether object `r` stands trapped: on a panel whose type traps (EXE4's
    /// pitfall, its `object_canMove`'s 0x0800AD42), not floating.
    pub fn trapped(&self, r: ObjectRef) -> bool {
        let o = self.objects.get(r);
        let Some(p) = self.field.panel(o.panel.x, o.panel.y) else { return false };
        let floats = o.collision.is_some_and(|c| self.collision.get(c).f1 & crate::collision::f1::FLOATSHOE != 0);
        self.content.rules().panels.rule(p.kind).traps && !floats
    }

    /// Whether nothing cracks or breaks panel (x, y): its type's rule
    /// (EXE4's metal, flag 0x20000, which its routines refuse: 0x08009AEC,
    /// 0x08009BAC, 0x08009BF0, 0x08009C4C, 0x08009D04).
    fn unbreakable(&self, x: u8, y: u8) -> bool {
        self.field.panel(x, y).is_some_and(|p| self.content.rules().panels.rule(p.kind).unbreakable)
    }

    /// `object_crackPanel`: crack a solid panel, or break an already
    /// cracked, unoccupied one.
    pub fn crack_panel(&mut self, x: u8, y: u8) -> bool {
        if self.unbreakable(x, y) {
            return false;
        }
        let (mask, roles) = (self.content.rules().panels.type_mask, self.content.rules().panels.roles);
        let Some(p) = self.field.panel_mut(x, y) else { return false };
        let f = p.flags;
        if f & pflags::SOLID == 0 {
            return false;
        }
        if f & pflags::CRACKED == 0 {
            p.flags = ((f | pflags::CRACKED) & !(mask & !(pflags::SOLID | pflags::CRACKED))) | roles.cracked.0 as u32;
            p.kind = roles.cracked;
            p.display_kind = roles.cracked;
            self.sound(SoundRole::PanelCrack);
            return true;
        }
        if f & pflags::OCCUPIED != 0 {
            return false;
        }
        p.flags = (f & !mask) | roles.broken.0 as u32;
        p.kind = roles.broken;
        p.display_kind = roles.broken;
        self.sound(SoundRole::PanelCrack);
        true
    }

    /// `object_breakPanel`: break a solid panel nothing stands on, cracked
    /// or not; nothing else. True when it broke.
    pub fn break_empty_panel(&mut self, x: u8, y: u8) -> bool {
        if self.unbreakable(x, y) {
            return false;
        }
        let (mask, roles) = (self.content.rules().panels.type_mask, self.content.rules().panels.roles);
        let Some(p) = self.field.panel_mut(x, y) else { return false };
        let f = p.flags;
        if f & pflags::SOLID == 0 || f & pflags::OCCUPIED != 0 {
            return false;
        }
        p.flags = (f & !mask) | roles.broken.0 as u32;
        p.kind = roles.broken;
        p.display_kind = roles.broken;
        self.sound(SoundRole::PanelCrack);
        true
    }

    /// `object_breakPanel_dup1`: break a solid panel, or crack it when
    /// something stands on it (bodies, blockers, reservations: not neutral
    /// objects). True only when it broke.
    pub fn shatter_panel(&mut self, x: u8, y: u8) -> bool {
        if self.unbreakable(x, y) {
            return false;
        }
        let (mask, roles) = (self.content.rules().panels.type_mask, self.content.rules().panels.roles);
        let Some(p) = self.field.panel_mut(x, y) else { return false };
        let f = p.flags;
        if f & pflags::SOLID == 0 {
            return false;
        }
        let broke = f & 0x0F08_0080 == 0;
        if broke {
            p.flags = (f & !mask) | roles.broken.0 as u32;
            p.kind = roles.broken;
            p.display_kind = roles.broken;
        } else {
            p.flags = ((f | pflags::CRACKED) & !(mask & !(pflags::SOLID | pflags::CRACKED))) | roles.cracked.0 as u32;
            p.kind = roles.cracked;
            p.display_kind = roles.cracked;
        }
        self.sound(SoundRole::PanelCrack);
        broke
    }

    /// `object_breakPanel_dup2` (and `object_breakPanel_dup3`, the same):
    /// break a solid panel, or crack it while something stands on it. (An
    /// unbreakable one stays: EXE4's dup2, 0x08009C4C; its dup3, 0x08009CAC,
    /// would crack it.)
    pub fn break_panel(&mut self, x: u8, y: u8) -> bool {
        self.break_panel_sounding(x, y, None)
    }

    /// The same with `sound` in place of the panel crack's (the role
    /// `panel_crack`): `object_breakPanelLoud` (EXE6's 0xDA; EXE5's crack's,
    /// 0x080C7768).
    pub fn break_panel_sounding(&mut self, x: u8, y: u8, sound: Option<SoundId>) -> bool {
        if self.unbreakable(x, y) {
            return false;
        }
        let (mask, roles) = (self.content.rules().panels.type_mask, self.content.rules().panels.roles);
        let Some(p) = self.field.panel_mut(x, y) else { return false };
        let f = p.flags;
        if f & pflags::SOLID == 0 {
            return false;
        }
        if f & pflags::OCCUPIED == 0 {
            p.flags = (f & !mask) | roles.broken.0 as u32;
            p.kind = roles.broken;
            p.display_kind = roles.broken;
        } else {
            p.flags = ((f | pflags::CRACKED) & !(mask & !(pflags::SOLID | pflags::CRACKED))) | roles.cracked.0 as u32;
            p.kind = roles.cracked;
            p.display_kind = roles.cracked;
        }
        match sound {
            Some(s) => self.play_sound(s),
            None => self.sound(SoundRole::PanelCrack),
        }
        true
    }

    /// `object_panel_setPoison`: a solid panel turns to poison.
    pub fn poison_panel(&mut self, x: u8, y: u8) -> bool {
        let rules = &self.content.rules().panels;
        let mask = rules.type_mask;
        let poison = rules.named("poison").expect("the game's poison panel");
        // (The type's own bits, those the mask owns: the original's 0x114.)
        let bits = rules.type_flags(poison) & mask;
        let Some(p) = self.field.panel_mut(x, y) else { return false };
        if p.flags & pflags::SOLID == 0 {
            return false;
        }
        p.flags = (p.flags & !mask) | bits;
        p.kind = poison;
        p.display_kind = poison;
        self.sound(SoundRole::PanelPoison);
        true
    }

    /// `object_setPanelTypeBlink`: panel (x, y) is drawn as `kind` of
    /// `side` this frame (drawn only).
    pub fn blink_panel(&mut self, x: u8, y: u8, kind: PanelType, side: u8) {
        if let Some(p) = self.field.panel_mut(x, y) {
            p.blink = Some((kind, side));
        }
    }

    /// `sub_800E618`: may `obj` step onto (x, y)?
    pub fn can_step(&self, obj: ObjectRef, x: u8, y: u8) -> bool {
        if !is_valid(x, y) {
            return false;
        }
        let o = self.objects.get(obj);
        let airshoes = o.collision.map(|c| self.collision.get(c).f1 & crate::collision::f1::AIRSHOE != 0).unwrap_or(false);
        let floor_free = airshoes || !self.field.is_solid(o.panel.x, o.panel.y);
        self.field.meets(x, y, self.content.rules().panels.step.get(floor_free, o.alliance))
    }
}

#[cfg(test)]
mod tests {
    use crate::battle::Battle;
    use crate::content::testing::panel;
    use crate::scenario;

    /// A panel's flags word holds its type in the low nibble by the game's
    /// number of it (its place in the rules' `numbers`: the test content's
    /// holy is its 5, its metal 13), with the type's own flags; the roles
    /// name the types the engine's code needs.
    #[test]
    fn a_panels_flags_hold_the_games_number_of_its_type() {
        let content = scenario::content();
        let rules = &content.rules().panels;
        let (holy, metal) = (panel("holy"), panel("metal"));
        assert_eq!((holy.0, metal.0), (5, 13));
        assert_eq!(rules.type_flags(holy), 5 | rules.rule(holy).flags);
        assert_eq!(rules.type_flags(metal), 13 | rules.rule(metal).flags);
        assert_eq!((rules.roles.missing, rules.roles.broken, rules.roles.cracked, rules.roles.normal), (panel("missing"), panel("broken"), panel("cracked"), panel("normal")));
        assert_eq!((rules.numbered(15), rules.numbered(16)), (Some(panel("sea")), None));
    }

    /// Stolen columns of one side that come due together return one a
    /// tick, the frontmost first (the original's list loop restarts at its
    /// first entry on every pass).
    #[test]
    fn columns_due_together_return_a_tick_apart() {
        let mut b = Battle::new(scenario::setup(), scenario::content());
        // Side 0 holds column 4 and one panel of column 5.
        for y in 1..=3 {
            b.set_panel_alliance(4, y, 0);
        }
        b.set_panel_alliance(5, 3, 0);
        b.field.columns[4].timer = 1;
        b.field.columns[5].timer = 1;
        let held = |b: &Battle| (b.field.panels[1][4].alliance, b.field.panels[3][4].alliance, b.field.panels[3][5].alliance);
        b.return_stolen_area();
        assert_eq!(held(&b), (1, 1, 0), "column 4 first");
        assert_eq!(b.field.columns[5].return_ready, 1);
        b.return_stolen_area();
        assert_eq!(held(&b), (1, 1, 1), "column 5 a tick later");
    }

    /// docs/design/exe5-map.md §15.2: a type that expires counts its own
    /// ticks from when the panel becomes it (the test content's lava and
    /// sea last 960, as EXE5's do; its roads 0x708, as EXE6's), blinking back
    /// to normal in its last second, then turns normal.
    #[test]
    fn expiring_panels_turn_normal_when_their_ticks_are_up() {
        let mut b = Battle::new(scenario::setup(), scenario::content());
        b.set_panel_type(2, 2, panel("lava"));
        b.set_panel_type(3, 2, panel("road_up"));
        assert_eq!((b.field.panels[2][2].expire_timer, b.field.panels[2][3].expire_timer), (960, 0x708));
        for _ in 0..959 {
            b.tick_panels();
        }
        let lava = b.field.panels[2][2];
        assert_eq!((lava.kind, lava.expire_timer), (panel("lava"), 1));
        b.tick_panels();
        assert_eq!(b.field.panels[2][2].kind, panel("normal"), "960 ticks of lava");
        assert_eq!(b.field.panels[2][3].kind, panel("road_up"), "the road counts on");
        // In its last second the panel shows normal every other pair of
        // ticks.
        b.set_panel_type(2, 2, panel("sea"));
        b.field.panels[2][2].expire_timer = 61;
        b.tick_panels();
        assert_eq!(b.field.panels[2][2].display_kind, panel("sea"));
        b.tick_panels();
        b.tick_panels();
        assert_eq!(b.field.panels[2][2].display_kind, panel("normal"));
    }

    /// A crack, a break and poison clear the type's bits the game's
    /// `type_mask` names: EXE6's 0x3F5F leaves a 0x20000 bit, EXE5's and
    /// EXE4's 0x23F5F (their sea's, their metal's) clears it; a crack keeps
    /// the solidity and sets the crack bit.
    #[test]
    fn a_crack_clears_the_games_type_bits() {
        use super::pflags;
        let run = |mask: u32| {
            let mut b = Battle::new(scenario::setup(), scenario::content());
            let mut c = (*b.content).clone();
            c.rules_mut().panels.type_mask = mask;
            b.content = std::sync::Arc::new(c);
            b.set_panel_type(2, 1, panel("normal"));
            b.set_panel_type(2, 2, panel("normal"));
            b.field.panels[1][2].flags |= 0x20000 | 0x0800_0000;
            b.field.panels[2][2].flags |= 0x20000;
            assert!(b.crack_panel(2, 1) && b.poison_panel(2, 2));
            (b.field.panels[1][2].flags, b.field.panels[2][2].flags)
        };
        let cracked = |extra: u32| extra | 0x0800_0000 | pflags::CRACKED | pflags::SOLID | 3;
        // (The first stood on, cracked; the second poisoned.)
        assert_eq!(run(0x3F5F), (cracked(0x20000 | 0x10000), 0x20000 | 0x10000 | 0x114));
        assert_eq!(run(0x23F5F), (cracked(0x10000), 0x10000 | 0x114));
    }

    /// The content API names a panel type as its game's rules do, both
    /// ways (Luau's `PanelType`).
    #[test]
    fn the_content_api_names_each_panel_type_as_its_game_does() {
        use nettai_content_api::CoreApi;
        let b = Battle::new(scenario::setup(), scenario::content());
        let names = &b.content.rules().panels.names;
        for (n, name) in names.iter().enumerate() {
            assert_eq!((b.panel_type_name(n as u8), b.panel_type_named(name)), (name.clone(), Some(n as u8)));
        }
        assert_eq!(b.panel_type_named("pitfall"), None, "the test content has none");
    }

    /// docs/design/exe4-map.md §18 item 12: EXE4's metal is unbreakable (its
    /// panel routines refuse flag 0x20000), and its pitfall turns normal a
    /// while after a type change makes it; a stage's waits, armed, for a
    /// grounded body. (The test content's metal and holy, made so.)
    #[test]
    fn exe4s_metal_holds_and_its_pitfall_crumbles() {
        use super::CRUMBLE_ARMED;
        let mut b = Battle::new(scenario::setup(), scenario::content());
        let mut c = (*b.content).clone();
        let (metal, pitfall) = (panel("metal"), panel("holy"));
        {
            let types = &mut c.rules_mut().panels.types;
            types[metal.0 as usize].unbreakable = true;
            types[pitfall.0 as usize].crumbles = Some(190);
            types[pitfall.0 as usize].stops_slides = true;
        }
        b.content = std::sync::Arc::new(c);
        b.set_panel_type(3, 1, metal);
        assert!(!b.crack_panel(3, 1) && !b.break_panel(3, 1) && !b.shatter_panel(3, 1) && !b.break_empty_panel(3, 1));
        assert_eq!(b.field.panels[1][3].kind, metal);
        b.set_panel_type(2, 1, pitfall);
        assert_eq!(b.field.panels[1][2].crumble_timer, 190);
        for _ in 0..189 {
            b.tick_panels();
        }
        assert_eq!(b.field.panels[1][2].kind, pitfall);
        b.tick_panels();
        assert_eq!(b.field.panels[1][2].kind, panel("normal"), "190 ticks after it was made");
        // Armed, as a stage's: nothing stands on it, so it waits.
        b.set_panel_type(2, 1, pitfall);
        b.field.panels[1][2].crumble_timer = CRUMBLE_ARMED | 190;
        for _ in 0..300 {
            b.tick_panels();
        }
        assert_eq!((b.field.panels[1][2].kind, b.field.panels[1][2].crumble_timer), (pitfall, CRUMBLE_ARMED | 190));
        // A grounded body stands on it: the count starts that tick.
        b.field.panels[1][2].flags |= super::pflags::BODY_SIDE0;
        b.tick_panels();
        assert_eq!(b.field.panels[1][2].crumble_timer, 189);
    }

}
