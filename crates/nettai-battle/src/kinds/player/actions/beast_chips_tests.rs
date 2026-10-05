//! The Gregar and Falzar chips (chips/gregar, chips/falzar: the Japanese
//! ROMs' routines, docs/engine/beast-chips.md) on EXE6's definitions over the
//! test content's made-up assets: each phase's timing, where the summons and
//! their attacks are, what they hit for, how the screen and the HUD go, and
//! that a copy of the battle plays on the same. No Japanese recording checks
//! them yet; the timings here are the routines' as docs/engine reads them.

use super::super::ai_mut;
use crate::battle::{Battle, FadeMode, battle_flags};
use crate::content::testing;
use crate::hand::ChipHand;
use crate::input::keys;
use crate::kinds::player::panel_coordinates;
use crate::object::{ObjectRef, PanelPos, Vec3, flags};
use crate::setup::{NaviStats, NaviWeapons};
use crate::sound::{SoundCue, SoundId};

/// A plain MegaMan with 1000 HP.
fn megaman() -> NaviStats {
    NaviStats {
        hp: 1000,
        max_hp: 1000,
        max_base_hp: 1000,
        mood: 0x80,
        weapons: NaviWeapons {
            buster: testing::weapon("megaman/buster"),
            charge_shot: testing::weapon("megaman/charged-shot"),
            ..Default::default()
        },
        ..testing::megaman_on(&testing::content())
    }
}

/// Two navis idle and fighting on `stage` (the link battle's: side 0 at
/// (2,2), side 1 at (5,2)), the screen clear.
fn fight_on(stage: &str) -> (Battle, [ObjectRef; 2]) {
    let mut setup = testing::round_setup(stage, megaman());
    setup.settings.effects = 0xE8C;
    let mut b = Battle::new(setup, testing::content());
    b.spawn_actors();
    b.run_objects();
    b.round.flags |= battle_flags::FIGHTING;
    b.fade.level = 0;
    let players = [b.player(0).unwrap(), b.player(1).unwrap()];
    for p in players {
        let o = b.objects.get_mut(p);
        (o.phase, o.phase_init) = (0, 0);
        super::super::set_navi_action(&mut b, p, super::super::NaviAction::Idle);
    }
    (b, players)
}

/// One tick of what the chips need: the objects (side 0 holding `held`),
/// then the banner and the screen fade, which the battle steps after them.
fn tick(b: &mut Battle, p: [ObjectRef; 2], held: u16) {
    for heard in &mut b.sound {
        heard.clear();
    }
    ai_mut(b, p[0]).pad.update(held | keys::PRESENT);
    ai_mut(b, p[1]).pad.update(keys::PRESENT);
    b.run_objects();
    b.banner.tick();
    b.fade.step();
}

/// Side 0 uses `chip` from idle (the first in its hand), and the ticks run
/// until its controller has come: tick 0 is the controller's first.
fn use_chip(b: &mut Battle, p: [ObjectRef; 2], chip: &str, controller: &str) -> ObjectRef {
    let chip = testing::chip_in(&b.content, chip);
    let mut hand = ChipHand::empty(&b.content);
    hand.ids[0] = Some(chip);
    hand.damage[0] = b.content.chip(chip).damage;
    b.hands[0] = hand;
    tick(b, p, keys::A);
    for _ in 0..10 {
        if let Some(c) = one(b, controller) {
            return c;
        }
        tick(b, p, 0);
    }
    panic!("no {controller}");
}

/// Run ticks until tick `last` has run.
fn run_to(b: &mut Battle, p: [ObjectRef; 2], now: &mut u32, last: u32) {
    assert!(*now <= last, "tick {last} has run");
    while *now < last {
        *now += 1;
        tick(b, p, 0);
    }
}

/// The objects of kind `key`, in update order.
fn all(b: &Battle, key: &str) -> Vec<ObjectRef> {
    b.objects.in_order().filter(|&o| b.local_kind_key(o) == key).collect()
}

fn one(b: &Battle, key: &str) -> Option<ObjectRef> {
    all(b, key).first().copied()
}

/// Whether the tick's sounds include the one named `name`.
fn heard(b: &Battle, name: &str) -> bool {
    let id = SoundId(crate::content::testing::asset_named(&b.content, nettai_content_api::AssetKind::Sound, name));
    b.sound_cues().contains(&SoundCue::Effect(id))
}

