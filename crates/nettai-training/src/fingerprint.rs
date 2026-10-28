//! Chip fingerprints (docs/design/training-ai.md §3, §12): what each chip
//! does, measured by running it, so a chip the network has seen little of
//! is still described by what it does, and setups (a chip that changes
//! panels or inflicts a status) can be matched with the payoffs they raise.
//!
//! For each chip, a sandbox battle of the game: two of its default builds,
//! through the first custom screen, every panel the rules' normal type, the
//! field split three columns a side; the user (side 0) on its front column's
//! middle panel, holding that chip alone, and the target (side 1, the other
//! navi, pressing nothing) walked to each of the nine panels of its area in
//! turn: 1 to 3 columns ahead of the user, a row above, level or below. The
//! user presses A, and the battle runs until it is quiet (the user idle,
//! none of the chip's attacks left, not dimmed) or [`RUN_TICKS`] ticks.
//! Recorded, by target panel: the damage dealt (0 where it hit nothing) and
//! the tick of the first hit; and of the run that hit hardest (the middle
//! one if none hit): the statuses inflicted on the target (by the engine's
//! status timer index), the user's HP and barrier gained, the panels whose
//! type changed and to which, the panels whose owner changed and to whom,
//! the objects left on the field.
//!
//! Then the conditioned runs, at the panel the chip hit hardest (the middle
//! one if none): the target standing on each panel type the content declares
//! (set as the run starts); in each status the content's statuses set (put
//! on it by a hit of the engine's own carrying it, then the chip under test
//! while the status holds); the user's front column held by the other side
//! (as a steal leaves it: `Battle::set_panel_alliance` and the column's
//! timer), the user a column back; and the front line advanced (the other
//! side's front column the user's, the user on it), the target a column
//! farther. Each is the damage as a ratio to the plain run's at the same
//! panel; a chip that hits nothing plainly and hits under a condition comes
//! out at the most, [`MAX_RATIO`].
//!
//! A pair (setup, payoff) is a chip whose effect brings a condition (the
//! panel under the target turned to a type, a status on it, a panel of the
//! other side's front column taken) and a chip whose damage that condition
//! raises past [`PAIR_RATIO`].
//!
//! A chip's fingerprint is its own alone ([`Conditions`] are the content's):
//! the same whatever chips are fingerprinted with it, so live play makes only
//! the chips it may see. A chip that arrives through a form ([`chip_forms`])
//! is run with that form chosen, as the rules have it.
//!
//! Nothing here names a game: a chip is what the battle shows of it.

use crate::builds::Maker;
use crate::env::step_battle;
use nettai_battle::battle::mode;
use nettai_battle::custom::FolderChip;
use nettai_battle::field::PanelType;
use nettai_battle::kinds::player::NaviAction;
use nettai_battle::object::ObjectRef;
use nettai_battle::content::CollisionRole;
use nettai_battle::{Battle, Content};
use nettai_content_api::{ChipHandle, FormHandle, KindHandle, Pool, StatusHandle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

/// The ticks a run lasts at most after the press: long enough for a navi
/// chip's dimmed sequence to land (EXE6's take up to about 400 ticks; at
/// 180 most of them hit nothing).
pub const RUN_TICKS: u32 = 600;
/// The ticks the observation's times of a run are in (`observe`: the first
/// hit, the longest-lived object, the run's length): the first limit's, so
/// the chips that fitted in it read as they did.
pub const TIME_SCALE: u32 = 180;
/// The ticks a run goes on at least.
const MIN_TICKS: u32 = 20;
/// A condition raising a payoff's damage by this ratio makes a pair.
pub const PAIR_RATIO: f32 = 1.3;
/// The status timers a fingerprint knows (the engine's `collision::timer`).
pub const STATUSES: usize = 9;
/// The panel types a fingerprint's conditions keep, those whose ratio is
/// farthest from 1.
pub const KIND_CONDITIONS: usize = 3;
/// The most a ratio is kept at.
pub const MAX_RATIO: f32 = 4.0;
/// The ticks a column taken in a condition stays taken (past any run).
const STOLEN_TICKS: u16 = 0x708;

/// The user's panel: its front column's middle.
pub const USER: (u8, u8) = (3, 2);

/// The nine panels of the target's area, as the user (on the left) sees
/// them: columns 4 to 6 (1 to 3 ahead of the user), rows 1 to 3, the row
/// first.
pub fn targets() -> [(u8, u8); 9] {
    std::array::from_fn(|i| (4 + (i % 3) as u8, 1 + (i / 3) as u8))
}

/// The middle of the target's area.
const MIDDLE: usize = 4;

/// What a run is set up with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Setup {
    Plain,
    /// The panel under the target of this type.
    Kind(PanelType),
    /// This status on the target, put there by a hit of the engine's own
    /// that carries it ([`Conditions`]).
    Status(StatusHandle),
    /// The user's front column held by the other side, the user a column
    /// back.
    Stolen,
    /// The other side's front column the user's, the user on it.
    Advanced,
}

/// What one run of a chip showed.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Run {
    pub damage: u16,
    pub first_hit: Option<u32>,
    /// Status timers the run started on the target (a bit a timer).
    pub statuses: u16,
    /// The user's HP after less before, and the barrier it gained.
    pub user_hp: i32,
    pub user_barrier: u16,
    /// The panels whose type changed (a bit a panel, the 18 as the user
    /// sees them), and the type each changed to.
    pub changed: u32,
    pub changed_to: Vec<PanelType>,
    /// The type the panel under the target turned to, if it did.
    pub under_target: Option<PanelType>,
    /// The panels whose owner changed (a bit a panel), and of those the ones
    /// now the user's.
    pub owner_changed: u32,
    pub owner_to_user: u32,
    /// The objects the run left (actors and attacks new since the press):
    /// how many, of the user's side, of the other's, and the kind of the
    /// longest-lived and its ticks.
    pub left: u8,
    pub left_own: u8,
    pub left_other: u8,
    #[serde(with = "kind_ticks")]
    pub longest: Option<(KindHandle, u32)>,
    /// The ticks the run took.
    pub ticks: u32,
}

