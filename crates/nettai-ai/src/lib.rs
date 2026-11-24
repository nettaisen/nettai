//! Built-in AI player for nettai: plays any game the engine runs (EXE4, EXE5,
//! EXE6, or future games) through [`nettai_frontend::driver::Ai`] with zero
//! hardcoded game/chip names and no external model files.
//!
//! - **Fair perception & reaction delay**: never reads the opponent's buttons
//!   (`theirs`), hand, folder, custom screen, RNG, or objects hidden from its
//!   side. Fight decisions are made from a delayed snapshot (`t - D`), with
//!   sanitized RNG and cleared hidden opponent state.
//! - **Custom screen** ([`screen`], [`nav`]): detects Program Advances, folds
//!   modifiers optimally, evaluates code-compatible hands with `builder::build`,
//!   picks matchup-favorable forms (Crosses / Souls / Beast Out) and probes
//!   rules buttons generically.
//! - **Fight** ([`fight`], [`worker`]): evaluates candidate actions (dodge,
//!   chip use, charge shot release, B+Back special, dimming cut-ins, movement)
//!   via bounded lookahead on a background thread (with inline fallback) and
//!   dynamically probes chip hit-panels so swords, bombs, and line attacks are
//!   aimed accurately without per-chip hardcoding.

mod fight;
mod nav;
mod screen;
mod sim;
mod worker;

use crate::fight::{AttackLog, FightContext, FightDecision, LevelConfig, ReachTable};
use crate::nav::Navigator;
use crate::screen::ScreenPlanner;
use crate::sim::{fight_runs, may_open, navi_panel, presses_l};
use crate::worker::Worker;
use nettai_battle::battle::mode;
use nettai_battle::custom::Layer;
use nettai_battle::input::Keys;
use nettai_battle::setup::SetScore;
use nettai_battle::Battle;
use nettai_content_api::PanelPos;
use nettai_frontend::driver::Ai;
use nettai_netplay::rng::SplitMix64;

/// How well the AI plays (maps to Practice mode's Easy / Normal / Hard).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Level {
    Easy,
    #[default]
    Normal,
    Hard,
}

impl Level {
    pub const ALL: [Level; 3] = [Level::Easy, Level::Normal, Level::Hard];

    pub(crate) fn config(self) -> LevelConfig {
        match self {
            Level::Easy => LevelConfig {
                delay: 22,
                k: 8,
                horizon: 24,
                cap: 180,
                probe_reach: false,
                dodge_chance: 0.62,
                cut_in_chance: 0.15,
                use_back_special: false,
                noise: 0.045,
                max_screen_picks: 3,
                form_chance: 0.35,
                use_redeal: false,
                opener_empty: (40, 110),
                opener_holding: (150, 320),
            },
            Level::Normal => LevelConfig {
                delay: 14,
                k: 6,
                horizon: 36,
                cap: 300,
                probe_reach: true,
                dodge_chance: 0.90,
                cut_in_chance: 0.55,
                use_back_special: true,
                noise: 0.014,
                max_screen_picks: 5,
                form_chance: 0.78,
                use_redeal: true,
                opener_empty: (16, 48),
                opener_holding: (95, 210),
            },
            Level::Hard => LevelConfig {
                delay: 8,
                k: 4,
                horizon: 44,
                cap: 420,
                probe_reach: true,
                dodge_chance: 1.0,
                cut_in_chance: 0.92,
                use_back_special: true,
                noise: 0.0,
                max_screen_picks: 5,
                form_chance: 1.0,
                use_redeal: true,
                opener_empty: (4, 16),
                opener_holding: (75, 160),
            },
        }
    }
}

/// Ticks ahead the background worker starts computing a fight decision.
const WORKER_LEAD: u32 = 2;

/// Ring buffer size for our own past inputs (must exceed max delay + lead).
const INPUT_RING: usize = 64;

struct Pending {
    at: u32,
    job_id: u64,
}

