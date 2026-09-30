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

/// The kind named `name` somewhere on the field.
fn find_kind(b: &Battle, name: &str) -> Option<ObjectRef> {
    let k = b.content.object_kind(name).unwrap();
    b.objects.in_order().find(|&o| (o.pool, b.objects.get(o).index) == (k.pool, k.index))
}

/// Give side `side` the chip `chip` as the next in its hand, with its
/// damage.
fn hand_with(b: &mut Battle, side: usize, chip: crate::content::ChipId) {
    let mut hand = ChipHand::empty();
    hand.ids[0] = chip;
    hand.damage[0] = b.content.chip(chip).damage;
    b.hands[side] = hand;
}

#[test]
fn a_recovery_chip_heals_its_hp_in_one_tick() {
    let (mut b, p0, p1) = fight();
    b.objects.get_mut(p0).hp = 500;
    hand_with(&mut b, 0, testing::MEND);
    tick(&mut b, p0, p1, keys::A);
    assert_eq!(b.objects.get(p0).action, 0x20);
    // The next tick: 40 HP, the sparkle right after the navi, a recovery
    // counted, and back to idle.
    tick(&mut b, p0, p1, 0);
    let o = b.objects.get(p0);
    assert_eq!((o.hp, o.action), (540, 8));
    assert_eq!(following(&b, p0)[0], (Pool::Effect, 0));
    assert_eq!(b.side_stats[0][5], 1);
    // Never past the maximum.
    b.objects.get_mut(p0).hp = 990;
    hand_with(&mut b, 0, testing::MEND);
    tick(&mut b, p0, p1, keys::A);
    tick(&mut b, p0, p1, 0);
    assert_eq!(b.objects.get(p0).hp, 1000);
}

#[test]
fn a_reflector_guards_for_its_first_parameter_then_its_shield_fades() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    hand_with(&mut b, 0, testing::MIRROR);
    let mut t = 0;
    tick(&mut b, p0, p1, keys::A);
    assert_eq!(b.objects.get(p0).action, 0x2B);
    // Tick 1: the shield, right after the navi at its attach point 6, and
    // the guard up.
    run_to(&mut b, p, &mut t, 1, 0);
    let shield = find_kind(&b, "reflector-shield").expect("the shield");
    assert_eq!(b.objects.in_order().skip_while(|&o| o != p0).nth(1), Some(shield));
    assert_ne!(f1_of(&b, p0) & f1::GUARD, 0);
    let (at, s) = (b.objects.get(p0).pos, b.objects.get(shield).pos);
    assert_eq!((s.x - at.x, s.y - at.y, s.z - at.z), (4 << 16, 0, 24 << 16));
    // It guards for 30 ticks after that one.
    run_to(&mut b, p, &mut t, 31, 0);
    assert_eq!(b.objects.get(p0).action, 0x2B);
    run_to(&mut b, p, &mut t, 32, 0);
    assert_eq!(b.objects.get(p0).action, 8);
    assert_eq!(f1_of(&b, p0) & f1::GUARD, 0);
    // The shield fades for 14 ticks and goes.
    run_to(&mut b, p, &mut t, 47, 0);
    assert_eq!(find_kind(&b, "reflector-shield"), Some(shield));
    run_to(&mut b, p, &mut t, 48, 0);
    assert_eq!(find_kind(&b, "reflector-shield"), None);
}

#[test]
fn a_reflector_sends_the_first_blocked_hit_back_along_the_row() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    // Side 0 steps to (3,2), in the column side 1's GunDelSol hits.
    let mut t = 0;
    tick(&mut b, p0, p1, keys::RIGHT);
    run_to(&mut b, p, &mut t, 12, 0);
    assert_eq!(b.objects.get(p0).panel, PanelPos { x: 3, y: 2 });
    hand_with(&mut b, 0, testing::MIRROR);
    hand_with(&mut b, 1, testing::SUN_GUN_3);
    // Both use their chips.
    let both = |b: &mut Battle, held: [u16; 2]| {
        ai_mut(b, p0).pad.update(held[0] | keys::PRESENT);
        ai_mut(b, p1).pad.update(held[1] | keys::PRESENT);
        b.run_objects();
    };
    both(&mut b, [keys::A, keys::A]);
    assert_eq!((b.objects.get(p0).action, b.objects.get(p1).action), (0x2B, 0x37));
    // The guard blocks the beam's hits (no drain), and the first one sends
    // a wave back that runs along the row into side 1: 50 damage, once.
    let mut waves = 0;
    for _ in 0..30 {
        both(&mut b, [0, 0]);
        waves += find_kind(&b, "reflected-shot").is_some() as u32;
    }
    assert!(waves > 0, "no wave");
    assert_eq!(b.objects.get(p0).hp, 1000);
    assert_eq!(b.objects.get(p1).hp, 950);
}

