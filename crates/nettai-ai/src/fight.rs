//! Tactical fight AI: delayed fair lookahead, opponent movement modeling &
//! punish timing, generic A-charge & B-charge discipline, B+Back specials,
//! dimming cut-ins, persistent relative `(dx, dy)` chip-reach learning,
//! 2-phase `Step → Fire` lookahead, Full Synchro awareness, and area/obstacle PvP tactics.

use crate::sim::{
    a_chargeable, dir_key_for_delta, enemy_seen_panel, fight_runs, navi_barrier_hp, navi_hp, navi_idle, navi_max_hp, navi_panel, navi_ref,
    navi_status, owned_panel_count, sanitize_battle, seen_to, step_battle, stepped_pos, walkable_panel,
};
use nettai_battle::actor::ActorStatus;
use nettai_battle::collision::StatusFlags;
use nettai_battle::content::{ChipClass, ChipFlags};
use nettai_battle::dimming::DimmingState;
use nettai_battle::field::PanelFlags;
use nettai_battle::input::Keys;
use nettai_battle::kinds::player::next_chip_doubles;
use nettai_battle::Battle;
use nettai_content_api::{ChipHandle, PanelPos};
use nettai_netplay::rng::SplitMix64;

/// Number of recent ticks tracked in [`AttackLog`].
const LOG_TICKS: usize = 8;

/// Recent per-tick `Battle::attacked_panels` masks (newest first).
#[derive(Clone, Copy, Debug, Default)]
pub struct AttackLog {
    pub ticks: [[u64; 2]; LOG_TICKS],
}

impl AttackLog {
    pub fn push(&mut self, b: &Battle) {
        self.ticks.copy_within(0..LOG_TICKS - 1, 1);
        self.ticks[0] = [b.attacked_panels(0), b.attacked_panels(1)];
    }

    /// Whether side `attacker` presented an attack hitbox on `(x, y)` within the last `LOG_TICKS` ticks.
    pub fn recent_attack_on(&self, attacker: u8, x: u8, y: u8) -> bool {
        let bit = 1u64 << (y as usize * 8 + x as usize);
        self.ticks.iter().any(|row| row[attacker as usize & 1] & bit != 0)
    }
}

/// Difficulty parameters controlling reaction time, lookahead depth, and decision quality.
#[derive(Clone, Copy, Debug)]
pub struct LevelConfig {
    /// Reaction delay in ticks (`t - delay`).
    pub delay: u32,
    /// Ticks between fight decisions.
    pub k: u32,
    /// Undimmed ticks simulated per candidate action.
    pub horizon: u32,
    /// Hard total tick cap per rollout (allows dimming sequences to finish).
    pub cap: u32,
    /// Whether to dynamically learn relative chip hit-offsets and model opponent evasion.
    pub probe_reach: bool,
    /// Probability of actively dodging an incoming attack.
    pub dodge_chance: f32,
    /// Probability of evaluating a dimming cut-in when dimmed.
    pub cut_in_chance: f32,
    /// Whether to evaluate the B+Back special (Shield / Reflect / AntiDmg / etc.).
    pub use_back_special: bool,
    /// Random tie-breaking / decision noise (0.0 for Hard).
    pub noise: f32,
    /// Max chips picked per custom screen.
    pub max_screen_picks: usize,
    /// Probability of taking a favorable Cross / Soul / Beast Out.
    pub form_chance: f32,
    /// Whether to use re-deal buttons (e.g. ChpShufl) on weak hands.
    pub use_redeal: bool,
    /// Opener wait range `(min, max)` ticks when hand is empty / stalled / low HP.
    pub opener_empty: (u32, u32),
    /// Opener wait range `(min, max)` ticks when still holding chips.
    pub opener_holding: (u32, u32),
}

#[derive(Clone, Debug)]
struct ReachEntry {
    chip: ChipHandle,
    offsets: Vec<(i8, i8)>,
    probed_vuln: bool,
    probed_invuln: bool,
    hits_invuln: bool,
}

/// Per-round learned relative `(dx, dy)` hit offsets for each chip seen so far, plus
/// whether a chip was observed hitting through invulnerability/Invis (e.g. GunDelSol, JudgeMan),
/// and learned relative `(dx, dy)` hit offsets for the active form's B-charge shot (handling
/// short-range melee forms like ProtoSoul/TomahawkCross AND fixed-range lobbed forms like MeddySoul!).
#[derive(Clone, Debug, Default)]
pub struct ReachTable {
    entries: Vec<ReachEntry>,
    charge_form: Option<nettai_content_api::FormHandle>,
    charge_offsets: Vec<(i8, i8)>,
    charge_probed: bool,
}

impl ReachTable {
    pub fn charge_probed_for(&self, form: nettai_content_api::FormHandle) -> bool {
        self.charge_form == Some(form) && self.charge_probed
    }

    pub fn record_charge_hit(&mut self, form: nettai_content_api::FormHandle, offset: (i8, i8)) {
        if self.charge_form != Some(form) {
            self.charge_form = Some(form);
            self.charge_offsets.clear();
            self.charge_probed = false;
        }
        if !self.charge_offsets.contains(&offset) && self.charge_offsets.len() < 12 {
            self.charge_offsets.push(offset);
        }
    }

    pub fn set_charge_probed(&mut self, form: nettai_content_api::FormHandle, offsets: Vec<(i8, i8)>) {
        if self.charge_form != Some(form) {
            self.charge_form = Some(form);
            self.charge_offsets.clear();
        }
        for off in offsets {
            if !self.charge_offsets.contains(&off) && self.charge_offsets.len() < 12 {
                self.charge_offsets.push(off);
            }
        }
        self.charge_probed = true;
    }

    /// Closest reachable walkable own-side panel from `my_pos` whose relative offset to `enemy_pos`
    /// is a known hit for `form`'s B-charge shot (routes around holes via BFS!).
    pub fn best_charge_target(
        &self,
        b: &Battle,
        side: u8,
        form: nettai_content_api::FormHandle,
        my_pos: PanelPos,
        enemy_pos: PanelPos,
        air_shoe: bool,
    ) -> Option<PanelPos> {
        if self.charge_form != Some(form) || self.charge_offsets.is_empty() {
            return None;
        }
        let forward: i32 = if side == 0 { 1 } else { -1 };
        let mut best: Option<(PanelPos, i32)> = None;
        for &(dx, dy) in &self.charge_offsets {
            let tx = enemy_pos.x as i32 - (dx as i32) * forward;
            let ty = enemy_pos.y as i32 - dy as i32;
            if !(1..=6).contains(&tx) || !(1..=3).contains(&ty) {
                continue;
            }
            let (tx, ty) = (tx as u8, ty as u8);
            if !walkable_panel(b, side, tx, ty, air_shoe) {
                continue;
            }
            let p = PanelPos { x: tx, y: ty };
            if let Some(d) = grid_dist(b, side, my_pos, p, air_shoe)
                && best.is_none_or(|(_, bd)| d < bd)
            {
                best = Some((p, d));
            }
        }
        best.map(|(p, _)| p)
    }

    pub fn already_probed(&self, chip: ChipHandle, while_invuln: bool) -> bool {
        self.entries
            .iter()
            .find(|e| e.chip == chip)
            .is_some_and(|e| if while_invuln { e.probed_invuln } else { e.probed_vuln })
    }

    pub fn record(&mut self, chip: ChipHandle, offset: (i8, i8), while_invuln: bool) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.chip == chip) {
            if !e.offsets.contains(&offset) && e.offsets.len() < 12 {
                e.offsets.push(offset);
            }
            if while_invuln {
                e.hits_invuln = true;
            }
        } else if self.entries.len() < 40 {
            self.entries.push(ReachEntry {
                chip,
                offsets: vec![offset],
                probed_vuln: false,
                probed_invuln: false,
                hits_invuln: while_invuln,
            });
        }
    }

    pub fn set_probed(&mut self, chip: ChipHandle, offsets: Vec<(i8, i8)>, while_invuln: bool) {
        let hit_any = !offsets.is_empty();
        if let Some(e) = self.entries.iter_mut().find(|e| e.chip == chip) {
            for off in offsets {
                if !e.offsets.contains(&off) && e.offsets.len() < 12 {
                    e.offsets.push(off);
                }
            }
            if while_invuln {
                e.probed_invuln = true;
                if hit_any {
                    e.hits_invuln = true;
                }
            } else {
                e.probed_vuln = true;
            }
        } else if self.entries.len() < 40 {
            self.entries.push(ReachEntry {
                chip,
                offsets,
                probed_vuln: !while_invuln,
                probed_invuln: while_invuln,
                hits_invuln: while_invuln && hit_any,
            });
        }
    }

    /// Closest walkable own-side panel from `my_pos` whose relative offset to `enemy_pos` is a known hit for `chip`.
    pub fn best_target(
        &self,
        b: &Battle,
        side: u8,
        chip: ChipHandle,
        my_pos: PanelPos,
        enemy_pos: PanelPos,
        air_shoe: bool,
        enemy_invuln: bool,
    ) -> Option<PanelPos> {
        let entry = self.entries.iter().find(|e| e.chip == chip)?;
        if enemy_invuln && !entry.hits_invuln {
            return None;
        }
        let forward: i32 = if side == 0 { 1 } else { -1 };
        let mut best: Option<(PanelPos, i32)> = None;
        for &(dx, dy) in &entry.offsets {
            let tx = enemy_pos.x as i32 - (dx as i32) * forward;
            let ty = enemy_pos.y as i32 - dy as i32;
            if !(1..=6).contains(&tx) || !(1..=3).contains(&ty) {
                continue;
            }
            let (tx, ty) = (tx as u8, ty as u8);
            if !walkable_panel(b, side, tx, ty, air_shoe) {
                continue;
            }
            let p = PanelPos { x: tx, y: ty };
            let dist = manhattan(my_pos, p);
            if best.is_none_or(|(_, bd)| dist < bd) {
                best = Some((p, dist));
            }
        }
        best.map(|(p, _)| p)
    }
}

/// Everything needed to compute one fight decision (pure & Send so it can run on a worker thread).
pub struct FightContext {
    pub snap: Battle,
    pub side: u8,
    pub past_inputs: Vec<Keys>,
    pub luck: u64,
    pub cfg: LevelConfig,
    pub attack_log: AttackLog,
    pub prev_panel: Option<PanelPos>,
    pub ticks_since_move: u32,
    pub strafe_timer: u32,
    pub enemy_last_panel: Option<PanelPos>,
    pub enemy_stationary_ticks: u32,
    pub reach: ReachTable,
}

/// Output of one fight decision: press `first` for `first_ticks` ticks, then hold `after`.
#[derive(Clone, Debug)]
pub struct FightDecision {
    pub first: Keys,
    pub after: Keys,
    pub first_ticks: u8,
    pub reach: ReachTable,
    pub strafe_timer: u32,
    pub enemy_last_panel: Option<PanelPos>,
    pub enemy_stationary_ticks: u32,
}

#[derive(Clone, Copy, Debug)]
struct Candidate {
    first: Keys,
    first_ticks: u8,
    after: Keys,
    is_move: bool,
    is_charge_release: bool,
    is_a_charge_hold: bool,
    is_step_fire: bool,
}

