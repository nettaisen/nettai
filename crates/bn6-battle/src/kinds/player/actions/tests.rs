//! Action timelines, as the original game runs them in a netbattle (tick
//! offsets from the tick the action starts), on the hand-authored test
//! content (`content::testing`): the timings that come from data are that
//! content's, the rest are the engine's. The same timelines with BN6's own
//! data are checked in the verification workspace.

use super::super::ai_mut;
use crate::actor::request;
use crate::battle::{Battle, battle_flags};
use crate::collision::f1;
use crate::hand::ChipHand;
use crate::input::keys;
use crate::object::{ObjectRef, PanelPos, Pool, state};
use crate::content::testing;
use crate::setup::{NaviStats, NaviWeapons};

/// A plain MegaMan with 1000 HP, fighting in the sun.
fn megaman() -> NaviStats {
    megaman_with(|_| {})
}

/// The same, changed by `f`.
fn megaman_with(f: impl FnOnce(&mut NaviStats)) -> NaviStats {
    let mut s = megaman_stats();
    f(&mut s);
    s
}

fn megaman_stats() -> NaviStats {
    NaviStats {
        hp: 1000,
        max_hp: 1000,
        max_base_hp: 1000,
        mood: 0x80,
        sun: true,
        weapons: NaviWeapons { buster: 0, charge_shot: 1, back_special: 0xFF, a_charge: 0xFF, mode9_a: 0xFF, ..Default::default() },
        ..Default::default()
    }
}

/// Two navis idle and fighting: side 0 at (2,2), side 1 at (5,2) (side 1
/// updates first). Returns the battle and both players.
fn fight() -> (Battle, ObjectRef, ObjectRef) {
    fight_with(megaman())
}

/// The same with both navis' stats `stats`.
fn fight_with(stats: NaviStats) -> (Battle, ObjectRef, ObjectRef) {
    // A link battle on a plain field with the usual two-navi list.
    let mut setup = testing::round_setup(testing::LINK_BATTLE, stats);
    setup.settings.effects = 0xE8C;
    let mut b = Battle::new(setup, testing::content());
    b.spawn_actors();
    b.run_objects();
    b.round.flags |= battle_flags::FIGHTING;
    let players = [b.player(0).unwrap(), b.player(1).unwrap()];
    for p in players {
        let o = b.objects.get_mut(p);
        (o.action, o.phase, o.phase_init) = (8, 0, 0);
    }
    (b, players[0], players[1])
}

/// One tick of objects with side 0 holding `held`.
fn tick(b: &mut Battle, p0: ObjectRef, p1: ObjectRef, held: u16) {
    ai_mut(b, p0).pad.update(held | keys::PRESENT);
    ai_mut(b, p1).pad.update(keys::PRESENT);
    b.run_objects();
}

fn following(b: &Battle, r: ObjectRef) -> Vec<(Pool, u8)> {
    b.objects.in_order().skip_while(|&o| o != r).skip(1).map(|o| (o.pool, b.objects.get(o).index)).collect()
}

fn f1_of(b: &Battle, r: ObjectRef) -> u32 {
    b.collision.get(b.objects.get(r).collision.unwrap()).f1
}

/// Run ticks with side 0 holding `held` until tick `last` (counting the
/// action's first tick as 0) has run.
fn run_to(b: &mut Battle, p: [ObjectRef; 2], now: &mut u32, last: u32, held: u16) {
    while *now < last {
        *now += 1;
        tick(b, p[0], p[1], held);
    }
}

#[test]
fn a_step_commits_on_the_third_tick_and_ends_on_the_twelfth() {
    let (mut b, p0, p1) = fight();
    let (p, mut t) = ([p0, p1], 0);
    tick(&mut b, p0, p1, keys::RIGHT);
    let o = b.objects.get(p0);
    assert_eq!((o.action, o.anim, o.panel, o.future_panel), (0x10, 4, PanelPos { x: 2, y: 2 }, PanelPos { x: 3, y: 2 }));
    assert_eq!(b.field.panel(3, 2).unwrap().reserver, Some(p0));
    assert_ne!(f1_of(&b, p0) & f1::MOVING, 0);
    run_to(&mut b, p, &mut t, 2, keys::RIGHT);
    assert_eq!(b.objects.get(p0).panel, PanelPos { x: 2, y: 2 });
    run_to(&mut b, p, &mut t, 3, keys::RIGHT);
    let o = b.objects.get(p0);
    assert_eq!((o.panel, o.pos.x, o.anim), (PanelPos { x: 3, y: 2 }, -20 << 16, 3));
    assert_eq!(b.field.panel(3, 2).unwrap().reserver, None);
    run_to(&mut b, p, &mut t, 8, keys::RIGHT);
    assert_eq!(b.objects.get(p0).anim, 0);
    assert_eq!(f1_of(&b, p0) & (f1::MOVING | f1::MOVE_COMPLETE), f1::MOVE_COMPLETE);
    run_to(&mut b, p, &mut t, 11, 0);
    assert_eq!(b.objects.get(p0).action, 0x10);
    run_to(&mut b, p, &mut t, 12, 0);
    assert_eq!(b.objects.get(p0).action, 8);
}

