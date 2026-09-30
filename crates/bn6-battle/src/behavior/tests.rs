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
            "blast-fire",
            "blast-man",
            "charge-car",
            "charge-man",
            "dust-ball",
            "elec-man",
            "elec-thunder",
            "elmnt-bolt",
            "elmnt-ice",
            "elmnt-man",
            "elmnt-vine",
            "erase-beam",
            "erase-man",
            "erase-mark",
            "flying-shot",
            "grab-shot",
            "heat-flame",
            "heat-man",
            "meteor",
            "projectile",
            "slash-man",
            "slash-wave",
            "spout-ball",
            "spout-geyser",
            "spout-man",
            "spout-mark",
            "spout-pillar",
            "spout-splash",
            "sun-beam",
            "tengu-man",
            "tomahawk-man"
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
    duel_with(&[testing::ERASER, testing::GRAB, testing::SUN_GUN_3], ticks, 11)
}

/// A duel with `chips` in the folders and the players' moves from `seed`:
/// the ticks each kind was on the field, by (pool, index).
fn duel_with(chips: &[crate::content::ChipId], ticks: usize, seed: u32) -> std::collections::BTreeMap<(crate::object::Pool, u8), usize> {
    let setup = || scenario::setup_with(chips);
    let tape = scenario::record_on(setup(), ticks, seed);
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
fn the_elements_navi_attacks() {
    use crate::object::Pool::{Actor, Attack};
    let seen = duel_with(&[testing::ELEMENTS], 2400, 11);
    let ticks = |k| seen.get(&k).copied().unwrap_or(0);
    // The elements navi comes with his overlay and attacks with an
    // element: meteors, ice, bolts or vines.
    assert!(ticks((Actor, 0x10)) > 0, "ElmntMan: {seen:?}");
    assert!(ticks((Actor, 0x56)) > 0, "ElmntMan's overlay: {seen:?}");
    let attacks = [0x8D, 0x8E, 0xB8, 0xB9].map(|i| ticks((Attack, i)));
    assert!(attacks.iter().any(|&t| t > 0), "ElmntMan's attacks: {seen:?}");
}

#[test]
fn the_water_navi_attacks() {
    use crate::object::Pool::{Actor, Attack, Effect};
    let seen = duel_with(&[testing::SPOUT], 2400, 11);
    let ticks = |k| seen.get(&k).copied().unwrap_or(0);
    // The water navi comes in his water (his layer) and throws his ball
    // or raises his geyser, which marks its column.
    assert!(ticks((Actor, 0x09)) > 0, "SpoutMan: {seen:?}");
    assert!(ticks((Actor, 0x55)) > 0, "SpoutMan's layer: {seen:?}");
    let ball = ticks((Attack, 0x22)) > 0 && ticks((Attack, 0x23)) > 0;
    let geyser = ticks((Effect, 0x2D)) > 0 && ticks((Attack, 0x17)) > 0 && ticks((Effect, 0x2E)) > 0;
    assert!(ball || geyser, "SpoutMan's attacks: {seen:?}");
}

#[test]
fn the_navi_chip_navis_come_and_go() {
    use crate::object::Pool::Actor;
    // Each navi chip's navi comes (and the duel goes on without a content
    // error); its attacks depend on where the players stand.
    for (chip, navi) in [
        (testing::HEAT, 0x07),
        (testing::ELEC, 0x08),
        (testing::SLASH, 0x0D),
        (testing::CHARGE, 0x16),
        (testing::TOMAHAWK, 0x0A),
        (testing::TENGU, 0x0C),
        (testing::BLAST, 0x06),
    ] {
        let seen = duel_with(&[chip], 1500, 11);
        assert!(seen.get(&(Actor, navi)).copied().unwrap_or(0) > 0, "navi {navi:#x} of chip {chip:#x}: {seen:?}");
    }
}

#[test]
fn scripted_chips_roll_back() {
    // A copy of the battle taken at any tick plays on exactly as the
    // battle does: the scripts' state is all in the battle.
    for chips in [&[testing::ERASER, testing::GRAB, testing::SUN_GUN_3][..], &[testing::ELEMENTS], &[testing::SPOUT], &[testing::HEAT, testing::ELEC, testing::SLASH, testing::CHARGE, testing::TOMAHAWK, testing::TENGU, testing::BLAST]] {
        let setup = || scenario::setup_with(chips);
        let tape = scenario::record_on(setup(), 2400, 11);
        let mut b = Battle::new(setup(), scenario::content());
        let whole = digests(&tape, Battle::new(setup(), scenario::content()));
        for (i, t) in tape.iter().enumerate() {
            if i % 97 == 0 {
                let copy = digests(&tape[i..], b.clone());
                assert_eq!(copy, whole[i..], "the copy from tick {i} went its own way ({chips:?})");
            }
            b.tick(&t.input, t.events.clone());
        }
    }
}

#[test]
fn registrations_follow_the_content_data() {
    let mut c = testing::build();
    let r = c.registrations().unwrap();
    // The four SunGun chips share one action; four weapons have theirs (the
    // buster's alias names none).
    let actions: Vec<u8> = r.actions.iter().map(|a| a.action).collect();
    assert_eq!(actions, [0x11, 0x16, 0x33, 0x37, 0x57], "{:?}", r.actions);
    // Two chips implementing one action with different scripts is an error.
    c.chips[testing::SUN_GUN_2 as usize].script = Some("objects/sun-beam/sun_beam".into());
    let e = c.registrations().unwrap_err();
    assert!(e.contains("implements action 0x37"), "{e}");
    // So is naming a script the pack doesn't have.
    let mut c = testing::build();
    c.objects.kinds[0].script = "objects/nowhere".into();
    assert!(c.registrations().unwrap_err().contains("isn't in the pack"));
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
