//! What a side sees, as the observation's arrays (docs/design/training-ai.md
//! §3): [`encode`] writes one viewer's view of a battle into an [`Obs`].
//!
//! Every read goes through what the viewer's console shows: the objects
//! [`Battle::visible_to`] it, the panels, its own HUD (`gauge_for`,
//! `telops_for`, `used_chip_for`, `chip_hud_for`), its own custom screen,
//! hand, navi stats and rules' state, and of the other side what shows on
//! the field (its navi's HP and form while it is seen, its B charge's glow).
//! Nothing reads the other side's hand, folder, custom screen, setup or
//! rules' state, the battle's or the consoles' RNG. The fairness test
//! (`tests`) scrambles all of those and an object hidden from the viewer,
//! and checks the view stays the same byte for byte.
//!
//! The field is laid out from the viewer's side: its own area on the left
//! (x mirrored for side 1, velocities, offsets, facing and regions with it),
//! so both sides of a round are one problem.

use crate::names;
use crate::spec::{self, chips, forms, global, objects, panels, rules, LOG_TICKS};
use nettai_battle::Battle;
use nettai_battle::battle::{fight, mode};
use nettai_battle::collision::{ATTACK_OF, f1};
use nettai_battle::content::{ChipClass, ChipModifier, ChipUsage, Content, DamageFormula, ExtraChipFlags, Trap};
use nettai_battle::custom::builder::{self, Pick};
use nettai_battle::custom::screen::{MAX_SELECTIONS, Phase, SLOTS, Screen, SlotKind, SlotState};
use nettai_battle::field::pflags;
use nettai_battle::hud::CustomGauge;
use nettai_battle::kinds::player::NaviAction;
use nettai_battle::object::ObjectRef;
use nettai_battle::perspective::TelopName;
use nettai_content_api::{ChipHandle, FieldType, FieldValue, Pool, Registry, Schema};

/// One token set's arrays: `count` tokens of `width` features and `ids`
/// name ids, and which tokens are there.
#[derive(Clone, Debug, PartialEq)]
pub struct SetBuf {
    pub x: Vec<f32>,
    pub ids: Vec<i32>,
    pub mask: Vec<bool>,
}

/// A side's view of a battle at a tick: each set's arrays, in
/// [`spec::SETS`]' order.
#[derive(Clone, Debug, PartialEq)]
pub struct Obs {
    pub sets: Vec<SetBuf>,
}

impl Default for Obs {
    fn default() -> Obs {
        Obs::new()
    }
}

impl Obs {
    pub fn new() -> Obs {
        Obs {
            sets: spec::SETS
                .iter()
                .map(|s| SetBuf { x: vec![0.0; s.count * s.width], ids: vec![0; s.count * s.ids], mask: vec![false; s.count] })
                .collect(),
        }
    }

    /// No token there: the tokens that were are zeroed (a token not there
    /// is all zeros already).
    fn clear(&mut self) {
        for (set, buf) in spec::SETS.iter().zip(&mut self.sets) {
            for i in 0..set.count {
                if buf.mask[i] {
                    buf.x[i * set.width..(i + 1) * set.width].fill(0.0);
                    buf.ids[i * set.ids..(i + 1) * set.ids].fill(0);
                    buf.mask[i] = false;
                }
            }
        }
    }

    /// Token `i` of set `set`: its features, its ids, and it marked there.
    fn token(&mut self, set: usize, i: usize) -> (&mut [f32], &mut [i32]) {
        let s = spec::SETS[set];
        let buf = &mut self.sets[set];
        buf.mask[i] = true;
        (&mut buf.x[i * s.width..(i + 1) * s.width], &mut buf.ids[i * s.ids..(i + 1) * s.ids])
    }
}

/// The ids of a content's names, made once for it ([`Names::of`]): a
/// chip's, a kind's, a form's... by handle, and the scalars of its rules'
/// state and its navis' stats of the game's own that a rules token holds.
#[derive(Clone, Debug)]
pub struct Names {
    chips: Vec<[i32; 2]>,
    kinds: Vec<[i32; 2]>,
    forms: Vec<[i32; 2]>,
    navis: Vec<[i32; 2]>,
    identities: Vec<[i32; 2]>,
    actions: Vec<[i32; 2]>,
    buttons: Vec<[i32; 2]>,
    panel_types: Vec<[i32; 2]>,
    /// The registries' keys' ids, by registry, for a rules value that names
    /// a definition.
    defs: Vec<(Registry, Vec<[i32; 2]>)>,
    /// The scalars a rules token holds: the navi stats' first, then the
    /// rules' state's.
    scalars: Vec<Scalar>,
}

/// Where a rules token's scalar is: in the navi's stats of its game's own
/// or in the rules' state, a top field, its record's field, its array's
/// element.
#[derive(Clone, Copy, Debug)]
struct Scalar {
    stats: bool,
    top: usize,
    field: Option<usize>,
    elem: Option<(usize, usize)>,
    name: [i32; 2],
}

impl Names {
    pub fn of(content: &Content) -> Names {
        let d = &content.defs;
        let keys = |it: &mut dyn Iterator<Item = &str>| it.map(names::ids).collect::<Vec<_>>();
        let mut scalars = Vec::new();
        if let Some(r) = d.rules() {
            scalars_of(d.schema(r.stats), true, &mut scalars);
            scalars_of(d.schema(r.state), false, &mut scalars);
        }
        let defs = [Registry::Chip, Registry::Form, Registry::Navi, Registry::Kind, Registry::Weapon, Registry::Action]
            .into_iter()
            .map(|r| (r, (0..u16::MAX).map_while(|h| nettai_match::ids::key_of(content, r, h)).map(names::ids).collect()))
            .collect();
        Names {
            chips: keys(&mut d.chips.iter().map(|c| c.key.as_str())),
            kinds: keys(&mut d.kinds.iter().map(|c| c.key.as_str())),
            forms: keys(&mut d.forms.iter().map(|c| c.key.as_str())),
            navis: keys(&mut d.navis.iter().map(|c| c.key.as_str())),
            identities: keys(&mut d.identities.iter().map(|c| c.key.as_str())),
            actions: keys(&mut d.actions.iter().map(|c| c.key.as_str())),
            buttons: keys(&mut d.buttons.iter().map(|c| c.name.as_str())),
            panel_types: content.rules.as_ref().map_or_else(Vec::new, |r| keys(&mut r.panels.names.iter().map(String::as_str))),
            defs,
            scalars,
        }
    }