#[test]
fn gun_del_sol_drains_4_hp_a_tick_in_the_sun() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    // Step to (3,2), in range of the target at (5,2).
    let mut t = 0;
    tick(&mut b, p0, p1, keys::RIGHT);
    run_to(&mut b, p, &mut t, 12, 0);
    assert_eq!((b.objects.get(p0).action, b.objects.get(p0).panel), (8, PanelPos { x: 3, y: 2 }));

    let mut hand = ChipHand::empty();
    hand.ids[0] = testing::SUN_GUN_3;
    let firing = b.content.chip(testing::SUN_GUN_3).gun_del_sol.unwrap().firing_ticks as u32;
    b.hands[0] = hand;
    let mut t = 0;
    tick(&mut b, p0, p1, keys::A);
    assert_eq!(b.objects.get(p0).action, 0x37);
    assert_eq!(b.hands[0].cursor, 1);
    assert_eq!(ai_mut(&mut b, p0).requests & request::CHIP, 0);

    // Tick 1: the gun comes out, at its owner's attach point.
    run_to(&mut b, p, &mut t, 1, keys::A);
    assert_eq!(b.objects.get(p0).anim, 0x0A);
    let attachment = b.content.object_kind("attachment").unwrap();
    assert_eq!(following(&b, p0)[0], (attachment.pool, attachment.index));
    let gun = ai_mut(&mut b, p0).overlay.unwrap();
    let (at, g) = (b.objects.get(p0).pos, b.objects.get(gun).pos);
    let point = testing::GUN_POINT;
    assert_eq!((g.x - at.x, g.y - at.y, g.z - at.z), ((point.x as i32) << 16, 0, (point.y as i32) << 16));

    // Tick 7: the beam, two panels ahead.
    run_to(&mut b, p, &mut t, 7, 0);
    let beam = b.objects.get(p0).related[0].unwrap();
    let sun_beam = b.content.object_kind("sun-beam").unwrap();
    assert_eq!((beam.pool, b.objects.get(beam).index), (sun_beam.pool, sun_beam.index));
    assert_eq!(b.objects.get(beam).pos.x, 60 << 16);
    assert_eq!(b.objects.get(gun).anim, 1);

    // Hits from tick 8 land on the target's next update: 4 HP a tick,
    // without flinching it.
    run_to(&mut b, p, &mut t, 8, 0);
    assert_eq!(b.objects.get(p1).hp, 1000);
    run_to(&mut b, p, &mut t, 9, 0);
    assert_eq!(b.objects.get(p1).hp, 996);
    assert_eq!(b.objects.get(p1).action, 8);
    assert_eq!(f1_of(&b, p1) & (f1::FLINCHING | f1::FLASHING), 0);
    // One hit a tick for the chip's firing time.
    let drained = 1000 - 4 * firing as u16;
    run_to(&mut b, p, &mut t, 8 + firing, 0);
    assert_eq!(b.objects.get(p1).hp, drained);
    assert!(!b.objects.is_allocated(beam));
    run_to(&mut b, p, &mut t, 9 + firing, 0);
    assert_eq!(b.objects.get(gun).anim, 2);

    // 11 ticks later: back to idle; the gun ends itself and is gone a
    // tick later.
    run_to(&mut b, p, &mut t, 18 + firing, 0);
    assert_eq!(b.objects.get(p0).action, 0x37);
    run_to(&mut b, p, &mut t, 19 + firing, 0);
    assert_eq!(b.objects.get(p0).action, 8);
    assert_eq!(b.objects.get(gun).state, state::DESTROY);
    run_to(&mut b, p, &mut t, 20 + firing, 0);
    assert!(!b.objects.is_allocated(gun));
    assert_eq!(b.objects.get(p1).hp, drained);
}

