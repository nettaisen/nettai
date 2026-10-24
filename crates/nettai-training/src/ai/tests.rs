use super::*;
use crate::testing::content;
use nettai_battle::hud::CustomGauge;
use nettai_battle::input::keys;
use nettai_match::{Picks, Set};

/// The parity fixture's network (small, its parameters random): it plays,
/// if not well.
fn net() -> Arc<Net> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/net.safetensors");
    Arc::new(Net::from_safetensors(&std::fs::read(path).unwrap()).unwrap())
}

/// A few chips' fingerprints of a game's test content, made once a game.
fn fingerprints(game: &str) -> Fingerprinter {
    static MADE: OnceLock<Mutex<HashMap<String, Arc<Fingerprints>>>> = OnceLock::new();
    let mut made = MADE.get_or_init(Default::default).lock().unwrap();
    let f = made.entry(game.to_string()).or_insert_with(|| {
        let c = content(game);
        let chips: Vec<_> = ["cannon", "sword", "areagrab", "minibomb", "recov10", "recover10"].iter().filter_map(|k| nettai_match::ids::chip(&c, game, k)).collect();
        Arc::new(Fingerprints::compute(&c, &chips, 2).unwrap())
    });
    Fingerprinter::made(f.clone())
}

/// A round of random builds of `game`: the player (side 0) picking the
/// first chip it can and opening the custom screen once the gauge is full,
/// the AI side 1, its fight buttons without L and R as live play strips
/// them (`theirs`: what it is told the player holds). The AI's buttons, a
/// tick each, and the battle after.
fn play(game: &str, seed: u32, ai: &mut NetAi, ticks: u32, theirs: u16) -> (Vec<u16>, Battle) {
    let (pressed, _, b) = play_fight(game, seed, ai, ticks, theirs);
    (pressed, b)
}

/// The same, and whether the fight ran on each tick.
fn play_fight(game: &str, seed: u32, ai: &mut NetAi, ticks: u32, theirs: u16) -> (Vec<u16>, Vec<bool>, Battle) {
    let c = content(game);
    let maker = crate::builds::Maker::new(&c).unwrap();
    let mut picks = Picks::new(seed);
    let none = crate::builds::Holdout::default();
    let sides = [0, 1].map(|_| maker.random(&mut picks, &none, 0.75, crate::builds::Combos::NONE).unwrap().side);
    let m = maker.round(sides, 1000, seed).unwrap();
    let mut b = Set::of(&c, &m, seed).start();
    let (mut pressed, mut runs) = (Vec::new(), Vec::new());
    for t in 0..ticks {
        let custom = b.round.mode == mode::CUSTOM;
        let mine = if custom {
            nettai_frontend::driver::bot_buttons(&b, 0, t)
        } else if fight_runs(&b) && !b.is_dimmed() && b.gauge.value >= CustomGauge::FULL && t % 2 == 0 {
            keys::L
        } else {
            0
        };
        let held = if custom { ai.custom_screen(&b, 1) } else { ai.fight(&b, 1, theirs) & !(keys::L | keys::R) };
        pressed.push(held);
        runs.push(fight_runs(&b));
        if !env::step_battle(&mut b, [mine, held]) || b.is_battle_over() {
            break;
        }
    }
    (pressed, runs, b)
}

/// It plays a round of each game: through custom screens (its own pressed
/// to OK) and in the fight, where it decides every k ticks from the view
/// of its delay before (once the round is that old), holding its buttons
/// between, and never presses L, R, START or SELECT.
#[test]
fn it_plays_a_round_as_the_env_plays_a_seat() {
    for game in crate::testing::GAMES {
        let mut ai = NetAi::of(net(), Level::Normal, 5, fingerprints(game)).unwrap();
        let (pressed, runs, b) = play_fight(game, 9, &mut ai, 2400, 0);
        assert_eq!(ai.error(), None, "{game}");
        assert!(b.round.turn >= 2, "{game}: it got through {} custom screens", b.round.turn);
        let d = ai.knobs.delay as u32;
        assert!(ai.decided.len() > 50, "{game}: {} fight decisions", ai.decided.len());
        for &(t, seen) in &ai.decided {
            assert!(t.is_multiple_of(ai.k), "{game}: a decision at tick {t}");
            assert_eq!(seen, t - d, "{game}: the decision at {t} saw the view of {seen}");
        }
        let fight: Vec<(usize, u16)> = pressed.iter().copied().enumerate().filter(|&(t, _)| ai.decided.iter().any(|&(d, _)| d as usize <= t)).collect();
        assert!(fight.iter().any(|&(_, h)| h & (keys::A | keys::B) != 0) && fight.iter().any(|&(_, h)| h & (keys::UP | keys::DOWN | keys::LEFT | keys::RIGHT) != 0), "{game}: it moves and attacks");
        assert!(pressed.iter().all(|h| h & (keys::L | keys::R | keys::SELECT) == 0), "{game}");
        // (Between decisions in the fight, the buttons held stay as they were.)
        let decisions: std::collections::HashSet<usize> = ai.decided.iter().map(|&(t, _)| t as usize).collect();
        for t in 1..pressed.len() {
            if runs[t] && runs[t - 1] && !decisions.contains(&t) {
                assert_eq!(pressed[t], pressed[t - 1], "{game}: tick {t}, no decision");
            }
        }
    }
}

