//! Fair simulation and battle inspection helpers shared across the AI.

use nettai_battle::actor::{ActorStatus, Requests};
use nettai_battle::battle::{fight, mode};
use nettai_battle::collision::StatusFlags;
use nettai_battle::content::{ChipFlags, ChipTraits};
use nettai_battle::field::PanelFlags;
use nettai_battle::hand::ChipHand;
use nettai_battle::hud::CustomGauge;
use nettai_battle::input::Keys;
use nettai_battle::kinds::player::NaviAction;
use nettai_battle::object::ObjectRef;
use nettai_battle::{Battle, PlayerTick, Rng};
use nettai_content_api::PanelPos;

/// Whether the fight is actively running (including while dimmed, when cut-ins are possible).
pub fn fight_runs(b: &Battle) -> bool {
    b.round.mode == mode::FIGHTING && b.fight.state == fight::FIGHTING && !b.is_battle_over()
}

/// Whether the custom screen can be opened with L right now.
pub fn may_open(b: &Battle) -> bool {
    fight_runs(b) && !b.is_dimmed() && b.gauge.value >= CustomGauge::FULL
}

/// Alternating L press/release to open the custom screen from tick `at`.
pub fn presses_l(t: u32, at: u32) -> bool {
    t >= at && (t - at).is_multiple_of(2)
}

/// Step `b` one tick with full `PlayerTick` inputs; returns `false` if the engine stopped or panicked.
pub fn step_ticks(b: &mut Battle, ticks: [PlayerTick; 2]) -> bool {
    let input = nettai_netplay::standin::tick_input(b, ticks.map(|t| t.word()));
    let stepped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.step(&input)));
    match stepped {
        Ok(()) => !b.is_stopped(),
        Err(_) => false,
    }
}

/// Step `b` one tick with raw held buttons for both sides.
pub fn step_battle(b: &mut Battle, held: [Keys; 2]) -> bool {
    step_ticks(b, held.map(PlayerTick::buttons))
}

/// Sanitize a cloned `Battle` before running lookahead for `side`:
/// - Re-seeds the battle RNG and both console RNGs from `luck` so the AI can never foresee luck.
/// - Clears the opponent's hidden hand (when undimmed) and custom screen so hidden state never leaks.
pub fn sanitize_battle(b: &Battle, side: u8, luck: u64) -> Battle {
    let mut c = b.clone();
    c.rng = Rng::new(luck as u32);
    let hi = (luck >> 32) as u32;
    c.consoles[0].rng = Rng::new(hi ^ 0x9E37_79B9);
    c.consoles[1].rng = Rng::new(hi ^ 0x85EB_CA6B);
    let other = (side ^ 1) as usize;
    if !c.is_dimmed() {
        c.hands[other] = ChipHand::empty(&c.content);
        if let Some(r) = navi_ref(&c, other as u8) {
            c.objects.get_mut(r).chips_held = 0;
            c.objects.get_mut(r).chip = None;
            if let Some(a) = c.objects.get(r).actor {
                c.actors
                    .get_mut(a)
                    .requests
                    .remove(Requests::CHIP | Requests::CHARGED_CHIP | Requests::ALT_CHIP | Requests::CUT_IN);
            }
        }
    }
    c.custom.sides[other].screen = None;
    c
}

/// Allocated player navi reference for `side`.
pub fn navi_ref(b: &Battle, side: u8) -> Option<ObjectRef> {
    b.player(side).filter(|&r| b.objects.is_allocated(r))
}

/// Current panel of `side`'s navi.
pub fn navi_panel(b: &Battle, side: u8) -> Option<PanelPos> {
    navi_ref(b, side).map(|r| b.objects.get(r).panel)
}

/// Whether `side`'s navi is currently idle and accepting inputs.
pub fn navi_idle(b: &Battle, side: u8) -> bool {
    navi_ref(b, side)
        .and_then(|r| b.objects.get(r).actor)
        .is_some_and(|a| b.actors.get(a).navi_action == NaviAction::Idle)
}

/// Current HP of `side`'s navi (0 if deleted/absent).
pub fn navi_hp(b: &Battle, side: u8) -> u16 {
    navi_ref(b, side).map_or(0, |r| b.objects.get(r).hp)
}

/// Max HP of `side`'s navi (at least 1).
pub fn navi_max_hp(b: &Battle, side: u8) -> u16 {
    navi_ref(b, side).map_or(1, |r| b.objects.get(r).max_hp.max(1))
}

/// Current barrier HP of `side`'s navi (0 if none).
pub fn navi_barrier_hp(b: &Battle, side: u8) -> u16 {
    navi_ref(b, side)
        .and_then(|r| b.objects.get(r).collision)
        .filter(|&c| b.collision.in_use(c))
        .map_or(0, |c| {
            let cd = b.collision.get(c);
            if cd.barrier != 0 {
                (cd.barrier_hp as u16).max(cd.barrier_threshold as u16).max(1)
            } else {
                0
            }
        })
}

/// Status flags of `side`'s navi.
pub fn navi_status(b: &Battle, side: u8) -> StatusFlags {
    navi_ref(b, side)
        .and_then(|r| b.objects.get(r).collision)
        .filter(|&c| b.collision.in_use(c))
        .map_or(StatusFlags::empty(), |c| b.collision.get(c).f1)
}

