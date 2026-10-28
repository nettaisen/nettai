use super::*;
use crate::fingerprint::{Sandbox, Setup};
use crate::observe::{self, Extra, Names, Obs};
use crate::spec::{self, panels};
use crate::testing::content;

/// The sandbox's fight (EXE6), the user (side 0) having pressed A with a
/// cannon in hand, the target (side 1) two columns ahead in its row.
fn cannon_fired() -> Battle {
    let c = content("exe6");
    let cannon = nettai_match::ids::chip(&c, "exe6", "cannon").expect("EXE6 has a cannon");
    let sandbox = Sandbox::new(&c).unwrap_or_else(|e| panic!("{e}"));
    let (mut b, _) = sandbox.prepared(4, Setup::Plain).expect("the target at its panel");
    crate::fingerprint::give(&mut b, 0, &[cannon]);
    assert!(step_battle(&mut b, [nettai_battle::input::keys::A, 0]));
    b
}

/// Panel bit (`y * 8 + x`) of side `side`'s navi.
fn navi_panel(b: &Battle, side: u8) -> usize {
    let o = b.objects.get(b.player(side).expect("a navi"));
    o.panel.y as usize * 8 + o.panel.x as usize
}

/// A cannon fired at the target: the map has side 0's attack land on the
/// target's panel, with its damage, and nothing of side 0's off its row;
/// side 1, pressing nothing, attacks nowhere. The battle itself is left as
/// it was, and the map doesn't depend on the battle's luck.
#[test]
fn a_cannon_under_way_lands_on_its_targets_panel() {
    let b = cannon_fired();
    let target = navi_panel(&b, 1);
    let d = foresee(&b, 7);
    let hit = d.at(0, target);
    assert!(hit.first >= 1 && hit.first as u32 <= HORIZON, "{hit:?}");
    assert!(hit.damage > 0, "{hit:?}");
    for p in 0..32 {
        if d.at(0, p).first > 0 {
            assert_eq!(p / 8, target / 8, "side 0's attack lands on panel {p}, off the target's row");
        }
        assert_eq!(d.at(1, p), Landing::default(), "side 1 attacks panel {p}");
    }
    let mut lucky = b.clone();
    lucky.rng = nettai_battle::Rng::new(lucky.rng.state ^ 0x5555_5555);
    lucky.consoles[0].rng = nettai_battle::Rng::new(12345);
    assert_eq!(foresee(&lucky, 7), d, "the battle's luck isn't foreseen");
    assert_eq!(b.digest(), cannon_fired().digest(), "the battle is left as it was");
}

/// No fight running (the round's first custom screen): no map.
#[test]
fn no_map_on_the_custom_screen() {
    let c = content("exe6");
    let maker = crate::builds::Maker::new(&c).unwrap();
    let mut picks = nettai_match::Picks::new(3);
    let none = crate::builds::Holdout::default();
    let sides = [0, 1].map(|_| maker.random(&mut picks, &none, 0.75, crate::builds::Combos::NONE).unwrap().side);
    let m = maker.round(sides, 1000, 3).unwrap();
    let b = nettai_match::Set::of(&c, &m, 3).start();
    assert!(!fight_runs(&b));
    assert_eq!(foresee(&b, 1), Danger::default());
}

/// The panel tokens carry it from the viewer's side: the target (side 1,
/// its view mirrored) sees the other side's attack landing on its own
/// panel, soon and with the damage, and the user sees its own there.
#[test]
fn panel_tokens_carry_the_map_from_the_viewers_side() {
    let b = cannon_fired();
    let target = navi_panel(&b, 1);
    let d = foresee(&b, 7);
    let names = Names::of(&b.content);
    let token = |viewer: u8| {
        let (x, y) = ((target % 8) as u8, (target / 8) as u8);
        let mx = if viewer == 1 { 7 - x } else { x };
        (y as usize - 1) * 6 + (mx as usize - 1)
    };
    let row = |obs: &Obs, t: usize| obs.sets[spec::PANELS].x[t * panels::WIDTH..(t + 1) * panels::WIDTH].to_vec();
    let mut obs = Obs::new();
    observe::encode(&b, 1, &names, &Extra { danger: d, ..Extra::default() }, &mut obs);
    let r = row(&obs, token(1));
    let hit = d.at(0, target);
    assert!(r[panels::danger] > 0.0 && r[panels::danger] <= 1.0, "the other side's, soon: {}", r[panels::danger]);
    assert_eq!(r[panels::danger + 1], hit.damage as f32 / 1000.0);
    assert_eq!((r[panels::danger + 2], r[panels::danger + 3]), (0.0, 0.0), "none of its own");
    observe::encode(&b, 0, &names, &Extra { danger: d, ..Extra::default() }, &mut obs);
    let r = row(&obs, token(0));
    assert_eq!((r[panels::danger], r[panels::danger + 1]), (0.0, 0.0));
    assert!(r[panels::danger + 2] > 0.0, "its own attack's");
    observe::encode(&b, 1, &names, &Extra::default(), &mut obs);
    assert!(row(&obs, token(1))[panels::danger..panels::danger + 4].iter().all(|&v| v == 0.0), "no map given, none");
}

/// What a map costs, by game: made every 8 ticks of fights the scripted
/// players play on the repository's content (printed: the mean and the
/// most a map). Not run unless asked (`--ignored`).
#[test]
#[ignore]
fn map_costs_by_game() {
    use crate::bots::BotKind;
    use crate::env::{Config, Controller, Env, Game};
    for game in ["exe6", "exe5", "exe4"] {
        let c = crate::pool::load_game(None, game).unwrap_or_else(|e| panic!("{e}"));
        let games = std::sync::Arc::new(vec![Game::new(c).unwrap()]);
        let seats = [Controller::Bot(BotKind::Shooter), Controller::Bot(BotKind::Masher)];
        let mut env = Env::new(games, Config { seats, max_fight_ticks: 3600, ..Config::default() }, 5).unwrap();
        let (mut n, mut total, mut most) = (0u32, std::time::Duration::ZERO, std::time::Duration::ZERO);
        for t in 0..30_000u32 {
            if t % 8 == 0 && fight_runs(env.battle()) {
                let at = std::time::Instant::now();
                std::hint::black_box(foresee(env.battle(), t as u64));
                let took = at.elapsed();
                total += took;
                most = most.max(took);
                n += 1;
            }
            env.run(1);
        }
        println!("{game}: {n} maps, {:?} a map, the most {:?}", total / n.max(1), most);
    }
}
