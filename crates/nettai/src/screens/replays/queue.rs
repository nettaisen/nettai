//! The play queue (docs/design/app-ui.md §2.91; docs/app.md §16): replays lined up on the Replays screen to be
//! watched one after another, a game's own. It is kept while the app runs and never saved (the user: "playlists
//! should be ephemeral, i don't want them to save them").
//!
//! An entry is a replay's file. One whose file is gone (deleted since it was added) or that no longer plays here
//! stays in the queue, shown as missing, and watching passes it by: whether each entry plays is the screen's to
//! say (`plays`, from its rows), not the queue's.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Queue {
    pub paths: Vec<PathBuf>,
    /// The entry watched last (while the queue plays, the one watched): Play next puts a replay after it.
    pub at: Option<usize>,
}

impl Queue {
    /// `path` at the end.
    pub fn add(&mut self, path: &Path) {
        self.paths.push(path.to_path_buf());
    }

    /// `path` after the entry watched last (at the start where none was): its place.
    pub fn add_next(&mut self, path: &Path) -> usize {
        let to = self.at.map_or(0, |at| at + 1).min(self.paths.len());
        self.paths.insert(to, path.to_path_buf());
        to
    }

    /// Entry `entry` taken out.
    pub fn remove(&mut self, entry: usize) {
        if entry >= self.paths.len() {
            return;
        }
        self.paths.remove(entry);
        self.at = match self.at {
            Some(at) if at == entry => None,
            Some(at) if at > entry => Some(at - 1),
            at => at,
        };
    }

    /// Entry `entry` moved one place up (`by` -1) or down (1): its new place, or none at an end.
    pub fn shift(&mut self, entry: usize, by: i32) -> Option<usize> {
        let to = entry.checked_add_signed(by as isize).filter(|&to| to < self.paths.len() && entry < self.paths.len())?;
        self.paths.swap(entry, to);
        self.at = self.at.map(|at| if at == entry { to } else if at == to { entry } else { at });
        Some(to)
    }

    pub fn clear(&mut self) {
        *self = Queue::default();
    }

    /// The first entry from `from` on that plays (`plays`: each entry's), if any.
    pub fn first_from(from: usize, plays: &[bool]) -> Option<usize> {
        (from..plays.len()).find(|&at| plays[at])
    }

    /// The entry that plays `by` entries from `from` (-1 the one before, 1 the one after), the missing ones passed
    /// by; none past an end.
    pub fn beside(from: usize, by: i32, plays: &[bool]) -> Option<usize> {
        let step = by.signum() as isize;
        let (mut at, mut left) = (from as isize, by.unsigned_abs());
        while left > 0 {
            at += step;
            if at < 0 || at as usize >= plays.len() {
                return None;
            }
            if plays[at as usize] {
                left -= 1;
            }
        }
        (step != 0).then_some(at as usize)
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

    /// Add to queue puts a replay at the end, Play next after the one watched last (at the start before any was);
    /// an entry is removed, or moved up or down but not past an end, the one watched last followed; Clear empties it.
    #[test]
    fn entries_are_added_removed_moved_and_cleared() {
        let mut q = queue(&["a", "b", "c"]);
        assert_eq!(q.add_next(Path::new("x")), 0);
        assert_eq!(names(&q), ["x", "a", "b", "c"]);
        q.at = Some(2);
        assert_eq!(q.add_next(Path::new("y")), 3);
        assert_eq!(names(&q), ["x", "a", "b", "y", "c"]);
        // (A replay may be in it twice.)
        q.add(Path::new("a"));
        assert_eq!(names(&q), ["x", "a", "b", "y", "c", "a"]);

        assert_eq!(q.shift(2, -1), Some(1));
        assert_eq!((names(&q), q.at), (vec!["x", "b", "a", "y", "c", "a"], Some(1)));
        assert_eq!(q.shift(0, -1), None);
        assert_eq!(q.shift(5, 1), None);
        assert_eq!(q.shift(0, 1), Some(1));
        assert_eq!((names(&q), q.at), (vec!["b", "x", "a", "y", "c", "a"], Some(0)));

        q.remove(3);
        assert_eq!((names(&q), q.at), (vec!["b", "x", "a", "c", "a"], Some(0)));
        q.at = Some(3);
        q.remove(1);
        assert_eq!((names(&q), q.at), (vec!["b", "a", "c", "a"], Some(2)));
        q.remove(2);
        assert_eq!(q.at, None);
        q.remove(9);
        assert_eq!(q.paths.len(), 3);

        q.at = Some(1);
        q.clear();
        assert_eq!(q, Queue::default());
    }

    /// Watching starts at the first entry that plays from the one asked for; next and previous pass the missing
    /// ones by, and stop at the ends.
    #[test]
    fn missing_entries_are_skipped() {
        let plays = [false, true, false, false, true, true, false];
        assert_eq!(Queue::first_from(0, &plays), Some(1));
        assert_eq!(Queue::first_from(2, &plays), Some(4));
        assert_eq!(Queue::first_from(6, &plays), None);
        assert_eq!(Queue::beside(1, 1, &plays), Some(4));
        assert_eq!(Queue::beside(4, -1, &plays), Some(1));
        assert_eq!(Queue::beside(1, -1, &plays), None);
        assert_eq!(Queue::beside(5, 1, &plays), None);
        assert_eq!(Queue::beside(1, 2, &plays), Some(5));
        assert_eq!(Queue::first_from(0, &[false, false]), None);
    }
}