#[derive(Clone, Debug, Default)]
struct RolloutResult {
    score: f32,
    dealt: f32,
    taken: f32,
    obs_dmg: u32,
    spent_chip: bool,
    gained_invis: bool,
    gained_barrier: u16,
    gained_trap: bool,
    ended_dimmed: bool,
    end_a_charging: bool,
    end_panel: Option<PanelPos>,
    end_enemy_panel: Option<PanelPos>,
    end_gauge_rate: u16,
    end_field_objs: u8,
    end_friendly_objs: u8,
    end_rules: Option<nettai_content_api::Block>,
}

/// Compute a fight decision purely from the delayed snapshot + our own recorded inputs.
pub fn decide(mut ctx: FightContext) -> FightDecision {
    let mut rng = SplitMix64::new(ctx.luck ^ 0xD1CE_5EED);
    let side = ctx.side;
    let other = side ^ 1;
    let cfg = ctx.cfg;

    // Update delayed opponent movement tracking purely from the delayed snapshot (100% fair!).
    let snap_enemy = if ctx.snap.sees(side, other) { enemy_seen_panel(&ctx.snap, side) } else { None };
    let (next_enemy_panel, next_enemy_stat) = match (snap_enemy, ctx.enemy_last_panel) {
        (Some(cur), Some(prev)) if cur == prev => (Some(cur), ctx.enemy_stationary_ticks.saturating_add(cfg.k)),
        (Some(cur), _) => (Some(cur), 0),
        (None, prev) => (prev, ctx.enemy_stationary_ticks.saturating_add(cfg.k)),
    };

    // 1. Sanitize delayed snapshot and fast-forward our own past inputs (strictly causal!).
    let mut sim = sanitize_battle(&ctx.snap, side, rng.next_u64());
    for &held in &ctx.past_inputs {
        let mut both = [Keys::empty(); 2];
        both[side as usize] = held;
        if !step_battle(&mut sim, both) || !fight_runs(&sim) {
            break;
        }
    }

    let Some(my_ref) = navi_ref(&sim, side) else {
        return idle_decision(ctx.reach, ctx.strafe_timer, next_enemy_panel, next_enemy_stat);
    };
    if !fight_runs(&sim) {
        return idle_decision(ctx.reach, ctx.strafe_timer, next_enemy_panel, next_enemy_stat);
    }

    let my_actor = sim.objects.get(my_ref).actor.map(|a| sim.actors.get(a));
    let can_b_charge = my_actor.is_some_and(|a| !a.status.contains(ActorStatus::NO_CHARGE) && (a.charge_shot.is_some() || a.b_charge_time.is_some()));
    let can_a_charge = a_chargeable(&sim, side);
    let a_charging = can_a_charge && my_actor.is_some_and(|a| a.charge_source == 1 && a.charge_level >= 1);
    let a_charge_full = can_a_charge && my_actor.is_some_and(|a| a.charge_source == 1 && a.charge_level >= 2);
    let b_charge_full = can_b_charge && my_actor.is_some_and(|a| a.charge_source == 2 && a.charge_level >= 2);
    let keep_b = if can_b_charge { Keys::B } else { Keys::empty() };
    // While A-charging a chip, maintain `Keys::A` through movement/dodges so we don't cancel the charge early!
    let keep_hold = if a_charging { Keys::A } else { keep_b };
    let has_back_special = my_actor.is_some_and(|a| a.back_special.is_some() && a.back_special_cooldown == 0);
    let next_chip = sim.hands[side as usize].next_chip();
    let is_doubled = next_chip.is_some() && next_chip_doubles(&sim, my_ref);
    let my_pos = sim.objects.get(my_ref).panel;
    let my_status = navi_status(&sim, side);
    let air_shoe = my_status.contains(StatusFlags::AIRSHOE);
    let float_shoe = my_status.contains(StatusFlags::FLOATSHOE);
    let confused = my_status.contains(StatusFlags::CONFUSED);
    let blind = !sim.sees(side, other);
    let enemy_pos = if blind { None } else { enemy_seen_panel(&sim, side) };
    let enemy_status = if enemy_pos.is_some() { navi_status(&sim, other) } else { StatusFlags::empty() };
    let enemy_invuln = enemy_status
        .intersects(StatusFlags::INVULNERABLE | StatusFlags::UNTOUCHABLE | StatusFlags::FLASHING | StatusFlags::INVISIBLE | StatusFlags::SUBMERGED);
    let enemy_attacking = enemy_pos.is_some()
        && navi_ref(&sim, other)
            .and_then(|r| sim.objects.get(r).actor)
            .is_some_and(|a| sim.actors.get(a).navi_action.is_attack());
    let enemy_locked = enemy_attacking
        || enemy_status.intersects(
            StatusFlags::PARALYZED
                | StatusFlags::FROZEN
                | StatusFlags::BUBBLED
                | StatusFlags::IMMOBILIZED
                | StatusFlags::FLINCHING
                | StatusFlags::DRAG
                | StatusFlags::SLIDING,
        );
    let enemy_moving_actively = next_enemy_stat < 28;
    let forward: i32 = if side == 0 { 1 } else { -1 };

    // 2. Dimming state:
    //    (a) If our own interactive dimming chip is reading dimmed keys (EraseMan, Magnum, ElementMan,
    //        BurnSquare, SearchMan, DeltaRayEdge, CircleGun, CrossScope, MetalMan, WoodMan, FireMan,
    //        LaserMan, ShadeMan, etc.), aim / trigger it generically using `Battle::dimmed_key_reads`!
    //    (b) Otherwise, check if we can & should counter cut-in with a dimming chip!
    if sim.is_dimmed() {
        let held_a_before = ctx.past_inputs.last().is_some_and(|k| k.contains(Keys::A));
        if let Some((first, first_ticks, after)) = decide_interactive_dimming(&sim, side, keep_hold, held_a_before, blind, cfg, &mut rng) {
            return FightDecision {
                first,
                after,
                first_ticks,
                reach: ctx.reach,
                strafe_timer: ctx.strafe_timer,
                enemy_last_panel: next_enemy_panel,
                enemy_stationary_ticks: next_enemy_stat,
            };
        }

        let other_dim = sim.dimming[other as usize];
        let my_dim = sim.dimming[side as usize];
        let can_cut = matches!(my_dim.state, DimmingState::Idle | DimmingState::Waiting)
            && !other_dim.no_cut_in
            && other_dim.state == DimmingState::ShowingName;
        if can_cut
            && let Some(chip_id) = next_chip
            && sim.content.chip(chip_id).flags.contains(ChipFlags::DIMMING)
            && rand_unit(&mut rng) < cfg.cut_in_chance
        {
            // Simulate long enough for BOTH our cut-in (~120 ticks) AND the opponent's dimming chip
            // (including slow/interactive ones like CircGun/EraseMan up to 360+ ticks) to finish!
            // Never credit `cut.taken < base.taken` if `cut.ended_dimmed` is still true (that just means
            // our extra dimming pushed the opponent's hit past `cut_cap`, not that we blocked it — e.g.
            // cutting in with a trap on CircGun, which removes traps and still hits!).
            let cut_cap = cfg.cap.max(760);
            let base = rollout(&sim, side, keep_b, 1, keep_b, 20, cut_cap, blind, Keys::empty(), false);
            let cut = rollout(&sim, side, Keys::A, 2, keep_b, 24, cut_cap, blind, Keys::empty(), true);
            let prevented_dmg = !cut.ended_dimmed && cut.taken + 0.005 < base.taken;
            let dealt_more = cut.dealt > base.dealt + 0.005;
            if cut.spent_chip && (prevented_dmg || dealt_more) && cut.score > base.score + 0.025 {
                return FightDecision {
                    first: Keys::A,
                    after: keep_b,
                    first_ticks: 2,
                    reach: ctx.reach,
                    strafe_timer: ctx.strafe_timer,
                    enemy_last_panel: next_enemy_panel,
                    enemy_stationary_ticks: next_enemy_stat,
                };
            }
        }
        return FightDecision {
            first: keep_hold,
            after: keep_hold,
            first_ticks: cfg.k as u8,
            reach: ctx.reach,
            strafe_timer: ctx.strafe_timer,
            enemy_last_panel: next_enemy_panel,
            enemy_stationary_ticks: next_enemy_stat,
        };
    }

    // 3. Evaluate baseline (staying put while maintaining current B or A charge).
    let base_cand = Candidate {
        first: keep_hold,
        first_ticks: cfg.k as u8,
        after: keep_hold,
        is_move: false,
        is_charge_release: false,
        is_a_charge_hold: a_charging,
        is_step_fire: false,
    };
    let base_res = rollout(&sim, side, base_cand.first, base_cand.first_ticks, base_cand.after, cfg.horizon, cfg.cap, blind, Keys::empty(), false);
    let under_threat = base_res.taken > 0.001;
    let should_dodge = !under_threat || rand_unit(&mut rng) < cfg.dodge_chance;

    // 4. Evaluate chip use (`A` press, or `A` release when A-charging!).
    let mut chip_res: Option<(Candidate, RolloutResult)> = None;
    let mut a_charge_start: Option<(Candidate, RolloutResult)> = None;

    if let Some(chip_id) = next_chip {
        let cd = sim.content.chip(chip_id);
        let can_counter = cd.hit_param & 0x80 == 0;
        let is_dimming_chip = cd.flags.contains(ChipFlags::DIMMING);

        // Fire the chip: release A if currently A-charging, otherwise press A (held 2 ticks then released to `keep_b`).
        let (fire_first, fire_ticks, fire_after) = if a_charging {
            (Keys::empty(), 2, keep_b)
        } else {
            (Keys::A | keep_b, 2, keep_b)
        };
        let fire_cand = Candidate {
            first: fire_first,
            first_ticks: fire_ticks,
            after: fire_after,
            is_move: false,
            is_charge_release: false,
            is_a_charge_hold: false,
            is_step_fire: false,
        };
        let mut r = rollout(
            &sim,
            side,
            fire_cand.first,
            fire_cand.first_ticks,
            fire_cand.after,
            cfg.horizon,
            cfg.cap,
            blind,
            Keys::empty(),
            is_dimming_chip,
        );

        // Learn relative hit offset whenever THIS chip lands additional damage (never credit already-in-flight hits!).
        let chip_added_dmg = r.dealt > base_res.dealt + 0.001;
        if chip_added_dmg
            && let Some(ep) = enemy_pos
        {
            let dx = ((ep.x as i32 - my_pos.x as i32) * forward) as i8;
            let dy = (ep.y as i32 - my_pos.y as i32) as i8;
            ctx.reach.record(chip_id, (dx, dy), enemy_invuln);
        }

        // Opponent movement modeling: if the opponent is actively moving and NOT locked in an attack/status,
        // verify whether a non-dimming chip still lands when the opponent steps off the row on tick 10!
        if cfg.probe_reach
            && !blind
            && chip_added_dmg
            && !enemy_locked
            && enemy_moving_actively
            && !is_dimming_chip
            && let Some(ep) = enemy_pos
        {
            if let Some(evade_dir) = enemy_evade_dir(&sim, other, ep) {
                let ev = rollout(&sim, side, fire_cand.first, fire_cand.first_ticks, fire_cand.after, cfg.horizon, cfg.cap, blind, evade_dir, false);
                let blended_dealt = 0.45 * r.dealt + 0.55 * ev.dealt;
                // Adjust score by the difference between stationary and evasive expected damage.
                let delta = blended_dealt - r.dealt;
                r.score += delta * 1.45;
                // Extra caution against whiffing Full Synchro (2x) on a slow dodgeable chip!
                if is_doubled && ev.dealt <= base_res.dealt + 0.001 {
                    r.score -= 0.06;
                }
            }
        }

        // Counter-hit hunting & Full Synchro burst bonuses!
        if chip_added_dmg {
            if enemy_attacking && can_counter {
                r.score += 0.075;
            }
            if is_doubled && (r.dealt - base_res.dealt) >= 0.08 {
                r.score += 0.055;
            }
        }

        // Defensive chip discipline:
        // - NEVER stack redundant defenses (`Invisible -> Invisible`, `Invisible -> Barrier`, `Barrier -> Barrier`, etc.).
        // - Save timed/reactive defenses (`Invisible`, `Invuln`, `Barrier`/`Aura`) for incoming threats
        //   (`r.taken < base_res.taken`, which is already naturally rewarded by `score_rollout`!),
        //   dimming cut-ins, or critically low HP under active pressure — NOT unprovoked when safe!
        let my_hp_frac = navi_hp(&sim, side) as f32 / navi_max_hp(&sim, side) as f32;
        let my_barrier_now = navi_barrier_hp(&sim, side);
        let my_trap_now = my_actor.is_some_and(|a| a.status.intersects(ActorStatus::TRAP_ARMED | ActorStatus::HEAT_TRAP));
        let already_invis = my_status.intersects(StatusFlags::INVISIBLE | StatusFlags::INVULNERABLE);
        let already_protected = already_invis || my_barrier_now > 0 || my_trap_now;

        if r.spent_chip && r.dealt == 0.0 {
            if r.gained_invis {
                // Only proactively pop Invis without immediate damage if NOT already protected AND critically low under attack;
                // otherwise save it in hand for incoming hits or counter cut-ins!
                if !already_protected && my_hp_frac < 0.35 && enemy_attacking {
                    r.score += 0.038;
                }
            } else if r.gained_barrier > 0 {
                // Never pop Barrier/Aura while Invisible or overwriting an existing barrier (unless a huge Aura upgrade under pressure).
                let big_upgrade = my_barrier_now == 0 || (r.gained_barrier >= 100 && (under_threat || enemy_attacking));
                if !already_invis && big_upgrade && (under_threat || enemy_attacking || (my_barrier_now == 0 && my_hp_frac < 0.55)) {
                    let gain = (r.gained_barrier as f32 / navi_max_hp(&sim, side) as f32).min(0.25);
                    r.score += 0.032 + gain * 0.18;
                }
            } else if r.gained_trap {
                // Persistent traps (AntiDmg, AntiNavi, etc.) are good to arm when not already holding Invis/Trap.
                if !already_invis && !my_trap_now {
                    r.score += 0.042;
                }
            } else {
                // Credit persistent non-defensive setup chips (FastGauge/SlowGauge, RockCubes, actor summons, rules traps).
                // Never credit lingering attack projectiles or non-trap Navi/attack rules writes when an attack whiffed!
                let is_attack_chip = cd.flags.contains(ChipFlags::HAS_DAMAGE) || cd.damage > 0 || cd.formula.is_some();
                if r.end_gauge_rate != base_res.end_gauge_rate {
                    r.score += 0.065;
                }
                if r.end_field_objs > base_res.end_field_objs {
                    r.score += 0.055;
                }
                if !cd.flags.contains(ChipFlags::HAS_DAMAGE) && r.end_friendly_objs > base_res.end_friendly_objs {
                    r.score += 0.055;
                }
                if (cd.trap.is_some() || !is_attack_chip) && r.end_rules != base_res.end_rules && !already_invis && !my_trap_now {
                    r.score += 0.045;
                }
            }
        }

        // Strongly penalize whiffed chips (no added damage, no net value over doing nothing) so Easy/Normal
        // decision noise never accidentally fires an attack chip into empty air or an Invisible/Invulnerable player!
        if r.spent_chip && !chip_added_dmg && r.score <= base_res.score + 0.001 {
            r.score -= 0.065;
        }

        // If we are A-charging and NOT full yet, slightly discourage releasing prematurely unless
        // it's lethal or countering an attack right now!
        if a_charging && !a_charge_full && r.score < 2.0 && !enemy_attacking {
            r.score -= 0.045;
        }

        chip_res = Some((fire_cand, r));

        // Consider starting an A-charge when `can_a_charge` is true, we aren't charging A yet, and opponent can be hit!
        if can_a_charge && !a_charging && !enemy_invuln && cfg.probe_reach && !under_threat {
            let is_attack_chip = cd.flags.contains(ChipFlags::HAS_DAMAGE) || cd.damage > 0 || cd.formula.is_some();
            if is_attack_chip {
                let c = Candidate {
                    first: Keys::A,
                    first_ticks: cfg.k as u8,
                    after: Keys::A,
                    is_move: false,
                    is_charge_release: false,
                    is_a_charge_hold: true,
                    is_step_fire: false,
                };
                let mut ar = rollout(&sim, side, c.first, c.first_ticks, c.after, cfg.horizon.min(28), cfg.cap, blind, Keys::empty(), false);
                // Only reward starting an A-charge if holding `A` actually entered A-charge state!
                if ar.end_a_charging && ar.taken <= base_res.taken + 0.001 {
                    let form = sim.form(side as usize);
                    let charge_worth = if form.charge_doubles.is_some() || form.fire_charge.is_some() { 0.024 } else { 0.016 };
                    ar.score += charge_worth;
                    a_charge_start = Some((c, ar));
                }
            }
        }
    }

    // 5. Persistent relative `(dx, dy)` chip-reach lookup & on-demand probing!
    //    Includes `damage == 0` non-dimming action chips (e.g. GunDelSol drain!) AND probes even when
    //    `enemy_invuln` is true so Invis-piercing chips (GunDelSol, JudgeMan) are still discovered and
    //    aimed right through Invisible (without blocking later re-probing of normal chips once Invis ends).
    if cfg.probe_reach
        && !blind
        && let (Some(chip_id), Some(epos)) = (next_chip, enemy_pos)
    {
        let cd = sim.content.chip(chip_id);
        let may_hit_enemy = cd.flags.contains(ChipFlags::HAS_DAMAGE)
            || cd.damage > 0
            || cd.formula.is_some()
            || (!cd.flags.contains(ChipFlags::DIMMING) && cd.modifier.is_none() && cd.trap.is_none());
        let chip_hit_now = chip_res.as_ref().is_some_and(|(_, r)| r.dealt > base_res.dealt + 0.001);
        if may_hit_enemy && !chip_hit_now && !ctx.reach.already_probed(chip_id, enemy_invuln) {
            let offsets = probe_chip_offsets(&sim, side, my_pos, epos, air_shoe);
            ctx.reach.set_probed(chip_id, offsets, enemy_invuln);
        }
    }

    let chip_target = if !blind {
        next_chip
            .zip(enemy_pos)
            .and_then(|(chip_id, epos)| ctx.reach.best_target(&sim, side, chip_id, my_pos, epos, air_shoe, enemy_invuln))
    } else {
        None
    };

    // 6. Build the rest of our candidate actions.
    let base_taken = base_res.taken;
    let base_dealt = base_res.dealt;
    let mut cands: Vec<(Candidate, RolloutResult)> = Vec::with_capacity(14);
    cands.push((base_cand, base_res));
    if let Some(cr) = chip_res {
        cands.push(cr);
    }
    if let Some(ac) = a_charge_start {
        cands.push(ac);
    }

    // Release full B charge shot, or finish off / poke with an uncharged buster shot!
    let cur_form = sim.stats[side as usize].form;
    let enemy_low_hp = !blind && navi_hp(&sim, other) <= 30;
    if can_b_charge && !a_charging && (b_charge_full || (enemy_low_hp && !enemy_invuln && enemy_pos.is_some_and(|ep| ep.y == my_pos.y))) {
        let c = Candidate {
            first: Keys::empty(),
            first_ticks: 2,
            after: Keys::B,
            is_move: false,
            is_charge_release: b_charge_full,
            is_a_charge_hold: false,
            is_step_fire: false,
        };
        let mut r = rollout(&sim, side, c.first, c.first_ticks, c.after, cfg.horizon, cfg.cap, blind, Keys::empty(), false);
        if b_charge_full
            && !enemy_invuln
            && let Some(ep) = enemy_pos
        {
            let dx = ((ep.x as i32 - my_pos.x as i32) * forward) as i8;
            let dy = (ep.y as i32 - my_pos.y as i32) as i8;
            if r.dealt > base_dealt + 0.001 {
                ctx.reach.record_charge_hit(cur_form, (dx, dy));
            } else if r.obs_dmg == 0 && cfg.probe_reach && !ctx.reach.charge_probed_for(cur_form) {
                let offsets = probe_b_charge_offsets(&sim, side, my_pos, ep, air_shoe);
                ctx.reach.set_charge_probed(cur_form, offsets);
            }
        }
        if !b_charge_full && r.dealt <= base_dealt + 0.001 && r.obs_dmg == 0 {
            r.score -= 0.06;
        }
        cands.push((c, r));
    } else if !can_b_charge && !a_charging && !enemy_invuln && enemy_pos.is_some_and(|ep| ep.y == my_pos.y) {
        // Uncharged buster poke for navis/forms that cannot charge B (only when opponent can be hit!).
        let c = Candidate {
            first: Keys::B,
            first_ticks: 2,
            after: Keys::empty(),
            is_move: false,
            is_charge_release: false,
            is_a_charge_hold: false,
            is_step_fire: false,
        };
        let mut r = rollout(&sim, side, c.first, c.first_ticks, c.after, cfg.horizon.min(24), cfg.cap, blind, Keys::empty(), false);
        if r.dealt == 0.0 && r.obs_dmg == 0 {
            r.score -= 0.06;
        }
        cands.push((c, r));
    } else if can_b_charge
        && !a_charging
        && !b_charge_full
        && !enemy_invuln
        && let Some(ep) = enemy_pos
        && ep.y == my_pos.y
        && ctx.reach.charge_probed_for(cur_form)
        && ctx.reach.best_charge_target(&sim, side, cur_form, my_pos, ep, air_shoe).is_none()
    {
        // Only tap uncharged B when we already know this form's B-charge shot can't reach from here!
        let c = Candidate {
            first: Keys::empty(),
            first_ticks: 1,
            after: Keys::B,
            is_move: false,
            is_charge_release: false,
            is_a_charge_hold: false,
            is_step_fire: false,
        };
        let mut r = rollout(&sim, side, c.first, c.first_ticks, c.after, cfg.horizon.min(24), cfg.cap, blind, Keys::empty(), false);
        if r.dealt == 0.0 && r.obs_dmg == 0 {
            r.score -= 0.06;
        }
        cands.push((c, r));
    }

    // B + Back special (Shield / Reflect / AntiDmg / Vacuum / etc.) when threatened or enemy attacking.
    if cfg.use_back_special && !a_charging && has_back_special && (under_threat || enemy_attacking) {
        let c = Candidate {
            first: Keys::B | Keys::LEFT,
            first_ticks: 2,
            after: keep_b,
            is_move: false,
            is_charge_release: false,
            is_a_charge_hold: false,
            is_step_fire: false,
        };
        let r = rollout(&sim, side, c.first, c.first_ticks, c.after, cfg.horizon, cfg.cap, blind, Keys::empty(), false);
        cands.push((c, r));
    }

    // Cardinal movement steps (+ 2-phase Step→Fire lookahead when 1 step from a chip hit panel,
    // or 1 step from an elemental-weakness position for an interactive dimming chip like ElementMan!).
    let step_ticks = cfg.k.min(3) as u8;
    let chip_missed_here = cands.get(1).is_some_and(|(_, r)| next_chip.is_some() && r.dealt == 0.0);
    let next_is_dimming = next_chip.is_some_and(|id| sim.content.chip(id).flags.contains(ChipFlags::DIMMING));
    let enemy_has_elem = enemy_pos.is_some()
        && navi_ref(&sim, other).is_some_and(|r| (sim.objects.get(r).element & 0x0F) != 0);
    if should_dodge || !under_threat {
        let dirs = [Keys::UP, Keys::DOWN, Keys::RIGHT, Keys::LEFT];
        for dir in dirs {
            let dest = stepped_pos(side, my_pos, dir);
            let valid = if confused {
                true
            } else {
                dest.is_some_and(|(nx, ny)| walkable_panel(&sim, side, nx, ny, air_shoe))
            };
            if !valid {
                continue;
            }
            let c = Candidate {
                first: dir | keep_hold,
                first_ticks: step_ticks,
                after: keep_hold,
                is_move: true,
                is_charge_release: false,
                is_a_charge_hold: a_charging,
                is_step_fire: false,
            };
            let h = if !under_threat && !cfg.probe_reach { cfg.horizon.min(18) } else { cfg.horizon };
            let r = rollout(&sim, side, c.first, c.first_ticks, c.after, h, cfg.cap, blind, Keys::empty(), false);
            cands.push((c, r));

            // 2-phase `Step → Fire` lookahead:
            // (a) If stepping `dir` lands on our learned `chip_target` (or onto the enemy's row / dx=2 while
            //     our chip missed here), evaluate stepping AND immediately firing `A` on the first idle frame.
            //     Runs even when `enemy_invuln` so Invis-piercing chips (GunDelSol, JudgeMan) step in & punish!
            // (b) If holding a dimming chip against an elemental opponent, also test stepping into key
            //     alignment (`dy == 0`, `dx == 3`, `dx == 1`) to unlock 2x elemental attacks (e.g. ElementMan)!
            if cfg.probe_reach
                && !blind
                && (!a_charging || a_charge_full)
                && let Some((nx, ny)) = dest
            {
                let dp = PanelPos { x: nx, y: ny };
                let promising_miss = chip_missed_here
                    && (chip_target == Some(dp)
                        || (chip_target.is_none()
                            && enemy_pos.is_some_and(|ep| {
                                let dx = (ep.x as i32 - nx as i32) * forward;
                                ny == ep.y || dx == 2
                            })));
                let promising_elem_dim = !enemy_invuln
                    && !chip_missed_here
                    && next_is_dimming
                    && enemy_has_elem
                    && enemy_pos.is_some_and(|ep| {
                        let dx = (ep.x as i32 - nx as i32) * forward;
                        ny == ep.y || dx == 3 || dx == 1
                    });
                if promising_miss || promising_elem_dim {
                    let sf_res = rollout_step_and_fire(
                        &sim,
                        side,
                        dir,
                        keep_hold,
                        keep_b,
                        a_charge_full,
                        cfg.horizon,
                        cfg.cap,
                        blind,
                        next_is_dimming,
                    );
                    if sf_res.dealt > 0.0 && sf_res.taken <= base_taken + 0.02 {
                        let sf_cand = Candidate {
                            first: dir | keep_hold,
                            first_ticks: step_ticks,
                            after: keep_hold,
                            is_move: true,
                            is_charge_release: false,
                            is_a_charge_hold: a_charging,
                            is_step_fire: true,
                        };
                        cands.push((sf_cand, sf_res));
                    }
                }
            }
        }
    }

    let b_charge_target = if b_charge_full && !enemy_invuln {
        enemy_pos.and_then(|ep| ctx.reach.best_charge_target(&sim, side, cur_form, my_pos, ep, air_shoe))
    } else {
        None
    };

    // 7. Score candidates with rollout outcome + tactical positioning + area/obstacle play + anti-pacing.
    let next_strafe = if ctx.strafe_timer >= 36 { 0 } else { ctx.strafe_timer + cfg.k };
    let want_strafe = ctx.strafe_timer >= 28 && !b_charge_full && !a_charge_full;
    let my_elem = sim.objects.get(my_ref).element & 0x0F;

    let mut best_idx = 0;
    let mut best_total = f32::NEG_INFINITY;

    for (i, (cand, res)) in cands.iter().enumerate() {
        let mut total = res.score;

        // Slight discount on 2-phase `Step → Fire` so an immediate hit on our current panel still wins ties.
        if cand.is_step_fire {
            total *= 0.88;
        }

        // Keep A-charging while moving/dodging until full!
        if cand.is_a_charge_hold && a_charging && !a_charge_full {
            total += 0.026;
        }

        // Bonus for landing damage during an enemy's attack (Counter Hit -> Full Synchro!).
        if res.dealt > 0.0 && enemy_attacking {
            total += 0.04;
        }

        // Never waste a full B charge shot on empty air or an invulnerable/invisible opponent
        // (penalty exceeds Easy's noise so it never fires into empty air or Invis!).
        if cand.is_charge_release && res.dealt <= base_dealt + 0.001 && res.obs_dmg == 0 {
            total -= 0.065;
        }

        // Subtle tie-breaker for maintaining B charge.
        if can_b_charge && !a_charging && cand.after.contains(Keys::B) && !cand.is_charge_release {
            total += 0.003;
        }

        // Positional heuristics on the ending panel of the rollout.
        if let Some(end_p) = res.end_panel {
            total += positional_score(
                &sim,
                side,
                my_pos,
                end_p,
                res.end_enemy_panel.or(enemy_pos),
                chip_target.or(b_charge_target),
                b_charge_full || a_charge_full || !can_b_charge,
                !enemy_invuln,
                want_strafe,
                air_shoe,
                float_shoe,
                my_elem,
                &ctx.attack_log,
            );

            // Anti-pacing hysteresis: discourage stepping right back to the panel we just left,
            // unless we dodged damage or gained a chip/charge hit.
            if cand.is_move
                && !cand.is_step_fire
                && ctx.ticks_since_move < 18
                && ctx.prev_panel == Some(end_p)
                && res.taken >= base_taken - 0.001
                && res.dealt <= base_dealt + 0.001
            {
                total -= 0.016;
            }
        }

        if cfg.noise > 0.0 {
            total += (rand_unit(&mut rng) - 0.5) * cfg.noise;
        }

        if total > best_total {
            best_total = total;
            best_idx = i;
        }
    }

    let chosen = cands[best_idx].0;
    FightDecision {
        first: chosen.first,
        after: chosen.after,
        first_ticks: chosen.first_ticks,
        reach: ctx.reach,
        strafe_timer: if chosen.is_move { 0 } else { next_strafe },
        enemy_last_panel: next_enemy_panel,
        enemy_stationary_ticks: next_enemy_stat,
    }
}

