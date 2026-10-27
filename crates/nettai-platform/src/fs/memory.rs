//! The browser's files: one tree in memory, behind a lock every thread shares, with `std::fs`'s calls and errors.
//!
//! A path is taken from the tree's root whatever it starts with (`a/b` is `/a/b`); `.` and `..` are resolved. A
//! file written through [`File`] is in the tree once it is flushed or dropped. What changes under the folder that
//! [`watch`] names is kept for [`changes`], in order: the page writes it to its own storage.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::{self, ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use web_time::SystemTime;

/// A change to a file or folder under the folder kept.
#[derive(Clone, Debug)]
pub enum Change {
    /// The file at the path has these bytes now.
    Write(PathBuf, Arc<Vec<u8>>),
    /// The file or folder at the path is gone, and everything in it.
    Remove(PathBuf),
}

enum Node {
    Dir,
    File { data: Arc<Vec<u8>>, modified: SystemTime },
}

struct Tree {
    nodes: BTreeMap<PathBuf, Node>,
    watched: Option<PathBuf>,
    changes: Vec<Change>,
}

static ROOT: Node = Node::Dir;

static TREE: Mutex<Tree> = Mutex::new(Tree { nodes: BTreeMap::new(), watched: None, changes: Vec::new() });

fn tree() -> MutexGuard<'static, Tree> {
    crate::thread::lock(&TREE).unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// `path` from the root, its `.` and `..` resolved.
fn key(path: &Path) -> PathBuf {
    let mut out = PathBuf::from("/");
    for part in path.components() {
        match part {
            Component::Normal(name) => out.push(name),
            Component::ParentDir => {
                out.pop();
            }
            Component::RootDir | Component::CurDir | Component::Prefix(_) => {}
        }
    }
    out
}

fn error(kind: ErrorKind, path: &Path) -> io::Error {
    io::Error::new(kind, format!("{}: {kind}", path.display()))
}

/// The paths under `dir` (not `dir` itself), deepest last.
fn under<'a>(nodes: &'a BTreeMap<PathBuf, Node>, dir: &'a Path) -> impl Iterator<Item = &'a PathBuf> + 'a {
    nodes.keys().filter(move |p| p.starts_with(dir) && p.as_path() != dir)
}

impl Tree {
    fn get(&self, path: &Path) -> Option<&Node> {
        if path == Path::new("/") {
            return Some(&ROOT);
        }
        self.nodes.get(path)
    }

    fn is_dir(&self, path: &Path) -> bool {
        matches!(self.get(path), Some(Node::Dir))
    }

    /// Keep `change` if it is under the folder kept.
    fn changed(&mut self, change: Change) {
        let path = match &change {
            Change::Write(p, _) | Change::Remove(p) => p,
        };
        if self.watched.as_ref().is_some_and(|w| path.starts_with(w)) {
            self.changes.push(change);
        }
    }

    /// `path`'s folder must be there.
    fn parent(&self, path: &Path) -> io::Result<()> {
        match path.parent() {
            Some(parent) if !self.is_dir(parent) => Err(error(ErrorKind::NotFound, parent)),
            _ => Ok(()),
        }
    }

    fn put(&mut self, path: PathBuf, data: Arc<Vec<u8>>) -> io::Result<()> {
        self.parent(&path)?;
        if self.is_dir(&path) {
            return Err(error(ErrorKind::IsADirectory, &path));
        }
        self.nodes.insert(path.clone(), Node::File { data: data.clone(), modified: SystemTime::now() });
        self.changed(Change::Write(path, data));
        Ok(())
    }

    fn mkdirs(&mut self, path: &Path) -> io::Result<()> {
        let mut at = PathBuf::from("/");
        for part in path.components().skip(1) {
            at.push(part);
            match self.nodes.get(&at) {
                Some(Node::Dir) => {}
                Some(Node::File { .. }) => return Err(error(ErrorKind::NotADirectory, &at)),
                None => {
                    self.nodes.insert(at.clone(), Node::Dir);
                }
            }
        }
        Ok(())
    }

    fn remove_all(&mut self, path: &Path) {
        let gone: Vec<PathBuf> = under(&self.nodes, path).cloned().collect();
        for p in gone {
            self.nodes.remove(&p);
        }
        self.nodes.remove(path);
        self.changed(Change::Remove(path.to_path_buf()));
    }
}

