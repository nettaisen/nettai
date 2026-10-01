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
        weapons: NaviWeapons {
            buster: testing::weapon(0),
            charge_shot: testing::weapon(1),
            back_special: None,
            a_charge: None,
            mode9_a: None,
            ..Default::default()
        },
        ..testing::megaman_on(&testing::content())
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

fn following(b: &Battle, r: ObjectRef) -> Vec<&str> {
    b.objects.in_order().skip_while(|&o| o != r).skip(1).map(|o| b.kind_key(o)).collect()
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

    let mut hand = ChipHand::empty(&b.content);
    hand.ids[0] = Some(testing::chip_in(&b.content, testing::SUN_GUN_3));
    let firing = b.content.chip(testing::chip_in(&b.content, testing::SUN_GUN_3)).gun_del_sol.unwrap().firing_ticks as u32;
    b.hands[0] = hand;
    let mut t = 0;
    tick(&mut b, p0, p1, keys::A);
    assert_eq!(b.objects.get(p0).action, 0x37);
    assert_eq!(b.hands[0].cursor, 1);
    assert_eq!(ai_mut(&mut b, p0).requests & request::CHIP, 0);

    // Tick 1: the gun comes out, at its owner's attach point.
    run_to(&mut b, p, &mut t, 1, keys::A);
    assert_eq!(b.objects.get(p0).anim, 0x0A);
    assert_eq!(following(&b, p0)[0], "attachment");
    let gun = ai_mut(&mut b, p0).overlay.unwrap();
    let (at, g) = (b.objects.get(p0).pos, b.objects.get(gun).pos);
    let point = testing::GUN_POINT;
    assert_eq!((g.x - at.x, g.y - at.y, g.z - at.z), ((point.x as i32) << 16, 0, (point.y as i32) << 16));

    // Tick 7: the beam, two panels ahead.
    run_to(&mut b, p, &mut t, 7, 0);
    let beam = b.objects.get(p0).related[0].unwrap();
    assert_eq!(b.kind_key(beam), "sun-beam");
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
    assert_eq!(runs(&b, p0), "megaman/blank-shot/action");

    // Tick 1: the arm is up.
    run_to(&mut b, p, &mut t, 1, 0);
    assert_eq!(b.objects.get(p0).anim, 0x0E);
    assert_ne!(f1_of(&b, p0) & f1::USING_ACTION, 0);
    let arm = ai_mut(&mut b, p0).overlay.expect("the buster arm");
    // The arm is the pack data's attachment row 6.
    assert_eq!(b.kind_key(arm), "attachment");
    assert_eq!(b.objects.sprite(arm).id, Some(b.content.attachment(6).sprite));

    // Five ticks up, then the recovery by the open panels from its own
    // (its body is off the field while it updates) to the enemy's:
    // rules.buster_recovery[Rapid 0][3].
    let recovery = b.content.rules.buster_recovery(0, 3) as u32;
    run_to(&mut b, p, &mut t, 5 + recovery, 0);
    assert_eq!(runs(&b, p0), "megaman/blank-shot/action");
    run_to(&mut b, p, &mut t, 6 + recovery, 0);
    assert_eq!(b.objects.get(p0).action, 8);
    assert_eq!(ai_mut(&mut b, p0).overlay, None);
    // Nothing was fired.
    assert_eq!(b.objects.get(p1).hp, 1000);
}

#[test]
fn dustcross_charged_shot_rolls_junk_into_the_enemy() {
    // Weapon routine 0x28 as the charged shot.
    let (mut b, p0, p1) = fight_with(megaman_with(|s| s.weapons.charge_shot = testing::weapon(0x28)));
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
    let ball = b.objects.in_order().find(|&o| b.kind_key(o) == "dust-ball");
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
    b.objects.in_order().find(|&o| b.kind_key(o) == name)
}

/// Give side `side` the chip `chip` as the next in its hand, with its
/// damage.
fn hand_with(b: &mut Battle, side: usize, chip: crate::content::ChipId) {
    let mut hand = ChipHand::empty(&b.content);
    hand.ids[0] = Some(testing::chip_in(&b.content, chip));
    hand.damage[0] = b.content.chip(testing::chip_in(&b.content, chip)).damage;
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
    assert_eq!(following(&b, p0)[0], "engine/effect");
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
    let chip = testing::chip_in(&b.content, chip);
    use_chip_handle(b, p0, p1, chip);
}

/// The same with the chip by handle.
fn use_chip_handle(b: &mut Battle, p0: ObjectRef, p1: ObjectRef, chip: bn6_content_api::ChipHandle) {
    let mut hand = ChipHand::empty(&b.content);
    hand.ids[0] = Some(chip);
    hand.damage[0] = b.content.chip(chip).damage;
    b.hands[0] = hand;
    tick(b, p0, p1, keys::A);
}

/// Whether `r` is an attachment showing the pack data's attachment row
/// `row` (its sprite).
fn shows_row(b: &Battle, r: ObjectRef, row: u8) -> bool {
    b.kind_key(r) == "attachment" && b.objects.sprite(r).id == Some(b.content.attachment(row).sprite)
}

/// The effect objects (effect #0) and afterimages (effect #0x28) there are.
fn effects(b: &Battle, key: &str) -> Vec<ObjectRef> {
    b.objects.in_order().filter(|&o| b.kind_key(o) == key).collect()
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
    let first = effects(&b, "engine/afterimage");
    assert_eq!(first.len(), 1);
    assert_eq!(b.objects.get(first[0]).pos.x, crate::kinds::player::panel_coordinates(2, 2).0);

    // Tick 3: the swing, with the blade.
    run_to(&mut b, p, &mut t, 3, 0);
    assert_eq!(b.objects.get(p0).anim, 5);
    // The sword's blade (lib/swords/parts: the sword sprite).
    let blade = b.objects.get(p0).related[0].expect("the blade");
    assert_eq!(b.kind_key(blade), "attachment");
    assert_eq!(b.objects.sprite(blade).id, Some(bn6_content_api::SpriteId { category: 0x0C, index: 0x00 }));
    // Tick 8: two more afterimages: the navi's and the blade's.
    run_to(&mut b, p, &mut t, 8, 0);
    assert_eq!(effects(&b, "engine/afterimage").len(), 3);
    // Tick 12: the slash on the column ahead hits the target at (5,2) on
    // its next update, and its effect shows.
    run_to(&mut b, p, &mut t, 11, 0);
    assert!(effects(&b, "engine/effect").is_empty());
    run_to(&mut b, p, &mut t, 12, 0);
    // The swords' wide slash (the sword-slash sprite's first animation).
    let look = b.content.effect(b.objects.get(effects(&b, "engine/effect")[0]).params[0]);
    assert_eq!((look.sprite, look.anim), (bn6_content_api::SpriteId { category: 0x0C, index: 0x14 }, 0));
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
    assert!(effects(&b, "engine/afterimage").is_empty());
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
    b.objects.in_order().filter(|&o| b.kind_key(o) == name).collect()
}

