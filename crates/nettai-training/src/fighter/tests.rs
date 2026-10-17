use super::*;
use nettai_battle::Content;
use nettai_frontend::driver::{Driver, InFight, LivePlayer, OnCustomScreen, Opponent};
use nettai_match::Set;
use nettai_netplay::standin::Masher;
use std::sync::Arc;

/// Game `game`'s content: the repository's, on a made-up asset index with
/// every sprite timed alike (`nettai_match::testing`); or, with
/// `FIGHTER_PACKS` (a packs directory: `<dir>/exe6` and so on), the game
/// as the app loads it, its pack's timings and all.
fn content(game: &str) -> Arc<Content> {
    if let Ok(dir) = std::env::var("FIGHTER_PACKS") {
        static LOADED: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, Arc<Content>>>> = std::sync::OnceLock::new();
        let mut loaded = LOADED.get_or_init(Default::default).lock().unwrap();
        return loaded
            .entry(game.to_string())
            .or_insert_with(|| {
                let found = nettai_frontend::game::Found::find(std::path::Path::new(&dir), &[]).unwrap_or_else(|e| panic!("packs in {dir}: {e}"));
                let content = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
                nettai_frontend::game::Game::load(&found, Some(&content), game).unwrap_or_else(|e| panic!("{game}: {e}: {:?}", e.report)).content
            })
            .clone();
    }
    match game {
        "exe4" => nettai_match::testing::exe4_content(),
        "exe5" => nettai_match::testing::exe5_content(),
        _ => nettai_match::testing::exe6_content(),
    }
}

/// A match of `game` picked from `seed`, both sides the first's player
/// with `hp`.
fn even_match(content: &Arc<Content>, game: &str, seed: u32, hp: i64) -> nettai_match::Match {
    let mut m = nettai_match::pick::live(content, game, seed, None).unwrap();
    m.sides[1] = m.sides[0].clone();
    let base_hp = nettai_battle::content::PlayerFact::BaseHp.name();
    for side in &mut m.sides {
        side.set_fact(content, base_hp, &[nettai_battle::rules::Fact::Value(nettai_content_api::Value::Int(hp))]).unwrap();
    }
    assert_eq!(nettai_match::check_match(content, &m), Vec::<String>::new());
    m
}

/// Presses at random in the fight, as live play's
/// `InFight::AttacksAtRandom` did, and picks most on the custom screen.
struct RandomPresses {
    masher: Masher,
    picker: Picker,
}

impl RandomPresses {
    fn new(seed: u64) -> RandomPresses {
        let mut masher = Masher::new(seed);
        masher.buster = true;
        RandomPresses { masher, picker: Picker::default() }
    }
}

impl Ai for RandomPresses {
    fn fight(&mut self, _b: &Battle, _side: u8, _theirs: u16) -> u16 {
        self.masher.buttons()
    }

    fn custom_screen(&mut self, b: &Battle, side: u8) -> u16 {
        // (Its presses go on as the screen is shown: one a tick, as live play's.)
        self.masher.buttons();
        self.picker.buttons(b, side as usize, None)
    }

    fn reset(&mut self) {}
}

/// A scripted player that keeps to the dummy's row and attacks it there:
/// it walks to the row the dummy is on, shoots every `every` ticks there
/// (a tap of B), and uses its chip there at once (every chip it can
/// picked, in the cursor's order; the custom screen opened as soon as its
/// hand is empty). What a dummy that dodges keeps out of.
struct Shooter {
    every: u32,
    t: u32,
    dir: u16,
    held: u8,
    last: u16,
    picker: Picker,
}

impl Shooter {
    fn new(every: u32) -> Shooter {
        Shooter { every, t: 0, dir: 0, held: 0, last: 0, picker: Picker::default() }
    }

    fn next(&mut self, b: &Battle, side: u8) -> u16 {
        let (Some(at), Some(them)) = (navi_at(b, side), navi_at(b, side ^ 1)) else {
            return 0;
        };
        if self.last != 0 {
            self.held += 1;
            if self.dir != 0 && self.held < 10 && at.y != them.y {
                return self.dir;
            }
            self.dir = 0;
            return 0;
        }
        self.held = 0;
        // (With nothing in hand, the custom screen, where the gauge lets it.)
        if b.hands[side as usize].remaining() == 0 && self.t % 30 == 0 {
            return keys::L;
        }
        if at.y != them.y {
            self.dir = if them.y < at.y { keys::UP } else { keys::DOWN };
            return self.dir;
        }
        if b.hands[side as usize].remaining() > 0 {
            return keys::A;
        }
        if self.t % self.every < 2 {
            return keys::B;
        }
        0
    }
}

impl Ai for Shooter {
    fn fight(&mut self, b: &Battle, side: u8, _theirs: u16) -> u16 {
        self.t += 1;
        let pressed = self.next(b, side);
        self.last = pressed;
        pressed
    }

    fn custom_screen(&mut self, b: &Battle, side: u8) -> u16 {
        self.picker.buttons(b, side as usize, None)
    }

    fn reset(&mut self) {}
}

/// A scripted player that only moves: a step up or down (or none) every
/// `every` ticks, by a seeded draw, and opens the custom screen as soon
/// as the gauge lets it (so the dummy has chips), picking nothing. What a
/// dummy's attacks must line up with to land.
struct Dodger {
    every: u32,
    t: u32,
    rng: SplitMix64,
    held: u16,
}

impl Ai for Dodger {
    fn fight(&mut self, _b: &Battle, _side: u8, _theirs: u16) -> u16 {
        self.t += 1;
        if self.t % self.every == 0 {
            self.held = [0, keys::UP, keys::DOWN][self.rng.below(3) as usize];
        } else if self.t % self.every >= 3 {
            self.held = 0;
        }
        if self.t % 30 == 15 {
            return keys::L;
        }
        self.held
    }

    fn custom_screen(&mut self, b: &Battle, side: u8) -> u16 {
        // (OK at once: nothing picked.)
        let s = &b.custom.sides[side as usize];
        match s.screen.as_ref().filter(|x| x.phase == Phase::Choosing) {
            Some(x) if self.t % 2 == 0 => {
                self.t += 1;
                if x.cursor != OK_SLOT { keys::START } else { keys::A }
            }
            _ => {
                self.t += 1;
                0
            }
        }
    }

    fn reset(&mut self) {}
}

/// Whether the round is decided: both navis came on the field, and one
/// is deleted or the time is up. (Before they come, the round reads as
/// over: no navis left.)
struct Decided {
    started: bool,
}

impl Decided {
    fn now(&mut self, b: &Battle) -> bool {
        self.started |= b.player(0).is_some() && b.player(1).is_some() && !b.is_battle_over();
        self.started && b.is_battle_over()
    }
}

