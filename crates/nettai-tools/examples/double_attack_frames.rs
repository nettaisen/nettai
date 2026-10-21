//! The frames of one of EXE5's Double Attacks (the DS's party battle:
//! content/exe5/double_attacks), drawn from the pack, for a frame comparison
//! with the DS's captures (the verification workspace's dslab `--frames`).
//!
//!     cargo run -p nettai-tools --example double_attack_frames -- PACK OUT KEY PARTNER [PARTNER] [--key-a TICK...]
//!
//! A round of MegaMan (side 0, with the partners in his party) against
//! MegaMan (both light, the value 1000, as the DS lab's saves) on the first
//! link stage, both pressing through their custom
//! screens; then side 0 switches: to member 1 out of Full Synchro (a pair
//! with MegaMan), or to member 1 out of a plain mood and, once that has
//! settled, to member 2 out of Full Synchro (a pair of two team navis). Every
//! tick from the switch that fires the Double Attack until its director has
//! gone is drawn (original graphics, 1x) as `OUT/nettai_NNNN.png`, NNNN the
//! ticks since the tick its telop's sound (0x173) played, and again without
//! the Double Attack's own drawings as `OUT/bare_NNNN.png` (what a comparison
//! of its drawings alone masks with); the sound calls and its objects (panel,
//! animation, palette) are listed in `OUT/ticks.tsv`. `--key-a` holds A
//! on those ticks (counted the same way, negative before the telop):
//! C-SCOPE's stop.

use nettai_battle::battle::{fight, mode};
use nettai_battle::input::{PlayerTick, keys};
use nettai_battle::setup::NaviStats;
use nettai_battle::Battle;
use nettai_frontend::game::{Options, load};
use nettai_render::GraphicsMode;
use std::io::Write;
use std::path::PathBuf;