#[test]
fn a_blank_shot_raises_the_arm_and_recovers_from_its_own_panel() {
    // The NaviCust's buster bug fills all 16 slots with blanks: the buster
    // script's draw always picks the blank shot.
    let (mut b, p0, p1) = fight_with(megaman_with(|s| s.bugs.buster_blanks = 16));
    let p = [p0, p1];
    // B fires on release (the navi has a charged shot).
    tick(&mut b, p0, p1, keys::B);
    let mut t = 0;
    tick(&mut b, p0, p1, 0);
    assert_eq!(b.objects.get(p0).action, 0x33);

    // Tick 1: the arm is up.
    run_to(&mut b, p, &mut t, 1, 0);
    assert_eq!(b.objects.get(p0).anim, 0x0E);
    assert_ne!(f1_of(&b, p0) & f1::USING_ACTION, 0);
    let arm = ai_mut(&mut b, p0).overlay.expect("the buster arm");
    let attachment = b.content.object_kind("attachment").unwrap();
    let o = b.objects.get(arm);
    assert_eq!((arm.pool, o.index, o.params[0]), (attachment.pool, attachment.index, 6));

    // Five ticks up, then the recovery by the open panels from its own
    // (its body is off the field while it updates) to the enemy's:
    // rules.buster_recovery[Rapid 0][3].
    let recovery = b.content.rules.buster_recovery(0, 3) as u32;
    run_to(&mut b, p, &mut t, 5 + recovery, 0);
    assert_eq!(b.objects.get(p0).action, 0x33);
    run_to(&mut b, p, &mut t, 6 + recovery, 0);
    assert_eq!(b.objects.get(p0).action, 8);
    assert_eq!(ai_mut(&mut b, p0).overlay, None);
    // Nothing was fired.
    assert_eq!(b.objects.get(p1).hp, 1000);
}

#[test]
fn dustcross_charged_shot_rolls_junk_into_the_enemy() {
    // Weapon routine 0x28 as the charged shot.
    let (mut b, p0, p1) = fight_with(megaman_with(|s| s.weapons.charge_shot = 0x28));
    let p = [p0, p1];
    // Charge fully (the test rules: 120 ticks at Charge 0), then release.
    for _ in 0..130 {
        tick(&mut b, p0, p1, keys::B);
    }
    let mut t = 0;
    tick(&mut b, p0, p1, 0);
    assert_eq!(b.objects.get(p0).action, 0x57);

    // Tick 2: the ball, in front of the navi.
    run_to(&mut b, p, &mut t, 2, 0);
    let dust_ball = b.content.object_kind("dust-ball").unwrap();
    let ball = b.objects.in_order().find(|&o| (o.pool, b.objects.get(o).index) == (dust_ball.pool, dust_ball.index));
    let ball = ball.expect("the ball");
    assert_eq!(b.objects.get(ball).panel, PanelPos { x: 3, y: 2 });

    // It rolls up to the enemy at (5,2), bursts, and hits: 50 damage and
    // 10 per buster Attack point (1 here). The enemy's panel cracks.
    run_to(&mut b, p, &mut t, 60, 0);
    assert_eq!(b.objects.get(ball).panel, PanelPos { x: 5, y: 2 });
    assert_eq!(b.objects.get(p1).hp, 940);
    assert_eq!(b.field.panel(5, 2).unwrap().kind, crate::field::PanelType::Cracked);
    // The navi idles 35 ticks after the shot.
    run_to(&mut b, p, &mut t, 70, 0);
    assert_eq!(b.objects.get(p0).action, 8);
}

// ---- The Beast forms' weapons --------------------------------------------------

/// Start weapon routine `routine` for `r` as a charged chip does
/// (`sub_800FB54`): the routine's setup, then its action. Returns the
/// action.
fn start_weapon(b: &mut Battle, r: ObjectRef, routine: u8) -> u8 {
    ai_mut(b, r).attack.charged = 0;
    let action = crate::kinds::player::idle::weapon_routine(b, r, routine);
    crate::kinds::player::set_attack(b, r, action, 2);
    action
}

/// The objects of the content kind `name`.
fn of_kind(b: &Battle, name: &str) -> Vec<ObjectRef> {
    let k = b.content.object_kind(name).unwrap();
    b.objects.in_order().filter(|&o| (o.pool, b.objects.get(o).index) == (k.pool, k.index)).collect()
}