/// A panel's center at height `z` pixels, 16.16.
fn at(x: u8, y: u8, z: i32) -> Vec3 {
    let (px, py) = panel_coordinates(x, y);
    Vec3 { x: px, y: py, z: z << 16 }
}

/// The controller's action and phase.
fn step(b: &Battle, c: ObjectRef) -> (u8, u8) {
    let o = b.objects.get(c);
    (o.action, o.phase)
}

/// The controller fades the field to black (16 steps from clear), shows
/// the telop for 60 ticks and runs its effect from tick 78, which hides the
/// gauge and the emotion window and warps the user out. Returns tick 78.
fn fade_out_and_telop(b: &mut Battle, p: [ObjectRef; 2], c: ObjectRef, t: &mut u32) -> u32 {
    assert!(b.is_dimmed());
    assert_eq!(step(b, c), (0, 0));
    run_to(b, p, t, 1);
    assert_eq!((b.fade.mode, b.fade.level), (FadeMode::BlackOut, 0x10));
    run_to(b, p, t, 16);
    assert_eq!(b.fade.level, 0x100);
    assert_eq!(step(b, c), (0, 0));
    run_to(b, p, t, 17);
    assert_eq!(step(b, c), (4, 0), "the telop");
    run_to(b, p, t, 76);
    assert_eq!(step(b, c).0, 4);
    run_to(b, p, t, 77);
    assert_eq!(step(b, c), (8, 0), "the effect");
    assert!(!b.hud_hidden.gauge);
    run_to(b, p, t, 78);
    assert!(b.hud_hidden.gauge && b.hud_hidden.emotion_window && !b.hud_hidden.level_gauge);
    assert!(one(b, "engine/navi-warp").is_some(), "the user warps out");
    78
}

/// The effect's end: the user warps back in at `back`, 31 ticks later the
/// HUD is back and the field fades in from black (the first step holds),
/// and the controller ends the dimming.
fn warp_back_and_fade_in(b: &mut Battle, p: [ObjectRef; 2], c: ObjectRef, t: &mut u32, back: u32) {
    run_to(b, p, t, back - 1);
    assert!(one(b, "engine/navi-warp").is_none());
    run_to(b, p, t, back);
    assert!(one(b, "engine/navi-warp").is_some(), "the user warps back in");
    run_to(b, p, t, back + 29);
    assert!(b.hud_hidden.gauge);
    run_to(b, p, t, back + 30);
    assert!(!b.hud_hidden.gauge && !b.hud_hidden.emotion_window);
    assert_eq!(step(b, c), (0xC, 0));
    run_to(b, p, t, back + 31);
    assert_eq!((b.fade.mode, b.fade.level, b.fade.active()), (FadeMode::BlackOutBack, 0x100, true));
    run_to(b, p, t, back + 47);
    assert_eq!((b.fade.level, b.fade.active()), (0, false));
    assert!(b.is_dimmed());
    run_to(b, p, t, back + 48);
    assert!(b.is_dimmed(), "the controller's end state");
    run_to(b, p, t, back + 49);
    assert!(!b.is_dimmed(), "the dimming is over");
}

