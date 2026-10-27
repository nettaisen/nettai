//! What hits the learned AI in the rounds it loses to the masher: a
//! diagnostic (docs/design/training-ai.md §9), not part of training.
//!
//! It plays EXE6 rounds of the masher (side 0: the env's `Bot`, its custom
//! screen the env's scripted pointer) against `NetAi` at Normal (side 1) on
//! the specialist run's four fixed builds (the env's `fixed_builds` 4,
//! `full_lists` 0.5, `randomize` 0.75, `combos` 0.3, no holdout, HP 1000),
//! each round's matchup drawn uniformly of the 4×4, stepped as the env
//! steps a round (`env::step_battle`). No practice options: the battle is
//! moved by its input alone, so every round plays back as a replay.
//!
//! For each tick the AI's navi loses HP it logs what hit it, and every
//! object of the masher's side the tick before, each with its kind, its
//! age (ticks since it appeared), where it is from the AI's panel, whether
//! side 1's console shows it (`Battle::visible_to`; seen: or it blinks
//! there, as the view reads it; encoded: one of the 48 object tokens), and
//! the margin: ticks from the first AI decision whose view held it (a view
//! of `D` ticks before the decision, decisions every `k` ticks) to the hit.
//! With them the AI's buttons over the 30 ticks before, whether its navi
//! moved, the navis' actions, the AI's panel's type.
//!
//! What hit it: an attack hits a navi as the later of the two updates (the
//! update order) unregisters its collision, and the navi takes the damage
//! in its own update. So an attack updated after the navi leaves its
//! collision in the navi's accumulators (`hit_by`) after the tick before
//! the drop (`acc`; the attack may be gone with its hit by then: read as it
//! was two ticks before, else one); one updated before it hits and does
//! its damage in one tick, and is still registered on the navi's panel
//! after it (`mask-0`); else what of the masher's attacks was registered
//! there one or two ticks before (`mask-1`, `mask-2`). A drop with none is
//! put down to the panel (a poison panel's drain) or the masher's chip.
//! `LOST_DEBUG=round:tick` prints the objects around one tick.
//!
//! Writes to OUT: `hits.tsv` (a row a hit), `summary.txt`, each lost round
//! as `replays/round_NNNN.nettaireplay` (recorded from side 1), and for the
//! first `--render` lost rounds `round_NNNN/frames.txt`: the replay, and
//! the frames nettai-tool renders of it (each of the 15 HP drops that cost
//! the most and 20 ticks before it; a frame is the picture after that many
//! ticks), e.g. `nettai-tool --replay R --headless F --side both --png-scale 2`.
//!
//! `NETTAI_PACKS=<packs> cargo run --release -p nettai-training --features live --example lost_rounds -- <weights> <OUT> [--rounds N] [--seed S] [--render N] [--threads T] [--cache DIR]`

use nettai_battle::Battle;
use nettai_battle::battle::{fight, mode};
use nettai_battle::collision::{ATTACK_OF, CollisionId, f1};
use nettai_battle::input::keys;
use nettai_battle::kinds::player::NaviAction;
use nettai_content_api::{KindHandle, ObjectRef, PanelPos, Pool};
use nettai_frontend::driver::Ai;
use nettai_match::{Picks, Set};
use nettai_netplay::rng::SplitMix64;
use nettai_training::act::{self, Navigator};
use nettai_training::ai::{Fingerprinter, Level, NetAi};
use nettai_training::bots::{Bot, BotKind};
use nettai_training::builds::{Build, Combos, Holdout};
use nettai_training::env::{self, Game};
use nettai_training::fingerprint::Fingerprints;
use nettai_training::net::Net;
use nettai_training::spec;
use nettai_training::stats::navi_hp;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// The env's first fixed build's seed (`env::FIXED_BUILDS_SEED`): build `i`
/// is made from this plus `i`.
const FIXED_BUILDS_SEED: u32 = 0x5EED_B11D;
const FIXED_BUILDS: u32 = 4;
/// The env's defaults: a fight this long is a stall; a custom screen open
/// this long cuts the round.
const MAX_FIGHT_TICKS: u32 = 18000;
const MAX_SCREEN_TICKS: u32 = 3600;
/// The AI's buttons kept before a hit, and the attack log's reach.
const BEFORE: usize = 30;
const WARN: usize = 60;
/// HP drops rendered a lost round, and the ticks before each.
const DROPS_RENDERED: usize = 15;
const LEAD: u32 = 20;
const TICKS_PER_MINUTE: f64 = 3600.0;

struct Args {
    weights: PathBuf,
    out: PathBuf,
    rounds: usize,
    seed: u64,
    render: usize,
    threads: usize,
    cache: PathBuf,
}

fn args() -> Args {
    let mut it = std::env::args().skip(1);
    let usage = "usage: lost_rounds <weights> <OUT> [--rounds N] [--seed S] [--render N] [--threads T] [--cache DIR]";
    let weights = PathBuf::from(it.next().expect(usage));
    let out = PathBuf::from(it.next().expect(usage));
    let mut a = Args {
        cache: out.join("fingerprints"),
        weights,
        out,
        rounds: 300,
        seed: 1,
        render: 12,
        threads: std::thread::available_parallelism().map_or(4, |n| n.get()),
    };
    while let Some(flag) = it.next() {
        let value = it.next().expect(usage);
        match flag.as_str() {
            "--rounds" => a.rounds = value.parse().expect(usage),
            "--seed" => a.seed = value.parse().expect(usage),
            "--render" => a.render = value.parse().expect(usage),
            "--threads" => a.threads = value.parse().expect(usage),
            "--cache" => a.cache = PathBuf::from(value),
            _ => panic!("{usage}"),
        }
    }
    a
}

/// What the run shares: the game, its fixed builds, the network.
struct Shared {
    content: Arc<nettai_battle::Content>,
    game: Game,
    fingerprints: Arc<Fingerprints>,
    builds: Vec<Build>,
    navis: Vec<String>,
    net: Arc<Net>,
    seed: u64,
}

/// An object as it stood at a tick, from the AI's side.
#[derive(Clone, Debug)]
struct Seen {
    kind: String,
    pool: Pool,
    age: u32,
    dx: i8,
    dy: i8,
    /// Side 1's console shows it.
    visible: bool,
    /// The AI's view holds it (visible, or blinking on a side it isn't
    /// blind to: `observe`'s `seen`).
    seen: bool,
    /// One of the 48 object tokens.
    encoded: bool,
    /// Ticks from the first AI decision whose view held it (encoded) to the
    /// hit; none: no decision before the hit saw it.
    margin: Option<i64>,
}

impl Seen {
    fn text(&self) -> String {
        let p = match self.pool {
            Pool::Actor => 'a',
            Pool::Attack => 't',
            Pool::Effect => 'e',
        };
        let margin = self.margin.map_or("-".to_string(), |m| m.to_string());
        format!("{}[{p}]@{}:{:+},{:+}:v{}s{}e{}:m{margin}", self.kind, self.age, self.dx, self.dy, self.visible as u8, self.seen as u8, self.encoded as u8)
    }
}