/// One tick of each of `ais`' buttons for its side of `b`: the right one
/// the dummy, which never opens the custom screen (live play clears L and
/// R from a computer-played dummy's presses in the fight: only the player
/// opens it).
fn both(b: &Battle, ais: [&mut dyn Ai; 2]) -> [u16; 2] {
    let custom = b.round.mode == mode::CUSTOM;
    let [left, right] = ais;
    let l = if custom { left.custom_screen(b, 0) } else { left.fight(b, 0, 0) };
    let r = if custom { right.custom_screen(b, 1) } else { right.fight(b, 1, l) & !(keys::L | keys::R) };
    [l, r]
}

/// How a round went: the side that won (none: undecided or a draw),
/// each navi's HP at its end, and its ticks; and, of the dummy (the right
/// side), what it did.
#[derive(Debug, Default)]
struct Round {
    won: Option<u8>,
    hp: [i64; 2],
    ticks: u32,
    /// The ticks of the fight (not dimmed, not on the custom screen).
    fight: u32,
    /// By side: the chips used (its hand one fewer in the fight), and
    /// the custom screens opened.
    chips: [u32; 2],
    screens: u32,
    /// By side: the longest stretch of the fight, in ticks, with chips in
    /// hand and none used.
    holding: [u32; 2],
    /// The dummy: the hits it took (ticks its HP went down).
    hits: u32,
    /// Its chips that attack (a damage, or a formula), and of those the
    /// ones after which the player lost HP within [`LANDS`] ticks; its chips
    /// that don't, and of those the ones used within [`THREAT`] ticks of a
    /// hit taken or the player pressing A or B.
    attacks: u32,
    landed: u32,
    supports: u32,
    timely: u32,
    /// Its steps (its panel changed), the steps back to the panel it had
    /// just left within [`BACK`] ticks, and the fight ticks between such
    /// pairs.
    steps: u32,
    backs: u32,
    pacing: u32,
    /// The custom screens it left in another form.
    forms: u32,
}

/// The ticks after a chip within which the player losing HP counts as it
/// landing.
const LANDS: u32 = 120;
/// The ticks after a threat within which a chip that doesn't attack
/// counts as used against it.
const THREAT: u32 = 60;
/// The ticks within which a step back counts as pacing.
const BACK: u32 = 20;

/// The first round of `m` stepped by hand: the left navi `player`, the
/// right one `dummy`; up to `most` ticks.
fn first_round_against(content: &Arc<Content>, m: &nettai_match::Match, seed: u32, player: &mut dyn Ai, dummy: &mut dyn Ai, most: u32) -> Round {
    let mut b = Set::of(content, m, seed).start();
    let mut decided = Decided { started: false };
    let mut r = Round::default();
    let mut held = [0u32; 2];
    // (Chips in flight: the tick used and the player's HP then; the last threat; the dummy's last step.)
    let mut flying: Vec<(u32, i64)> = Vec::new();
    let mut threat: Option<u32> = None;
    let mut last_step: Option<(PanelPos, PanelPos, u32)> = None;
    let mut last_hp = 0;
    let mut last_theirs = 0u16;
    let mut formed = false;
    while r.ticks < most && !decided.now(&b) && !b.is_stopped() {
        r.ticks += 1;
        let [left, right] = both(&b, [&mut *player, &mut *dummy]);
        let (hands, custom) = (b.hands.each_ref().map(|h| h.remaining()), b.round.mode == mode::CUSTOM);
        let next = b.hands[1].next_chip();
        let at = navi_at(&b, 1);
        step(&mut b, 0, left, right);
        let fighting = fight_goes_on(&b) && !b.is_dimmed();
        r.fight += fighting as u32;
        for side in 0..2 {
            let used = b.hands[side].remaining() != hands[side];
            let chip = b.round.mode == mode::FIGHTING && b.hands[side].remaining() < hands[side];
            r.chips[side] += chip as u32;
            held[side] = if used || hands[side] == 0 { 0 } else { held[side] + fight_goes_on(&b) as u32 };
            r.holding[side] = r.holding[side].max(held[side]);
            if side == 1 && chip {
                let data = b.chip_field(next);
                if data.damage > 0 || data.formula.is_some() {
                    r.attacks += 1;
                    flying.push((r.ticks, hp(&b, 0)));
                } else {
                    r.supports += 1;
                    r.timely += threat.is_some_and(|t| r.ticks - t <= THREAT) as u32;
                }
            }
        }
        // (A chip lands where the player loses HP within its ticks.)
        let player_hp = hp(&b, 0);
        flying.retain(|&(t, was)| {
            if player_hp < was {
                r.landed += 1;
                return false;
            }
            r.ticks - t < LANDS
        });
        let dummy_hp = hp(&b, 1);
        if dummy_hp < last_hp {
            r.hits += 1;
            threat = Some(r.ticks);
        }
        last_hp = dummy_hp;
        let pressed = left & !std::mem::replace(&mut last_theirs, left);
        if !custom && pressed & (keys::A | keys::B) != 0 {
            threat = Some(r.ticks);
        }
        if let (Some(from), Some(to)) = (at, navi_at(&b, 1)) {
            if from != to && fight_goes_on(&b) {
                r.steps += 1;
                if let Some((was, _, t)) = last_step.filter(|(was, left_to, t)| *was == to && *left_to == from && r.ticks - t <= BACK) {
                    let _ = was;
                    r.backs += 1;
                    r.pacing += r.ticks - t;
                }
                last_step = Some((from, to, r.ticks));
            }
        }
        r.screens += (!custom && b.round.mode == mode::CUSTOM) as u32;
        let form = b.custom.sides[1].screen.is_some_and(|s| (s.form.is_some() || s.trade.is_some() || s.hold.is_some()) && b.round.mode == mode::CUSTOM);
        r.forms += (form && !formed) as u32;
        formed = form || (formed && b.round.mode == mode::CUSTOM);
    }
    if let Some(nettai_battle::battle::RoundEnd::Error(why)) = b.round_end() {
        println!("seed {seed}: the engine stopped at tick {}: {why}", r.ticks);
    }
    r.hp = [hp(&b, 0), hp(&b, 1)];
    r.won = match r.hp {
        [0, 0] => None,
        [0, _] => Some(1),
        [_, 0] => Some(0),
        _ => None,
    };
    r
}