/// A copy of the battle plays the next `n` ticks exactly as the battle
/// does (the scripts' state is all in the battle).
fn plays_on_the_same(b: &mut Battle, p: [ObjectRef; 2], n: u32) {
    let mut copy = b.clone();
    for _ in 0..n {
        tick(b, p[0], p[1], 0);
        tick(&mut copy, p[0], p[1], 0);
        assert_eq!(b.digest(), copy.digest());
    }
}

#[test]
fn heat_beast_charge_raises_fire_pillars_on_its_region() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    assert_eq!(start_weapon(&mut b, p0, 0x07), 0x35);
    // 50 damage and 30 per buster Attack point (1 here), Fire.
    let a = &ai_mut(&mut b, p0).attack;
    assert_eq!((a.damage, a.hit_param, a.element), (80, 0x8A, 1));
    let mut t = 0;
    run_to(&mut b, p, &mut t, 1, 0);
    assert_eq!(b.objects.get(p0).anim, 0x12);
    assert_eq!(f1_of(&b, p0) & (f1::USING_ACTION | f1::MOVING), f1::USING_ACTION | f1::MOVING);
    // When the wind-up ends: pillars on the test region 0x1A from the
    // panel in front (it and two past it).
    while of_kind(&b, "element-pillar").is_empty() {
        let next = t + 1;
        run_to(&mut b, p, &mut t, next, 0);
        assert!(t < 20, "no pillars");
    }
    assert_eq!(b.objects.get(p0).anim, 0x13);
    // (Each runs right after its spawner: the later one first.)
    let panels: Vec<PanelPos> = of_kind(&b, "element-pillar").iter().map(|&o| b.objects.get(o).panel).collect();
    assert_eq!(panels, [PanelPos { x: 5, y: 2 }, PanelPos { x: 3, y: 2 }]);
    let pillar = of_kind(&b, "element-pillar")[1];
    let (x, y) = crate::kinds::player::panel_coordinates(3, 2);
    let o = b.objects.get(pillar);
    assert_eq!((o.pos.x, o.pos.y, o.pos.z, o.timer), (x, y + (2 << 16), 2 << 16, 0x5A - 1));
    plays_on_the_same(&mut b, p, 20);
    t += 20;
    // One hit: the pillar's region goes once it hits.
    assert_eq!(b.objects.get(p1).hp, 920);
    // The navi idles 91 ticks after the pillars; they are gone by then.
    let end = t + 75;
    run_to(&mut b, p, &mut t, end, 0);
    assert_eq!(b.objects.get(p0).action, 8);
    assert_eq!(f1_of(&b, p0) & f1::MOVING, 0);
    assert!(of_kind(&b, "element-pillar").is_empty());
    assert_eq!(b.objects.get(p1).hp, 920);
}

#[test]
fn elec_beast_charge_strikes_lightning_that_cracks_panels() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    assert_eq!(start_weapon(&mut b, p0, 0x09), 0x3C);
    let mut t = 0;
    while of_kind(&b, "element-pillar").is_empty() {
        let next = t + 1;
        run_to(&mut b, p, &mut t, next, 0);
        assert!(t < 20, "no lightning");
    }
    // Lightning (Param1 1) cracks its panels, the enemy's too.
    assert_eq!(b.field.panel(3, 2).unwrap().kind, crate::field::PanelType::Cracked);
    assert_eq!(b.field.panel(5, 2).unwrap().kind, crate::field::PanelType::Cracked);
    plays_on_the_same(&mut b, p, 5);
    t += 5;
    // 40 damage and 30 per buster Attack point.
    assert_eq!(b.objects.get(p1).hp, 930);
    // The navi stops 91 ticks after the lightning, which lasts 9 more at
    // most.
    let end = t + 100;
    run_to(&mut b, p, &mut t, end, 0);
    assert_eq!(b.objects.get(p0).action, 8);
    assert!(of_kind(&b, "element-pillar").is_empty());
}