/// Side 0 uses `chip` (the first in its hand) from idle: the tick the
/// action starts.
fn use_chip(b: &mut Battle, p0: ObjectRef, p1: ObjectRef, chip: crate::content::ChipId) {
    let mut hand = ChipHand::empty();
    hand.ids[0] = chip;
    hand.damage[0] = b.content.chip(chip).damage;
    b.hands[0] = hand;
    tick(b, p0, p1, keys::A);
}

/// The effect objects (effect #0) and afterimages (effect #0x28) there are.
fn effects(b: &Battle, index: u8) -> Vec<ObjectRef> {
    b.objects.in_order().filter(|&o| o.pool == Pool::Effect && b.objects.get(o).index == index).collect()
}

#[test]
fn a_step_sword_steps_in_slashes_and_steps_back() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    let mut t = 0;
    use_chip(&mut b, p0, p1, testing::STEP_BLADE);
    assert_eq!(b.objects.get(p0).action, 0x13);

    // Tick 1: an afterimage where it stood, and it is two panels ahead,
    // its own panel held for the way back.
    run_to(&mut b, p, &mut t, 1, 0);
    let o = b.objects.get(p0);
    assert_eq!((o.panel, o.future_panel), (PanelPos { x: 4, y: 2 }, PanelPos { x: 2, y: 2 }));
    assert_eq!(b.field.panel(2, 2).unwrap().reserver, Some(p0));
    assert_ne!(f1_of(&b, p0) & f1::MOVING, 0);
    let first = effects(&b, 0x28);
    assert_eq!(first.len(), 1);
    assert_eq!(b.objects.get(first[0]).pos.x, crate::kinds::player::panel_coordinates(2, 2).0);

    // Tick 3: the swing, with the blade.
    run_to(&mut b, p, &mut t, 3, 0);
    assert_eq!(b.objects.get(p0).anim, 5);
    let blade = b.objects.get(p0).related[0].expect("the blade");
    assert_eq!(b.objects.get(blade).params[0], 7);
    // Tick 8: two more afterimages: the navi's and the blade's.
    run_to(&mut b, p, &mut t, 8, 0);
    assert_eq!(effects(&b, 0x28).len(), 3);
    // Tick 12: the slash on the column ahead hits the target at (5,2) on
    // its next update, and its effect shows.
    run_to(&mut b, p, &mut t, 11, 0);
    assert!(effects(&b, 0).is_empty());
    run_to(&mut b, p, &mut t, 12, 0);
    assert_eq!(b.objects.get(effects(&b, 0)[0]).params[0], 0x16);
    run_to(&mut b, p, &mut t, 13, 0);
    assert_eq!(b.objects.get(p1).hp, 920);

    // Tick 25, once the swing's animation is over: back home, the blade
    // let go.
    run_to(&mut b, p, &mut t, 24, 0);
    assert_eq!(b.objects.get(p0).panel, PanelPos { x: 4, y: 2 });
    run_to(&mut b, p, &mut t, 25, 0);
    assert_eq!(b.objects.get(p0).panel, PanelPos { x: 2, y: 2 });
    assert_eq!(b.field.panel(2, 2).unwrap().reserver, None);
    // Tick 31: idle.
    run_to(&mut b, p, &mut t, 30, 0);
    assert_eq!(b.objects.get(p0).action, 0x13);
    run_to(&mut b, p, &mut t, 31, 0);
    assert_eq!(b.objects.get(p0).action, 8);
    assert_eq!(b.objects.get(p0).related[0], None);
    assert_eq!(f1_of(&b, p0) & f1::MOVING, 0);
}