fn idle_decision(reach: ReachTable, strafe_timer: u32, enemy_last_panel: Option<PanelPos>, enemy_stationary_ticks: u32) -> FightDecision {
    FightDecision {
        first: Keys::empty(),
        after: Keys::empty(),
        first_ticks: 4,
        reach,
        strafe_timer,
        enemy_last_panel,
        enemy_stationary_ticks,
    }
}

/// Pick a realistic 1-panel evasive step direction (`UP` or `DOWN`) for the opponent to model dodging.
fn enemy_evade_dir(b: &Battle, other: u8, ep: PanelPos) -> Option<Keys> {
    let air = navi_status(b, other).contains(StatusFlags::AIRSHOE);
    // Prefer stepping toward the center row (y=2), else whichever vertical neighbor is walkable.
    let order = if ep.y == 1 {
        [Keys::DOWN, Keys::UP]
    } else if ep.y == 3 {
        [Keys::UP, Keys::DOWN]
    } else {
        [Keys::UP, Keys::DOWN]
    };
    order
        .into_iter()
        .find(|&d| stepped_pos(other, ep, d).is_some_and(|(nx, ny)| walkable_panel(b, other, nx, ny, air)))
}

/// Keys that interactive dimming chips (EraseMan, Magnum, ElementMan, BurnSquare,
/// SearchMan, DeltaRayEdge, CircleGun, CrossScope, MetalMan, WoodMan, FireMan, LaserMan,
/// ShadeMan, etc.) may read via `dimmed_pressed` / `dimmed_held`.
const DIMMED_INTERACTIVE_KEYS: Keys = Keys::from_bits_truncate(
    Keys::A.bits()
        | Keys::B.bits()
        | Keys::UP.bits()
        | Keys::DOWN.bits()
        | Keys::LEFT.bits()
        | Keys::RIGHT.bits(),
);

