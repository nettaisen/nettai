//! Compare a pack's animation timing (loaded at run time, no images) with
//! the table compiled into the engine today (`bn6_battle::data::animation`).
//!
//!     cargo run -p bn6-content --example engine_timing -- <pack>

use bn6_battle::data::{SpriteId, animation};
use bn6_content::report::Report;
use bn6_content::timing;

fn main() {
    let pack = std::env::args().nth(1).expect("usage: engine_timing <pack>");
    let mut r = Report::default();
    let t = timing::load(std::path::Path::new(&pack), &mut r).unwrap_or_else(|| panic!("{r}"));
    let (mut anims, mut frames, mut differ) = (0, 0, 0);
    for (&(category, index), sprite) in &t.sprites {
        for (a, fr) in sprite.iter().enumerate() {
            anims += 1;
            frames += fr.len();
            let engine: Vec<(u8, u8)> = animation(SpriteId { category, index }, a as u8).iter().map(|f| (f.duration, f.flags)).collect();
            let pack: Vec<(u8, u8)> = fr.iter().map(|f| (f.ticks, f.flags)).collect();
            if engine != pack {
                differ += 1;
                if differ <= 10 {
                    println!("sprite {category:02x}-{index:02x} animation {a}: engine {engine:?}, pack {pack:?}");
                }
            }
        }
    }
    println!("{} sprites, {anims} animations, {frames} frames: {differ} animations differ from the engine's table", t.sprites.len());
}
