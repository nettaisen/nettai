use super::*;
use crate::testing::content;
use nettai_battle::custom::screen::{OK_SLOT, SLOTS, SlotKind};
use nettai_netplay::rng::SplitMix64;

pub(crate) use crate::testing::{GAMES, games};

/// A random action of those allowed.
pub(crate) fn random_action(rng: &mut SplitMix64, legal: &[bool]) -> i32 {
    let allowed: Vec<usize> = (0..legal.len()).filter(|&i| legal[i]).collect();
    assert!(!allowed.is_empty(), "no action allowed");
    allowed[rng.below(allowed.len() as u64) as usize] as i32
}

/// A round of each game runs to its end with scripted players on both
/// seats: a step plays a whole round (no seat decides), and the next starts.
#[test]
fn a_round_runs_to_its_end_with_scripted_players() {
    let games = games(&GAMES);
    for g in 0..games.len() {
        let one = Arc::new(vec![Game::new(games[g].content.clone()).unwrap()]);
        let seats = [Controller::Bot(BotKind::Shooter), Controller::Bot(BotKind::Masher)];
        let mut env = Env::new(one, Config { practice: 0.5, seats, ..Config::default() }, 3 + g as u64).unwrap();
        let mut rounds = 0;
        let mut steps = 0;
        while rounds < 3 {
            let out = env.step([0, 0]).unwrap_or_else(|e| panic!("{e}"));
            steps += 1;
            if out.done {
                rounds += 1;
            }
            assert!(steps < 2000, "{}: no round ended", games[g].name);
        }
        for r in &env.ended {
            println!("{}: {:?} {:?} result {:?} ticks {} screens {} hp lost {:?} chips {:?} picked {:?} cut {} {:?}", r.game, r.controllers, r.navis, r.result, r.ticks, r.screens, r.hp_lost, r.chips_used, r.chips_picked, r.truncated, r.error);
            assert_eq!(r.controllers, ["shooter".to_string(), "masher".to_string()]);
            assert!(r.error.is_none(), "{}: {:?}", r.game, r.error);
            assert!(r.screens >= 1 && r.ticks > 0);
            assert!(r.result.iter().sum::<i8>() == 0);
            assert!(r.chips_picked.iter().sum::<u32>() > 0, "{}: no chip picked", r.game);
        }
    }
}

/// Policy seats in every game are asked in the fight (every k ticks) and
/// on the custom screen, with an observation and what they may do; random
/// allowed actions play rounds to their ends.
#[test]
fn policy_seats_decide_in_the_fight_and_on_the_screen() {
    for game in GAMES {
        let mut env = Env::new(games(&[game]), Config { max_fight_ticks: 3000, ..Config::default() }, 11).unwrap();
        let mut rng = SplitMix64::new(5);
        let mut actions = [0; 2];
        let (mut fight, mut screen, mut rounds, mut gains) = (0, 0, 0, 0.0f32);
        let mut last_tick = [0u32; 2];
        for _ in 0..4000 {
            let out = env.step(actions).unwrap_or_else(|e| panic!("{e}"));
            for s in 0..2 {
                match out.phase[s] {
                    FIGHT => {
                        fight += 1;
                        assert!(env.legal_fight[s].iter().all(|&l| l));
                        assert!(env.obs[s].sets[spec::GENERAL].mask[0]);
                        assert_eq!(env.t % env.cfg.k, 0);
                        assert!(env.t > last_tick[s] || out.done);
                        last_tick[s] = env.t;
                        actions[s] = random_action(&mut rng, &env.legal_fight[s]);
                    }
                    SCREEN => {
                        screen += 1;
                        assert!(env.legal_screen[s].iter().any(|&l| l));
                        // (Its screen's slots are tokens.)
                        assert!(env.obs[s].sets[spec::SLOTS].mask[OK_SLOT as usize]);
                        actions[s] = random_action(&mut rng, &env.legal_screen[s]);
                    }
                    _ => actions[s] = 0,
                }
                gains += out.hp_gain[s];
            }
            if out.done {
                rounds += 1;
                last_tick = [0; 2];
            }
        }
        println!("{game}: {fight} fight decisions, {screen} screen decisions, {rounds} rounds, gains {gains}");
        for r in &env.ended {
            println!("  {:?} result {:?} ticks {} fight {} screens {} picked {:?} forms {:?} cut {} {:?}", r.navis, r.result, r.ticks, r.fight_ticks, r.screens, r.chips_picked, r.forms_taken, r.truncated, r.error);
            assert!(r.error.is_none(), "{game}: {:?}", r.error);
        }
        assert!(fight > 100 && screen > 10 && rounds >= 1, "{game}");
    }
}