    /// How many scalars a rules token could hold (more than the set's
    /// count are left out, the last first).
    pub fn rule_scalars(&self) -> usize {
        self.scalars.len()
    }

    fn chip(&self, h: ChipHandle) -> [i32; 2] {
        self.chips.get(h.0 as usize).copied().unwrap_or_default()
    }

    fn def(&self, r: Registry, h: u16) -> [i32; 2] {
        self.defs.iter().find(|(x, _)| *x == r).and_then(|(_, v)| v.get(h as usize).copied()).unwrap_or_default()
    }
}

/// A schema's scalars, one level of records deep, an array's elements each.
fn scalars_of(schema: &Schema, stats: bool, out: &mut Vec<Scalar>) {
    for (top, f) in schema.fields().iter().enumerate() {
        match &f.ty {
            FieldType::Record(inner) => {
                for (j, g) in inner.fields().iter().enumerate() {
                    if is_scalar(&g.ty) {
                        out.push(Scalar { stats, top, field: Some(j), elem: None, name: names::ids(&format!("{}.{}", f.name, g.name)) });
                    }
                }
            }
            FieldType::Array(elem, n) if is_scalar(elem) => {
                for k in 0..*n as usize {
                    out.push(Scalar { stats, top, field: None, elem: Some((k, *n as usize)), name: names::ids(&f.name) });
                }
            }
            ty if is_scalar(ty) => out.push(Scalar { stats, top, field: None, elem: None, name: names::ids(&f.name) }),
            _ => {}
        }
    }
}

fn is_scalar(ty: &FieldType) -> bool {
    matches!(
        ty,
        FieldType::Bool
            | FieldType::U8
            | FieldType::U16
            | FieldType::U32
            | FieldType::I8
            | FieldType::I16
            | FieldType::I32
            | FieldType::Enum(_)
            | FieldType::OptionalU8
            | FieldType::Ref(..)
            | FieldType::Code
            | FieldType::Object
    )
}

/// The last ticks' panels attacked, by the battle's sides (§5): what an
/// env keeps of `Battle::attacked_panels` each tick, the newest first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AttackHistory {
    pub ticks: [[u64; 2]; LOG_TICKS],
}

impl AttackHistory {
    /// The tick just run's.
    pub fn push(&mut self, b: &Battle) {
        self.ticks.copy_within(0..LOG_TICKS - 1, 1);
        self.ticks[0] = [b.attacked_panels(0), b.attacked_panels(1)];
    }
}

/// What a viewer's player knows that the battle doesn't hold for it: its
/// reaction delay, the attacks of the last ticks, which keys open a window
/// on its custom screen (the key probe, §4.2), the slots its screen
/// refused, and how many decisions it has made on the screen.
#[derive(Clone, Copy, Debug, Default)]
pub struct Extra {
    pub delay: u16,
    pub log: AttackHistory,
    pub probe: Option<[bool; spec::SCREEN_KEYS.len()]>,
    pub refused: u16,
    pub screen_decisions: u16,
    /// Only OK may be pointed at (the screen's decisions are spent).
    pub only_ok: bool,
}

/// Viewer `me`'s side: its own, the other's, or neither.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Rel {
    Own,
    Other,
    Neutral,
}

/// One viewer's reading of a battle.
struct View<'a> {
    b: &'a Battle,
    names: &'a Names,
    me: u8,
    them: u8,
    /// The viewer sees the other side's objects (it isn't blind).
    sees: bool,
}

impl View<'_> {
    /// A field column as the viewer lays it out (its own area left).
    fn x(&self, x: u8) -> u8 {
        if self.me == 1 { 7u8.wrapping_sub(x) } else { x }
    }

    /// A horizontal amount as the viewer lays it out.
    fn dx(&self, v: f32) -> f32 {
        if self.me == 1 { -v } else { v }
    }

    fn rel(&self, alliance: u8) -> Rel {
        match alliance {
            a if a == self.me => Rel::Own,
            a if a == self.them => Rel::Other,
            _ => Rel::Neutral,
        }
    }

    /// The viewer's side's and the other side's bit of a pair of side bits
    /// (side 0's, side 1's).
    fn sides(&self, pair: [u32; 2], word: u32) -> [bool; 2] {
        [word & pair[self.me as usize] != 0, word & pair[self.them as usize] != 0]
    }

    /// A panel bit (`y * 8 + x`) of the viewer's layout's token: its
    /// index among the 18.
    fn panel_token(&self, x: u8, y: u8) -> Option<usize> {
        let mx = self.x(x);
        ((1..=6).contains(&mx) && (1..=3).contains(&y)).then(|| (y as usize - 1) * 6 + (mx as usize - 1))
    }
}

fn clip(v: f32, lo: f32, hi: f32) -> f32 {
    v.clamp(lo, hi)
}

fn one_hot(row: &mut [f32], at: usize, n: usize, i: usize) {
    if i < n {
        row[at + i] = 1.0;
    }
}

fn flag(b: bool) -> f32 {
    b as u8 as f32
}

fn put_ids(ids: &mut [i32], at: usize, pair: [i32; 2]) {
    ids[at..at + 2].copy_from_slice(&pair);
}