#[test]
fn gregar_sends_its_two_pieces_which_breathe_flames_and_drop_rocks() {
    let (mut b, p) = fight_on(testing::LINK_BATTLE);
    let c = use_chip(&mut b, p, testing::GREGAR, "gregar/controller");
    let mut t = 0;
    let e = fade_out_and_telop(&mut b, p, c, &mut t);

    // 31 ticks on, Gregar's two pieces one panel behind the user, the
    // second a pixel down the field and up.
    run_to(&mut b, p, &mut t, e + 30);
    assert!(all(&b, "gregar/summon").is_empty());
    run_to(&mut b, p, &mut t, e + 31);
    let pieces = all(&b, "gregar/summon");
    assert_eq!(pieces.len(), 2);
    let behind = at(1, 2, 0);
    let (second, first) = (pieces[0], pieces[1]);
    assert_eq!(b.objects.get(first).pos, behind);
    assert_eq!(b.objects.get(second).pos, Vec3 { y: behind.y + 0x1_0000, z: 0x1_0000, ..behind });
    assert_eq!((b.objects.get(first).anim, b.objects.get(second).anim), (0, 4));
    for &r in &pieces {
        assert_eq!(b.objects.get(r).panel, PanelPos { x: 1, y: 2 });
        assert_eq!(b.objects.get(r).flags & flags::VISIBLE, 0);
    }

    // They appear with a sound; the second brings the arrival, five pixels
    // down the field and up from it, for 60 ticks.
    run_to(&mut b, p, &mut t, e + 32);
    assert!(heard(&b, "log-in"));
    assert!(pieces.iter().all(|&r| b.objects.get(r).flags & flags::VISIBLE != 0));
    let arrival = one(&b, "beast-chips/arrival").expect("the arrival");
    let s = b.objects.get(second).pos;
    assert_eq!(b.objects.get(arrival).pos, Vec3 { x: s.x, y: s.y + 0x5_0000, z: s.z + 0x5_0000 });
    assert_eq!(b.objects.get(arrival).anim, 8);
    run_to(&mut b, p, &mut t, e + 92);
    assert_eq!(b.objects.get(first).action, 4, "60 ticks");
    run_to(&mut b, p, &mut t, e + 93);
    assert!(heard(&b, "bug"));
    assert!(one(&b, "beast-chips/arrival").is_none());
    assert_eq!((b.objects.get(first).anim, b.objects.get(second).anim), (1, 5));
    run_to(&mut b, p, &mut t, e + 108);
    assert_eq!(b.objects.get(first).action, 8, "15 ticks white");
    run_to(&mut b, p, &mut t, e + 139);
    assert_eq!((b.objects.get(first).action, b.objects.get(second).action), (0xC, 0x10), "30 ticks rising");

    // The second roars and breathes seven flames over the three columns
    // from the user's (300 and the counter byte, while dimmed), and a rock
    // falls on the enemy's panel (the chip's parameter byte, 100).
    let hp = |b: &Battle| b.objects.get(p[1]).hp;
    assert_eq!(hp(&b), 1000);
    run_to(&mut b, p, &mut t, e + 140);
    assert!(heard(&b, "beast-out-chosen-gregar") && heard(&b, "roar"));
    let flames = all(&b, "gregar/flame");
    let mut panels: Vec<(u8, u8)> = flames.iter().map(|&f| (b.objects.get(f).panel.x, b.objects.get(f).panel.y)).collect();
    panels.sort();
    assert_eq!(panels, [(3, 2), (4, 1), (4, 2), (4, 3), (5, 1), (5, 2), (5, 3)]);
    for &f in &flames {
        let o = b.objects.get(f);
        assert_eq!(o.pos, at(o.panel.x, o.panel.y, 8));
        assert_eq!((o.damage, o.stamina), (300, 0x8A));
    }
    let rocks = all(&b, "grndman/rock");
    assert_eq!(rocks.len(), 1);
    let rock = b.objects.get(rocks[0]);
    assert_eq!((rock.panel, rock.damage, rock.stamina), (PanelPos { x: 5, y: 2 }, 100, 0x8A));
    run_to(&mut b, p, &mut t, e + 141);
    assert_eq!(hp(&b), 700, "the flame on its panel");

    // A rock every 20 ticks, a growl every 16; after 180 ticks they sink
    // for 10 and go, and the controller moves on.
    let mut drops = 1;
    let mut growls = vec![];
    for n in e + 141..e + 319 {
        let before = all(&b, "grndman/rock");
        run_to(&mut b, p, &mut t, n);
        if all(&b, "grndman/rock").iter().any(|r| !before.contains(r)) {
            drops += 1;
            assert_eq!((n - (e + 140)) % 20, 0, "a rock at {n}");
        }
        if heard(&b, "roar") {
            growls.push(n - (e + 140));
        }
    }
    assert_eq!(drops, 9);
    assert_eq!(growls, [15, 31, 47, 63, 79, 95, 111, 127, 143, 159, 175]);
    run_to(&mut b, p, &mut t, e + 319);
    assert_eq!((b.objects.get(first).action, b.objects.get(second).action), (0x14, 0x14));
    run_to(&mut b, p, &mut t, e + 330);
    assert_eq!(step(&b, c), (8, 4));
    run_to(&mut b, p, &mut t, e + 331);
    assert!(all(&b, "gregar/summon").is_empty());
    assert_eq!(step(&b, c), (8, 8));
    warp_back_and_fade_in(&mut b, p, c, &mut t, e + 363);
}