/// Side 1's first custom screen of a round of random builds of `game`,
/// choosing.
pub(crate) fn at_the_custom_screen(game: &str, seed: u32) -> Battle {
    let c = content(game);
    let maker = Maker::new(&c).unwrap();
    let mut picks = Picks::new(seed);
    let none = crate::builds::Holdout::default();
    let sides = [maker.random(&mut picks, &none, 0.75, crate::builds::Combos::NONE).unwrap().side, maker.random(&mut picks, &none, 0.75, crate::builds::Combos::NONE).unwrap().side];
    let m = maker.round_with(sides, None, seed).unwrap();
    let mut b = Set::of(&c, &m, seed).start();
    for _ in 0..900 {
        if b.round.mode == mode::CUSTOM && b.custom.sides[1].screen.is_some_and(|s| s.phase == Phase::Choosing) {
            return b;
        }
        step_battle(&mut b, [0, 0]);
    }
    panic!("{game}: no custom screen");
}

/// Point side 1's navigator at `action` in `b` and press until it is idle
/// again; the ticks it took.
fn navigate(b: &mut Battle, nav: &mut Navigator, action: i32) -> u32 {
    nav.point(action);
    for t in 0..200 {
        let held = nav.buttons(b, 1);
        step_battle(b, [0, held]);
        if nav.idle() {
            return t + 1;
        }
    }
    panic!("the navigator never finished");
}

/// The navigator reaches every slot a pointer may be at and presses A
/// there: a chip is picked, OK closes the screen, a button does what it
/// does; none is refused.
#[test]
fn the_navigator_reaches_every_slot_it_may_point_at() {
    for game in GAMES {
        for seed in 1..4 {
            let b = at_the_custom_screen(game, seed);
            let screen = b.custom.sides[1].screen.unwrap();
            let legal = act::legal_screen(&screen, &b.game_rules().custom_screen, 0, false);
            assert!(legal[OK_SLOT as usize], "{game}: OK");
            for slot in (0..SLOTS).filter(|&s| legal[s]) {
                let mut copy = b.clone();
                let mut nav = Navigator::default();
                let ticks = navigate(&mut copy, &mut nav, slot as i32);
                let after = copy.custom.sides[1].screen.unwrap();
                assert_eq!(nav.refused, 0, "{game} seed {seed}: slot {slot} refused");
                match screen.slots[slot].kind {
                    SlotKind::Chip { .. } | SlotKind::Offered(_) => {
                        assert!(after.selection().contains(&(slot as u8)), "{game} seed {seed}: slot {slot} not picked ({ticks} ticks)")
                    }
                    SlotKind::Ok => assert!(!matches!(after.phase, Phase::Choosing), "{game}: OK didn't close the screen"),
                    _ => assert_ne!(after, screen, "{game}: the button at {slot} did nothing"),
                }
            }
        }
    }
}

/// A slot whose A the screen refuses (EXE4's chip in a code it doesn't
/// come in) is refused after one press and masked; the screen goes on to
/// OK.
#[test]
fn a_refused_pick_is_masked() {
    let mut b = at_the_custom_screen("exe4", 1);
    let content = b.content.clone();
    let screen = b.custom.sides[1].screen.unwrap();
    let (slot, chip) = screen.dealt(&b.custom.sides[1].folder).next().expect("a chip dealt");
    let code = ('A'..='Z').filter_map(nettai_battle::content::ChipCode::from_letter).find(|c| !content.chip(chip.id).codes.contains(c)).unwrap();
    let SlotKind::Chip { index, .. } = screen.slots[slot as usize].kind else { panic!("a chip's slot") };
    b.custom.sides[1].folder.chips[index as usize] = Some(FolderChip::new(chip.id, code));
    let mut nav = Navigator::default();
    navigate(&mut b, &mut nav, slot as i32);
    assert_eq!(nav.refused, 1 << slot, "the refused pick");
    let screen = b.custom.sides[1].screen.unwrap();
    let legal = act::legal_screen(&screen, &b.game_rules().custom_screen, nav.refused, false);
    assert!(!legal[slot as usize] && legal[OK_SLOT as usize]);
    navigate(&mut b, &mut nav, OK_SLOT as i32);
    assert!(!matches!(b.custom.sides[1].screen.unwrap().phase, Phase::Choosing));
}

/// The key probe finds the window a game's rules put behind a key (EXE6's
/// form window on the screen's first decision), and nothing where a key
/// opens none; it reads its own screen of a copy, and the battle is as it
/// was.
#[test]
fn the_key_probe_finds_a_window_where_there_is_one() {
    for game in GAMES {
        let mut found = [false; act::KEYS.len()];
        for seed in 1..12 {
            let b = at_the_custom_screen(game, seed);
            let digest = b.digest();
            let p = probe_keys(&b, 1);
            assert_eq!(b.digest(), digest);
            for k in 0..p.len() {
                found[k] |= p[k];
            }
            // (A key that opens one, pressed, does.)
            if let Some(k) = p.iter().position(|&o| o) {
                let mut copy = b.clone();
                let mut nav = Navigator::default();
                navigate(&mut copy, &mut nav, (SLOTS + k) as i32);
                assert!(matches!(copy.custom.sides[1].screen.unwrap().phase, Phase::Window { .. }), "{game}: key {k}");
            }
        }
        let has_windows = !content(game).defs.windows.is_empty();
        println!("{game}: keys that open a window {found:?} (windows: {has_windows})");
        if game == "exe6" {
            assert!(found.iter().any(|&f| f), "{game}: the form window");
        }
    }
}