/// Write viewer `viewer`'s view of `b` into `out` (§3).
pub fn encode(b: &Battle, viewer: u8, names: &Names, extra: &Extra, out: &mut Obs) {
    out.clear();
    let me = viewer & 1;
    let v = View { b, names, me, them: me ^ 1, sees: b.sees(me, me ^ 1) };
    encode_global(&v, extra, out);
    encode_panels(&v, extra, out);
    encode_objects(&v, out);
    encode_chips(&v, extra, out);
    encode_rules(&v, out);
    encode_forms(&v, out);
}

/// Whether the viewer sees object `r`: it is shown to it this tick
/// (`Battle::visible_to`), or it blinks (flashing, or invisible: shown two
/// ticks of four, which a person sees as there) on a side the viewer isn't
/// blind to.
fn seen(v: &View, r: ObjectRef) -> bool {
    sees_object(v.b, v.me, r)
}

/// Whether `viewer` sees object `r` in `b` ([`seen`]).
pub(crate) fn sees_object(b: &Battle, viewer: u8, r: ObjectRef) -> bool {
    if b.visible_to(r, viewer) {
        return true;
    }
    let o = b.objects.get(r);
    let blinks = o.collision.is_some_and(|c| {
        let f = b.collision.get(c).f1;
        f & (f1::FLASHING | f1::INVISIBLE) != 0 && f & f1::DEAD == 0
    });
    blinks && b.sees(viewer, o.alliance)
}

/// The viewer's navi, and the other's while the viewer sees it.
fn navis(v: &View) -> (Option<ObjectRef>, Option<ObjectRef>) {
    let other = v.b.player(v.them).filter(|&r| v.b.objects.is_allocated(r) && seen(v, r));
    (v.b.player(v.me).filter(|&r| v.b.objects.is_allocated(r)), other)
}

/// The round's phase as a player tells it: the custom screen, the fight
/// running, dimmed, or between (the intro, banners, fades, the judge).
fn phase(b: &Battle) -> usize {
    if b.round.mode == mode::CUSTOM {
        0
    } else if b.is_dimmed() {
        2
    } else if b.round.mode == mode::FIGHTING && b.fight.state == fight::FIGHTING {
        1
    } else {
        3
    }
}

fn encode_global(v: &View, extra: &Extra, out: &mut Obs) {
    let b = v.b;
    let (own, other) = navis(v);
    let (row, ids) = out.token(spec::GLOBAL, 0);
    one_hot(row, global::phase, 4, phase(b));
    let side = &b.custom.sides[v.me as usize];
    let screen = side.screen.filter(|_| b.round.mode == mode::CUSTOM);
    if let Some(s) = &screen {
        let i = match s.phase {
            Phase::Choosing => 0,
            Phase::Window { .. } => 1,
            _ if side.sent.is_some() => 3,
            _ => 2,
        };
        one_hot(row, global::screen_phase, 4, i);
    }
    let gauge = b.gauge_for(v.me);
    row[global::gauge] = gauge as f32 / CustomGauge::FULL as f32;
    row[global::gauge + 1] = flag(gauge >= CustomGauge::FULL);
    let late = b.late_turns();
    row[global::turn] = clip(b.round.turn as f32 / 16.0, 0.0, 2.0);
    row[global::turn + 1] = flag(late);
    row[global::turn + 2] = if late { clip(b.fight.turn_timer as f32 / 3600.0, 0.0, 2.0) } else { 0.0 };
    row[global::battle_time] = clip(b.round.battle_time as f32 / 3600.0, 0.0, 5.0);
    let hp = |r: ObjectRef| {
        let o = b.objects.get(r);
        [o.hp as f32 / 1000.0, o.max_hp as f32 / 1000.0, if o.max_hp > 0 { o.hp as f32 / o.max_hp as f32 } else { 0.0 }]
    };
    if let Some(r) = own {
        row[global::own_hp..global::own_hp + 3].copy_from_slice(&hp(r));
        if let Some(a) = b.objects.get(r).actor.map(|a| b.actors.get(a)) {
            one_hot(row, global::own_charge, 3, a.charge_level as usize);
            one_hot(row, global::own_charge + 3, 3, a.charge_source as usize);
            row[global::own_charge + 6] = clip(a.charge_counter as f32 / 200.0, 0.0, 2.0);
        }
    }
    if let Some(r) = other {
        row[global::other_hp] = 1.0;
        row[global::other_hp + 1..global::other_hp + 4].copy_from_slice(&hp(r));
        // (The other's charge shows by its glow, where the viewer sees it.)
        if let Some(a) = b.objects.get(r).actor.map(|a| b.actors.get(a))
            && a.charge_glow.is_some_and(|g| b.objects.is_allocated(g) && b.visible_to(g, v.me))
        {
            row[global::other_charge] = 1.0;
            one_hot(row, global::other_charge + 1, 3, a.charge_level as usize);
        }
    }
    let s = &b.stats[v.me as usize];
    let stats = [
        s.attack as f32 / 5.0,
        s.rapid as f32 / 5.0,
        s.charge as f32 / 5.0,
        s.custom_level as f32 / 10.0,
        s.mega_level as f32 / 5.0,
        s.giga_level as f32 / 5.0,
        s.reg_up as f32 / 50.0,
        s.max_base_hp as f32 / 1000.0,
    ];
    row[global::own_stats..global::own_stats + 8].copy_from_slice(&stats);
    row[global::chips_left] = b.hands[v.me as usize].remaining() as f32 / 6.0;
    row[global::chips_left + 1] = side.folder.chips.iter().flatten().count() as f32 / 30.0;
    row[global::chips_left + 2] = flag(side.folder.regular_pending);
    for (k, t) in b.telops_for(v.me).iter().enumerate() {
        if let Some((t, _)) = t {
            let at = global::telops + 4 * k;
            row[at] = 1.0;
            row[at + 1] = flag(t.remote);
            row[at + 2] = flag(t.name == TelopName::Hidden);
            row[at + 3] = t.damage as f32 / 400.0;
        }
    }
    if let Some(u) = b.used_chip_for(v.me) {
        row[global::used_chip] = 1.0;
        row[global::used_chip + 1] = u.ticks as f32 / 60.0;
    }
    row[global::delay] = extra.delay as f32 / 30.0;
    if let Some(p) = extra.probe {
        row[global::probe] = 1.0;
        for (k, opens) in p.iter().enumerate() {
            row[global::probe + 1 + k] = flag(*opens);
        }
    }
    if let Some(s) = &screen {
        row[global::screen] = s.selected as f32 / MAX_SELECTIONS as f32;
        row[global::screen + 1] = flag(s.form.is_some());
        row[global::screen + 2] = flag(side.in_custom);
        row[global::screen + 3] = extra.screen_decisions as f32 / 24.0;
        row[global::screen + 4] = flag(s.trade.is_some() || s.hold.is_some());
        row[global::screen + 5] = flag(extra.only_ok);
    }
    row[global::navis] = flag(own.is_some());
    row[global::navis + 1] = flag(other.is_some());
    row[global::navis + 2] = flag(!v.sees);
    put_ids(ids, 0, v.names.navis.get(b.stats[v.me as usize].navi.0 as usize).copied().unwrap_or_default());
    if other.is_some() {
        put_ids(ids, 2, v.names.navis.get(b.stats[v.them as usize].navi.0 as usize).copied().unwrap_or_default());
    }
}

