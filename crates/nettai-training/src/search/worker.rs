//! The lookahead's thread in live play (training-ai.md §10.7): the battle's
//! thread asks it for a decision [`Search::lead`] ticks before the decision
//! takes effect ([`Worker::ask`]) and takes the answer then
//! ([`Worker::wait`]), so a decision's tries are never stepped on the
//! battle's thread. What it answers depends only on what it was asked, never
//! on how fast it is: an answer not ready when it is due is waited for.
//!
//! Its runtime is loaded as a round starts ([`Worker::load`]), while the
//! custom screen is open, not at the fight's first decision.

use super::{LATE, Search};
use crate::spec;
use nettai_battle::{Battle, Content};
use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Condvar, Mutex};

/// A thread of its own that looks ahead for one player.
pub struct Worker {
    shared: Arc<Shared>,
    asked: u64,
}

#[derive(Default)]
struct Shared {
    queue: Mutex<Queue>,
    wake: Condvar,
    answer: Mutex<Option<Answer>>,
}

/// The last decision answered: its number and the action (None: none was
/// allowed), or why the thread stopped.
type Answer = (u64, Result<Option<usize>, String>);

#[derive(Default)]
struct Queue {
    jobs: VecDeque<Job>,
    closed: bool,
}

enum Job {
    Load(Arc<Content>),
    Ahead(Box<Asked>),
}

/// A decision asked for ([`Search::ahead`]'s arguments).
struct Asked {
    n: u64,
    search: Search,
    battle: Battle,
    side: u8,
    k: u32,
    held: u16,
    logits: [f32; spec::FIGHT_ACTIONS],
    luck: u64,
}

impl Worker {
    /// Its thread started; an error if it can't be.
    pub fn start() -> Result<Worker, String> {
        let shared = Arc::new(Shared::default());
        let s = shared.clone();
        nettai_threads::spawn("lookahead", move || work(&s)).map_err(|e| format!("the lookahead's thread couldn't start: {e}"))?;
        Ok(Worker { shared, asked: 0 })
    }

    fn push(&self, job: Job) {
        nettai_threads::lock(&self.shared.queue).unwrap_or_else(|p| p.into_inner()).jobs.push_back(job);
        self.shared.wake.notify_one();
    }

    /// Load `content`'s runtime on its thread (a round of it starts).
    pub fn load(&self, content: Arc<Content>) {
        self.push(Job::Load(content));
    }

    /// Ask for side `side`'s decision `search.lead` ticks after `b`
    /// ([`Search::ahead`]): its number, to [`Worker::wait`] for.
    #[allow(clippy::too_many_arguments)]
    pub fn ask(&mut self, search: Search, b: &Battle, side: u8, k: u32, held: u16, logits: [f32; spec::FIGHT_ACTIONS], luck: u64) -> u64 {
        self.asked += 1;
        let n = self.asked;
        self.push(Job::Ahead(Box::new(Asked { n, search, battle: b.clone(), side, k, held, logits, luck })));
        n
    }

    /// Decision `n`'s answer, waited for if it isn't ready (counted in
    /// [`LATE`]); an error if the thread stopped.
    pub fn wait(&self, n: u64) -> Result<Option<usize>, String> {
        let mut late = false;
        loop {
            {
                let answer = nettai_threads::lock(&self.shared.answer).unwrap_or_else(|p| p.into_inner());
                match &*answer {
                    Some((got, a)) if *got == n => return a.clone(),
                    Some((_, Err(why))) => return Err(why.clone()),
                    _ => {}
                }
            }
            if !late {
                late = true;
                LATE.fetch_add(1, Ordering::Relaxed);
            }
            std::hint::spin_loop();
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let mut q = nettai_threads::lock(&self.shared.queue).unwrap_or_else(|p| p.into_inner());
        q.closed = true;
        q.jobs.clear();
        drop(q);
        self.shared.wake.notify_one();
    }
}

/// The thread: each job in turn till the worker is dropped.
fn work(s: &Shared) {
    loop {
        let job = {
            let mut q = s.queue.lock().unwrap_or_else(|p| p.into_inner());
            loop {
                if q.closed {
                    return;
                }
                if let Some(job) = q.jobs.pop_front() {
                    break job;
                }
                q = s.wake.wait(q).unwrap_or_else(|p| p.into_inner());
            }
        };
        let done = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match job {
            Job::Load(content) => nettai_battle::behavior::Behaviors::for_content(&content).map(|_| None).map_err(|e| e.to_string()),
            Job::Ahead(a) => Ok(Some((a.n, a.search.ahead(&a.battle, a.side, a.k, a.held, &a.logits, a.luck)))),
        }));
        let answer = match done {
            Ok(Ok(None)) => continue,
            Ok(Ok(Some((n, a)))) => (n, Ok(a)),
            Ok(Err(why)) => (0, Err(format!("the lookahead's runtime wouldn't load: {why}"))),
            Err(_) => (0, Err("the lookahead panicked".into())),
        };
        let stopped = answer.1.is_err();
        *s.answer.lock().unwrap_or_else(|p| p.into_inner()) = Some(answer);
        if stopped {
            return;
        }
    }
}