/// While the fingerprints of the chips its view shows aren't made it
/// decides nothing (its custom screen waits, asking for them); and what it
/// is told the player holds changes nothing it does; and as the set starts
/// over it plays as it did.
#[test]
fn it_waits_for_its_fingerprints_hears_nothing_and_starts_over() {
    let game = "exe6";
    let c = content(game);
    let idle = Fingerprinter::idle(&c);
    let mut waiting = NetAi::of(net(), Level::Easy, 5, idle.clone()).unwrap();
    let (pressed, b) = play(game, 4, &mut waiting, 900, 0);
    assert!(pressed.iter().all(|&h| h == 0) && waiting.decided.is_empty() && !waiting.ready());
    assert_eq!(b.round.mode, mode::CUSTOM, "the custom screen waits for it");
    assert!(matches!(idle.status(), Status::Making { done: 0, of } if of > 10), "both folders' chips asked for: {:?}", idle.status());
    let mut ai = NetAi::of(net(), Level::Easy, 5, fingerprints(game)).unwrap();
    let (one, _) = play(game, 4, &mut ai, 1500, 0);
    let mut deaf = NetAi::of(net(), Level::Easy, 5, fingerprints(game)).unwrap();
    let (two, _) = play(game, 4, &mut deaf, 1500, 0xFFFF);
    assert_eq!(one, two, "the player's buttons are never heard");
    ai.reset();
    let (again, _) = play(game, 4, &mut ai, 1500, 0);
    assert_eq!(one, again, "started over");
    assert!(one.iter().any(|&h| h != 0));
}

/// Fingerprints of another content stop it, with the reason, which its host
/// reads too once it has given it away; as the set starts over it is gone.
#[test]
fn fingerprints_of_another_content_are_refused() {
    let mut ai = NetAi::of(net(), Level::Hard, 5, fingerprints("exe5")).unwrap();
    let stopped = ai.stopped();
    assert_eq!(stopped.why(), None);
    let (pressed, _) = play("exe6", 4, &mut ai, 600, 0);
    assert!(pressed.iter().all(|&h| h == 0));
    assert!(ai.error().is_some_and(|e| e.contains("fingerprints are of content")), "{:?}", ai.error());
    assert_eq!(stopped.why(), ai.error(), "the host's");
    ai.reset();
    assert_eq!((ai.error(), stopped.why()), (None, None));
}

/// The shipped weights are this encoder's, with the three levels, Easy the
/// slowest to react and Hard the quickest, taking its best action; read once,
/// one network every player shares.
#[test]
fn the_shipped_weights_play_at_three_levels() {
    let net = shipped().unwrap();
    assert!(Arc::ptr_eq(&net, &shipped().unwrap()), "read once");
    let knobs = Level::ALL.map(|l| net.level(l.name()).cloned().unwrap_or_else(|| panic!("{l:?}")));
    let [easy, normal, hard] = &knobs;
    assert!(easy.delay > normal.delay && normal.delay > hard.delay, "{knobs:?}");
    assert!(hard.greedy && !easy.greedy && !normal.greedy, "{knobs:?}");
    let ai = NetAi::of(net, Level::Hard, 5, fingerprints("exe6")).unwrap();
    assert_eq!(ai.knobs(), hard);
}

/// Fingerprints made on threads of their own (the host's, here std's) as
/// they are asked for are `Fingerprints::compute`'s, written to the cache
/// folder, and read from it the next time without being made.
#[test]
fn fingerprints_are_made_as_asked_on_threads_and_kept() {
    let game = "exe6";
    let c = content(game);
    let chips: Vec<_> = ["cannon", "sword", "minibomb"].iter().filter_map(|k| nettai_match::ids::chip(&c, game, k)).collect();
    let dir = std::env::temp_dir().join(format!("nettai-fingerprints-{}", std::process::id()));
    let wait = |f: &Fingerprinter| {
        for _ in 0..6000 {
            match f.status() {
                Status::Ready => return f.given().1,
                Status::Failed(e) => panic!("{e}"),
                _ => std::thread::sleep(std::time::Duration::from_millis(10)),
            }
        }
        panic!("never made");
    };
    let made = Fingerprinter::begin(&c, 2, Some(dir.clone()));
    made.want(chips.clone());
    let f = wait(&made);
    let want = Fingerprints::compute(&c, &chips, 1).unwrap();
    assert_eq!(f.chips, want.chips);
    assert_eq!(made.unkept(), None);
    assert!(dir.join(fingerprint::file_name(&c)).is_file());
    let read = Fingerprinter::begin(&c, 2, Some(dir.clone()));
    read.want(chips);
    let again = wait(&read);
    assert_eq!(again.chips, f.chips);
    assert_eq!(read.generation(), 1, "read, not made");
    let _ = std::fs::remove_dir_all(&dir);
}
