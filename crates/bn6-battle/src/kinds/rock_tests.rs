//! Rock tests. The navis aren't implemented yet, so the trace test drives
//! only the rocks and the objects they spawn, with the battle-wide state
//! (pause, flags, the navis' positions and actions) taken from the trace.

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
        navi_stats: [NaviStats([0; 0x64]); 2],
        rng: 1,
        local_side: 0,
        score: SetScore::default(),
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

#[cfg(feature = "trace")]
mod against_trace {
    use super::*;
    use crate::trace::{self, Frame, unhex};
    use std::io::BufRead;

    fn trace_path() -> Option<std::path::PathBuf> {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/traces/soundmod.jsonl");
        p.exists().then_some(p)
    }

    #[derive(serde::Deserialize)]
    struct SetupLine {
        setup: trace::Setup,
    }

    /// Round 1 of the soundmod trace up to frame `last`.
    fn round_one(last: u32) -> Option<trace::Round> {
        let f = std::io::BufReader::new(std::fs::File::open(trace_path()?).unwrap());
        let mut setup = None;
        let mut frames = Vec::new();
        for l in f.lines() {
            let l = l.unwrap();
            if l.starts_with("{\"setup\"") {
                if setup.is_some() {
                    break;
                }
                setup = Some(serde_json::from_str::<SetupLine>(&l).unwrap().setup);
            } else if l.starts_with("{\"frame\"") && setup.is_some() {
                let fr: Frame = serde_json::from_str(&l).unwrap();
                if fr.frame > last {
                    break;
                }
                frames.push(fr);
            }
        }
        Some(trace::Round { setup: setup?, exchanges: Vec::new(), frames })
    }

    fn compared(pool: Pool, index: u8) -> bool {
        matches!((pool, index), (Pool::Attack, INDEX) | (Pool::Effect, 0x38) | (Pool::Effect, 0x87))
    }

    fn describe_trace(o: &trace::Object) -> String {
        format!(
            "T{}#{:02x} f{:02x} p{:08x} s{:?} p{},{} a{} flip{} hp{}/{} pos{:?} t{} anim{} st{:x}",
            o.kind,
            o.index,
            o.flags,
            o.params,
            o.state,
            o.panel[0],
            o.panel[1],
            o.alliance,
            o.flip,
            o.hp,
            o.max_hp,
            o.pos,
            o.timer,
            o.anim,
            o.status
        )
    }

    fn describe(b: &Battle, r: ObjectRef) -> String {
        let o = b.objects.get(r);
        let status = o.collision.map(|c| b.collision.get(c).f1).unwrap_or(0);
        format!(
            "T{}#{:02x} f{:02x} p{:08x} s{:?} p{},{} a{} flip{} hp{}/{} pos{:?} t{} anim{} st{:x}",
            r.pool.type_number(),
            o.index,
            o.flags,
            u32::from_le_bytes(o.params),
            [o.state, o.action, o.phase, o.phase_init],
            o.panel.x,
            o.panel.y,
            o.alliance,
            o.flip,
            o.hp,
            o.max_hp,
            [o.pos.x, o.pos.y, o.pos.z],
            o.timer,
            o.anim,
            status
        )
    }

    /// Put the navis where the trace has them (they aren't simulated).
    fn drive_navis(b: &mut Battle, f: &Frame) {
        for side in 0..2u8 {
            let p = b.player(side).unwrap();
            let t = f.objects.iter().find(|o| o.kind == 1 && o.index == 0 && o.alliance == side).unwrap();
            let o = b.objects.get_mut(p);
            o.panel = PanelPos { x: t.panel[0], y: t.panel[1] };
            o.flip = t.flip;
            o.pos = Vec3 { x: t.pos[0], y: t.pos[1], z: t.pos[2] };
            [o.state, o.action, o.phase, o.phase_init] = t.state;
            let c = o.collision.unwrap();
            b.collision.get_mut(c).f1 = t.status;
        }
    }

    /// The soundmod trace's round 1 starts with two rocks (variant 3) at
    /// (3,3) and (4,1). They stand through the first custom screen and
    /// two time stops, then side 1's navi absorbs them (player action
    /// 0x58): they leave as absorbed obstacles that fly to it.
    #[test]
    fn soundmod_rocks_match_the_trace() {
        const LAST: u32 = 3800;
        let Some(round) = round_one(LAST) else {
            eprintln!("soundmod trace not found; skipping");
            return;
        };
        let frames: Vec<&Frame> = round.battle_frames().collect();
        let mut b = Battle::new(round.round_setup());
        assert_eq!(b.setup.settings.actors.source, 0x080B_1AAD);
        // The intro spawns the sequencer and the actor list. The navis'
        // init (not simulated) gives them collision slots 0 and 1.
        crate::kinds::intro::spawn(&mut b);
        b.spawn_actors();
        for side in 0..2 {
            let p = b.player(side).unwrap();
            b.create_collision(p).unwrap();
        }
        // Side 1's navi pulls on the tenth tick of its absorbing action:
        // the action first runs the tick after the navi shows it.
        let first_absorbing = frames
            .iter()
            .find(|f| f.objects.iter().any(|o| o.kind == 1 && o.index == 0 && o.alliance == 1 && o.state[1] == 0x58))
            .unwrap()
            .frame;
        let pull = first_absorbing + 10;
        let rng = b.rng.state;
        let mut seen_absorbed = false;
        for f in &frames {
            let bs = unhex(&f.bs);
            b.paused = f.paused != 0;
            b.round.flags = u16::from_le_bytes([bs[0x32], bs[0x33]]);
            b.round.alive = [bs[0x12], bs[0x13]];
            b.round.time_up = bs[0x0B];
            drive_navis(&mut b, f);
            if f.frame == pull {
                let navi = b.player(1).unwrap();
                crate::kinds::obstacle::absorb_all(&mut b, navi);
            }
            run_only(&mut b, &ROCK_KINDS);
            let ours: Vec<String> = b
                .objects
                .in_order()
                .filter(|r| compared(r.pool, b.objects.get(*r).index))
                .map(|r| describe(&b, r))
                .collect();
            let theirs: Vec<String> = f
                .objects
                .iter()
                .filter(|o| compared(if o.kind == 3 { Pool::Attack } else { Pool::Effect }, o.index) && o.kind != 1)
                .map(describe_trace)
                .collect();
            assert_eq!(ours, theirs, "frame {}", f.frame);
            seen_absorbed |= theirs.iter().any(|s| s.starts_with("T4#87"));
        }
        assert!(seen_absorbed, "the trace window covers the absorption");
        assert!(b.objects.in_order().all(|r| r.pool != Pool::Attack), "both rocks are gone");
        assert_eq!(b.rng.state, rng, "rocks standing and leaving this way draw no RNG");
        let navi = b.player(1).unwrap();
        let actor = b.objects.get(navi).actor.unwrap();
        let absorbed = &b.actors.get(actor).absorbed;
        assert_eq!(absorbed.len(), 2);
        assert!(absorbed.iter().all(|a| a.kind == 4 && a.anim == 2));
    }
}