#[test]
fn a_buster_shot_flies_a_panel_every_two_ticks_and_hits() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    // B fires on release (the navi has a charged shot).
    tick(&mut b, p0, p1, keys::B);
    let mut t = 0;
    tick(&mut b, p0, p1, 0);
    assert_eq!(runs(&b, p0), "megaman/buster/shot");

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
    assert!(shows_row(&b, flash, 5), "the muzzle flash");

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
    assert_eq!(runs(&b, p0), "megaman/buster/shot");
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
    assert_eq!(runs(&b, p0), "megaman/buster/shot");
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
    assert_eq!(runs(&b, p0), "megaman/charged-shot/action");
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
    assert!(shows_row(&b, flash, 5), "the muzzle flash");
    // (Attack + 1) * 10 damage, four ticks later.
    run_to(&mut b, p, &mut t, 10, 0);
    assert_eq!(b.objects.get(p1).hp, 990);
    let recovery = b.content.rules.buster_recovery(0, 2) as u32;
    run_to(&mut b, p, &mut t, 9 + recovery, 0);
    assert_eq!(runs(&b, p0), "megaman/charged-shot/action");
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
    // The slash (the swords' wide slash, in AssnSwrd's colours: palette
    // offset 2 + 7) over the target's panel.
    let slash = effects(&b, "engine/effect")[0];
    let (x, y) = crate::kinds::player::panel_coordinates(5, 2);
    let o = b.objects.get(slash);
    assert_eq!((&o.params[1..], o.pos.x, o.pos.y), (&[0, 2 + 7, 0][..], x, y));
    let wide = b.content.effect(o.params[0]);
    assert_eq!((wide.sprite, wide.anim, wide.palette), (bn6_content_api::SpriteId { category: 0x0C, index: 0x14 }, 0, 0));
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
    let (mut b, p0, p1) = fight_with(megaman_with(|s| s.weapons.buster = testing::weapon(0x2E)));
    tick(&mut b, p0, p1, keys::B);
    tick(&mut b, p0, p1, 0);
    assert_eq!(runs(&b, p0), "megaman/buster/shot");
}

#[test]
fn the_absorbed_obstacle_flies_at_the_enemy() {
    // Weapon routine 0x2B throws the last obstacle absorbed.
    let (mut b, p0, p1) = fight_with(megaman_with(|s| s.weapons.buster = testing::weapon(0x2B)));
    let p = [p0, p1];
    ai_mut(&mut b, p0).absorbed.push(crate::actor::AbsorbedObstacle { kind: 1, anim: 2 });
    tick(&mut b, p0, p1, keys::B);
    let mut t = 0;
    tick(&mut b, p0, p1, 0);
    assert_eq!(runs(&b, p0), "megaman/buster/shot");
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
    assert!(shows_row(&b, arm, 6), "the arm");
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
    let (mut b, p0, p1) = fight_with(megaman_with(|s| s.weapons.buster = testing::weapon(0x2B)));
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
    assert_eq!((runs(&b, p0), ai_mut(&mut b, p0).attack.variant), ("megaman/buster/shot".to_string(), 2));
    run_to(&mut b, p, &mut t, 5 + recovery, 0);
    assert_eq!(runs(&b, p0), "megaman/buster/shot");
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
    let effects = b.objects.in_order().filter(|&o| b.kind_key(o) == "engine/effect").count();
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
    let effects = b.objects.in_order().filter(|&o| b.kind_key(o) == "engine/effect").count();
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

// ---- Form weapons ---------------------------------------------------------------

/// Weapon `routine` from idle, run until the navi idles again (at most
/// `ticks`); copies of the battle taken along the way play on exactly as
/// it does. Returns the ticks it took.
fn run_weapon(b: &mut Battle, p: [ObjectRef; 2], routine: u8, ticks: u32) -> u32 {
    start_weapon(b, p[0], routine);
    let mut digests = Vec::new();
    let mut copies = Vec::new();
    let mut t = 0;
    while t < ticks {
        t += 1;
        if t % 7 == 3 {
            copies.push((digests.len(), b.clone()));
        }
        tick(b, p[0], p[1], 0);
        digests.push(b.digest());
        if b.objects.get(p[0]).action == 8 {
            break;
        }
    }
    for (from, mut copy) in copies {
        for want in &digests[from..] {
            tick(&mut copy, p[0], p[1], 0);
            assert_eq!(copy.digest(), *want, "a copy of the battle went its own way");
        }
    }
    t
}

#[test]
fn groundcross_beast_dash_runs_the_enemy_over_and_lands_back() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    assert_eq!(start_weapon(&mut b, p0, 0x17), 0x1A);
    let mut t = 0;
    // Six ticks of wind-up, then the dash, invulnerable, with its two hits.
    run_to(&mut b, p, &mut t, 6, 0);
    assert_eq!(ai_mut(&mut b, p0).attack.step, 0);
    run_to(&mut b, p, &mut t, 7, 0);
    assert_eq!(of_kind(&b, "dash-hit").len(), 2);
    assert_ne!(f1_of(&b, p0) & f1::INVULNERABLE, 0);
    // Both hits run through the enemy: 90 damage and 20 per buster Attack
    // point (1 here) each.
    run_to(&mut b, p, &mut t, 27, 0);
    assert_eq!(b.objects.get(p1).hp, 1000 - 2 * 110);
    assert!(of_kind(&b, "dash-hit").is_empty());
    // Off the field, then back on its panel.
    run_to(&mut b, p, &mut t, 28, 0);
    assert_eq!(b.objects.get(p0).pos.x, 0xDC << 16);
    run_to(&mut b, p, &mut t, 37, 0);
    let o = b.objects.get(p0);
    assert_eq!((o.action, o.panel, o.pos.z), (8, PanelPos { x: 2, y: 2 }, 0));
    assert_eq!(f1_of(&b, p0) & f1::INVULNERABLE, 0);
    assert_eq!(b.field.panel(2, 2).unwrap().reserver, None);
}

#[test]
fn erasecross_beast_drop_falls_on_the_enemy() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    assert_eq!(start_weapon(&mut b, p0, 0x15), 0x46);
    let mut t = 0;
    // The arms go up for their animation; the drop at the next's end.
    run_to(&mut b, p, &mut t, 9, 0);
    let drop = of_kind(&b, "erase-drop");
    assert_eq!(drop.len(), 1);
    assert_eq!(b.objects.get(drop[0]).panel, PanelPos { x: 5, y: 2 });
    // It lands 3 ticks later: 70 damage and 30 per buster Attack point.
    run_to(&mut b, p, &mut t, 13, 0);
    assert_eq!(b.objects.get(p1).hp, 900);
    run_to(&mut b, p, &mut t, 30, 0);
    assert_eq!(b.objects.get(p0).action, 8);
    assert!(of_kind(&b, "erase-drop").is_empty());
}

#[test]
fn chargecross_beast_wave_rolls_through_the_enemy() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    assert_eq!(start_weapon(&mut b, p0, 0x1C), 0x4F);
    let mut t = 0;
    run_to(&mut b, p, &mut t, 30, 0);
    assert!(of_kind(&b, "charge-wave").is_empty());
    run_to(&mut b, p, &mut t, 31, 0);
    assert_eq!(of_kind(&b, "charge-wave").len(), 1);
    // 70 damage and 30 per buster Attack point; afterimages behind it.
    run_to(&mut b, p, &mut t, 45, 0);
    assert_eq!(b.objects.get(p1).hp, 900);
    let afterimages = b.objects.in_order().filter(|&o| b.kind_key(o) == "engine/afterimage").count();
    assert_eq!(afterimages, 2);
    run_to(&mut b, p, &mut t, 82, 0);
    assert_eq!(b.objects.get(p0).action, 8);
}

#[test]
fn dustcross_beast_scatter_throws_at_six_panels_the_enemy_first() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    assert_eq!(start_weapon(&mut b, p0, 0x1D), 0x50);
    let mut t = 0;
    // Up for 17 ticks, then six throws 11 ticks apart.
    run_to(&mut b, p, &mut t, 17, 0);
    assert!(b.objects.get(p0).pos.z > 0);
    let mut thrown = 0;
    let mut last = Vec::new();
    while t < 77 {
        let next = t + 1;
        run_to(&mut b, p, &mut t, next, 0);
        let now = of_kind(&b, "junk-shot");
        thrown += now.iter().filter(|o| !last.contains(*o)).count();
        last = now;
    }
    assert_eq!(thrown, 6);
    // The enemy's panel is always among them: 80 damage and 20 per buster
    // Attack point.
    run_to(&mut b, p, &mut t, 90, 0);
    assert_eq!(b.objects.get(p1).hp, 900);
    run_to(&mut b, p, &mut t, 108, 0);
    let o = b.objects.get(p0);
    assert_eq!((o.action, o.pos.z), (8, 0));
}

