//! The content scripts' tests, on the hand-authored test content (whose
//! scripts are this repository's BN6 scripts): what the content registers,
//! and the Luau runtime's rules (no state kept in the VM, no
//! nondeterminism, integers only, bounded work).

use crate::battle::Battle;
use crate::behavior::{Behaviors, Options};
use crate::content::{Content, testing};
use crate::scenario;

/// Each tick's state digest over a tape.
fn digests(tape: &[scenario::Tick], mut b: Battle) -> Vec<u64> {
    tape.iter()
        .map(|t| {
            b.tick(&t.input, t.events.clone());
            b.digest()
        })
        .collect()
}

#[test]
fn battles_run_the_content_scripts() {
    let b = Battle::new(scenario::setup(), scenario::content());
    assert_eq!(b.behaviors.runtime(), "luau");
    let m = b.behaviors.manifest().expect("the test content has scripts");
    let kinds: Vec<&str> = m.objects.iter().map(|k| k.name.as_str()).collect();
    assert_eq!(
        kinds,
        [
            "area-grab",
            "attachment",
            "dust-ball",
            "erase-beam",
            "erase-man",
            "erase-mark",
            "gauge-speed",
            "grab-shot",
            "invisible",
            "navi-boost",
            "rising-bubble",
            "rock",
            "rock-cube",
            "rock-debris",
            "sun-beam",
            "trap-chip",
        ]
    );
    assert!(b.behaviors.action(0x37).is_some(), "GunDelSol is a script");
    assert!(b.behaviors.action(0x10).is_none(), "the step is the engine's");
}

#[test]
fn the_duel_fires_scripted_gun_del_sols() {
    // The tape's players fire their level-3 GunDelSols: the scripted gun
    // and beam appear, and the hits land.
    let tape = scenario::record(900);
    let mut b = Battle::new(scenario::setup(), scenario::content());
    let (mut beams, mut guns) = (0, 0);
    for t in &tape {
        b.tick(&t.input, t.events.clone());
        for r in b.objects.in_order() {
            match (r.pool, b.objects.get(r).index) {
                (crate::object::Pool::Effect, 0x48) => beams += 1,
                (crate::object::Pool::Actor, 5) => guns += 1,
                _ => {}
            }
        }
    }
    assert!(beams > 0 && guns > 0, "{beams} beam and {guns} gun object-ticks");
    let hp: Vec<u16> = (0..2).map(|s| b.objects.get(b.player(s).unwrap()).hp).collect();
    assert!(hp.iter().any(|&h| h < 1000), "someone got hit: {hp:?}");
}

/// A duel with the eraser navi chip, the grab dimming chip and GunDelSols
/// in the folders: the ticks each scripted kind was on the field, by
/// (pool, index).
fn chip_duel(ticks: usize) -> std::collections::BTreeMap<(crate::object::Pool, u8), usize> {
    let setup = || scenario::setup_with(&[testing::ERASER, testing::GRAB, testing::SUN_GUN_3]);
    let tape = scenario::record_on(setup(), ticks, 11);
    let mut b = Battle::new(setup(), scenario::content());
    let mut seen = std::collections::BTreeMap::new();
    for t in &tape {
        b.tick(&t.input, t.events.clone());
        for r in b.objects.in_order() {
            *seen.entry((r.pool, b.objects.get(r).index)).or_insert(0) += 1;
        }
    }
    seen
}

#[test]
fn the_scripted_navi_and_dimming_chips_play() {
    use crate::object::Pool::{Actor, Attack, Effect};
    let seen = chip_duel(2400);
    let ticks = |k| seen.get(&k).copied().unwrap_or(0);
    // The eraser navi comes, marks its aim and slashes along it.
    assert!(ticks((Actor, 0x15)) > 0, "EraseMan: {seen:?}");
    assert!(ticks((Effect, 0x62)) > 0, "EraseMan's marks: {seen:?}");
    assert!(ticks((Attack, 0xC3)) > 0, "EraseMan's slash: {seen:?}");
    // The grab's controller drops grab shots.
    assert!(ticks((Effect, 0x03)) > 0, "the grab's controller: {seen:?}");
    assert!(ticks((Attack, 0x0F)) > 0, "grab shots: {seen:?}");
}

