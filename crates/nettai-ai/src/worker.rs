//! Background lookahead worker thread (desktop + Wasm + mobile via `nettai_threads`).
//! Runs fight lookahead off the battle's main thread so 60fps playback stays silky smooth.

use crate::fight::{self, FightContext, FightDecision};
use nettai_battle::Content;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

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

type Answer = (u64, Result<FightDecision, String>);

#[derive(Default)]
struct Queue {
    jobs: VecDeque<Job>,
    closed: bool,
}

enum Job {
    Load(Arc<Content>),
    Decide(u64, Box<FightContext>),
}

impl Worker {
    pub fn start() -> Result<Worker, String> {
        let shared = Arc::new(Shared::default());
        let s = shared.clone();
        nettai_threads::spawn("nettai-ai", move || work(&s)).map_err(|e| format!("failed to spawn AI worker thread: {e}"))?;
        Ok(Worker { shared, asked: 0 })
    }

    fn push(&self, job: Job) {
        nettai_threads::lock(&self.shared.queue).unwrap_or_else(|p| p.into_inner()).jobs.push_back(job);
        self.shared.wake.notify_one();
    }

    /// Pre-load `content`'s Luau runtime on the worker thread at round start (while the custom screen is open).
    pub fn load(&self, content: Arc<Content>) {
        self.push(Job::Load(content));
    }

    /// Submit a fight decision job; returns its id to pass to [`Worker::wait`].
    pub fn ask(&mut self, ctx: FightContext) -> u64 {
        self.asked += 1;
        let id = self.asked;
        self.push(Job::Decide(id, Box::new(ctx)));
        id
    }

    /// Wait for job `id`'s decision.
    pub fn wait(&self, id: u64) -> Result<FightDecision, String> {
        loop {
            {
                let answer = nettai_threads::lock(&self.shared.answer).unwrap_or_else(|p| p.into_inner());
                match &*answer {
                    Some((got, res)) if *got == id => return res.clone(),
                    Some((_, Err(why))) => return Err(why.clone()),
                    _ => {}
                }
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
            Job::Decide(id, ctx) => Ok(Some((id, fight::decide(*ctx)))),
        }));
        let answer = match done {
            Ok(Ok(None)) => continue,
            Ok(Ok(Some((id, dec)))) => (id, Ok(dec)),
            Ok(Err(why)) => (0, Err(format!("AI worker runtime failed to load: {why}"))),
            Err(_) => (0, Err("AI worker panicked".into())),
        };
        let stopped = answer.1.is_err();
        *s.answer.lock().unwrap_or_else(|p| p.into_inner()) = Some(answer);
        if stopped {
            return;
        }
    }
}
