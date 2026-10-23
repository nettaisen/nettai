//! Many envs on persistent worker threads, stepped as one batch
//! (docs/design/training-ai.md §6.5, §6.6).
//!
//! A worker owns its envs for its whole life, and with them its thread's
//! content runtimes (a Luau VM a game, about a second to load: `behavior`'s
//! per-thread cache). [`Pool::step`] hands every worker the batch's
//! actions; each steps its envs to their next decisions and writes their
//! rows of the pool's [`Buffers`], which the pool owns and never
//! reallocates (the bridge shows them to Python as numpy arrays without a
//! copy). While a worker waits for the next step it makes builds ahead
//! (`Env::prepare`), from each game's own stream of draws, so a round's
//! builds are the same however busy the workers were.

use crate::bots::BotKind;
use crate::env::{self, Config, Controller, Env, Game};
use crate::spec;
use crate::stats::RoundStats;
use nettai_battle::Content;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

/// What a pool is made with (§6.6's `Pool(...)`).
#[derive(Clone, Debug)]
pub struct PoolConfig {
    /// The games, by name; each env draws one each round.
    pub games: Vec<String>,
    pub envs: usize,
    pub threads: usize,
    pub seed: u64,
    /// The content directory (none: `nettai_content::index::content`, the
    /// repository's) and the packs directory (none: `$NETTAI_PACKS`, else
    /// `data`).
    pub content: Option<PathBuf>,
    pub packs: Option<PathBuf>,
    pub env: Config,
}

/// The pool's arrays, refilled by every step (§6.6): by set, the
/// observation's features `[E, 2, C, W]`, ids `[E, 2, C, I]` and masks
/// `[E, 2, C]`; by env and seat, the phase, the actions allowed, the HP
/// gain, the result and the delay; by env, whether the round ended (and
/// was cut off), and its game.
#[derive(Debug)]
pub struct Buffers {
    pub envs: usize,
    pub x: Vec<Vec<f32>>,
    pub ids: Vec<Vec<i32>>,
    pub mask: Vec<Vec<bool>>,
    pub phase: Vec<i8>,
    pub legal_fight: Vec<bool>,
    pub legal_screen: Vec<bool>,
    pub hp_gain: Vec<f32>,
    pub done: Vec<bool>,
    pub truncated: Vec<bool>,
    pub result: Vec<i8>,
    pub game: Vec<i8>,
    pub delay: Vec<i16>,
}

impl Buffers {
    fn new(envs: usize) -> Buffers {
        let seats = envs * 2;
        Buffers {
            envs,
            x: spec::SETS.iter().map(|s| vec![0.0; seats * s.count * s.width]).collect(),
            ids: spec::SETS.iter().map(|s| vec![0; seats * s.count * s.ids]).collect(),
            mask: spec::SETS.iter().map(|s| vec![false; seats * s.count]).collect(),
            phase: vec![0; seats],
            legal_fight: vec![false; seats * spec::FIGHT_ACTIONS],
            legal_screen: vec![false; seats * spec::SCREEN_ACTIONS],
            hp_gain: vec![0.0; seats],
            done: vec![false; envs],
            truncated: vec![false; envs],
            result: vec![0; seats],
            game: vec![0; envs],
            delay: vec![0; seats],
        }
    }
}

/// Where a worker writes its envs' rows: the buffers' memory, which the
/// pool keeps alive and in place while any worker lives. Workers write
/// disjoint rows (their own envs'), and only while the pool waits for
/// them in a call that holds it mutably.
#[derive(Clone, Copy)]
struct Rows {
    x: [*mut f32; spec::SETS.len()],
    ids: [*mut i32; spec::SETS.len()],
    mask: [*mut bool; spec::SETS.len()],
    phase: *mut i8,
    legal_fight: *mut bool,
    legal_screen: *mut bool,
    hp_gain: *mut f32,
    done: *mut bool,
    truncated: *mut bool,
    result: *mut i8,
    game: *mut i8,
    delay: *mut i16,
}