/// Plugged into live play as its opponent's AI, in the fight and on
/// the custom screen, it plays as it does by hand: the same battle,
/// tick for tick (and the same again after the set starts over), one
/// thinking on its own thread as one thinking inline. It deletes a
/// player pressing at random.
#[test]
fn plugged_into_live_play_it_plays_as_by_hand() {
    let content = content("exe6");
    let m = even_match(&content, "exe6", 3, 400);
    let opponent = Opponent { in_fight: InFight::Ai, on_custom_screen: OnCustomScreen::Ai };
    let mut live = LivePlayer::against(Set::of(&content, &m, 3), opponent, 3);
    live.set_ai(Some(Box::new(Fighter::new(Level::HARD, 3))));
    let by_live = |live: &mut LivePlayer| {
        let mut b = live.start();
        let mut player = RandomPresses::new(3 ^ 0x5EED);
        let (mut digests, mut decided) = (Vec::new(), Decided { started: false });
        while digests.len() < 6000 && !decided.now(&b) {
            let left = if b.round.mode == mode::CUSTOM { player.custom_screen(&b, 0) } else { player.fight(&b, 0, 0) };
            let s = live.next(&b, left).unwrap();
            b.tick(&s.input, s.events);
            digests.push(b.digest());
        }
        (digests, decided.now(&b), [hp(&b, 0), hp(&b, 1)])
    };
    let (live_digests, over, hp) = by_live(&mut live);
    // (The fighter deletes the player.)
    assert_eq!((over, hp[0]), (true, 0), "{} ticks, HP {hp:?}", live_digests.len());
    assert!(hp[1] > 0);
    assert_eq!(by_live(&mut live).0, live_digests, "the set started over");
    // By hand.
    let mut b = Set::of(&content, &m, 3).start();
    let mut player = RandomPresses::new(3 ^ 0x5EED);
    let mut fighter = Fighter::inline(Level::HARD, 3);
    for (tick, digest) in live_digests.iter().enumerate() {
        let [left, right] = both(&b, [&mut player, &mut fighter]);
        step(&mut b, 0, left, right);
        assert_eq!(b.digest(), *digest, "tick {tick}");
    }
}

/// Its plans walk where they say: from where the navi stands, a plan to
/// each panel it can reach gets it there, a step at a time (the direction
/// keys move it as [`stepped`] says).
#[test]
fn a_plan_walks_to_its_panel() {
    let content = content("exe6");
    let m = even_match(&content, "exe6", 1, 400);
    let b = Set::of(&content, &m, 1).start();
    let b = to_the_fight(&b, 1, &|_| Some(0), &mut 0).expect("the fight starts");
    let panels = reachable(&b, 1);
    assert!(panels.len() >= 6, "{panels:?}");
    for &(to, _) in &panels {
        let mut trial = b.clone();
        let mut exec = Exec::new(Plan { to, then: Then::Hold, after: None });
        for _ in 0..120 {
            let mine = exec.buttons(&trial, 1);
            step(&mut trial, 1, mine, 0);
        }
        assert_eq!(navi_at(&trial, 1), Some(to));
    }
}

/// A level by name (`easy`, `normal`, `hard`) or by its parts
/// (`r6h60e8g7b0`: reaction, horizon, every, guesses, blunders, any left
/// out as Hard's); none for `random`.
fn level_named(name: &str) -> Option<Level> {
    match name {
        "random" => return None,
        "easy" => return Some(Level::EASY),
        "normal" => return Some(Level::NORMAL),
        "hard" => return Some(Level::HARD),
        _ => {}
    }
    let mut level = Level::HARD;
    let mut rest = name;
    while let Some(c) = rest.chars().next() {
        let digits: String = rest[1..].chars().take_while(|c| c.is_ascii_digit()).collect();
        let n: u32 = digits.parse().unwrap_or_else(|_| panic!("a level's part {c} has no number: {name:?}"));
        match c {
            'r' => level.reaction = n,
            'h' => level.horizon = n,
            'e' => level.every = n,
            'g' => level.guesses = n,
            'b' => level.blunders = n,
            'a' => level.alert = n != 0,
            'k' => level.caution = n,
            's' => level.steady = n,
            'p' => level.chooses = n != 0,
            'f' => level.forms = n != 0,
            'l' => level.looks = n,
            _ => panic!("no level part {c:?} in {name:?}"),
        }
        rest = &rest[1 + digits.len()..];
    }
    Some(level)
}

/// What plays a side by name: `random` (presses at random), `shooter`
/// and `dodger` (scripted: [`Shooter`], [`Dodger`]), `old-<level>` (the
/// fighter before it planned by tiles, [`old`]), or a level of this one
/// ([`level_named`]); and what its decisions cost, read when it's done.
enum Named {
    Random(RandomPresses),
    Shooter(Shooter),
    Dodger(Dodger),
    Old(old::Fighter),
    New(Fighter),
}

impl Named {
    fn of(name: &str, seed: u64) -> Named {
        if let Some(old) = name.strip_prefix("old-") {
            let level = match old {
                "easy" => old::Level::EASY,
                "normal" => old::Level::NORMAL,
                _ => old::Level::HARD,
            };
            return Named::Old(old::Fighter::new(level, seed));
        }
        match name {
            "random" => Named::Random(RandomPresses::new(seed)),
            "shooter" => Named::Shooter(Shooter::new(12)),
            "dodger" => Named::Dodger(Dodger { every: 40, t: 0, rng: SplitMix64::new(seed), held: 0 }),
            _ => Named::New(Fighter::new(level_named(name).expect("a level"), seed)),
        }
    }

    fn ai(&mut self) -> &mut dyn Ai {
        match self {
            Named::Random(a) => a,
            Named::Shooter(a) => a,
            Named::Dodger(a) => a,
            Named::Old(a) => a,
            Named::New(a) => a,
        }
    }

    fn spent(&self) -> Spent {
        match self {
            Named::New(f) => f.spent,
            Named::Old(f) => {
                let s = f.spent;
                Spent { decisions: s.decisions, ticks: s.ticks, time: s.time, longest: s.longest, late: s.late, waited: s.waited, chips: s.chips, chip_time: s.chip_time, forms: 0, ..Spent::default() }
            }
            _ => Spent::default(),
        }
    }
}

/// The rounds of each game's matches between a player and dummies, each
/// round on a thread of `FIGHTER_THREADS`: by game and dummy, the rounds
/// and what each dummy's decisions cost.
fn rounds(player: &str, games: &str, dummies: &str, seeds: u32, hp: i64, most: u32) -> Vec<(String, String, Vec<(Round, Spent)>)> {
    let var = |name: &str, or: &str| std::env::var(name).unwrap_or_else(|_| or.into());
    let threads: usize = var("FIGHTER_THREADS", "4").parse().unwrap();
    let mut out = Vec::new();
    for game in games.split(',') {
        let content = content(game);
        for dummy in dummies.split(',') {
            let results = std::sync::Mutex::new(Vec::new());
            let next = std::sync::atomic::AtomicU32::new(0);
            std::thread::scope(|s| {
                for _ in 0..threads {
                    s.spawn(|| {
                        loop {
                            let seed = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            if seed >= seeds {
                                break;
                            }
                            let m = even_match(&content, game, seed, hp);
                            let mut d = Named::of(dummy, seed as u64);
                            let mut p = Named::of(player, seed as u64 ^ 0x5EED);
                            let round = first_round_against(&content, &m, seed, p.ai(), d.ai(), most);
                            results.lock().unwrap().push((seed, round, d.spent()));
                        }
                    });
                }
            });
            let mut results = results.into_inner().unwrap();
            results.sort_by_key(|(seed, _, _)| *seed);
            out.push((game.to_string(), dummy.to_string(), results.into_iter().map(|(_, r, s)| (r, s)).collect()));
        }
    }
    out
}