#[derive(Clone, Debug)]
struct Hit {
    tick: u32,
    fight_tick: u32,
    damage: u16,
    hp_after: u16,
    mode: String,
    ai_panel: (u8, u8),
    masher_panel: Option<(u8, u8)>,
    /// The type of the AI navi's panel.
    panel_type: String,
    /// The navis' actions the tick before.
    ai_action: String,
    masher_action: String,
    /// Where the hitters were read: the navi's accumulators the tick before
    /// (`acc`), else the masher's attacks registered on its panel after the
    /// tick, or one or two ticks before (`mask-0`, `mask-1`, `mask-2`), or
    /// none found.
    source: &'static str,
    /// The collisions the accumulators the tick before held.
    acc_bits: u32,
    hitters: Vec<Seen>,
    /// The masher's side's objects the tick before (its navi apart).
    near: Vec<Seen>,
    buttons: String,
    pressed_direction: bool,
    moved: bool,
    /// Ticks of the [`BEFORE`] before the hit the AI's navi was in the
    /// masher's navi's row.
    same_row: u32,
    /// Ticks since the AI's panel was first marked attacked by the masher's
    /// side, within the last [`WARN`].
    warned: Option<u32>,
    masher_chip: Option<(String, u32)>,
}

struct RoundOut {
    n: usize,
    matchup: (usize, usize),
    /// By the AI: +1 won, −1 lost, 0 neither.
    result: i8,
    ending: String,
    ticks: u32,
    fight_ticks: u32,
    hp_end: [Option<u16>; 2],
    /// Each side's navi's max HP, and the HP it lost in all (by side).
    max_hp: [u16; 2],
    lost_hp: [u32; 2],
    hits: Vec<Hit>,
    replay: Vec<u8>,
    replay_ended: bool,
    /// Each side's attack chips used in the fight, and those that landed
    /// (the other navi losing HP within `stats::LANDS` ticks), by side.
    attack_chips: [u32; 2],
    landed: [u32; 2],
}

/// An object followed from the tick it appeared.
struct Tracked {
    kind: KindHandle,
    born: u32,
    /// The first AI decision whose view held it.
    decided: Option<u32>,
}

/// An object as it stood after a tick.
struct Snap {
    r: ObjectRef,
    slot: Option<u8>,
    kind: KindHandle,
    pool: Pool,
    alliance: u8,
    born: u32,
    decided: Option<u32>,
    panel: PanelPos,
    visible: bool,
    seen: bool,
    encoded: bool,
    /// Its collision is an attack of side 0's.
    attack0: bool,
}

impl Snap {
    /// It from the AI's panel `me`, at the hit on tick `now`.
    fn seen(&self, b: &Battle, me: PanelPos, now: u32) -> Seen {
        Seen {
            kind: b.content.defs.kind(self.kind).key.clone(),
            pool: self.pool,
            age: now.saturating_sub(self.born),
            dx: self.panel.x as i8 - me.x as i8,
            dy: self.panel.y as i8 - me.y as i8,
            visible: self.visible,
            seen: self.seen,
            encoded: self.encoded,
            margin: self.decided.filter(|&dt| dt <= now).map(|dt| now as i64 - dt as i64),
        }
    }
}

/// The battle after a tick, as far as a hit on the next ticks reads it.
struct State {
    objects: Vec<Snap>,
    ai_panel: Option<PanelPos>,
    /// The AI navi's accumulators' hitters, and the collisions registered
    /// on its panel.
    hit_by: u32,
    ai_mask: u32,
    masher_navi: Option<ObjectRef>,
    /// Each collision slot's parent.
    parents: [Option<ObjectRef>; 32],
    ai_action: Option<NaviAction>,
    masher_action: Option<NaviAction>,
}

impl State {
    fn of(b: &Battle, tracked: &HashMap<ObjectRef, Tracked>, view: &HashMap<ObjectRef, bool>) -> State {
        let objects = b
            .objects
            .in_order()
            .filter(|&r| b.objects.is_allocated(r))
            .map(|r| {
                let o = b.objects.get(r);
                let tr = tracked.get(&r);
                Snap {
                    r,
                    slot: o.collision.map(|c| c.0),
                    kind: o.kind,
                    pool: r.pool,
                    alliance: o.alliance,
                    born: tr.map_or(0, |tr| tr.born),
                    decided: tr.and_then(|tr| tr.decided),
                    panel: o.panel,
                    visible: b.visible_to(r, 1),
                    seen: view.contains_key(&r),
                    encoded: view.get(&r) == Some(&true),
                    attack0: o.collision.is_some_and(|c| b.collision.get(c).self_flags & ATTACK_OF[0] != 0),
                }
            })
            .collect();
        let ai = b.player(1).filter(|&r| b.objects.is_allocated(r));
        let ai_panel = ai.map(|r| b.objects.get(r).panel);
        let action = |side: u8| b.player(side).filter(|&r| b.objects.is_allocated(r)).and_then(|r| b.objects.get(r).actor).map(|a| b.actors.get(a).navi_action);
        State {
            objects,
            ai_panel,
            hit_by: ai.and_then(|r| b.objects.get(r).collision).map_or(0, |c| b.collision.get(c).acc.hit_by),
            ai_mask: ai_panel.map_or(0, |p| b.collision.masks[(p.y * 8 + p.x) as usize]),
            masher_navi: b.player(0),
            parents: std::array::from_fn(|s| b.collision.get(CollisionId(s as u8)).parent),
            ai_action: action(1),
            masher_action: action(0),
        }
    }

    /// The object of collision slot `slot`: its owner, else the
    /// collision's parent.
    fn by_slot(&self, slot: u8) -> Option<&Snap> {
        self.objects.iter().find(|o| o.slot == Some(slot)).or_else(|| self.parents[slot as usize].and_then(|p| self.objects.iter().find(|o| o.r == p)))
    }
}

fn panel(b: &Battle, side: u8) -> Option<PanelPos> {
    b.player(side).filter(|&r| b.objects.is_allocated(r)).map(|r| b.objects.get(r).panel)
}

/// Whether `viewer`'s view holds `r` (observe's `sees_object`).
fn sees_object(b: &Battle, viewer: u8, r: ObjectRef) -> bool {
    if b.visible_to(r, viewer) {
        return true;
    }
    let o = b.objects.get(r);
    let blinks = o.collision.is_some_and(|c| {
        let f = b.collision.get(c).f1;
        f & (f1::FLASHING | f1::INVISIBLE) != 0 && f & f1::DEAD == 0
    });
    blinks && b.sees(viewer, o.alliance)
}

