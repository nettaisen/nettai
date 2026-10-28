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
fn play(game: &str, seed: u32, ai: &mut dyn Ai, ticks: u32, theirs: u16) -> (Vec<u16>, Battle) {
    let (pressed, _, b) = play_fight(game, seed, ai, ticks, theirs);
    (pressed, b)
}

/// The same, and whether the fight ran on each tick.
fn play_fight(game: &str, seed: u32, ai: &mut dyn Ai, ticks: u32, theirs: u16) -> (Vec<u16>, Vec<bool>, Battle) {
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
        let held = if custom { ai.custom_screen(&b, 1) } else { ai.fight(&b, 1, theirs) };
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
/// between, and never presses R, START or SELECT (L, to open the custom
/// screen, its own test's).
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
        assert!(pressed.iter().all(|h| h & (keys::R | keys::SELECT | keys::START) == 0), "{game}");
        // (Between decisions in the fight, the buttons held stay as they were.)
        let decisions: std::collections::HashSet<usize> = ai.decided.iter().map(|&(t, _)| t as usize).collect();
        for t in 1..pressed.len() {
            // (Its L, pressed and let go as it opens the custom screen, apart.)
            if runs[t] && runs[t - 1] && !decisions.contains(&t) {
                assert_eq!(pressed[t] & !keys::L, pressed[t - 1] & !keys::L, "{game}: tick {t}, no decision");
            }
        }
    }
}

/// At Hard it looks ahead (§10.7) on a thread of its own: each fight
/// decision asked for `lead` ticks before it takes effect, from the view of
/// its delay before that, and taken on a tick of `k`. Its buttons are the
/// same every time, however fast the thread answers, and they aren't the
/// network's alone.
#[test]
fn at_hard_it_looks_ahead_the_same_every_time() {
    for game in crate::testing::GAMES {
        let mut ai = NetAi::of(net(), Level::Hard, 5, fingerprints(game)).unwrap();
        assert!(ai.search.is_some() && ai.worker.is_some(), "{game}: Hard looks ahead");
        let (one, _, b) = play_fight(game, 9, &mut ai, 2400, 0);
        assert_eq!(ai.error(), None, "{game}");
        assert!(b.round.turn >= 2, "{game}: it got through {} custom screens", b.round.turn);
        let d = ai.knobs.delay as u32;
        assert!(ai.decided.len() > 50, "{game}: {} fight decisions", ai.decided.len());
        for &(t, seen) in &ai.decided {
            assert!(t.is_multiple_of(ai.k), "{game}: a decision at tick {t}");
            assert_eq!(seen, t - d, "{game}: the decision at {t} saw the view of {seen}");
        }
        ai.reset();
        let (again, _, _) = play_fight(game, 9, &mut ai, 2400, 0);
        assert_eq!(one, again, "{game}: started over, the same buttons");
        let mut alone = NetAi::of(net(), Level::Hard, 5, fingerprints(game)).unwrap().with_search(None).unwrap();
        assert!(alone.worker.is_none());
        let (plain, _, _) = play_fight(game, 9, &mut alone, 2400, 0);
        assert_ne!(one, plain, "{game}: the lookahead changes its choices");
    }
}

/// It opens the custom screen itself as the env did in training (the user:
/// "allow the dummy to go into custom screen"): with the other side never
/// opening it, L a while after the gauge is full, within `opener_wait`'s
/// 600 ticks, the while drawn anew each time.
#[test]
fn it_opens_the_custom_screen_as_the_env_did() {
    let game = "exe6";
    let c = content(game);
    let maker = crate::builds::Maker::new(&c).unwrap();
    let mut waits = Vec::new();
    for seed in [3, 4] {
        let mut ai = NetAi::of(net(), Level::Normal, seed as u64, fingerprints(game)).unwrap();
        let mut picks = Picks::new(seed);
        let none = crate::builds::Holdout::default();
        let sides = [0, 1].map(|_| maker.random(&mut picks, &none, 0.75, crate::builds::Combos::NONE).unwrap().side);
        let m = maker.round(sides, 1000, seed).unwrap();
        let mut b = Set::of(&c, &m, seed).start();
        let (mut full_since, mut first_l) = (None, None);
        for t in 0..4000u32 {
            let custom = b.round.mode == mode::CUSTOM;
            // (The other side picks its chips and never opens the screen.)
            let mine = if custom { nettai_frontend::driver::bot_buttons(&b, 0, t) } else { 0 };
            let held = if custom { ai.custom_screen(&b, 1) } else { ai.fight(&b, 1, 0) };
            if env::may_open(&b) {
                full_since.get_or_insert(t);
                if held & keys::L != 0 {
                    first_l.get_or_insert(t);
                }
            }
            if !env::step_battle(&mut b, [mine, held]) || b.is_battle_over() {
                break;
            }
            // (A screen the gauge was full for: not a round's first, which opens on its own.)
            if !custom && b.round.mode == mode::CUSTOM {
                if let Some(since) = full_since.take() {
                    let l = first_l.take().expect("the AI's L opened it");
                    waits.push(l - since);
                }
            }
        }
    }
    println!("waits {waits:?}");
    assert!(waits.len() >= 4, "{waits:?}");
    assert!(waits.iter().all(|&w| w <= 601), "{waits:?}");
    assert!(waits.iter().any(|&w| w != waits[0]), "{waits:?}: drawn anew");
}