/// A chip's fingerprint.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Fingerprint {
    /// By target panel ([`targets`]): damage and first hit.
    pub runs: Vec<(u16, Option<u32>)>,
    /// The panel it hit hardest (none: it hit none), and that run.
    pub best: Option<usize>,
    pub effect: Run,
    /// Statuses inflicted on the target in any run, a bit a timer; and the
    /// types the panel under the target turned to in any run, a bit a type.
    pub statuses: u16,
    pub turns_under_target: u64,
    /// The conditioned runs' damage, as a ratio to the plain run's: by
    /// status timer, and by panel type.
    pub status_ratios: [Option<f32>; STATUSES],
    pub kind_ratios: Vec<(PanelType, f32)>,
    /// The front column stolen and the front line advanced: the ratio and
    /// the damage.
    pub stolen: Option<(f32, u16)>,
    pub advanced: Option<(f32, u16)>,
    /// Some run took a panel of the other side's front column for the user.
    pub takes_front: bool,
}

/// A condition a setup brings and a payoff's damage is raised by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Condition {
    Kind(PanelType),
    Status(u8),
    /// The front line advanced: a panel of the other side's front column
    /// taken.
    Advanced,
    /// The user's front column held by the other side (no setup of the
    /// user's brings it: no pair is of it).
    Stolen,
}

impl Condition {
    /// The kind of effect a setup brings it by, for drawing pairs by kind
    /// (§6.2): a panel type made, a status inflicted, ownership taken.
    pub fn category(self) -> &'static str {
        match self {
            Condition::Kind(_) => "panel",
            Condition::Status(_) => "status",
            Condition::Advanced | Condition::Stolen => "ownership",
        }
    }
}

/// A condition tells nothing when its ratio is the same (to this) for at
/// least [`SAME_SHARE`] of the chips run under it.
const SAME_TO: f32 = 0.01;
pub const SAME_SHARE: f32 = 0.95;

/// A pair: a setup chip whose effect brings `condition`, and a payoff whose
/// damage it raises by `ratio`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pair {
    #[serde(with = "chip")]
    pub setup: ChipHandle,
    #[serde(with = "chip")]
    pub payoff: ChipHandle,
    pub condition: Condition,
    pub ratio: f32,
}

/// A game's fingerprints and pairs, and what computing them took.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Fingerprints {
    /// The sandbox's version ([`VERSION`]) and the content's hash they are
    /// of.
    pub version: u32,
    pub hash: u64,
    #[serde(with = "chip_map")]
    pub chips: HashMap<ChipHandle, Fingerprint>,
    pub pairs: Vec<Pair>,
    /// The conditions that tell nothing of these chips ([`uninformative`]).
    pub uninformative: Vec<Condition>,
    /// The runs the engine stopped (a content error), said.
    #[serde(skip)]
    pub stops: Vec<String>,
    #[serde(skip)]
    pub cost: Cost,
}

/// The sandbox's version: a change to how chips are run makes the
/// fingerprints kept on disk another's.
pub const VERSION: u32 = 4;

/// Handles as numbers in the file.
mod chip {
    use nettai_content_api::ChipHandle;
    pub fn serialize<S: serde::Serializer>(h: &ChipHandle, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u16(h.0)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<ChipHandle, D::Error> {
        <u16 as serde::Deserialize>::deserialize(d).map(ChipHandle)
    }
}

mod kind_ticks {
    use nettai_content_api::KindHandle;
    pub fn serialize<S: serde::Serializer>(v: &Option<(KindHandle, u32)>, s: S) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&v.map(|(k, t)| (k.0, t)), s)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<(KindHandle, u32)>, D::Error> {
        <Option<(u16, u32)> as serde::Deserialize>::deserialize(d).map(|v| v.map(|(k, t)| (KindHandle(k), t)))
    }
}