#[test]
fn falzar_sends_feathers_then_its_three_pieces_and_a_whirlwind() {
    let (mut b, p) = fight_on(testing::LINK_BATTLE);
    let c = use_chip(&mut b, p, testing::FALZAR, "falzar/controller");
    let mut t = 0;
    let e = fade_out_and_telop(&mut b, p, c, &mut t);

    // 31 ticks on, a feather every 12 ticks for 120, from 192 pixels back and
    // up, diving 16 ticks onto a panel of the other side's: 100 and the
    // counter byte (no bonus), a spark and the camera shaking.
    run_to(&mut b, p, &mut t, e + 30);
    assert!(one(&b, "falzar/feather").is_none());
    assert_eq!(step(&b, c), (8, 4));
    run_to(&mut b, p, &mut t, e + 31);
    let feather = one(&b, "falzar/feather").expect("a feather");
    let o = b.objects.get(feather);
    let (target, dive) = (o.panel, at(o.panel.x, o.panel.y, 0));
    assert_eq!(o.pos, Vec3 { x: dive.x - 0xC0_0000, z: 0xC0_0000, ..dive });
    assert_eq!((o.damage, o.stamina), (100, 0x8A));
    assert!(o.panel.x >= 4, "the other side's area: {:?}", o.panel);
    run_to(&mut b, p, &mut t, e + 32);
    assert!(heard(&b, "justice-one"));
    assert_eq!(b.objects.get(feather).pos.z, 0xC0_0000);
    run_to(&mut b, p, &mut t, e + 47);
    assert_eq!(b.objects.get(feather).pos, Vec3 { x: dive.x - 0xC_0000, z: 0xC_0000, ..dive });
    let hp = b.objects.get(p[1]).hp;
    run_to(&mut b, p, &mut t, e + 48);
    assert_eq!(b.objects.get(feather).state, crate::object::state::DESTROY, "landed");
    run_to(&mut b, p, &mut t, e + 49);
    assert!(!all(&b, "falzar/feather").contains(&feather));
    let landed = target == PanelPos { x: 5, y: 2 };
    assert_eq!(b.objects.get(p[1]).hp, if landed { hp - 100 } else { hp });
    // (A feather's phase is 0 while it lives on the tick it comes only.)
    let (mut b, p) = fight_on(testing::LINK_BATTLE);
    let c = use_chip(&mut b, p, testing::FALZAR, "falzar/controller");
    let mut t = 0;
    let mut sent = vec![];
    for n in 1..e + 151 {
        run_to(&mut b, p, &mut t, n);
        let new = |o: &crate::object::Object| o.state == crate::object::state::UPDATE && o.phase == 0;
        if all(&b, "falzar/feather").iter().any(|&f| new(b.objects.get(f))) {
            sent.push(n - e);
        }
    }
    assert_eq!(sent, [31, 43, 55, 67, 79, 91, 103, 115, 127, 139]);
    assert_eq!(step(&b, c), (8, 8));
    run_to(&mut b, p, &mut t, e + 181);
    assert_eq!(step(&b, c), (8, 0xC));

    // Then Falzar's three pieces 36 pixels over the user, the leading one
    // a pixel up the field and lower; it brings the arrival.
    run_to(&mut b, p, &mut t, e + 182);
    let pieces = all(&b, "falzar/summon");
    assert_eq!(pieces.len(), 3);
    let over = at(2, 2, 0x24);
    let anims: Vec<u8> = pieces.iter().map(|&r| b.objects.get(r).anim).collect();
    assert_eq!(anims, [8, 4, 0], "each spawns right after the controller");
    let lead = pieces[2];
    assert_eq!(b.objects.get(lead).pos, Vec3 { y: over.y - 0x1_0000, z: over.z - 0x1_0000, ..over });
    assert_eq!(b.objects.get(pieces[0]).pos, over);
    run_to(&mut b, p, &mut t, e + 183);
    assert!(heard(&b, "log-in"));
    let arrival = one(&b, "beast-chips/arrival").expect("the arrival");
    let l = b.objects.get(lead).pos;
    assert_eq!(b.objects.get(arrival).pos, Vec3 { x: l.x, y: l.y + 0x5_0000, z: l.z + 0x5_0000 });
    assert_eq!(b.objects.get(arrival).anim, 0xC);
    run_to(&mut b, p, &mut t, e + 244);
    assert!(heard(&b, "bug"));
    run_to(&mut b, p, &mut t, e + 290);
    let actions: Vec<u8> = pieces.iter().map(|&r| b.objects.get(r).action).collect();
    assert_eq!(actions, [0xC, 0xC, 0x10]);

    // The leading piece cries and sends the whirlwind 40 pixels ahead of the
    // panel in front of the user: three blows, 40 ticks apart, 100 each.
    let hp = |b: &Battle| b.objects.get(p[1]).hp;
    let before = hp(&b);
    run_to(&mut b, p, &mut t, e + 291);
    assert!(heard(&b, "beast-out-chosen-falzar") && heard(&b, "elec-pulse"));
    let wind = one(&b, "falzar/tornado").expect("the whirlwind");
    let o = b.objects.get(wind);
    let front = at(3, 2, 0);
    assert_eq!(o.pos, Vec3 { x: front.x + 0x28_0000, ..front });
    assert_eq!((o.damage, o.stamina), (100, 0x8A));
    run_to(&mut b, p, &mut t, e + 293);
    assert_eq!(hp(&b), before - 100);
    run_to(&mut b, p, &mut t, e + 333);
    assert_eq!(hp(&b), before - 200);
    run_to(&mut b, p, &mut t, e + 373);
    assert_eq!(hp(&b), before - 300);
    run_to(&mut b, p, &mut t, e + 410);
    assert!(pieces.iter().all(|&r| b.objects.get(r).action == 0x14));
    run_to(&mut b, p, &mut t, e + 412);
    assert!(one(&b, "falzar/tornado").is_none(), "120 ticks");
    run_to(&mut b, p, &mut t, e + 421);
    assert_eq!(step(&b, c), (8, 0xC));
    run_to(&mut b, p, &mut t, e + 422);
    assert!(all(&b, "falzar/summon").is_empty());
    assert_eq!(step(&b, c), (8, 0x10));
    run_to(&mut b, p, &mut t, e + 453);
    assert_eq!(step(&b, c), (8, 0x14));
    warp_back_and_fade_in(&mut b, p, c, &mut t, e + 454);
}