#[test]
fn scripted_chips_roll_back() {
    // A copy of the battle taken at any tick plays on exactly as the
    // battle does: the scripts' state is all in the battle.
    let setup = || scenario::setup_with(&[testing::ERASER, testing::GRAB, testing::SUN_GUN_3]);
    let tape = scenario::record_on(setup(), 2400, 11);
    let mut b = Battle::new(setup(), scenario::content());
    let whole = digests(&tape, Battle::new(setup(), scenario::content()));
    for (i, t) in tape.iter().enumerate() {
        if i % 97 == 0 {
            let copy = digests(&tape[i..], b.clone());
            assert_eq!(copy, whole[i..], "the copy from tick {i} went its own way");
        }
        b.tick(&t.input, t.events.clone());
    }
}

#[test]
fn registrations_follow_the_content_data() {
    let mut c = testing::build();
    let r = c.registrations().unwrap();
    // The four SunGun chips share one action; two weapons have theirs.
    let actions: Vec<u8> = r.actions.iter().map(|a| a.action).collect();
    assert_eq!(actions, [0x33, 0x37, 0x57], "{:?}", r.actions);
    // Two chips implementing one action with different scripts is an error.
    c.chips[testing::SUN_GUN_2 as usize].script = Some("objects/sun-beam/sun_beam".into());
    let e = c.registrations().unwrap_err();
    assert!(e.contains("implements action 0x37"), "{e}");
    // So is naming a script the pack doesn't have.
    let mut c = testing::build();
    c.objects.kinds[0].script = "objects/nowhere".into();
    assert!(c.registrations().unwrap_err().contains("isn't in the pack"));
}

// ---- Dimming chips and rocks ------------------------------------------------------------

/// A duel with the cube, veil and trap dimming chips in side 0's folder
/// (and GunDelSols in side 1's: two sides with dimming chips would cut in
/// on each other).
fn dimming_setup() -> crate::setup::RoundSetup {
    let mut s = scenario::setup_with(&[testing::CUBE, testing::VEIL, testing::TRAP]);
    s.players[1] = scenario::setup().players[1];
    s
}

/// `dimming_setup`'s duel: the ticks each object kind was on the field, by
/// (pool, index) (and invisible navi-ticks under (Actor, 0xFF)), and the
/// battle at the end.
fn dimming_duel(ticks: usize) -> (std::collections::BTreeMap<(crate::object::Pool, u8), usize>, Battle) {
    let setup = dimming_setup;
    let tape = scenario::record_on(setup(), ticks, 5);
    let mut b = Battle::new(setup(), scenario::content());
    let mut seen = std::collections::BTreeMap::new();
    let mut invisible = 0;
    for t in &tape {
        b.tick(&t.input, t.events.clone());
        for r in b.objects.in_order() {
            *seen.entry((r.pool, b.objects.get(r).index)).or_insert(0) += 1;
        }
        for side in 0..2 {
            let Some(p) = b.player(side) else { continue };
            let c = b.objects.get(p).collision.unwrap();
            if b.collision.get(c).f1 & crate::collision::f1::INVISIBLE != 0 {
                invisible += 1;
            }
        }
    }
    seen.insert((crate::object::Pool::Actor, 0xFF), invisible);
    (seen, b)
}

#[test]
fn the_scripted_dimming_chips_and_rocks_play() {
    use crate::object::Pool::{Actor, Attack, Effect};
    let (seen, _) = dimming_duel(2400);
    let ticks = |k| seen.get(&k).copied().unwrap_or(0);
    // The cube's controller places rocks, which break into debris.
    assert!(ticks((Effect, 0x37)) > 0, "the cube's controller: {seen:?}");
    assert!(ticks((Attack, 0x59)) > 0, "rocks: {seen:?}");
    // The veil's controller makes its user invisible.
    assert!(ticks((Effect, 0x5D)) > 0, "the veil's controller: {seen:?}");
    assert!(ticks((Actor, 0xFF)) > 0, "an invisible navi: {seen:?}");
    // The trap's controller runs its hidden telop.
    assert!(ticks((Effect, 0x2A)) > 0, "the trap's controller: {seen:?}");
}