mod chip_map {
    use super::Fingerprint;
    use nettai_content_api::ChipHandle;
    use std::collections::HashMap;
    pub fn serialize<S: serde::Serializer>(m: &HashMap<ChipHandle, Fingerprint>, s: S) -> Result<S::Ok, S::Error> {
        let mut v: Vec<(u16, &Fingerprint)> = m.iter().map(|(c, f)| (c.0, f)).collect();
        v.sort_by_key(|(c, _)| *c);
        serde::Serialize::serialize(&v, s)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<HashMap<ChipHandle, Fingerprint>, D::Error> {
        <Vec<(u16, Fingerprint)> as serde::Deserialize>::deserialize(d).map(|v| v.into_iter().map(|(c, f)| (ChipHandle(c), f)).collect())
    }
}

/// What computing fingerprints took: chips, runs, ticks, and the time
/// (summed over threads), and the wall time.
#[derive(Clone, Copy, Debug, Default)]
pub struct Cost {
    pub chips: usize,
    pub runs: u64,
    pub ticks: u64,
    /// Runs the engine stopped.
    pub stopped: u64,
    pub nanos: u64,
    pub wall_nanos: u64,
}

/// A user's hand of `chips` in `b`, the next first.
pub(crate) fn give(b: &mut Battle, side: u8, chips: &[ChipHandle]) {
    let h = &mut b.hands[side as usize];
    h.cursor = 0;
    for i in 0..6 {
        let id = chips.get(i).copied();
        h.ids[i] = id;
        h.damage[i] = id.map_or(0, |c| b.content.chip(c).damage);
        h.attack_bonus[i] = 0;
        h.charge_bonus[i] = 0;
        h.modifiers[i] = 0;
        h.selection[i] = id.map(|c| FolderChip::new(c, b.content.chip(c).codes.first().copied().unwrap_or(nettai_battle::content::ChipCode(0))));
    }
}

fn navi(b: &Battle, side: u8) -> Option<ObjectRef> {
    b.player(side).filter(|&r| b.objects.is_allocated(r))
}

fn idle(b: &Battle, side: u8) -> bool {
    navi(b, side).and_then(|r| b.objects.get(r).actor).is_some_and(|a| b.actors.get(a).navi_action == NaviAction::Idle)
}

fn hp(b: &Battle, side: u8) -> u16 {
    navi(b, side).map_or(0, |r| b.objects.get(r).hp)
}

fn collision(b: &Battle, side: u8) -> Option<&nettai_battle::collision::CollisionData> {
    navi(b, side).and_then(|r| b.objects.get(r).collision).map(|c| b.collision.get(c))
}

fn panel_of(b: &Battle, side: u8) -> Option<(u8, u8)> {
    navi(b, side).map(|r| {
        let p = b.objects.get(r).panel;
        (p.x, p.y)
    })
}

/// Walk side `side`'s navi to panel `to`, the other pressing nothing;
/// whether it got there, idle.
fn walk(b: &mut Battle, side: u8, to: (u8, u8)) -> bool {
    use nettai_battle::input::keys;
    let mut released = true;
    for _ in 0..120 {
        let Some(at) = panel_of(b, side) else { return false };
        if at == to && idle(b, side) {
            return true;
        }
        // (RIGHT is a side's forward, toward the other side: +x for side 0,
        // −x for side 1.)
        let (plus, minus) = if side == 0 { (keys::RIGHT, keys::LEFT) } else { (keys::LEFT, keys::RIGHT) };
        let dir = if at.0 < to.0 {
            plus
        } else if at.0 > to.0 {
            minus
        } else if at.1 < to.1 {
            keys::DOWN
        } else if at.1 > to.1 {
            keys::UP
        } else {
            0
        };
        // (A press when idle, let go between.)
        let press = if idle(b, side) && released { dir } else { 0 };
        released = press == 0;
        let mut held = [0; 2];
        held[side as usize] = press;
        if !step_battle(b, held) {
            return false;
        }
    }
    false
}

/// The sandbox a game's chips run in: its battle at the start of a run, and
/// the control runs made so far (what each setup costs the target with no
/// chip used, by tick).
pub struct Sandbox {
    base: Battle,
    /// The forms the chips that arrive through one arrive through
    /// ([`chip_forms`]).
    forms: HashMap<ChipHandle, FormHandle>,
    controls: std::cell::RefCell<HashMap<(Setup, usize), Vec<u16>>>,
    /// The runs the engine stopped (a content error), said.
    stops: std::cell::RefCell<Vec<String>>,
}

/// The ticks a status's hit has to land in.
const STATUS_TICKS: u32 = 8;

impl Sandbox {
    /// The game's sandbox: two of its default builds (the first group's
    /// new build), through the first custom screen with no chip picked,
    /// every playable panel the rules' normal type, the fight running.
    pub fn new(content: &Arc<Content>) -> Result<Sandbox, String> {
        let maker = Maker::new(content)?;
        let side = maker.default_build()?;
        let m = maker.round([side.clone(), side], 1000, 1)?;
        let mut b = nettai_match::Set::of(content, &m, 1).start();
        let mut navs = [crate::act::Navigator::default(); 2];
        for _ in 0..2000 {
            if crate::env::fight_runs(&b) && idle(&b, 0) && idle(&b, 1) {
                let normal = b.game_rules().panels.roles.normal;
                for y in 1..=3u8 {
                    for x in 1..=6u8 {
                        b.set_panel_type(x, y, normal);
                        let p = &b.field.panels[y as usize][x as usize];
                        let side = (x > 3) as u8;
                        if p.alliance != side || p.home != side {
                            return Err(format!("{}: the sandbox's stage isn't three columns a side", content.game()));
                        }
                    }
                }
                if !walk(&mut b, 0, USER) {
                    return Err(format!("{}: the sandbox's user can't stand on its front column", content.game()));
                }
                return Ok(Sandbox { base: b, forms: chip_forms(content), controls: Default::default(), stops: Default::default() });
            }
            let mut held = [0u16; 2];
            if b.round.mode == mode::CUSTOM {
                for s in 0..2 {
                    if navs[s].idle() {
                        navs[s].point(nettai_battle::custom::screen::OK_SLOT as i32);
                    }
                    held[s] = navs[s].buttons(&b, s as u8);
                }
            }
            if !step_battle(&mut b, held) {
                return Err(format!("{}: the sandbox stopped: {:?}", content.game(), b.round_end()));
            }
        }
        Err(format!("{}: the sandbox's fight never started", content.game()))
    }