/// The opponent that fights against a player (pressing at random, or
/// another), the first rounds of each game's matches, both sides the same
/// build at 400 HP. Prints its wins, the HP it takes away and loses a
/// minute, and what deciding cost. A measurement: `cargo test -p
/// nettai-training --release --lib against_random_presses -- --ignored
/// --nocapture`, with `FIGHTER_GAMES` (exe6,exe5,exe4), `FIGHTER_LEVELS`
/// (the dummies: random,old-hard,easy,normal,hard; or parts,
/// [`level_named`]), `FIGHTER_PLAYER` (what plays the player: random, or a
/// dummy's name, [`Named`]), `FIGHTER_SEEDS` (24), `FIGHTER_THREADS` (4)
/// and `FIGHTER_PACKS` ([`content`]).
#[test]
#[ignore]
fn against_random_presses() {
    nettai_frontend::session::quiet_engine_panics();
    const HP: i64 = 400;
    const MOST: u32 = 60 * 60 * 3;
    let var = |name: &str, or: &str| std::env::var(name).unwrap_or_else(|_| or.into());
    let seeds: u32 = var("FIGHTER_SEEDS", "24").parse().unwrap();
    let (games, levels) = (var("FIGHTER_GAMES", "exe6,exe5,exe4"), var("FIGHTER_LEVELS", "random,old-hard,easy,normal,hard"));
    let player = var("FIGHTER_PLAYER", "random");
    println!("player: {player}");
    println!(
        "| game | dummy | won | lost | undecided | HP a minute (dealt / taken) | chips a minute (its / player's) | longest hold, s (its / player's) | custom screens a minute | forms | ticks a round | decisions a round | ms a decision (mean / longest) | late a round (ms waited) | trial ticks a decision | µs a trial tick |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for (game, name, results) in rounds(&player, &games, &levels, seeds, HP, MOST) {
        let count = |won: Option<u8>| results.iter().filter(|(r, _)| r.won == won).count();
        let ticks: u64 = results.iter().map(|(r, _)| r.ticks as u64).sum();
        let minutes = ticks as f64 / 3600.0;
        let dealt: i64 = results.iter().map(|(r, _)| HP - r.hp[0]).sum();
        let taken: i64 = results.iter().map(|(r, _)| HP - r.hp[1]).sum();
        let decisions: u64 = results.iter().map(|(_, s)| s.decisions as u64).sum();
        let time: Duration = results.iter().map(|(_, s)| s.time).sum();
        let trial: u64 = results.iter().map(|(_, s)| s.ticks).sum();
        let longest = results.iter().map(|(_, s)| s.longest).max().unwrap_or_default();
        let per = |x: f64| {
            if decisions == 0 { 0.0 } else { x / decisions as f64 }
        };
        println!(
            "| {game} | {name} | {} | {} | {} | {:.0} / {:.0} | {:.1} / {:.1} | {:.1} / {:.1} | {:.1} | {} | {:.0} | {:.0} | {:.2} / {:.1} | {:.1} ({:.0}) | {:.0} | {:.2} |",
            count(Some(1)),
            count(Some(0)),
            count(None),
            dealt as f64 / minutes,
            taken as f64 / minutes,
            results.iter().map(|(r, _)| r.chips[1]).sum::<u32>() as f64 / minutes,
            results.iter().map(|(r, _)| r.chips[0]).sum::<u32>() as f64 / minutes,
            results.iter().map(|(r, _)| r.holding[1]).sum::<u32>() as f64 / 60.0 / results.len() as f64,
            results.iter().map(|(r, _)| r.holding[0]).sum::<u32>() as f64 / 60.0 / results.len() as f64,
            results.iter().map(|(r, _)| r.screens).sum::<u32>() as f64 / minutes,
            results.iter().map(|(r, _)| r.forms).sum::<u32>(),
            ticks as f64 / results.len() as f64,
            decisions as f64 / results.len() as f64,
            per(time.as_secs_f64() * 1000.0),
            longest.as_secs_f64() * 1000.0,
            results.iter().map(|(_, s)| s.late).sum::<u32>() as f64 / results.len() as f64,
            results.iter().map(|(_, s)| s.waited).sum::<Duration>().as_secs_f64() * 1000.0 / results.len() as f64,
            per(trial as f64),
            if trial == 0 { 0.0 } else { time.as_secs_f64() * 1e6 / trial as f64 },
        );
    }
}

/// Game sense, against scripted players: the first rounds of each game's
/// matches at 400 HP, the dummy at each level (and the fighter before it,
/// `old-hard`), the player `FIGHTER_PLAYER` (shooter: keeps to its row and
/// attacks there; dodger: steps up and down and never attacks; or any
/// [`Named`]). Prints, a minute of the fight (not dimmed, not on the custom
/// screen): the hits it takes and the HP each loses; its chips that attack
/// and the share that land; its chips that don't and the share used
/// against a threat; its steps, its steps back (to the panel just left
/// within [`BACK`] ticks) and the share of the fight spent pacing so; and
/// the forms it took. A measurement: `cargo test -p nettai-training
/// --release --lib game_sense -- --ignored --nocapture` (`FIGHTER_GAMES`,
/// `FIGHTER_LEVELS`, `FIGHTER_SEEDS` (12), `FIGHTER_THREADS`,
/// `FIGHTER_PACKS`).
#[test]
#[ignore]
fn game_sense() {
    nettai_frontend::session::quiet_engine_panics();
    const HP: i64 = 400;
    const MOST: u32 = 60 * 60 * 2;
    let var = |name: &str, or: &str| std::env::var(name).unwrap_or_else(|_| or.into());
    let seeds: u32 = var("FIGHTER_SEEDS", "12").parse().unwrap();
    let (games, levels) = (var("FIGHTER_GAMES", "exe6,exe5,exe4"), var("FIGHTER_LEVELS", "old-hard,easy,normal,hard"));
    for player in var("FIGHTER_PLAYER", "shooter,dodger").split(',') {
        println!("player: {player}");
        println!(
            "| game | dummy | won–lost | hits taken a min | HP a min (dealt / taken) | attack chips a min, landed | support chips a min, against a threat | steps a min | steps back a min | pacing | forms |"
        );
        println!("|---|---|---|---|---|---|---|---|---|---|---|");
        for (game, name, results) in rounds(player, &games, &levels, seeds, HP, MOST) {
            let sum = |f: &dyn Fn(&Round) -> u32| results.iter().map(|(r, _)| f(r)).sum::<u32>();
            let minutes = sum(&|r| r.fight).max(1) as f64 / 3600.0;
            let share = |a: u32, of: u32| {
                if of == 0 { "–".to_string() } else { format!("{:.0}%", 100.0 * a as f64 / of as f64) }
            };
            println!(
                "| {game} | {name} | {}–{} | {:.1} | {:.0} / {:.0} | {:.1}, {} | {:.1}, {} | {:.1} | {:.1} | {} | {} |",
                results.iter().filter(|(r, _)| r.won == Some(1)).count(),
                results.iter().filter(|(r, _)| r.won == Some(0)).count(),
                sum(&|r| r.hits) as f64 / minutes,
                results.iter().map(|(r, _)| HP - r.hp[0]).sum::<i64>() as f64 / minutes,
                results.iter().map(|(r, _)| HP - r.hp[1]).sum::<i64>() as f64 / minutes,
                sum(&|r| r.attacks) as f64 / minutes,
                share(sum(&|r| r.landed), sum(&|r| r.attacks)),
                sum(&|r| r.supports) as f64 / minutes,
                share(sum(&|r| r.timely), sum(&|r| r.supports)),
                sum(&|r| r.steps) as f64 / minutes,
                sum(&|r| r.backs) as f64 / minutes,
                share(sum(&|r| r.pacing), sum(&|r| r.fight)),
                sum(&|r| r.forms),
            );
        }
    }
}