/// Its network's memory takes each decision's action, the next decision's
/// GRU input: on the custom screen the slot or entry it chose, in the
/// fight the action it holds; the set starting over forgets it with the
/// GRU's state (a new round's [`Memory::reset`] too).
#[test]
fn its_memory_takes_what_each_decision_took() {
    let game = "exe6";
    let mut ai = NetAi::of(net(), Level::Normal, 5, fingerprints(game)).unwrap();
    assert_eq!(ai.mem.last, None);
    let mut on_screen = false;
    for ticks in (20..900).step_by(10) {
        ai.reset();
        let (_, _, b) = play_fight(game, 9, &mut ai, ticks, 0);
        if b.round.mode == mode::CUSTOM && ai.round.as_ref().is_some_and(|r| r.decisions > 0) {
            assert!(matches!(ai.mem.last, Some(Action::Screen(a)) if a < spec::SCREEN_ACTIONS), "{:?}", ai.mem.last);
            on_screen = true;
            break;
        }
    }
    assert!(on_screen, "no screen decision in the first custom screen");
    ai.reset();
    assert_eq!((ai.mem.last, ai.mem.h.iter().all(|&v| v == 0.0)), (None, true));
    // (A play that ends 40 ticks into a fight that runs: its last decision a fight one.)
    let mut in_fight = false;
    for ticks in (1200..2400).step_by(7) {
        ai.reset();
        let (_, runs, _) = play_fight(game, 9, &mut ai, ticks, 0);
        if runs.len() == ticks as usize && runs[runs.len() - 40..].iter().all(|&r| r) {
            let held = ai.round.as_ref().unwrap().action as usize;
            assert_eq!(ai.mem.last, Some(Action::Fight(held)));
            in_fight = true;
            break;
        }
    }
    assert!(in_fight, "no play ended in a fight that ran");
    ai.reset();
    assert_eq!(ai.mem.last, None);
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

/// The weights carry the three levels, Easy the slowest to react and Hard
/// the quickest, taking its best action (an export's, the fixture's too);
/// and they play EXE6 through its custom screens at Hard.
fn plays_at_three_levels(net: Arc<Net>) {
    let knobs = Level::ALL.map(|l| net.level(l.name()).cloned().unwrap_or_else(|| panic!("{l:?}")));
    let [easy, normal, hard] = &knobs;
    assert!(easy.delay > normal.delay && normal.delay > hard.delay, "{knobs:?}");
    assert!(hard.greedy && !easy.greedy && !normal.greedy, "{knobs:?}");
    let mut ai = NetAi::of(net, Level::Hard, 5, fingerprints("exe6")).unwrap();
    assert_eq!(ai.knobs(), hard);
    let (_, _, b) = play_fight("exe6", 9, &mut ai, 2400, 0);
    assert_eq!(ai.error(), None);
    assert!(b.round.turn >= 2, "it got through {} custom screens", b.round.turn);
}

#[test]
fn the_fixtures_weights_play_at_three_levels() {
    plays_at_three_levels(net());
}

/// Weights to publish (the file `NETTAI_AI_WEIGHTS` names: the publishing
/// step's check, `nettai_ai.publish`) are this encoder's and play at the
/// three levels. Not run unless asked (`--ignored`): no weights are in the
/// repository but the fixture's.
#[test]
#[ignore]
fn weights_to_publish_play_at_three_levels() {
    let path = std::env::var("NETTAI_AI_WEIGHTS").expect("NETTAI_AI_WEIGHTS: the weights to check");
    plays_at_three_levels(Arc::new(Net::from_safetensors(&std::fs::read(&path).unwrap()).unwrap()));
}

/// A waiting AI presses nothing while its network hasn't arrived, its
/// custom screen held; once it has, it plays as that network's `NetAi`
/// does from there, every AI waiting on it sharing it.
#[test]
fn a_waiting_ai_waits_for_its_network_then_plays() {
    let game = "exe6";
    let arriving = Arriving::default();
    let mut waiting = WaitingAi::new(arriving.clone(), Level::Easy, 5, fingerprints(game));
    let (pressed, b) = play(game, 4, &mut waiting, 900, 0);
    assert!(pressed.iter().all(|&h| h == 0));
    assert_eq!(b.round.mode, mode::CUSTOM, "the custom screen waits for it");
    assert_eq!((waiting.error(), waiting.stopped().why()), (None, None));
    arriving.arrive(Ok(net()));
    let mut ai = NetAi::of(net(), Level::Easy, 5, fingerprints(game)).unwrap();
    let mut arrived = WaitingAi::new(arriving.clone(), Level::Easy, 5, fingerprints(game));
    let (one, _) = play(game, 4, &mut ai, 1500, 0);
    let (two, _) = play(game, 4, &mut arrived, 1500, 0);
    assert_eq!(one, two, "the network's NetAi");
    assert!(one.iter().any(|&h| h != 0));
    let (_, b) = play(game, 4, &mut waiting, 1500, 0);
    assert!(b.round.turn >= 1 && waiting.error().is_none(), "the first one plays too: turn {}", b.round.turn);
}

/// A network that arrives as an error, or one without the level, stops a
/// waiting AI with the reason, which its host reads too.
#[test]
fn a_network_that_arrives_as_an_error_stops_it() {
    let mut ai = WaitingAi::new(Arriving::arrived(Err("no connection".into())), Level::Hard, 5, fingerprints("exe6"));
    let stopped = ai.stopped();
    let (pressed, _) = play("exe6", 4, &mut ai, 300, 0);
    assert!(pressed.iter().all(|&h| h == 0));
    assert_eq!(ai.error().as_deref(), Some("no connection"));
    assert_eq!(stopped.why().as_deref(), Some("no connection"), "the host's");
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
