//! A video's frames presented on threads of their own (the user: "is replay export slower than it should be?"):
//! the battle plays in order on the thread that makes the video, which draws each tick's frame at 240x160
//! (`Viewer::frame_view`) and takes its sound; the frame's picture at the video's size (the scaling, the enhanced
//! mode's text over it: `nettai_render::present::present`, the window's own) and its RGB bytes are made on one of a
//! few threads, each with a text renderer of its own, a whole frame each; and they come back in the order they were
//! played.
//!
//! - **The threads** are the platform's (`nettai_platform::thread`: a Web Worker in a browser), as many as the
//!   cores but one, at most [`MOST`] (two where the system doesn't say: a browser), one at least. Each says it has
//!   started; a video whose threads don't all start within [`STARTING`] seconds isn't made ([`Error::Threads`], as
//!   one whose ffmpeg doesn't start isn't). They take the frames from one queue, the oldest first, and end once it is
//!   let go (the video made, canceled or failed), each after the frame it is on.
//! - **At most a few frames are under way** ([`Presenters::ahead`]): those being presented, and those done waiting
//!   for the ones before them. Their pictures' buffers go round: a frame's, once written, is a next one's
//!   ([`Presenters::give_back`]).
//! - **The same picture.** Each thread's text renderer is the viewer's like (`Viewer::text_renderer`), its font with
//!   nothing made, and what it makes (its layouts, its glyphs, its drawings at the video's size) depends on nothing
//!   but the frames (nettai-render's `a_renderer_draws_a_frame_the_same_whatever_it_drew_before`), so a frame's
//!   picture is the same whichever thread presents it.

use crate::Error;
use nettai_render::render::Frame;
use nettai_render::vfont::TextRenderer;
use std::collections::BTreeMap;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};

/// The most threads presenting a video's frames.
pub(crate) const MOST: usize = 4;

/// How long the threads have to start, in seconds (a browser's load the page's module each).
pub(crate) const STARTING: u64 = 20;

/// A frame to present: its place in the video, the frame of each console shown (from the left), the sound that
/// goes with it (each track's), and the buffer its picture's RGB bytes go in.
struct Job {
    index: u64,
    frames: Vec<Frame>,
    sound: Vec<Vec<[f32; 2]>>,
    rgb: Vec<u8>,
}

/// A frame presented: its picture's RGB bytes (the consoles' rows side by side) and its sound.
pub(crate) struct Presented {
    index: u64,
    pub rgb: Vec<u8>,
    pub sound: Vec<Vec<[f32; 2]>>,
}

/// What a thread says: that it started, a frame it presented, or that it stopped (its presenting panicked).
enum Said {
    Started,
    Presented(Presented),
    Stopped(String),
}

/// What presents frames, on a thread: a text renderer of its own, and each console's picture, `width` by `height`.
struct Presenter {
    text: Option<TextRenderer>,
    pictures: Vec<Vec<u32>>,
    width: usize,
    height: usize,
}

impl Presenter {
    /// The job's frames presented, each console's picture at the video's size, and their rows side by side as
    /// RGB bytes in the job's buffer.
    fn present(&mut self, job: Job) -> Presented {
        let Job { index, frames, sound, mut rgb } = job;
        let (w, h) = (self.width, self.height);
        for (frame, picture) in frames.iter().zip(&mut self.pictures) {
            nettai_render::present::present(frame, self.text.as_mut(), picture, w, h);
        }
        let pictures = &self.pictures[..frames.len()];
        let row = 3 * w * pictures.len();
        if rgb.len() != row * h {
            rgb.clear();
            rgb.resize(row * h, 0);
        }
        for (y, line) in rgb.chunks_exact_mut(row).enumerate() {
            for (part, picture) in line.chunks_exact_mut(3 * w).zip(pictures) {
                for (o, &c) in part.chunks_exact_mut(3).zip(&picture[y * w..(y + 1) * w]) {
                    o.copy_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8]);
                }
            }
        }
        Presented { index, rgb, sound }
    }

    /// A thread's work: frames from `queue` presented and told on `said`, till the queue is let go (or what it was
    /// told to is).
    fn work(mut self, queue: Arc<Mutex<Receiver<Job>>>, said: Sender<Said>) {
        if said.send(Said::Started).is_err() {
            return;
        }
        loop {
            // (Holding the queue while waiting for a frame: the others wait for it to be free.)
            let job = match queue.lock() {
                Ok(queue) => queue.recv(),
                Err(_) => return,
            };
            let Ok(job) = job else { return };
            let told = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.present(job))) {
                Ok(presented) => said.send(Said::Presented(presented)),
                Err(_) => {
                    let _ = said.send(Said::Stopped("a frame's presenting panicked".into()));
                    return;
                }
            };
            if told.is_err() {
                return;
            }
        }
    }
}