/// What a tick of each game's battle costs, stepped with random presses
/// on both sides, and a copy of it: what the trials are made of. A
/// measurement: `cargo test -p nettai-training --lib tick_cost --
/// --ignored --nocapture`.
#[test]
#[ignore]
fn tick_cost() {
    nettai_frontend::session::quiet_engine_panics();
    for game in ["exe6", "exe5", "exe4"] {
        let content = content(game);
        let (mut ticks, mut stepping, mut copies, mut copying) = ([0u64; 2], [Duration::ZERO; 2], 0u64, Duration::ZERO);
        for seed in 0..4 {
            let m = even_match(&content, game, seed, 400);
            let mut b = Set::of(&content, &m, seed).start();
            let mut copy = b.clone();
            let (mut left, mut right) = (RandomPresses::new(seed as u64), RandomPresses::new(seed as u64 ^ 1));
            for _ in 0..3000 {
                let [l, r] = both(&b, [&mut left, &mut right]);
                let fight = fight_goes_on(&b) as usize;
                let at = Instant::now();
                step(&mut b, 0, l, r);
                stepping[fight] += at.elapsed();
                ticks[fight] += 1;
                if b.round.mode == mode::FIGHTING {
                    let at = Instant::now();
                    copy.clone_from(&b);
                    copying += at.elapsed();
                    copies += 1;
                }
            }
        }
        let per = |k: usize| stepping[k].as_secs_f64() * 1e6 / ticks[k].max(1) as f64;
        println!(
            "{game}: {:.1} µs a tick of the fight ({} ticks), {:.1} µs a tick otherwise ({}), {:.1} µs a copy",
            per(1),
            ticks[1],
            per(0),
            ticks[0],
            copying.as_secs_f64() * 1e6 / copies.max(1) as f64
        );
    }
}

/// Whether its decisions keep up with a battle run at its own speed (60
/// ticks a second, as the app runs one): a round of each game at each
/// level for `FIGHTER_SECONDS` (20), the dummy thinking on its own
/// thread, against `FIGHTER_PLAYER` (random). Prints the decisions that
/// weren't ready when they were to take effect and the time the battle
/// waited for them. A measurement: `cargo test -p nettai-training --lib
/// real_time -- --ignored --nocapture` (`FIGHTER_GAMES`, `FIGHTER_LEVELS`;
/// `FIGHTER_CASE`, a match of [`dimming_match`]'s in place of the picked
/// one).
#[test]
#[ignore]
fn real_time() {
    nettai_frontend::session::quiet_engine_panics();
    let var = |name: &str, or: &str| std::env::var(name).unwrap_or_else(|_| or.into());
    let seconds: u32 = var("FIGHTER_SECONDS", "20").parse().unwrap();
    let (games, levels) = (var("FIGHTER_GAMES", "exe6,exe5,exe4"), var("FIGHTER_LEVELS", "easy,normal,hard"));
    let player = var("FIGHTER_PLAYER", "random");
    println!(
        "| game | dummy | decisions | late | waited (ms, all / longest) | ms a decision (mean / longest) | custom screens | ms a screen (mean) | dimmed decisions | ms a dimmed decision (mean / longest) |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|");
    for game in games.split(',') {
        let content = content(game);
        for name in levels.split(',') {
            let m = match std::env::var("FIGHTER_CASE") {
                Ok(case) => dimming_match(&content, game, &case, 1),
                Err(_) => even_match(&content, game, 1, 400),
            };
            let mut b = Set::of(&content, &m, 1).start();
            let mut p = Named::of(&player, 1 ^ 0x5EED);
            let mut d = Named::of(name, 1);
            let (mut decided, mut longest_wait) = (Decided { started: false }, Duration::ZERO);
            let start = Instant::now();
            for tick in 0..seconds * 60 {
                let due = start + Duration::from_micros(tick as u64 * 16_667);
                if let Some(rest) = due.checked_duration_since(Instant::now()) {
                    std::thread::sleep(rest);
                }
                let waited = d.spent().waited;
                let [left, right] = both(&b, [p.ai(), d.ai()]);
                longest_wait = longest_wait.max(d.spent().waited - waited);
                step(&mut b, 0, left, right);
                if decided.now(&b) {
                    // (Another round, from where the set starts.)
                    b = Set::of(&content, &m, tick).start();
                    decided = Decided { started: false };
                }
            }
            let s = d.spent();
            println!(
                "| {game} | {name} | {} | {} | {:.0} / {:.0} | {:.1} / {:.0} | {} | {:.0} | {} | {:.1} / {:.0} |",
                s.decisions,
                s.late,
                s.waited.as_secs_f64() * 1000.0,
                longest_wait.as_secs_f64() * 1000.0,
                s.time.as_secs_f64() * 1000.0 / s.decisions.max(1) as f64,
                s.longest.as_secs_f64() * 1000.0,
                s.chips,
                s.chip_time.as_secs_f64() * 1000.0 / s.chips.max(1) as f64,
                s.dimmings,
                s.dim_time.as_secs_f64() * 1000.0 / s.dimmings.max(1) as f64,
                s.dim_longest.as_secs_f64() * 1000.0,
            );
        }
    }
}

/// A match for the dimming scenarios ([`dimming_chips`]), side 1 the
/// dummy's: `searchmn` (its MegaMan with EXE6's patch card SearchMn: its
/// charged shot CircGun), `eraseman` (its folder all EraseMan), `cut-in`
/// (the same, the player's folder all CircGun: what it may cut in on).
fn dimming_match(content: &Arc<Content>, game: &str, case: &str, seed: u32) -> nettai_match::Match {
    let mut m = even_match(content, game, seed, 400);
    let all = |name: &str, code: char| {
        let chip = nettai_match::ids::chip(content, game, name).unwrap_or_else(|| panic!("no chip {name}"));
        let code = nettai_battle::content::ChipCode::from_letter(code).unwrap();
        nettai_match::Folder { chips: [Some(FolderChip::new(chip, code)); 30], regular: None, tags: None }
    };
    match case {
        "searchmn" => {
            let cards = nettai_match::testing::patch_cards(content, game, "searchmn");
            m.sides[1] = m.sides[1].without_patch_cards(content);
            m.sides[1].set_fact(content, nettai_match::facts::PATCH_CARDS, &cards).unwrap();
        }
        "eraseman" => m.sides[1].set_folder(content, &all("eraseman", 'K')).unwrap(),
        "cut-in" => {
            m.sides[1].set_folder(content, &all("eraseman", 'K')).unwrap();
            m.sides[0].set_folder(content, &all("circgun", 'P')).unwrap();
        }
        _ => panic!("no dimming case {case:?}"),
    }
    m
}

