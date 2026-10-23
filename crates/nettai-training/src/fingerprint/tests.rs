use super::*;
use crate::testing::content;

/// Fingerprints of a few chips of each game: a damaging chip hits some
/// target panel (its damage the record's or more), a chip that hits none has
/// zeros; the conditions run; the cost is printed.
#[test]
fn fingerprints_of_a_few_chips() {
    for game in ["exe6", "exe5", "exe4"] {
        let c = content(game);
        let pick = |key: &str| nettai_match::ids::chip(&c, game, key);
        let keys = ["cannon", "minibomb", "sword", "recov10", "recover10", "areagrab", "grabrvng", "grabrevenge", "heatshot", "firearm", "panlgrab", "panelgrab", "aquaswrd", "icestage", "grasstge", "grassstage", "elecsword", "zapring1", "bubbler"];
        let chips: Vec<ChipHandle> = keys.iter().filter_map(|k| pick(k)).collect();
        assert!(chips.len() >= 3, "{game}: chips {chips:?}");
        let fp = Fingerprints::compute(&c, &chips, 4).unwrap_or_else(|e| panic!("{e}"));
        let k = fp.cost;
        println!(
            "{game}: {} chips, {} runs, {} ticks, {:.1} ms a chip of work, {:.1} s wall on 4 threads (test build)",
            k.chips,
            k.runs,
            k.ticks,
            k.nanos as f64 / k.chips as f64 / 1e6,
            k.wall_nanos as f64 / 1e9
        );
        for chip in &chips {
            let Some(f) = fp.of(*chip) else {
                println!("  {}: none", c.defs.chip(*chip).key);
                continue;
            };
            println!(
                "  {}: runs {:?} best {:?} statuses {:#x} under {:#x} takes front {} kinds {:?} statuses {:?} stolen {:?} advanced {:?} left {} changed {:#x} owners {:#x}",
                c.defs.chip(*chip).key,
                f.runs.iter().map(|r| r.0).collect::<Vec<_>>(),
                f.best,
                f.statuses,
                f.turns_under_target,
                f.takes_front,
                f.kind_ratios.iter().filter(|(_, r)| (*r - 1.0).abs() > 0.01).collect::<Vec<_>>(),
                f.status_ratios,
                f.stolen,
                f.advanced,
                f.effect.left,
                f.effect.changed,
                f.effect.owner_changed,
            );
        }
        println!("  pairs: {:?}", fp.pairs.iter().map(|p| (c.defs.chip(p.setup).key.clone(), c.defs.chip(p.payoff).key.clone(), p.condition, p.ratio)).collect::<Vec<_>>());
        let cannon = pick("cannon").and_then(|h| fp.of(h));
        if let Some(f) = cannon {
            assert!(f.best.is_some(), "{game}: the cannon hit nothing");
        }
    }
}


/// A few EXE6 chips' fingerprints, made once for the tests.
fn few() -> (Arc<nettai_battle::Content>, Arc<Fingerprints>) {
    static FEW: std::sync::OnceLock<Arc<Fingerprints>> = std::sync::OnceLock::new();
    let c = content("exe6");
    let f = FEW.get_or_init(|| {
        let keys = ["cannon", "sword", "widesword", "areagrab", "panlgrab", "grabrvng", "heatshot", "recov10", "minibomb"];
        let chips: Vec<ChipHandle> = keys.iter().filter_map(|k| nettai_match::ids::chip(&c, "exe6", k)).collect();
        Arc::new(Fingerprints::compute(&c, &chips, 4).unwrap())
    });
    (c, f.clone())
}

/// Folders seeded with pairs: each round's seats' folders hold both of
/// their pair's chips, and the round's stats name the pair; a chip token
/// of the view carries its fingerprint.
#[test]
fn pairs_seed_folders_and_are_watched() {
    use crate::env::{Config, Controller, Env, Game};
    let (c, f) = few();
    assert!(!f.pairs.is_empty(), "no pair among {:?}", f.chips.keys());
    let games = Arc::new(vec![Game::with_fingerprints(c.clone(), f.clone()).unwrap()]);
    let seats = [Controller::Bot(crate::bots::BotKind::Shooter), Controller::Bot(crate::bots::BotKind::Masher)];
    let mut env = Env::new(games, Config { combos: 1.0, seats, max_fight_ticks: 2400, ..Config::default() }, 4).unwrap();
    let mut seen = 0;
    for _ in 0..4 {
        loop {
            if env.step([0, 0]).unwrap().done {
                break;
            }
        }
    }
    for r in &env.ended {
        for s in 0..2 {
            if let Some((setup, payoff)) = &r.pair[s] {
                seen += 1;
                println!("seat {s}: pair {setup} -> {payoff}: picked {} ordered {} used {} payoff {:?}", r.pair_picked[s], r.pair_ordered[s], r.pair_used[s], r.payoff_damage[s]);
            }
        }
    }
    assert!(seen > 0, "no folder was seeded");
    // (A view's chip tokens carry the fingerprints: some chip token of
    // some view is non-zero in the block.)
    let mut obs = crate::observe::Obs::new();
    let mut env = Env::new(Arc::new(vec![Game::with_fingerprints(c.clone(), f.clone()).unwrap()]), Config { combos: 1.0, seats, ..Config::default() }, 5).unwrap();
    let mut found = false;
    for _ in 0..30 {
        env.run(60);
        crate::observe::encode(env.battle(), 0, env.names(), &env.extra_now(0), &mut obs);
        for set in [crate::spec::CHIPS, crate::spec::SLOTS] {
            let w = crate::spec::SETS[set].width;
            let buf = &obs.sets[set];
            for t in 0..crate::spec::SETS[set].count {
                found |= buf.x[t * w + crate::spec::chips::fingerprint..(t + 1) * w].iter().any(|&v| v != 0.0);
            }
        }
    }
    assert!(found, "no chip token carried a fingerprint");
}

/// The fingerprints read back from their file as they were written.
#[test]
fn fingerprints_keep_on_disk() {
    let (_, f) = few();
    let bytes = serde_json::to_vec(&*f).unwrap();
    let back: Fingerprints = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(back.chips, f.chips);
    assert_eq!(back.pairs, f.pairs);
    assert_eq!((back.version, back.hash), (f.version, f.hash));
}
