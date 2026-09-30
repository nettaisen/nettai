//! Behaviors-layer tests: every content runtime plays the GunDelSol
//! scenario tick for tick like the built-in kinds, with the scripts' data
//! from the battle's content (here the hand-authored test content).

// ---- Every runtime plays the same battle -------------------------------------------

use crate::battle::Battle;
use crate::behavior::Behaviors;
use crate::scenario;

/// The digest of what every runtime represents the same way: everything
/// but the kinds' and actions' own state structs (`Vars`, `ActionVars`),
/// whose representation differs between the built-in kinds and content.
/// What those kinds do to shared state is all still in.
fn engine_digest(b: &Battle) -> u64 {
    use crate::object::{ObjectRef, Pool, SLOTS};
    let mut b = b.clone();
    for pool in Pool::ALL {
        for slot in 0..SLOTS as u8 {
            b.objects.get_mut(ObjectRef { pool, slot }).vars = crate::kinds::Vars::None;
        }
    }
    for i in 0..crate::actor::SLOTS as u8 {
        b.actors.get_mut(crate::actor::ActorId(i)).attack.action = Default::default();
    }
    b.digest()
}

/// Each tick's digest of what every runtime represents the same way,
/// with the sound cues it made.
fn engine_digests(tape: &[scenario::Tick], mut b: Battle) -> Vec<(u64, Vec<crate::sound::SoundCue>)> {
    tape.iter()
        .map(|t| {
            b.tick(&t.input, t.events.clone());
            (engine_digest(&b), b.sound_cues().to_vec())
        })
        .collect()
}

#[test]
fn every_runtime_plays_the_duel_like_the_engine() {
    let tape = scenario::record(900);
    let want = engine_digests(&tape, Battle::with_behaviors(scenario::setup(), scenario::content(), Behaviors::builtin()));
    #[allow(unused_mut)]
    let mut runs: Vec<(&str, Behaviors)> = Vec::new();
    #[cfg(feature = "rust-content")]
    runs.push(("rust", Behaviors::rust(&scenario::content())));
    #[cfg(feature = "luau")]
    runs.push(("luau", Behaviors::luau(&scenario::content()).unwrap()));
    #[cfg(feature = "luau-jit")]
    runs.push((
        "luau native",
        Behaviors::luau_with(&scenario::content(), bn6_luau::Options { native_code: true, ..Default::default() }).unwrap(),
    ));
    for (name, content) in runs {
        let have = engine_digests(&tape, Battle::with_behaviors(scenario::setup(), scenario::content(), content));
        let first = have.iter().zip(&want).position(|(a, b)| a != b);
        assert_eq!(first, None, "{name} content diverges from the engine's kinds at tick {first:?}");
    }
}

// ---- Luau keeps no state ----------------------------------------------------------------

#[cfg(feature = "luau")]
mod luau {
    use super::*;
    use crate::behavior::luau_pack;

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
    fn luau_is_the_default_content_with_the_feature() {
        assert_eq!(Battle::new(scenario::setup(), scenario::content()).behaviors.runtime(), "luau");
    }

    /// The pack with text replaced in one module (each `(from, to)` once).
    fn patched(module: &str, edits: &[(&str, &str)]) -> bn6_luau::Pack {
        bn6_luau::Pack::new(luau_pack(&scenario::content()).modules().map(|(path, src)| {
            let mut src = src.to_string();
            if path == module {
                for (from, to) in edits {
                    assert!(src.contains(from), "{module}.luau has no {from:?}");
                    src = src.replacen(from, to, 1);
                }
            }
            (path.to_string(), src)
        }))
    }

    fn load(pack: &bn6_luau::Pack) -> Result<Behaviors, String> {
        bn6_luau::LuauContent::load(pack, bn6_luau::Options::default())
            .and_then(Behaviors::new)
            .map_err(|e| e.to_string())
    }