#[test]
fn slashcross_beast_lunge_strikes_from_beside_its_target() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    crate::kinds::lockon_marker::spawn(&mut b, p0);
    tick(&mut b, p0, p1, 0);
    tick(&mut b, p0, p1, 0);
    assert_eq!(start_weapon(&mut b, p0, 0x1A), 0x4C);
    let mut t = 0;
    // Mode 2 (the target in its row): the panel before it.
    run_to(&mut b, p, &mut t, 3, 0);
    assert_eq!(b.objects.get(p0).panel, PanelPos { x: 4, y: 2 });
    assert_eq!(b.field.panel(2, 2).unwrap().reserver, Some(p0));
    // The slash and the hit: 50 damage and 30 per buster Attack point
    // each; the struck body flashes.
    run_to(&mut b, p, &mut t, 16, 0);
    assert_eq!(b.objects.get(p1).hp, 1000 - 2 * 80);
    assert_eq!(of_kind(&b, "hit-flash").len(), 1);
    run_to(&mut b, p, &mut t, 37, 0);
    assert_eq!(b.objects.get(p0).panel, PanelPos { x: 2, y: 2 });
    run_to(&mut b, p, &mut t, 40, 0);
    assert_eq!(b.objects.get(p0).action, 8);
}

#[test]
fn chargecross_charged_shot_tackles_the_enemy() {
    let (mut b, p0, p1) = fight_with(megaman_with(|s| s.weapons.charge_shot = testing::weapon(0x27)));
    let p = [p0, p1];
    for _ in 0..130 {
        tick(&mut b, p0, p1, keys::B);
    }
    let mut t = 0;
    tick(&mut b, p0, p1, 0);
    assert_eq!(b.objects.get(p0).action, 0x56);
    // Nine ticks of wind-up, then the charge with its hit.
    run_to(&mut b, p, &mut t, 10, 0);
    assert_eq!(of_kind(&b, "dash-hit").len(), 1);
    assert_ne!(f1_of(&b, p0) & f1::INVULNERABLE, 0);
    // The hit strikes (30 damage and 20 per buster Attack point) and is
    // gone; the navi slows down, then drops back onto its panel.
    run_to(&mut b, p, &mut t, 40, 0);
    assert_eq!(b.objects.get(p1).hp, 950);
    assert!(of_kind(&b, "dash-hit").is_empty());
    run_to(&mut b, p, &mut t, 60, 0);
    let o = b.objects.get(p0);
    assert_eq!((o.action, o.panel), (8, PanelPos { x: 2, y: 2 }));
    assert_eq!(f1_of(&b, p0) & f1::INVULNERABLE, 0);
}

#[test]
fn form_weapons_roll_back() {
    for routine in [0x15, 0x17, 0x1C, 0x1D] {
        let (mut b, p0, p1) = fight();
        let t = run_weapon(&mut b, [p0, p1], routine, 200);
        assert!(t < 200, "weapon {routine:#x} never ended");
    }
}

/// Start weapon routine `routine` for side 0 as a `set_attack` of `kind`
/// (1 buster, 2 charged shot, 3 B+Back); returns the action it named.
fn start_weapon_as(b: &mut Battle, p0: ObjectRef, routine: u8, kind: u8) -> u8 {
    let action = super::super::idle::weapon_routine(b, p0, b.content.weapon_numbered(routine));
    super::super::set_attack(b, p0, action, kind);
    action.number
}

/// The objects of content kind `name`, in update order.
fn kind_objects(b: &Battle, name: &str) -> Vec<ObjectRef> {
    assert!(b.content.defs.kind_by_key(name).is_some(), "no kind {name}");
    b.objects.in_order().filter(|&o| b.kind_key(o) == name).collect()
}

/// Put side 0's navi on (x, 2).
fn stand_at(b: &mut Battle, p0: ObjectRef, x: u8) {
    let (px, py) = crate::kinds::player::panel_coordinates(x, 2);
    let o = b.objects.get_mut(p0);
    (o.panel, o.future_panel) = (PanelPos { x, y: 2 }, PanelPos { x, y: 2 });
    (o.pos.x, o.pos.y) = (px, py);
}

#[test]
fn tengu_cross_back_special_blows_a_gust_down_each_row() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    assert_eq!(start_weapon_as(&mut b, p0, 0x10, 3), 0x1C);
    assert_eq!(ai_mut(&mut b, p0).attack.variant, 0x14);
    let mut t = 0;
    // Its first tick: a gust in each row from the far column, blowing back
    // toward the navi.
    run_to(&mut b, p, &mut t, 1, 0);
    // (Each runs right after its spawner, so the last spawned first.)
    let gusts = kind_objects(&b, "gust");
    let panels: Vec<_> = gusts.iter().map(|&g| b.objects.get(g).panel).collect();
    assert_eq!(panels, [PanelPos { x: 6, y: 3 }, PanelPos { x: 6, y: 2 }, PanelPos { x: 6, y: 1 }]);
    assert_eq!(b.objects.get(gusts[0]).vel.x, -(10 << 16));
    // The navi waits 8 ticks, then idles with the B+Back cooldown.
    run_to(&mut b, p, &mut t, 8, 0);
    assert_eq!(b.objects.get(p0).action, 0x1C);
    run_to(&mut b, p, &mut t, 9, 0);
    assert_eq!(b.objects.get(p0).action, 8);
    assert_eq!(ai_mut(&mut b, p0).back_special_cooldown, 0x27);
    // The gusts end at the field's edge at the latest.
    run_to(&mut b, p, &mut t, 30, 0);
    assert!(kind_objects(&b, "gust").is_empty());
}

#[test]
fn slash_cross_charged_shot_sends_a_sword_wave() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    assert_eq!(start_weapon_as(&mut b, p0, 0x12, 2), 0x41);
    // 60 damage and 20 per buster damage point (1).
    assert_eq!(ai_mut(&mut b, p0).attack.damage, 80);
    let mut t = 0;
    run_to(&mut b, p, &mut t, 11, 0);
    assert!(kind_objects(&b, "sword-wave").is_empty());
    // The slash starts on tick 3; the wave goes out 9 ticks later, from
    // the panel in front.
    run_to(&mut b, p, &mut t, 12, 0);
    let waves = kind_objects(&b, "sword-wave");
    assert_eq!(waves.len(), 1);
    assert_eq!(b.objects.get(waves[0]).panel, PanelPos { x: 3, y: 2 });
    run_to(&mut b, p, &mut t, 29, 0);
    assert_eq!(b.objects.get(p1).hp, 920);
    assert_eq!(b.objects.get(p0).action, 0x41);
    run_to(&mut b, p, &mut t, 30, 0);
    assert_eq!(b.objects.get(p0).action, 8);
}

#[test]
fn erase_cross_charged_shot_beams_the_row_while_the_navi_holds() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    assert_eq!(start_weapon_as(&mut b, p0, 0x14, 2), 0x45);
    // Outside EraseCross, the beam's second kind, 4 pixels lower.
    assert_eq!(ai_mut(&mut b, p0).attack.variant, 1);
    let mut t = 0;
    run_to(&mut b, p, &mut t, 1, 0);
    let rays = kind_objects(&b, "erase-ray");
    assert_eq!(rays.len(), 1);
    assert_eq!(b.objects.get(rays[0]).pos.z, -(4 << 16));
    // After its opening animation, a hit zone on each panel from its own
    // to the field's edge; the one on the enemy hits once and ends.
    run_to(&mut b, p, &mut t, 10, 0);
    let mut zones: Vec<_> = kind_objects(&b, "thunder-column").iter().map(|&z| b.objects.get(z).panel.x).collect();
    zones.sort();
    assert_eq!(zones, [3, 4, 6]);
    assert_eq!(b.objects.get(p1).hp, 940);
    // The navi holds for 71 ticks; then the beam shuts.
    run_to(&mut b, p, &mut t, 71, 0);
    assert_eq!(b.objects.get(p0).action, 0x45);
    run_to(&mut b, p, &mut t, 72, 0);
    assert_eq!(b.objects.get(p0).action, 8);
    run_to(&mut b, p, &mut t, 80, 0);
    assert!(kind_objects(&b, "thunder-column").is_empty());
    assert!(kind_objects(&b, "erase-ray").is_empty());
}