#[test]
fn scripted_dimming_chips_and_rocks_roll_back() {
    let setup = dimming_setup;
    let tape = scenario::record_on(setup(), 2400, 5);
    let mut b = Battle::new(setup(), scenario::content());
    let whole = digests(&tape, Battle::new(setup(), scenario::content()));
    for (i, t) in tape.iter().enumerate() {
        if i % 89 == 0 {
            let copy = digests(&tape[i..], b.clone());
            assert_eq!(copy, whole[i..], "the copy from tick {i} went its own way");
        }
        b.tick(&t.input, t.events.clone());
    }
}

/// A round on the test content's battle settings `settings` (panel
/// pattern 0x38: columns 1-3 are side 0's), not a link battle.
fn rock_battle() -> Battle {
    use crate::setup::NaviStats;
    let mut setup = testing::round_setup(testing::ROCK_BATTLE, NaviStats::from_bytes(&[0; 0x64]));
    setup.settings.effects = 0;
    Battle::new(setup, testing::content())
}

/// Step the objects of the given kinds, in list order and with the game's
/// pause and dimming gating (as `Battle::run_objects`).
fn run_only(b: &mut Battle, kinds: &[(crate::object::Pool, u8)]) {
    use crate::object::flags;
    let mut cur = b.objects.loop_first();
    while let Some(r) = cur {
        let o = b.objects.get(r);
        let gated = (b.paused && o.flags & flags::RUN_WHILE_PAUSED == 0)
            || (b.is_dimmed() && o.flags & flags::RUN_WHILE_DIMMED == 0);
        if !gated && kinds.contains(&(r.pool, o.index)) {
            crate::kinds::update(b, r);
        }
        cur = b.objects.loop_next();
    }
}

#[test]
fn actor_lists_place_scripted_rocks_outside_the_navi_bookkeeping() {
    use crate::object::Pool;
    use crate::setup::ActorKind;
    let mut b = rock_battle();
    let list = b.content.rules.stages.actor_list(testing::NAVIS_AND_ROCKS).clone();
    assert_eq!(
        list.entries.iter().map(|e| e.kind).collect::<Vec<_>>(),
        [ActorKind::Navi, ActorKind::Navi, ActorKind::Rock { variant: 1 }, ActorKind::Rock { variant: 1 }]
    );
    b.spawn_actors();
    assert_eq!(b.round.alive, [1, 1]);
    assert_eq!(b.round.name_counts, [1, 1]);
    let rocks: Vec<_> = b.objects.in_order().filter(|r| r.pool == Pool::Attack).collect();
    assert_eq!(rocks.len(), 2);
    let o = b.objects.get(rocks[0]);
    assert_eq!((o.index, o.params), (0x59, [1, 0, 3, 0]));
    assert_eq!((o.damage, o.stamina), (200, 0), "the stage's rocks hit with 200 when thrown");
    // Registered on their panels' sides: (3,3) is side 0's, (4,1) side 1's.
    assert_eq!(b.field.objects.slots[0], Some(rocks[0]));
    assert_eq!(b.field.objects.slots[3], Some(rocks[1]));
}

