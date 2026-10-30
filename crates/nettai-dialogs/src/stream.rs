//! A file written as it is made ([`crate::save_stream`]). What writes it is the system's ([`Writing`]): a file on the
//! desktop and a phone; in the browser the page's own storage, written on its main thread and handed over as a
//! download once whole.

use crate::platform::stream::Writing;

/// A file being written as it is made ([`crate::save_stream`]), from any thread: what is written goes to it in order.
/// [`Stream::finish`] ends it whole, with what goes before it put first (a head made last: an MP4's index); dropped
/// unfinished, or [`Stream::abort`]ed, it is let go (in the browser, nothing of it is handed over).
pub struct Stream {
    writing: Writing,
    finished: bool,
}

/// How a stream ended.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ending {
    Finished,
    Aborted,
}

impl Stream {
    pub(crate) fn new(writing: Writing) -> Stream {
        Stream { writing, finished: false }
    }

    /// The file's name, as the player chose it.
    pub fn name(&self) -> &str {
        self.writing.name()
    }

    /// Write `bytes` next (taken as they are: no copy). An error: writing failed (what the system said), and the
    /// file is let go.
    pub fn send(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        self.writing.send(bytes)
    }

    /// How many bytes wait to be written (a writer far ahead waits for them).
    pub fn waiting(&self) -> usize {
        self.writing.waiting()
    }

    /// The file is whole, `head` first and then what was written: on the desktop it is made so and takes its name
    /// (an error: it couldn't be, and nothing of it is left); in the browser what waits is written and it is
    /// handed over as a download (what goes wrong from here is logged).
    pub fn finish(mut self, head: Vec<u8>) -> std::io::Result<()> {
        self.finished = true;
        self.writing.end(Ending::Finished, head)
    }

    /// The file is let go: in the browser nothing of it is handed over; on the desktop it is removed.
    pub fn abort(mut self) {
        self.finished = true;
        let _ = self.writing.end(Ending::Aborted, Vec::new());
    }
}

/// Written as any `std::io::Write` is (a writer of the app's that takes one: a video).
impl std::io::Write for Stream {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.send(bytes.to_vec()).map_err(std::io::Error::other)?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.writing.end(Ending::Aborted, Vec::new());
        }
    }
}