#[test]
fn tomahawk_cross_charged_shot_swings_ahead() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    stand_at(&mut b, p0, 4);
    assert_eq!(start_weapon_as(&mut b, p0, 0x16, 2), 0x4A);
    let mut t = 0;
    run_to(&mut b, p, &mut t, 15, 0);
    assert_eq!(b.objects.get(p0).anim, 0x12);
    run_to(&mut b, p, &mut t, 16, 0);
    assert_eq!(b.objects.get(p0).anim, 0x13);
    // The hit on the swing's 10th tick, 40 damage and 20 per buster damage
    // point.
    run_to(&mut b, p, &mut t, 25, 0);
    assert_eq!(b.objects.get(p1).hp, 1000);
    run_to(&mut b, p, &mut t, 28, 0);
    assert_eq!(b.objects.get(p1).hp, 940);
    run_to(&mut b, p, &mut t, 43, 0);
    assert_eq!(b.objects.get(p0).action, 0x4A);
    run_to(&mut b, p, &mut t, 44, 0);
    assert_eq!(b.objects.get(p0).action, 8);
}

#[test]
fn ground_cross_charged_shot_burrows_to_the_enemy_and_drills() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    assert_eq!(start_weapon_as(&mut b, p0, 0x19, 2), 0x4D);
    let mut t = 0;
    // It leaves the field on its 4th tick.
    run_to(&mut b, p, &mut t, 4, 0);
    assert_eq!(b.objects.get(p0).pos.x, 0xB4_0000);
    // After 30 ticks it comes up in front of the enemy (lock-on mode 0xC)
    // and drills from tick 40.
    run_to(&mut b, p, &mut t, 30, 0);
    assert_eq!(b.objects.get(p0).panel, PanelPos { x: 4, y: 2 });
    run_to(&mut b, p, &mut t, 41, 0);
    let mut drills: Vec<_> = kind_objects(&b, "drill").iter().map(|&d| b.objects.get(d).panel.x).collect();
    drills.sort();
    assert_eq!(drills, [5, 6]);
    run_to(&mut b, p, &mut t, 73, 0);
    assert!(b.objects.get(p1).hp < 1000);
    // Then it sinks, and is back on its own panel on tick 86.
    run_to(&mut b, p, &mut t, 85, 0);
    assert_eq!(b.objects.get(p0).action, 0x4D);
    run_to(&mut b, p, &mut t, 86, 0);
    let o = b.objects.get(p0);
    assert_eq!((o.action, o.panel), (8, PanelPos { x: 2, y: 2 }));
    assert!(kind_objects(&b, "drill").is_empty());
}

#[test]
fn the_cross_charged_shots_roll_back() {
    for (routine, kind, from) in [(0x10, 3, 0), (0x12, 2, 8), (0x14, 2, 5), (0x16, 2, 20), (0x19, 2, 35)] {
        let (mut b, p0, p1) = fight();
        stand_at(&mut b, p0, if routine == 0x16 { 4 } else { 2 });
        start_weapon_as(&mut b, p0, routine, kind);
        for _ in 0..from {
            tick(&mut b, p0, p1, 0);
        }
        assert_rolls_back(&mut b, [p0, p1], 60, 0);
    }
}

// ---- The Beast forms' weapons and the instant chips ------------------------------

/// Start weapon routine `routine` for `r` as a charged chip does
/// (`sub_800FB54`): the routine's setup, then its action. Returns the
/// action.
fn start_weapon(b: &mut Battle, r: ObjectRef, routine: u8) -> u8 {
    ai_mut(b, r).attack.charged = 0;
    let action = crate::kinds::player::idle::weapon_routine(b, r, b.content.weapon_numbered(routine));
    crate::kinds::player::set_attack(b, r, action, 2);
    action.number
}

/// What navi `r` runs, as the traces name it: a content action's key, else
/// the ruleset's action number.
fn runs(b: &Battle, r: ObjectRef) -> String {
    match super::super::running_content_action(b, r) {
        Some(h) => b.content.defs.action(h).key.clone(),
        None => format!("{:#04x}", b.objects.get(r).action),
    }
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
        assert!(shows_row(&b, arm, 6), "the arm");
        // (The shot is the buster's projectile, which isn't content yet.)
    }
}

#[test]
fn dustcross_beast_throws_its_newest_obstacle_or_fires_the_beast_buster() {
    let (mut b, p0, _) = fight();
    assert_eq!(start_weapon(&mut b, p0, 0x2C), 0x1E);
    let actor = b.objects.get(p0).actor.unwrap();
    b.actors.get_mut(actor).absorbed.push(crate::actor::AbsorbedObstacle { kind: 2, anim: 1 });
    start_weapon(&mut b, p0, 0x2C);
    assert_eq!(runs(&b, p0), "megaman/buster/shot");
    let a = &ai_mut(&mut b, p0).attack;
    assert_eq!((a.damage, a.variant, a.marker), (200, 2, 0x12));
    assert!(b.actors.get(actor).absorbed.is_empty());
}