/// Generically detect and control any interactive dimming chip by observing `Battle::dimmed_key_reads`
/// on our own side while dimmed. Returns `Some((first, first_ticks, after))` when an active dimming
/// chip is reading our inputs, or `None` for non-interactive dimming (e.g. AreaGrab / Invis / standard Navi).
fn decide_interactive_dimming(
    sim: &Battle,
    side: u8,
    keep_hold: Keys,
    held_a_before: bool,
    blind: bool,
    cfg: LevelConfig,
    rng: &mut SplitMix64,
) -> Option<(Keys, u8, Keys)> {
    // Cheap 2-tick probe: does ANY active dimming script read our dimmed keys right now?
    let mut probe = sim.clone();
    let r0_all = probe.dimmed_key_reads(side, DIMMED_INTERACTIVE_KEYS);
    let r0_a = probe.dimmed_key_reads(side, Keys::A);
    let r0_b = probe.dimmed_key_reads(side, Keys::B);
    let r0_u = probe.dimmed_key_reads(side, Keys::UP);
    let r0_d = probe.dimmed_key_reads(side, Keys::DOWN);
    let r0_l = probe.dimmed_key_reads(side, Keys::LEFT);
    let r0_r = probe.dimmed_key_reads(side, Keys::RIGHT);

    for _ in 0..2 {
        let mut both = [Keys::empty(); 2];
        both[side as usize] = Keys::empty();
        if !step_battle(&mut probe, both) || !probe.is_dimmed() {
            break;
        }
    }
    if probe.dimmed_key_reads(side, DIMMED_INTERACTIVE_KEYS) == r0_all {
        return None;
    }

    let reads_a = probe.dimmed_key_reads(side, Keys::A) != r0_a;
    let reads_b = probe.dimmed_key_reads(side, Keys::B) != r0_b;
    let reads_u = probe.dimmed_key_reads(side, Keys::UP) != r0_u;
    let reads_d = probe.dimmed_key_reads(side, Keys::DOWN) != r0_d;
    let reads_l = probe.dimmed_key_reads(side, Keys::LEFT) != r0_l;
    let reads_r = probe.dimmed_key_reads(side, Keys::RIGHT) != r0_r;
    let reads_dirs = reads_u || reads_d || reads_l || reads_r;

    // Easy difficulty occasionally mistimes/delays aiming, while Normal/Hard aim cleanly.
    if cfg.noise > 0.03 && rand_unit(rng) < 0.25 {
        return Some((Keys::empty(), cfg.k as u8, Keys::empty()));
    }

    let dim_cap = cfg.cap.min(200);
    let base_none = rollout_dimmed_action(sim, side, Keys::empty(), 2, Keys::empty(), dim_cap, blind);

    // Case 1: Directional / multi-button interactive dimming (MetalMan, FireMan, LaserMan, WoodMan, ShadeMan).
    if reads_dirs {
        let mut choices: Vec<(Keys, u8, Keys)> = Vec::with_capacity(12);
        choices.push((Keys::empty(), cfg.k as u8, Keys::empty()));
        if reads_a {
            choices.push((Keys::A, 2, Keys::empty()));
            // Hold A + direction (e.g. EXE4 MetalMan aiming reticle while holding A).
            for (read, dir) in [(reads_u, Keys::UP), (reads_d, Keys::DOWN), (reads_l, Keys::LEFT), (reads_r, Keys::RIGHT)] {
                if read {
                    choices.push((Keys::A | dir, 2, Keys::A));
                }
            }
            choices.push((Keys::A, cfg.k as u8, Keys::A));
        }
        if reads_b {
            choices.push((Keys::B, 2, Keys::empty()));
        }
        for (read, dir) in [(reads_u, Keys::UP), (reads_d, Keys::DOWN), (reads_l, Keys::LEFT), (reads_r, Keys::RIGHT)] {
            if read {
                choices.push((dir, cfg.k as u8, dir));
            }
        }

        let mut best = (Keys::empty(), cfg.k as u8, Keys::empty());
        let mut best_score = base_none.score;
        for (first, ticks, after) in choices {
            let r = rollout_dimmed_action(sim, side, first, ticks, after, dim_cap, blind);
            if r.score > best_score + 0.005 {
                best_score = r.score;
                best = (first, ticks, after);
            }
        }
        return Some(best);
    }

    // Case 2: Timed button press while dimming (EraseMan, Magnum, BurnSquare, ElementMan,
    //         CircleGun, CrossScope, DeltaRayEdge, SearchMan).
    if reads_a || reads_b {
        // Ensure a fresh rising edge if A was held on the previous tick!
        if held_a_before && reads_a {
            return Some((Keys::empty(), 1, Keys::empty()));
        }

        let mut now_key = Keys::empty();
        let mut now_res = base_none.clone();
        for (read, key) in [(reads_a, Keys::A), (reads_b, Keys::B), (reads_a && reads_b, Keys::A | Keys::B)] {
            if !read {
                continue;
            }
            let r = rollout_dimmed_action(sim, side, key, 2, Keys::empty(), dim_cap, blind);
            if r.score > now_res.score + 0.005 || (r.dealt > now_res.dealt + 0.001 && r.score >= now_res.score - 0.005) {
                now_res = r;
                now_key = key;
            }
        }

        // Look ahead across the next ~60-72 ticks of the aiming cycle (e.g. EraseMan's 3-line cycle,
        // ElementMan's 4-element cycle, Magnum's 6-position sight) to see if waiting produces a
        // strictly better hit!
        let mut best_future = f32::NEG_INFINITY;
        let step = 10u32;
        let max_steps = if cfg.probe_reach { 7 } else { 4 };
        let mut future = sim.clone();
        for _ in 0..max_steps {
            let mut stepped_ok = true;
            for _ in 0..step {
                let both = [Keys::empty(); 2];
                if !step_battle(&mut future, both) || !future.is_dimmed() {
                    stepped_ok = false;
                    break;
                }
            }
            if !stepped_ok {
                break;
            }
            let r = rollout_dimmed_action(&future, side, if reads_a { Keys::A } else { Keys::B }, 2, Keys::empty(), dim_cap, blind);
            if r.score > best_future {
                best_future = r.score;
            }
            if reads_b && reads_a {
                let rb = rollout_dimmed_action(&future, side, Keys::A | Keys::B, 2, Keys::empty(), dim_cap, blind);
                if rb.score > best_future {
                    best_future = rb.score;
                }
            }
        }

        // Press NOW if pressing immediately beats doing nothing AND waiting doesn't find a clearly better window!
        let worth_pressing = !now_key.is_empty()
            && (now_res.dealt > 0.0 || now_res.obs_dmg > 0 || now_res.score > base_none.score + 0.015);
        let better_later = best_future > now_res.score + 0.020;

        if worth_pressing && !better_later {
            return Some((now_key, 2, Keys::empty()));
        }
        // Otherwise wait cleanly with `Keys::empty()` so our next decision has a crisp rising edge!
        return Some((Keys::empty(), cfg.k as u8, Keys::empty()));
    }

    let _ = keep_hold;
    None
}

