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

/// Seats pinned to their own k and delay (from the next round) decide in
/// the fight each every its k ticks, from the view its delay before: a
/// network meets another trained at another k each at its own. Seat 0's
/// delay is past the drawn range's, so its views need more room.
#[test]
fn each_seat_decides_at_its_own_k() {
    let mut env = Env::new(games(&["exe6"]), Config { max_fight_ticks: 3000, ..Config::default() }, 7).unwrap();
    let (ks, delays) = ([4u32, 8], [40u16, 13]);
    assert!(delays[0] > env.cfg.delay.1);
    for s in 0..2 {
        env.set_k(s, Some(ks[s]));
        env.set_delay(s, Some(delays[s]));
    }
    let mut rng = SplitMix64::new(9);
    let mut actions = [0; 2];
    let (mut rounds, mut fight, mut off_eight) = (0, [0u32; 2], 0);
    while rounds < 3 {
        let out = env.step(actions).unwrap_or_else(|e| panic!("{e}"));
        for s in 0..2 {
            actions[s] = match out.phase[s] {
                FIGHT => {
                    if rounds >= 1 {
                        let seat = &env.seats[s];
                        assert_eq!((seat.k, seat.delay), (ks[s], delays[s]));
                        assert_eq!(env.t % ks[s], 0, "seat {s}");
                        off_eight += (env.t % 8 != 0) as u32;
                        if env.t > delays[s] as u32 {
                            let slot = &seat.ring[(env.t / seat.k) as usize % seat.ring.len()];
                            assert_eq!(slot.0, Some(env.t - delays[s] as u32), "seat {s}'s view at {}", env.t);
                        }
                        fight[s] += 1;
                    }
                    random_action(&mut rng, &env.legal_fight[s])
                }
                SCREEN => random_action(&mut rng, &env.legal_screen[s]),
                _ => 0,
            };
        }
        rounds += out.done as u32;
    }
    println!("fight decisions {fight:?}, seat 0's off a multiple of 8: {off_eight}");
    assert!(fight[1] > 100 && fight[0] > fight[1] * 3 / 2 && off_eight > 0);
}

/// Envs of one deal deal the same rounds from the next, however each round
/// went (random play, different seeds, so different screens and lengths);
/// another key deals others.
#[test]
fn envs_of_one_deal_deal_the_same_rounds() {
    let deals = |seed: u64, key: u64| {
        let mut env = Env::new(games(&GAMES), Config { max_fight_ticks: 900, ..Config::default() }, seed).unwrap();
        env.set_deal(Some(key));
        let mut rng = SplitMix64::new(seed);
        let mut actions = [0; 2];
        let mut rounds = Vec::new();
        while rounds.len() < 4 {
            let out = env.step(actions).unwrap_or_else(|e| panic!("{e}"));
            if out.done {
                rounds.push((env.game, env.folders.clone(), env.battle.digest()));
            }
            for s in 0..2 {
                actions[s] = match out.phase[s] {
                    FIGHT => random_action(&mut rng, &env.legal_fight[s]),
                    SCREEN => random_action(&mut rng, &env.legal_screen[s]),
                    _ => 0,
                };
            }
        }
        let ended: Vec<_> = env.ended.iter().map(|r| (r.game.clone(), r.navis.clone(), r.max_hp, r.ticks)).collect();
        (rounds, ended)
    };
    let (a, ea) = deals(1, 42);
    let (b, eb) = deals(2, 42);
    // (Set again after rounds of their own, envs deal from the first again.)
    let again = |seed: u64, extra: usize| {
        let mut env = Env::new(games(&GAMES), Config { max_fight_ticks: 900, ..Config::default() }, seed).unwrap();
        env.set_deal(Some(42));
        let (mut rng, mut actions, mut ended, mut first) = (SplitMix64::new(seed), [0; 2], 0, None);
        while first.is_none() {
            let out = env.step(actions).unwrap_or_else(|e| panic!("{e}"));
            if out.done {
                ended += 1;
                if ended == 1 + extra {
                    env.set_deal(Some(42));
                }
                if ended == 2 + extra {
                    first = Some(env.battle.digest());
                }
            }
            for s in 0..2 {
                actions[s] = match out.phase[s] {
                    FIGHT => random_action(&mut rng, &env.legal_fight[s]),
                    SCREEN => random_action(&mut rng, &env.legal_screen[s]),
                    _ => 0,
                };
            }
        }
        first.unwrap()
    };
    assert_eq!(again(1, 0), again(2, 2), "set again, from its first round");
    assert_eq!(again(1, 0), a[0].2, "its first round again");
    let (c, _) = deals(3, 43);
    // (The first round is each env's own; the deal starts with the second.)
    assert_eq!(a, b);
    assert_ne!(a, c);
    for i in 1..4 {
        assert_eq!((&ea[i].0, &ea[i].1, ea[i].2), (&eb[i].0, &eb[i].1, eb[i].2), "round {i}");
    }
    assert!((1..4).any(|i| ea[i].3 != eb[i].3), "the rounds were played alike");
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
            let legal = act::legal_screen(&b, 1, &Navigator::default(), false);
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
    let legal = act::legal_screen(&b, 1, &nav, false);
    assert!(!legal[slot as usize] && legal[OK_SLOT as usize]);
    navigate(&mut b, &mut nav, OK_SLOT as i32);
    assert!(!matches!(b.custom.sides[1].screen.unwrap().phase, Phase::Choosing));
}