/// Put a file in the tree, its folders made, as it was kept: not a change.
pub fn preload(path: impl AsRef<Path>, data: Vec<u8>) {
    let path = key(path.as_ref());
    let mut tree = tree();
    if let Some(parent) = path.parent() {
        let _ = tree.mkdirs(parent);
    }
    tree.nodes.insert(path, Node::File { data: Arc::new(data), modified: SystemTime::now() });
}

/// Keep what changes under `dir` from here on, for [`changes`].
pub fn watch(dir: impl AsRef<Path>) {
    tree().watched = Some(key(dir.as_ref()));
}

/// The folder whose changes are kept ([`watch`]), if one is.
pub fn watched() -> Option<PathBuf> {
    tree().watched.clone()
}

/// What changed under the folder kept since the last call, in order.
pub fn changes() -> Vec<Change> {
    std::mem::take(&mut tree().changes)
}

pub fn read(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    let path = key(path.as_ref());
    // (The bytes are copied once the tree is let go.)
    let data = match tree().get(&path) {
        Some(Node::File { data, .. }) => data.clone(),
        Some(Node::Dir) => return Err(error(ErrorKind::IsADirectory, &path)),
        None => return Err(error(ErrorKind::NotFound, &path)),
    };
    Ok(data.to_vec())
}

pub fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
    String::from_utf8(read(path)?).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))
}

pub fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
    tree().put(key(path.as_ref()), Arc::new(contents.as_ref().to_vec()))
}

pub fn create_dir(path: impl AsRef<Path>) -> io::Result<()> {
    let path = key(path.as_ref());
    let mut tree = tree();
    if tree.get(&path).is_some() {
        return Err(error(ErrorKind::AlreadyExists, &path));
    }
    tree.parent(&path)?;
    tree.nodes.insert(path, Node::Dir);
    Ok(())
}

pub fn create_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    tree().mkdirs(&key(path.as_ref()))
}

pub fn remove_file(path: impl AsRef<Path>) -> io::Result<()> {
    let path = key(path.as_ref());
    let mut tree = tree();
    match tree.get(&path) {
        Some(Node::File { .. }) => {
            tree.remove_all(&path);
            Ok(())
        }
        Some(Node::Dir) => Err(error(ErrorKind::IsADirectory, &path)),
        None => Err(error(ErrorKind::NotFound, &path)),
    }
}

pub fn remove_dir(path: impl AsRef<Path>) -> io::Result<()> {
    let path = key(path.as_ref());
    let mut tree = tree();
    match tree.get(&path) {
        Some(Node::Dir) if under(&tree.nodes, &path).next().is_some() => Err(error(ErrorKind::DirectoryNotEmpty, &path)),
        Some(Node::Dir) => {
            tree.remove_all(&path);
            Ok(())
        }
        Some(Node::File { .. }) => Err(error(ErrorKind::NotADirectory, &path)),
        None => Err(error(ErrorKind::NotFound, &path)),
    }
}

pub fn remove_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    let path = key(path.as_ref());
    let mut tree = tree();
    match tree.get(&path) {
        Some(Node::Dir) => {
            tree.remove_all(&path);
            Ok(())
        }
        Some(Node::File { .. }) => Err(error(ErrorKind::NotADirectory, &path)),
        None => Err(error(ErrorKind::NotFound, &path)),
    }
}