/// How the dummy's dimmings went over a round ([`dimming_round`]).
#[derive(Debug, Default)]
struct Dimmings {
    won: Option<u8>,
    /// Each side's HP lost.
    lost: [i64; 2],
    fight: u32,
    /// Its dimming chips used (a dimming it started, or a cut-in), the
    /// dimmings in which the player lost HP, and the HP the player lost in
    /// them (from their start to [`DIM_SETTLE`] ticks after they end).
    uses: u32,
    hits: u32,
    damage: i64,
    /// The player's dimmings begun while the dummy's next chip was a
    /// dimming chip, those it cut in on, and the HP each side lost in them.
    counters: u32,
    cut: u32,
    exchange: [i64; 2],
}

/// The fight ticks after a dimming within which its HP lost counts.
const DIM_SETTLE: u32 = 10;

/// The first round of `m`, the left navi `player`, the right `dummy`,
/// up to `most` ticks: what the dummy's dimmings did.
fn dimming_round(content: &Arc<Content>, m: &nettai_match::Match, seed: u32, player: &mut dyn Ai, dummy: &mut dyn Ai, most: u32) -> Dimmings {
    struct Episode {
        initiator: u8,
        hp: [i64; 2],
        counter: bool,
        uses: u32,
        cut: bool,
        settle: Option<u32>,
    }
    let mut b = Set::of(content, m, seed).start();
    let mut decided = Decided { started: false };
    let mut d = Dimmings::default();
    let mut open: Option<Episode> = None;
    let mut ticks = 0;
    let mut first: Option<[i64; 2]> = None;
    let close = |e: Episode, b: &Battle, d: &mut Dimmings| {
        let lost = [e.hp[0] - hp(b, 0), e.hp[1] - hp(b, 1)];
        if e.uses > 0 {
            d.uses += e.uses;
            d.hits += (lost[0] > 0) as u32;
            d.damage += lost[0];
        }
        if e.counter {
            d.counters += 1;
            d.cut += e.cut as u32;
            d.exchange[0] += lost[0];
            d.exchange[1] += lost[1];
        }
    };
    while ticks < most && !decided.now(&b) && !b.is_stopped() {
        ticks += 1;
        let [left, right] = both(&b, [&mut *player, &mut *dummy]);
        let (was_dimmed, hand, hps) = (b.is_dimmed(), b.hands[1].remaining(), [hp(&b, 0), hp(&b, 1)]);
        let counter = b.hands[1].next_chip().is_some_and(|c| b.content.chip(c).flags.has(nettai_battle::content::ChipFlags::DIMMING));
        step(&mut b, 0, left, right);
        if decided.started && first.is_none() {
            first = Some([hp(&b, 0), hp(&b, 1)]);
        }
        d.fight += (fight_goes_on(&b) && !b.is_dimmed()) as u32;
        if b.is_dimmed() && !was_dimmed {
            if let Some(e) = open.take() {
                close(e, &b, &mut d);
            }
            let initiator = b.dimming[0].initiator;
            open = Some(Episode { initiator, hp: hps, counter: counter && initiator == 0, uses: (initiator == 1) as u32, cut: false, settle: None });
        }
        if let Some(e) = open.as_mut() {
            if b.is_dimmed() && b.hands[1].remaining() < hand {
                e.uses += 1;
                e.cut |= e.initiator == 0;
            }
            if !b.is_dimmed() {
                let left = e.settle.get_or_insert(DIM_SETTLE);
                *left = left.saturating_sub(1);
                if *left == 0 {
                    close(open.take().unwrap(), &b, &mut d);
                }
            }
        }
    }
    if let Some(e) = open.take() {
        close(e, &b, &mut d);
    }
    let hps = [hp(&b, 0), hp(&b, 1)];
    let first = first.unwrap_or(hps);
    d.lost = [first[0] - hps[0], first[1] - hps[1]];
    d.won = match hps {
        [0, 0] => None,
        [0, _] => Some(1),
        [_, 0] => Some(0),
        _ => None,
    };
    d
}