/// A screen whose decisions are spent allows OK alone, the form list's
/// entries among what it no longer allows; none while a window of the
/// rules is up.
#[test]
fn past_its_decisions_a_screen_allows_ok_alone() {
    for game in GAMES {
        for seed in 1..6 {
            let b = at_the_custom_screen(game, seed);
            let mut nav = Navigator::default();
            probe(&b, 1, &mut nav);
            let past = act::legal_screen(&b, 1, &nav, true);
            assert_eq!((0..spec::SCREEN_ACTIONS).filter(|&a| past[a]).collect::<Vec<_>>(), [OK_SLOT as usize], "{game} seed {seed}");
        }
    }
    let mut windows = 0;
    for seed in 1..12 {
        let mut b = at_the_custom_screen("exe6", seed);
        let mut nav = Navigator::default();
        probe(&b, 1, &mut nav);
        let Some(o) = nav.opener else { continue };
        open_the_list(&mut b, o);
        assert!(act::legal_screen(&b, 1, &nav, false).iter().all(|&l| !l), "seed {seed}: an action in a window");
        windows += 1;
    }
    assert!(windows > 0, "no seed's screen had a form list");
}

/// Press opener `o`'s key on side 1's screen in `b`, the cursor where it
/// was found, till the form list is up.
fn open_the_list(b: &mut Battle, o: act::Opener) {
    assert_eq!(b.custom.sides[1].screen.unwrap().cursor, o.cursor);
    for held in [o.key, o.key, 0] {
        step_battle(b, [0, held]);
    }
    for _ in 0..30 {
        let Some(Phase::Window { window, .. }) = b.custom.sides[1].screen.map(|s| s.phase) else { panic!("the list didn't open") };
        if b.content.defs.window(window).view == Some(nettai_battle::content::WindowView::FormList) {
            return;
        }
        step_battle(b, [0, 0]);
    }
    panic!("the list never came up");
}