/// Two navis idle and fighting on the test content's battle settings
/// `settings`, with both navis' stats `stats`.
fn fight_on(settings: u8, stats: NaviStats) -> (Battle, ObjectRef, ObjectRef) {
    let mut setup = testing::round_setup(settings, stats);
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

#[test]
fn an_instant_chip_runs_its_effect_once_and_idles() {
    let (mut b, p0, p1) = fight();
    let mut hand = ChipHand::empty(&b.content);
    hand.ids[0] = b.content.defs.chip_by_key(testing::FULL_CUST);
    b.hands[0] = hand;
    b.gauge.value = 0;
    tick(&mut b, p0, p1, keys::A);
    assert_eq!(b.objects.get(p0).action, 0x1C);
    assert_eq!(b.gauge.value, 0);
    // Its first tick: the effect (the gauge fills) and back to idle, with
    // the chip's lockout.
    tick(&mut b, p0, p1, 0);
    assert_eq!(b.gauge.value, crate::hud::CustomGauge::FULL);
    assert_eq!(b.objects.get(p0).action, 8);
    // (Applied, and counted down once already.)
    assert_eq!(ai_mut(&mut b, p0).lockout, 19);
}

#[test]
fn dustcross_back_special_pulls_the_rocks_in() {
    // Weapon routine 0x2A as the B+Back special, and no buster, so B does
    // nothing else.
    let stats = megaman_with(|s| {
        s.weapons.buster = testing::weapon(0xFF);
        s.weapons.charge_shot = testing::weapon(0xFF);
        s.weapons.back_special = testing::weapon(0x2A);
    });
    let (mut b, p0, p1) = fight_on(testing::ROCK_BATTLE, stats);
    let p = [p0, p1];
    let rocks: Vec<_> = b.objects.in_order().filter(|r| r.pool == Pool::Attack).collect();
    assert_eq!(rocks.len(), 2);
    tick(&mut b, p0, p1, keys::B);
    let mut t = 0;
    tick(&mut b, p0, p1, keys::B | keys::LEFT);
    assert_eq!(b.objects.get(p0).action, 0x58);

    // Tick 1: the pose and the vortex (effect 0x63), kept alive.
    run_to(&mut b, p, &mut t, 1, keys::B);
    assert_eq!(b.objects.get(p0).anim, 0x17);
    assert_ne!(f1_of(&b, p0) & (f1::USING_ACTION | f1::MOVING), 0);
    let vortex = b.objects.in_order().find(|&o| o.pool == Pool::Effect && b.objects.get(o).params[0] == 0x63);
    let vortex = vortex.expect("the vortex");
    assert_eq!(b.objects.get(vortex).timer, 2);

    // Tick 10: the pull. The rocks go on their next update, each leaving
    // an absorbed obstacle that flies to the navi.
    run_to(&mut b, p, &mut t, 11, 0);
    let flying = b.objects.in_order().filter(|&o| b.kind_key(o) == "absorbed-obstacle").count();
    assert_eq!(flying, 2);

    // They arrive 9 ticks later, while the navi still absorbs, and join
    // its list; the navi idles 11 ticks after the pull.
    run_to(&mut b, p, &mut t, 20, 0);
    assert_eq!(b.objects.get(p0).action, 0x58);
    run_to(&mut b, p, &mut t, 21, 0);
    assert_eq!(b.objects.get(p0).action, 8);
    let actor = b.objects.get(p0).actor.unwrap();
    assert_eq!(b.actors.get(actor).absorbed.len(), 2);
    // The B+Back cooldown (40 ticks, counted down once already).
    assert_eq!(ai_mut(&mut b, p0).back_special_cooldown, 0x27);
}

#[test]
fn the_beast_claw_slashes_the_panel_ahead_twice() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    // The navi right in front of the enemy at (5,2).
    let (x, y) = crate::kinds::player::panel_coordinates(4, 2);
    let o = b.objects.get_mut(p0);
    (o.panel, o.future_panel) = (PanelPos { x: 4, y: 2 }, PanelPos { x: 4, y: 2 });
    (o.pos.x, o.pos.y) = (x, y);
    let action = super::super::idle::weapon_routine(&mut b, p0, testing::weapon(0x1E).unwrap());
    assert_eq!(action.number, 0x52);
    super::super::set_attack(&mut b, p0, action, 2);
    let mut t = 0;
    // The claw is up for 3 ticks; the first slash on the third: its effect
    // and a hit on the panel ahead, 50 damage and 10 per buster damage
    // point (1).
    let slashes = |b: &Battle| {
        let looks = b.objects.in_order().filter(|&o| b.kind_key(o) == "engine/effect");
        looks.map(|o| b.objects.get(o).params[0]).filter(|&l| l == 0x3A || l == 0x39).collect::<Vec<_>>()
    };
    run_to(&mut b, p, &mut t, 2, 0);
    assert_eq!(b.objects.get(p0).anim, 0x0C);
    assert_eq!(slashes(&b), Vec::<u8>::new());
    run_to(&mut b, p, &mut t, 3, 0);
    assert_eq!(slashes(&b), [0x3A]);
    assert_eq!(ai_mut(&mut b, p0).attack.damage, 60);
    run_to(&mut b, p, &mut t, 5, 0);
    assert_eq!(b.objects.get(p1).hp, 940);
    // 12 ticks later the second slash, and 12 after it, idle.
    run_to(&mut b, p, &mut t, 18, 0);
    assert_eq!(b.objects.get(p0).action, 0x52);
    run_to(&mut b, p, &mut t, 29, 0);
    assert_eq!(b.objects.get(p0).action, 0x52);
    run_to(&mut b, p, &mut t, 30, 0);
    assert_eq!(b.objects.get(p0).action, 8);
    assert_eq!(f1_of(&b, p0) & f1::USING_ACTION, 0);
}

/// Run `b` and a copy of it taken now for `ticks` ticks with side 0
/// holding `held`: both must go the same way.
fn assert_rolls_back(b: &mut Battle, p: [ObjectRef; 2], ticks: u32, held: u16) {
    let mut copy = b.clone();
    for i in 0..ticks {
        tick(b, p[0], p[1], held);
        tick(&mut copy, p[0], p[1], held);
        assert_eq!(b.digest(), copy.digest(), "the copy went its own way on tick {i}");
    }
}

#[test]
fn absorbing_and_the_claw_roll_back() {
    let stats = megaman_with(|s| {
        s.weapons.buster = testing::weapon(0xFF);
        s.weapons.charge_shot = testing::weapon(0xFF);
        s.weapons.back_special = testing::weapon(0x2A);
    });
    let (mut b, p0, p1) = fight_on(testing::ROCK_BATTLE, stats);
    tick(&mut b, p0, p1, keys::B);
    tick(&mut b, p0, p1, keys::B | keys::LEFT);
    for _ in 0..5 {
        tick(&mut b, p0, p1, 0);
    }
    // Mid-pull, then through the flight and the absorbing.
    assert_rolls_back(&mut b, [p0, p1], 20, 0);

    let (mut b, p0, p1) = fight();
    let action = super::super::idle::weapon_routine(&mut b, p0, testing::weapon(0x1E).unwrap());
    super::super::set_attack(&mut b, p0, action, 2);
    tick(&mut b, p0, p1, 0);
    assert_rolls_back(&mut b, [p0, p1], 30, 0);
}

/// Use the instant chip `chip` from side 0's hand; returns once its
/// effect ran (the tick after the chip starts).
fn use_instant_chip(b: &mut Battle, p0: ObjectRef, p1: ObjectRef, chip: u16) {
    let chip = testing::chip_in(&b.content, chip);
    use_instant_chip_handle(b, p0, p1, chip);
}

/// The same with the chip by handle (one content defines).
fn use_instant_chip_handle(b: &mut Battle, p0: ObjectRef, p1: ObjectRef, chip: bn6_content_api::ChipHandle) {
    let mut hand = ChipHand::empty(&b.content);
    hand.ids[0] = Some(chip);
    b.hands[0] = hand;
    tick(b, p0, p1, keys::A);
    assert_eq!(b.objects.get(p0).action, 0x1C);
    tick(b, p0, p1, 0);
    assert_eq!(b.objects.get(p0).action, 8);
}

#[test]
fn a_plus_chip_on_its_own_raises_a_sparkle() {
    let (mut b, p0, p1) = fight();
    use_instant_chip(&mut b, p0, p1, testing::PLUS);
    // 4 pixels toward the enemy from the navi's panel, 48 up, and rising
    // (its first rise at once).
    let s = b.objects.in_order().find(|&o| b.kind_key(o) == "rising-bubble");
    let s = s.expect("the sparkle");
    let (x, y) = crate::kinds::player::panel_coordinates(2, 2);
    let o = b.objects.get(s);
    assert_eq!((o.pos.x, o.pos.y, o.pos.z), (x + (4 << 16), y, 50 << 16));
    // Gone after its animation (12 ticks in the test content).
    for _ in 0..13 {
        tick(&mut b, p0, p1, 0);
    }
    assert!(!b.objects.is_allocated(s) || b.kind_key(s) != "rising-bubble");
    // From a special source the damage goes into the side's Atk+ bonus
    // instead.
    let (mut b, p0, p1) = fight();
    let mut hand = ChipHand::empty(&b.content);
    (hand.ids[0], hand.damage[0]) = (Some(testing::chip_in(&b.content, testing::PLUS)), 10);
    b.hands[0] = hand;
    tick(&mut b, p0, p1, keys::A);
    ai_mut(&mut b, p0).attack.special_source = 1;
    tick(&mut b, p0, p1, 0);
    assert_eq!((b.sides[0].special_attack_bonus, b.sides[0].special_navi_bonus), (10, 0));
}

#[test]
fn the_plus_chips_content_defines_raise_their_bonus() {
    // Atk+10 and Navi+20 are definitions whose hooks name their bonus (the
    // records' first parameter): alone, the sparkle; from a special source,
    // their damage into the side's attack or navi bonus.
    let (mut b, p0, p1) = fight();
    let atk = b.content.defs.chip_by_key(testing::ATTACK_10).unwrap();
    use_instant_chip_handle(&mut b, p0, p1, atk);
    assert!(b.objects.in_order().any(|o| b.kind_key(o) == "rising-bubble"), "the sparkle");
    for (key, bonus) in [(testing::ATTACK_10, (10, 0)), (testing::NAVI_20, (0, 20))] {
        let (mut b, p0, p1) = fight();
        let chip = b.content.defs.chip_by_key(key).unwrap();
        let mut hand = ChipHand::empty(&b.content);
        (hand.ids[0], hand.damage[0]) = (Some(chip), b.content.chip(chip).damage);
        b.hands[0] = hand;
        tick(&mut b, p0, p1, keys::A);
        ai_mut(&mut b, p0).attack.special_source = 1;
        tick(&mut b, p0, p1, 0);
        assert_eq!((b.sides[0].special_attack_bonus, b.sides[0].special_navi_bonus), bonus, "{key}");
    }
}