    /// Play the duel with `content`: the content error it stopped on, if
    /// any.
    fn play_error(content: Behaviors) -> Option<String> {
        let tape = scenario::record(500);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scenario::play(&tape, content)));
        r.err().map(|e| e.downcast_ref::<String>().cloned().unwrap_or_default())
    }

    const UPDATE: &str = "function gun_del_sol.update(me: Object, s: State)\n";

    /// GunDelSol's update with `line` added at its top.
    fn in_update(line: &str) -> bn6_luau::Pack {
        patched("chips/gun_del_sol", &[(UPDATE, &format!("{UPDATE}    {line}\n"))])
    }

    #[test]
    fn state_in_module_locals_is_rejected_at_load() {
        let pack = patched(
            "objects/sun_beam",
            &[
                ("local sun_beam = {", "local hums = 0\nlocal sun_beam = {"),
                ("    s.ticks += 1\n", "    s.ticks += 1\n    hums += 1\n"),
            ],
        );
        let e = load(&pack).err().expect("rejected");
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
        let pack = patched(
            "chips/gun_del_sol",
            &[
                ("local gun_del_sol = {", "local seen = {}\nlocal gun_del_sol = {"),
                (UPDATE, &format!("{UPDATE}    seen[1] = me.step\n")),
            ],
        );
        let e = play_error(load(&pack).unwrap()).expect("the write fails");
        assert!(e.contains("readonly"), "{e}");
    }

    #[test]
    fn nondeterministic_libraries_are_absent() {
        for call in [
            "math.random()",
            "math.sin(1)",
            "os.time()",
            "collectgarbage()",
            "coroutine.create(print)",
            "buffer.create(4)",
        ] {
            let e = play_error(load(&in_update(&format!("local _ = {call}"))).unwrap())
                .unwrap_or_else(|| panic!("{call} ran"));
            assert!(e.contains("attempt to"), "{call}: {e}");
        }
    }

    #[test]
    fn weak_tables_are_refused() {
        let content = load(&in_update("local _ = setmetatable({}, { __mode = \"k\" })")).unwrap();
        assert!(play_error(content).expect("refused").contains("weak tables"));
    }

    #[test]
    fn fractions_cannot_enter_battle_state() {
        let content =
            load(&patched("chips/gun_del_sol", &[("        s.timer = 6\n", "        s.timer = 13 / 2\n")])).unwrap();
        let e = play_error(content).expect("refused");
        assert!(e.contains("6.5 is not an integer"), "{e}");
    }

    #[test]
    fn runaway_scripts_stop() {
        let content = load(&in_update("while true do end")).unwrap();
        assert!(play_error(content).expect("stopped").contains("past its budget"));
    }

    #[test]
    fn the_vm_is_not_part_of_the_battle() {
        let tape = scenario::record(900);
        let shared = Behaviors::luau(&scenario::content()).unwrap();
        let want = digests(&tape, Battle::with_behaviors(scenario::setup(), scenario::content(), shared.clone()));
        // Halfway, move the battle to a fresh VM, as restoring a snapshot
        // on another machine would.
        let mut b = Battle::with_behaviors(scenario::setup(), scenario::content(), shared.clone());
        let mut have = Vec::new();
        for (i, t) in tape.iter().enumerate() {
            if i == 450 {
                b.behaviors = Behaviors::luau(&scenario::content()).unwrap();
            }
            b.tick(&t.input, t.events.clone());
            have.push(b.digest());
        }
        assert_eq!(have, want, "a fresh VM continues the battle identically");
        // Interleave a second battle, playing a different tape, on the
        // same VM.
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

    /// An engine panic inside a script's API call unwinds through the Luau
    /// VM; the VM stays sound for every other battle using it.
    #[test]
    fn a_panic_inside_content_leaves_the_vm_sound() {
        let tape = scenario::record(900);
        let shared = Behaviors::luau(&scenario::content()).unwrap();
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
            // calls isn't implemented and panics when a defense triggered.
            if let Some(p) = c.player(0) {
                if c.objects.get(p).action == 0x37 {
                    let actor = c.objects.get(p).actor.unwrap();
                    c.actors.get_mut(actor).requests |= crate::actor::request::ANTI_SWORD_TRIGGERED;
                }
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
        let want = digests(&tape, Battle::with_behaviors(scenario::setup(), scenario::content(), Behaviors::luau(&scenario::content()).unwrap()));
        let options = bn6_luau::Options { collect_garbage: true, ..Default::default() };
        let have = digests(&tape, Battle::with_behaviors(scenario::setup(), scenario::content(), Behaviors::luau_with(&scenario::content(), options).unwrap()));
        assert_eq!(have, want);
    }
}