    /// Run `chip` against the target at panel `target` ([`targets`]) as
    /// `setup` sets the run up; none where the run can't be set up (a
    /// panel not reached, a status not taken). Its damage is the target's HP
    /// lost less what the setup costs it with no chip used (a panel's own
    /// damage, the first chip's that goes on).
    pub fn run(&self, chip: ChipHandle, target: usize, setup: Setup) -> Option<(Run, u32)> {
        let (mut b, ticks) = self.prepared(target, setup)?;
        let control = self.control(&b, target, setup);
        // (A chip that arrives through a form, with that form the side's
        // choice at OK, as the rules have it when they put the chip in the
        // hand: what the custom screen's OK sends, and this turn's.)
        if let Some(&form) = self.forms.get(&chip) {
            b.transform_requests[0].form = Some(form);
            b.turn_transforms[0].form = Some(form);
        }
        give(&mut b, 0, &[chip]);
        let (run, n) = watch(&mut b, targets()[target], &control);
        self.note(&b, || format!("{} {setup:?} at panel {target}", b.content.defs.chip(chip).key));
        Some((run, ticks + n))
    }

    /// The runs the engine stopped so far, said (and forgotten here).
    pub fn take_stops(&self) -> Vec<String> {
        std::mem::take(&mut self.stops.borrow_mut())
    }

    /// Note `b` stopped, if it was: what ran (`what`), and why.
    fn note(&self, b: &Battle, what: impl FnOnce() -> String) {
        if let Some(nettai_battle::RoundEnd::Error(why)) = b.round_end() {
            self.stops.borrow_mut().push(format!("{}: {}: {why}", b.content.game(), what()));
        }
    }

    /// A chip's fingerprint: its plain runs at each target panel, then its
    /// conditioned runs at its best panel (the middle one if it hit none);
    /// and the runs made and their ticks. Of it alone: nothing of another
    /// chip goes in.
    pub fn fingerprint(&self, chip: ChipHandle, c: &Conditions) -> (Fingerprint, u64, u64) {
        let (mut runs, mut ticks) = (0u64, 0u64);
        let mut ran = |n: u32| {
            runs += 1;
            ticks += n as u64;
        };
        let mut fp = Fingerprint::default();
        let mut all = Vec::new();
        for target in 0..9 {
            let Some((run, n)) = self.run(chip, target, Setup::Plain) else {
                fp.runs.push((0, None));
                all.push(Run::default());
                continue;
            };
            ran(n);
            fp.runs.push((run.damage, run.first_hit));
            fp.statuses |= run.statuses;
            if let Some(k) = run.under_target {
                fp.turns_under_target |= 1 << k.0;
            }
            // (A panel of the other side's front column now the user's.)
            fp.takes_front |= (0..3).any(|row| run.owner_to_user & (1 << (row * 6 + USER.0 as usize)) != 0);
            all.push(run);
        }
        let best = (0..9).filter(|&i| fp.runs[i].0 > 0).max_by_key(|&i| (fp.runs[i].0, i == MIDDLE));
        fp.best = best;
        fp.effect = all[best.unwrap_or(MIDDLE)].clone();
        let best = best.unwrap_or(MIDDLE);
        let plain = fp.runs[best].0;
        for &k in &c.kinds {
            if let Some((run, n)) = self.run(chip, best, Setup::Kind(k)) {
                ran(n);
                fp.kind_ratios.push((k, ratio(run.damage, plain)));
            }
        }
        for (s, status) in c.statuses.iter().enumerate() {
            let Some(status) = *status else { continue };
            if let Some((run, n)) = self.run(chip, best, Setup::Status(status)) {
                ran(n);
                fp.status_ratios[s] = Some(ratio(run.damage, plain));
            }
        }
        if let Some((run, n)) = self.run(chip, best, Setup::Stolen) {
            ran(n);
            fp.stolen = Some((ratio(run.damage, plain), run.damage));
        }
        // (Advanced: the target a column farther than the user's front, in
        // the best panel's row, against the plain run there.)
        let farther = best / 3 * 3 + 1;
        if let Some((run, n)) = self.run(chip, farther, Setup::Advanced) {
            ran(n);
            fp.advanced = Some((ratio(run.damage, fp.runs[farther].0), run.damage));
        }
        (fp, runs, ticks)
    }

    /// What `setup` costs the target at `target`, with no chip used, by tick
    /// (from `b`, the run set up): made once.
    fn control(&self, b: &Battle, target: usize, setup: Setup) -> Vec<u16> {
        if let Some(c) = self.controls.borrow().get(&(setup, target)) {
            return c.clone();
        }
        let mut c = b.clone();
        give(&mut c, 0, &[]);
        let hp0 = hp(&c, 1);
        let mut lost = vec![0u16; RUN_TICKS as usize + 1];
        for l in lost.iter_mut().skip(1) {
            step_battle(&mut c, [0, 0]);
            *l = hp0.saturating_sub(hp(&c, 1));
        }
        self.note(&c, || format!("the control run {setup:?} at panel {target}"));
        self.controls.borrow_mut().insert((setup, target), lost.clone());
        lost
    }

