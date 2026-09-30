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

/// The objects of content kind `name` alive in `b`.
fn count_kind(b: &Battle, name: &str) -> usize {
    let k = b.content.object_kind(name).unwrap_or_else(|| panic!("no kind {name}"));
    b.objects.in_order().filter(|&o| (o.pool, b.objects.get(o).index) == (k.pool, k.index)).count()
}

#[test]
fn spawning_instant_chips_run_their_objects_and_roll_back() {
    // Each effect's object appears the tick the chip's effect runs, plays
    // out, rolls back at any point, and is gone within 200 ticks.
    let chips = [
        (testing::BOOMERANG, "boomerang"),
        (testing::LANCE, "lance"),
        (testing::FIST, "fire-hit"),
        (testing::WORM, "sand-worm"),
        (testing::FLAME_HOOK, "flame-hook"),
        (testing::JUSTICE, "justice-one"),
        (testing::GOLEM, "golem"),
    ];
    for (chip, name) in chips {
        let (mut b, p0, p1) = fight();
        if chip == testing::WORM {
            // The worm comes out behind the enemy, on a panel with the flag
            // the test content's panel types don't give.
            let mut hand = ChipHand::empty();
            hand.ids[0] = chip;
            b.hands[0] = hand;
            tick(&mut b, p0, p1, keys::A);
            b.field.panel_mut(6, 2).unwrap().flags |= 0x1_0000;
            tick(&mut b, p0, p1, 0);
        } else {
            use_instant_chip(&mut b, p0, p1, chip);
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
    use_instant_chip(&mut b, p0, p1, testing::LANCE);
    // Three lances on column 6, one per row, 64 pixels out and one 8-pixel
    // step back already (the init runs the first tick).
    let lance = b.content.object_kind("lance").unwrap().clone();
    let lances: Vec<ObjectRef> =
        b.objects.in_order().filter(|&o| (o.pool, b.objects.get(o).index) == (lance.pool, lance.index)).collect();
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
    let tomahawks = |b: &Battle| {
        let k = b.content.object_kind("boomerang").unwrap();
        let t = b.objects.in_order().filter(|&o| (o.pool, b.objects.get(o).index) == (k.pool, k.index));
        t.map(|o| (b.objects.get(o).params[0], b.objects.get(o).panel.y)).collect::<Vec<_>>()
    };
    let start = || {
        let (mut b, p0, p1) = fight();
        let action = super::super::idle::weapon_routine(&mut b, p0, 0x1B);
        assert_eq!(action, 0x4E);
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
    // Two tomahawks (boomerang kind 4); the second 10 ticks after the
    // first, on row 3.
    let (mut b, p0, p1) = start();
    let mut t = 0;
    run_to(&mut b, [p0, p1], &mut t, first + 9, 0);
    assert_eq!(tomahawks(&b).len(), 1);
    run_to(&mut b, [p0, p1], &mut t, first + 10, 0);
    let mut both = tomahawks(&b);
    both.sort();
    assert_eq!(both, [(4, 1), (4, 3)]);
    assert_rolls_back(&mut b, [p0, p1], 20, 0);
    // 96 ticks into the swing, idle.
    run_to(&mut b, [p0, p1], &mut t, first + 120, 0);
    assert_eq!(b.objects.get(p0).action, 8);
}

/// Use `chip` from side 0's hand as a charged chip (the request a full A
/// charge raises), with the A-charge routine `routine`; the test content's
/// base form charges no chip, so the charge itself is skipped.
fn use_charged_chip(b: &mut Battle, p0: ObjectRef, routine: u8, chip: u16) {
    let mut hand = ChipHand::empty();
    hand.ids[0] = chip;
    b.hands[0] = hand;
    let a = ai_mut(b, p0);
    a.a_charge = routine;
    a.requests |= request::CHARGED_CHIP;
    super::super::chip_use::use_chip(b, p0);
}

#[test]
fn a_charged_chip_with_a_bonus_routine_is_used_charged() {
    // An A-charge routine that is the chip's charged use (ElecCross's).
    let (mut b, p0, _) = fight();
    use_charged_chip(&mut b, p0, 0x0D, testing::BUSTER_UP);
    assert_eq!(b.objects.get(p0).action, 0x1C);
    assert_eq!(ai_mut(&mut b, p0).attack.charged, 1);
    // Without a routine: the chip family's register (Plus, 4).
    let (mut b, p0, _) = fight();
    use_charged_chip(&mut b, p0, 0xFF, testing::BUSTER_UP);
    assert_eq!(ai_mut(&mut b, p0).attack.charged, 4);
}

#[test]
fn ground_cross_charge_drops_rocks_on_the_enemy() {
    let (mut b, p0, p1) = fight();
    let p = [p0, p1];
    use_charged_chip(&mut b, p0, 0x18, testing::BUSTER_UP);
    // The rocks come first, then the chip, used with the routine's result
    // (2: there is an enemy) as its charge.
    assert_eq!(b.objects.get(p0).action, 0x1C);
    assert_eq!(ai_mut(&mut b, p0).attack.charged, 2);
    let falling_rock = b.content.object_kind("falling-rock").unwrap().clone();
    let rocks: Vec<_> =
        b.objects.in_order().filter(|&o| (o.pool, b.objects.get(o).index) == (falling_rock.pool, falling_rock.index)).collect();
    assert_eq!(rocks.len(), 3);
    // One falls on the enemy's panel: 30 damage and 20 per buster damage
    // point (1). They fall from 104 pixels, faster each tick.
    assert!(rocks.iter().any(|&r| b.objects.get(r).panel == PanelPos { x: 5, y: 2 }));
    assert_eq!(b.objects.get(rocks[0]).damage, 50);
    let mut t = 0;
    run_to(&mut b, p, &mut t, 40, 0);
    assert_eq!(b.objects.get(p1).hp, 950);
    // They broke into chunks, thrown up and gone after a blink.
    let chunks = b.objects.in_order().filter(|&o| (o.pool, b.objects.get(o).index) == (Pool::Effect, 9)).count();
    assert!(chunks > 0);
    run_to(&mut b, p, &mut t, 120, 0);
    let chunks = b.objects.in_order().filter(|&o| (o.pool, b.objects.get(o).index) == (Pool::Effect, 9)).count();
    assert_eq!(chunks, 0);
    assert!(rocks.iter().all(|&r| !b.objects.is_allocated(r) || b.objects.get(r).index != falling_rock.index));
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