#[test]
fn buster_up_and_sync_trigger_change_the_navi() {
    // (A navi whose Beast Out is spent keeps its mood.)
    let (mut b, p0, p1) = fight_with(megaman_with(|s| s.beast_out_counter = 3));
    let attack = b.stats[0].attack;
    let buster_up = b.content.defs.chip_by_key(testing::BUSTER_UP).unwrap();
    use_instant_chip_handle(&mut b, p0, p1, buster_up);
    assert_eq!(b.stats[0].attack, attack + 1);
    // At 9 or more it stays at 9 (once its 20-tick lockout is over).
    for _ in 0..20 {
        tick(&mut b, p0, p1, 0);
    }
    b.stats[0].attack = 9;
    use_instant_chip_handle(&mut b, p0, p1, buster_up);
    assert_eq!(b.stats[0].attack, 9);
    // SyncTrgr's effect alone (the Full Synchro aura that follows is the
    // framework's, not ported yet): the mood goes to the top.
    let sync = b.content.defs.chip_by_key(testing::SYNC_TRIGGER).unwrap();
    let Some(crate::content::ChipUsage::Instant(hook)) = b.content.defs.chip(sync).usage else {
        panic!("SyncTrgr is an instant chip")
    };
    let spec = bn6_content_api::InstantChipSpec::default();
    crate::behavior::call_hook(&mut b, hook, bn6_content_api::HookCall::InstantChip { user: p0, spec });
    assert_eq!(b.stats[0].mood, 0xFF);
}

/// The objects of content kind `name` alive in `b`.
fn count_kind(b: &Battle, name: &str) -> usize {
    assert!(b.content.defs.kind_by_key(name).is_some(), "no kind {name}");
    b.objects.in_order().filter(|&o| b.kind_key(o) == name).count()
}

#[test]
fn spawning_instant_chips_run_their_objects_and_roll_back() {
    // Each effect's object appears the tick the chip's effect runs, plays
    // out, rolls back at any point, and is gone within 200 ticks.
    // BN6's definitions, and the numbered chips that reach the records'
    // shims (FireHit's, FlmHook's).
    let chips = [
        (testing::defined_chip(testing::BOOMER), "boomerang"),
        (testing::defined_chip(testing::LANCE), "lance/lance"),
        (testing::chip_handle(testing::FIST), "firehit/fist"),
        (testing::defined_chip(testing::SAND_WORM), "sandwrm/worm"),
        (testing::chip_handle(testing::FLAME_HOOK), "flmhook/hook"),
        (testing::defined_chip(testing::JUSTICE_ONE), "justcone/strike"),
        (testing::defined_chip(testing::GOLEM_HIT), "golmhit/golem"),
    ];
    for (chip, name) in chips {
        let (mut b, p0, p1) = fight();
        if name == "sandwrm/worm" {
            // The worm comes out behind the enemy, on a panel with the flag
            // the test content's panel types don't give.
            let mut hand = ChipHand::empty(&b.content);
            hand.ids[0] = Some(chip);
            b.hands[0] = hand;
            tick(&mut b, p0, p1, keys::A);
            b.field.panel_mut(6, 2).unwrap().flags |= 0x1_0000;
            tick(&mut b, p0, p1, 0);
        } else {
            use_instant_chip_handle(&mut b, p0, p1, chip);
        }
        assert!(count_kind(&b, name) > 0, "{name} didn't appear");
        for _ in 0..8 {
            assert_rolls_back(&mut b, [p0, p1], 5, 0);
            for _ in 0..7 {
                tick(&mut b, p0, p1, 0);
            }
        }
        for _ in 0..200 {
            tick(&mut b, p0, p1, 0);
        }
        assert_eq!(count_kind(&b, name), 0, "{name} didn't go");
    }
}

#[test]
fn lances_thrust_from_the_far_column() {
    let (mut b, p0, p1) = fight();
    let lance = testing::defined_chip(testing::LANCE);
    use_instant_chip_handle(&mut b, p0, p1, lance);
    // Three lances on column 6, one per row, 64 pixels out and one 8-pixel
    // step back already (the init runs the first tick).

    let lances: Vec<ObjectRef> =
        b.objects.in_order().filter(|&o| b.kind_key(o) == "lance/lance").collect();
    let mut rows: Vec<u8> = lances.iter().map(|&l| b.objects.get(l).panel.y).collect();
    rows.sort();
    assert_eq!(rows, [1, 2, 3]);
    let (x, _) = crate::kinds::player::panel_coordinates(6, 1);
    assert!(lances.iter().all(|&l| b.objects.get(l).pos.x == x + (56 << 16)));
    // Column 6 is empty: the enemy at (5,2) is untouched.
    for _ in 0..30 {
        tick(&mut b, p0, p1, 0);
    }
    assert_eq!(b.objects.get(p1).hp, 1000);
}

#[test]
fn the_tomahawk_throw_sends_two_tomahawks() {
    // The boomerangs out, by their sprites and rows.
    let tomahawks = |b: &Battle| {
        let t = b.objects.in_order().filter(|&o| b.kind_key(o) == "boomerang");
        t.map(|o| (b.objects.sprite(o).id, b.objects.get(o).panel.y)).collect::<Vec<_>>()
    };
    let start = || {
        let (mut b, p0, p1) = fight();
        let action = super::super::idle::weapon_routine(&mut b, p0, testing::weapon(0x1B).unwrap());
        assert_eq!(action.number, 0x4E);
        super::super::set_attack(&mut b, p0, action, 2);
        (b, p0, p1)
    };
    let (mut b, p0, p1) = start();
    // 50 damage and 20 per buster damage point (1), Wood.
    assert_eq!(ai_mut(&mut b, p0).attack.damage, 70);
    let mut t = 0;
    let mut first = None;
    while t < 40 {
        let next = t + 1;
        run_to(&mut b, [p0, p1], &mut t, next, 0);
        if first.is_none() && !tomahawks(&b).is_empty() {
            first = Some(t);
        }
    }
    let first = first.expect("no tomahawk");
    // Two tomahawks (the boomerang's tomahawk variant, on its own sprite);
    // the second 10 ticks after the first, on row 3.
    let (mut b, p0, p1) = start();
    let mut t = 0;
    run_to(&mut b, [p0, p1], &mut t, first + 9, 0);
    assert_eq!(tomahawks(&b).len(), 1);
    run_to(&mut b, [p0, p1], &mut t, first + 10, 0);
    let mut both = tomahawks(&b);
    both.sort();
    let tomahawk = Some(b.content.assets.sprites["boomerang-tomahawk"]);
    assert_eq!(both, [(tomahawk, 1), (tomahawk, 3)]);
    assert_rolls_back(&mut b, [p0, p1], 20, 0);
    // 96 ticks into the swing, idle.
    run_to(&mut b, [p0, p1], &mut t, first + 120, 0);
    assert_eq!(b.objects.get(p0).action, 8);
}

/// Use `chip` from side 0's hand as a charged chip (the request a full A
/// charge raises), with the A-charge routine `routine`; the test content's
/// base form charges no chip, so the charge itself is skipped.
fn use_charged_chip(b: &mut Battle, p0: ObjectRef, routine: u8, chip: bn6_content_api::ChipHandle) {
    let mut hand = ChipHand::empty(&b.content);
    hand.ids[0] = Some(chip);
    b.hands[0] = hand;
    let routine = testing::weapon_in(&b.content, routine);
    let a = ai_mut(b, p0);
    a.a_charge = routine;
    a.requests |= request::CHARGED_CHIP;
    super::super::chip_use::use_chip(b, p0);
}