/// A rock broken by damage throws two debris chunks (a jitter draw each,
/// then two draws in each chunk's init) and a dust effect.
#[test]
fn breaking_a_scripted_rock_throws_debris() {
    use crate::object::{Pool, state};
    const ROCK_KINDS: [(Pool, u8); 3] = [(Pool::Attack, 0x59), (Pool::Effect, 0x38), (Pool::Effect, 0)];
    let mut b = rock_battle();
    b.spawn_actors();
    let r = b.objects.in_order().find(|r| r.pool == Pool::Attack).unwrap();
    // A fight in progress (with nobody alive the battle is over, and
    // obstacles break).
    b.round.flags |= crate::battle::battle_flags::FIGHTING;
    run_only(&mut b, &ROCK_KINDS);
    assert_eq!(b.objects.get(r).action, 8, "placed rocks stand at once");
    assert_eq!(b.objects.get(r).hp, b.content.objects.rock(1).hp);
    let before = b.rng.state;
    b.objects.get_mut(r).hp = 0;
    run_only(&mut b, &ROCK_KINDS);
    let mut rng = crate::rng::Rng::new(before);
    for _ in 0..6 {
        rng.next();
    }
    assert_eq!(b.rng.state, rng.state);
    let order: Vec<_> =
        b.objects.in_order().filter(|o| o.pool != Pool::Actor).map(|o| (o.pool, b.objects.get(o).index)).collect();
    assert_eq!(
        &order[..4],
        [(Pool::Attack, 0x59), (Pool::Effect, 0), (Pool::Effect, 0x38), (Pool::Effect, 0x38)],
        "the rock, then what it spawned in reverse order"
    );
    assert_eq!(b.objects.get(r).state, state::DESTROY);
    assert_eq!(b.field.objects.slots[0], None);
    run_only(&mut b, &ROCK_KINDS);
    assert!(!b.objects.is_allocated(r));
}

// ---- The navi-changing chips (subtype 38) ----------------------------------------------------

#[test]
fn the_navi_changing_chips_change_the_navi() {
    use crate::object::Pool::Effect;
    let setup = || {
        let mut s = scenario::setup_with(&[testing::BOOST, testing::ARM]);
        s.players[1] = scenario::setup().players[1];
        s
    };
    let tape = scenario::record_on(setup(), 2400, 5);
    let mut b = Battle::new(setup(), scenario::content());
    let before = b.stats[0].clone();
    let mut controllers = 0;
    for t in &tape {
        b.tick(&t.input, t.events.clone());
        controllers += b.objects.in_order().filter(|r| r.pool == Effect && b.objects.get(*r).index == 0x84).count();
    }
    assert!(controllers > 0, "the controller ran");
    let after = &b.stats[0];
    assert_eq!((after.rapid, after.charge, after.custom_level), (4, 4, 8), "{before:?}");
    assert!(after.float_shoes && after.air_shoes && after.undershirt);
    assert_eq!(after.weapons.charge_shot, 1, "the arm's charged shot");
    let copy = digests(&tape, Battle::new(setup(), scenario::content()));
    let mut b = Battle::new(setup(), scenario::content());
    for (i, t) in tape.iter().enumerate() {
        if i % 131 == 0 {
            assert_eq!(digests(&tape[i..], b.clone()), copy[i..], "the copy from tick {i} went its own way");
        }
        b.tick(&t.input, t.events.clone());
    }
}

// ---- The gauge chips (subtype 25) -----------------------------------------------------------

#[test]
fn the_slow_gauge_chip_slows_the_gauge() {
    let setup = || {
        let mut s = scenario::setup_with(&[testing::SLOW_GAUGE]);
        s.players[1] = scenario::setup().players[1];
        s
    };
    let tape = scenario::record_on(setup(), 2400, 5);
    let mut b = Battle::new(setup(), scenario::content());
    let mut slowed = false;
    for t in &tape {
        b.tick(&t.input, t.events.clone());
        slowed |= b.gauge.rate == 0x10 && b.sides[0].slow_gauge_ticks > 0;
    }
    assert!(slowed, "the gauge slowed");
}

// ---- Luau keeps no state ----------------------------------------------------------------

/// The test content with text replaced in one module (each `(from, to)`
/// once).
fn patched(module: &str, edits: &[(&str, &str)]) -> Content {
    let mut c = testing::build();
    let src = c.scripts.modules.get_mut(module).unwrap_or_else(|| panic!("no module {module}"));
    for (from, to) in edits {
        assert!(src.contains(from), "{module}.luau has no {from:?}");
        *src = src.replacen(from, to, 1);
    }
    c
}