/// A folder of Tango's EXE5 saves (their first): Standard chips only.
const FOLDER: [&str; 30] = [
    "cannon A", "cannon A", "cannon B", "cannon B", "airshot *", "airshot *", "airshot *",
    "vulcan1 D", "vulcan1 D", "vulcan1 D", "minibomb B", "minibomb B", "minibomb L", "minibomb L",
    "sword S", "sword S", "sword S", "sword S", "wideswrd S", "wideswrd S", "busterup *",
    "crakout *", "crakout *", "recov10 A", "recov10 A", "recov10 L", "recov10 L", "areagrab S",
    "attck-10 *", "attck-10 *",
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = "usage: double_attack_frames PACK OUT KEY PARTNER [PARTNER] [--key-a TICK...]";
    let split = args.iter().position(|a| a == "--key-a").unwrap_or(args.len());
    let (main, key_a) = args.split_at(split);
    let key_a: Vec<i64> = key_a.iter().skip(1).map(|t| t.parse().expect(usage)).collect();
    if main.len() < 4 || main.len() > 5 {
        panic!("{usage}");
    }
    let (pack, out, key) = (PathBuf::from(&main[0]), PathBuf::from(&main[1]), main[2].clone());
    let partners: Vec<&str> = main[3..].iter().map(String::as_str).collect();
    std::fs::create_dir_all(&out).unwrap();

    let options = Options { packs: vec![pack], mode: GraphicsMode::Original, sound: false, ..Options::default() };
    let loaded = load("exe5", &options).unwrap_or_else(|e| panic!("exe5: {e:?}"));
    let content = loaded.game.content.clone();
    let mut renderer = loaded.renderer();

    let folder: Vec<String> = FOLDER
        .iter()
        .map(|c| {
            let (name, code) = c.rsplit_once(' ').unwrap();
            format!("{{ chip = \"{name}\", code = \"{code}\" }}")
        })
        .collect();
    // (MegaMan's light/dark value 1000, as the DS lab's saves' light MegaMan has it.)
    let side = format!("navi = \"megaman\"\nfolder = [{}]\nkarma = 1000\n", folder.join(", "));
    let stage = nettai_match::ids::local(&content.defs.stage(nettai_match::link_battle_stages(&content, "exe5")[0]).key);
    let text = format!("game = \"exe5\"\n\n[[round]]\nstage = \"{stage}\"\n\n[left]\n{side}\n\n[right]\n{side}\n");
    let m = nettai_match::parse(&content, &text).unwrap_or_else(|p| panic!("{p:?}"));
    let mut b = Battle::new(m.round(&content, 0x5EED), content.clone());
    for partner in &partners {
        let navi = nettai_match::ids::navi(&content, "exe5", partner).unwrap();
        b.party[0].join(NaviStats::fresh(navi, &content).unwrap());
    }

    // Through the custom screens to the fight.
    let mut last = [0u16; 2];
    while !(b.round.mode == mode::FIGHTING && b.fight.state == fight::FIGHTING) {
        let input = [0, 1].map(|s| {
            last[s] = nettai_battle::scenario::custom_buttons(&b, s, last[s]);
            PlayerTick::buttons(last[s])
        });
        b.tick(&input, Default::default());
    }
    let switch = |b: &mut Battle, member: u8, mood: u8| {
        b.stats[0].mood = mood;
        let ask = PlayerTick { switch_to: Some(member), ..Default::default() };
        b.tick(&[ask, Default::default()], Default::default());
    };
    if partners.len() == 2 {
        switch(&mut b, 1, 0x80);
        for _ in 0..60 {
            b.tick(&Default::default(), Default::default());
        }
    }
    let director = content.defs.kind_by_key(&format!("{key}/director")).expect("the Double Attack's director");
    let telop = content.assets.handle(nettai_content_api::AssetKind::Sound, "telop").unwrap();
    let names = content.assets.names(nettai_content_api::AssetKind::Sound);

    // The switch that fires it, then each tick until its director has gone.
    switch(&mut b, partners.len() as u8, 0xFF);
    let mut ticks: Vec<(Battle, String)> = Vec::new();
    let mut telop_at = None;
    let mut seen = false;
    for t in 0..2000i64 {
        let held = match telop_at {
            Some(at) if key_a.contains(&(t - at)) => keys::A,
            _ => 0,
        };
        b.tick(&[PlayerTick::buttons(held), Default::default()], Default::default());
        let mut sounds = Vec::new();
        for c in b.sound_cues() {
            if let nettai_battle::sound::SoundCue::Effect(id) = c {
                if id.0 == telop && telop_at.is_none() {
                    telop_at = Some(t);
                }
                sounds.push(names.get(id.0 as usize).copied().unwrap_or("?").to_string());
            }
        }
        let here = b.objects.in_order().any(|r| b.objects.get(r).kind == director);
        seen |= here;
        // Each of its objects: its kind, panel, animation and palette.
        let mut objects = Vec::new();
        for r in b.objects.in_order() {
            let o = b.objects.get(r);
            let k = &content.defs.kind(o.kind).key;
            if k.starts_with(&format!("{key}/")) {
                let look = b.objects.sprite(r).look;
                objects.push(format!("{k}@{},{} anim {} pal {}", o.panel.x, o.panel.y, o.anim, look.palette));
            }
        }
        ticks.push((b.clone(), format!("{}\t{}", sounds.join(","), objects.join("; "))));
        if seen && !here {
            break;
        }
    }
    let at = telop_at.expect("no telop: the Double Attack didn't fire");
    let mut tsv = std::fs::File::create(out.join("ticks.tsv")).unwrap();
    for (i, (battle, sounds)) in ticks.iter().enumerate() {
        let n = i as i64 - at;
        let frame = renderer.render(battle);
        nettai_render::present::write_png(&out.join(format!("nettai_{n:+05}.png")), &frame, 1, None).unwrap();
        // The same without the Double Attack's own drawings (its kinds, the attachments and one-shot effects), for
        // a comparison of what it draws alone.
        let mut bare = battle.clone();
        let hidden: Vec<_> = bare
            .objects
            .in_order()
            .filter(|&r| {
                let k = &content.defs.kind(bare.objects.get(r).kind).key;
                k.starts_with(&format!("{key}/")) || k == "attachment" || k == "engine/effect"
            })
            .collect();
        for r in hidden {
            bare.objects.get_mut(r).set_visible(false);
        }
        let frame = renderer.render(&bare);
        nettai_render::present::write_png(&out.join(format!("bare_{n:+05}.png")), &frame, 1, None).unwrap();
        writeln!(tsv, "{n}\t{sounds}").unwrap();
    }
    println!("{} ticks drawn into {}, the telop at tick {at} after the switch", ticks.len(), out.display());
}