// SAFETY: the pointers are into the pool's buffers, which outlive the
// workers (the pool joins them before it drops the buffers), never move
// (they are never resized), and each worker writes only its own envs' rows,
// while the pool's own thread is blocked waiting for it.
unsafe impl Send for Rows {}

impl Rows {
    fn of(b: &mut Buffers) -> Rows {
        Rows {
            x: std::array::from_fn(|i| b.x[i].as_mut_ptr()),
            ids: std::array::from_fn(|i| b.ids[i].as_mut_ptr()),
            mask: std::array::from_fn(|i| b.mask[i].as_mut_ptr()),
            phase: b.phase.as_mut_ptr(),
            legal_fight: b.legal_fight.as_mut_ptr(),
            legal_screen: b.legal_screen.as_mut_ptr(),
            hp_gain: b.hp_gain.as_mut_ptr(),
            done: b.done.as_mut_ptr(),
            truncated: b.truncated.as_mut_ptr(),
            result: b.result.as_mut_ptr(),
            game: b.game.as_mut_ptr(),
            delay: b.delay.as_mut_ptr(),
        }
    }

    /// Env `e`'s rows, after a step that ended on `out`.
    ///
    /// SAFETY: `e` is one of the calling worker's envs, and the pool waits.
    unsafe fn write(&self, e: usize, env: &Env, out: &env::Outcome) {
        unsafe {
            for s in 0..2 {
                let at = e * 2 + s;
                *self.phase.add(at) = out.phase[s] as i8;
                *self.hp_gain.add(at) = out.hp_gain[s];
                *self.result.add(at) = out.result[s];
                *self.delay.add(at) = env.delay()[s] as i16;
                if out.phase[s] == env::NONE {
                    continue;
                }
                for (k, set) in spec::SETS.iter().enumerate() {
                    let o = &env.obs[s].sets[k];
                    std::ptr::copy_nonoverlapping(o.x.as_ptr(), self.x[k].add(at * set.count * set.width), o.x.len());
                    std::ptr::copy_nonoverlapping(o.ids.as_ptr(), self.ids[k].add(at * set.count * set.ids), o.ids.len());
                    std::ptr::copy_nonoverlapping(o.mask.as_ptr(), self.mask[k].add(at * set.count), o.mask.len());
                }
                std::ptr::copy_nonoverlapping(env.legal_fight[s].as_ptr(), self.legal_fight.add(at * spec::FIGHT_ACTIONS), spec::FIGHT_ACTIONS);
                std::ptr::copy_nonoverlapping(env.legal_screen[s].as_ptr(), self.legal_screen.add(at * spec::SCREEN_ACTIONS), spec::SCREEN_ACTIONS);
            }
            *self.done.add(e) = out.done;
            *self.truncated.add(e) = out.truncated;
            *self.game.add(e) = env.game() as i8;
        }
    }
}

/// What the pool asks of a worker.
enum Cmd {
    Step(Arc<Vec<i32>>),
    Seat(Vec<usize>, usize, Controller),
    Delay(Vec<usize>, usize, Option<u16>),
    Hp(u16, u16),
    Exclude(Vec<String>),
    Stats,
}

enum Reply {
    Done(Result<(), String>),
    Stats(Vec<RoundStats>),
}

struct Worker {
    /// Its envs, by the pool's numbering.
    envs: std::ops::Range<usize>,
    tx: Sender<Cmd>,
    rx: Receiver<Reply>,
    thread: Option<std::thread::JoinHandle<()>>,
}

/// N envs on persistent worker threads.
pub struct Pool {
    games: Vec<String>,
    workers: Vec<Worker>,
    buffers: Buffers,
}

