//! What single content-API operations cost from Luau: each is run 200,000
//! times inside one sun beam update (the test content's sun beam script,
//! patched), and the tick's time over a plain tick is divided out.
//!
//! cargo run --release -p nettai-battle --example luau_ops --features test-content
//! (add `luau-jit` to also time native code)

use std::time::Instant;

use nettai_battle::Battle;
use nettai_battle::behavior::{self, Behaviors, Options};
use nettai_battle::content::testing;
use nettai_battle::input::PlayerTick;
use nettai_battle::object::Vec3;
use nettai_battle::scenario;
use nettai_content_api::{AssetKind, Value};

const N: u32 = 200_000;

const OPS: [(&str, &str); 15] = [
    ("loop overhead", "local _ = i"),
    ("field read `me.anim`", "local _ = me.anim"),
    ("field write `me.anim = 3`", "me.anim = 3"),
    ("method `me:set_animation(0)`", "me:set_animation(0)"),
    ("state read `s.ticks`", "local _ = s.ticks"),
    ("state write `s.ticks = 5`", "s.ticks = 5"),
    ("enum state read `s.slot`", "local _ = s.slot"),
    ("library call `battle.dimmed()`", "local _ = battle.dimmed()"),
    ("panel flags `field.flags(3, 2)`", "local _ = field.flags(3, 2)"),
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
    let natives: &[bool] = if nettai_luau::native_code_supported() { &[false, true] } else { &[false] };
    println!("| operation | interpreted |{}", if natives.len() > 1 { " native |" } else { "" });
    println!("|---|---|{}", if natives.len() > 1 { "---|" } else { "" });
    for (name, op) in OPS {
        let mut content = testing::build();
        let beam = content.scripts.modules.get_mut("chips/gundels/beam").expect("the sun beam script");
        let hook = "    if s.ticks % HUM_TICKS == 0 then";
        assert!(beam.contains(hook), "the sun beam script has no {hook:?}");
        *beam = beam.replacen(hook, &format!("    if me.timer2 == 7 then for i = 1, {N} do {op} end end\n{hook}"), 1);
        let mut row = format!("| {name} |");
        for &native in natives {
            let options = Options { native_code: native, budget: u32::MAX, ..Default::default() };
            let behaviors = Behaviors::load(&content, options).unwrap();
            let ns = behavior::with_runtime(&behaviors, || {
                let mut b = Battle::new(scenario::setup(), scenario::content());
                for t in &tape[..250] {
                    b.tick(&t.input, t.events.clone());
                }
                // A sun beam on player 0 that runs `op` N times when its timer2 is 7.
                let owner = b.player(0).unwrap();
                let r = behavior::spawn_kind(&mut b, "gundels/beam", Vec3::default()).unwrap();
                b.objects.get_mut(r).related[0] = Some(owner);
                let look = b.content.assets.handle(AssetKind::Sprite, "sun-beam").expect("the sun beam's sprite");
                behavior::set_state_field(&mut b, r, "sprite", Value::Asset(AssetKind::Sprite, look));
                behavior::set_state_field(&mut b, r, "palette", Value::Int(2));
                behavior::set_state_variant(&mut b, r, "slot", "related");
                b.objects.get_mut(owner).related[0] = Some(r);
                b.tick(&idle, Default::default());
                let t = Instant::now();
                b.tick(&idle, Default::default());
                let plain = t.elapsed();
                b.objects.get_mut(r).timer2 = 7;
                let t = Instant::now();
                b.tick(&idle, Default::default());
                t.elapsed().saturating_sub(plain).as_secs_f64() * 1e9 / N as f64
            });
            row += &format!(" {ns:.0} ns |");
        }
        println!("{row}");
    }
}