/// The objects side 1's view holds, and whether each is one of the tokens
/// (actors, then attacks, then effects, as many as the set holds).
fn view_of(b: &Battle) -> HashMap<ObjectRef, bool> {
    let cap = spec::SETS[spec::OBJECTS].count;
    let mut out = HashMap::new();
    let mut n = 0;
    for pool in [Pool::Actor, Pool::Attack, Pool::Effect] {
        for r in b.objects.in_order().filter(|r| r.pool == pool) {
            if b.objects.is_allocated(r) && sees_object(b, 1, r) {
                out.insert(r, n < cap);
                n += 1;
            }
        }
    }
    out
}

fn action_name(b: &Battle, a: NaviAction) -> String {
    match a {
        NaviAction::Content(h) => b.content.defs.action(h).key.clone(),
        NaviAction::Engine(e) => format!("{e:?}"),
        other => format!("{other:?}"),
    }
}

fn mode_name(b: &Battle) -> String {
    match b.round.mode {
        mode::CUSTOM => "custom".into(),
        mode::FIGHTING if b.fight.state == fight::FIGHTING && b.is_dimmed() => "dimmed".into(),
        mode::FIGHTING if b.fight.state == fight::FIGHTING => "fight".into(),
        mode::FIGHTING => format!("fight-state-{:02x}", b.fight.state),
        m => format!("mode-{m:02x}"),
    }
}

fn buttons_text(held: &VecDeque<u16>) -> String {
    let one = |h: u16| {
        let mut s = String::new();
        for (bit, c) in [(keys::UP, 'U'), (keys::DOWN, 'D'), (keys::LEFT, 'L'), (keys::RIGHT, 'R'), (keys::A, 'A'), (keys::B, 'B')] {
            if h & bit != 0 {
                s.push(c);
            }
        }
        if s.is_empty() { "-".into() } else { s }
    };
    // (Runs: "12x-,8xRB".)
    let mut out: Vec<String> = Vec::new();
    let mut run: Option<(String, u32)> = None;
    for &h in held {
        let s = one(h);
        match &mut run {
            Some((last, n)) if *last == s => *n += 1,
            _ => {
                if let Some((last, n)) = run.take() {
                    out.push(format!("{n}x{last}"));
                }
                run = Some((s, 1));
            }
        }
    }
    if let Some((last, n)) = run {
        out.push(format!("{n}x{last}"));
    }
    out.join(",")
}