/// Game `game`'s content from its pack in `packs` and the content in
/// `content` (none: the repository's), as the app loads it: the battle data
/// and the animations' timings (no picture or sound).
pub fn load_game(content: Option<&Path>, packs: &Path, game: &str) -> Result<Arc<Content>, String> {
    let mut report = nettai_content::report::Report::default();
    let found = nettai_content::pack::find(packs, &[], &mut report).ok_or_else(|| format!("no packs in {}: {report}", packs.display()))?;
    let loaded = nettai_content::pack::load_game(content, game, &found).map_err(|r| format!("{game}: {r}"))?;
    Ok(Arc::new(loaded.content))
}

impl Pool {
    /// A pool as `cfg` says, its games loaded from the packs.
    pub fn new(cfg: PoolConfig) -> Result<Pool, String> {
        let packs = cfg.packs.clone().unwrap_or_else(nettai_content::pack::packs_dir);
        let contents = cfg.games.iter().map(|g| load_game(cfg.content.as_deref(), &packs, g)).collect::<Result<Vec<_>, _>>()?;
        Pool::with_contents(cfg, contents)
    }

    /// A pool on contents already loaded (by game, in `cfg.games`' order).
    pub fn with_contents(cfg: PoolConfig, contents: Vec<Arc<Content>>) -> Result<Pool, String> {
        if cfg.envs == 0 || cfg.threads == 0 {
            return Err("a pool has at least one env and one thread".into());
        }
        if cfg.games.len() > i8::MAX as usize {
            return Err("too many games".into());
        }
        let games: Arc<Vec<Game>> = Arc::new(contents.into_iter().map(Game::new).collect::<Result<_, _>>()?);
        for (name, g) in cfg.games.iter().zip(games.iter()) {
            if *name != g.name {
                return Err(format!("the content loaded for {name} is {}'s", g.name));
            }
        }
        let mut buffers = Buffers::new(cfg.envs);
        let rows = Rows::of(&mut buffers);
        let threads = cfg.threads.min(cfg.envs);
        let mut workers = Vec::with_capacity(threads);
        for w in 0..threads {
            let envs = (cfg.envs * w / threads)..(cfg.envs * (w + 1) / threads);
            let (tx, rx_cmd) = channel();
            let (tx_reply, rx) = channel();
            let (games, config, seed, range) = (games.clone(), cfg.env.clone(), cfg.seed, envs.clone());
            let thread = std::thread::Builder::new()
                .name(format!("nettai-pool-{w}"))
                .spawn(move || work(games, config, seed, range, rows, rx_cmd, tx_reply))
                .map_err(|e| format!("a worker thread: {e}"))?;
            workers.push(Worker { envs, tx, rx, thread: Some(thread) });
        }
        let pool = Pool { games: cfg.games.clone(), workers, buffers };
        // (Each worker's first reply: its envs made and at their first decisions.)
        for w in &pool.workers {
            match w.rx.recv() {
                Ok(Reply::Done(r)) => r?,
                _ => return Err("a worker stopped as it started".into()),
            }
        }
        Ok(pool)
    }

    pub fn games(&self) -> &[String] {
        &self.games
    }

    pub fn envs(&self) -> usize {
        self.buffers.envs
    }

    pub fn buffers(&self) -> &Buffers {
        &self.buffers
    }

    /// The buffers' memory, to show without a copy (the bridge's numpy
    /// views). It stays where it is for the pool's life.
    pub fn buffers_mut(&mut self) -> &mut Buffers {
        &mut self.buffers
    }

    /// Ask every worker `cmd` (made for each), and wait for their answers.
    fn ask(&mut self, cmd: impl Fn(&Worker) -> Option<Cmd>) -> Result<Vec<Reply>, String> {
        let mut asked = Vec::new();
        for (i, w) in self.workers.iter().enumerate() {
            if let Some(c) = cmd(w) {
                w.tx.send(c).map_err(|_| format!("worker {i} has stopped"))?;
                asked.push(i);
            }
        }
        asked.into_iter().map(|i| self.workers[i].rx.recv().map_err(|_| format!("worker {i} stopped (a panic: see its message above)"))).collect()
    }

    fn done(replies: Vec<Reply>) -> Result<(), String> {
        for r in replies {
            if let Reply::Done(Err(e)) = r {
                return Err(e);
            }
        }
        Ok(())
    }