/// Simulate pressing `first` for `first_ticks` ticks then `after` while dimmed, until dimming ends
/// (+ a few undimmed ticks for post-dim effects) or `cap` ticks elapse.
fn rollout_dimmed_action(
    b: &Battle,
    side: u8,
    first: Keys,
    first_ticks: u8,
    after: Keys,
    cap: u32,
    blind: bool,
) -> RolloutResult {
    let other = side ^ 1;
    let mut c = b.clone();
    let snap0 = capture_snapshot(&c, side, blind);
    let mut undimmed = 0u32;
    let mut t = 0u32;
    while undimmed < 8 && t < cap {
        let mine = if t < first_ticks as u32 { first } else { after };
        let mut held = [Keys::empty(); 2];
        held[side as usize] = mine;
        if !step_battle(&mut c, held) || c.round_end().is_some() {
            break;
        }
        t += 1;
        undimmed += !c.is_dimmed() as u32;
        if navi_hp(&c, side) == 0 || (!blind && navi_hp(&c, other) == 0) {
            break;
        }
    }
    score_rollout(b, &c, side, blind, &snap0)
}

/// Called during a dimming-chip cast rollout the instant an interactive dimming chip starts reading
/// `dimmed_key_reads(side, Keys::A)`: tests pressing `A` across the aiming cycle (`dt = 0, 12, 24, 36, 48, 60`)
/// and returns the best achievable `RolloutResult` so chips like EraseMan, Magnum, ElementMan, and
/// DeltaRayEdge are accurately valued when deciding whether to cast them!
fn best_dimmed_a_rollout(
    mid: &Battle,
    orig: &Battle,
    side: u8,
    blind: bool,
    snap0: &Snap0,
    rem_cap: u32,
) -> RolloutResult {
    let other = side ^ 1;
    let cap = rem_cap.clamp(120, 220);
    let mut best: Option<RolloutResult> = None;
    let mut future = mid.clone();

    for step_idx in 0..6u32 {
        if step_idx > 0 {
            let mut ok = true;
            for _ in 0..12 {
                let both = [Keys::empty(); 2];
                if !step_battle(&mut future, both) || !future.is_dimmed() {
                    ok = false;
                    break;
                }
            }
            if !ok {
                break;
            }
        }
        let mut c = future.clone();
        let mut undimmed = 0u32;
        let mut t = 0u32;
        while undimmed < 8 && t < cap {
            let mine = if t < 2 { Keys::A } else { Keys::empty() };
            let mut held = [Keys::empty(); 2];
            held[side as usize] = mine;
            if !step_battle(&mut c, held) || c.round_end().is_some() {
                break;
            }
            t += 1;
            undimmed += !c.is_dimmed() as u32;
            if navi_hp(&c, side) == 0 || (!blind && navi_hp(&c, other) == 0) {
                break;
            }
        }
        let res = score_rollout(orig, &c, side, blind, snap0);
        if best.as_ref().is_none_or(|b| res.score > b.score) {
            best = Some(res);
        }
    }

    best.unwrap_or_else(|| score_rollout(orig, mid, side, blind, snap0))
}

/// Simulate holding `first` for `first_ticks` ticks, then `after` for up to `horizon` undimmed ticks.
/// If `enemy_evade` is non-empty, the opponent steps in that direction at tick 10 (models human dodging).
/// If `watch_interactive_dim` is true and a newly cast dimming chip begins reading `Keys::A` while dimmed,
/// evaluates pressing `A` across the aiming cycle so interactive dimming chips (EraseMan, Magnum, ElementMan,
/// DeltaRayEdge, etc.) are valued accurately!
#[allow(clippy::too_many_arguments)]
fn rollout(
    b: &Battle,
    side: u8,
    first: Keys,
    first_ticks: u8,
    after: Keys,
    horizon: u32,
    cap: u32,
    blind: bool,
    enemy_evade: Keys,
    watch_interactive_dim: bool,
) -> RolloutResult {
    let other = side ^ 1;
    let mut c = b.clone();

    let snap0 = capture_snapshot(&c, side, blind);
    let attacks0 = count_side_attacks(&c, side);
    let reads_a0 = c.dimmed_key_reads(side, Keys::A);
    let mut undimmed = 0u32;
    let mut t = 0u32;
    while undimmed < horizon && t < cap {
        let mine = if t < first_ticks as u32 { first } else { after };
        let theirs = if !enemy_evade.is_empty() && (10..13).contains(&undimmed) {
            enemy_evade
        } else {
            Keys::empty()
        };
        let mut held = [Keys::empty(); 2];
        held[side as usize] = mine;
        held[other as usize] = theirs;
        if !step_battle(&mut c, held) || c.round_end().is_some() {
            break;
        }
        t += 1;
        undimmed += !c.is_dimmed() as u32;
        if watch_interactive_dim && t >= 4 && c.is_dimmed() && c.dimmed_key_reads(side, Keys::A) != reads_a0 {
            return best_dimmed_a_rollout(&c, b, side, blind, &snap0, cap.saturating_sub(t));
        }
        if navi_hp(&c, side) == 0 || (!blind && navi_hp(&c, other) == 0) {
            break;
        }
    }

    let mut res = score_rollout(b, &c, side, blind, &snap0);

    // If this action spawned a slow lobbed/delayed attack projectile still in flight past `horizon`
    // (e.g. MeddySoul capsule landing at ~47f / column-bursting at ~71f, NumberSoul dice, MiniBomb),
    // follow its flight up to 82 undimmed ticks and credit ONLY the delayed damage/KO (never modifying
    // `res.taken`, so there is zero horizon mismatch against `base_cand`!).
    if !blind && res.dealt == 0.0 && res.obs_dmg == 0 && count_side_attacks(&c, side) > attacks0 {
        extend_delayed_projectile(&mut c, side, after, &snap0, attacks0, &mut res, undimmed, t, cap);
    }

    res
}

