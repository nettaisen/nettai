//! A flying shot (the projectile, @exelib/projectile, on the engine's fly
//! drive, `kinds::drive`) hits what it meets once.

use crate::testing::exe6_content;
use nettai_battle::battle::{fight, mode};
use nettai_battle::input::{PlayerTick, keys};
use nettai_battle::{Battle, Content};
use std::sync::Arc;

/// An EXE6 round of MegaMan against MegaMan on stage `stage` (a link
/// battle's), both pressing through their custom screens until the fight
/// runs.
fn fighting(content: &Arc<Content>, stage: &str) -> Battle {
    let stage = crate::link_stage(content, "exe6", stage).unwrap();
    let mut m = crate::pick::live(content, "exe6", 3, Some(stage)).unwrap();
    for side in &mut m.sides {
        side.set_navi(content, crate::ids::navi(content, "exe6", "megaman").unwrap()).unwrap();
    }
    let mut b = Battle::new(m.round(content, 0x5EED), content.clone());
    let mut last = [None; 2];
    for _ in 0..3000 {
        if b.round.mode == mode::FIGHTING && b.fight.state == fight::FIGHTING {
            return b;
        }
        let input = [0, 1].map(|side| {
            last[side] = nettai_battle::scenario::custom_choice(&b, side, last[side]);
            PlayerTick::buttons(0).with_choice(last[side])
        });
        b.tick(&input, Default::default());
    }
    panic!("the fight never started");
}

fn hp(b: &Battle, side: u8) -> u16 {
    b.objects.get(b.player(side).expect("the side's navi")).hp
}

/// The right MegaMan's buster shot down the middle row, the left one
/// stepping down into it from the row above, the step pressed on each tick
/// of its flight in turn, with either navi first in the update order (the
/// two stages place them in either order): it hits him at most once. A
/// navi that steps onto the shot's panel is met by the shot as its
/// collision is taken out, which the fly drive must read as the shot's
/// update does: read before it, the shot missed its hit and hit him again
/// a tick later (a player's report, 0.20).
#[test]
fn a_shot_hits_a_navi_stepping_into_it_once() {
    let content = exe6_content();
    for stage in ["netbattle-1", "netbattle-2"] {
        let mut start = fighting(&content, stage);
        // (A direction acts on a hold's second tick.)
        for held in [keys::UP, keys::UP, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] {
            start.tick(&[PlayerTick::buttons(held), PlayerTick::default()], Default::default());
        }
        assert_eq!(start.objects.get(start.player(0).unwrap()).panel.y, 1, "{stage}: he stepped up");
        let mut hit_at = Vec::new();
        for step in 0..24 {
            let mut b = start.clone();
            let before = hp(&b, 0);
            let (mut hits, mut last) = (0, before);
            b.tick(&[PlayerTick::default(), PlayerTick::buttons(keys::B)], Default::default());
            for t in 0..60 {
                let left = if t == step || t == step + 1 { keys::DOWN } else { 0 };
                b.tick(&[PlayerTick::buttons(left), PlayerTick::default()], Default::default());
                let now = hp(&b, 0);
                if now < last {
                    hits += 1;
                }
                last = now;
            }
            assert!(hits <= 1, "{stage}, stepping down {step} ticks after the shot: hit {hits} times ({before} to {last} HP)");
            if hits == 1 {
                hit_at.push(step);
            }
        }
        assert!(!hit_at.is_empty(), "{stage}: the shot hits him stepping into it at some tick");
    }
}