    /// The sandbox set up for a run at `target` as `setup` says, and the
    /// ticks that took past walking.
    pub(crate) fn prepared(&self, target: usize, setup: Setup) -> Option<(Battle, u32)> {
        let to = targets()[target];
        let mut b = self.base.clone();
        let mut ticks = 0;
        match setup {
            Setup::Stolen => {
                // (The user a column back, then its front column the other
                // side's as a steal leaves it.)
                if !walk(&mut b, 0, (USER.0 - 1, USER.1)) {
                    return None;
                }
                steal(&mut b, USER.0, 1);
            }
            Setup::Advanced => {
                if !walk(&mut b, 1, to) {
                    return None;
                }
                steal(&mut b, USER.0 + 1, 0);
                if !walk(&mut b, 0, (USER.0 + 1, USER.1)) {
                    return None;
                }
            }
            _ => {}
        }
        if !walk(&mut b, 1, to) {
            return None;
        }
        if let Setup::Kind(k) = setup {
            b.set_panel_type(to.0, to.1, k);
        }
        if let Setup::Status(status) = setup {
            // (A hit of the engine's own on the target's panel, an attack of
            // the user's side doing no damage that carries the status, the
            // rules' own collision types for a hit that reaches navis: it
            // lands as any attack's status does, through the target's
            // intake, or doesn't.)
            let t = b.content.status(status).timer.timer()?;
            let owner = navi(&b, 0)?;
            let roles = b.roles();
            let spec = nettai_battle::kinds::hitbox::HitboxSpec {
                panel: nettai_battle::object::PanelPos { x: to.0, y: to.1 },
                region: b.anchor_region(),
                target: roles.collision(CollisionRole::ThrownObstacleTarget),
                self_type: roles.collision(CollisionRole::ThrownObstacle),
                status: Some(status),
                ..Default::default()
            };
            nettai_battle::kinds::hitbox::spawn(&mut b, owner, &spec)?;
            let mut holds = false;
            for _ in 0..STATUS_TICKS {
                let went_on = step_battle(&mut b, [0, 0]);
                self.note(&b, || format!("the status hit {setup:?} at panel {target}"));
                if !went_on {
                    return None;
                }
                ticks += 1;
                holds = collision(&b, 1).is_some_and(|c| c.status_timers[t] > 0);
                if holds {
                    break;
                }
            }
            if !holds {
                return None;
            }
        }
        Some((b, ticks))
    }
}

/// Column `x` held by side `side`, as a steal leaves it: each panel's owner
/// (`Battle::set_panel_alliance`), and the column's timer, which returns it
/// (past any run).
fn steal(b: &mut Battle, x: u8, side: u8) {
    for y in 1..=3 {
        b.set_panel_alliance(x, y, side);
    }
    b.field.columns[x as usize].timer = STOLEN_TICKS;
}

/// Press A for the user's hand in `b` and watch the run until quiet or
/// [`RUN_TICKS`]; the target at `to`, `control` what it would lose with no
/// chip used, by tick.
fn watch(b: &mut Battle, to: (u8, u8), control: &[u16]) -> (Run, u32) {
    let mut run = Run::default();
    let (hp0, user0) = (hp(b, 1), hp(b, 0));
    let barrier0 = collision(b, 0).map_or(0, |c| c.barrier_hp);
    let timers0 = collision(b, 1).map_or([0; STATUSES], |c| c.status_timers);
    let kinds0: Vec<PanelType> = (1..=3u8).flat_map(|y| (1..=6u8).map(move |x| (x, y))).map(|(x, y)| b.field.panels[y as usize][x as usize].kind).collect();
    let owners0: Vec<u8> = (1..=3u8).flat_map(|y| (1..=6u8).map(move |x| (x, y))).map(|(x, y)| b.field.panels[y as usize][x as usize].alliance).collect();
    let before: Vec<(ObjectRef, KindHandle)> = b.objects.in_order().map(|r| (r, b.objects.get(r).kind)).collect();
    let mut born: Vec<(ObjectRef, KindHandle, u32)> = Vec::new();
    let mut t = 0;
    let mut barrier = barrier0;
    while t < RUN_TICKS {
        let press = if t == 0 { nettai_battle::input::keys::A } else { 0 };
        let went_on = step_battle(b, [press, 0]);
        t += 1;
        let lost = hp0.saturating_sub(hp(b, 1));
        if lost > control[t as usize] && run.first_hit.is_none() {
            run.first_hit = Some(t);
        }
        if let Some(c) = collision(b, 1) {
            for (i, &v) in c.status_timers.iter().enumerate() {
                if v > 0 && timers0[i] == 0 {
                    run.statuses |= 1 << i;
                }
            }
        }
        barrier = barrier.max(collision(b, 0).map_or(0, |c| c.barrier_hp));
        for r in b.objects.in_order() {
            let kind = b.objects.get(r).kind;
            if r.pool != Pool::Effect && !before.contains(&(r, kind)) && !born.iter().any(|&(x, k, _)| x == r && k == kind) {
                born.push((r, kind, t));
            }
        }
        born.retain(|&(r, k, _)| b.objects.is_allocated(r) && b.objects.get(r).kind == k);
        if !went_on || hp(b, 1) == 0 {
            break;
        }
        let attacks_left = born.iter().any(|(r, _, _)| r.pool == Pool::Attack);
        if t >= MIN_TICKS && idle(b, 0) && !b.is_dimmed() && !attacks_left {
            break;
        }
    }
    run.ticks = t;
    run.damage = hp0.saturating_sub(hp(b, 1)).saturating_sub(control[t as usize]);
    run.user_hp = hp(b, 0) as i32 - user0 as i32;
    run.user_barrier = barrier.saturating_sub(barrier0) as u16;
    for (i, (x, y)) in (1..=3u8).flat_map(|y| (1..=6u8).map(move |x| (x, y))).enumerate() {
        let owner = b.field.panels[y as usize][x as usize].alliance;
        if owner != owners0[i] {
            run.owner_changed |= 1 << i;
            if owner == 0 {
                run.owner_to_user |= 1 << i;
            }
        }
        let kind = b.field.panels[y as usize][x as usize].kind;
        if kind != kinds0[i] {
            run.changed |= 1 << i;
            run.changed_to.push(kind);
            if (x, y) == to {
                run.under_target = Some(kind);
            }
        }
    }
    let user_side = 0;
    for &(r, k, from) in &born {
        run.left += 1;
        match b.objects.get(r).alliance {
            a if a == user_side => run.left_own += 1,
            a if a == 1 - user_side => run.left_other += 1,
            _ => {}
        }
        let life = t - from;
        if run.longest.is_none_or(|(_, l)| life > l) {
            run.longest = Some((k, life));
        }
    }
    (run, t)
}

/// The type most of a run's changed panels turned to.
pub fn top_change(run: &Run) -> Option<PanelType> {
    let mut counts: Vec<(PanelType, usize)> = Vec::new();
    for &k in &run.changed_to {
        match counts.iter_mut().find(|(x, _)| *x == k) {
            Some(c) => c.1 += 1,
            None => counts.push((k, 1)),
        }
    }
    counts.into_iter().max_by_key(|&(k, n)| (n, std::cmp::Reverse(k))).map(|(k, _)| k)
}

/// The ratio of a conditioned run's damage to the plain run's (neither hit:
/// 1; only the conditioned: the most, [`MAX_RATIO`]).
pub fn ratio(conditioned: u16, plain: u16) -> f32 {
    match (conditioned, plain) {
        (0, 0) => 1.0,
        (_, 0) => MAX_RATIO,
        (c, p) => (c as f32 / p as f32).min(MAX_RATIO),
    }
}

impl Fingerprints {
    /// The fingerprints of `chips` of `content`'s game, on `threads`
    /// threads (each loads the content's runtime first), and the pairs
    /// among them: a [`Job`] those threads work on.
    pub fn compute(content: &Arc<Content>, chips: &[ChipHandle], threads: usize) -> Result<Fingerprints, String> {
        let wall = std::time::Instant::now();
        let threads = threads.max(1).min(chips.len().max(1));
        let job = Job::new(content, chips);
        let busy = AtomicU64::new(0);
        std::thread::scope(|scope| {
            for _ in 0..threads {
                scope.spawn(|| {
                    let started = std::time::Instant::now();
                    job.work();
                    busy.fetch_add(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
                });
            }
        });
        let made = job.done.into_inner().unwrap_or_else(|| Err("the fingerprints' threads ended with chips left".into()))?;
        let mut out = Arc::try_unwrap(made).unwrap_or_else(|f| (*f).clone());
        out.cost.nanos = busy.into_inner();
        out.cost.wall_nanos = wall.elapsed().as_nanos() as u64;
        Ok(out)
    }

    /// A chip's fingerprint, if it has one.
    pub fn of(&self, chip: ChipHandle) -> Option<&Fingerprint> {
        self.chips.get(&chip)
    }

    /// The fingerprints of every chip of `content`'s game: kept in this
    /// process by content hash, and in `dir` where one is given (a file a
    /// game and hash, `fingerprints-<game>-<hash>.json`); else computed on
    /// `threads` threads, and kept.
    pub fn for_game(content: &Arc<Content>, threads: usize, dir: Option<&std::path::Path>) -> Result<Arc<Fingerprints>, String> {
        static KEPT: std::sync::OnceLock<std::sync::Mutex<HashMap<u64, Arc<Fingerprints>>>> = std::sync::OnceLock::new();
        let hash = content.hash().0;
        let kept = KEPT.get_or_init(Default::default);
        if let Some(f) = kept.lock().unwrap_or_else(|p| p.into_inner()).get(&hash) {
            return Ok(f.clone());
        }
        let file = dir.map(|d| d.join(file_name(content)));
        let read = file.as_ref().and_then(|f| std::fs::read(f).ok()).and_then(|bytes| serde_json::from_slice::<Fingerprints>(&bytes).ok());
        let f = match read.filter(|f| f.version == VERSION && f.hash == hash) {
            Some(f) => f,
            None => {
                let f = Fingerprints::compute(content, &game_chips(content), threads)?;
                if let Some(file) = &file {
                    let bytes = serde_json::to_vec(&f).map_err(|e| e.to_string())?;
                    if let Some(d) = file.parent() {
                        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
                    }
                    std::fs::write(file, bytes).map_err(|e| format!("{}: {e}", file.display()))?;
                }
                f
            }
        };
        let f = Arc::new(f);
        kept.lock().unwrap_or_else(|p| p.into_inner()).insert(hash, f.clone());
        Ok(f)
    }
}

/// The chips that arrive through a form, and the form: a form whose
/// definition names the chip in a field of its rules' extension of forms
/// typed as a chip (EXE4's souls' chips, which OK puts in the hand for the
/// soul given, whose use is the change into it). No field is named: the
/// extension's type says it.
pub fn chip_forms(content: &Content) -> HashMap<ChipHandle, FormHandle> {
    use nettai_battle::content::ExtensionType;
    use nettai_content_api::{Data, FieldType, Registry};
    let mut out = HashMap::new();
    let Some(r) = content.defs.rules() else { return out };
    let fields: Vec<&str> =
        r.extends.iter().filter(|e| e.registry == Registry::Form && matches!(e.ty, ExtensionType::Value(FieldType::Ref(Registry::Chip, _)))).map(|e| e.field.as_str()).collect();
    for (i, f) in content.defs.forms.iter().enumerate() {
        for field in &fields {
            if let Some(Data::Ref(Registry::Chip, key)) = content.defs.extension(Registry::Form, &f.key, field)
                && let Some(chip) = content.defs.chip_by_key(key)
            {
                out.insert(chip, FormHandle(i as u16));
            }
        }
    }
    out
}

/// The chips of `content`'s game that are fingerprinted: its own (an
/// imported chip is its game's).
pub fn game_chips(content: &Content) -> Vec<ChipHandle> {
    let game = content.game();
    (0..content.defs.chips.len() as u16).map(ChipHandle).filter(|&c| nettai_match::ids::in_game(content, game, &content.defs.chip(c).key)).collect()
}

/// The file a game's fingerprints are kept in, in a cache folder: by game
/// and content hash.
pub fn file_name(content: &Content) -> String {
    format!("fingerprints-{}-{:016x}.json", content.game(), content.hash().0)
}

/// The conditions that tell nothing of the chips fingerprinted: those
/// whose ratio is the same (to [`SAME_TO`]) for at least [`SAME_SHARE`] of
/// the chips they apply to (a status every hit leaves, under which nothing
/// hits; a panel type under which no chip's damage changes), but those a
/// pair is made of. A chip a condition applies to hit in its plain run at
/// the condition's panel or under the condition: one that hit in neither is
/// at 1 under every condition, which says nothing of the condition. They
/// are said, not dropped: a chip's fingerprint is its own alone, the same
/// however many of the game's chips are fingerprinted with it (live play
/// fingerprints two folders' chips), so nothing of the others' goes in it.
pub fn uninformative(chips: &HashMap<ChipHandle, Fingerprint>) -> Vec<Condition> {
    let paired: Vec<Condition> = pairs(chips).iter().map(|p| p.condition).collect();
    let mut ratios: HashMap<Condition, Vec<f32>> = HashMap::new();
    for f in chips.values() {
        let best = f.best.unwrap_or(MIDDLE);
        let plain = f.runs.get(best).map_or(0, |r| r.0);
        // (Hit plainly, or only under the condition: its ratio is the most.)
        let applies = |r: f32| plain > 0 || r >= MAX_RATIO;
        for (s, r) in f.status_ratios.iter().enumerate() {
            if let Some(r) = r.filter(|&r| applies(r)) {
                ratios.entry(Condition::Status(s as u8)).or_default().push(r);
            }
        }
        for &(k, r) in f.kind_ratios.iter().filter(|(_, r)| applies(*r)) {
            ratios.entry(Condition::Kind(k)).or_default().push(r);
        }
        if let Some((r, _)) = f.stolen.filter(|&(r, _)| applies(r)) {
            ratios.entry(Condition::Stolen).or_default().push(r);
        }
        let farther = best / 3 * 3 + 1;
        if let Some((r, d)) = f.advanced.filter(|&(_, d)| d > 0 || f.runs.get(farther).is_some_and(|x| x.0 > 0)) {
            let _ = d;
            ratios.entry(Condition::Advanced).or_default().push(r);
        }
    }
    let mut dropped: Vec<Condition> = ratios
        .into_iter()
        .filter(|(_, rs)| {
            let mut counts: HashMap<i32, usize> = HashMap::new();
            for r in rs {
                *counts.entry((r / SAME_TO).round() as i32).or_default() += 1;
            }
            let most = counts.values().copied().max().unwrap_or(0);
            most as f32 >= SAME_SHARE * rs.len() as f32
        })
        .filter(|(c, _)| !paired.contains(c))
        .map(|(c, _)| c)
        .collect();
    dropped.sort_by_key(|c| format!("{c:?}"));
    dropped
}

/// The pairs among fingerprints: a setup whose runs brought a condition
/// (turned the panel under the target to a type, inflicted a status) and a
/// payoff whose conditioned damage under it is past [`PAIR_RATIO`] times
/// its plain damage. In handle order.
pub fn pairs(chips: &HashMap<ChipHandle, Fingerprint>) -> Vec<Pair> {
    let mut sorted: Vec<(&ChipHandle, &Fingerprint)> = chips.iter().collect();
    sorted.sort_by_key(|(c, _)| c.0);
    let mut out = Vec::new();
    for &(&payoff, p) in &sorted {
        let raised: Vec<(Condition, f32)> = p
            .kind_ratios
            .iter()
            .filter(|(_, r)| *r > PAIR_RATIO)
            .map(|&(k, r)| (Condition::Kind(k), r))
            .chain(p.status_ratios.iter().enumerate().filter_map(|(s, r)| r.filter(|r| *r > PAIR_RATIO).map(|r| (Condition::Status(s as u8), r))))
            .chain(p.advanced.filter(|(r, _)| *r > PAIR_RATIO).map(|(r, _)| (Condition::Advanced, r)))
            .collect();
        for (condition, ratio) in raised {
            for &(&setup, s) in &sorted {
                let brings = match condition {
                    Condition::Kind(k) => s.turns_under_target & (1 << k.0) != 0,
                    Condition::Status(i) => s.statuses & (1 << i) != 0,
                    Condition::Advanced => s.takes_front,
                    Condition::Stolen => false,
                };
                if brings && setup != payoff {
                    out.push(Pair { setup, payoff, condition, ratio });
                }
            }
        }
    }
    out
}

/// The fingerprints of a list of chips being made, by whatever threads work
/// on it ([`Job::work`]: each with the content's runtime and a sandbox of
/// its own, claiming a chip at a time). A chip's fingerprint is its own
/// alone ([`Sandbox::fingerprint`]), so what it makes is the same on any
/// number of threads in any order, and the same for a chip whatever chips
/// are made with it. It reads no clock and starts no thread, so a host makes
/// its own (the browser's workers among them).
pub struct Job {
    content: Arc<Content>,
    chips: Vec<ChipHandle>,
    conditions: Conditions,
    /// The next chip to claim, and the chips done.
    next: AtomicUsize,
    finished: AtomicUsize,
    made: Vec<std::sync::Mutex<Option<Fingerprint>>>,
    stops: std::sync::Mutex<Vec<String>>,
    runs: AtomicU64,
    ticks: AtomicU64,
    /// The fingerprints, once every chip is done (or why they can't be).
    done: OnceLock<Result<Arc<Fingerprints>, String>>,
}

/// What the conditioned runs are set up with, of the content alone: by
/// status timer, the status of the content's that sets it for longest (the
/// first such in handle order), which a status condition's hit carries
/// (none: no status of the content's sets it); the panel types the content
/// declares but the normal role's (the plain runs' own) and the missing and
/// broken roles'.
#[derive(Clone, Debug)]
pub struct Conditions {
    statuses: [Option<StatusHandle>; STATUSES],
    kinds: Vec<PanelType>,
}

impl Conditions {
    pub fn of(content: &Content) -> Conditions {
        let mut statuses: [Option<StatusHandle>; STATUSES] = [None; STATUSES];
        for (i, def) in content.defs.statuses.iter().enumerate() {
            let Some(t) = def.effect.timer.timer().filter(|&t| t < STATUSES) else { continue };
            if statuses[t].is_none_or(|h| content.status(h).duration < def.effect.duration) {
                statuses[t] = Some(StatusHandle(i as u16));
            }
        }
        let p = &content.rules().panels;
        let kinds = (0..p.types.len() as u8).map(PanelType).filter(|&k| k != p.roles.normal && k != p.roles.missing && k != p.roles.broken).collect();
        Conditions { statuses, kinds }
    }