/// The navigator chooses every entry of EXE6's form list an action may
/// (its Crosses): from the screen as it opens, and with the cursor moved
/// away by a pick first. The entry is chosen (the screen's form set, the
/// list's choice its place), the screen choosing again, nothing refused;
/// then no entry is allowed, a Cross being chosen.
#[test]
fn the_navigator_chooses_every_entry_of_the_form_list() {
    let mut chosen = 0;
    for seed in 1..12 {
        let b = at_the_custom_screen("exe6", seed);
        let mut nav = Navigator::default();
        probe(&b, 1, &mut nav);
        let legal = act::legal_screen(&b, 1, &nav, false);
        let entries: Vec<usize> = (0..spec::FORM_ENTRIES).filter(|&k| legal[SLOTS + k]).collect();
        if entries.is_empty() {
            continue;
        }
        let list = b.form_list(1).expect("EXE6's form list");
        // (A chip on the bottom row picked first, if one may be, so the cursor walks back.)
        let far = (0..SLOTS).rev().find(|&s| legal[s] && s != OK_SLOT as usize && matches!(b.custom.sides[1].screen.unwrap().slots[s].kind, SlotKind::Chip { .. }));
        for first in [None, far].into_iter().flatten().map(Some).chain([None]) {
            for &k in &entries {
                let mut copy = b.clone();
                let mut nav = nav;
                if let Some(slot) = first {
                    navigate(&mut copy, &mut nav, slot as i32);
                }
                let ticks = navigate(&mut copy, &mut nav, (SLOTS + k) as i32);
                let after = copy.custom.sides[1].screen.unwrap();
                assert!(act::chooses(&after), "seed {seed} entry {k}: the screen isn't choosing ({ticks} ticks)");
                assert_eq!(nav.refused_forms, 0, "seed {seed} entry {k}: refused");
                assert!(after.form.is_some(), "seed {seed} entry {k}: no form");
                assert_eq!(copy.form_list(1).unwrap().chosen, Some(list.offered[k]), "seed {seed} entry {k}");
                let again = act::legal_screen(&copy, 1, &nav, false);
                assert!(again[SLOTS..].iter().all(|&l| !l), "seed {seed}: an entry allowed with a Cross chosen");
                assert!(again[OK_SLOT as usize]);
                chosen += 1;
            }
        }
    }
    assert!(chosen > 0, "no seed's screen offered a Cross");
}

/// An entry the list doesn't take is refused and masked, and the screen is
/// choosing again: here the opener's key moves the cursor rather than
/// opening the list.
#[test]
fn an_entry_the_list_does_not_take_is_refused() {
    for seed in 1..12 {
        let mut b = at_the_custom_screen("exe6", seed);
        let mut nav = Navigator::default();
        probe(&b, 1, &mut nav);
        let Some(o) = nav.opener else { continue };
        if !act::legal_screen(&b, 1, &nav, false)[SLOTS] {
            continue;
        }
        nav.opener = Some(act::Opener { key: keys::RIGHT, ..o });
        navigate(&mut b, &mut nav, SLOTS as i32);
        assert_eq!(nav.refused_forms, 1);
        assert!(act::chooses(&b.custom.sides[1].screen.unwrap()));
        assert!(!act::legal_screen(&b, 1, &nav, false)[SLOTS]);
        return;
    }
    panic!("no seed's screen offered a Cross");
}

/// Seats choosing at random on EXE6's screens (Crosses among their
/// choices), past their decisions OK alone, have every screen closed, and
/// the navigator backs out of the form list wherever a move on its way
/// opened it. With Beast Out's button shown and the bottom row's last
/// chips hidden, the shortest way from the top row to OK may begin with UP
/// (to the bottom row, along it to the button, up to OK), and UP on the top
/// row opens the form list while a Cross is on offer: that way opened the
/// list again after every B, till the screen was cut off, unless the
/// navigator goes around a move that opened a window.
#[test]
fn random_seats_have_their_screens_closed() {
    let cfg = Config { max_fight_ticks: 600, ..Config::default() };
    let (mut longest, mut crosses) = (0, 0);
    for seed in 1..40 {
        let mut env = Env::new(games(&["exe6"]), cfg.clone(), seed).unwrap();
        let mut rng = SplitMix64::new(seed * 7 + 1);
        let mut actions = [0; 2];
        let mut steps = 0;
        while env.ended.len() < 6 {
            let out = env.step(actions).unwrap_or_else(|e| panic!("{e}"));
            longest = longest.max(env.screen_ticks);
            for s in 0..2 {
                actions[s] = match out.phase[s] {
                    FIGHT => random_action(&mut rng, &env.legal_fight[s]),
                    SCREEN => random_action(&mut rng, &env.legal_screen[s]),
                    _ => 0,
                };
                crosses += (out.phase[s] == SCREEN && actions[s] >= SLOTS as i32) as u32;
            }
            steps += 1;
            assert!(steps < 30_000, "seed {seed}: 6 rounds didn't end");
        }
        assert!(env.ended.iter().all(|r| !r.truncated || r.ticks > cfg.max_fight_ticks), "seed {seed}: a round cut at the screen");
    }
    println!("longest screen {longest} ticks; Crosses chosen {crosses}");
    assert!(crosses > 0, "no seat chose a Cross");
    assert!(longest < cfg.max_screen_ticks / 4, "a screen stayed open {longest} ticks");
}

