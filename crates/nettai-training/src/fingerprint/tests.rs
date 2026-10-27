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

/// A job makes the same fingerprints on one thread as on four, from the
/// chips in another order; and a chip's fingerprint is its own alone, the
/// same whatever chips are fingerprinted with it; a status condition runs
/// for every status timer a status of the content's sets.
#[test]
fn a_job_makes_the_same_on_any_threads() {
    let (c, f) = few();
    let mut chips: Vec<ChipHandle> = f.chips.keys().copied().collect();
    chips.sort_by_key(|c| std::cmp::Reverse(c.0));
    let one = Fingerprints::compute(&c, &chips, 1).unwrap();
    assert_eq!(one.chips, f.chips);
    assert_eq!(one.pairs, f.pairs);
    let alone = Fingerprints::compute(&c, &chips[..2], 1).unwrap();
    for chip in &chips[..2] {
        assert_eq!(alone.of(*chip), f.of(*chip), "{}", c.defs.chip(*chip).key);
    }
    let conditions = Conditions::of(&c);
    for chip in &chips {
        let fp = f.of(*chip).unwrap();
        for (t, r) in fp.status_ratios.iter().enumerate() {
            assert_eq!(conditions.status(t).is_some(), r.is_some(), "{} status {t}", c.defs.chip(*chip).key);
        }
    }
    assert!(f.stops.is_empty(), "{:?}", f.stops);
    let job = Job::new(&c, &[]);
    assert!(matches!(job.result(), Some(Ok(f)) if f.chips.is_empty()), "no chips: done at once");
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

/// Every chip of the three games is fingerprinted with no run the engine
/// stops (a content error stops a browser's page): a chip that arrives
/// through a form with that form taken.
#[test]
fn every_chip_of_every_game_runs_without_a_stop() {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    for game in ["exe6", "exe5", "exe4"] {
        let c = content(game);
        let chips = game_chips(&c);
        let started = std::time::Instant::now();
        let f = Fingerprints::compute(&c, &chips, threads).unwrap_or_else(|e| panic!("{e}"));
        println!("{game}: {} chips, {} runs in {:.1} s on {threads} threads", chips.len(), f.cost.runs, started.elapsed().as_secs_f64());
        assert_eq!(f.chips.len(), chips.len(), "{game}");
        assert!(f.stops.is_empty(), "{game}: {} runs stopped: {:?}", f.stops.len(), &f.stops[..f.stops.len().min(3)]);
    }
}

/// A chip that arrives through a form is one no player brings into a
/// battle by itself: a folder that holds it is refused by its game's rules
/// (EXE4's souls' chips: the UNITE button puts them in the hand, the soul
/// given at OK), so the sandbox runs it with its form taken.
#[test]
fn a_chip_that_arrives_through_a_form_is_in_no_folder() {
    let mut found = 0;
    for game in ["exe6", "exe5", "exe4"] {
        let c = content(game);
        let maker = Maker::new(&c).unwrap();
        for (chip, form) in chip_forms(&c) {
            found += 1;
            assert!(!c.form(form).base, "{game}: {}", c.defs.chip(chip).key);
            let mut side = maker.default_build().unwrap();
            let mut folder = side.folder(&c);
            let code = c.chip(chip).codes[0];
            folder.chips[0] = Some(FolderChip::new(chip, code));
            side.set_folder(&c, &folder).unwrap();
            let other = maker.default_build().unwrap();
            assert!(maker.round([side, other], 1000, 1).is_err(), "{game}: a folder holds {}", c.defs.chip(chip).key);
        }
    }
    assert!(found >= 12, "EXE4's twelve souls' chips: {found}");
}

/// A navi chip counts as landed (`stats::Measure`) wherever it hits: its
/// sequence plays dimmed, past `stats::LANDS` ticks from its use, and the
/// window counts only undimmed ticks. Its fingerprint is printed beside it.
#[test]
fn a_navi_chip_counts_as_landed_when_it_hits() {
    let c = content("exe6");
    let chip = nettai_match::ids::chip(&c, "exe6", "heatman").expect("EXE6 has HeatMan");
    let sandbox = Sandbox::new(&c).unwrap_or_else(|e| panic!("{e}"));
    let (mut hits, mut late) = (0, 0);
    for target in 0..targets().len() {
        let Some((mut b, _)) = sandbox.prepared(target, Setup::Plain) else { continue };
        give(&mut b, 0, &[chip]);
        let mut m = crate::stats::Measure::new(Default::default());
        m.tick(&b, 0);
        let hp0 = hp(&b, 1);
        let (mut first_hit, mut dimmed) = (None, 0);
        for t in 1..1200 {
            let press = if t == 1 { nettai_battle::input::keys::A } else { 0 };
            if !step_battle(&mut b, [press, 0]) {
                break;
            }
            m.tick(&b, t);
            dimmed += b.is_dimmed() as u32;
            if first_hit.is_none() && hp(&b, 1) < hp0 {
                first_hit = Some(t);
                break;
            }
        }
        println!("target {target} {:?}: first hit at tick {first_hit:?}, {dimmed} ticks dimmed before", targets()[target]);
        assert_eq!(m.stats.attack_chips[0], 1, "target {target}: the chip's use wasn't counted");
        if let Some(t) = first_hit {
            hits += 1;
            late += (t > crate::stats::LANDS) as u32;
            assert_eq!(m.stats.attacks_landed[0], 1, "target {target}: hit at tick {t} ({dimmed} dimmed) not counted as landed");
        }
    }
    assert!(hits > 0, "HeatMan hit no target panel");
    assert!(late > 0, "no hit came later than LANDS ticks: the test doesn't cover the dimmed sequence");
    let fp = Fingerprints::compute(&c, &[chip], 1).unwrap_or_else(|e| panic!("{e}"));
    let f = fp.of(chip).expect("HeatMan's fingerprint");
    println!("HeatMan's fingerprint: runs {:?} best {:?}", f.runs.iter().map(|r| r.0).collect::<Vec<_>>(), f.best);
}

/// The real games' attack chips (a damage) whose fingerprint hit nothing,
/// and those whose first hit came in the last 30 of [`RUN_TICKS`]: a chip
/// whose sequence outlasts the run is described as hitting nothing. Needs
/// the packs (`NETTAI_PACKS`); `FP_CACHE` names a fingerprints folder.
#[test]
#[ignore]
fn attack_chips_that_hit_nothing_in_their_fingerprint() {
    let packs = nettai_content::pack::packs_dir();
    let cache = std::env::var_os("FP_CACHE").map(std::path::PathBuf::from);
    for game in ["exe6", "exe5", "exe4"] {
        let c = crate::pool::load_game(None, &packs, game).unwrap_or_else(|e| panic!("{e}"));
        let fp = Fingerprints::for_game(&c, 8, cache.as_deref()).unwrap_or_else(|e| panic!("{e}"));
        let (mut none, mut late) = (Vec::new(), Vec::new());
        for chip in game_chips(&c) {
            if c.chip(chip).damage == 0 {
                continue;
            }
            let Some(f) = fp.of(chip) else { continue };
            let key = c.defs.chip(chip).key.clone();
            match f.best {
                None => none.push(key),
                Some(_) if f.effect.first_hit.is_some_and(|t| t + 30 > RUN_TICKS) => late.push(format!("{key}@{}", f.effect.first_hit.unwrap())),
                Some(_) => {}
            }
        }
        println!("{game}: {} attack chips hit nothing: {}", none.len(), none.join(", "));
        println!("{game}: {} first hit in the last 30 ticks: {}", late.len(), late.join(", "));
    }
}