    /// The status a status timer's condition is run with.
    pub fn status(&self, timer: usize) -> Option<StatusHandle> {
        self.statuses.get(timer).copied().flatten()
    }
}

impl Job {
    pub fn new(content: &Arc<Content>, chips: &[ChipHandle]) -> Job {
        let job = Job {
            content: content.clone(),
            chips: chips.to_vec(),
            conditions: Conditions::of(content),
            next: AtomicUsize::new(0),
            finished: AtomicUsize::new(0),
            made: chips.iter().map(|_| std::sync::Mutex::new(None)).collect(),
            stops: Default::default(),
            runs: AtomicU64::new(0),
            ticks: AtomicU64::new(0),
            done: OnceLock::new(),
        };
        if chips.is_empty() {
            job.finish();
        }
        job
    }

    /// Chips done and chips in all.
    pub fn progress(&self) -> (usize, usize) {
        (self.finished.load(Ordering::Relaxed), self.chips.len())
    }

    /// The fingerprints, once every chip is done; or why they can't be made
    /// (a thread's runtime or sandbox that wouldn't load, a run that
    /// panicked).
    pub fn result(&self) -> Option<&Result<Arc<Fingerprints>, String>> {
        self.done.get()
    }

    /// Work on this thread till nothing is left to claim: the content's
    /// runtime and a sandbox loaded, then a chip at a time.
    pub fn work(&self) {
        if self.done.get().is_some() {
            return;
        }
        let made = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            nettai_battle::behavior::Behaviors::for_content(&self.content).map_err(|e| e.to_string())?;
            Sandbox::new(&self.content)
        }));
        let sandbox = match made {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => return self.fail(e),
            Err(_) => return self.fail(format!("{}: the fingerprints' sandbox panicked as it loaded", self.content.game())),
        };
        loop {
            let i = self.next.fetch_add(1, Ordering::SeqCst);
            if i >= self.chips.len() || self.done.get().is_some() {
                return;
            }
            let chip = self.chips[i];
            let item = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| sandbox.fingerprint(chip, &self.conditions)));
            let Ok((fp, runs, ticks)) = item else {
                return self.fail(format!("{}: a fingerprint run of chip {} panicked", self.content.game(), self.content.defs.chip(chip).key));
            };
            self.runs.fetch_add(runs, Ordering::Relaxed);
            self.ticks.fetch_add(ticks, Ordering::Relaxed);
            self.stops.lock().unwrap_or_else(|p| p.into_inner()).extend(sandbox.take_stops());
            *self.made[i].lock().unwrap_or_else(|p| p.into_inner()) = Some(fp);
            if self.finished.fetch_add(1, Ordering::SeqCst) + 1 == self.chips.len() {
                self.finish();
            }
        }
    }

    fn fail(&self, why: String) {
        let _ = self.done.set(Err(why));
    }

    /// Every chip done: the fingerprints, the conditions that tell nothing,
    /// the pairs.
    fn finish(&self) {
        let mut out = Fingerprints { version: VERSION, hash: self.content.hash().0, ..Fingerprints::default() };
        for (&chip, fp) in self.chips.iter().zip(&self.made) {
            if let Some(fp) = fp.lock().unwrap_or_else(|p| p.into_inner()).take() {
                out.chips.insert(chip, fp);
            }
        }
        out.uninformative = uninformative(&out.chips);
        out.pairs = pairs(&out.chips);
        out.stops = std::mem::take(&mut self.stops.lock().unwrap_or_else(|p| p.into_inner()));
        out.cost = Cost {
            chips: self.chips.len(),
            runs: self.runs.load(Ordering::Relaxed),
            ticks: self.ticks.load(Ordering::Relaxed),
            stopped: out.stops.len() as u64,
            nanos: 0,
            wall_nanos: 0,
        };
        let _ = self.done.set(Ok(Arc::new(out)));
    }
}

#[cfg(test)]
mod tests;