/// Whether object `r` is visible to `viewer` (including blinking actors when `viewer` isn't blind).
pub fn seen_to(b: &Battle, r: ObjectRef, viewer: u8) -> bool {
    if !b.objects.is_allocated(r) {
        return false;
    }
    if b.visible_to(r, viewer) {
        return true;
    }
    let o = b.objects.get(r);
    if !b.sees(viewer, o.alliance) {
        return false;
    }
    // A human sees a blinking (flashing/invisible) navi even on its off-blink ticks.
    o.collision
        .filter(|&c| b.collision.in_use(c))
        .is_some_and(|c| b.collision.get(c).f1.intersects(StatusFlags::FLASHING | StatusFlags::INVISIBLE))
}

/// Visible opponent navi's panel, if `viewer` can see it.
pub fn enemy_seen_panel(b: &Battle, viewer: u8) -> Option<PanelPos> {
    let other = viewer ^ 1;
    let r = navi_ref(b, other)?;
    seen_to(b, r, viewer).then(|| b.objects.get(r).panel)
}

/// Count of playable panels currently owned by `side`.
pub fn owned_panel_count(b: &Battle, side: u8) -> i32 {
    let mut n = 0;
    for y in 1..=3u8 {
        for x in 1..=6u8 {
            if b.field.panel(x, y).is_some_and(|p| p.alliance == side) {
                n += 1;
            }
        }
    }
    n
}

/// Whether `side`'s navi can stand on panel `(x, y)` (owned by `side`, solid or AirShoes, and no blocker/other body).
pub fn walkable_panel(b: &Battle, side: u8, x: u8, y: u8, air_shoe: bool) -> bool {
    if !(1..=6).contains(&x) || !(1..=3).contains(&y) {
        return false;
    }
    let Some(p) = b.field.panel(x, y) else {
        return false;
    };
    if p.alliance != side {
        return false;
    }
    if !air_shoe && !p.flags.contains(PanelFlags::SOLID) {
        return false;
    }
    // Avoid stepping into obstacles/blockers/other bodies (our own body is fine).
    let my_body = if side == 0 { PanelFlags::BODY_SIDE0 } else { PanelFlags::BODY_SIDE1 };
    let blocking = (PanelFlags::OCCUPIED - my_body) | PanelFlags::BLOCKER;
    !p.flags.intersects(blocking)
}

/// Map a field `dx` (+1 = right on grid, -1 = left on grid) to the console key for `side`.
/// On side 0, RIGHT moves +x; on side 1, RIGHT moves -x (toward side 0).
pub fn dir_key_for_delta(side: u8, dx: i32, dy: i32) -> Keys {
    if dy < 0 {
        return Keys::UP;
    }
    if dy > 0 {
        return Keys::DOWN;
    }
    if dx == 0 {
        return Keys::empty();
    }
    let toward_x = if side == 0 { 1 } else { -1 };
    if dx.signum() == toward_x {
        Keys::RIGHT
    } else {
        Keys::LEFT
    }
}

/// Target `(x, y)` panel for a console movement key `dir` from `(x, y)` on `side`.
pub fn stepped_pos(side: u8, pos: PanelPos, dir: Keys) -> Option<(u8, u8)> {
    let forward: i32 = if side == 0 { 1 } else { -1 };
    let (nx, ny) = match dir {
        Keys::UP => (pos.x as i32, pos.y as i32 - 1),
        Keys::DOWN => (pos.x as i32, pos.y as i32 + 1),
        Keys::RIGHT => (pos.x as i32 + forward, pos.y as i32),
        Keys::LEFT => (pos.x as i32 - forward, pos.y as i32),
        _ => (pos.x as i32, pos.y as i32),
    };
    if (1..=6).contains(&nx) && (1..=3).contains(&ny) {
        Some((nx as u8, ny as u8))
    } else {
        None
    }
}

/// Whether `side`'s next chip in hand can be charged by holding `A` in its current navi/form
/// (mirrors `nettai_battle::kinds::player::input::a_chargeable`, purely from public engine data).
pub fn a_chargeable(b: &Battle, side: u8) -> bool {
    let Some(r) = navi_ref(b, side) else { return false };
    let Some(aid) = b.objects.get(r).actor else { return false };
    let a = b.actors.get(aid);
    if (a.a_charge.is_none() && a.alt_a_charge.is_none()) || a.status.contains(ActorStatus::NO_CHARGE) {
        return false;
    }
    let Some(chip) = b.hands[side as usize].next_chip() else { return false };
    if b.content.chip_links(chip).own_chip_of.is_some() {
        return false;
    }
    let c = b.content.chip(chip);
    let family = c.family;
    let damaging = c.flags.contains(ChipFlags::HAS_DAMAGE) && !c.flags.contains(ChipFlags::DIMMING);
    let plain = !c.flags.contains(ChipFlags::DIMMING) && !c.flags.contains(ChipFlags::DARK);
    let form = b.form(side as usize);
    if form.charged_chips.iter().any(|rule| {
        (family == rule.family || (rule.element_swords && c.traits.contains(ChipTraits::ELEMENT_SWORD)))
            && (damaging || !rule.damaging)
            && (plain || !rule.plain)
    }) {
        return true;
    }
    let navi_handle = b.stats[side as usize].navi;
    let Some(own) = b.content.navi(navi_handle).charged_chips else {
        return false;
    };
    if !b.given.navis[side as usize & 1].charges {
        return false;
    }
    family == own.family && (damaging || !own.damaging) && (plain || !own.plain)
}