#[test]
fn a_sword_without_a_target_ahead_swings_at_nothing() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    let mut t = 0;
    use_chip(&mut b, p0, p1, testing::BLADE);
    // No step: it swings from where it stands on tick 3, and the slash
    // (on tick 12) finds nobody at (3,2).
    run_to(&mut b, p, &mut t, 3, 0);
    let o = b.objects.get(p0);
    assert_eq!((o.panel, o.anim), (PanelPos { x: 2, y: 2 }, 5));
    assert!(effects(&b, 0x28).is_empty());
    run_to(&mut b, p, &mut t, 13, 0);
    assert_eq!(b.objects.get(p1).hp, 1000);
    // No way back to walk: idle on tick 30.
    run_to(&mut b, p, &mut t, 29, 0);
    assert_eq!(b.objects.get(p0).action, 0x13);
    run_to(&mut b, p, &mut t, 30, 0);
    assert_eq!(b.objects.get(p0).action, 8);
}

/// The objects of content kind `name` on the field, in update order.
fn of_kind(b: &Battle, name: &str) -> Vec<ObjectRef> {
    let k = b.content.object_kind(name).unwrap();
    b.objects.in_order().filter(|&o| (o.pool, b.objects.get(o).index) == (k.pool, k.index)).collect()
}

#[test]
fn a_buster_shot_flies_a_panel_every_two_ticks_and_hits() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    // B fires on release (the navi has a charged shot).
    tick(&mut b, p0, p1, keys::B);
    let mut t = 0;
    tick(&mut b, p0, p1, 0);
    assert_eq!(b.objects.get(p0).action, 0x11);

    // Tick 1: the arm is up; tick 2: the shot, in front of the navi, and
    // the muzzle flash in the first related slot.
    run_to(&mut b, p, &mut t, 1, 0);
    assert_eq!(b.objects.get(p0).anim, 0x0E);
    assert!(of_kind(&b, "projectile").is_empty());
    run_to(&mut b, p, &mut t, 2, 0);
    let shot = of_kind(&b, "projectile");
    assert_eq!(shot.len(), 1);
    let shot = shot[0];
    let o = b.objects.get(shot);
    assert_eq!((o.panel, o.pos.z, o.params[0]), (PanelPos { x: 3, y: 2 }, 0x18 << 16, 0));
    let flash = b.objects.get(p0).related[0].expect("the muzzle flash");
    assert_eq!(b.objects.get(flash).params[0], 5);

    // A panel every two ticks, from the tick after it appears.
    run_to(&mut b, p, &mut t, 3, 0);
    assert_eq!(b.objects.get(shot).panel, PanelPos { x: 4, y: 2 });
    run_to(&mut b, p, &mut t, 5, 0);
    assert_eq!(b.objects.get(shot).panel, PanelPos { x: 5, y: 2 });
    assert_eq!(b.objects.get(p1).hp, 1000);
    // The enemy updates before it: the hit lands on the next tick (the
    // buster's damage, 1), and the shot ends.
    run_to(&mut b, p, &mut t, 6, 0);
    assert_eq!(b.objects.get(p1).hp, 999);
    assert_eq!(b.objects.get(shot).state, state::DESTROY);
    run_to(&mut b, p, &mut t, 7, 0);
    assert!(of_kind(&b, "projectile").is_empty());

    // Five ticks up, then the recovery by the open panels from the one in
    // front (3,2) to the enemy's: rules.buster_recovery[Rapid 0][2].
    let recovery = b.content.rules.buster_recovery(0, 2) as u32;
    run_to(&mut b, p, &mut t, 5 + recovery, 0);
    assert_eq!(b.objects.get(p0).action, 0x11);
    run_to(&mut b, p, &mut t, 6 + recovery, 0);
    assert_eq!(b.objects.get(p0).action, 8);
    assert_eq!((ai_mut(&mut b, p0).overlay, b.objects.get(p0).related[0]), (None, None));
}

