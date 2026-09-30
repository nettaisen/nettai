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
    let mut hand = ChipHand::empty();
    hand.ids[0] = testing::FULL_GAUGE;
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
        s.weapons.buster = 0xFF;
        s.weapons.charge_shot = 0xFF;
        s.weapons.back_special = 0x2A;
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
    let absorbed = b.content.object_kind("absorbed-obstacle").unwrap();
    let flying = b.objects.in_order().filter(|&o| (o.pool, b.objects.get(o).index) == (absorbed.pool, absorbed.index)).count();
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
    let action = super::super::idle::weapon_routine(&mut b, p0, 0x1E);
    assert_eq!(action, 0x52);
    super::super::set_attack(&mut b, p0, action, 2);
    let mut t = 0;
    // The claw is up for 3 ticks; the first slash on the third: its effect
    // and a hit on the panel ahead, 50 damage and 10 per buster damage
    // point (1).
    let slashes = |b: &Battle| {
        let looks = b.objects.in_order().filter(|&o| (o.pool, b.objects.get(o).index) == (Pool::Effect, 0));
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
        s.weapons.buster = 0xFF;
        s.weapons.charge_shot = 0xFF;
        s.weapons.back_special = 0x2A;
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
    let action = super::super::idle::weapon_routine(&mut b, p0, 0x1E);
    super::super::set_attack(&mut b, p0, action, 2);
    tick(&mut b, p0, p1, 0);
    assert_rolls_back(&mut b, [p0, p1], 30, 0);
}

/// Use the instant chip `chip` from side 0's hand; returns once its
/// effect ran (the tick after the chip starts).
fn use_instant_chip(b: &mut Battle, p0: ObjectRef, p1: ObjectRef, chip: u16) {
    let mut hand = ChipHand::empty();
    hand.ids[0] = chip;
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
    let sparkle = b.content.object_kind("plus-sparkle").unwrap().clone();
    let s = b.objects.in_order().find(|&o| (o.pool, b.objects.get(o).index) == (sparkle.pool, sparkle.index));
    let s = s.expect("the sparkle");
    let (x, y) = crate::kinds::player::panel_coordinates(2, 2);
    let o = b.objects.get(s);
    assert_eq!((o.pos.x, o.pos.y, o.pos.z), (x + (4 << 16), y, 50 << 16));
    // Gone after its animation (12 ticks in the test content).
    for _ in 0..13 {
        tick(&mut b, p0, p1, 0);
    }
    assert!(!b.objects.is_allocated(s) || b.objects.get(s).index != sparkle.index);
    // From a special source the damage goes into the side's Atk+ bonus
    // instead.
    let (mut b, p0, p1) = fight();
    let mut hand = ChipHand::empty();
    (hand.ids[0], hand.damage[0]) = (testing::PLUS, 10);
    b.hands[0] = hand;
    tick(&mut b, p0, p1, keys::A);
    ai_mut(&mut b, p0).attack.special_source = 1;
    tick(&mut b, p0, p1, 0);
    assert_eq!(b.sides[0].plus_bonus, [10, 0]);
}

#[test]
fn buster_up_and_sync_trigger_change_the_navi() {
    // (A navi whose Beast Out is spent keeps its mood.)
    let (mut b, p0, p1) = fight_with(megaman_with(|s| s.beast_out_counter = 3));
    let attack = b.stats[0].attack;
    use_instant_chip(&mut b, p0, p1, testing::BUSTER_UP);
    assert_eq!(b.stats[0].attack, attack + 1);
    // At 9 or more it stays at 9.
    b.stats[0].attack = 9;
    use_instant_chip(&mut b, p0, p1, testing::BUSTER_UP);
    assert_eq!(b.stats[0].attack, 9);
    // SyncTrgr's effect alone (the Full Synchro aura that follows is the
    // framework's, not ported yet): the mood goes to the top.
    let hook = b.behaviors.hook(bn6_content_api::Hook::InstantChip(13)).expect("SyncTrgr's effect");
    let spec = bn6_content_api::InstantChipSpec::default();
    crate::behavior::call_hook(&mut b, hook, bn6_content_api::HookCall::InstantChip { user: p0, spec });
    assert_eq!(b.stats[0].mood, 0xFF);
}