/// Follow a newly-spawned slow projectile (MeddySoul capsule, lobbed bombs, etc.) past `horizon` until
/// it hits, disappears, or hits 82 undimmed ticks, crediting only delayed opponent/obstacle damage.
#[allow(clippy::too_many_arguments)]
fn extend_delayed_projectile(
    c: &mut Battle,
    side: u8,
    after: Keys,
    s0: &Snap0,
    attacks0: u8,
    res: &mut RolloutResult,
    mut undimmed: u32,
    mut t: u32,
    cap: u32,
) {
    let other = side ^ 1;
    let ext_horizon = undimmed.max(82);
    let ext_cap = cap.max(t + 64);
    while undimmed < ext_horizon && t < ext_cap {
        let mut held = [Keys::empty(); 2];
        held[side as usize] = after;
        if !step_battle(c, held) || c.round_end().is_some() {
            break;
        }
        t += 1;
        undimmed += !c.is_dimmed() as u32;
        let eh = navi_hp(c, other);
        let (obs_hp1, obs_cnt1) = visible_obstacle_hp(c, side);
        if eh < s0.enemy_hp || obs_hp1 < s0.obs_hp {
            let extra_dealt = s0.enemy_hp.saturating_sub(eh) as f32 / s0.enemy_max;
            let extra_obs = s0.obs_hp.saturating_sub(obs_hp1);
            res.dealt = extra_dealt;
            res.obs_dmg = extra_obs;
            if extra_dealt > 0.0 {
                res.score += extra_dealt * 1.25;
                if s0.enemy_hp > 0 && eh == 0 {
                    res.score += 4.0;
                }
            }
            if extra_obs > 0 && !res.spent_chip {
                res.score += 0.020 + if obs_cnt1 < s0.obs_cnt { 0.025 } else { 0.0 };
            }
            break;
        }
        if count_side_attacks(c, side) <= attacks0 {
            break;
        }
    }
}

fn count_side_attacks(b: &Battle, side: u8) -> u8 {
    let mut n = 0u8;
    for slot in 0..nettai_battle::object::SLOTS as u8 {
        let r = nettai_content_api::ObjectRef { pool: nettai_content_api::Pool::Attack, slot };
        if b.objects.is_allocated(r) && b.objects.get(r).alliance == side {
            n = n.saturating_add(1);
        }
    }
    n
}

/// 2-phase `Step → Fire` rollout: step `dir` cleanly, and on the first idle tick after landing press (or release) `A`!
/// Supports interactive dimming chips (EraseMan, ElementMan, Magnum, etc.) via `watch_interactive_dim`.
#[allow(clippy::too_many_arguments)]
fn rollout_step_and_fire(
    b: &Battle,
    side: u8,
    dir: Keys,
    keep_hold: Keys,
    keep_b: Keys,
    release_a: bool,
    horizon: u32,
    cap: u32,
    blind: bool,
    watch_interactive_dim: bool,
) -> RolloutResult {
    let other = side ^ 1;
    let mut c = b.clone();
    let start_pos = navi_panel(&c, side);
    let snap0 = capture_snapshot(&c, side, blind);
    let reads_a0 = c.dimmed_key_reads(side, Keys::A);

    let mut undimmed = 0u32;
    let mut t = 0u32;
    let mut fired_at: Option<u32> = None;

    while undimmed < horizon && t < cap {
        let cur_pos = navi_panel(&c, side);
        let mine = if t < 3 {
            dir | keep_hold
        } else if fired_at.is_none() && cur_pos != start_pos && navi_idle(&c, side) {
            fired_at = Some(t);
            if release_a { Keys::empty() } else { Keys::A | keep_b }
        } else if let Some(fa) = fired_at {
            if t < fa + 2 {
                if release_a { Keys::empty() } else { Keys::A | keep_b }
            } else {
                keep_b
            }
        } else {
            keep_hold
        };
        let mut held = [Keys::empty(); 2];
        held[side as usize] = mine;
        if !step_battle(&mut c, held) || c.round_end().is_some() {
            break;
        }
        t += 1;
        undimmed += !c.is_dimmed() as u32;
        if watch_interactive_dim
            && let Some(fa) = fired_at
            && t >= fa + 4
            && c.is_dimmed()
            && c.dimmed_key_reads(side, Keys::A) != reads_a0
        {
            return best_dimmed_a_rollout(&c, b, side, blind, &snap0, cap.saturating_sub(t));
        }
        if navi_hp(&c, side) == 0 || (!blind && navi_hp(&c, other) == 0) {
            break;
        }
    }

    score_rollout(b, &c, side, blind, &snap0)
}

struct Snap0 {
    my_hp: u16,
    my_max: f32,
    enemy_hp: u16,
    enemy_max: f32,
    my_barrier: u16,
    enemy_barrier: u16,
    my_panels: i32,
    my_status: StatusFlags,
    enemy_status: StatusFlags,
    obs_hp: u32,
    obs_cnt: u32,
    my_cursor: u8,
    spent_chip_id: Option<ChipHandle>,
}

fn capture_snapshot(c: &Battle, side: u8, blind: bool) -> Snap0 {
    let other = side ^ 1;
    let (obs_hp, obs_cnt) = if blind { (0, 0) } else { visible_obstacle_hp(c, side) };
    Snap0 {
        my_hp: navi_hp(c, side),
        my_max: navi_max_hp(c, side) as f32,
        enemy_hp: if blind { 0 } else { navi_hp(c, other) },
        enemy_max: if blind { 1.0 } else { navi_max_hp(c, other) as f32 },
        my_barrier: navi_barrier_hp(c, side),
        enemy_barrier: if blind { 0 } else { navi_barrier_hp(c, other) },
        my_panels: owned_panel_count(c, side),
        my_status: navi_status(c, side),
        enemy_status: if blind { StatusFlags::empty() } else { navi_status(c, other) },
        obs_hp,
        obs_cnt,
        my_cursor: c.hands[side as usize].cursor,
        spent_chip_id: c.hands[side as usize].next_chip(),
    }
}

fn score_rollout(b: &Battle, c: &Battle, side: u8, blind: bool, s0: &Snap0) -> RolloutResult {
    let other = side ^ 1;
    let my_hp1 = navi_hp(c, side);
    let enemy_hp1 = if blind { 0 } else { navi_hp(c, other) };
    let my_barrier1 = navi_barrier_hp(c, side);
    let enemy_barrier1 = if blind { 0 } else { navi_barrier_hp(c, other) };
    let my_panels1 = owned_panel_count(c, side);
    let my_status1 = navi_status(c, side);
    let enemy_status1 = if blind { StatusFlags::empty() } else { navi_status(c, other) };
    let (obs_hp1, obs_cnt1) = if blind { (0, 0) } else { visible_obstacle_hp(c, side) };
    let obs_dmg = s0.obs_hp.saturating_sub(obs_hp1);
    let spent_chip = c.hands[side as usize].cursor > s0.my_cursor;

    let dealt = if blind { 0.0 } else { s0.enemy_hp.saturating_sub(enemy_hp1) as f32 / s0.enemy_max };
    let taken = s0.my_hp.saturating_sub(my_hp1) as f32 / s0.my_max;
    let healed = my_hp1.saturating_sub(s0.my_hp) as f32 / s0.my_max;

    // Prioritize winning/finishing over passive draws: value dealing damage more than minor chip trades,
    // and ramp aggression when opponent is low, when we are behind on HP, or in late turns.
    let my_frac = s0.my_hp as f32 / s0.my_max;
    let enemy_frac = if blind { 1.0 } else { s0.enemy_hp as f32 / s0.enemy_max };
    let push_win = !blind && (enemy_frac < 0.35 || my_frac < enemy_frac - 0.08 || b.round.turn >= 8);
    let dealt_w = if push_win { 1.80 } else { 1.45 };
    let taken_w = if push_win { 1.00 } else { 1.10 };
    let mut score = dealt * dealt_w - taken * taken_w;

    // Reward clearing/damaging obstacles with free buster/charge shots (don't waste attack chips just to scratch obstacles).
    if obs_dmg > 0 && !spent_chip {
        score += 0.024 + if obs_cnt1 < s0.obs_cnt { 0.030 } else { 0.0 };
    }

    // Lethal KO always dominates!
    if !blind && s0.enemy_hp > 0 && enemy_hp1 == 0 {
        score += 4.0;
    }
    if s0.my_hp > 0 && my_hp1 == 0 {
        score -= 3.5;
    }

    // Healing value: high when hurt, zero when nearly full HP so Recovery isn't wasted!
    // Never credit a passive form heal (e.g. AquaCross +5% on Water chips, RollSoul) for throwing away
    // an attack chip into empty air (`dealt == 0.0`) unless critically low HP (< 18%) — align and hit
    // to get BOTH the heal and the damage!
    let wasted_attack_heal = spent_chip
        && dealt == 0.0
        && s0.spent_chip_id.is_some_and(|id| {
            let cd = c.content.chip(id);
            cd.flags.contains(ChipFlags::HAS_DAMAGE) || cd.damage > 0 || cd.formula.is_some()
        })
        && my_frac >= 0.18;
    if healed > 0.0 && !wasted_attack_heal {
        let heal_weight = if my_frac < 0.50 {
            1.15
        } else if my_frac < 0.80 {
            0.78
        } else if my_frac < 0.92 {
            0.35
        } else {
            0.0
        };
        score += healed * heal_weight;
    }

    // Barrier gain / enemy barrier break.
    // Note: defensive buff gains (`gained_invis`, `gained_barrier`, `gained_trap`) are NOT given an
    // unconditional bonus here — they are evaluated contextually in `decide` (and naturally rewarded
    // here whenever they prevent incoming `taken` damage or win a dimming cut-in!).
    let gained_barrier = my_barrier1.saturating_sub(s0.my_barrier);
    if !blind && s0.enemy_barrier > enemy_barrier1 {
        let broken = (s0.enemy_barrier - enemy_barrier1) as f32 / s0.enemy_max;
        score += broken.min(0.25) * 0.20 + if enemy_barrier1 == 0 { 0.045 } else { 0.0 };
    }

    // Area control (AreaGrab / PanelGrab!).
    let area_diff = my_panels1 - s0.my_panels;
    if area_diff != 0 {
        score += area_diff as f32 * 0.060;
    }

    // Track defensive buffs gained so `decide` can apply them reactively without spamming/stacking.
    let good_flags = StatusFlags::INVISIBLE | StatusFlags::INVULNERABLE;
    let gained_invis = (my_status1 - s0.my_status).intersects(good_flags);
    let trap_before = navi_ref(b, side)
        .and_then(|r| b.objects.get(r).actor)
        .is_some_and(|a| b.actors.get(a).status.intersects(ActorStatus::TRAP_ARMED | ActorStatus::HEAT_TRAP));
    let trap_after = navi_ref(c, side)
        .and_then(|r| c.objects.get(r).actor)
        .is_some_and(|a| c.actors.get(a).status.intersects(ActorStatus::TRAP_ARMED | ActorStatus::HEAT_TRAP));
    let gained_trap = !trap_before && trap_after;

    // Negative statuses suffered / inflicted.
    let bad_flags =
        StatusFlags::PARALYZED | StatusFlags::FROZEN | StatusFlags::BUBBLED | StatusFlags::CONFUSED | StatusFlags::IMMOBILIZED | StatusFlags::BLIND;
    if (my_status1 - s0.my_status).intersects(bad_flags) {
        score -= 0.085;
    }
    if !blind && (enemy_status1 - s0.enemy_status).intersects(bad_flags) {
        score += 0.070;
    }

    // Chip conservation cost: never throw away a chip for nothing, while eagerly firing chips that deal real damage!
    if spent_chip {
        let class_cost = s0.spent_chip_id.map_or(0.016, |id| match c.content.chip(id).class {
            ChipClass::Giga | ChipClass::ProgramAdvance => 0.032,
            ChipClass::Mega => 0.022,
            _ => 0.015,
        });
        let non_damaging = s0.spent_chip_id.is_some_and(|id| {
            let cd = c.content.chip(id);
            !cd.flags.contains(ChipFlags::HAS_DAMAGE) && cd.damage == 0 && cd.formula.is_none()
        });
        if non_damaging && panel_types_changed(b, c) {
            score += 0.048;
        }
        score -= class_cost;
    }

    let end_enemy_panel = if blind {
        None
    } else {
        navi_ref(c, other).filter(|&r| seen_to(c, r, side)).map(|r| c.objects.get(r).panel)
    };
    let end_field_objs = count_side_field_objects(c, side);
    let end_friendly_objs = count_friendly_objects(c, side);
    let end_rules = c.rules_state(side).map(|(_, blk)| blk.clone());

    let end_a_charging = navi_ref(c, side)
        .and_then(|r| c.objects.get(r).actor)
        .is_some_and(|a| {
            let ad = c.actors.get(a);
            ad.charge_source == 1 && ad.charge_level >= 1
        });

    RolloutResult {
        score,
        dealt,
        taken,
        obs_dmg,
        spent_chip,
        gained_invis,
        gained_barrier,
        gained_trap,
        ended_dimmed: c.is_dimmed(),
        end_a_charging,
        end_panel: navi_panel(c, side),
        end_enemy_panel,
        end_gauge_rate: c.gauge.rate,
        end_field_objs,
        end_friendly_objs,
        end_rules,
    }
}