/// One round: the matchup and seeds from its number.
fn play_round(s: &Shared, n: usize) -> RoundOut {
    let content = &s.content;
    // (`LOST_DEBUG=round:tick`: the objects around one hit, on stderr.)
    let debug: Option<(usize, u32)> = std::env::var("LOST_DEBUG").ok().and_then(|v| {
        let (a, b) = v.split_once(':')?;
        Some((a.parse().ok()?, b.parse().ok()?))
    });
    let mut rng = SplitMix64::new(s.seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (n as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9));
    let (i, j) = (rng.below(FIXED_BUILDS as u64) as usize, rng.below(FIXED_BUILDS as u64) as usize);
    let seed = rng.next_u64() as u32;
    let m = s.game.maker.round_with([s.builds[i].side.clone(), s.builds[j].side.clone()], Some(1000), seed).expect("a round of the fixed builds");
    let set = Set::of(content, &m, seed);
    let mut b = set.start();
    let mut ai = NetAi::of(s.net.clone(), Level::Normal, rng.next_u64(), Fingerprinter::made(s.fingerprints.clone())).expect("the AI");
    let (d, k) = (ai.knobs().delay as u32, s.net.k());
    // (The masher, else the scripted player LOST_BOT names.)
    let kind = std::env::var("LOST_BOT").ok().and_then(|n| BotKind::named(&n)).unwrap_or(BotKind::Masher);
    let mut bot = Bot::new(kind, rng.next_u64());
    let mut nav = Navigator::default();

    // The replay, written as it is played.
    let head = nettai_replay::Head { engine: nettai_frontend::replay::ENGINE.to_string(), game: content.game().to_string(), content: content.hash().0, start: b.digest() };
    let info = nettai_replay::Info { names: ["masher".into(), "ai-normal".into()], ..nettai_replay::Info::of(nettai_replay::MatchKind::Offline, 1) };
    let bytes = nettai_match::binary::match_bytes(content, &m).expect("the match's bytes");
    let mut writer = nettai_replay::Writer::new(Vec::new(), &head, &bytes, &info).expect("a replay");

    let mut out = RoundOut {
        n,
        matchup: (i, j),
        result: 0,
        ending: String::new(),
        ticks: 0,
        fight_ticks: 0,
        hp_end: [None; 2],
        max_hp: [0; 2],
        lost_hp: [0; 2],
        hits: Vec::new(),
        replay: Vec::new(),
        replay_ended: false,
        attack_chips: [0; 2],
        landed: [0; 2],
    };
    let mut measure = nettai_training::stats::Measure::new(Default::default());
    let mut t: u32 = 0;
    let mut in_screen = false;
    let mut screen_ticks = 0;
    let mut hp: [Option<u16>; 2] = [None; 2];
    let mut tracked: HashMap<ObjectRef, Tracked> = HashMap::new();
    let mut held_ai: VecDeque<u16> = VecDeque::new();
    // The states after the last two ticks, the newer last.
    let mut states: VecDeque<State> = VecDeque::new();
    // The navis' panels after the last ticks, and the panels the masher's
    // side attacked.
    let mut panels: VecDeque<(Option<PanelPos>, Option<PanelPos>)> = VecDeque::new();
    let mut attacked: VecDeque<u64> = VecDeque::new();
    let mut masher_hand = 0u8;
    let mut masher_chip: Option<(String, u32)> = None;

    let result = loop {
        if writer.ended() {
            break (0, "set ended".to_string());
        }
        // (The env's `over`.)
        if let Some(nettai_battle::RoundEnd::Error(why)) = b.round_end() {
            break (0, format!("error: {why}"));
        }
        if b.round.mode == mode::FIGHTING && matches!(b.fight.state, fight::WIN | fight::LOSE | fight::DRAW) {
            let local_won = match b.fight.state {
                fight::WIN => 1,
                fight::LOSE => -1,
                _ => 0,
            };
            // (The simulation's own side is 0: the masher's.)
            let by = if b.round.time_up != 0 { "judge" } else { "ko" };
            break (-local_won * if b.round.local_side & 1 == 0 { 1 } else { -1 }, by.to_string());
        }
        if out.fight_ticks > MAX_FIGHT_TICKS {
            break (0, "limit".into());
        }
        if screen_ticks > MAX_SCREEN_TICKS || t > 4 * MAX_FIGHT_TICKS + 4 * MAX_SCREEN_TICKS {
            break (0, "cut".into());
        }

        // This tick's buttons (the env's `tick`; the AI's as live play's).
        let custom = b.round.mode == mode::CUSTOM;
        if custom && !in_screen {
            nav.reset();
            screen_ticks = 0;
        }
        screen_ticks += custom as u32;
        let masher = if custom {
            if nav.idle() {
                let p = act::scripted_pointer(&b, 0, &nav);
                nav.point(p);
            }
            nav.buttons(&b, 0)
        } else if env::fight_runs(&b) {
            bot.fight(&b, 0)
        } else {
            0
        };
        let mine = if custom { ai.custom_screen(&b, 1) } else { ai.fight(&b, 1, 0) };
        in_screen = custom;
        let went_on = env::step_battle(&mut b, [masher, mine]);
        let index = t as u64;
        t += 1;
        measure.tick(&b, t);
        let ended = matches!(b.round_end(), Some(nettai_battle::RoundEnd::NextRound { .. } | nettai_battle::RoundEnd::Over(_)));
        let tick = nettai_replay::Tick {
            inputs: [masher as u32, mine as u32],
            digest: (nettai_frontend::replay::digest_due(index) || ended).then(|| b.digest()),
            round_ended: ended,
            set_ended: matches!(b.round_end(), Some(nettai_battle::RoundEnd::Over(_))),
        };
        writer.tick(&tick).expect("a replay tick");
        if !went_on {
            break (0, format!("stopped: {:?}", b.round_end()));
        }
        if b.round.mode == mode::FIGHTING {
            out.fight_ticks += 1;
        }

        // The objects, followed.
        let mut alive: Vec<ObjectRef> = Vec::new();
        for r in b.objects.in_order() {
            if !b.objects.is_allocated(r) {
                continue;
            }
            alive.push(r);
            let kind = b.objects.get(r).kind;
            match tracked.get(&r) {
                Some(tr) if tr.kind == kind => {}
                _ => {
                    tracked.insert(r, Tracked { kind, born: t, decided: None });
                }
            }
        }
        tracked.retain(|r, _| alive.contains(r));
        let view = view_of(&b);
        // (A view the AI encodes now is decided on `d` ticks later.)
        if (t + d).is_multiple_of(k) {
            for (r, tr) in tracked.iter_mut() {
                if tr.decided.is_none() && view.get(r) == Some(&true) {
                    tr.decided = Some(t + d);
                }
            }
        }

        // The masher's chips used.
        let hand = b.hands[0].remaining();
        if b.round.mode == mode::FIGHTING && hand < masher_hand {
            let used = b.hands[0].ids[b.hands[0].cursor.saturating_sub(1) as usize];
            masher_chip = Some((used.map_or("?".to_string(), |c| b.content.defs.chip(c).key.clone()), t));
        }
        masher_hand = hand;

        // The AI navi's HP, and a hit: read of the states before it.
        let now = navi_hp(&b, 1, hp[1]);
        held_ai.push_back(mine);
        if held_ai.len() > BEFORE + 1 {
            held_ai.pop_front();
        }
        let current = State::of(&b, &tracked, &view);
        if let (Some(was), Some(now)) = (hp[1], now)
            && now < was
            && let Some(prev) = states.back()
        {
            let older = (states.len() == 2).then(|| &states[0]);
            let me = prev.ai_panel.unwrap_or_default();
            let seen = |o: &Snap| o.seen(&b, me, t);
            // The navi's accumulators after the tick before hold what hit it
            // there, after the navi's update (its next update takes the
            // damage): each hitter by its collision slot, as it was two ticks
            // before (it may have gone with its hit), else one. Else what of
            // the masher's side's attacks is registered on the navi's panel
            // after this tick (an attack updated before the navi that moved
            // onto its panel: the hit and the damage in one tick), else after
            // the tick before or the one before that.
            let mut hitters: Vec<Seen> = (0..32u8)
                .filter(|&slot| prev.hit_by & (0x8000_0000 >> slot) != 0)
                .filter_map(|slot| older.and_then(|st| st.by_slot(slot)).or_else(|| prev.by_slot(slot)))
                .map(seen)
                .collect();
            let mut source = if hitters.is_empty() { "none" } else { "acc" };
            if hitters.is_empty() {
                for (name, st) in [("mask-0", Some(&current)), ("mask-1", Some(prev)), ("mask-2", older)] {
                    let Some(st) = st else { continue };
                    let found: Vec<Seen> = (0..32u8).filter(|&slot| st.ai_mask & (0x8000_0000 >> slot) != 0).filter_map(|slot| st.by_slot(slot)).filter(|o| o.attack0).map(seen).collect();
                    if !found.is_empty() {
                        hitters = found;
                        source = name;
                        break;
                    }
                }
            }
            let near: Vec<Seen> = prev.objects.iter().filter(|o| o.alliance == 0 && Some(o.r) != prev.masher_navi && (o.pool != Pool::Effect || o.slot.is_some())).map(seen).collect();
            let warned = prev.ai_panel.and_then(|p| {
                let bit = 1u64 << (p.y * 8 + p.x);
                attacked.iter().position(|&a| a & bit != 0).map(|i| (attacked.len() - i) as u32)
            });
            let first = panels.iter().find_map(|p| p.0);
            let moved = first.is_some_and(|f| panels.iter().filter_map(|p| p.0).any(|p| p != f));
            let same_row = panels.iter().skip(panels.len().saturating_sub(BEFORE)).filter(|(a, m)| a.zip(*m).is_some_and(|(a, m)| a.y == m.y)).count() as u32;
            let before: VecDeque<u16> = held_ai.iter().take(held_ai.len().saturating_sub(1)).copied().collect();
            out.hits.push(Hit {
                tick: t,
                fight_tick: out.fight_ticks,
                damage: was - now,
                hp_after: now,
                mode: mode_name(&b),
                ai_panel: (me.x, me.y),
                panel_type: b.game_rules().panels.names.get(b.field.panels[me.y as usize][me.x as usize].kind.0 as usize).cloned().unwrap_or_default(),
                masher_panel: panels.back().and_then(|p| p.1).map(|p| (p.x, p.y)),
                ai_action: prev.ai_action.map_or("-".into(), |a| action_name(&b, a)),
                masher_action: prev.masher_action.map_or("-".into(), |a| action_name(&b, a)),
                source,
                acc_bits: prev.hit_by.count_ones(),
                hitters,
                near,
                buttons: buttons_text(&before),
                pressed_direction: before.iter().any(|&h| h & (keys::UP | keys::DOWN | keys::LEFT | keys::RIGHT) != 0),
                moved,
                same_row,
                warned,
                masher_chip: masher_chip.clone().map(|(c, at)| (c, t - at)),
            });
        }
        let masher_now = navi_hp(&b, 0, hp[0]);
        for (side, (was, now)) in [(hp[0], masher_now), (hp[1], now)].into_iter().enumerate() {
            if let (Some(was), Some(now)) = (was, now) {
                out.lost_hp[side] += was.saturating_sub(now) as u32;
            }
            if let Some(r) = b.player(side as u8).filter(|&r| b.objects.is_allocated(r)) {
                out.max_hp[side] = out.max_hp[side].max(b.objects.get(r).max_hp);
            }
        }
        hp = [masher_now, now];
        if let Some((dn, dt)) = debug
            && dn == n
            && t + 4 >= dt
            && t <= dt + 1
        {
            let ai = b.player(1).filter(|&r| b.objects.is_allocated(r));
            let acc = ai.and_then(|r| b.objects.get(r).collision).map(|c| b.collision.get(c).acc);
            eprintln!("t {t}: ai hp {:?} panel {:?} acc hit_by {:08x} hit_flags {:08x} final_damage {:?}", now, panel(&b, 1), acc.map_or(0, |a| a.hit_by), acc.map_or(0, |a| a.hit_flags), acc.map(|a| a.final_damage));
            for r in b.objects.in_order().filter(|&r| b.objects.is_allocated(r)) {
                let o = b.objects.get(r);
                let c = o.collision.map(|c| (c.0, b.collision.get(c).self_flags, b.collision.get(c).enabled));
                eprintln!("   {:?} {} all {} panel {},{} pos {:x},{:x} coll {:x?} hp {} state {} vis {}", r, b.content.defs.kind(o.kind).key, o.alliance, o.panel.x, o.panel.y, o.pos.x, o.pos.y, c, o.hp, o.state, b.visible_to(r, 1));
            }
            if let Some(p) = panel(&b, 1) {
                eprintln!("   mask at ai panel {:08x}", b.collision.masks[(p.y * 8 + p.x) as usize]);
            }
        }
        states.push_back(current);
        if states.len() > 2 {
            states.pop_front();
        }
        panels.push_back((panel(&b, 1), panel(&b, 0)));
        if panels.len() > BEFORE + 1 {
            panels.pop_front();
        }
        attacked.push_back(b.attacked_panels(0));
        if attacked.len() > WARN {
            attacked.pop_front();
        }
    };
    out.result = result.0 as i8;
    out.ending = result.1;
    out.attack_chips = measure.stats.attack_chips;
    out.landed = measure.stats.attacks_landed;
    out.ticks = t;
    out.hp_end = hp;
    // (The round played out to its end with no one pressing, so the replay
    // holds the whole set.)
    let mut extra = 0;
    while !writer.ended() && b.round_end().is_none() && extra < 3000 {
        let index = t as u64;
        if !env::step_battle(&mut b, [0, 0]) && !matches!(b.round_end(), Some(nettai_battle::RoundEnd::Over(_) | nettai_battle::RoundEnd::NextRound { .. })) {
            break;
        }
        t += 1;
        extra += 1;
        let ended = matches!(b.round_end(), Some(nettai_battle::RoundEnd::NextRound { .. } | nettai_battle::RoundEnd::Over(_)));
        let tick = nettai_replay::Tick {
            inputs: [0, 0],
            digest: (nettai_frontend::replay::digest_due(index) || ended).then(|| b.digest()),
            round_ended: ended,
            set_ended: matches!(b.round_end(), Some(nettai_battle::RoundEnd::Over(_))),
        };
        writer.tick(&tick).expect("a replay tick");
    }
    out.replay_ended = writer.ended();
    out.replay = writer.into_inner();
    out
}

