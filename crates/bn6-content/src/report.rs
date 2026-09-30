//! What an import found: errors (the pack can't be built as it is),
//! warnings (it builds, but probably not as the author meant) and notes.
//! Every message names the file and says what to do about it.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Note,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    pub level: Level,
    /// The file, relative to the pack.
    pub file: String,
    pub message: String,
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub issues: Vec<Issue>,
}

impl Report {
    pub fn push(&mut self, level: Level, file: impl Into<String>, message: impl Into<String>) {
        self.issues.push(Issue { level, file: file.into(), message: message.into() });
    }

    pub fn error(&mut self, file: impl Into<String>, message: impl Into<String>) {
        self.push(Level::Error, file, message);
    }

    pub fn warn(&mut self, file: impl Into<String>, message: impl Into<String>) {
        self.push(Level::Warning, file, message);
    }

    pub fn note(&mut self, file: impl Into<String>, message: impl Into<String>) {
        self.push(Level::Note, file, message);
    }

    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.level == Level::Error)
    }

    pub fn count(&self, level: Level) -> usize {
        self.issues.iter().filter(|i| i.level == level).count()
    }
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let level = match self.level {
            Level::Note => "note",
            Level::Warning => "warning",
            Level::Error => "error",
        };
        write!(f, "{level}: {}: {}", self.file, self.message)
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for i in &self.issues {
            writeln!(f, "{i}")?;
        }
        write!(
            f,
            "{} errors, {} warnings, {} notes",
            self.count(Level::Error),
            self.count(Level::Warning),
            self.count(Level::Note)
        )
    }
}

/// A 64-bit FNV-1a hash, written as 16 hex digits: the stamps packs keep
/// to tell an untouched exported file from an edited one. (Stable across
/// builds and platforms, unlike std's hasher.)
pub fn stamp(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}