/// Total HP and count of visible non-player actor objects (obstacles/boulders/viruses) on the field.
fn visible_obstacle_hp(b: &Battle, viewer: u8) -> (u32, u32) {
    let p0 = navi_ref(b, 0);
    let p1 = navi_ref(b, 1);
    let mut hp = 0u32;
    let mut cnt = 0u32;
    for slot in 0..nettai_battle::object::SLOTS as u8 {
        let r = nettai_content_api::ObjectRef { pool: nettai_content_api::Pool::Actor, slot };
        if Some(r) == p0 || Some(r) == p1 || !seen_to(b, r, viewer) {
            continue;
        }
        let o = b.objects.get(r);
        if o.hp > 0 {
            hp += o.hp as u32;
            cnt += 1;
        }
    }
    (hp, cnt)
}

fn count_side_field_objects(b: &Battle, side: u8) -> u8 {
    let base = 3 * (side as usize & 1);
    b.field.objects.slots[base..base + 3].iter().filter(|s| s.is_some()).count() as u8
}

fn count_friendly_objects(b: &Battle, side: u8) -> u8 {
    let my = navi_ref(b, side);
    let mut n = 0u8;
    // Only count persistent actor summons/obstacles (RockCube, Guardian, Anubis, Fan, etc.),
    // never transient `Pool::Attack` projectiles that haven't finished flying off-screen.
    for slot in 0..nettai_battle::object::SLOTS as u8 {
        let r = nettai_content_api::ObjectRef { pool: nettai_content_api::Pool::Actor, slot };
        if Some(r) == my || !b.objects.is_allocated(r) {
            continue;
        }
        if b.objects.get(r).alliance == side {
            n = n.saturating_add(1);
        }
    }
    n
}

fn panel_types_changed(before: &Battle, after: &Battle) -> bool {
    for y in 1..=3u8 {
        for x in 1..=6u8 {
            if let (Some(a), Some(b)) = (before.field.panel(x, y), after.field.panel(x, y))
                && a.kind != b.kind
            {
                return true;
            }
        }
    }
    false
}

/// Check if there is a visible obstacle (e.g. RockCube) on `end_p.y` directly between `end_p` and the opponent.
fn obstacle_shields_panel(b: &Battle, side: u8, end_p: PanelPos, enemy_p: PanelPos) -> bool {
    if end_p.y != enemy_p.y {
        return false;
    }
    let lo = end_p.x.min(enemy_p.x);
    let hi = end_p.x.max(enemy_p.x);
    let p0 = navi_ref(b, 0);
    let p1 = navi_ref(b, 1);
    for slot in 0..nettai_battle::object::SLOTS as u8 {
        let r = nettai_content_api::ObjectRef { pool: nettai_content_api::Pool::Actor, slot };
        if Some(r) == p0 || Some(r) == p1 || !seen_to(b, r, side) {
            continue;
        }
        let o = b.objects.get(r);
        if o.hp > 0 && o.panel.y == end_p.y && o.panel.x > lo && o.panel.x < hi {
            return true;
        }
    }
    false
}

/// Positional tie-breaker score for ending on panel `end_p`. Kept small (~0.001..0.035) so
/// dodging damage and landing hits always dominate.
#[allow(clippy::too_many_arguments)]
fn positional_score(
    b: &Battle,
    side: u8,
    start_p: PanelPos,
    end_p: PanelPos,
    enemy_p: Option<PanelPos>,
    nav_target: Option<PanelPos>,
    ready_shot: bool,
    enemy_vulnerable: bool,
    want_strafe: bool,
    air_shoe: bool,
    float_shoe: bool,
    my_elem: u8,
    attack_log: &AttackLog,
) -> f32 {
    let mut score = 0.0f32;
    let other = side ^ 1;

    // 1. Steer directly toward the learned chip / short-range charge hit-panel using exact BFS
    //    walkable distance so holes and stage obstacles are routed around cleanly!
    if let Some(target) = nav_target {
        let d_start = grid_dist(b, side, start_p, target, air_shoe).unwrap_or_else(|| manhattan(start_p, target));
        let d_end = grid_dist(b, side, end_p, target, air_shoe).unwrap_or_else(|| manhattan(end_p, target));
        score += (d_start - d_end) as f32 * 0.018;
        if end_p == target {
            score += 0.010;
        }
    } else if let Some(ep) = enemy_p {
        if ready_shot && enemy_vulnerable {
            let d_row_start = (start_p.y as i32 - ep.y as i32).abs();
            let d_row_end = (end_p.y as i32 - ep.y as i32).abs();
            score += (d_row_start - d_row_end) as f32 * 0.014;
            if end_p.y == ep.y {
                score += 0.006;
            }
        } else if want_strafe {
            if end_p.y != ep.y {
                score += 0.008;
            }
        }
    }

    // 2. Area control & PvP column tactics:
    //    - Standing on a stolen column locks it so it doesn't revert to the opponent!
    //    - Standing on our front column blocks incoming AreaGrab steals and shortens travel time.
    if let Some(p) = b.field.panel(end_p.x, end_p.y) {
        if p.alliance == side && p.home != side {
            let col = b.field.columns[end_p.x as usize];
            score += if col.return_ready != 0 || col.timer < 90 { 0.014 } else { 0.009 };
        }
        if p.front_edge && p.alliance == side {
            score += 0.004;
        }

        // Panel quality & safety.
        if let Some(rule) = b.game_rules().panels.types.get(p.kind.0 as usize) {
            if rule.damage_shift > 0 {
                score += 0.010; // Holy panel!
            }
            if !float_shoe && rule.traps {
                score -= 0.016;
            }
            if !float_shoe && rule.doubles.is_some() {
                score -= 0.005;
            }
            if !float_shoe && rule.expires.is_some() && rule.doubles.is_none() && rule.carries.is_none() && my_elem == 0 {
                score -= 0.006;
            }
        }
        if p.flags.contains(PanelFlags::CRACKED) && !float_shoe {
            score -= 0.006;
        }
        if p.highlight > 0 {
            score -= 0.014;
        }
    }

    // Obstacle cover: when charging/recovering (not firing a straight shot), standing behind a RockCube shields us!
    if !ready_shot && nav_target.is_none() && enemy_p.is_some_and(|ep| obstacle_shields_panel(b, side, end_p, ep)) {
        score += 0.008;
    }

    // Avoid panels the opponent just attacked.
    if attack_log.recent_attack_on(other, end_p.x, end_p.y) {
        score -= 0.010;
    }

    // Mild preference for center row & panels with escape routes (avoid backing into corners).
    if end_p.y == 2 {
        score += 0.003;
    }
    let neighbors = [Keys::UP, Keys::DOWN, Keys::RIGHT, Keys::LEFT]
        .into_iter()
        .filter(|&d| stepped_pos(side, end_p, d).is_some_and(|(nx, ny)| walkable_panel(b, side, nx, ny, air_shoe)))
        .count();
    score += (neighbors.min(4) as f32) * 0.0015;

    score
}

fn manhattan(a: PanelPos, b: PanelPos) -> i32 {
    (a.x as i32 - b.x as i32).abs() + (a.y as i32 - b.y as i32).abs()
}

/// Shortest walkable step distance between `from` and `to` on the 6x3 battle grid (routes around
/// holes, broken panels, and stage obstacles cleanly; zero heap allocation).
fn grid_dist(b: &Battle, side: u8, from: PanelPos, to: PanelPos, air_shoe: bool) -> Option<i32> {
    if from == to {
        return Some(0);
    }
    let idx = |x: u8, y: u8| ((y - 1) as usize) * 6 + ((x - 1) as usize);
    let mut dist = [u8::MAX; 18];
    let mut queue = [PanelPos { x: 1, y: 1 }; 18];
    let (mut head, mut tail) = (0usize, 0usize);
    dist[idx(from.x, from.y)] = 0;
    queue[tail] = from;
    tail += 1;

    while head < tail {
        let cur = queue[head];
        head += 1;
        let d = dist[idx(cur.x, cur.y)];
        for (dx, dy) in [(0i32, -1i32), (0, 1), (-1, 0), (1, 0)] {
            let nx = cur.x as i32 + dx;
            let ny = cur.y as i32 + dy;
            if !(1..=6).contains(&nx) || !(1..=3).contains(&ny) {
                continue;
            }
            let (nx, ny) = (nx as u8, ny as u8);
            let ni = idx(nx, ny);
            if dist[ni] != u8::MAX {
                continue;
            }
            if !walkable_panel(b, side, nx, ny, air_shoe) {
                continue;
            }
            let nd = d + 1;
            if nx == to.x && ny == to.y {
                return Some(nd as i32);
            }
            dist[ni] = nd;
            queue[tail] = PanelPos { x: nx, y: ny };
            tail += 1;
        }
    }
    None
}