fn median(v: &mut [i64]) -> Option<i64> {
    if v.is_empty() {
        return None;
    }
    v.sort_unstable();
    Some(v[v.len() / 2])
}

fn pct(a: usize, b: usize) -> String {
    if b == 0 { "-".into() } else { format!("{:.1}%", 100.0 * a as f64 / b as f64) }
}

/// A hit's main hitter: the first of them.
fn main_hitter(h: &Hit) -> Option<&Seen> {
    h.hitters.first()
}

/// What a hit is put down to: its hitter's kind; without one, a collision
/// with no object (its accumulators held one) or nothing, by the masher's
/// chip used in the 300 ticks before, or the AI's panel's type where it
/// isn't plain (a poison panel's drain).
fn cause(h: &Hit) -> String {
    if let Some(s) = main_hitter(h) {
        return s.kind.clone();
    }
    let chip = h.masher_chip.as_ref().filter(|(_, age)| *age <= 300).map_or("-", |(c, _)| c.as_str());
    if h.acc_bits > 0 {
        format!("(objectless hit; masher chip {chip})")
    } else if h.panel_type != "normal" {
        format!("(no hitter; panel {})", h.panel_type)
    } else {
        format!("(no hitter; masher chip {chip})")
    }
}

/// Rounds by result: a name, and which.
type Group = (&'static str, fn(&RoundOut) -> bool);

fn summary(s: &Shared, rounds: &[RoundOut], a: &Args, d: u32, k: u32) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "lost_rounds: {} EXE6 rounds, masher (side 0) vs NetAi Normal (side 1); weights {}", rounds.len(), a.weights.display());
    let _ = writeln!(o, "AI: delay D = {d} ticks, decisions every k = {k} ticks; fixed builds (seed 0x5EED_B11D + i, full_lists 0.5, randomize 0.75, combos 0.3, HP 1000):");
    for (i, n) in s.navis.iter().enumerate() {
        let pair = s.builds[i].pair.as_ref().map(|p| format!(" (folder seeded with {} + {})", s.content.defs.chip(p.setup).key, s.content.defs.chip(p.payoff).key)).unwrap_or_default();
        let _ = writeln!(o, "  build {i}: {n}{pair}");
    }
    let _ = writeln!(o, "Hit = a tick on which the AI navi's HP dropped. Hitter = an object whose collision hit the navi (its hit_by on the tick before the drop).");
    let _ = writeln!(o, "Age = ticks since the object appeared. seen = in the AI's view (visible_to side 1, or blinking); margin = ticks from the first AI decision whose (delayed) view held it to the hit.");
    let _ = writeln!(o);

    let (used, landed) = rounds.iter().fold(([0u32; 2], [0u32; 2]), |(u, l), r| ([u[0] + r.attack_chips[0], u[1] + r.attack_chips[1]], [l[0] + r.landed[0], l[1] + r.landed[1]]));
    let _ = writeln!(o, "attack chips used in the fight and landed (the other navi losing HP within {} ticks): side 0 (the scripted player) {} of {} ({}), side 1 (the AI) {} of {} ({})", nettai_training::stats::LANDS, landed[0], used[0], pct(landed[0] as usize, used[0] as usize), landed[1], used[1], pct(landed[1] as usize, used[1] as usize));
    let _ = writeln!(o);
    let groups: [Group; 3] = [("lost", |r| r.result < 0), ("won", |r| r.result > 0), ("neither", |r| r.result == 0)];
    let _ = writeln!(o, "{:<8} {:>6} {:>7} {:>8} {:>6} {:>9} {:>10} {:>9} {:>11}", "rounds", "count", "share", "fight-min", "hits", "hits/min", "dmg/min", "dmg/hit", "dealt/min");
    for (name, f) in &groups {
        let rs: Vec<&RoundOut> = rounds.iter().filter(|r| f(r)).collect();
        let minutes: f64 = rs.iter().map(|r| r.fight_ticks as f64).sum::<f64>() / TICKS_PER_MINUTE;
        let hits: usize = rs.iter().map(|r| r.hits.len()).sum();
        let damage: u64 = rs.iter().flat_map(|r| &r.hits).map(|h| h.damage as u64).sum();
        let dealt: u64 = rs.iter().map(|r| r.lost_hp[0] as u64).sum();
        let _ = writeln!(
            o,
            "{name:<8} {:>6} {:>7} {:>8.1} {:>6} {:>9.1} {:>10.0} {:>9.1} {:>11.0}",
            rs.len(),
            pct(rs.len(), rounds.len()),
            minutes,
            hits,
            hits as f64 / minutes.max(1e-9),
            damage as f64 / minutes.max(1e-9),
            damage as f64 / hits.max(1) as f64,
            dealt as f64 / minutes.max(1e-9)
        );
    }
    let mut endings: BTreeMap<String, usize> = BTreeMap::new();
    for r in rounds {
        *endings.entry(format!("{} {}", if r.result < 0 { "lost" } else if r.result > 0 { "won" } else { "neither" }, r.ending)).or_default() += 1;
    }
    let _ = writeln!(o, "endings: {}", endings.iter().map(|(e, n)| format!("{e} {n}")).collect::<Vec<_>>().join(", "));
    let _ = writeln!(o, "(dmg/min: HP the AI lost a minute of fight; dealt/min: HP the masher lost.)");
    let _ = writeln!(o);
    let _ = writeln!(o, "AI losses by matchup (masher's build × AI's build): lost/rounds");
    let _ = write!(o, "{:>14}", "masher \\ AI");
    for (j, navi) in s.navis.iter().enumerate() {
        let _ = write!(o, " {:>16}", format!("{j}:{navi}"));
    }
    let _ = writeln!(o);
    for (i, navi) in s.navis.iter().enumerate() {
        let _ = write!(o, "{:>14}", format!("{i}:{navi}"));
        for j in 0..FIXED_BUILDS as usize {
            let rs: Vec<&RoundOut> = rounds.iter().filter(|r| r.matchup == (i, j)).collect();
            let lost = rs.iter().filter(|r| r.result < 0).count();
            let _ = write!(o, " {:>16}", format!("{lost}/{}", rs.len()));
        }
        let _ = writeln!(o);
    }
    let _ = writeln!(o);
    let _ = writeln!(o, "by matchup: AI's max HP / masher's (median), HP a fight minute the AI lost / dealt, the AI's fight minutes a round");
    for i in 0..FIXED_BUILDS as usize {
        for j in 0..FIXED_BUILDS as usize {
            let rs: Vec<&RoundOut> = rounds.iter().filter(|r| r.matchup == (i, j)).collect();
            if rs.is_empty() {
                continue;
            }
            let mut ai_hp: Vec<i64> = rs.iter().map(|r| r.max_hp[1] as i64).collect();
            let mut masher_hp: Vec<i64> = rs.iter().map(|r| r.max_hp[0] as i64).collect();
            let minutes: f64 = rs.iter().map(|r| r.fight_ticks as f64).sum::<f64>() / TICKS_PER_MINUTE;
            let taken: f64 = rs.iter().map(|r| r.lost_hp[1] as f64).sum();
            let dealt: f64 = rs.iter().map(|r| r.lost_hp[0] as f64).sum();
            let _ = writeln!(
                o,
                "  masher {i}:{:<10} AI {j}:{:<10} lost {:>2}/{:<2}  HP {:>4} / {:<4}  lost {:>4.0}/min  dealt {:>4.0}/min  {:.1} min",
                s.navis[i],
                s.navis[j],
                rs.iter().filter(|r| r.result < 0).count(),
                rs.len(),
                median(&mut ai_hp).unwrap_or(0),
                median(&mut masher_hp).unwrap_or(0),
                taken / minutes.max(1e-9),
                dealt / minutes.max(1e-9),
                minutes / rs.len() as f64
            );
        }
    }
    let _ = writeln!(o);

    for (name, f) in &groups[..2] {
        let hits: Vec<&Hit> = rounds.iter().filter(|r| f(r)).flat_map(|r| &r.hits).collect();
        let n = hits.len();
        let _ = writeln!(o, "=== hits in {name} rounds: {n} ===");
        if n == 0 {
            continue;
        }
        let modes = count(hits.iter().map(|h| h.mode.clone()));
        let _ = writeln!(o, "mode at the hit: {}", top(&modes, n, 6));
        let sources = count(hits.iter().map(|h| h.source.to_string()));
        let _ = writeln!(o, "hitters read from: {}", top(&sources, n, 3));
        let actions = count(hits.iter().map(|h| h.ai_action.clone()));
        let _ = writeln!(o, "AI navi's action the tick before: {}", top(&actions, n, 8));
        let mactions = count(hits.iter().map(|h| h.masher_action.clone()));
        let _ = writeln!(o, "masher navi's action the tick before: {}", top(&mactions, n, 8));
        let _ = writeln!(o, "the AI navi's accumulators held a hitter the tick before: {}", pct(hits.iter().filter(|h| h.acc_bits > 0).count(), n));

        // By the main hitter's kind.
        let mut by: BTreeMap<String, Vec<&Hit>> = BTreeMap::new();
        for h in &hits {
            by.entry(cause(h)).or_default().push(h);
        }
        let mut kinds: Vec<(String, Vec<&Hit>)> = by.into_iter().collect();
        let damage_of = |hs: &[&Hit]| hs.iter().map(|h| h.damage as u64).sum::<u64>();
        kinds.sort_by(|a, b| damage_of(&b.1).cmp(&damage_of(&a.1)).then(a.0.cmp(&b.0)));
        let total_damage: f64 = hits.iter().map(|h| h.damage as f64).sum();
        let _ = writeln!(o, "by cause (top 15 by damage; dmg% = share of the HP lost; the age columns, seen, no-dec and med-mrg are of the hitter object):");
        let _ = writeln!(
            o,
            "  {:<44} {:>5} {:>6} {:>6} {:>7} {:>6} {:>6} {:>6} {:>6} {:>7} {:>7} {:>7} {:>7} {:>7}",
            "kind", "hits", "share", "dmg%", "med-age", "<10", "10-30", ">30", "seen", "no-dec", "med-mrg", "dirpr", "moved", "inrow"
        );
        for (kind, hs) in kinds.iter().take(15) {
            let m = hs.len();
            let mut ages: Vec<i64> = hs.iter().filter_map(|h| main_hitter(h)).map(|s| s.age as i64).collect();
            let young = ages.iter().filter(|&&a| a < 10).count();
            let mid = ages.iter().filter(|&&a| (10..=30).contains(&a)).count();
            let old = ages.iter().filter(|&&a| a > 30).count();
            let seen = hs.iter().filter_map(|h| main_hitter(h)).filter(|s| s.seen).count();
            let undecided = hs.iter().filter_map(|h| main_hitter(h)).filter(|s| s.margin.is_none()).count();
            let mut margins: Vec<i64> = hs.iter().filter_map(|h| main_hitter(h)).filter_map(|s| s.margin).collect();
            let dmg = 100.0 * hs.iter().map(|h| h.damage as f64).sum::<f64>() / total_damage.max(1.0);
            let dir = hs.iter().filter(|h| h.pressed_direction).count();
            let moved = hs.iter().filter(|h| h.moved).count();
            let in_row = hs.iter().filter(|h| h.same_row >= 20).count();
            let has = ages.len();
            let _ = writeln!(
                o,
                "  {:<44} {:>5} {:>6} {:>5.1}% {:>7} {:>6} {:>6} {:>6} {:>6} {:>7} {:>7} {:>7} {:>7} {:>7}",
                kind,
                m,
                pct(m, n),
                dmg,
                median(&mut ages).map_or("-".into(), |a| a.to_string()),
                pct(young, has),
                pct(mid, has),
                pct(old, has),
                pct(seen, has),
                pct(undecided, has),
                median(&mut margins).map_or("-".into(), |a| a.to_string()),
                pct(dir, m),
                pct(moved, m),
                pct(in_row, m)
            );
        }
        let with: Vec<&Seen> = hits.iter().filter_map(|h| main_hitter(h)).collect();
        let w = with.len();
        let _ = writeln!(o, "hits with a hitter object: {w} ({}); without: {}", pct(w, n), n - w);
        let young = with.iter().filter(|s| s.age < 10).count();
        let mid = with.iter().filter(|s| (10..=30).contains(&s.age)).count();
        let old = with.iter().filter(|s| s.age > 30).count();
        let _ = writeln!(o, "hitter's age at the hit: <10 {} | 10-30 {} | >30 {}", pct(young, w), pct(mid, w), pct(old, w));
        let unseen = with.iter().filter(|s| !s.seen).count();
        let invisible = with.iter().filter(|s| !s.visible).count();
        let unencoded = with.iter().filter(|s| s.seen && !s.encoded).count();
        let _ = writeln!(o, "hitter not in the AI's view the tick before the hit: {} (not visible_to side 1: {}; seen but past the 48 tokens: {})", pct(unseen, w), pct(invisible, w), pct(unencoded, w));
        let undecided = with.iter().filter(|s| s.margin.is_none()).count();
        let _ = writeln!(o, "hitter never in a decision's view before the hit (no chance to react): {}", pct(undecided, w));
        // (The same without the projectile kind: shots' projectiles, the
        // buster's among them, which no console draws.)
        let other: Vec<&Seen> = with.iter().copied().filter(|s| s.kind != "projectile").collect();
        let w2 = other.len();
        let mut ages2: Vec<i64> = other.iter().map(|s| s.age as i64).collect();
        let _ = writeln!(
            o,
            "hitters other than `projectile` ({w2}): age <10 {} | 10-30 {} | >30 {} (median {}); not in the AI's view the tick before {}; never in a decision's view {}; damage share of all {}",
            pct(other.iter().filter(|s| s.age < 10).count(), w2),
            pct(other.iter().filter(|s| (10..=30).contains(&s.age)).count(), w2),
            pct(other.iter().filter(|s| s.age > 30).count(), w2),
            median(&mut ages2).map_or("-".into(), |a| a.to_string()),
            pct(other.iter().filter(|s| !s.seen).count(), w2),
            pct(other.iter().filter(|s| s.margin.is_none()).count(), w2),
            pct(hits.iter().filter(|h| main_hitter(h).is_some_and(|s| s.kind != "projectile")).map(|h| h.damage as usize).sum(), total_damage as usize)
        );
        let mut margins: Vec<i64> = with.iter().filter_map(|s| s.margin).collect();
        let short = margins.iter().filter(|&&m| m < 8).count();
        let _ = writeln!(
            o,
            "margin (first decision on it → hit), where some: median {} ticks, <8 ticks {}, ≥30 ticks {}",
            median(&mut margins).map_or("-".into(), |m| m.to_string()),
            pct(short, margins.len()),
            pct(margins.iter().filter(|&&m| m >= 30).count(), margins.len())
        );
        // Hits with no hitter object: what was near.
        let lone: Vec<&&Hit> = hits.iter().filter(|h| main_hitter(h).is_none()).collect();
        if !lone.is_empty() {
            let nearest = count(lone.iter().map(|h| {
                h.near.iter().min_by_key(|s| (s.dx.unsigned_abs() + s.dy.unsigned_abs(), s.age)).map_or("(no masher object)".to_string(), |s| format!("{} (dist {})", s.kind, s.dx.unsigned_abs() + s.dy.unsigned_abs()))
            }));
            let _ = writeln!(o, "no hitter object ({}): nearest masher object: {}", lone.len(), top(&nearest, lone.len(), 8));
        }
        let dir = hits.iter().filter(|h| h.pressed_direction).count();
        let moved = hits.iter().filter(|h| h.moved).count();
        let _ = writeln!(o, "AI pressed a direction in the {BEFORE} ticks before: {}; its navi changed panel in them: {}", pct(dir, n), pct(moved, n));
        let _ = writeln!(
            o,
            "AI navi in the masher navi's row for all {BEFORE} ticks before: {}; for ≥20 of them: {}",
            pct(hits.iter().filter(|h| h.same_row as usize >= BEFORE).count(), n),
            pct(hits.iter().filter(|h| h.same_row >= 20).count(), n)
        );
        let warned: Vec<u32> = hits.iter().filter_map(|h| h.warned).collect();
        let _ = writeln!(
            o,
            "AI's panel marked attacked by the masher's side in the {WARN} ticks before: {} (of those, ≥10 ticks before: {})",
            pct(warned.len(), n),
            pct(warned.iter().filter(|&&w| w >= 10).count(), warned.len())
        );
        let chips = count(hits.iter().filter(|h| h.masher_chip.as_ref().is_some_and(|(_, age)| *age <= 120)).map(|h| h.masher_chip.as_ref().unwrap().0.clone()));
        let within: usize = chips.values().sum();
        let _ = writeln!(o, "masher used a chip in the 120 ticks before: {} — {}", pct(within, n), top(&chips, within, 10));
        let _ = writeln!(o);
    }
    o
}