/// A file or a folder moved, as `std::fs::rename` does: a file in the way is replaced, and so is an empty folder.
pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    let (from, to) = (key(from.as_ref()), key(to.as_ref()));
    let mut tree = tree();
    let from_dir = match tree.get(&from) {
        Some(node) => matches!(node, Node::Dir),
        None => return Err(error(ErrorKind::NotFound, &from)),
    };
    if from == to {
        return Ok(());
    }
    if to.starts_with(&from) {
        return Err(error(ErrorKind::InvalidInput, &to));
    }
    tree.parent(&to)?;
    match tree.get(&to) {
        Some(Node::Dir) if !from_dir => return Err(error(ErrorKind::IsADirectory, &to)),
        Some(Node::Dir) if under(&tree.nodes, &to).next().is_some() => return Err(error(ErrorKind::DirectoryNotEmpty, &to)),
        Some(Node::File { .. }) if from_dir => return Err(error(ErrorKind::NotADirectory, &to)),
        Some(_) => tree.remove_all(&to),
        None => {}
    }
    let moved: Vec<PathBuf> = std::iter::once(from.clone()).chain(under(&tree.nodes, &from).cloned()).collect();
    for old in moved {
        let node = tree.nodes.remove(&old).expect("listed above");
        let new = to.join(old.strip_prefix(&from).expect("under it"));
        if let Node::File { data, .. } = &node {
            tree.changed(Change::Write(new.clone(), data.clone()));
        }
        tree.nodes.insert(new, node);
    }
    tree.changed(Change::Remove(from));
    Ok(())
}

pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<u64> {
    let data = read(from)?;
    let len = data.len() as u64;
    write(to, data)?;
    Ok(len)
}

/// What a path is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileType {
    dir: bool,
}

impl FileType {
    pub fn is_dir(&self) -> bool {
        self.dir
    }
    pub fn is_file(&self) -> bool {
        !self.dir
    }
    pub fn is_symlink(&self) -> bool {
        false
    }
}

/// A path's kind, size and time written.
#[derive(Clone, Debug)]
pub struct Metadata {
    kind: FileType,
    len: u64,
    modified: Option<SystemTime>,
}

impl Metadata {
    pub fn file_type(&self) -> FileType {
        self.kind
    }
    pub fn is_dir(&self) -> bool {
        self.kind.is_dir()
    }
    pub fn is_file(&self) -> bool {
        self.kind.is_file()
    }
    pub fn len(&self) -> u64 {
        self.len
    }
    pub fn modified(&self) -> io::Result<SystemTime> {
        self.modified.ok_or_else(|| io::Error::new(ErrorKind::Unsupported, "a folder has no time written"))
    }
}

fn meta(node: &Node) -> Metadata {
    match node {
        Node::Dir => Metadata { kind: FileType { dir: true }, len: 0, modified: None },
        Node::File { data, modified } => Metadata { kind: FileType { dir: false }, len: data.len() as u64, modified: Some(*modified) },
    }
}

pub fn metadata(path: impl AsRef<Path>) -> io::Result<Metadata> {
    let path = key(path.as_ref());
    tree().get(&path).map(meta).ok_or_else(|| error(ErrorKind::NotFound, &path))
}

/// A folder's entry.
#[derive(Clone, Debug)]
pub struct DirEntry {
    path: PathBuf,
    meta: Metadata,
}

impl DirEntry {
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }
    pub fn file_name(&self) -> OsString {
        self.path.file_name().unwrap_or_default().to_os_string()
    }
    pub fn file_type(&self) -> io::Result<FileType> {
        Ok(self.meta.kind)
    }
    pub fn metadata(&self) -> io::Result<Metadata> {
        Ok(self.meta.clone())
    }
}

/// A folder's entries, as they were when it was read.
pub struct ReadDir(std::vec::IntoIter<DirEntry>);

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(Ok)
    }
}

pub fn read_dir(path: impl AsRef<Path>) -> io::Result<ReadDir> {
    let path = key(path.as_ref());
    let tree = tree();
    match tree.get(&path) {
        Some(Node::Dir) => {}
        Some(Node::File { .. }) => return Err(error(ErrorKind::NotADirectory, &path)),
        None => return Err(error(ErrorKind::NotFound, &path)),
    }
    let entries = under(&tree.nodes, &path)
        .filter(|p| p.parent() == Some(path.as_path()))
        .map(|p| DirEntry { path: p.clone(), meta: meta(&tree.nodes[p]) })
        .collect::<Vec<_>>();
    Ok(ReadDir(entries.into_iter()))
}