/// The dummy's dimming chips: a chip that waits for its user's A while
/// the battle is dimmed (CircGun's sight, EraseMan's aim) used where it
/// pays, aimed; and the player's dimmings cut in on. EXE6's cases
/// ([`dimming_match`]: `searchmn`, `eraseman`, `cut-in`), against a
/// player that stands still (`still`, opening the custom screen), one that
/// steps around (`dodger`) and one that attacks (`shooter`). Prints, a
/// minute of the fight: its dimming chips used, the share that hit, the
/// damage a use; the player's dimmings it could cut in on and the share it
/// did, and the HP each lost in them. A measurement: `cargo test -p
/// nettai-training --release --lib dimming_chips -- --ignored --nocapture`
/// (`FIGHTER_CASES` (searchmn/still,searchmn/dodger,eraseman/still,
/// eraseman/dodger,cut-in/shooter), `FIGHTER_LEVELS` (hard),
/// `FIGHTER_SEEDS` (12), `FIGHTER_THREADS`, `FIGHTER_PACKS`).
#[test]
#[ignore]
fn dimming_chips() {
    nettai_frontend::session::quiet_engine_panics();
    const MOST: u32 = 60 * 60 * 2;
    let var = |name: &str, or: &str| std::env::var(name).unwrap_or_else(|_| or.into());
    let seeds: u32 = var("FIGHTER_SEEDS", "12").parse().unwrap();
    let threads: usize = var("FIGHTER_THREADS", "4").parse().unwrap();
    let cases = var("FIGHTER_CASES", "searchmn/still,searchmn/dodger,eraseman/still,eraseman/dodger,cut-in/shooter");
    let content = content("exe6");
    println!(
        "| case | player | dummy | won–lost | HP a min (dealt / taken) | dimming chips a min | hit | damage a use | cut-ins (could / did) | HP in them (player / dummy) | ms a decision (mean / longest) | dimmed decisions, ms (mean / longest) | late |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for case in cases.split(',') {
        let (case, player) = case.split_once('/').expect("case/player");
        for level in var("FIGHTER_LEVELS", "hard").split(',') {
            let results = std::sync::Mutex::new(Vec::new());
            let next = std::sync::atomic::AtomicU32::new(0);
            std::thread::scope(|s| {
                for _ in 0..threads {
                    s.spawn(|| {
                        loop {
                            let seed = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            if seed >= seeds {
                                break;
                            }
                            let m = dimming_match(&content, "exe6", case, seed);
                            let mut d = Named::of(level, seed as u64);
                            let mut p = match player {
                                "still" => Named::Dodger(Dodger { every: u32::MAX, t: 0, rng: SplitMix64::new(seed as u64), held: 0 }),
                                _ => Named::of(player, seed as u64 ^ 0x5EED),
                            };
                            let r = dimming_round(&content, &m, seed, p.ai(), d.ai(), MOST);
                            results.lock().unwrap().push((r, d.spent()));
                        }
                    });
                }
            });
            let results = results.into_inner().unwrap();
            let spent: Vec<Spent> = results.iter().map(|(_, s)| *s).collect();
            let results: Vec<Dimmings> = results.into_iter().map(|(r, _)| r).collect();
            let sum = |f: &dyn Fn(&Dimmings) -> i64| results.iter().map(f).sum::<i64>();
            let decisions = spent.iter().map(|s| s.decisions).sum::<u32>().max(1) as f64;
            let dimmings = spent.iter().map(|s| s.dimmings).sum::<u32>();
            let minutes = sum(&|r| r.fight as i64).max(1) as f64 / 3600.0;
            let uses = sum(&|r| r.uses as i64);
            println!(
                "| {case} | {player} | {level} | {}–{} | {:.0} / {:.0} | {:.1} | {} | {:.0} | {} / {} | {} / {} | {:.1} / {:.0} | {}, {:.1} / {:.0} | {} |",
                results.iter().filter(|r| r.won == Some(1)).count(),
                results.iter().filter(|r| r.won == Some(0)).count(),
                sum(&|r| r.lost[0]) as f64 / minutes,
                sum(&|r| r.lost[1]) as f64 / minutes,
                uses as f64 / minutes,
                if uses == 0 { "–".to_string() } else { format!("{:.0}%", 100.0 * sum(&|r| r.hits as i64) as f64 / uses as f64) },
                if uses == 0 { 0.0 } else { sum(&|r| r.damage) as f64 / uses as f64 },
                sum(&|r| r.counters as i64),
                sum(&|r| r.cut as i64),
                sum(&|r| r.exchange[0]),
                sum(&|r| r.exchange[1]),
                spent.iter().map(|s| s.time).sum::<Duration>().as_secs_f64() * 1000.0 / decisions,
                spent.iter().map(|s| s.longest).max().unwrap_or_default().as_secs_f64() * 1000.0,
                dimmings,
                spent.iter().map(|s| s.dim_time).sum::<Duration>().as_secs_f64() * 1000.0 / dimmings.max(1) as f64,
                spent.iter().map(|s| s.dim_longest).max().unwrap_or_default().as_secs_f64() * 1000.0,
                spent.iter().map(|s| s.late).sum::<u32>(),
            );
        }
    }
}

/// What its plans think a dimming chip is worth: EXE6's `searchmn` and
/// `eraseman` cases ([`dimming_match`]), the fight's first decision (the
/// dummy's charge full, the player standing where it stands), each kind of
/// plan's best worth, in HP over the guesses' average (Hard's), and
/// supposing the player stands (one guess); and what the fighter makes in
/// five seconds from there, the player standing still. The player stands
/// on its area's middle panel, or a step up from it: CircGun's sight
/// circles the area's edge, so a player in the middle can't be hit. A
/// measurement: `cargo test -p nettai-training --release --lib
/// dimming_worths -- --ignored --nocapture` (`FIGHTER_SEEDS` (3)).
#[test]
#[ignore]
fn dimming_worths() {
    nettai_frontend::session::quiet_engine_panics();
    let seeds: u32 = std::env::var("FIGHTER_SEEDS").unwrap_or_else(|_| "3".into()).parse().unwrap();
    let content = content("exe6");
    let mut search = Search::with(4);
    println!("| case | seed | player at | stand | shot | chip | stand, shot, chip (it stands) | played out (dealt / lost) |");
    println!("|---|---|---|---|---|---|---|---|");
    for case in ["searchmn", "eraseman"] {
        for (seed, moved) in (0..seeds).flat_map(|s| [(s, 0), (s, keys::UP)]) {
            let m = dimming_match(&content, "exe6", case, seed);
            let b = Set::of(&content, &m, seed).start();
            let mut b = to_the_fight(&b, 1, &|_| Some(0), &mut 0).expect("the fight starts");
            // (The charge full; the player a step up, or not.)
            for t in 0..200 {
                step(&mut b, 1, keys::B, if t < 2 { moved } else { 0 });
            }
            let at = navi_at(&b, 1).unwrap();
            let ask = |search: &mut Search, level: Level| {
                let q = Question { id: 1, battle: Box::new(b.clone()), now: Exec::stand(at), theirs: 0, past: Past([0; PAST]), stale: 0, side: 1, level, asked: Asked::Decide };
                search.answer(&q)
            };
            let a = ask(&mut search, Level::HARD);
            let per = |a: &Answer, then: Then| a.tried[..a.tried.len() - 1].iter().filter(|(e, _)| e.plan.then == then).map(|(_, w)| *w).max().map_or("–".into(), |w| format!("{:.0}", w as f64 / a.guesses as f64 / DEALT as f64));
            let one = ask(&mut search, Level { guesses: 1, ..Level::HARD });
            let played = {
                let mut trial = b.clone();
                let mut fighter = Fighter::inline(Level::HARD, seed as u64);
                for _ in 0..300 {
                    let mine = fighter.buttons(&trial, 1, 0) & !(keys::L | keys::R);
                    step(&mut trial, 1, mine, 0);
                    if !fight_goes_on(&trial) {
                        break;
                    }
                }
                let (d, l) = outcome(&b, &trial, 1);
                format!("{:.0} / {:.0}", d as f64 / DEALT as f64, l as f64 / TAKEN as f64)
            };
            let them = navi_at(&b, 0).map_or("–".into(), |p| format!("{},{}", p.x, p.y));
            println!(
                "| {case} | {seed} | {them} | {} | {} | {} | {}, {}, {} | {played} |",
                per(&a, Then::Hold),
                per(&a, Then::Shot),
                per(&a, Then::Chip),
                per(&one, Then::Hold),
                per(&one, Then::Shot),
                per(&one, Then::Chip)
            );
        }
    }
}