#[test]
fn a_move_cuts_the_buster_recovery_short() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    tick(&mut b, p0, p1, keys::B);
    let mut t = 0;
    tick(&mut b, p0, p1, 0);
    // Up is held from tick 6 (the recovery's first tick): the step starts
    // at once.
    run_to(&mut b, p, &mut t, 5, 0);
    assert_eq!(b.objects.get(p0).action, 0x11);
    run_to(&mut b, p, &mut t, 6, keys::UP);
    assert_eq!(b.objects.get(p0).action, 0x10);
    assert_eq!(b.objects.get(p0).future_panel, PanelPos { x: 2, y: 1 });
}

#[test]
fn the_spread_fires_two_more_shots_a_row_up_and_down() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    tick(&mut b, p0, p1, keys::B);
    let mut t = 0;
    tick(&mut b, p0, p1, 0);
    // Variant 1 (no weapon routine sets it up for MegaMan).
    ai_mut(&mut b, p0).attack.variant = 1;
    run_to(&mut b, p, &mut t, 2, 0);
    let shots = of_kind(&b, "projectile");
    assert_eq!(shots.len(), 3);
    assert!(shots.iter().all(|&s| b.objects.get(s).panel == PanelPos { x: 3, y: 2 }));
    run_to(&mut b, p, &mut t, 3, 0);
    let mut panels: Vec<_> = shots.iter().map(|&s| b.objects.get(s).panel).collect();
    panels.sort_by_key(|p| p.y);
    assert_eq!(panels, [PanelPos { x: 4, y: 1 }, PanelPos { x: 4, y: 2 }, PanelPos { x: 4, y: 3 }]);
    // The side shots' row is kept in the attack's second parameter.
    assert_eq!(ai_mut(&mut b, p0).attack.params[1], 0xFF);
    // The side shots fly off the field.
    run_to(&mut b, p, &mut t, 12, 0);
    assert!(of_kind(&b, "projectile").is_empty());
    assert_eq!(b.objects.get(p1).hp, 999);
}

#[test]
fn a_charged_shot_waits_then_fires_the_charged_kind() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    // Charge fully (the test rules: 120 ticks at Charge 0), then release.
    for _ in 0..130 {
        tick(&mut b, p0, p1, keys::B);
    }
    let mut t = 0;
    tick(&mut b, p0, p1, 0);
    assert_eq!(b.objects.get(p0).action, 0x16);
    // Five ticks of waiting, the arm on tick 5, the shot on tick 6.
    run_to(&mut b, p, &mut t, 4, 0);
    assert_ne!(b.objects.get(p0).anim, 0x0E);
    run_to(&mut b, p, &mut t, 5, 0);
    assert_eq!(b.objects.get(p0).anim, 0x0E);
    assert!(of_kind(&b, "projectile").is_empty());
    run_to(&mut b, p, &mut t, 6, 0);
    let shot = of_kind(&b, "projectile")[0];
    assert_eq!(b.objects.get(shot).params[0], 6);
    let flash = b.objects.get(p0).related[0].expect("the muzzle flash");
    assert_eq!(b.objects.get(flash).params[0], 5);
    // (Attack + 1) * 10 damage, four ticks later.
    run_to(&mut b, p, &mut t, 10, 0);
    assert_eq!(b.objects.get(p1).hp, 990);
    let recovery = b.content.rules.buster_recovery(0, 2) as u32;
    run_to(&mut b, p, &mut t, 9 + recovery, 0);
    assert_eq!(b.objects.get(p0).action, 0x16);
    run_to(&mut b, p, &mut t, 10 + recovery, 0);
    assert_eq!(b.objects.get(p0).action, 8);
}

