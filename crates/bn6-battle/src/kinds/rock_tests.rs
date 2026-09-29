//! Rock tests: spawning from an actor list and breaking into debris.

use super::*;
use crate::object::Pool;
use crate::setup::{ActorKind, BattleSettings, NaviStats, RoundSetup, SetScore};

/// Step the objects of the given kinds, in list order and with the game's
/// pause and time-stop gating (as `Battle::run_objects`).
fn run_only(b: &mut Battle, kinds: &[(Pool, u8)]) {
    let mut cur = b.objects.loop_first();
    while let Some(r) = cur {
        let o = b.objects.get(r);
        let gated = (b.paused && o.flags & flags::RUN_WHILE_PAUSED == 0)
            || (b.is_time_stop() && o.flags & flags::RUN_IN_TIME_STOP == 0);
        if !gated && kinds.contains(&(r.pool, o.index)) {
            crate::kinds::update(b, r);
        }
        cur = b.objects.loop_next();
    }
}

const ROCK_KINDS: [(Pool, u8); 4] = [
    (Pool::Attack, INDEX),
    (Pool::Effect, crate::kinds::rock_debris::INDEX),
    (Pool::Effect, crate::kinds::absorbed_obstacle::INDEX),
    (Pool::Effect, 0),
];

/// A netbattle round with actor list `actors` (panel pattern 0x38: columns
/// 1-3 are side 0's).
fn setup(actors: u32) -> RoundSetup {
    let mut settings = [0u8; 16];
    settings[6] = 0x38;
    settings[12..16].copy_from_slice(&actors.to_le_bytes());
    RoundSetup {
        settings: BattleSettings::netbattle_from_bytes(&settings),
        navi_stats: [NaviStats::from_bytes(&[0; 0x64]); 2],
        rng: 1,
        local_side: 0,
        score: SetScore::default(),
        later_stages: Default::default(),
        low_hp_music_latched: false,
        sp_times: Default::default(),
    }
}

#[test]
fn start_lists_spawn_rocks_outside_the_navi_bookkeeping() {
    let list = crate::setup::ActorList::find(0x080B_1AAD).unwrap();
    assert_eq!(
        list.entries.iter().map(|e| e.kind).collect::<Vec<_>>(),
        [ActorKind::Navi, ActorKind::Navi, ActorKind::Rock { variant: 3 }, ActorKind::Rock { variant: 3 }]
    );
    let mut b = Battle::new(setup(0x080B_1AAD));
    b.spawn_actors();
    assert_eq!(b.round.alive, [1, 1]);
    assert_eq!(b.round.name_counts, [1, 1]);
    let rocks: Vec<_> = b.objects.in_order().filter(|r| r.pool == Pool::Attack).collect();
    assert_eq!(rocks.len(), 2);
    assert_eq!(b.objects.get(rocks[0]).params, [3, 0, 3, 0]);
    // Registered on their panels' sides: (3,3) is side 0's, (4,1) side 1's.
    assert_eq!(b.field.objects.slots[0], Some(rocks[0]));
    assert_eq!(b.field.objects.slots[3], Some(rocks[1]));
}

/// A rock broken by damage throws two debris chunks (a jitter draw each,
/// then two draws in each chunk's init) and a dust effect.
#[test]
fn breaking_throws_debris() {
    let mut b = Battle::new(setup(0x080B_1989));
    let spec = Spec { variant: 1, class: 0, entrance: Entrance::Instant };
    let r = spawn(&mut b, PanelPos { x: 3, y: 2 }, 0, spec, 200, 0).unwrap();
    // A fight in progress (with nobody alive the battle is over, and
    // obstacles break).
    b.round.flags |= crate::battle::battle_flags::FIGHTING;
    b.round.alive = [1, 1];
    run_only(&mut b, &ROCK_KINDS);
    assert_eq!(b.objects.get(r).action, Action::Idle as u8);
    assert_eq!(b.objects.get(r).hp, 200);
    let before = b.rng.state;
    b.objects.get_mut(r).hp = 0;
    run_only(&mut b, &ROCK_KINDS);
    let mut rng = crate::rng::Rng::new(before);
    for _ in 0..6 {
        rng.next();
    }
    assert_eq!(b.rng.state, rng.state);
    let order: Vec<_> = b.objects.in_order().map(|o| (o.pool, b.objects.get(o).index)).collect();
    assert_eq!(
        order,
        [(Pool::Attack, INDEX), (Pool::Effect, 0), (Pool::Effect, 0x38), (Pool::Effect, 0x38)],
        "the rock, then what it spawned in reverse order"
    );
    assert_eq!(b.objects.get(r).state, state::DESTROY);
    assert_eq!(b.field.objects.slots[0], None);
    run_only(&mut b, &ROCK_KINDS);
    assert!(!b.objects.is_allocated(r));
}
