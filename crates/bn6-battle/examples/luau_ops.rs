//! What single content-API operations cost from Luau: each is run 200,000
//! times inside one sun beam update (the pack's sun beam, patched), and
//! the tick's time over a plain tick is divided out.
//!
//! cargo run --release -p bn6-battle --example luau_ops --features luau
//! (add `luau-jit` to also time native code)

use std::time::Instant;

use bn6_battle::Battle;
use bn6_battle::behavior::{self, Behaviors, luau_pack};
use bn6_battle::input::PlayerTick;
use bn6_battle::object::{Pool, Vec3};
use bn6_battle::scenario;
use bn6_content_api::Value;

const N: u32 = 200_000;

const OPS: [(&str, &str); 14] = [
    ("loop overhead", "local _ = i"),
    ("field read `me.anim`", "local _ = me.anim"),
    ("field write `me.anim = 3`", "me.anim = 3"),
    ("method `me:param(1)`", "local _ = me:param(1)"),
    ("state read `s.ticks`", "local _ = s.ticks"),
    ("state write `s.ticks = 5`", "s.ticks = 5"),
    ("enum state read `s.slot`", "local _ = s.slot"),
    ("library call `battle.time_stop()`", "local _ = battle.time_stop()"),
    ("string field `me.lifecycle`", "local _ = me.lifecycle"),
    ("Vec3 field `me.pos`", "local _ = me.pos"),
    ("handle field `me.related1`", "local _ = me.related1"),
    ("handle field `me.sprite`", "local _ = me.sprite"),
    ("`Vec3.new(1, 2, 3)`", "local _ = Vec3.new(1, 2, 3)"),
    ("9-field table literal", "local _ = { a = 1, b = 2, c = 3, d = 4, e = 5, f = 6, g = 7, h = 8, i = 9 }"),
];

fn main() {
    let tape = scenario::record(300);
    let idle = [PlayerTick::default(), PlayerTick::default()];
    let natives: &[bool] = if bn6_luau::native_code_supported() { &[false, true] } else { &[false] };
    println!("| operation | interpreted |{}", if natives.len() > 1 { " native |" } else { "" });
    println!("|---|---|{}", if natives.len() > 1 { "---|" } else { "" });
    for (name, op) in OPS {
        let full = luau_pack();
        let pack = bn6_luau::Pack::new(full.modules().map(|(p, s)| {
            let s = if p == "objects/sun_beam" {
                let hook = "    if s.ticks % 11 == 0 then";
                s.replacen(hook, &format!("    if me.timer2 == 7 then for i = 1, {N} do {op} end end\n{hook}"), 1)
            } else {
                s.to_string()
            };
            (p.to_string(), s)
        }));
        let mut row = format!("| {name} |");
        for &native in natives {
            let options = bn6_luau::Options { native_code: native, budget: u32::MAX, ..Default::default() };
            let mut b = Battle::with_behaviors(
                scenario::setup(),
                Behaviors::new(bn6_luau::LuauContent::load(&pack, options).unwrap()).unwrap(),
            );
            for t in &tape[..250] {
                b.tick(&t.input, t.events.clone());
            }
            // A sun beam on player 0 that runs `op` N times when its timer2 is 7.
            let owner = b.player(0).unwrap();
            let r = behavior::spawn_object(&mut b, Pool::Effect, 0x48, Vec3::default(), [0, 2, 0, 0]).unwrap();
            b.objects.get_mut(r).related[0] = Some(owner);
            behavior::set_state_field(&mut b, r, "slot", Value::Int(1));
            b.objects.get_mut(owner).related[0] = Some(r);
            b.tick(&idle, Default::default());
            let t = Instant::now();
            b.tick(&idle, Default::default());
            let plain = t.elapsed();
            b.objects.get_mut(r).timer2 = 7;
            let t = Instant::now();
            b.tick(&idle, Default::default());
            let ns = t.elapsed().saturating_sub(plain).as_secs_f64() * 1e9 / N as f64;
            row += &format!(" {ns:.0} ns |");
        }
        println!("{row}");
    }
}
