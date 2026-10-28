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