/// The type bits of a panel that are its bodies, by side.
const BODY_OF: [u32; 2] = [pflags::BODY_SIDE0, pflags::BODY_SIDE1];
/// A panel's blocker and neutral object bits (field-collision-damage.md §3.3).
const BLOCKER: u32 = 0x0008_0000;
const NEUTRAL: u32 = 0x0080_0000;

fn encode_panels(v: &View, extra: &Extra, out: &mut Obs) {
    let b = v.b;
    let rules = &b.game_rules().panels;
    for y in 1..=3u8 {
        for x in 1..=6u8 {
            let Some(t) = v.panel_token(x, y) else { continue };
            let p = &b.field.panels[y as usize][x as usize];
            let (row, ids) = out.token(spec::PANELS, t);
            row[panels::place] = (v.x(x) - 1) as f32 / 5.0;
            row[panels::place + 1] = (y - 1) as f32 / 2.0;
            if let Some(r) = rules.types.get(p.kind.0 as usize) {
                let rule = [
                    flag(r.doubles.is_some()),
                    r.doubles.map_or(0.0, |e| e as f32 / 5.0),
                    r.damage_shift as f32 / 2.0,
                    flag(r.expires.is_some()),
                    flag(r.submerges),
                    flag(r.unbreakable),
                    flag(r.traps),
                    flag(r.cleared_by.is_some()),
                    flag(r.carries.is_some()),
                    flag(r.carries_by_move.is_some()),
                    flag(p.flags & pflags::SOLID != 0),
                    flag(r.flags != 0),
                ];
                row[panels::rule..panels::rule + 12].copy_from_slice(&rule);
            }
            put_ids(ids, 0, v.names.panel_types.get(p.kind.0 as usize).copied().unwrap_or_default());
            row[panels::owner] = flag(p.alliance == v.me);
            row[panels::owner + 1] = flag(p.home == v.me);
            row[panels::owner + 2] = flag(p.alliance != p.home);
            row[panels::state] = flag(p.flags & pflags::CRACKED != 0);
            row[panels::state + 1] = flag(p.visible);
            row[panels::state + 2] = clip(p.hole_timer as f32 / 600.0, 0.0, 2.0);
            row[panels::state + 3] = clip(p.timer as f32 / 1800.0, 0.0, 2.0);
            row[panels::state + 4] = flag(p.blink.is_some());
            row[panels::highlight] = flag(p.highlight != 0);
            let types = b.collision.panel_self_flags(x, y);
            let bodies = v.sides(BODY_OF, p.flags);
            let attacks = v.sides(ATTACK_OF, types);
            let on = [
                bodies[0],
                bodies[1] && v.sees,
                p.flags & pflags::FLOATING != 0,
                types & BLOCKER != 0,
                types & NEUTRAL != 0,
                attacks[0],
                attacks[1] && v.sees,
            ];
            for (k, on) in on.into_iter().enumerate() {
                row[panels::on_it + k] = flag(on);
            }
            let bit = 1u64 << (y * 8 + x);
            for (k, tick) in extra.log.ticks.iter().enumerate() {
                row[panels::attacked + 2 * k] = flag(tick[v.me as usize] & bit != 0);
                row[panels::attacked + 2 * k + 1] = flag(tick[v.them as usize] & bit != 0 && v.sees);
            }
        }
    }
}

/// The status bits an object's token holds, in order.
const STATUS_BITS: [u32; 21] = [
    f1::GUARD,
    f1::INVISIBLE,
    f1::SUBMERGED,
    f1::INVULNERABLE,
    f1::AIRSHOE,
    f1::FLOATSHOE,
    f1::MOVING,
    f1::DEAD,
    f1::FLASHING,
    f1::FLINCHING,
    f1::PARALYZED,
    f1::SLIDING,
    f1::BLIND,
    f1::IMMOBILIZED,
    f1::CONFUSED,
    f1::FROZEN,
    f1::SUPERARMOR,
    f1::UNDERSHIRT,
    f1::USING_ACTION,
    f1::UNTOUCHABLE,
    f1::BUBBLED,
];

