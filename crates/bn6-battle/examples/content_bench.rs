//! Costs of running content scripts: the synthetic GunDelSol duel
//! (`bn6_battle::scenario`, on the test content, whose scripts are this
//! repository's BN6 scripts), scripted objects in bulk, and snapshots, with
//! the Luau interpreter and, where available, native code. Self-contained.
//!
//! cargo run --release -p bn6-battle --example content_bench --features test-content
//! (add `luau-jit` for Luau native code)

use std::hint::black_box;
use std::time::{Duration, Instant};

use bn6_battle::Battle;
use bn6_battle::behavior::{self, Behaviors, Options};
use bn6_battle::content::testing;
use bn6_battle::input::PlayerTick;
use bn6_battle::object::{Vec3, flags};
use bn6_battle::scenario::{self, Tick};
use bn6_content_api::Value;

fn runtimes() -> Vec<(&'static str, Options)> {
    let mut v = vec![("luau", Options::default())];
    if bn6_luau::native_code_supported() {
        v.push(("luau native", Options { native_code: true, ..Default::default() }));
    }
    v
}

fn at(tape: &[Tick], options: Options, ticks: usize) -> Battle {
    let behaviors = Behaviors::load(&testing::build(), options).unwrap();
    let mut b = Battle::with_behaviors(scenario::setup(), scenario::content(), behaviors);
    for t in &tape[..ticks] {
        b.tick(&t.input, t.events.clone());
    }
    b
}

/// Time `f` over `reps` runs, returning the mean.
fn time(reps: u32, mut f: impl FnMut()) -> Duration {
    f();
    let t = Instant::now();
    for _ in 0..reps {
        f();
    }
    t.elapsed() / reps
}

fn us(d: Duration) -> String {
    format!("{:.2} µs", d.as_secs_f64() * 1e6)
}

/// Attach `n` scripted objects to player 0: attachments (in the overlay
/// slot) and sun beams (in the related slot), alternating. Each follows its
/// owner every tick until the slot is cleared, which here it never is.
fn attach(b: &mut Battle, n: usize) {
    let owner = b.player(0).unwrap();
    let actor = b.objects.get(owner).actor.unwrap();
    let offset = Vec3::px(80, 0, 0);
    for i in 0..n {
        let beam = i % 2 == 1;
        let (panel, alliance, flip) = {
            let o = b.objects.get(owner);
            (o.panel, o.alliance, o.flip)
        };
        let r = if beam {
            behavior::spawn_kind(b, "sun-beam", offset, [0, 1, 0, 0]).unwrap()
        } else {
            behavior::spawn_kind(b, "attachment", Vec3::default(), [2, 0, 0, 0]).unwrap()
        };
        let o = b.objects.get_mut(r);
        o.related[0] = Some(owner);
        (o.alliance, o.flip) = (alliance, flip);
        if beam {
            behavior::set_state_variant(b, r, "slot", "related");
            behavior::set_state_field(b, r, "offset", Value::Vec3(offset));
            b.objects.get_mut(owner).related[0] = Some(r);
        } else {
            o.panel = panel;
            o.flags |= flags::RUN_WHILE_PAUSED | flags::RUN_WHILE_DIMMED;
            behavior::set_state_variant(b, r, "slot", "overlay");
            b.actors.get_mut(actor).overlay = Some(r);
        }
    }
}

fn main() {
    let tape = scenario::record(900);
    let idle = [PlayerTick::default(), PlayerTick::default()];
    println!("Luau native code supported here: {}", bn6_luau::native_code_supported());
    println!("size_of::<Battle>() = {} bytes (plus heap: objects and sprites)\n", std::mem::size_of::<Battle>());

    let t = Instant::now();
    black_box(Behaviors::load(&testing::build(), Options::default()).unwrap());
    println!("Loading the scripts (VM, data, compile, verify, freeze): {}\n", us(t.elapsed()));

    println!(
        "| runtime | duel, whole tape (900 ticks) | duel, GunDelSol firing (ticks 300-900) | 30 objects × 10,000 ticks: per object-tick | snapshot (clone) | restore | rollback frame: snapshot + restore + 10 ticks |"
    );
    println!("|---|---|---|---|---|---|---|");
    for (name, options) in runtimes() {
        let behaviors = Behaviors::load(&testing::build(), options).unwrap();
        // The whole duel from the start of the round.
        let whole = time(20, || {
            black_box(scenario::play(&tape, behaviors.clone()));
        }) / tape.len() as u32;

        // Steady state while GunDelSols fire.
        let base = at(&tape, options, 300);
        let mut b = base.clone();
        let firing = time(50, || {
            b.clone_from(&base);
            for t in &tape[300..900] {
                b.tick(&t.input, t.events.clone());
            }
        }) / 600;

        // 30 scripted objects following their owner for 10,000 ticks, minus
        // the same battle without them.
        let ticks = 10_000;
        let run = |n: usize| {
            let mut b = at(&tape, options, 250);
            attach(&mut b, n);
            let t = Instant::now();
            for _ in 0..ticks {
                b.tick(&idle, Default::default());
            }
            t.elapsed()
        };
        let (with, without) = (run(30), run(0));
        let per_object = with.saturating_sub(without) / (ticks * 30);

        // Snapshots mid-GunDelSol, and a rollback frame: take a snapshot,
        // restore an older one, re-simulate 10 ticks.
        let base = at(&tape, options, 400);
        let mut b = base.clone();
        let snapshot = time(2000, || {
            black_box(b.clone());
        });
        let restore = time(2000, || b.clone_from(&base));
        let frame = time(200, || {
            black_box(b.clone());
            b.clone_from(&base);
            for t in &tape[400..410] {
                b.tick(&t.input, t.events.clone());
            }
        });
        println!(
            "| {name} | {} / tick | {} / tick | {} | {} | {} | {} |",
            us(whole),
            us(firing),
            us(per_object),
            us(snapshot),
            us(restore),
            us(frame)
        );
    }
    println!("\nFrame budget at 60 Hz: 16,667 µs.");
}
