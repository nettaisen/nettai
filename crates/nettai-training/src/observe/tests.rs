use super::*;
use crate::bots::BotKind;
use crate::env::{Config, Controller, Env};
use crate::testing::{GAMES, games};
use nettai_battle::Rng;
use nettai_netplay::rng::SplitMix64;

/// Each set's arrays have the spec's shapes; a token not there is all
/// zeros; every number is finite and every id a bucket.
fn check_shapes(obs: &Obs) {
    assert_eq!(obs.sets.len(), spec::SETS.len());
    for (s, buf) in spec::SETS.iter().zip(&obs.sets) {
        assert_eq!(buf.x.len(), s.count * s.width, "{}", s.name);
        assert_eq!(buf.ids.len(), s.count * s.ids, "{}", s.name);
        assert_eq!(buf.mask.len(), s.count, "{}", s.name);
        for i in 0..s.count {
            let x = &buf.x[i * s.width..(i + 1) * s.width];
            let ids = &buf.ids[i * s.ids..(i + 1) * s.ids];
            assert!(x.iter().all(|v| v.is_finite()), "{} token {i}: {x:?}", s.name);
            assert!(ids.iter().all(|&id| (0..names::BUCKETS as i32).contains(&id)), "{} token {i}", s.name);
            if !buf.mask[i] {
                assert!(x.iter().all(|&v| v == 0.0) && ids.iter().all(|&id| id == 0), "{} token {i} isn't there but has values", s.name);
            }
        }
    }
}

/// An env of `game` whose seats a masher and a shooter play, `ticks` into
/// its first round.
fn played(game: &str, seed: u64, ticks: u32) -> Env {
    let seats = [Controller::Bot(BotKind::Masher), Controller::Bot(BotKind::Shooter)];
    let mut env = Env::new(games(&[game]), Config { seats, practice: 0.5, ..Config::default() }, seed).unwrap();
    env.run(ticks);
    env
}

#[test]
fn an_observation_has_the_specs_shapes() {
    for game in GAMES {
        for (seed, ticks) in [(1, 0), (2, 200), (3, 700), (4, 1500), (5, 3000)] {
            let env = played(game, seed, ticks);
            for viewer in 0..2u8 {
                let mut obs = Obs::new();
                encode(env.battle(), viewer, env.names(), &env.extra_now(viewer as usize), &mut obs);
                check_shapes(&obs);
                assert!(obs.sets[spec::GLOBAL].mask[0]);
                assert_eq!(obs.sets[spec::PANELS].mask.iter().filter(|&&m| m).count(), 18);
            }
        }
        // How many scalars the rules tokens could hold, against the set's
        // count.
        let n = Names::of(&crate::testing::content(game)).rule_scalars();
        println!("{game}: {n} rules scalars ({} tokens)", spec::SETS[spec::RULES].count);
    }
}

/// The field is laid out from the viewer's side: at a round's start each
/// viewer's own area is the left three columns of the panel tokens.
#[test]
fn each_viewer_sees_its_own_area_on_the_left() {
    for game in GAMES {
        let env = played(game, 9, 30);
        for viewer in 0..2u8 {
            let mut obs = Obs::new();
            encode(env.battle(), viewer, env.names(), &env.extra_now(viewer as usize), &mut obs);
            let w = spec::panels::WIDTH;
            for t in 0..18 {
                let own = obs.sets[spec::PANELS].x[t * w + spec::panels::owner];
                let column = t % 6;
                assert_eq!(own, if column < 3 { 1.0 } else { 0.0 }, "{game}: viewer {viewer}'s token {t}");
            }
        }
    }
}