fn load(content: &Content) -> Result<Behaviors, String> {
    Behaviors::load(content, Options::default()).map_err(|e| e.to_string())
}

/// Play the duel with `behaviors`: the content error it stopped on, if
/// any.
fn play_error(behaviors: Behaviors) -> Option<String> {
    let tape = scenario::record(500);
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scenario::play(&tape, behaviors)));
    r.err().map(|e| e.downcast_ref::<String>().cloned().unwrap_or_default())
}

const GUN_DEL_SOL: &str = "chips/001-sungun1/chip";
const UPDATE: &str = "function gun_del_sol.update(me: Object, s: State)\n";

/// GunDelSol's update with `line` added at its top.
fn in_update(line: &str) -> Content {
    patched(GUN_DEL_SOL, &[(UPDATE, &format!("{UPDATE}    {line}\n"))])
}

#[test]
fn state_in_module_locals_is_rejected_at_load() {
    let c = patched(
        "objects/sun-beam/sun_beam",
        &[("local sun_beam = {", "local hums = 0\nlocal sun_beam = {"), ("    s.ticks += 1\n", "    s.ticks += 1\n    hums += 1\n")],
    );
    let e = load(&c).err().expect("rejected");
    assert!(e.contains("assigns the module-level local `hums`"), "{e}");
}

#[test]
fn global_writes_are_rejected_at_load() {
    let e = load(&in_update("last_user = me")).err().expect("rejected");
    assert!(e.contains("assigns a global"), "{e}");
}

#[test]
fn module_tables_and_data_are_frozen() {
    for line in ["gun_del_sol.uses = me.step", "data.chips[me.chip].gun_del_sol.firing_ticks = 1", "math.floor = math.ceil"] {
        let e = play_error(load(&in_update(line)).unwrap()).expect("the write fails");
        assert!(e.contains("readonly"), "{line}: {e}");
    }
}

#[test]
fn tables_captured_by_functions_are_frozen() {
    let c = patched(
        GUN_DEL_SOL,
        &[("local gun_del_sol = {", "local seen = {}\nlocal gun_del_sol = {"), (UPDATE, &format!("{UPDATE}    seen[1] = me.step\n"))],
    );
    let e = play_error(load(&c).unwrap()).expect("the write fails");
    assert!(e.contains("readonly"), "{e}");
}

#[test]
fn nondeterministic_libraries_are_absent() {
    for call in ["math.random()", "math.sin(1)", "os.time()", "collectgarbage()", "coroutine.create(print)", "buffer.create(4)"]
    {
        let e = play_error(load(&in_update(&format!("local _ = {call}"))).unwrap())
            .unwrap_or_else(|| panic!("{call} ran"));
        assert!(e.contains("attempt to"), "{call}: {e}");
    }
}

#[test]
fn weak_tables_are_refused() {
    let b = load(&in_update("local _ = setmetatable({}, { __mode = \"k\" })")).unwrap();
    assert!(play_error(b).expect("refused").contains("weak tables"));
}

#[test]
fn fractions_cannot_enter_battle_state() {
    let b = load(&patched(GUN_DEL_SOL, &[("        s.timer = 6\n", "        s.timer = 13 / 2\n")])).unwrap();
    let e = play_error(b).expect("refused");
    assert!(e.contains("6.5 is not an integer"), "{e}");
}

#[test]
fn runaway_scripts_stop() {
    let b = load(&in_update("while true do end")).unwrap();
    assert!(play_error(b).expect("stopped").contains("past its budget"));
}

#[test]
fn content_errors_name_the_script() {
    let b = load(&in_update("error(\"boom\")")).unwrap();
    let e = play_error(b).expect("stopped");
    assert!(e.contains("action 0x37 (chips/001-sungun1/chip)") && e.contains("boom"), "{e}");
}

