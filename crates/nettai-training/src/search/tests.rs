use super::*;
use crate::fingerprint::{Sandbox, Setup};
use crate::testing::content;

/// The candidates: the network's likeliest first, then each direction with
/// the first's A and B, none twice.
#[test]
fn candidates_are_the_likeliest_then_the_steps_beside_the_first() {
    let mut logits = vec![0.0f32; spec::FIGHT_ACTIONS];
    // (Action 1: no direction, B held; 6: up, A held.)
    logits[1] = 3.0;
    logits[6] = 2.0;
    let c = candidates(&logits, 2);
    assert_eq!(&c[..2], &[1, 6]);
    assert_eq!(&c[2..], &[5, 9, 13, 17], "each direction with B held, after the two");
}

/// Standing in a cannon's row as it fires, the lookahead steps out of it,
/// though the network would stay (5 nats likelier): staying costs the
/// cannon's 40 HP (of 1000), a step up or down nothing.
#[test]
fn it_steps_out_of_a_cannons_row() {
    let c = content("exe6");
    let cannon = nettai_match::ids::chip(&c, "exe6", "cannon").expect("EXE6 has a cannon");
    let sandbox = Sandbox::new(&c).unwrap_or_else(|e| panic!("{e}"));
    // (Target 4: two columns ahead of the user, its row.)
    let (mut b, _) = sandbox.prepared(4, Setup::Plain).expect("the target at its panel");
    crate::fingerprint::give(&mut b, 0, &[cannon]);
    // The user presses A: the cannon starts.
    assert!(step_battle(&mut b, [nettai_battle::input::keys::A, 0]));
    let search = Search::default();
    let mut logits = vec![0.0f32; spec::FIGHT_ACTIONS];
    logits[0] = 5.0; // (the network: stand still)
    let stay = search.outcome(&b, 1, 8, 0);
    assert!(stay < 0.0, "staying in the row costs HP: {stay}");
    let a = search.pick(&b, 1, 8, &logits).expect("an action");
    assert!(matches!(a / 4, 1 | 2), "a step up or down, not {a} (staying worth {stay})");
}

/// Live play's lookahead foresees the battle `lead` ticks on (its own
/// held action pressed through them, the other side's nothing, the copy's
/// luck its own) and decides from there: it still steps out of the
/// cannon's row.
#[test]
fn ahead_it_still_steps_out_of_a_cannons_row() {
    let c = content("exe6");
    let cannon = nettai_match::ids::chip(&c, "exe6", "cannon").expect("EXE6 has a cannon");
    let sandbox = Sandbox::new(&c).unwrap_or_else(|e| panic!("{e}"));
    let (mut b, _) = sandbox.prepared(4, Setup::Plain).expect("the target at its panel");
    crate::fingerprint::give(&mut b, 0, &[cannon]);
    assert!(step_battle(&mut b, [nettai_battle::input::keys::A, 0]));
    let search = Search::default();
    assert!(search.lead >= 1);
    let mut logits = vec![0.0f32; spec::FIGHT_ACTIONS];
    logits[0] = 5.0;
    let a = search.ahead(&b, 1, 8, 0, &logits, 7).expect("an action");
    assert!(matches!(a / 4, 1 | 2), "a step up or down, not {a}");
}

/// What a try costs, by game: a copy of a battle in its fight, and a tick
/// of one (the repository's content, real timing; printed). COST_GAMES
/// (comma-separated) and COST_TICKS (2000) change what is timed.
#[test]
#[ignore]
fn lookahead_costs_by_game() {
    let games = std::env::var("COST_GAMES").unwrap_or_else(|_| "exe6,exe5,exe4".into());
    let ticks: u32 = std::env::var("COST_TICKS").ok().and_then(|v| v.parse().ok()).unwrap_or(2000);
    for game in games.split(',') {
        let c = crate::pool::load_game(None, game).unwrap_or_else(|e| panic!("{e}"));
        let maker = crate::builds::Maker::new(&c).unwrap_or_else(|e| panic!("{e}"));
        let mut picks = nettai_match::Picks::new(7);
        let build = |p: &mut nettai_match::Picks| maker.random_with(p, &crate::builds::Holdout::default(), 0.75, 0.5, crate::builds::Combos::NONE).expect("a build");
        let (b0, b1) = (build(&mut picks), build(&mut picks));
        let m = maker.round_with([b0.side, b1.side], Some(1000), 7).expect("a round");
        let mut b = nettai_match::Set::of(&c, &m, 7).start();
        // (Into the fight: the first custom screens chosen as the scripted players choose.)
        let mut navs = [act::Navigator::default(), act::Navigator::default()];
        for t in 0..6000 {
            let mut press = [0u16; 2];
            if b.round.mode == nettai_battle::battle::mode::CUSTOM {
                for s in 0..2u8 {
                    let nav = &mut navs[s as usize];
                    if nav.idle() {
                        let p = act::scripted_pointer(&b, s, nav);
                        nav.point(p);
                    }
                    press[s as usize] = nav.buttons(&b, s);
                }
            } else {
                navs = [act::Navigator::default(), act::Navigator::default()];
            }
            step_battle(&mut b, press);
            if crate::env::fight_runs(&b) && t > 300 {
                break;
            }
        }
        let at = std::time::Instant::now();
        for _ in 0..200 {
            std::hint::black_box(b.clone());
        }
        let copy = at.elapsed() / 200;
        let mut c2 = b.clone();
        let at = std::time::Instant::now();
        let mut n = 0;
        for t in 0..ticks {
            if t % 2000 == 0 {
                c2 = b.clone();
            }
            if !step_battle(&mut c2, [if t % 9 == 0 { nettai_battle::input::keys::A } else { 0 }, 0]) {
                c2 = b.clone();
            }
            n += 1;
        }
        let tick = at.elapsed() / n.max(1);
        println!("{game}: a copy {copy:?}, a tick {tick:?} (fight runs {}, mode {})", crate::env::fight_runs(&b), b.round.mode);
    }
}