/// What its questions cost with dimming chips in play: EXE6's dimming
/// cases ([`dimming_match`]), a custom screen's chips and a decision in
/// the fight that follows. A measurement: `cargo test -p nettai-training
/// --release --lib dimming_costs -- --ignored --nocapture`
/// (`FIGHTER_SEEDS` (3), `FIGHTER_AIM`: the aims' stride and window in
/// plans' copies).
#[test]
#[ignore]
fn dimming_costs() {
    nettai_frontend::session::quiet_engine_panics();
    let seeds: u32 = std::env::var("FIGHTER_SEEDS").unwrap_or_else(|_| "3".into()).parse().unwrap();
    let content = content("exe6");
    let mut search = Search::with(4);
    println!("| case | seed | chips: ms, trial ticks, aims | decision: ms, trial ticks, aims |");
    println!("|---|---|---|---|");
    for case in ["eraseman", "searchmn", "cut-in"] {
        for seed in 0..seeds {
            let m = dimming_match(&content, "exe6", case, seed);
            let mut b = Set::of(&content, &m, seed).start();
            let mut t = 0;
            while !b.custom.sides[1].screen.is_some_and(|s| s.phase == Phase::Choosing && b.round.mode == mode::CUSTOM) && t < 900 {
                step(&mut b, 1, 0, 0);
                t += 1;
            }
            let chips: Vec<(u8, FolderChip)> = {
                let s = &b.custom.sides[1];
                let screen = s.screen.unwrap();
                screen
                    .slots
                    .iter()
                    .enumerate()
                    .filter_map(|(at, slot)| match slot.kind {
                        SlotKind::Chip { index, .. } => s.folder.chips.get(index as usize).copied().flatten().map(|chip| (at as u8, chip)),
                        _ => None,
                    })
                    .collect()
            };
            let ask = |search: &mut Search, b: &Battle, asked: Asked| {
                let at = navi_at(b, 1).unwrap_or(NOWHERE);
                let q = Question { id: 1, battle: Box::new(b.clone()), now: Exec::stand(at), theirs: 0, past: Past([0; PAST]), stale: 0, side: 1, level: Level::HARD, asked };
                let aimed = || (dimmed::AIMED[0].load(std::sync::atomic::Ordering::Relaxed), dimmed::AIMED[1].load(std::sync::atomic::Ordering::Relaxed), dimmed::AIM_TICKS.load(std::sync::atomic::Ordering::Relaxed));
                let before = aimed();
                let a = search.answer(&q);
                let after = aimed();
                format!("{:.0}, {}, {} + {} ({} ticks)", a.took.as_secs_f64() * 1000.0, a.ticks, after.0 - before.0, after.1 - before.1, after.2 - before.2)
            };
            let on_screen = ask(&mut search, &b, Asked::Chips(chips));
            let mut f = to_the_fight(&b, 1, &|_| Some(0), &mut 0).expect("the fight starts");
            for _ in 0..200 {
                step(&mut f, 1, keys::B, 0);
            }
            let fight = ask(&mut search, &f, Asked::Decide);
            println!("| {case} | {seed} | {on_screen} | {fight} |");
        }
    }
}

/// A chip that waits for its user's A while dimmed is aimed: the dummy's
/// EraseMan (EXE6), its dimming begun with the player standing on one row
/// or another, is decided a press (a reaction time on at the earliest)
/// that takes away as much as any press could (each tried), more than
/// some (his aim turns: where the press is matters).
#[test]
fn a_chip_that_waits_for_a_is_aimed() {
    nettai_frontend::session::quiet_engine_panics();
    let content = content("exe6");
    let level = Level::HARD;
    let mut mattered = false;
    for (seed, step_to) in [(1, 0), (1, keys::UP), (1, keys::DOWN)] {
        let m = dimming_match(&content, "exe6", "eraseman", seed);
        let b = Set::of(&content, &m, seed).start();
        let mut b = to_the_fight(&b, 1, &|_| Some(0), &mut 0).expect("the fight starts");
        assert!(b.hands[1].remaining() > 0, "EraseMan in hand");
        for t in 0..20 {
            step(&mut b, 1, 0, if t < 2 { step_to } else { 0 });
        }
        step(&mut b, 1, keys::A, 0);
        let mut t = 0;
        while !b.is_dimmed() && t < 60 {
            step(&mut b, 1, 0, 0);
            t += 1;
        }
        assert!(b.is_dimmed(), "EraseMan's dimming");
        let q = Question { id: 1, battle: Box::new(b.clone()), now: Exec::stand(navi_at(&b, 1).unwrap()), theirs: 0, past: Past([0; PAST]), stale: 0, side: 1, level, asked: Asked::Dimmed };
        let answer = Search::default().answer(&q).dimmed.expect("a dimming's answer");
        let press = answer.press.expect("a press");
        assert!(press >= level.reaction, "{press}");
        let play = |press: u32| {
            let mut c = b.clone();
            let (mut k, mut after) = (0, 0);
            while after < 20 && k < 900 {
                step(&mut c, 1, if press == k { keys::A | keys::B } else { keys::B }, 0);
                k += 1;
                after = if c.is_dimmed() { 0 } else { after + 1 };
            }
            hp(&b, 0) - hp(&c, 0)
        };
        let dealt = play(press);
        let each: Vec<i64> = (level.reaction..level.reaction + 240).step_by(2).map(play).collect();
        let (most, least) = (*each.iter().max().unwrap(), *each.iter().min().unwrap());
        assert_eq!(dealt, most, "pressed on {press}, the player at {:?}", navi_at(&b, 0));
        mattered |= least < most;
    }
    assert!(mattered, "where the press is never mattered");
}

/// It cuts in where that pays: the player's CircGun (EXE6) used against
/// the dummy holding EraseMan, a dimming chip, next in hand (cut-ins are a
/// link battle's: a match's stages are), the dummy decides to press A
/// while CircGun's name shows, and its EraseMan cuts in.
#[test]
fn it_cuts_in_where_that_pays() {
    nettai_frontend::session::quiet_engine_panics();
    let content = content("exe6");
    let m = dimming_match(&content, "exe6", "cut-in", 1);
    let b = Set::of(&content, &m, 1).start();
    assert_ne!(b.setup.settings.effects & nettai_battle::setup::effects::LINK, 0, "a link battle");
    let mut b = to_the_fight(&b, 1, &|_| Some(0), &mut 0).expect("the fight starts");
    assert!(b.hands[0].remaining() > 0 && b.hands[1].remaining() > 0, "chips in hand");
    step(&mut b, 1, 0, keys::A);
    let mut t = 0;
    while !b.is_dimmed() && t < 60 {
        step(&mut b, 1, 0, 0);
        t += 1;
    }
    assert!(b.is_dimmed() && b.dimming[0].initiator == 0, "CircGun's dimming");
    let level = Level::HARD;
    let q = Question { id: 1, battle: Box::new(b.clone()), now: Exec::stand(navi_at(&b, 1).unwrap()), theirs: 0, past: Past([0; PAST]), stale: 0, side: 1, level, asked: Asked::Dimmed };
    let answer = Search::default().answer(&q).dimmed.expect("a dimming's answer");
    let press = answer.press.expect("a cut-in");
    let hand = b.hands[1].remaining();
    let mut c = b.clone();
    for k in 0..=press + 2 {
        step(&mut c, 1, if k == press { keys::A | keys::B } else { keys::B }, 0);
    }
    assert_eq!(c.hands[1].remaining(), hand - 1, "pressed on {press}: no cut-in");
}