fn encode_objects(v: &View, out: &mut Obs) {
    let b = v.b;
    let mut n = 0;
    // (The actors first, then the attacks, then the effects, each in the
    // update order, as many as the set holds.)
    for pool in [Pool::Actor, Pool::Attack, Pool::Effect] {
        for r in b.objects.in_order().filter(|r| r.pool == pool) {
            if n == spec::SETS[spec::OBJECTS].count {
                return;
            }
            if !seen(v, r) {
                continue;
            }
            encode_object(v, r, n, out);
            n += 1;
        }
    }
}

fn encode_object(v: &View, r: ObjectRef, i: usize, out: &mut Obs) {
    let b = v.b;
    let o = b.objects.get(r);
    let (row, ids) = out.token(spec::OBJECTS, i);
    one_hot(row, objects::pool, 3, r.pool as usize);
    let class = if o.actor.is_some() || o.identity.is_some() { b.content.identity(o.identity).class as usize } else { 4 };
    one_hot(row, objects::class, 5, class);
    let rel = v.rel(o.alliance);
    one_hot(row, objects::side, 3, rel as usize);
    let (own_player, other_player) = (b.player(v.me) == Some(r), b.player(v.them) == Some(r));
    row[objects::player] = flag(own_player);
    row[objects::player + 1] = flag(other_player);
    row[objects::panel] = v.x(o.panel.x) as f32 / 7.0;
    row[objects::panel + 1] = o.panel.y as f32 / 4.0;
    row[objects::panel + 2] = v.x(o.future_panel.x) as f32 / 7.0;
    row[objects::panel + 3] = o.future_panel.y as f32 / 4.0;
    let (cx, cy) = nettai_battle::kinds::player::panel_coordinates(o.panel.x, o.panel.y);
    let px = |w: i32| w as f32 / 65536.0;
    row[objects::offset] = clip(v.dx(px(o.pos.x - cx)) / 40.0, -4.0, 4.0);
    row[objects::offset + 1] = clip(px(o.pos.y - cy) / 24.0, -4.0, 4.0);
    row[objects::offset + 2] = clip(px(o.pos.z) / 32.0, -4.0, 4.0);
    row[objects::velocity] = clip(v.dx(px(o.vel.x)) / 4.0, -4.0, 4.0);
    row[objects::velocity + 1] = clip(px(o.vel.y) / 4.0, -4.0, 4.0);
    row[objects::velocity + 2] = clip(px(o.vel.z) / 4.0, -4.0, 4.0);
    // (Facing is alliance ^ flip, 0 to the right: toward the other side's
    // area as the viewer lays the field out.)
    row[objects::facing] = flag((o.alliance ^ o.flip) & 1 == v.me);
    if o.max_hp > 0 {
        row[objects::hp] = 1.0;
        row[objects::hp + 1] = o.hp as f32 / 1000.0;
        row[objects::hp + 2] = o.max_hp as f32 / 1000.0;
        row[objects::hp + 3] = o.hp as f32 / o.max_hp as f32;
    }
    let collision = o.collision.map(|c| b.collision.get(c));
    let damage = collision.map_or(o.damage & b.game_rules().effects.damage_word.damage, |c| c.self_damage);
    row[objects::damage] = clip(damage as f32 / 400.0, 0.0, 5.0);
    one_hot(row, objects::element, 6, (o.element & 0xF).min(5) as usize);
    for k in 0..4 {
        row[objects::element + 6 + k] = flag(o.element & (0x10 << k) != 0);
    }
    if let Some(c) = collision {
        let t = c.self_flags;
        let attacks = v.sides(ATTACK_OF, t);
        let bodies = v.sides([0x0800_0000, 0x0400_0000], t);
        let others = v.sides([0x0200_0000, 0x0100_0000], t);
        let types = [
            true,
            attacks[0],
            attacks[1],
            bodies[0],
            bodies[1],
            others[0] || others[1],
            t & NEUTRAL != 0,
            t & 0x0010_0000 != 0,
            t & BLOCKER != 0,
            t & 0x5000 == 0x5000 || t & 0x2 != 0,
            t & 0x4 != 0,
            t & 0x8000 != 0,
        ];
        for (k, on) in types.into_iter().enumerate() {
            row[objects::collision + k] = flag(on);
        }
        // (The panels it is registered on now.)
        for y in 1..=3u8 {
            for x in 1..=6u8 {
                if b.collision.masks[(y * 8 + x) as usize] & c.bit != 0
                    && let Some(t) = v.panel_token(x, y)
                {
                    row[objects::region + t] = 1.0;
                }
            }
        }
        for (k, bit) in STATUS_BITS.iter().enumerate() {
            row[objects::status + k] = flag(c.f1 & bit != 0);
        }
        // (A status's time left and a barrier's HP show to no one but its
        // own side's player.)
        if rel == Rel::Own {
            for (k, t) in c.status_timers.iter().enumerate() {
                row[objects::timers + k] = clip(*t as f32 / 120.0, 0.0, 4.0);
            }
            row[objects::barrier + 1] = clip(c.barrier_hp as f32 / 200.0, 0.0, 2.0);
        }
        row[objects::barrier] = flag(c.barrier_hp > 0);
    }
    if let Some(a) = o.actor.map(|a| b.actors.get(a)) {
        row[objects::actor] = 1.0;
        let glow = a.charge_glow.is_some_and(|g| b.objects.is_allocated(g) && b.visible_to(g, v.me));
        if rel == Rel::Own || glow {
            row[objects::actor + 1] = a.charge_level as f32 / 2.0;
            row[objects::actor + 2] = flag(a.charge_source == 2);
        }
        if rel == Rel::Own {
            row[objects::actor + 3] = a.lockout as f32 / 60.0;
        }
        // (Its chips show over its head to its own side, and to the other's
        // whose console's icons have them: `ChipHud::opponent`.)
        if rel == Rel::Own || b.chip_hud_for(v.me).opponent {
            row[objects::actor + 4] = o.chips_held as f32 / 6.0;
            row[objects::actor + 5] = 1.0;
        }
        let action = match a.navi_action {
            NaviAction::Entry => 0,
            NaviAction::TakeControl => 1,
            NaviAction::Deletion => 2,
            NaviAction::Flinch => 3,
            NaviAction::Paralysis => 4,
            NaviAction::Drag => 5,
            NaviAction::Freeze => 6,
            NaviAction::Bubble => 7,
            NaviAction::Idle => 8,
            NaviAction::Engine(e) => {
                put_ids(ids, 4, names::debug_ids(&e));
                9
            }
            NaviAction::Content(h) => {
                put_ids(ids, 4, v.names.actions.get(h.0 as usize).copied().unwrap_or_default());
                10
            }
        };
        one_hot(row, objects::action, 11, action);
    }
    row[objects::anim] = o.anim as f32 / 64.0;
    put_ids(ids, 0, v.names.kinds.get(o.kind.0 as usize).copied().unwrap_or_default());
    if let Some(h) = o.identity {
        put_ids(ids, 2, v.names.identities.get(h.0 as usize).copied().unwrap_or_default());
    }
}