/// Start weapon routine `routine` for side 0 as a `set_attack` of `kind`
/// (1 buster, 2 charged shot, 3 B+Back); returns the action it named.
fn start_weapon(b: &mut Battle, p0: ObjectRef, routine: u8, kind: u8) -> u8 {
    let action = super::super::idle::weapon_routine(b, p0, routine);
    super::super::set_attack(b, p0, action, kind);
    action
}

/// The objects of content kind `name`, in update order.
fn kind_objects(b: &Battle, name: &str) -> Vec<ObjectRef> {
    let k = b.content.object_kind(name).unwrap_or_else(|| panic!("no kind {name}"));
    let (pool, index) = (k.pool, k.index);
    b.objects.in_order().filter(|&o| (o.pool, b.objects.get(o).index) == (pool, index)).collect()
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
    assert_eq!(start_weapon(&mut b, p0, 0x10, 3), 0x1C);
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
    assert_eq!(start_weapon(&mut b, p0, 0x12, 2), 0x41);
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
    assert_eq!(start_weapon(&mut b, p0, 0x14, 2), 0x45);
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
    let mut zones: Vec<_> = kind_objects(&b, "hit-zone").iter().map(|&z| b.objects.get(z).panel.x).collect();
    zones.sort();
    assert_eq!(zones, [3, 4, 6]);
    assert_eq!(b.objects.get(p1).hp, 940);
    // The navi holds for 71 ticks; then the beam shuts.
    run_to(&mut b, p, &mut t, 71, 0);
    assert_eq!(b.objects.get(p0).action, 0x45);
    run_to(&mut b, p, &mut t, 72, 0);
    assert_eq!(b.objects.get(p0).action, 8);
    run_to(&mut b, p, &mut t, 80, 0);
    assert!(kind_objects(&b, "hit-zone").is_empty());
    assert!(kind_objects(&b, "erase-ray").is_empty());
}

#[test]
fn tomahawk_cross_charged_shot_swings_ahead() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    stand_at(&mut b, p0, 4);
    assert_eq!(start_weapon(&mut b, p0, 0x16, 2), 0x4A);
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
    assert_eq!(start_weapon(&mut b, p0, 0x19, 2), 0x4D);
    let mut t = 0;
    // It leaves the field on its 4th tick.
    run_to(&mut b, p, &mut t, 4, 0);
    assert_eq!(b.objects.get(p0).pos.x, 0xB4_0000);
    // After 30 ticks it comes up in front of the enemy (lock-on mode 0xC)
    // and drills from tick 40.
    run_to(&mut b, p, &mut t, 30, 0);
    assert_eq!(b.objects.get(p0).panel, PanelPos { x: 4, y: 2 });
    run_to(&mut b, p, &mut t, 41, 0);
    let mut drills: Vec<_> = kind_objects(&b, "drill-hit").iter().map(|&d| b.objects.get(d).panel.x).collect();
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
    assert!(kind_objects(&b, "drill-hit").is_empty());
}

#[test]
fn the_cross_charged_shots_roll_back() {
    for (routine, kind, from) in [(0x10, 3, 0), (0x12, 2, 8), (0x14, 2, 5), (0x16, 2, 20), (0x19, 2, 35)] {
        let (mut b, p0, p1) = fight();
        stand_at(&mut b, p0, if routine == 0x16 { 4 } else { 2 });
        start_weapon(&mut b, p0, routine, kind);
        for _ in 0..from {
            tick(&mut b, p0, p1, 0);
        }
        assert_rolls_back(&mut b, [p0, p1], 60, 0);
    }
}