    /// Apply `actions` (`[E, 2]`, read where the last step asked a seat),
    /// and step every env to its next decision (§6.6).
    pub fn step(&mut self, actions: &[i32]) -> Result<(), String> {
        if actions.len() != self.envs() * 2 {
            return Err(format!("{} actions for {} envs (2 a env)", actions.len(), self.envs()));
        }
        // (An action not allowed refuses the whole step, before any env moves.)
        let b = &self.buffers;
        for (at, &a) in actions.iter().enumerate() {
            let legal = match b.phase[at] {
                1 => usize::try_from(a).ok().and_then(|a| (a < spec::FIGHT_ACTIONS).then(|| b.legal_fight[at * spec::FIGHT_ACTIONS + a])),
                2 => usize::try_from(a).ok().and_then(|a| (a < spec::SCREEN_ACTIONS).then(|| b.legal_screen[at * spec::SCREEN_ACTIONS + a])),
                _ => Some(true),
            };
            if legal != Some(true) {
                return Err(format!("env {} seat {}: action {a} isn't allowed (phase {})", at / 2, at % 2, b.phase[at]));
            }
        }
        let actions = Arc::new(actions.to_vec());
        let replies = self.ask(|_| Some(Cmd::Step(actions.clone())))?;
        Pool::done(replies)
    }

    /// Who plays seat `seat` of envs `envs` from their next round.
    pub fn set_seat(&mut self, envs: &[usize], seat: usize, controller: Controller) -> Result<(), String> {
        self.check(envs, seat)?;
        let replies = self.ask(|w| {
            let mine: Vec<usize> = envs.iter().copied().filter(|e| w.envs.contains(e)).collect();
            (!mine.is_empty()).then_some(Cmd::Seat(mine, seat, controller))
        })?;
        Pool::done(replies)
    }

    /// The delay seat `seat` of envs `envs` plays at from their next round
    /// (none: drawn again).
    pub fn set_delay(&mut self, envs: &[usize], seat: usize, ticks: Option<u16>) -> Result<(), String> {
        self.check(envs, seat)?;
        let replies = self.ask(|w| {
            let mine: Vec<usize> = envs.iter().copied().filter(|e| w.envs.contains(e)).collect();
            (!mine.is_empty()).then_some(Cmd::Delay(mine, seat, ticks))
        })?;
        Pool::done(replies)
    }

    /// Both sides' base HP drawn from `lo` to `hi`, from the next rounds.
    pub fn set_hp(&mut self, lo: u16, hi: u16) -> Result<(), String> {
        if lo == 0 || lo > hi {
            return Err(format!("HP {lo} to {hi}"));
        }
        let replies = self.ask(|_| Some(Cmd::Hp(lo, hi)))?;
        Pool::done(replies)
    }

    /// The chips (by key) no random build puts in a folder, from the next
    /// rounds.
    pub fn set_exclude_chips(&mut self, keys: Vec<String>) -> Result<(), String> {
        let replies = self.ask(|_| Some(Cmd::Exclude(keys.clone())))?;
        Pool::done(replies)
    }

    /// The rounds ended since the last call.
    pub fn round_stats(&mut self) -> Result<Vec<RoundStats>, String> {
        let replies = self.ask(|_| Some(Cmd::Stats))?;
        Ok(replies.into_iter().flat_map(|r| if let Reply::Stats(s) = r { s } else { Vec::new() }).collect())
    }

    fn check(&self, envs: &[usize], seat: usize) -> Result<(), String> {
        if seat > 1 {
            return Err(format!("seat {seat}: a seat is 0 or 1"));
        }
        if let Some(e) = envs.iter().find(|&&e| e >= self.envs()) {
            return Err(format!("env {e}: the pool has {}", self.envs()));
        }
        Ok(())
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        for w in &mut self.workers {
            // (A worker whose channel closes ends.)
            let (tx, _) = channel();
            drop(std::mem::replace(&mut w.tx, tx));
        }
        for w in &mut self.workers {
            if let Some(t) = w.thread.take() {
                let _ = t.join();
            }
        }
    }
}