/// The key probe finds the window a game's rules put behind a key (EXE6's
/// form list on the screen's first decision, its opener), and nothing
/// where a key opens none; it reads its own screen of a copy, and the
/// battle is as it was.
#[test]
fn the_key_probe_finds_a_window_where_there_is_one() {
    for game in GAMES {
        let mut found = [false; act::KEYS.len()];
        let mut openers = 0;
        for seed in 1..12 {
            let mut b = at_the_custom_screen(game, seed);
            let digest = b.digest();
            let mut nav = Navigator::default();
            let p = probe(&b, 1, &mut nav);
            assert_eq!(b.digest(), digest);
            assert_eq!(p, probe_keys(&b, 1).map(|w| w.is_some()));
            for k in 0..p.len() {
                found[k] |= p[k];
            }
            // (The opener's key, pressed, opens the list.)
            if let Some(o) = nav.opener {
                open_the_list(&mut b, o);
                openers += 1;
            }
        }
        let has_windows = !content(game).defs.windows.is_empty();
        println!("{game}: keys that open a window {found:?} (windows: {has_windows}); screens with an opener {openers}");
        if game == "exe6" {
            assert!(found.iter().any(|&f| f) && openers > 0, "{game}: the form list");
        } else {
            assert_eq!(openers, 0, "{game} has no form list");
        }
    }
}

/// How a round ends (§6.4): two players that never attack go to the
/// judge's ruling after the final turn (a draw), and a round past the
/// fight's limit ends there with no result, not cut off.
#[test]
fn rounds_end_by_the_judge_or_at_the_fight_limit() {
    let seats = [Controller::Bot(BotKind::Still), Controller::Bot(BotKind::Still)];
    let mut env = Env::new(games(&["exe6"]), Config { practice: 1.0, seats, ..Config::default() }, 5).unwrap();
    let out = env.step([0, 0]).unwrap_or_else(|e| panic!("{e}"));
    let r = env.ended.last().unwrap();
    println!("judged: ticks {} fight {} result {:?} ending {}", r.ticks, r.fight_ticks, r.result, r.ending);
    assert!(out.done && !out.truncated, "{r:?}");
    assert_eq!((out.ending, out.result, r.ending), (BY_JUDGE, [0, 0], BY_JUDGE));
    let mut env = Env::new(games(&["exe6"]), Config { practice: 1.0, seats, max_fight_ticks: 300, ..Config::default() }, 5).unwrap();
    let out = env.step([0, 0]).unwrap_or_else(|e| panic!("{e}"));
    assert!(out.done && !out.truncated);
    assert_eq!((out.ending, out.result, env.ended.last().unwrap().ending), (BY_LIMIT, [0, 0], BY_LIMIT));
}

/// The fight action held is let go when a custom screen opens: the fight
/// resumes pressing nothing till its next decision, not the action from
/// before the screen (held on, an A in it used the new hand's first chip
/// at once, unknown to the decision after). Seats always choosing up with
/// A and B are found holding nothing on every screen.
#[test]
fn a_custom_screen_lets_the_fight_action_go() {
    let mut env = Env::new(games(&["exe6"]), Config { max_fight_ticks: 3000, ..Config::default() }, 4).unwrap();
    let mut rng = SplitMix64::new(2);
    let held = 4 + 2 + 1; // up, A, B
    let mut actions = [0; 2];
    let mut screens = 0;
    for _ in 0..3000 {
        let out = env.step(actions).unwrap_or_else(|e| panic!("{e}"));
        for s in 0..2 {
            actions[s] = match out.phase[s] {
                FIGHT => held,
                SCREEN => {
                    assert_eq!(env.seats[s].action, 0, "seat {s} holds its fight action on the screen");
                    screens += 1;
                    random_action(&mut rng, &env.legal_screen[s])
                }
                _ => 0,
            };
        }
    }
    assert!(screens > 20, "{screens} screen decisions");
}