#[test]
fn spout_beast_charge_surges_from_the_panel_in_front() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    // Step to (3,2) first.
    let mut t = 0;
    tick(&mut b, p0, p1, keys::RIGHT);
    run_to(&mut b, p, &mut t, 12, 0);
    assert_eq!(b.objects.get(p0).panel, PanelPos { x: 3, y: 2 });
    assert_eq!(start_weapon(&mut b, p0, 0x08), 0x3A);
    let mut t = 0;
    // 8 ticks in, the surge, 20 pixels ahead of the panel in front.
    run_to(&mut b, p, &mut t, 8, 0);
    assert!(of_kind(&b, "aqua-surge").is_empty());
    run_to(&mut b, p, &mut t, 9, 0);
    let surge = of_kind(&b, "aqua-surge")[0];
    let (x, _) = crate::kinds::player::panel_coordinates(4, 2);
    let o = b.objects.get(surge);
    assert_eq!((o.panel, o.pos.x, o.pos.z), (PanelPos { x: 4, y: 2 }, x + (20 << 16), 0));
    // It reaches the enemy on the panel ahead (region 2): 10 damage and 10
    // per buster Attack point, once until its region comes back.
    plays_on_the_same(&mut b, p, 5);
    t += 5;
    assert_eq!(b.objects.get(p1).hp, 980);
    // The navi idles 61 ticks after the surge; the surge lasts 60.
    run_to(&mut b, p, &mut t, 9 + 60, 0);
    assert_eq!(b.objects.get(p0).action, 0x3A);
    run_to(&mut b, p, &mut t, 9 + 62, 0);
    assert_eq!(b.objects.get(p0).action, 8);
    assert!(of_kind(&b, "aqua-surge").is_empty());
}

#[test]
fn tengu_beast_charge_sends_a_whirlwind_that_leaves_hits() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    assert_eq!(start_weapon(&mut b, p0, 0x0A), 0x3D);
    let a = &ai_mut(&mut b, p0).attack;
    assert_eq!((a.damage, a.element, a.params), (50, 0x20, [0x00, 0x3D, 0x2D, 0x00]));
    let mut t = 0;
    while of_kind(&b, "whirlwind").is_empty() {
        let next = t + 1;
        run_to(&mut b, p, &mut t, next, 0);
        assert!(t < 20, "no whirlwind");
    }
    let w = of_kind(&b, "whirlwind")[0];
    let (x, _) = crate::kinds::player::panel_coordinates(3, 2);
    let o = b.objects.get(w);
    assert_eq!((o.panel, o.pos.x, o.timer), (PanelPos { x: 3, y: 2 }, x + (40 << 16), 0x2D - 1));
    // Its first wave's last hit covers the column two panels ahead, where
    // the enemy stands.
    plays_on_the_same(&mut b, p, 5);
    t += 5;
    assert_eq!(b.objects.get(p1).hp, 950);
    let end = t + 60;
    run_to(&mut b, p, &mut t, end, 0);
    assert_eq!(b.objects.get(p0).action, 8);
    assert!(of_kind(&b, "whirlwind").is_empty());
}

#[test]
fn the_beast_busters_raise_the_arm_for_their_projectile() {
    for (routine, action) in [(0x03, 0x1E), (0x04, 0x1D)] {
        let (mut b, p0, p1) = fight();
        let p = [p0, p1];
        assert_eq!(start_weapon(&mut b, p0, routine), action);
        let a = &ai_mut(&mut b, p0).attack;
        // The buster's damage (at most 5), no repeats outside Beast Over.
        assert_eq!((a.damage, a.params[1], a.variant), (1, 0, 0));
        let mut t = 0;
        run_to(&mut b, p, &mut t, 1, 0);
        assert_eq!(b.objects.get(p0).anim, 0x0E);
        let arm = ai_mut(&mut b, p0).overlay.expect("the buster arm");
        assert_eq!(b.objects.get(arm).params[0], 6);
        // (The shot is the buster's projectile, which isn't content yet.)
    }
}

#[test]
fn dustcross_beast_throws_its_newest_obstacle_or_fires_the_beast_buster() {
    let (mut b, p0, _) = fight();
    assert_eq!(start_weapon(&mut b, p0, 0x2C), 0x1E);
    let actor = b.objects.get(p0).actor.unwrap();
    b.actors.get_mut(actor).absorbed.push(crate::actor::AbsorbedObstacle { kind: 2, anim: 1 });
    assert_eq!(start_weapon(&mut b, p0, 0x2C), 0x11);
    let a = &ai_mut(&mut b, p0).attack;
    assert_eq!((a.damage, a.variant, a.marker), (200, 2, 0x12));
    assert!(b.actors.get(actor).absorbed.is_empty());
}
