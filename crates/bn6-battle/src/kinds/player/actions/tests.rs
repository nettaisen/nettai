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
    // A link battle on a plain field with the usual two-navi list.
    let mut setup = testing::round_setup(testing::LINK_BATTLE, megaman());
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
    assert_eq!(following(&b, p0)[0], (Pool::Actor, crate::kinds::attachment::INDEX));
    let gun = ai_mut(&mut b, p0).overlay.unwrap();
    let (at, g) = (b.objects.get(p0).pos, b.objects.get(gun).pos);
    let point = testing::GUN_POINT;
    assert_eq!((g.x - at.x, g.y - at.y, g.z - at.z), ((point.x as i32) << 16, 0, (point.y as i32) << 16));

    // Tick 7: the beam, two panels ahead.
    run_to(&mut b, p, &mut t, 7, 0);
    let beam = b.objects.get(p0).related[0].unwrap();
    assert_eq!((beam.pool, b.objects.get(beam).index), (Pool::Effect, crate::kinds::sun_beam::INDEX));
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
