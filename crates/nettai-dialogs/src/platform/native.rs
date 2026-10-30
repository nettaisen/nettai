//! A file written as it is made on the desktop and a phone ([`crate::Stream`]): a file, at the place the player chose
//! (or the app did), written beside it as `NAME.body.part` and put after its head as `NAME.part` once it is whole,
//! which then takes the name (a file there before stays till then); removed if it is let go. Whole, it is saved as
//! `save_path`'s file is (a phone's is handed over: it is in the app's own storage, which the player doesn't see).

use crate::stream::Ending;
use crate::{Filter, Stream};
use std::path::{Path, PathBuf};

pub fn save_stream(filter: Filter, name: &str, done: Box<dyn FnOnce(Option<Stream>)>) {
    done(super::system::save_path(filter, name, None).and_then(file_stream));
}

/// Written at `path`, no dialog asked.
pub fn save_stream_to(path: &Path, done: Box<dyn FnOnce(std::io::Result<Option<Stream>>)>) {
    done(Writing::at(path.to_path_buf()).map(|writing| Some(Stream::new(writing))));
}

fn file_stream(path: PathBuf) -> Option<Stream> {
    match Writing::at(path.clone()) {
        Ok(writing) => Some(Stream::new(writing)),
        Err(why) => {
            log::warn!("saving {}: {why}", path.display());
            None
        }
    }
}

/// The file being written: its name, and (till it ends) its place, what is written (`NAME.body.part` beside it)
/// and that file.
pub struct Writing {
    name: String,
    file: Option<(PathBuf, PathBuf, std::fs::File)>,
}

impl Writing {
    fn at(path: PathBuf) -> std::io::Result<Writing> {
        if let Some(folder) = path.parent().filter(|f| !f.as_os_str().is_empty()) {
            std::fs::create_dir_all(folder)?;
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let body = path.with_file_name(format!("{name}.body.part"));
        let file = std::fs::File::create(&body)?;
        Ok(Writing { name, file: Some((path, body, file)) })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn send(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        let Some((_, _, file)) = &mut self.file else { return Err(format!("{} has ended", self.name)) };
        std::io::Write::write_all(file, &bytes).map_err(|e| e.to_string())
    }

    /// Nothing waits: what is written goes to the file as it comes.
    pub fn waiting(&self) -> usize {
        0
    }

    /// Made whole (finished: an error if it couldn't be, and nothing of it is left) and saved, or removed.
    pub fn end(&mut self, ending: Ending, head: Vec<u8>) -> std::io::Result<()> {
        let Some((path, body, file)) = self.file.take() else { return Ok(()) };
        drop(file);
        let made = if ending == Ending::Finished { whole(&path, &body, &head) } else { Ok(()) };
        let _ = std::fs::remove_file(body);
        if made.is_ok() && ending == Ending::Finished {
            super::system::saved(&path);
        }
        made
    }
}

/// The file at `path` made whole: `head`, then what was written at `body`; made beside it as `NAME.part`, which
/// takes the name once it is whole (and is removed if it can't be).
fn whole(path: &Path, body: &Path, head: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let part = path.with_file_name(format!("{}.part", path.file_name().unwrap_or_default().to_string_lossy()));
    let made = (|| {
        let mut out = std::io::BufWriter::with_capacity(1 << 20, std::fs::File::create(&part)?);
        out.write_all(head)?;
        std::io::copy(&mut std::fs::File::open(body)?, &mut out)?;
        out.into_inner().map_err(|e| e.into_error())?.sync_all()?;
        std::fs::rename(&part, path)
    })();
    if made.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    made
}

#[cfg(test)]
mod tests {
    use crate::{Filter, Stream, save_stream_to};
    use std::path::{Path, PathBuf};
    use std::io::Write;

    /// A folder of this test's own under the system's temporary one, and what it holds, by name.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nettai-dialogs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn held(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        names
    }

    fn stream(path: &Path) -> Stream {
        let made = std::rc::Rc::new(std::cell::RefCell::new(None));
        let heard = made.clone();
        save_stream_to(Filter { kind: "Videos", extensions: &["mp4"] }, path, move |s| *heard.borrow_mut() = Some(s));
        made.take().unwrap().unwrap().unwrap()
    }

    /// Finished, the file is its head and then what was written, under its name, nothing else beside it; a file
    /// there before stays till then. Let go, nothing of it is left, and the file before stays.
    #[test]
    fn a_stream_is_its_head_then_what_was_written() {
        let dir = scratch("stream");
        let path = dir.join("a video.mp4");
        std::fs::write(&path, b"before").unwrap();
        let mut s = stream(&path);
        s.write_all(b"samples ").unwrap();
        s.write_all(b"and more").unwrap();
        assert_eq!(held(&dir), ["a video.mp4", "a video.mp4.body.part"]);
        assert_eq!(std::fs::read(&path).unwrap(), b"before");
        s.finish(b"head: ".to_vec()).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"head: samples and more");
        assert_eq!(held(&dir), ["a video.mp4"]);
        let mut s = stream(&path);
        s.write_all(b"half").unwrap();
        s.abort();
        assert_eq!(held(&dir), ["a video.mp4"]);
        drop(stream(&path));
        assert_eq!((held(&dir), std::fs::read(&path).unwrap()), (vec!["a video.mp4".to_string()], b"head: samples and more".to_vec()));
        let _ = std::fs::remove_dir_all(dir);
    }
}