/// A controller by its name (§6.6: "policy" or a scripted player's).
pub fn controller(name: &str) -> Result<Controller, String> {
    Controller::named(name).ok_or_else(|| {
        let bots: Vec<&str> = BotKind::ALL.iter().map(|k| k.name()).collect();
        format!("no controller {name:?} (policy, {})", bots.join(", "))
    })
}

/// A worker's life: its envs made on its thread (its runtimes loaded
/// first), their first decisions written; then each command, and builds
/// made ahead while none waits.
fn work(games: Arc<Vec<Game>>, cfg: Config, seed: u64, range: std::ops::Range<usize>, rows: Rows, rx: Receiver<Cmd>, tx: Sender<Reply>) {
    let started = (|| -> Result<Vec<Env>, String> {
        for g in games.iter() {
            nettai_battle::behavior::Behaviors::for_content(&g.content).map_err(|e| format!("{}: {e}", g.name))?;
        }
        let mut envs = Vec::with_capacity(range.len());
        for e in range.clone() {
            let mut env = Env::new(games.clone(), cfg.clone(), seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(e as u64))?;
            let out = env.step([0, 0])?;
            // SAFETY: env `e` is this worker's; the pool waits for its first reply.
            unsafe { rows.write(e, &env, &out) };
            envs.push(env);
        }
        Ok(envs)
    })();
    let mut envs = match started {
        Ok(envs) => {
            let _ = tx.send(Reply::Done(Ok(())));
            envs
        }
        Err(e) => {
            let _ = tx.send(Reply::Done(Err(e)));
            return;
        }
    };
    let mut failed: Option<String> = None;
    loop {
        let cmd = match rx.try_recv() {
            Ok(c) => c,
            Err(TryRecvError::Disconnected) => return,
            Err(TryRecvError::Empty) => {
                // (Nothing asked: a build made ahead, then look again.)
                let mut made = false;
                if failed.is_none() {
                    for env in &mut envs {
                        match env.prepare() {
                            Ok(true) => {
                                made = true;
                                break;
                            }
                            Ok(false) => {}
                            Err(e) => {
                                failed = Some(e);
                                break;
                            }
                        }
                    }
                }
                if made {
                    continue;
                }
                match rx.recv() {
                    Ok(c) => c,
                    Err(_) => return,
                }
            }
        };
        let reply = match cmd {
            Cmd::Step(actions) => Reply::Done(match failed.take() {
                Some(e) => Err(e),
                None => (|| {
                    for (k, env) in envs.iter_mut().enumerate() {
                        let e = range.start + k;
                        let out = env.step([actions[e * 2], actions[e * 2 + 1]]).map_err(|why| format!("env {e}: {why}"))?;
                        // SAFETY: env `e` is this worker's; the pool waits for the reply.
                        unsafe { rows.write(e, env, &out) };
                    }
                    Ok(())
                })(),
            }),
            Cmd::Seat(list, seat, c) => {
                for e in list {
                    envs[e - range.start].set_seat(seat, c);
                }
                Reply::Done(Ok(()))
            }
            Cmd::Delay(list, seat, d) => {
                for e in list {
                    envs[e - range.start].set_delay(seat, d);
                }
                Reply::Done(Ok(()))
            }
            Cmd::Hp(lo, hi) => {
                for env in &mut envs {
                    env.set_hp(lo, hi);
                }
                Reply::Done(Ok(()))
            }
            Cmd::Exclude(keys) => {
                for env in &mut envs {
                    env.set_exclude_chips(keys.clone());
                }
                Reply::Done(Ok(()))
            }
            Cmd::Stats => Reply::Stats(envs.iter_mut().flat_map(|e| std::mem::take(&mut e.ended)).collect()),
        };
        if tx.send(reply).is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests;