struct RoundState {
    key: (u64, SetScore),
    t: u32,
    fight_ticks: u32,
    attack_log: AttackLog,
    /// Snapshots stored at `(s + D) % k == 0` for delayed fight decisions.
    ring: Vec<(Option<u32>, Option<Box<Battle>>, AttackLog)>,
    /// Our own pressed keys on recent fight ticks, indexed by `t % INPUT_RING`.
    my_inputs: [Keys; INPUT_RING],
    /// Action currently being executed in the fight.
    held_first: Keys,
    held_after: Keys,
    held_first_ticks: u8,
    decision_age: u32,
    /// Panel tracking for anti-pacing hysteresis & delayed opponent movement cadence.
    cur_panel: Option<PanelPos>,
    prev_panel: Option<PanelPos>,
    ticks_since_move: u32,
    strafe_timer: u32,
    enemy_last_panel: Option<PanelPos>,
    enemy_stationary_ticks: u32,
    /// Persistent per-round relative `(dx, dy)` chip hit-offset table.
    reach: ReachTable,
    /// Custom screen state.
    in_screen: bool,
    nav: Navigator,
    planner: ScreenPlanner,
    /// Tick to begin pressing L when the custom gauge is full.
    opener_at: Option<u32>,
    /// Last chip cursor seen in hand, and fight ticks since cursor last advanced.
    last_cursor: u8,
    ticks_same_chip: u32,
}

/// Built-in AI player implementing [`nettai_frontend::driver::Ai`].
pub struct Fighter {
    level: Level,
    cfg: LevelConfig,
    seed: u64,
    open_custom: bool,
    rng: SplitMix64,
    round: Option<RoundState>,
    worker: Option<Worker>,
    pending: Option<Pending>,
}

impl Fighter {
    /// Create a new AI player at `level`, seeded by `seed`. Uses a background
    /// lookahead worker thread when available, falling back cleanly to inline
    /// evaluation if thread creation fails.
    pub fn new(level: Level, seed: u64) -> Fighter {
        let worker = Worker::start().ok();
        Fighter::with_worker(level, seed, worker)
    }

    /// Create an AI player that runs lookahead synchronously on the calling
    /// thread (useful for tests and deterministic headless benchmarks).
    pub fn inline(level: Level, seed: u64) -> Fighter {
        Fighter::with_worker(level, seed, None)
    }

    /// Control whether the AI may press `L` to open the custom screen when the
    /// custom gauge fills (enabled by default; disabled in practice mode so
    /// only the player opens the custom screen).
    pub fn with_open_custom(mut self, open: bool) -> Fighter {
        self.open_custom = open;
        self
    }

    fn with_worker(level: Level, seed: u64, worker: Option<Worker>) -> Fighter {
        Fighter {
            level,
            cfg: level.config(),
            seed,
            open_custom: true,
            rng: SplitMix64::new(seed ^ 0xA1_F164_7E57),
            round: None,
            worker,
            pending: None,
        }
    }

    /// Difficulty level of this AI.
    pub fn level(&self) -> Level {
        self.level
    }

    fn start_round(&mut self, b: &Battle, key: (u64, SetScore)) -> RoundState {
        let ring_len = (self.cfg.delay / self.cfg.k) as usize + 3;
        self.pending = None;
        if let Some(w) = &self.worker {
            w.load(b.content.clone());
        }
        RoundState {
            key,
            t: 0,
            fight_ticks: 0,
            attack_log: AttackLog::default(),
            ring: (0..ring_len).map(|_| (None, None, AttackLog::default())).collect(),
            my_inputs: [Keys::empty(); INPUT_RING],
            held_first: Keys::empty(),
            held_after: Keys::empty(),
            held_first_ticks: 0,
            decision_age: 0,
            cur_panel: None,
            prev_panel: None,
            ticks_since_move: 100,
            strafe_timer: 0,
            enemy_last_panel: None,
            enemy_stationary_ticks: 100,
            reach: ReachTable::default(),
            in_screen: false,
            nav: Navigator::default(),
            planner: ScreenPlanner::default(),
            opener_at: None,
            last_cursor: 0,
            ticks_same_chip: 0,
        }
    }

    fn clear_fight_ring(r: &mut RoundState) {
        for slot in &mut r.ring {
            slot.0 = None;
            slot.1 = None;
        }
        r.my_inputs = [Keys::empty(); INPUT_RING];
        r.fight_ticks = 0;
        r.held_first = Keys::empty();
        r.held_after = Keys::empty();
        r.held_first_ticks = 0;
        r.decision_age = 0;
        r.opener_at = None;
        r.last_cursor = 0;
        r.ticks_same_chip = 0;
        r.enemy_last_panel = None;
        r.enemy_stationary_ticks = 100;
    }