#[test]
fn the_beast_chips_break_the_field_objects() {
    use crate::object::Pool;
    // The rock battle's rocks at (3,3) and (4,1); side 0 at (1,2).
    for (chip, controller, breaks) in [(testing::GREGAR, "gregar/controller", 32), (testing::FALZAR, "falzar/controller", 30)] {
        let (mut b, p) = fight_on(testing::ROCK_BATTLE);
        let rocks: Vec<ObjectRef> = b.objects.in_order().filter(|r| r.pool == Pool::Attack).collect();
        assert_eq!(rocks.len(), 2);
        let c = use_chip(&mut b, p, chip, controller);
        let mut t = 0;
        let e = fade_out_and_telop(&mut b, p, c, &mut t);
        run_to(&mut b, p, &mut t, e + breaks - 1);
        assert!(rocks.iter().all(|&r| b.objects.get(r).hp != 0), "{chip}");
        run_to(&mut b, p, &mut t, e + breaks);
        assert!(rocks.iter().all(|&r| b.objects.get(r).hp == 0), "{chip}: every field object's HP to 0");
    }
}

#[test]
fn a_copy_of_the_battle_plays_the_beast_chips_on_the_same() {
    for (chip, controller) in [(testing::GREGAR, "gregar/controller"), (testing::FALZAR, "falzar/controller")] {
        let (mut b, p) = fight_on(testing::LINK_BATTLE);
        use_chip(&mut b, p, chip, controller);
        for _ in 0..6 {
            let mut copy = b.clone();
            for _ in 0..97 {
                tick(&mut b, p, 0);
                tick(&mut copy, p, 0);
                assert_eq!(b.digest(), copy.digest(), "{chip}");
            }
        }
    }
}