/// What a chip's record says of it (§3: damage, element, class, codes, MB,
/// the flag and trait bits, trap kind, modifier kind, damage formula kind,
/// hit strength, lockout, how it's used), with this copy's code.
fn chip_facts(content: &Content, h: ChipHandle, code: Option<u8>, row: &mut [f32]) {
    let def = &content.defs.chips[h.0 as usize];
    let c = &def.record;
    let mut at = chips::facts;
    one_hot(row, at, 6, (c.element as usize).min(5));
    at += 6;
    let class = match c.class {
        ChipClass::Standard => 0,
        ChipClass::Mega => 1,
        ChipClass::Giga => 2,
        ChipClass::Special => 3,
        ChipClass::ProgramAdvance => 4,
    };
    one_hot(row, at, 5, class);
    at += 5;
    if let Some(code) = code {
        one_hot(row, at, 27, code as usize);
    }
    at += 27;
    let usage = match def.usage {
        ChipUsage::Action(_) => 0,
        ChipUsage::Dimming(_) => 1,
        ChipUsage::HandOff(_) => 2,
        ChipUsage::Instant(_) => 3,
    };
    one_hot(row, at, 4, usage);
    at += 4;
    for k in 0..8 {
        row[at + k] = flag(c.flags.0 & (1 << k) != 0);
    }
    at += 8;
    for (k, bit) in [ExtraChipFlags::RUSH_CANCELS, ExtraChipFlags::MODIFIER, ExtraChipFlags::FREE_SLOT_IN].iter().enumerate() {
        row[at + k] = flag(c.extra_flags.has(*bit));
    }
    at += 3;
    for k in 0..8 {
        // (The named traits, in their order.)
        let bit = [0x01, 0x02, 0x04, 0x08, 0x100, 0x400, 0x800, 0x1000][k];
        row[at + k] = flag(c.traits.has(bit));
    }
    at += 8;
    let trap = match c.trap {
        None => 0,
        Some(Trap::AntiDamage) => 1,
        Some(Trap::AntiSword) => 2,
        Some(Trap::BodyGuard) => 3,
        Some(Trap::AntiNavi) => 4,
        Some(Trap::AntiRecovery) => 5,
    };
    one_hot(row, at, 6, trap);
    at += 6;
    let modifier = match c.modifier {
        None => 0,
        Some(ChipModifier::AttackPlus) => 1,
        Some(ChipModifier::NaviPlus) => 2,
        Some(ChipModifier::Paralyze) => 3,
        Some(ChipModifier::Uninstall) => 4,
    };
    one_hot(row, at, 5, modifier);
    at += 5;
    let formula = match &c.formula {
        None => 0,
        Some(DamageFormula::OpponentHp) => 1,
        Some(DamageFormula::Gauge { .. }) => 2,
        Some(DamageFormula::HpLost { .. }) => 3,
        Some(DamageFormula::HpLastDigits) => 4,
        Some(DamageFormula::HalfOpponentMaxHp) => 5,
        Some(DamageFormula::NaviLevel { .. }) => 6,
        Some(DamageFormula::Given(_)) => 7,
        Some(DamageFormula::GaugeLevel { .. } | DamageFormula::Count { .. }) => 8,
    };
    one_hot(row, at, 9, formula);
    at += 9;
    row[at] = c.damage as f32 / 400.0;
    row[at + 1] = c.mb as f32 / 100.0;
    row[at + 2] = c.hit_param as f32 / 255.0;
    row[at + 3] = c.lockout as f32 / 60.0;
    at += 4;
    row[at] = flag(!c.program_advances.is_empty());
    row[at + 1] = flag(def.links.own_chip_of.is_some());
    row[at + 2] = c.codes.len() as f32 / 4.0;
    debug_assert_eq!(at + 3, chips::in_hand);
}

/// The picks a screen's selection holds, with `extra` picked after them:
/// what OK would build the hand from (the screen's own `confirm`, but for
/// the invalid-chip rule and a button's chip).
fn picks(b: &Battle, side: u8, screen: &Screen, extra: Option<u8>) -> Vec<Pick> {
    let folder = &b.custom.sides[side as usize].folder;
    screen
        .selection()
        .iter()
        .copied()
        .chain(extra)
        .filter_map(|slot| {
            let kind = screen.slots[slot as usize].kind;
            let chip = screen.chip_in(slot, folder)?;
            let offer = screen.offers[slot as usize].filter(|_| matches!(kind, SlotKind::Offered(_)));
            let marks = screen.slots[slot as usize].marks | offer.map_or(0, |o| o.marks);
            Some(Pick { chip, regular: matches!(kind, SlotKind::Chip { regular: true, .. }), marks, listed: offer.is_none_or(|o| o.listed) })
        })
        .collect()
}