/// Probe up to 6 canonical walkable own-side panels to learn all relative `(dx, dy)` hit offsets for the current chip!
fn probe_chip_offsets(b: &Battle, side: u8, my_pos: PanelPos, enemy_pos: PanelPos, air_shoe: bool) -> Vec<(i8, i8)> {
    let forward: i32 = if side == 0 { 1 } else { -1 };
    let xs: Vec<u8> = if side == 0 { (1..=6).rev().collect() } else { (1..=6).collect() };
    let front_on_row = xs.iter().copied().find(|&x| walkable_panel(b, side, x, enemy_pos.y, air_shoe));
    let back_on_row = xs.iter().rev().copied().find(|&x| walkable_panel(b, side, x, enemy_pos.y, air_shoe));
    let two_ahead_x = if side == 0 {
        enemy_pos.x.checked_sub(2)
    } else {
        let tx = enemy_pos.x + 2;
        (tx <= 6).then_some(tx)
    };
    let bomb_x = if side == 0 {
        enemy_pos.x.checked_sub(3)
    } else {
        let bx = enemy_pos.x + 3;
        (bx <= 6).then_some(bx)
    };

    let mut tries: Vec<PanelPos> = Vec::with_capacity(8);
    let mut push = |x: u8, y: u8| {
        let p = PanelPos { x, y };
        if p != my_pos && walkable_panel(b, side, x, y, air_shoe) && !tries.contains(&p) {
            tries.push(p);
        }
    };

    push(my_pos.x, enemy_pos.y);
    if let Some(tx) = two_ahead_x {
        push(tx, my_pos.y);
        push(tx, enemy_pos.y);
    }
    if let Some(fx) = front_on_row {
        push(fx, enemy_pos.y);
    }
    if let Some(bx) = bomb_x {
        push(bx, enemy_pos.y);
    }
    if let Some(bx) = back_on_row {
        push(bx, enemy_pos.y);
    }
    for adj_y in [enemy_pos.y.saturating_sub(1), enemy_pos.y + 1] {
        if (1..=3).contains(&adj_y)
            && let Some(fx) = xs.iter().copied().find(|&x| walkable_panel(b, side, x, adj_y, air_shoe))
        {
            push(fx, adj_y);
        }
    }

    tries.sort_by_key(|&p| manhattan(my_pos, p));

    let mut offsets = Vec::new();
    for target in tries {
        if walk_and_chip_hits(b, side, target, air_shoe) {
            let dx = ((enemy_pos.x as i32 - target.x as i32) * forward) as i8;
            let dy = (enemy_pos.y as i32 - target.y as i32) as i8;
            if !offsets.contains(&(dx, dy)) {
                offsets.push((dx, dy));
            }
            // Once we've found 2 hitting offsets (e.g. close + same row), stop early to stay fast!
            if offsets.len() >= 2 {
                break;
            }
        }
    }
    offsets
}

/// First cardinal direction key along a shortest walkable path from `from` to `to` (routes around holes!).
fn next_step_toward(b: &Battle, side: u8, from: PanelPos, to: PanelPos, air_shoe: bool) -> Keys {
    let mut best_dir = Keys::empty();
    let mut best_d = i32::MAX;
    for dir in [Keys::UP, Keys::DOWN, Keys::RIGHT, Keys::LEFT] {
        if let Some((nx, ny)) = stepped_pos(side, from, dir)
            && walkable_panel(b, side, nx, ny, air_shoe)
            && let Some(d) = grid_dist(b, side, PanelPos { x: nx, y: ny }, to, air_shoe)
            && d < best_d
        {
            best_d = d;
            best_dir = dir;
        }
    }
    if !best_dir.is_empty() {
        best_dir
    } else {
        dir_key_for_delta(side, to.x as i32 - from.x as i32, to.y as i32 - from.y as i32)
    }
}

/// Walk `side` to `target` on a battle copy (routing around holes via BFS) and press A; returns true if
/// the opponent takes damage (handling both normal/dimming chips like JudgeMan and interactive dimming
/// aimers like EraseMan, Magnum, and ElementMan via `Battle::dimmed_key_reads`).
fn walk_and_chip_hits(b: &Battle, side: u8, target: PanelPos, air_shoe: bool) -> bool {
    let other = side ^ 1;
    let mut c = b.clone();
    let enemy_hp0 = navi_hp(&c, other);
    if enemy_hp0 == 0 {
        return false;
    }

    let mut released = true;
    let mut arrived = false;
    for _ in 0..56 {
        let Some(at) = navi_panel(&c, side) else { return false };
        if at == target && navi_idle(&c, side) {
            arrived = true;
            break;
        }
        let dir = next_step_toward(&c, side, at, target, air_shoe);
        let press = if navi_idle(&c, side) && released { dir } else { Keys::empty() };
        released = press.is_empty();
        let mut held = [Keys::empty(); 2];
        held[side as usize] = press;
        if !step_battle(&mut c, held) || !fight_runs(&c) {
            return false;
        }
    }
    if !arrived {
        return false;
    }

    let hp_before_chip = navi_hp(&c, other);
    let attacks0 = count_side_attacks(&c, side);
    let snap0 = capture_snapshot(&c, side, false);
    let reads_a0 = c.dimmed_key_reads(side, Keys::A);
    let is_dimming = c
        .hands[side as usize]
        .next_chip()
        .is_some_and(|id| c.content.chip(id).flags.contains(ChipFlags::DIMMING));
    let mut undimmed = 0u32;
    let mut t = 0u32;
    while (undimmed < 34 || (undimmed < 82 && count_side_attacks(&c, side) > attacks0)) && t < 320 {
        let mut held = [Keys::empty(); 2];
        held[side as usize] = if t < 2 { Keys::A } else { Keys::empty() };
        if !step_battle(&mut c, held) || c.round_end().is_some() {
            break;
        }
        t += 1;
        undimmed += !c.is_dimmed() as u32;
        if is_dimming && t >= 4 && c.is_dimmed() && c.dimmed_key_reads(side, Keys::A) != reads_a0 {
            let best = best_dimmed_a_rollout(&c, &c, side, false, &snap0, 220);
            return best.dealt > 0.0;
        }
        if navi_hp(&c, other) < hp_before_chip {
            return true;
        }
    }
    navi_hp(&c, other) < hp_before_chip
}

/// Probe canonical walkable own-side panels to learn relative `(dx, dy)` hit offsets for the active form's
/// full B-charge shot (handles melee forms like ProtoSoul/SlashCross AND fixed-range lobbed forms like MeddySoul!).
fn probe_b_charge_offsets(b: &Battle, side: u8, my_pos: PanelPos, enemy_pos: PanelPos, air_shoe: bool) -> Vec<(i8, i8)> {
    let forward: i32 = if side == 0 { 1 } else { -1 };
    let xs: Vec<u8> = if side == 0 { (1..=6).rev().collect() } else { (1..=6).collect() };
    let front_on_row = xs.iter().copied().find(|&x| walkable_panel(b, side, x, enemy_pos.y, air_shoe));
    let back_on_row = xs.iter().rev().copied().find(|&x| walkable_panel(b, side, x, enemy_pos.y, air_shoe));
    let two_ahead_x = if side == 0 {
        enemy_pos.x.checked_sub(2)
    } else {
        let tx = enemy_pos.x + 2;
        (tx <= 6).then_some(tx)
    };
    let bomb_x = if side == 0 {
        enemy_pos.x.checked_sub(3)
    } else {
        let bx = enemy_pos.x + 3;
        (bx <= 6).then_some(bx)
    };

    let mut tries: Vec<PanelPos> = Vec::with_capacity(8);
    let mut push = |x: u8, y: u8| {
        let p = PanelPos { x, y };
        if p != my_pos && walkable_panel(b, side, x, y, air_shoe) && !tries.contains(&p) {
            tries.push(p);
        }
    };

    push(my_pos.x, enemy_pos.y);
    if let Some(bx) = bomb_x {
        push(bx, enemy_pos.y);
        push(bx, my_pos.y);
    }
    if let Some(tx) = two_ahead_x {
        push(tx, enemy_pos.y);
    }
    if let Some(fx) = front_on_row {
        push(fx, enemy_pos.y);
    }
    if let Some(bx) = back_on_row {
        push(bx, enemy_pos.y);
    }
    for adj_y in [enemy_pos.y.saturating_sub(1), enemy_pos.y + 1] {
        if (1..=3).contains(&adj_y) {
            if let Some(bx) = bomb_x {
                push(bx, adj_y);
            }
            if let Some(fx) = xs.iter().copied().find(|&x| walkable_panel(b, side, x, adj_y, air_shoe)) {
                push(fx, adj_y);
            }
        }
    }

    tries.sort_by_key(|&p| manhattan(my_pos, p));

    let mut offsets = Vec::new();
    for target in tries {
        if walk_and_b_charge_hits(b, side, target, air_shoe) {
            let dx = ((enemy_pos.x as i32 - target.x as i32) * forward) as i8;
            let dy = (enemy_pos.y as i32 - target.y as i32) as i8;
            if !offsets.contains(&(dx, dy)) {
                offsets.push((dx, dy));
            }
            if offsets.len() >= 2 {
                break;
            }
        }
    }
    offsets
}

/// Walk `side` to `target` while maintaining `Keys::B` charge, release `B`, and check if the charge shot hits.
fn walk_and_b_charge_hits(b: &Battle, side: u8, target: PanelPos, air_shoe: bool) -> bool {
    let other = side ^ 1;
    let mut c = b.clone();
    if navi_hp(&c, other) == 0 {
        return false;
    }
    let mut released = true;
    let mut arrived = false;
    for _ in 0..56 {
        let Some(at) = navi_panel(&c, side) else { return false };
        if at == target && navi_idle(&c, side) {
            arrived = true;
            break;
        }
        let dir = next_step_toward(&c, side, at, target, air_shoe);
        let step = if navi_idle(&c, side) && released { dir } else { Keys::empty() };
        released = step.is_empty();
        let mut held = [Keys::empty(); 2];
        held[side as usize] = step | Keys::B;
        if !step_battle(&mut c, held) || !fight_runs(&c) {
            return false;
        }
    }
    if !arrived {
        return false;
    }
    let hp_before = navi_hp(&c, other);
    let attacks0 = count_side_attacks(&c, side);
    let mut undimmed = 0u32;
    let mut t = 0u32;
    while (undimmed < 34 || (undimmed < 82 && count_side_attacks(&c, side) > attacks0)) && t < 160 {
        let mut held = [Keys::empty(); 2];
        held[side as usize] = if t < 2 { Keys::empty() } else { Keys::B };
        if !step_battle(&mut c, held) || c.round_end().is_some() {
            break;
        }
        t += 1;
        undimmed += !c.is_dimmed() as u32;
        if navi_hp(&c, other) < hp_before {
            return true;
        }
    }
    navi_hp(&c, other) < hp_before
}

fn rand_unit(rng: &mut SplitMix64) -> f32 {
    ((rng.next_u64() >> 40) as f32) / ((1u64 << 24) as f32)
}