#[test]
fn a_charged_chip_with_a_bonus_routine_is_used_charged() {
    // An A-charge routine that is the chip's charged use (ElecCross's).
    let (mut b, p0, _) = fight();
    let buster_up = testing::defined_chip(testing::BUSTER_UP);
    use_charged_chip(&mut b, p0, 0x0D, buster_up);
    assert_eq!(b.objects.get(p0).action, 0x1C);
    assert_eq!(ai_mut(&mut b, p0).attack.charged, 1);
    // Without a routine: the chip family's register (Plus, 4).
    let (mut b, p0, _) = fight();
    use_charged_chip(&mut b, p0, 0xFF, buster_up);
    assert_eq!(ai_mut(&mut b, p0).attack.charged, 4);
}

#[test]
fn a_cross_change_lands_changes_and_settles_while_paused() {
    use crate::actor::status;
    // Side 0 changes to the navi it already is (the test content has only
    // MegaMan): kept, then taken back from the kept stats.
    let (mut b, p0, p1) = fight();
    b.turn_transforms[0].cross_change = Some(b.content.navi_numbered(crate::setup::Navi(0)));
    super::cross_change::request_change(&mut b, p0);
    b.paused = true;
    b.objects.get_mut(p0).hp = 700;
    // The request starts the pause action, then 4 ticks landing, the
    // change, 21 ticks settling.
    tick(&mut b, p0, p1, 0);
    assert_eq!(b.objects.get(p0).action, 0x1C);
    assert_ne!(ai_mut(&mut b, p0).status & status::CHANGING_CROSS, 0);
    for _ in 0..4 {
        tick(&mut b, p0, p1, 0);
    }
    assert_eq!(ai_mut(&mut b, p0).attack.step, 4);
    tick(&mut b, p0, p1, 0);
    assert_eq!(ai_mut(&mut b, p0).attack.step, 8);
    // The kept stats carry the HP the navi had.
    assert_eq!((b.cross_stats[0].hp, b.objects.get(p0).hp), (700, 700));
    for _ in 0..20 {
        tick(&mut b, p0, p1, 0);
    }
    assert_eq!(b.objects.get(p0).action, 0x1C);
    tick(&mut b, p0, p1, 0);
    assert_eq!(b.objects.get(p0).action, 8);
    let st = ai_mut(&mut b, p0).status;
    assert_eq!((st & status::CROSSED != 0, st & status::CHANGING_CROSS), (true, 0));
}

// ---- Content model v2: definitions in a battle ---------------------------------------------