#[test]
fn a_stun_strike_slashes_a_paralyzed_navi_where_it_stands() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    let c = b.objects.get(p1).collision.unwrap();
    b.collision.get_mut(c).status_timers[crate::collision::timer::PARALYZE] = 100;
    let mut t = 0;
    use_chip(&mut b, p0, p1, testing::STUN_BLADE);
    assert_eq!(b.objects.get(p0).action, 0x49);
    // The slashes land on tick 10, on the target's own column.
    run_to(&mut b, p, &mut t, 10, 0);
    let slash = effects(&b, 0)[0];
    let (x, y) = crate::kinds::player::panel_coordinates(5, 2);
    let o = b.objects.get(slash);
    assert_eq!((o.params, o.pos.x, o.pos.y), ([0x16, 0, 2 + 7, 0], x, y));
    run_to(&mut b, p, &mut t, 11, 0);
    assert_eq!(b.objects.get(p1).hp, 920);
    // Idle on tick 28.
    run_to(&mut b, p, &mut t, 27, 0);
    assert_eq!(b.objects.get(p0).action, 0x49);
    run_to(&mut b, p, &mut t, 28, 0);
    assert_eq!(b.objects.get(p0).action, 8);
}

#[test]
fn a_buster_alias_fires_the_buster() {
    // Weapon routine 0x2E is the buster's.
    let (mut b, p0, p1) = fight_with(megaman_with(|s| s.weapons.buster = 0x2E));
    tick(&mut b, p0, p1, keys::B);
    tick(&mut b, p0, p1, 0);
    assert_eq!(b.objects.get(p0).action, 0x11);
}

#[test]
fn the_absorbed_obstacle_flies_at_the_enemy() {
    // Weapon routine 0x2B throws the last obstacle absorbed.
    let (mut b, p0, p1) = fight_with(megaman_with(|s| s.weapons.buster = 0x2B));
    let p = [p0, p1];
    ai_mut(&mut b, p0).absorbed.push(crate::actor::AbsorbedObstacle { kind: 1, anim: 2 });
    tick(&mut b, p0, p1, keys::B);
    let mut t = 0;
    tick(&mut b, p0, p1, 0);
    assert_eq!(b.objects.get(p0).action, 0x11);
    assert!(ai_mut(&mut b, p0).absorbed.is_empty());

    // Tick 2: it flies from the center of the panel in front, 12 pixels
    // up, drawn as the obstacle; a second arm in the first related slot.
    run_to(&mut b, p, &mut t, 2, 0);
    let thrown = of_kind(&b, "flying-shot");
    assert_eq!(thrown.len(), 1);
    let thrown = thrown[0];
    let o = b.objects.get(thrown);
    assert_eq!((o.params[0], o.anim, o.pos.z, o.pos.y), (6, 2, 0xC << 16, 28 << 16));
    let arm = b.objects.get(p0).related[0].expect("the second arm");
    assert_eq!(b.objects.get(arm).params[0], 6);
    // No shot before: no recovery; the navi idles once the arm is down.
    run_to(&mut b, p, &mut t, 6, 0);
    assert_eq!(b.objects.get(p0).action, 8);

    // 10 pixels a tick from x -20: over the enemy's panel at tick 7, and
    // the hit (200 damage) lands on the next.
    run_to(&mut b, p, &mut t, 7, 0);
    assert_eq!(b.objects.get(thrown).panel, PanelPos { x: 5, y: 2 });
    assert_eq!(b.objects.get(p1).hp, 1000);
    run_to(&mut b, p, &mut t, 8, 0);
    assert_eq!(b.objects.get(p1).hp, 800);
    assert_eq!(b.objects.get(thrown).state, state::DESTROY);
}

#[test]
fn a_throw_waits_the_last_shots_recovery() {
    let (mut b, p0, p1) = fight_with(megaman_with(|s| s.weapons.buster = 0x2B));
    let p = [p0, p1];
    // Nothing absorbed: the plain buster, and its recovery.
    tick(&mut b, p0, p1, keys::B);
    let mut t = 0;
    tick(&mut b, p0, p1, 0);
    let recovery = b.content.rules.buster_recovery(0, 2) as u32;
    run_to(&mut b, p, &mut t, 6 + recovery, 0);
    assert_eq!(b.objects.get(p0).action, 8);
    // Something else runs (a step), then the throw: it doesn't write the
    // recovery, and waits the shot's.
    tick(&mut b, p0, p1, keys::UP);
    let mut t = 0;
    run_to(&mut b, p, &mut t, 12, 0);
    assert_eq!(b.objects.get(p0).action, 8);
    ai_mut(&mut b, p0).absorbed.push(crate::actor::AbsorbedObstacle { kind: 0, anim: 0 });
    tick(&mut b, p0, p1, keys::B);
    let mut t = 0;
    tick(&mut b, p0, p1, 0);
    assert_eq!((b.objects.get(p0).action, ai_mut(&mut b, p0).attack.variant), (0x11, 2));
    run_to(&mut b, p, &mut t, 5 + recovery, 0);
    assert_eq!(b.objects.get(p0).action, 0x11);
    run_to(&mut b, p, &mut t, 6 + recovery, 0);
    assert_eq!(b.objects.get(p0).action, 8);
}