/// The frames being presented on the threads.
pub(crate) struct Presenters {
    /// The frames to present, to the threads.
    jobs: Sender<Job>,
    /// What the threads say, and the frames done before those ahead of them, by place.
    said: Receiver<Said>,
    waiting: BTreeMap<u64, Presented>,
    /// The next frame's place, and the next to give back.
    sent: u64,
    next: u64,
    threads: usize,
    /// Buffers of pictures written, for the next frames'.
    spare: Vec<Vec<u8>>,
}

impl Presenters {
    /// The threads presenting frames `width` by `height` (each console's), `sides` consoles side by side, their text
    /// drawn by renderers like `text` (none: the frames carry their own), started: or why they weren't.
    pub(crate) fn start(text: Option<TextRenderer>, width: usize, height: usize, sides: usize) -> Result<Presenters, Error> {
        let threads = std::thread::available_parallelism().map_or(2, |n| n.get().saturating_sub(1)).clamp(1, MOST);
        let (jobs, taken) = mpsc::channel::<Job>();
        let queue = Arc::new(Mutex::new(taken));
        let (tell, said) = mpsc::channel();
        for n in 0..threads {
            let (queue, tell) = (queue.clone(), tell.clone());
            let presenter = Presenter { text: text.as_ref().map(TextRenderer::like), pictures: vec![vec![0u32; width * height]; sides], width, height };
            nettai_platform::thread::spawn(&format!("video-frames-{n}"), move || presenter.work(queue, tell))
                .map_err(|e| Error::Threads(format!("a thread didn't start: {e}")))?;
        }
        drop(tell);
        // (Each says it started: a browser's start a moment later, or not at all.)
        let waited = nettai_platform::time::Instant::now();
        let mut started = 0;
        while started < threads {
            match said.try_recv() {
                Ok(_) => started += 1,
                Err(TryRecvError::Disconnected) => return Err(Error::Threads("a thread ended as it started".into())),
                Err(TryRecvError::Empty) if waited.elapsed().as_secs() >= STARTING => {
                    return Err(Error::Threads(format!("{} of {threads} threads started in {STARTING} seconds", started)));
                }
                Err(TryRecvError::Empty) => std::thread::sleep(std::time::Duration::from_millis(1)),
            }
        }
        Ok(Presenters { jobs, said, waiting: BTreeMap::new(), sent: 0, next: 0, threads, spare: Vec::new() })
    }

    /// How many frames may be under way at once: a frame a thread, and two more (one presented while the playing
    /// thread plays the next, one written).
    pub(crate) fn ahead(&self) -> usize {
        self.threads + 2
    }

    /// The frames sent and not yet taken back.
    pub(crate) fn under_way(&self) -> usize {
        (self.sent - self.next) as usize
    }

    /// A frame to present: the frame of each console shown (from the left), and the sound that goes with it.
    pub(crate) fn send(&mut self, frames: Vec<Frame>, sound: Vec<Vec<[f32; 2]>>) {
        let job = Job { index: self.sent, frames, sound, rgb: self.spare.pop().unwrap_or_default() };
        self.sent += 1;
        // (The threads gone, the frame is missed by `next`, which says why.)
        let _ = self.jobs.send(job);
    }

    /// The next frame presented, in the order they were sent, once it is: or why it won't be.
    pub(crate) fn next(&mut self) -> Result<Presented, Error> {
        assert!(self.next < self.sent, "a frame taken back that wasn't sent");
        loop {
            if let Some(presented) = self.waiting.remove(&self.next) {
                self.next += 1;
                return Ok(presented);
            }
            match self.said.recv() {
                Ok(Said::Presented(presented)) => {
                    self.waiting.insert(presented.index, presented);
                }
                Ok(Said::Started) => {}
                Ok(Said::Stopped(why)) => return Err(Error::Threads(why)),
                Err(_) => return Err(Error::Threads("the threads ended".into())),
            }
        }
    }

    /// A buffer of a picture written (or any the stream gave back), for a next frame's.
    pub(crate) fn give_back(&mut self, rgb: Vec<u8>) {
        if rgb.capacity() > 0 && self.spare.len() < self.ahead() {
            self.spare.push(rgb);
        }
    }
}