/// The hand OK would build from `picks` (its chips' damage their records',
/// a formula's left out: it can read what the viewer doesn't see).
fn would_build(b: &Battle, side: u8, picks: &[Pick]) -> builder::Built {
    let mut used = b.custom.sides[side as usize].program_advances;
    let own_gauges = b.round.flags & nettai_battle::battle::battle_flags::OWN_GAUGES != 0;
    builder::build(picks, b.round.turn, &mut used, &*b.content, own_gauges, |id| b.content.chip(id).damage)
}

fn hand_damage(built: &builder::Built) -> f32 {
    (0..6).map_while(|i| built.hand.ids[i].map(|_| (built.hand.damage[i] + built.hand.attack_bonus[i]) as f32)).sum()
}

fn encode_chips(v: &View, extra: &Extra, out: &mut Obs) {
    let b = v.b;
    let content = &*b.content;
    // The hand in the fight, the next chip first.
    let hand = &b.hands[v.me as usize];
    for (k, i) in (hand.cursor as usize..6).enumerate() {
        let Some(id) = hand.ids[i] else { break };
        let (row, ids) = out.token(spec::CHIPS, k);
        row[chips::role] = 1.0;
        row[chips::hand_place] = k as f32 / 6.0;
        let code = hand.selection[i].filter(|s| s.id == id).map(|s| s.code.0);
        chip_facts(content, id, code, row);
        row[chips::in_hand] = (hand.damage[i] + hand.attack_bonus[i]) as f32 / 400.0;
        row[chips::in_hand + 1] = hand.attack_bonus[i] as f32 / 100.0;
        for bit in 0..3 {
            row[chips::in_hand + 2 + bit] = flag(hand.modifiers[i] & (1 << bit) != 0);
        }
        put_ids(ids, 0, v.names.chip(id));
    }
    // The custom screen's slots.
    let side = &b.custom.sides[v.me as usize];
    if let Some(screen) = side.screen.filter(|_| b.round.mode == mode::CUSTOM) {
        let layout = &b.game_rules().custom_screen;
        let steps = crate::act::steps(&screen, layout);
        let legal = crate::act::legal_slots(&screen, &steps, extra.refused, extra.only_ok);
        let base = picks(b, v.me, &screen, None);
        let base_built = would_build(b, v.me, &base);
        let base_damage = hand_damage(&base_built);
        for slot in 0..SLOTS {
            let s = &screen.slots[slot];
            let (row, ids) = out.token(spec::CHIPS, spec::FIRST_SLOT_TOKEN + slot);
            row[chips::role + 1] = 1.0;
            one_hot(row, chips::slot, 12, slot);
            let kind = match s.kind {
                SlotKind::Chip { .. } => 0,
                SlotKind::Offered(_) => 1,
                SlotKind::Ok => 2,
                SlotKind::Button { button, .. } => {
                    put_ids(ids, 0, v.names.buttons.get(button.0 as usize).copied().unwrap_or_default());
                    3
                }
                SlotKind::Empty => 4,
                SlotKind::Hidden => 5,
            };
            let at = chips::on_screen;
            one_hot(row, at, 6, kind);
            let state = match s.state {
                SlotState::Selectable => 0,
                SlotState::Unavailable => 1,
                SlotState::Selected => 2,
            };
            one_hot(row, at + 6, 3, state);
            row[at + 9] = flag(screen.cursor as usize == slot);
            row[at + 10] = screen.selection().iter().position(|&p| p as usize == slot).map_or(0.0, |p| (p + 1) as f32 / 5.0);
            row[at + 11] = s.uses_left as f32 / 3.0;
            row[at + 12] = flag(s.marks != 0);
            row[at + 13] = flag(s.attached.is_some());
            row[at + 14] = flag(matches!(s.kind, SlotKind::Chip { regular: true, .. }));
            row[at + 15] = flag(screen.offers[slot].is_some_and(|o| o.shades));
            row[chips::pointer] = flag(legal[slot]);
            row[chips::pointer + 1] = flag(extra.refused & (1 << slot) != 0);
            row[chips::pointer + 2] = steps[slot].map_or(1.0, |n| clip(n as f32 / 6.0, 0.0, 1.0));
            let chip = screen.chip_in(slot as u8, &side.folder).or_else(|| s.face.map(|f| nettai_battle::custom::FolderChip::new(f, nettai_battle::content::ChipCode(0))));
            if let Some(c) = chip {
                chip_facts(content, c.id, matches!(s.kind, SlotKind::Chip { .. } | SlotKind::Offered(_)).then_some(c.code.0), row);
                put_ids(ids, 0, v.names.chip(c.id));
            }
            // What picking it would add (§4.2): a Program Advance it
            // completes, the damage the hand gains, a modifier that would
            // stand alone.
            if s.state == SlotState::Selectable
                && matches!(s.kind, SlotKind::Chip { .. } | SlotKind::Offered(_))
                && let Some(c) = chip
            {
                let with = picks(b, v.me, &screen, Some(slot as u8));
                let built = would_build(b, v.me, &with);
                if let Some(pa) = built.program_advance.filter(|pa| base_built.program_advance.is_none_or(|old| old.chip != pa.chip)) {
                    row[chips::would_be] = 1.0;
                    row[chips::would_be + 1] = content.chip(pa.chip).damage as f32 / 400.0;
                }
                row[chips::would_be + 2] = clip((hand_damage(&built) - base_damage) / 400.0, -5.0, 5.0);
                let last = base.last().map(|p| p.chip.id);
                row[chips::would_be + 3] = flag(builder::stays_unfolded(content, last, c.id));
            }
        }
    }
    // The other side's last chip, as its telop named it.
    if let Some(u) = b.used_chip_for(v.me) {
        let (row, ids) = out.token(spec::CHIPS, spec::USED_TOKEN);
        row[chips::role + 2] = 1.0;
        chip_facts(content, u.chip, None, row);
        row[chips::used] = 1.0;
        row[chips::used + 1] = u.damage as f32 / 400.0;
        row[chips::used + 2] = u.bonus as f32 / 100.0;
        row[chips::used + 3] = flag(u.doubled);
        put_ids(ids, 0, v.names.chip(u.chip));
    }
}