/// A projectile of kind `kind` from side 0's navi, on (x, y).
fn projectile(b: &mut Battle, owner: ObjectRef, kind: u8, x: u8, y: u8) -> ObjectRef {
    let r = crate::behavior::spawn_kind(b, "projectile", crate::object::Vec3 { x: 0, y: 0, z: 0x18 << 16 }, [kind, 0, 0, 0])
        .expect("a free attack slot");
    let o = b.objects.get_mut(r);
    (o.panel, o.damage, o.alliance, o.flip) = (PanelPos { x, y }, 10, 0, 0);
    o.related[0] = Some(owner);
    r
}

#[test]
fn projectile_kinds_change_the_panel_they_hit() {
    use crate::field::PanelType;
    // By kind (the test content's): 1 cracks, 2 breaks (cracks, with the
    // enemy on it), 3 lays grass, 4 a road away from the shooter's side.
    for (kind, panel) in [(1, PanelType::Cracked), (2, PanelType::Cracked), (3, PanelType::Grass), (4, PanelType::RoadRight)] {
        let (mut b, p0, p1) = fight();
        let p = [p0, p1];
        let shot = projectile(&mut b, p0, kind, 4, 2);
        let mut t = 0;
        run_to(&mut b, p, &mut t, 4, 0);
        assert_eq!(b.objects.get(p1).hp, 990, "kind {kind}");
        assert_eq!(b.field.panel(5, 2).unwrap().kind, panel, "kind {kind}");
        assert!(!b.objects.is_allocated(shot) || b.objects.get(shot).state == state::DESTROY);
    }
}

#[test]
fn a_bursting_projectile_bursts_on_the_enemy_and_a_missed_one_off_the_field() {
    // Kind 5 bursts: effects and a second hit around what it hit.
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    projectile(&mut b, p0, 5, 4, 2);
    let mut t = 0;
    run_to(&mut b, p, &mut t, 4, 0);
    assert!(b.objects.get(p1).hp < 1000);
    let effects = b.objects.in_order().filter(|&o| o.pool == Pool::Effect && b.objects.get(o).index == 0).count();
    assert!(effects > 0, "the burst's effects");
    // Missing (a row away), it bursts over the last two columns.
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    let shot = projectile(&mut b, p0, 5, 4, 1);
    let mut t = 0;
    // Off the field (7,1) on tick 6: effects on the valid panels of the
    // burst's region from (5,1).
    run_to(&mut b, p, &mut t, 5, 0);
    assert_eq!(b.objects.get(shot).panel, PanelPos { x: 6, y: 1 });
    run_to(&mut b, p, &mut t, 6, 0);
    assert_eq!(b.objects.get(shot).state, state::DESTROY);
    let effects = b.objects.in_order().filter(|&o| o.pool == Pool::Effect && b.objects.get(o).index == 0).count();
    assert_eq!(effects, 4, "the burst off the field");
}

#[test]
fn a_climbing_projectile_rises_a_pixel_a_panel() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    // Kind 7 is drawn and climbs; row 1 misses the enemy.
    let shot = projectile(&mut b, p0, 7, 2, 1);
    let mut t = 0;
    run_to(&mut b, p, &mut t, 1, 0);
    let o = b.objects.get(shot);
    assert_eq!((o.pos.z, o.flags & crate::object::flags::VISIBLE != 0), ((0x18 + 1) << 16, true));
    run_to(&mut b, p, &mut t, 2, 0);
    assert_eq!(b.objects.get(shot).pos.z, (0x18 + 2) << 16);
}
