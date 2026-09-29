//! Costs of running content through the content API, per runtime: the
//! engine's built-in kinds, the slice as Rust content, and the slice as
//! Luau (interpreted, and native where available). Self-contained: it
//! plays the synthetic GunDelSol duel (`bn6_battle::scenario`).
//!
//! cargo run --release -p bn6-battle --example content_bench --features luau,rust-content
//! (add `luau-jit` for Luau native code)

use std::hint::black_box;
use std::time::{Duration, Instant};

use bn6_battle::Battle;
use bn6_battle::content::{self, Content};
use bn6_battle::data::attacks::SunBeamLook;
use bn6_battle::input::PlayerTick;
use bn6_battle::kinds::attachment::{self, AttachSlot};
use bn6_battle::kinds::sun_beam;
use bn6_battle::object::{Pool, Vec3, flags};
use bn6_battle::scenario::{self, Tick};
use bn6_content_api::Value;

fn runtimes() -> Vec<(&'static str, Box<dyn Fn() -> Content>)> {
    let mut v: Vec<(&'static str, Box<dyn Fn() -> Content>)> = vec![
        ("built-in", Box::new(Content::builtin)),
        ("rust content", Box::new(Content::rust)),
        ("luau", Box::new(|| Content::luau().unwrap())),
    ];
    if bn6_luau::native_code_supported() {
        v.push((
            "luau native",
            Box::new(|| Content::luau_with(bn6_luau::Options { native_code: true, ..Default::default() }).unwrap()),
        ));
    }
    v
}

fn at(tape: &[Tick], content: Content, ticks: usize) -> Battle {
    let mut b = Battle::with_content(scenario::setup(), content);
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

/// Attach `n` objects of the slice to player 0: attachments (in the
/// overlay slot) and sun beams (in the related slot), alternating. Each
/// follows its owner every tick until the slot is cleared, which here it
/// never is.
fn attach(b: &mut Battle, n: usize) {
    let owner = b.player(0).unwrap();
    let actor = b.objects.get(owner).actor.unwrap();
    let offset = Vec3::px(80, 0, 0);
    let look = SunBeamLook { sprite: 0, palette: 2 };
    for i in 0..n {
        let beam = i % 2 == 1;
        if b.content.object_kind(Pool::Actor, attachment::INDEX).is_none() {
            if beam {
                sun_beam::spawn(b, owner, look, offset, AttachSlot::Related(owner)).unwrap();
            } else {
                attachment::spawn(b, owner, 9, AttachSlot::Overlay(actor)).unwrap();
            }
            continue;
        }
        let (panel, alliance, flip) = {
            let o = b.objects.get(owner);
            (o.panel, o.alliance, o.flip)
        };
        let r = if beam {
            content::spawn_object(b, Pool::Effect, sun_beam::INDEX, offset, [look.sprite, look.palette, 0, 0]).unwrap()
        } else {
            content::spawn_object(b, Pool::Actor, attachment::INDEX, Vec3::default(), [9, 0, 0, 0]).unwrap()
        };
        let o = b.objects.get_mut(r);
        o.related[0] = Some(owner);
        (o.alliance, o.flip) = (alliance, flip);
        if beam {
            content::set_state_field(b, r, "slot", Value::Int(1));
            content::set_state_field(b, r, "offset", Value::Vec3(offset));
            b.objects.get_mut(owner).related[0] = Some(r);
        } else {
            o.panel = panel;
            o.flags |= flags::RUN_WHILE_PAUSED | flags::RUN_IN_TIME_STOP;
            content::set_state_field(b, r, "slot", Value::Int(0));
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
    black_box(Content::luau().unwrap());
    println!("Loading the Luau pack (VM, compile, verify, freeze): {}\n", us(t.elapsed()));

    println!("| runtime | duel, whole tape (900 ticks) | duel, GunDelSol firing (ticks 300-900) | 30 objects × 10,000 ticks: per object-tick | snapshot (clone) | restore | rollback frame: snapshot + restore + 10 ticks |");
    println!("|---|---|---|---|---|---|---|");
    for (name, make) in runtimes() {
        // The whole duel from the start of the round.
        let whole = time(20, || {
            black_box(scenario::play(&tape, make()));
        }) / tape.len() as u32;

        // Steady state while GunDelSols fire.
        let base = at(&tape, make(), 300);
        let mut b = base.clone();
        let firing = time(50, || {
            b.clone_from(&base);
            for t in &tape[300..900] {
                b.tick(&t.input, t.events.clone());
            }
        }) / 600;

        // 30 slice objects following their owner for 10,000 ticks, minus
        // the same battle without them.
        let ticks = 10_000;
        let run = |n: usize| {
            let mut b = at(&tape, make(), 250);
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
        let base = at(&tape, make(), 400);
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