    fn tick(&mut self, b: &Battle, side: u8, layer: Option<&Layer>) -> Keys {
        let key = (b.setup.content.0, b.setup.score);
        let new_round = self.round.as_ref().is_none_or(|r| r.key != key);
        if new_round {
            let fresh = self.start_round(b, key);
            self.round = Some(fresh);
        }
        let mut r = self.round.take().expect("round state");
        let keys = self.step_round(b, side, layer, &mut r);
        r.t = r.t.wrapping_add(1);
        self.round = Some(r);
        keys
    }

    fn step_round(&mut self, b: &Battle, side: u8, layer: Option<&Layer>, r: &mut RoundState) -> Keys {
        if r.t > 0 {
            r.attack_log.push(b);
        }

        let custom = b.round.mode == mode::CUSTOM;
        if custom && !r.in_screen {
            Self::clear_fight_ring(r);
            self.pending = None;
            r.nav.reset();
            r.planner.reset();
        }
        r.in_screen = custom;

        // ---- Custom Screen ----
        if custom {
            let Some(layer) = layer else {
                return Keys::empty();
            };
            if r.nav.idle() && b.custom.sides[side as usize].screen.as_ref().is_some_and(|s| nav::chooses(s, layer)) {
                if let Some(action) = r.planner.decide(b, side, layer, &mut r.nav, self.cfg, &mut self.rng) {
                    r.nav.point(action);
                }
            }
            return r.nav.buttons(b, side, layer);
        }

        // ---- Fight ----
        let runs = fight_runs(b) && b.player(side).is_some();
        if !runs {
            self.pending = None;
            return Keys::empty();
        }

        let ft = r.fight_ticks;
        let k = self.cfg.k;
        let d = self.cfg.delay;

        // Track our own panel (from our own side — always visible) for anti-pacing.
        if let Some(p) = navi_panel(b, side) {
            if let Some(cur) = r.cur_panel {
                if cur != p {
                    r.prev_panel = Some(cur);
                    r.cur_panel = Some(p);
                    r.ticks_since_move = 0;
                } else {
                    r.ticks_since_move = r.ticks_since_move.saturating_add(1);
                }
            } else {
                r.cur_panel = Some(p);
            }
        }

        // Collect a finished background worker decision due on this tick.
        if let Some(p) = self.pending.take_if(|p| p.at <= ft) {
            if p.at == ft {
                if let Some(w) = &self.worker {
                    match w.wait(p.job_id) {
                        Ok(dec) => apply_decision(r, dec),
                        Err(_) => {
                            // Worker failed; drop it and fall back to inline on subsequent decisions.
                            self.worker = None;
                        }
                    }
                }
            }
        }

        // Save snapshot into the reaction-delay ring whenever a future decision will read this tick.
        if (ft + d).is_multiple_of(k) {
            let slot_idx = (((ft + d) / k) as usize) % r.ring.len();
            r.ring[slot_idx] = (Some(ft), Some(Box::new(b.clone())), r.attack_log);
        }

        // Schedule or run a fight decision.
        let lead = if self.worker.is_some() { WORKER_LEAD.min(k.saturating_sub(1)).min(d) } else { 0 };
        let due = ft + lead;
        if due.is_multiple_of(k) && (lead == 0 || self.pending.is_none()) {
            let ctx = self.build_fight_context(b, side, r, ft, due, lead);
            if lead > 0 {
                if let Some(w) = &mut self.worker {
                    let job_id = w.ask(ctx);
                    self.pending = Some(Pending { at: due, job_id });
                }
            } else {
                let dec = fight::decide(ctx);
                apply_decision(r, dec);
            }
        }

        // Emit this tick's fight buttons.
        let mut out = if r.decision_age < r.held_first_ticks as u32 {
            r.held_first
        } else {
            r.held_after
        };

        // Track how long the same chip has sat unused at the front of our hand.
        let cur_cursor = b.hands[side as usize].cursor;
        if cur_cursor != r.last_cursor {
            r.last_cursor = cur_cursor;
            r.ticks_same_chip = 0;
        } else {
            r.ticks_same_chip = r.ticks_same_chip.saturating_add(1);
        }

        // Custom screen opener when gauge is full (never opens while dimmed, or when disabled e.g. practice mode).
        if self.open_custom && may_open(b) {
            let empty_hand = b.hands[side as usize].remaining() == 0;
            let stalled_hand = r.ticks_same_chip > 110;
            let low_hp = b.player(side).is_some_and(|p| {
                let o = b.objects.get(p);
                o.max_hp > 0 && (o.hp as u32) * 10 < (o.max_hp as u32) * 3
            });
            let urgent = empty_hand || stalled_hand || low_hp;
            let (lo, hi) = if urgent { self.cfg.opener_empty } else { self.cfg.opener_holding };
            let at = *r.opener_at.get_or_insert_with(|| {
                let span = hi.saturating_sub(lo).max(1) as u64;
                let u = (self.rng.next_u64() % span) as u32;
                ft + lo + u
            });
            // If hand just emptied or stalled while a longer wait was scheduled, pull opener closer.
            let at = if urgent && at > ft + self.cfg.opener_empty.1 {
                let shorter = ft + self.cfg.opener_empty.0;
                r.opener_at = Some(shorter);
                shorter
            } else {
                at
            };
            if presses_l(ft, at) {
                out.insert(Keys::L);
            }
        } else {
            r.opener_at = None;
        }

        // Record our actual fight keys (without L) so delayed lookahead replays our exact inputs.
        r.my_inputs[(ft as usize) % INPUT_RING] = out & !(Keys::L | Keys::R);
        r.decision_age = r.decision_age.saturating_add(1);
        r.fight_ticks = r.fight_ticks.wrapping_add(1);
        out
    }