/// How a file is opened.
#[derive(Clone, Debug, Default)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
    create_new: bool,
}

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions::default()
    }
    pub fn read(&mut self, on: bool) -> &mut Self {
        self.read = on;
        self
    }
    pub fn write(&mut self, on: bool) -> &mut Self {
        self.write = on;
        self
    }
    pub fn append(&mut self, on: bool) -> &mut Self {
        self.append = on;
        self
    }
    pub fn truncate(&mut self, on: bool) -> &mut Self {
        self.truncate = on;
        self
    }
    pub fn create(&mut self, on: bool) -> &mut Self {
        self.create = on;
        self
    }
    pub fn create_new(&mut self, on: bool) -> &mut Self {
        self.create_new = on;
        self
    }

    pub fn open(&self, path: impl AsRef<Path>) -> io::Result<File> {
        let path = key(path.as_ref());
        let writes = self.write || self.append;
        let mut tree = tree();
        let data = match tree.get(&path) {
            Some(Node::Dir) => return Err(error(ErrorKind::IsADirectory, &path)),
            Some(Node::File { .. }) if self.create_new => return Err(error(ErrorKind::AlreadyExists, &path)),
            Some(Node::File { data, .. }) => data.clone(),
            None if writes && (self.create || self.create_new) => {
                tree.parent(&path)?;
                let empty = Arc::new(Vec::new());
                tree.put(path.clone(), empty.clone())?;
                empty
            }
            None => return Err(error(ErrorKind::NotFound, &path)),
        };
        let data = if writes && self.truncate { Arc::new(Vec::new()) } else { data };
        let pos = if self.append { data.len() as u64 } else { 0 };
        Ok(File { path, data, pos, writes, append: self.append, dirty: writes && self.truncate })
    }
}

/// An open file: its bytes as they were opened, and, if it writes, as written so far (in the tree once flushed or
/// dropped).
#[derive(Debug)]
pub struct File {
    path: PathBuf,
    data: Arc<Vec<u8>>,
    pos: u64,
    writes: bool,
    append: bool,
    dirty: bool,
}

impl File {
    pub fn open(path: impl AsRef<Path>) -> io::Result<File> {
        OpenOptions::new().read(true).open(path)
    }
    pub fn create(path: impl AsRef<Path>) -> io::Result<File> {
        OpenOptions::new().write(true).create(true).truncate(true).open(path)
    }
    pub fn create_new(path: impl AsRef<Path>) -> io::Result<File> {
        OpenOptions::new().write(true).create_new(true).open(path)
    }
    pub fn options() -> OpenOptions {
        OpenOptions::new()
    }
    pub fn metadata(&self) -> io::Result<Metadata> {
        Ok(Metadata { kind: FileType { dir: false }, len: self.data.len() as u64, modified: Some(SystemTime::now()) })
    }
    pub fn set_len(&mut self, len: u64) -> io::Result<()> {
        self.check_writes()?;
        Arc::make_mut(&mut self.data).resize(len as usize, 0);
        self.dirty = true;
        Ok(())
    }
    pub fn sync_all(&mut self) -> io::Result<()> {
        self.flush()
    }
    pub fn sync_data(&mut self) -> io::Result<()> {
        self.flush()
    }

    fn check_writes(&self) -> io::Result<()> {
        if self.writes { Ok(()) } else { Err(io::Error::new(ErrorKind::PermissionDenied, "the file was opened to read")) }
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let at = (self.pos as usize).min(self.data.len());
        let n = buf.len().min(self.data.len() - at);
        buf[..n].copy_from_slice(&self.data[at..at + n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.check_writes()?;
        let data = Arc::make_mut(&mut self.data);
        if self.append {
            self.pos = data.len() as u64;
        }
        let at = self.pos as usize;
        if data.len() < at {
            data.resize(at, 0);
        }
        let over = buf.len().min(data.len() - at);
        data[at..at + over].copy_from_slice(&buf[..over]);
        data.extend_from_slice(&buf[over..]);
        self.pos += buf.len() as u64;
        self.dirty = true;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.dirty {
            self.dirty = false;
            tree().put(self.path.clone(), self.data.clone())?;
        }
        Ok(())
    }
}

impl Seek for File {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let pos = match to {
            SeekFrom::Start(n) => Some(n),
            SeekFrom::End(d) => (self.data.len() as u64).checked_add_signed(d),
            SeekFrom::Current(d) => self.pos.checked_add_signed(d),
        };
        self.pos = pos.ok_or_else(|| io::Error::new(ErrorKind::InvalidInput, "a seek before the file's start"))?;
        Ok(self.pos)
    }
}

impl Drop for File {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}
