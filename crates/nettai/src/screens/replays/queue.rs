//! The play queue (docs/design/app-ui.md §2.91, §2.92; docs/app.md §16): replays lined up on the Replays screen to
//! be watched one after another, a game's own. It is kept while the app runs and never saved (the user: "playlists
//! should be ephemeral, i don't want them to save them").
//!
//! It is what is still to watch (the user: "the replay queue should remove replays when they're done playing"):
//! while it plays, its first entry, the head, is the replay watched, and leaves once it is done: played to its end,
//! or passed by a skip to the next. Watching from an entry passes the ones before it, which leave too. Back
//! part way through a replay leaves it at the head, to be watched first the next time.
//!
//! An entry is a replay's file. One whose file is gone (deleted since it was added) or that no longer plays here
//! stays in the queue, shown as missing, until watching comes to it: it is passed by, and leaves as a replay played
//! past does. Whether each entry plays is the screen's to say (`plays`, from its rows), not the queue's.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Queue {
    pub paths: Vec<PathBuf>,
}

impl Queue {
    /// `path` at the end (Add to queue).
    pub fn add(&mut self, path: &Path) {
        self.paths.push(path.to_path_buf());
    }

    /// `path` at the start, what plays next (Play next).
    pub fn add_first(&mut self, path: &Path) {
        self.paths.insert(0, path.to_path_buf());
    }

    /// Entry `entry` taken out.
    pub fn remove(&mut self, entry: usize) {
        if entry < self.paths.len() {
            self.paths.remove(entry);
        }
    }

    /// Entry `entry` moved one place up (`by` -1) or down (1): its new place, or none at an end.
    pub fn shift(&mut self, entry: usize, by: i32) -> Option<usize> {
        let to = entry.checked_add_signed(by as isize).filter(|&to| to < self.paths.len() && entry < self.paths.len())?;
        self.paths.swap(entry, to);
        Some(to)
    }

    pub fn clear(&mut self) {
        *self = Queue::default();
    }

    /// The first entry from `from` on that plays (`plays`: each entry's), if any.
    pub fn first_from(from: usize, plays: &[bool]) -> Option<usize> {
        (from..plays.len()).find(|&at| plays[at])
    }

    /// Watch from entry `from`: the first entry from it on that plays becomes the head, and the ones before it
    /// leave (passed by: those before `from`, and the missing ones after it). Its file; none, and nothing changed,
    /// where none from `from` on plays.
    pub fn start(&mut self, from: usize, plays: &[bool]) -> Option<PathBuf> {
        let at = Queue::first_from(from, plays)?;
        self.paths.drain(..at);
        self.paths.first().cloned()
    }

    /// The head is done (played to its end, or skipped): it leaves.
    pub fn done(&mut self) {
        self.remove(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue(names: &[&str]) -> Queue {
        let mut q = Queue::default();
        for name in names {
            q.add(Path::new(name));
        }
        q
    }

    fn names(q: &Queue) -> Vec<&str> {
        q.paths.iter().map(|p| p.to_str().unwrap()).collect()
    }

    /// Add to queue puts a replay at the end, Play next at the start; an entry is removed, or moved up
    /// or down but not past an end; Clear empties it.
    #[test]
    fn entries_are_added_removed_moved_and_cleared() {
        let mut q = queue(&["a", "b", "c"]);
        q.add_first(Path::new("x"));
        assert_eq!(names(&q), ["x", "a", "b", "c"]);
        // (A replay may be in it twice.)
        q.add(Path::new("a"));
        assert_eq!(names(&q), ["x", "a", "b", "c", "a"]);

        assert_eq!(q.shift(2, -1), Some(1));
        assert_eq!(names(&q), ["x", "b", "a", "c", "a"]);
        assert_eq!(q.shift(0, -1), None);
        assert_eq!(q.shift(4, 1), None);
        assert_eq!(q.shift(0, 1), Some(1));
        assert_eq!(names(&q), ["b", "x", "a", "c", "a"]);

        q.remove(3);
        assert_eq!(names(&q), ["b", "x", "a", "a"]);
        q.remove(9);
        assert_eq!(q.paths.len(), 4);

        q.clear();
        assert_eq!(q, Queue::default());
    }

    /// Watching is consumed: the head plays and leaves when done; watching from an entry passes the ones before
    /// it, and missing entries are passed by and leave as they are reached; past the end nothing plays.
    #[test]
    fn played_replays_leave_the_queue() {
        let mut q = queue(&["a", "b", "c", "d"]);
        let all = [true; 4];
        // WATCH: the head plays; done, it leaves, and the next is the head.
        assert_eq!(q.start(0, &all).as_deref(), Some(Path::new("a")));
        assert_eq!(names(&q), ["a", "b", "c", "d"]);
        q.done();
        assert_eq!(q.start(0, &all[1..]).as_deref(), Some(Path::new("b")));
        // A skip to the next: the head is done, then the next plays.
        q.done();
        assert_eq!(names(&q), ["c", "d"]);
        // Watch from here on the second entry: the first is passed and leaves.
        assert_eq!(q.start(1, &[true, true]).as_deref(), Some(Path::new("d")));
        assert_eq!(names(&q), ["d"]);
        q.done();
        assert_eq!(q.start(0, &[]), None);
        assert!(q.paths.is_empty());
    }

    /// Missing entries are passed by and leave once reached; where nothing from the entry asked for plays, the
    /// queue is left as it was.
    #[test]
    fn missing_entries_are_skipped_and_leave() {
        let plays = [false, true, false, false, true, true, false];
        assert_eq!(Queue::first_from(0, &plays), Some(1));
        assert_eq!(Queue::first_from(2, &plays), Some(4));
        assert_eq!(Queue::first_from(6, &plays), None);

        let mut q = queue(&["m1", "a", "m2", "m3", "b", "c", "m4"]);
        assert_eq!(q.start(0, &plays).as_deref(), Some(Path::new("a")));
        assert_eq!(names(&q), ["a", "m2", "m3", "b", "c", "m4"]);
        q.done();
        assert_eq!(q.start(0, &plays[2..]).as_deref(), Some(Path::new("b")));
        assert_eq!(names(&q), ["b", "c", "m4"]);
        q.done();
        q.done();
        // (Only a missing one is left: nothing plays, and it stays.)
        assert_eq!(q.start(0, &[false]), None);
        assert_eq!(names(&q), ["m4"]);
    }
}