    fn build_fight_context(&mut self, b: &Battle, side: u8, r: &RoundState, ft: u32, due: u32, lead: u32) -> FightContext {
        let k = self.cfg.k;
        let d = self.cfg.delay;
        let want = due.checked_sub(d);
        let slot_idx = ((due / k) as usize) % r.ring.len();

        let (snap_tick, snap, attack_log) = match (&r.ring[slot_idx], want) {
            ((Some(st), Some(saved), log), Some(w)) if *st == w => (*st, (**saved).clone(), *log),
            _ => (ft, b.clone(), r.attack_log),
        };

        // Reconstruct our exact past inputs from snap_tick..ft, plus predicted held keys for ft..due.
        let mut past_inputs = Vec::with_capacity((due.saturating_sub(snap_tick)) as usize);
        for t in snap_tick..ft {
            past_inputs.push(r.my_inputs[(t as usize) % INPUT_RING]);
        }
        for step in 0..lead {
            let age = r.decision_age + step;
            let keys = if age < r.held_first_ticks as u32 { r.held_first } else { r.held_after };
            past_inputs.push(keys);
        }

        FightContext {
            snap,
            side,
            past_inputs,
            luck: self.rng.next_u64(),
            cfg: self.cfg,
            attack_log,
            prev_panel: r.prev_panel,
            ticks_since_move: r.ticks_since_move,
            strafe_timer: r.strafe_timer,
            enemy_last_panel: r.enemy_last_panel,
            enemy_stationary_ticks: r.enemy_stationary_ticks,
            reach: r.reach.clone(),
        }
    }
}

fn apply_decision(r: &mut RoundState, dec: FightDecision) {
    r.held_first = dec.first;
    r.held_after = dec.after;
    r.held_first_ticks = dec.first_ticks;
    r.decision_age = 0;
    r.reach = dec.reach;
    r.strafe_timer = dec.strafe_timer;
    r.enemy_last_panel = dec.enemy_last_panel;
    r.enemy_stationary_ticks = dec.enemy_stationary_ticks;
}

impl Ai for Fighter {
    fn fight(&mut self, b: &Battle, side: u8, _theirs: Keys) -> Keys {
        self.tick(b, side, None)
    }

    fn custom_screen(&mut self, b: &Battle, side: u8, layer: &Layer) -> Keys {
        self.tick(b, side, Some(layer))
    }

    fn reset(&mut self) {
        self.round = None;
        self.pending = None;
        self.rng = SplitMix64::new(self.seed ^ 0xA1_F164_7E57);
    }
}

#[cfg(test)]
mod tests;