/// A fight on the test content with the test pack's definitions
/// (`testing::with_test_pack`).
fn fight_on_test_pack() -> (Battle, ObjectRef, ObjectRef) {
    let content = std::sync::Arc::new(testing::with_test_pack());
    // Its own handles: the pack's definitions come first.
    let stats = NaviStats {
        weapons: NaviWeapons {
            buster: testing::weapon_in(&content, 0),
            charge_shot: testing::weapon_in(&content, 1),
            ..megaman().weapons
        },
        ..megaman()
    };
    let mut setup = testing::round_setup(testing::LINK_BATTLE, stats);
    setup.content = content.hash();
    setup.settings.stage = content.stage_numbered(testing::LINK_BATTLE);
    setup.settings.effects = 0xE8C;
    let mut b = Battle::new(setup, content);
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

/// A content object's state field.
fn state_field(b: &Battle, r: ObjectRef, name: &str) -> i64 {
    let crate::kinds::Vars::Content(s) = &b.objects.get(r).vars else { panic!("{r:?} has no content state") };
    let schema = b.content.defs.schema(s.id());
    s.get(schema, schema.index_of(name).expect("the field")).load().int().expect("an integer")
}

/// How many ticks the navi stays in the content action `action` from now.
fn ticks_in(b: &mut Battle, p0: ObjectRef, p1: ObjectRef, action: bn6_content_api::ActionHandle) -> u32 {
    let mut n = 0;
    while super::super::running_content_action(b, p0) == Some(action) {
        tick(b, p0, p1, 0);
        n += 1;
        assert!(n < 100, "the action never ended");
    }
    n
}

#[test]
fn a_defined_kind_runs_by_its_handle_with_its_state() {
    let (mut b, p0, p1) = fight_on_test_pack();
    let r = crate::behavior::spawn_kind(&mut b, "test/ticker", crate::object::Vec3::default(), [0; 4]).unwrap();
    // It has no number: the object is of its kind, in its pool.
    let kind = b.content.defs.kind_by_key("test/ticker").unwrap();
    assert_eq!((r.pool, b.objects.get(r).kind), (Pool::Effect, kind));
    assert_eq!(b.content.defs.kind(kind).slot, None);
    for n in 1..=5 {
        tick(&mut b, p0, p1, 0);
        assert_eq!(state_field(&b, r, "ticks"), n);
    }
    assert_rolls_back(&mut b, [p0, p1], 10, 0);
    for _ in 0..30 {
        tick(&mut b, p0, p1, 0);
    }
    assert!(!b.objects.in_order().any(|o| o == r), "it left after 30 ticks");
}

#[test]
fn a_weapon_names_a_defined_action_which_runs_by_handle() {
    let (mut b, p0, p1) = fight_on_test_pack();
    let shot = b.content.defs.weapon_by_key(testing::TICK_SHOT).unwrap();
    let action = super::super::idle::weapon_routine(&mut b, p0, shot);
    assert_eq!(action.number, super::super::CONTENT_ACTION);
    let h = action.content.expect("a defined action");
    assert_eq!(b.content.defs.action(h).key, "test/tick-shot/shot");
    super::super::set_attack(&mut b, p0, action, 1);
    // The navi runs it by handle, under no number of its own; its update
    // runs: it stands for 12 ticks.
    assert_eq!(b.objects.get(p0).action, super::super::CONTENT_ACTION);
    assert_eq!(ticks_in(&mut b, p0, p1, h), 12);
}

#[test]
fn chips_of_a_series_run_their_own_actions() {
    // Each chip runs the action its definition composed, with its own
    // length.
    let (mut b, p0, p1) = fight_on_test_pack();
    let defs = &b.content.defs;
    let [ticker1, ticker2] = [testing::TICKER_1, testing::TICKER_2].map(|key| defs.chip_by_key(key).unwrap());
    let [one, two] = [ticker1, ticker2].map(|h| match defs.chip(h).usage.unwrap() {
        crate::content::ChipUsage::Action(h) => h,
        u => panic!("{u:?}"),
    });
    assert_ne!(one, two);
    assert_eq!(defs.action(one).schema, defs.action(two).schema, "one builder, one state layout");
    use_chip_handle(&mut b, p0, p1, ticker1);
    assert_eq!(b.objects.get(p0).action, super::super::CONTENT_ACTION);
    let first = ticks_in(&mut b, p0, p1, one);
    let (mut b, p0, p1) = fight_on_test_pack();
    use_chip_handle(&mut b, p0, p1, ticker2);
    assert_rolls_back(&mut b, [p0, p1], 3, 0);
    let second = ticks_in(&mut b, p0, p1, two) + 3;
    assert_eq!(second - first, 3, "Ticker2 stands 9 ticks to Ticker1's 6");
}

#[test]
fn a_legacy_marker_gives_a_definitions_record_its_bytes() {
    // The transitional marker: what reads a chip's subtype and parameters
    // besides its own action (SlashCross's charged slash reads a sword's)
    // finds them in the attack, from the record.
    let (mut b, p0, p1) = fight_on_test_pack();
    let defs = &b.content.defs;
    let [ticker1, ticker2] = [testing::TICKER_1, testing::TICKER_2].map(|key| defs.chip_by_key(key).unwrap());
    let bytes = |b: &Battle, h| {
        let c = b.content.chip(h);
        (c.subtype, c.params)
    };
    assert_eq!(bytes(&b, ticker1), (0, [0; 4]));
    assert_eq!(bytes(&b, ticker2), (7, [1, 2, 0, 0]));
    use_chip_handle(&mut b, p0, p1, ticker2);
    let a = &ai_mut(&mut b, p0).attack;
    assert_eq!((a.variant, a.params), (7, [1, 2, 0, 0]));
}

// ---- Content model v2: the v2 API (step 4) ---------------------------------------------------

/// The objects of the kind content defines as `key`.
fn defined(b: &Battle, key: &str) -> Vec<ObjectRef> {
    let kind = b.content.defs.kind_by_key(key).unwrap_or_else(|| panic!("no kind {key}"));
    b.objects.in_order().filter(|&o| b.objects.get(o).kind == kind).collect()
}

/// A content object's reference field, as the definition it holds.
fn state_def(b: &Battle, r: ObjectRef, name: &str) -> Option<(bn6_content_api::Registry, u16)> {
    let crate::kinds::Vars::Content(s) = &b.objects.get(r).vars else { panic!("{r:?} has no content state") };
    let schema = b.content.defs.schema(s.id());
    s.get(schema, schema.index_of(name).expect("the field")).load().def()
}

#[test]
fn a_kind_spawns_by_definition_and_its_state_holds_definitions() {
    use bn6_content_api::Registry;
    let (mut b, p0, p1) = fight_on_test_pack();
    let launcher = crate::behavior::spawn_kind(&mut b, "test/launcher", crate::object::Vec3::default(), [0; 4]).unwrap();
    tick(&mut b, p0, p1, 0);
    let [ticker] = defined(&b, "test/ticker")[..] else { panic!("one ticker") };
    let defs = &b.content.defs;
    // Its variant is the launcher's record, its parent the launcher's kind.
    let (registry, h) = state_def(&b, ticker, "variant").expect("a variant");
    assert_eq!(registry, Registry::Record);
    assert_eq!(defs.records[h as usize].record_type, "ticker-variant");
    assert_eq!(state_def(&b, ticker, "parent"), Some((Registry::Kind, defs.kind_by_key("test/launcher").unwrap().0)));
    // The effect is the definition's look, and the sound the asset's.
    let burst = b.objects.in_order().find(|&o| defs.engine_kind(b.objects.get(o).kind) == Some(crate::kinds::EngineKind::Effect));
    let burst = burst.expect("the burst");
    assert_eq!(b.objects.sprite(burst).id, Some(bn6_content_api::SpriteId { category: 0x14, index: 0 }));
    assert!(b.sound_cues().contains(&crate::sound::SoundCue::Effect(crate::sound::SoundId(0x1A6))));
    // The collision types and region are the definitions'.
    let c = b.collision.get(b.objects.get(launcher).collision.expect("a collision"));
    assert_eq!(c.self_flags & 0xFFFE_FFFF, 0x80000088);
    assert_eq!(c.target_flags, 0x15800000);
    let wide: Vec<(i8, i8)> = b.content.region(c.region).iter().map(|p| (p.dx, p.dy)).collect();
    assert_eq!(wide, [(1, -1), (1, 0), (1, 1)]);
    // The variant's lifetime (5) ends it.
    for _ in 0..4 {
        tick(&mut b, p0, p1, 0);
    }
    assert!(defined(&b, "test/ticker").is_empty(), "the short variant's ticker left after 5 ticks");
}

#[test]
#[should_panic(expected = "expected a record:ticker-variant, got a record:other-variant")]
fn a_reference_field_refuses_a_record_of_another_type() {
    let (mut b, p0, p1) = fight_on_test_pack();
    crate::behavior::spawn_kind(&mut b, "test/misuse", crate::object::Vec3::default(), [0; 4]).unwrap();
    tick(&mut b, p0, p1, 0);
}

#[test]
fn an_action_starts_the_next_by_definition() {
    // Ticker3's action stands 3 ticks, then starts its `next` (2 ticks) with
    // `set_attack`; each checks the navi runs it (`navi_action`).
    let (mut b, p0, p1) = fight_on_test_pack();
    let defs = &b.content.defs;
    let ticker3 = defs.chip_by_key(testing::TICKER_3).unwrap();
    let Some(crate::content::ChipUsage::Action(first)) = defs.chip(ticker3).usage else { panic!("an action") };
    let next = bn6_content_api::ActionHandle(
        defs.actions.iter().position(|a| a.key == "test/ticker3/action/args/next").expect("a derived key") as u16,
    );
    use_chip_handle(&mut b, p0, p1, ticker3);
    assert_eq!(ticks_in(&mut b, p0, p1, first), 3);
    assert_rolls_back(&mut b, [p0, p1], 1, 0);
    assert_eq!(ticks_in(&mut b, p0, p1, next) + 1, 2);
}

#[test]
fn the_ruleset_starts_a_role_action() {
    // A caught hit starts AntiDmg's counter: the role content fills.
    let (mut b, p0, p1) = fight_on_test_pack();
    let role = b.content.defs.roles.actions.anti_damage_counter.expect("the test pack fills it");
    assert_eq!(b.content.defs.action(role).key, "test/anti-damage-counter");
    ai_mut(&mut b, p0).requests |= request::ANTI_DAMAGE_TRIGGERED;
    super::reactive::counter(&mut b, p0);
    assert_eq!(super::super::running_content_action(&b, p0), Some(role));
    assert_eq!(b.objects.get(p0).action, super::super::CONTENT_ACTION);
    use bn6_content_api::CoreApi;
    assert_eq!(b.navi_action(p0).unwrap(), bn6_content_api::NaviAction::Content(role.0));
    // It ran its first tick with the counter's set-up; three more.
    assert_eq!(ticks_in(&mut b, p0, p1, role), 3);
}

#[test]
fn a_forced_charged_shot_starts_its_role() {
    // The request that starts the charged shot from idle without its
    // weapon's setup starts the role's action.
    let (mut b, p0, p1) = fight_on_test_pack();
    let role = b.content.defs.roles.actions.forced_charged_shot.expect("the test pack fills it");
    ai_mut(&mut b, p0).requests |= request::FORCED_CHARGED_SHOT;
    tick(&mut b, p0, p1, 0);
    assert_eq!(super::super::running_content_action(&b, p0), Some(role));
    assert_eq!(runs(&b, p0), "test/forced-charged-shot");
}

#[test]
fn a_weapon_definition_takes_its_legacy_routines() {
    // The buster's definition is what every routine number aliasing it
    // names (the pack's forms name weapons by number), and the ruleset's
    // numeric logic reads its first; the charged shot's charge times are
    // its own, with Charge 5 read on into the next row.
    let c = testing::content();
    let buster = c.defs.weapon_by_key("megaman/buster").expect("the buster's definition");
    for n in [0x00, 0x2E, 0x82] {
        assert_eq!(c.weapon_numbered(n), buster, "routine {n:#x}");
    }
    assert_eq!(c.weapon_number(buster), Some(0));
    let charged = c.defs.weapon(c.weapon_numbered(1));
    assert_eq!((charged.key.as_str(), &charged.charge_ticks[..]), ("megaman/charged-shot", &[100, 90, 80, 70, 60, 180][..]));
    // A routine nobody implements is still a placeholder of its own.
    assert_eq!(c.defs.weapon(c.weapon_numbered(0x81)).key, "v1/weapon-81");
}

#[test]
#[should_panic(expected = "the role actions.body_guard_counter is not filled")]
fn an_unfilled_role_names_itself() {
    let (mut b, p0, _) = fight_on_test_pack();
    ai_mut(&mut b, p0).requests |= request::BODY_GUARD_TRIGGERED;
    super::reactive::counter(&mut b, p0);
}