#[test]
fn the_vm_is_not_part_of_the_battle() {
    let tape = scenario::record(900);
    let shared = load(&testing::build()).unwrap();
    let want = digests(&tape, Battle::with_behaviors(scenario::setup(), scenario::content(), shared.clone()));
    // Halfway, move the battle to a fresh VM, as restoring a snapshot on
    // another machine would.
    let mut b = Battle::with_behaviors(scenario::setup(), scenario::content(), shared.clone());
    let mut have = Vec::new();
    for (i, t) in tape.iter().enumerate() {
        if i == 450 {
            b.behaviors = load(&testing::build()).unwrap();
        }
        b.tick(&t.input, t.events.clone());
        have.push(b.digest());
    }
    assert_eq!(have, want, "a fresh VM continues the battle identically");
    // Interleave a second battle, playing a different tape, on the same VM.
    let other = scenario::record_seeded(900, 11);
    let (mut a, mut c) = (
        Battle::with_behaviors(scenario::setup(), scenario::content(), shared.clone()),
        Battle::with_behaviors(scenario::setup(), scenario::content(), shared.clone()),
    );
    let mut have = Vec::new();
    for (t, u) in tape.iter().zip(other.iter()) {
        a.tick(&t.input, t.events.clone());
        have.push(a.digest());
        c.tick(&[u.input[1].clone(), u.input[0].clone()], u.events.clone());
    }
    let first = have.iter().zip(&want).position(|(a, b)| a != b);
    assert_eq!(first, None, "another battle on the same VM changes nothing");
}

/// An engine panic inside a script's API call unwinds through the Luau VM;
/// the VM stays sound for every other battle using it.
#[test]
fn a_panic_inside_content_leaves_the_vm_sound() {
    let tape = scenario::record(900);
    let shared = load(&testing::build()).unwrap();
    let want = digests(&tape, Battle::with_behaviors(scenario::setup(), scenario::content(), shared.clone()));
    let other = scenario::record_seeded(900, 11);
    let (mut a, mut c) = (
        Battle::with_behaviors(scenario::setup(), scenario::content(), shared.clone()),
        Battle::with_behaviors(scenario::setup(), scenario::content(), shared.clone()),
    );
    let mut have = Vec::new();
    let mut panics = 0;
    for (t, u) in tape.iter().zip(&other) {
        a.tick(&t.input, t.events.clone());
        have.push(a.digest());
        // A Rust panic inside a Luau call: the reactive abort GunDelSol
        // calls isn't ported and panics when a defense triggered.
        if let Some(p) = c.player(0)
            && c.objects.get(p).action == 0x37
        {
            let actor = c.objects.get(p).actor.unwrap();
            c.actors.get_mut(actor).requests |= crate::actor::request::ANTI_SWORD_TRIGGERED;
        }
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| c.tick(&u.input, u.events.clone()))).is_err() {
            panics += 1;
        }
    }
    let first = have.iter().zip(&want).position(|(x, y)| x != y);
    assert!(panics > 0, "the other battle should have panicked inside GunDelSol");
    assert_eq!(first, None, "a battle sharing the VM with {panics} panics diverged");
}

#[test]
fn gc_timing_does_not_reach_the_battle() {
    let tape = scenario::record(900);
    let want = digests(&tape, Battle::new(scenario::setup(), scenario::content()));
    let options = Options { collect_garbage: true, ..Default::default() };
    let b = Behaviors::load(&testing::build(), options).unwrap();
    let have = digests(&tape, Battle::with_behaviors(scenario::setup(), scenario::content(), b));
    assert_eq!(have, want);
}

#[cfg(feature = "luau-jit")]
#[test]
fn native_code_plays_the_duel_like_the_interpreter() {
    if !bn6_luau::native_code_supported() {
        return;
    }
    let tape = scenario::record(900);
    let want = digests(&tape, Battle::new(scenario::setup(), scenario::content()));
    let options = Options { native_code: true, ..Default::default() };
    let b = Behaviors::load(&testing::build(), options).unwrap();
    let have = digests(&tape, Battle::with_behaviors(scenario::setup(), scenario::content(), b));
    assert_eq!(have, want);
}