fn count(it: impl Iterator<Item = String>) -> BTreeMap<String, usize> {
    let mut m = BTreeMap::new();
    for s in it {
        *m.entry(s).or_insert(0) += 1;
    }
    m
}

fn top(m: &BTreeMap<String, usize>, of: usize, n: usize) -> String {
    let mut v: Vec<(&String, &usize)> = m.iter().collect();
    v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    v.iter().take(n).map(|(k, c)| format!("{k} {} ({})", c, pct(**c, of))).collect::<Vec<_>>().join(", ")
}

fn write_hits(path: &Path, s: &Shared, rounds: &[RoundOut]) {
    let mut o = String::from(
        "round\tresult\tmatchup\tmasher_navi\tai_navi\ttick\tfight_tick\tdamage\thp_after\tmode\tai_panel\tpanel_type\tmasher_panel\tai_action_before\tmasher_action_before\thitter_source\tacc_bits\thitters\tmasher_last_chip\tmasher_chip_age\twarned\tai_dir_pressed_30\tai_moved_30\tsame_row_30\tai_buttons_30\tmasher_objects\n",
    );
    for r in rounds {
        for h in &r.hits {
            let _ = writeln!(
                o,
                "{}\t{}\t{}x{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{},{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                r.n,
                match r.result {
                    -1 => "lost",
                    1 => "won",
                    _ => "neither",
                },
                r.matchup.0,
                r.matchup.1,
                s.navis[r.matchup.0],
                s.navis[r.matchup.1],
                h.tick,
                h.fight_tick,
                h.damage,
                h.hp_after,
                h.mode,
                h.ai_panel.0,
                h.ai_panel.1,
                h.panel_type,
                h.masher_panel.map_or("-".into(), |p| format!("{},{}", p.0, p.1)),
                h.ai_action,
                h.masher_action,
                h.source,
                h.acc_bits,
                h.hitters.iter().map(Seen::text).collect::<Vec<_>>().join(" "),
                h.masher_chip.as_ref().map_or("-".into(), |c| c.0.clone()),
                h.masher_chip.as_ref().map_or("-".into(), |c| c.1.to_string()),
                h.warned.map_or("-".into(), |w| w.to_string()),
                h.pressed_direction as u8,
                h.moved as u8,
                h.same_row,
                h.buttons,
                h.near.iter().map(Seen::text).collect::<Vec<_>>().join(" "),
            );
        }
    }
    std::fs::write(path, o).expect("hits.tsv");
}