fn encode_rules(v: &View, out: &mut Obs) {
    let b = v.b;
    let defs = &b.content.defs;
    let Some(r) = defs.rules() else { return };
    let stats_schema = defs.schema(r.stats);
    let stats = &b.stats[v.me as usize].game;
    let state = b.rules_state(v.me);
    let mut n = 0;
    for s in &v.names.scalars {
        if n == spec::SETS[spec::RULES].count {
            return;
        }
        let value = if s.stats {
            if stats.id() != r.stats {
                continue;
            }
            place(stats_schema, s).map(|p| stats.get_at(p))
        } else {
            let Some((schema, block)) = state else { continue };
            place(schema, s).map(|p| block.get_at(p))
        };
        let Some(value) = value else { continue };
        let (row, ids) = out.token(spec::RULES, n);
        n += 1;
        row[rules::source + (!s.stats) as usize] = 1.0;
        put_ids(ids, 0, s.name);
        let (ty, number, enumed): (usize, Option<i64>, Option<u8>) = match value {
            FieldValue::Bool(x) => (0, Some(x as i64), None),
            FieldValue::U8(x) => (1, Some(x as i64), None),
            FieldValue::U16(x) => (1, Some(x as i64), None),
            FieldValue::U32(x) => (1, Some(x as i64), None),
            FieldValue::I8(x) => (1, Some(x as i64), None),
            FieldValue::I16(x) => (1, Some(x as i64), None),
            FieldValue::I32(x) => (1, Some(x as i64), None),
            FieldValue::OptionalU8(x) => (1, x.map(|x| x as i64), None),
            FieldValue::Enum(x) => (2, None, Some(x)),
            FieldValue::Ref(d) => {
                if let Some((reg, h)) = d {
                    put_ids(ids, 2, v.names.def(reg, h));
                }
                (3, d.map(|_| 1), None)
            }
            FieldValue::Code(c) => (4, c.map(|c| (c as i64 - b'A' as i64).clamp(0, 26)), None),
            FieldValue::Object(o) => (5, o.map(|_| 1), None),
            _ => (5, None, None),
        };
        row[rules::ty + ty] = 1.0;
        if let Some(x) = number {
            let x = x as f32;
            row[rules::value] = flag(x != 0.0);
            row[rules::value + 1] = clip(x / 100.0, -10.0, 10.0);
            row[rules::value + 2] = x.signum() * x.abs().ln_1p() / 8.0;
        }
        if let Some(e) = enumed {
            row[rules::value] = 1.0;
            row[rules::value + 3] = e as f32 / 8.0;
        }
        if let Some((k, len)) = s.elem {
            row[rules::place] = 1.0;
            row[rules::place + 1] = k as f32 / len as f32;
        }
    }
}

/// Where a rules token's scalar is in a block of `schema`.
fn place<'s>(schema: &'s Schema, s: &Scalar) -> Option<nettai_content_api::Place<'s>> {
    let mut p = schema.place(s.top);
    if let Some(j) = s.field {
        p = p.field_at(j);
    }
    match s.elem {
        Some((k, _)) => p.elem(k),
        None => Some(p),
    }
}

fn encode_forms(v: &View, out: &mut Obs) {
    let b = v.b;
    let content = &*b.content;
    let mut n = 0;
    let mut put = |out: &mut Obs, form: nettai_content_api::FormHandle, role: usize, window: Option<[f32; 4]>| {
        if n == spec::SETS[spec::FORMS].count {
            return;
        }
        let (row, ids) = out.token(spec::FORMS, n);
        n += 1;
        row[forms::role + role] = 1.0;
        if let Some(w) = window {
            row[forms::window..forms::window + 4].copy_from_slice(&w);
        }
        let f = content.form(form);
        let mut at = forms::facts;
        one_hot(row, at, 6, (f.element as usize).min(5));
        at += 6;
        for k in 0..4 {
            row[at + k] = flag(f.weakness.0 & (0x10 << k) != 0);
        }
        at += 4;
        row[at] = flag(f.base);
        row[at + 1] = f.buster_bonus as f32 / 5.0;
        row[at + 2] = flag(f.chip_bonus.is_some());
        row[at + 3] = f.null_bonus as f32 / 50.0;
        row[at + 4] = f.charged_chips.len() as f32 / 4.0;
        put_ids(ids, 0, v.names.forms.get(form.0 as usize).copied().unwrap_or_default());
    };
    put(out, b.stats[v.me as usize].form, 0, None);
    let (_, other) = navis(v);
    if other.is_some() {
        put(out, b.stats[v.them as usize].form, 1, None);
    }
    if b.round.mode == mode::CUSTOM && b.custom.sides[v.me as usize].screen.is_some() {
        if let Some(list) = b.form_list(v.me) {
            for k in 0..(list.count as usize).min(list.forms.len()) {
                if let Some(f) = list.forms[k] {
                    let w = [k as f32 / 5.0, flag(list.marked[k]), flag(list.cursor as usize == k), flag(list.chosen == Some(k as u8))];
                    put(out, f, 2, Some(w));
                }
            }
        }
        if let Some(f) = b.offer(v.me).and_then(|o| o.form) {
            put(out, f, 2, None);
        }
    }
}

#[cfg(test)]
mod tests;