/// Everything the viewer must not see, changed: the other side's hand,
/// folder, custom screen, setup and rules' state, the battle's and both
/// consoles' RNG, and the objects hidden from the viewer.
fn scramble(b: &mut Battle, viewer: u8, rng: &mut SplitMix64) {
    let them = (viewer ^ 1) as usize;
    let chips = b.content.defs.chips.len() as u64;
    let chip = |rng: &mut SplitMix64| nettai_content_api::ChipHandle(rng.below(chips) as u16);
    let h = &mut b.hands[them];
    for i in 0..6 {
        h.ids[i] = Some(chip(rng));
        h.damage[i] = rng.below(400) as u16;
        h.attack_bonus[i] = rng.below(50) as u16;
        h.modifiers[i] = rng.below(8) as u8;
    }
    h.cursor = rng.below(5) as u8;
    let side = &mut b.custom.sides[them];
    side.folder.chips.rotate_left(1 + rng.below(28) as usize);
    for c in side.folder.chips.iter_mut().take(5) {
        *c = Some(nettai_battle::custom::FolderChip::new(chip(rng), nettai_battle::content::ChipCode(rng.below(26) as u8)));
    }
    side.folder.regular_pending ^= true;
    side.program_advances.0 ^= rng.next_u64();
    side.class_uses.standard = side.class_uses.standard.wrapping_add(3);
    if let Some(screen) = &mut side.screen {
        screen.cursor = rng.below(10) as u8;
        screen.form = None;
    }
    let setup = &mut b.setup.players[them];
    setup.folder.chips.rotate_left(3);
    setup.console.rng ^= rng.next_u64() as u32;
    b.rng = Rng { state: rng.next_u64() as u32 };
    for c in &mut b.consoles {
        c.rng = Rng { state: rng.next_u64() as u32 };
    }
    // (The other's rules' state: its top numbers and flags.)
    if let Some(r) = b.content.defs.rules() {
        let schema = b.content.defs.schema(r.state).clone();
        if let Some(block) = b.rules[them].state.as_mut() {
            for i in 0..schema.fields().len() {
                let v = match schema.field(i).ty {
                    nettai_content_api::FieldType::Bool => nettai_content_api::Value::Bool(rng.chance(1, 2)),
                    nettai_content_api::FieldType::U8 | nettai_content_api::FieldType::U16 => nettai_content_api::Value::Int(rng.below(200) as i64),
                    _ => continue,
                };
                let _ = block.set(&schema, i, v);
            }
        }
    }
    // The objects hidden from the viewer (but its own navi, whose HP and
    // charge its HUD shows).
    let own = b.player(viewer);
    let hidden: Vec<ObjectRef> = b.objects.in_order().filter(|&r| !sees_object(b, viewer, r) && Some(r) != own).collect();
    for r in hidden {
        let o = b.objects.get_mut(r);
        o.hp = rng.below(900) as u16;
        o.max_hp = o.max_hp.max(o.hp);
        o.pos.x += rng.below(20 << 16) as i32;
        o.vel.y = rng.below(4 << 16) as i32;
        o.anim = rng.below(30) as u8;
        o.damage = rng.below(300) as u16;
        o.timer = rng.below(100) as u16;
    }
}

fn bits(obs: &Obs) -> Vec<u32> {
    obs.sets.iter().flat_map(|s| s.x.iter().map(|v| v.to_bits()).chain(s.ids.iter().map(|&i| i as u32)).chain(s.mask.iter().map(|&m| m as u32))).collect()
}

/// Fairness (§3): for random positions of all three games, a viewer's
/// observation is the same byte for byte whatever it must not see.
#[test]
fn a_view_is_the_same_whatever_the_viewer_must_not_see() {
    let mut rng = SplitMix64::new(42);
    let mut positions = 0;
    let mut screens = 0;
    for game in GAMES {
        for seed in 0..6u64 {
            let mut env = played(game, 100 + seed, 0);
            // (Positions along the round: on the screen, in the fight.)
            for _ in 0..8 {
                env.run(50 + rng.below(400) as u32);
                let b = env.battle();
                screens += (b.round.mode == nettai_battle::battle::mode::CUSTOM) as u32;
                for viewer in 0..2u8 {
                    let extra = env.extra_now(viewer as usize);
                    let mut seen = Obs::new();
                    encode(b, viewer, env.names(), &extra, &mut seen);
                    for _ in 0..3 {
                        let mut changed = b.clone();
                        scramble(&mut changed, viewer, &mut rng);
                        let mut after = Obs::new();
                        encode(&changed, viewer, env.names(), &extra, &mut after);
                        if bits(&after) != bits(&seen) {
                            for (s, (x, y)) in spec::SETS.iter().zip(seen.sets.iter().zip(&after.sets)) {
                                if x != y {
                                    let w = s.width;
                                    let at = x.x.iter().zip(&y.x).position(|(a, b)| a.to_bits() != b.to_bits());
                                    let what = at.map(|i| (i / w, s.fields.iter().scan(0, |o, (n, wd)| { let r = (*o, *n); *o += wd; Some(r) }).filter(|(o, _)| *o <= i % w).last().map(|(_, n)| n)));
                                    panic!("{game} seed {seed} viewer {viewer}: set {} changed (token, feature: {what:?})", s.name);
                                }
                            }
                        }
                    }
                    positions += 1;
                }
            }
        }
    }
    println!("{positions} positions, {screens} of them on the custom screen");
    assert!(screens > 0);
}