fn main() {
    let a = args();
    std::fs::create_dir_all(&a.out).expect("the output folder");
    env::quiet_engine_panics();
    let packs = nettai_content::pack::packs_dir();
    let content = nettai_training::pool::load_game(None, &packs, "exe6").expect("EXE6's content");
    eprintln!("content {:016x}; fingerprints from {}", content.hash().0, a.cache.display());
    let fingerprints = Fingerprints::for_game(&content, a.threads, Some(a.cache.as_path())).expect("the fingerprints");
    let game = Game::with_fingerprints(content.clone(), fingerprints.clone()).expect("the game");
    // (The env's `EnvBuilds::make` for a fixed build.)
    let combos = match game.names.fingerprints() {
        Some(f) => Combos { pairs: &f.pairs, rate: 0.3 },
        None => Combos::NONE,
    };
    let builds: Vec<Build> = (0..FIXED_BUILDS)
        .map(|i| game.maker.random_with(&mut Picks::new(FIXED_BUILDS_SEED.wrapping_add(i)), &Holdout::default(), 0.75, 0.5, combos).expect("a fixed build"))
        .collect();
    let navis: Vec<String> = builds.iter().map(|b| content.defs.navi(b.side.navi(&content)).key.clone()).collect();
    eprintln!("fixed builds: {navis:?}");
    let net = Arc::new(Net::from_safetensors(&std::fs::read(&a.weights).expect("the weights")).expect("the network"));
    let shared = Shared { content: content.clone(), game, fingerprints, builds, navis, net: net.clone(), seed: a.seed };

    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<RoundOut>>> = Mutex::new((0..a.rounds).map(|_| None).collect());
    let started = std::time::Instant::now();
    std::thread::scope(|scope| {
        for _ in 0..a.threads.max(1) {
            scope.spawn(|| {
                loop {
                    let n = next.fetch_add(1, Ordering::Relaxed);
                    if n >= a.rounds {
                        break;
                    }
                    let r = play_round(&shared, n);
                    results.lock().unwrap()[n] = Some(r);
                    let d = done.fetch_add(1, Ordering::Relaxed) + 1;
                    if d.is_multiple_of(25) || d == a.rounds {
                        eprintln!("{d}/{} rounds ({:.0?})", a.rounds, started.elapsed());
                    }
                }
            });
        }
    });
    let rounds: Vec<RoundOut> = results.into_inner().unwrap().into_iter().map(|r| r.expect("every round played")).collect();

    let probe = NetAi::of(net.clone(), Level::Normal, 0, Fingerprinter::made(shared.fingerprints.clone())).expect("the AI");
    let (d, k) = (probe.knobs().delay as u32, net.k());
    write_hits(&a.out.join("hits.tsv"), &shared, &rounds);
    let text = summary(&shared, &rounds, &a, d, k);
    std::fs::write(a.out.join("summary.txt"), &text).expect("summary.txt");
    print!("{text}");

    // Each lost round's replay; the frames of the first ones.
    let replays = a.out.join("replays");
    std::fs::create_dir_all(&replays).expect("the replays' folder");
    let mut rendered = 0;
    for r in rounds.iter().filter(|r| r.result < 0) {
        let path = replays.join(format!("round_{:04}.nettaireplay", r.n));
        std::fs::write(&path, &r.replay).expect("a replay");
        if rendered < a.render {
            rendered += 1;
            let dir = a.out.join(format!("round_{:04}", r.n));
            std::fs::create_dir_all(&dir).expect("a round's folder");
            // (The drops that cost the most, the earlier first of equal ones:
            // a poison panel's drain of one HP a tick says little.)
            let mut drops: Vec<&Hit> = r.hits.iter().collect();
            drops.sort_by(|a, b| b.damage.cmp(&a.damage).then(a.tick.cmp(&b.tick)));
            drops.truncate(DROPS_RENDERED);
            drops.sort_by_key(|h| h.tick);
            let mut frames: Vec<u32> = drops.iter().flat_map(|h| [h.tick.saturating_sub(LEAD).max(1), h.tick]).collect();
            frames.sort_unstable();
            frames.dedup();
            let list = frames.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(",");
            let mut text = format!("{}\n{list}\n# frame (= ticks played) of each HP drop rendered, its damage, what it is put down to, the AI's HP after\n", path.display());
            for h in &drops {
                let _ = writeln!(text, "# {} -{} {} hp {}", h.tick, h.damage, cause(h), h.hp_after);
            }
            std::fs::write(dir.join("frames.txt"), text).expect("frames.txt");
        }
    }
    eprintln!("wrote {} and {}; {} lost rounds' replays in {}", a.out.join("hits.tsv").display(), a.out.join("summary.txt").display(), rounds.iter().filter(|r| r.result < 0).count(), replays.display());
    let unended = rounds.iter().filter(|r| !r.replay_ended).count();
    if unended > 0 {
        eprintln!("{unended} rounds' replays end before the set's end (open)");
    }
}
