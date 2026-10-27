//! The store of content bundles (docs/design/content-deployment.md §5): the bundles an app has, unpacked, each in
//! a folder named by its id, and a record of it beside the folder.
//!
//! ```text
//! <store>/<id>/          the bundle, unpacked (crate::bundle::unpack): bundle.toml and a folder each pack, a
//!                        content directory
//! <store>/<id>.toml      its record (`Record`): its game, line and revision, the content hash its game's index
//!                        gives it, and the one computed here
//! ```
//!
//! A bundle is unpacked whole or not at all (`bundle::unpack`), and its record written after it: a folder without a
//! record is one whose storing was cut short, and goes ([`Store::tidy`]), as does a record without its folder.
//!
//! The content hash says what a bundle plays (`Content::hash`): it is the engine's, of the content alone
//! ([`content_hash`]). A bundle is stored with
//! the hash its index gives (`Record::published`), and the hash computed here (`Record::hash`) is recorded when the
//! game is first loaded from it, which checks the two are the same.

use std::path::{Path, PathBuf};

use nettai_battle::ContentHash;
use nettai_platform::fs;
use serde::{Deserialize, Serialize};

use crate::bundle::{self, BundleId, MANIFEST, Manifest};
use crate::report::Report;

/// A record's extension: `<id>.toml`.
const RECORD: &str = "toml";

/// What the store knows of a bundle: the manifest's game, line and revision, and its content hashes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub game: String,
    pub line: String,
    pub revision: u32,
    /// The content hash its game's index gives it; none for a bundle stored without one.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "hash_hex")]
    pub published: Option<ContentHash>,
    /// The content hash computed here, once the game has been loaded from it.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "hash_hex")]
    pub hash: Option<ContentHash>,
}

/// A bundle in the store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stored {
    pub id: BundleId,
    /// Its folder: a content directory.
    pub dir: PathBuf,
    pub record: Record,
}

impl Stored {
    /// What it plays, as far as is known: the hash computed here, else its index's.
    pub fn content_hash(&self) -> Option<ContentHash> {
        self.record.hash.or(self.record.published)
    }
}

/// The store: a folder of bundles (an app's is its data folder's `content/`).
#[derive(Clone, Debug)]
pub struct Store {
    pub dir: PathBuf,
}

impl Store {
    pub fn at(dir: PathBuf) -> Store {
        Store { dir }
    }

    fn folder(&self, id: BundleId) -> PathBuf {
        self.dir.join(id.to_string())
    }

    fn record_file(&self, id: BundleId) -> PathBuf {
        self.dir.join(format!("{id}.{RECORD}"))
    }

    /// Bundle `id`, if the store has it.
    pub fn get(&self, id: BundleId) -> Option<Stored> {
        let dir = self.folder(id);
        if !fs::is_file(dir.join(MANIFEST)) {
            return None;
        }
        let text = fs::read_to_string(self.record_file(id)).ok()?;
        let record = toml::from_str(&text).ok()?;
        Some(Stored { id, dir, record })
    }

    /// Every bundle the store has, by id.
    pub fn list(&self) -> Vec<Stored> {
        let Ok(entries) = fs::read_dir(&self.dir) else { return Vec::new() };
        let mut out: Vec<Stored> = entries
            .flatten()
            .filter_map(|e| e.file_name().to_str()?.strip_suffix(&format!(".{RECORD}"))?.parse::<BundleId>().ok())
            .filter_map(|id| self.get(id))
            .collect();
        out.sort_by_key(|s| s.id);
        out
    }

    /// Game `game`'s newest bundle of engine line `line`: its highest revision.
    pub fn newest(&self, game: &str, line: &str) -> Option<Stored> {
        self.list().into_iter().filter(|s| s.record.game == game && s.record.line == line).max_by_key(|s| s.record.revision)
    }

    /// Game `game`'s newest bundle of line `line` that plays `hash`.
    pub fn playing(&self, game: &str, line: &str, hash: ContentHash) -> Option<Stored> {
        self.list()
            .into_iter()
            .filter(|s| s.record.game == game && s.record.line == line && s.content_hash() == Some(hash))
            .max_by_key(|s| s.record.revision)
    }

    /// Store gzipped bundle `gz`, whose id must be `id` (its tar's SHA-256), with the content hash its index gives
    /// it (`published`): unpacked into its folder, whole, then its record written. A bundle the store has already
    /// is left as it is. The bundle stored.
    pub fn add(&self, gz: &[u8], id: BundleId, published: Option<ContentHash>) -> Result<Stored, String> {
        if let Some(stored) = self.get(id) {
            return Ok(stored);
        }
        fs::create_dir_all(&self.dir).map_err(|e| format!("{}: {e}", self.dir.display()))?;
        let dir = self.folder(id);
        // (A folder without its record: a storing cut short, made again.)
        if fs::exists(&dir) {
            fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let manifest = bundle::unpack(gz, id, &dir)?;
        let record = Record { game: manifest.game, line: manifest.line, revision: manifest.revision, published, hash: None };
        self.write_record(id, &record)?;
        Ok(Stored { id, dir, record })
    }

    /// Record the content hash computed here for bundle `id`.
    pub fn record_hash(&self, id: BundleId, hash: ContentHash) -> Result<(), String> {
        let Some(mut stored) = self.get(id) else { return Err(format!("the store has no bundle {id}")) };
        stored.record.hash = Some(hash);
        self.write_record(id, &stored.record)
    }

    fn write_record(&self, id: BundleId, record: &Record) -> Result<(), String> {
        let path = self.record_file(id);
        let text = format!(
            "# A bundle of the store (docs/design/content-deployment.md §5): what it is, and what it plays.\n{}",
            toml::to_string(record).map_err(|e| e.to_string())?
        );
        fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Take bundle `id` out of the store: its record, then its folder.
    pub fn remove(&self, id: BundleId) -> Result<(), String> {
        let (record, dir) = (self.record_file(id), self.folder(id));
        if fs::exists(&record) {
            fs::remove_file(&record).map_err(|e| format!("{}: {e}", record.display()))?;
        }
        if fs::exists(&dir) {
            fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        Ok(())
    }

    /// What a storing cut short left: a folder being unpacked (`<id>.part`), a folder without its record, a record
    /// without its folder. What couldn't be removed, said.
    pub fn tidy(&self) -> Vec<String> {
        let Ok(entries) = fs::read_dir(&self.dir) else { return Vec::new() };
        let mut failed = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = entry.file_name().to_str().map(str::to_string) else { continue };
            let partial = name.ends_with(".part");
            let unrecorded = name.parse::<BundleId>().is_ok_and(|id| !fs::is_file(self.record_file(id)) || !fs::is_file(path.join(MANIFEST)));
            let orphan = name.strip_suffix(&format!(".{RECORD}")).and_then(|id| id.parse::<BundleId>().ok()).is_some_and(|id| !fs::is_file(self.folder(id).join(MANIFEST)));
            let gone = if fs::is_dir(&path) && (partial || unrecorded) {
                fs::remove_dir_all(&path)
            } else if orphan {
                fs::remove_file(&path)
            } else {
                continue;
            };
            if let Err(e) = gone {
                failed.push(format!("removing {}: {e}", path.display()));
            }
        }
        failed
    }
}

/// A bundle's manifest, from its folder (a stored bundle's, or any unpacked one).
pub fn manifest(dir: &Path) -> Result<Manifest, String> {
    let path = dir.join(MANIFEST);
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The content hash of game `game` of content directory `content` (a bundle's folder): the content loaded and defined
/// as a battle plays it (`crate::load::load_battle`: the content alone), hashed (`Content::hash`).
pub fn content_hash(content: &Path, game: &str) -> Result<ContentHash, Report> {
    let (content, _) = crate::load::load_battle(content, game)?;
    Ok(content.hash())
}

mod hash_hex {
    use nettai_battle::ContentHash;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(h: &Option<ContentHash>, s: S) -> Result<S::Ok, S::Error> {
        match h {
            Some(h) => s.serialize_str(&format!("{:016x}", h.0)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<ContentHash>, D::Error> {
        let s = String::deserialize(d)?;
        u64::from_str_radix(&s, 16).map(|h| Some(ContentHash(h))).map_err(|_| serde::de::Error::custom(format!("a content hash is 16 hex digits, not {s:?}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{build, gzip};

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nettai-store-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn put(dir: &Path, path: &str, text: &str) {
        let file = dir.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }

    /// A bundle of game `game` at `revision`, its one pack holding `text`.
    fn bundle_of(root: &Path, game: &str, revision: u32, text: &str) -> bundle::Bundle {
        let pack = root.join(format!("src-{game}-{revision}-{}", text.len()));
        put(&pack, "manifest.toml", &format!("id = \"{game}\"\nkind = \"game\"\n"));
        put(&pack, "init.luau", text);
        let manifest = Manifest { game: game.into(), line: "0.11".into(), revision, packs: [(game.to_string(), "c0ffee".to_string())].into() };
        build(manifest, &[(game, &pack)]).unwrap()
    }

    /// Bundles are stored by id with their records; the newest of a game's line is its highest revision; one is
    /// found by its content hash, the one computed here before its index's; and it is taken out whole.
    #[test]
    fn bundles_are_kept_by_id_and_found_by_revision_and_hash() {
        let root = scratch("keep");
        let store = Store::at(root.join("content"));
        assert!(store.list().is_empty() && store.newest("exe6", "0.11").is_none());
        let (one, two, other) = (bundle_of(&root, "exe6", 1, "return 1"), bundle_of(&root, "exe6", 2, "return 2"), bundle_of(&root, "exe5", 1, "return 1"));
        let a = store.add(&gzip(&one.tar), one.id, Some(ContentHash(0xA))).unwrap();
        assert_eq!(a.record, Record { game: "exe6".into(), line: "0.11".into(), revision: 1, published: Some(ContentHash(0xA)), hash: None });
        assert_eq!(std::fs::read_to_string(a.dir.join("exe6/init.luau")).unwrap(), "return 1");
        store.add(&gzip(&two.tar), two.id, Some(ContentHash(0xB))).unwrap();
        store.add(&gzip(&other.tar), other.id, None).unwrap();
        assert_eq!(store.list().len(), 3);
        assert_eq!(store.newest("exe6", "0.11").unwrap().id, two.id);
        assert!(store.newest("exe6", "0.12").is_none());
        assert_eq!(store.playing("exe6", "0.11", ContentHash(0xA)).unwrap().id, one.id);
        // (Stored again, it is the one there: nothing is written.)
        assert_eq!(store.add(&gzip(&one.tar), one.id, None).unwrap(), a);
        // The hash computed here is what it plays.
        store.record_hash(one.id, ContentHash(0xC)).unwrap();
        assert!(store.playing("exe6", "0.11", ContentHash(0xA)).is_none());
        assert_eq!(store.playing("exe6", "0.11", ContentHash(0xC)).unwrap().record.published, Some(ContentHash(0xA)));
        store.remove(two.id).unwrap();
        assert_eq!(store.newest("exe6", "0.11").unwrap().id, one.id);
        assert!(!store.dir.join(two.id.to_string()).exists() && !store.dir.join(format!("{}.toml", two.id)).exists());
        // A bundle that isn't the id it is stored as is refused, and nothing of it stays.
        let err = store.add(&gzip(&two.tar), BundleId([9; 32]), None).unwrap_err();
        assert!(err.contains("SHA-256"), "{err}");
        assert_eq!(store.list().len(), 2);
        assert_eq!(store.tidy(), Vec::<String>::new());
        assert_eq!(std::fs::read_dir(&store.dir).unwrap().count(), 4, "two bundles and their records");
        let _ = std::fs::remove_dir_all(root);
    }

    /// What a storing cut short leaves goes: a folder being unpacked, a folder without its record, a record
    /// without its folder; a whole bundle stays.
    #[test]
    fn what_a_storing_cut_short_leaves_is_tidied() {
        let root = scratch("tidy");
        let store = Store::at(root.join("content"));
        let (one, two, three) = (bundle_of(&root, "exe6", 1, "a"), bundle_of(&root, "exe6", 2, "b"), bundle_of(&root, "exe6", 3, "c"));
        store.add(&gzip(&one.tar), one.id, None).unwrap();
        store.add(&gzip(&two.tar), two.id, None).unwrap();
        store.add(&gzip(&three.tar), three.id, None).unwrap();
        std::fs::remove_file(store.dir.join(format!("{}.toml", two.id))).unwrap();
        std::fs::remove_dir_all(store.dir.join(three.id.to_string())).unwrap();
        put(&store.dir, &format!("{}.part/exe6/init.luau", three.id), "half");
        assert_eq!(store.tidy(), Vec::<String>::new());
        let mut left: Vec<String> = std::fs::read_dir(&store.dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        left.sort();
        assert_eq!(left, [one.id.to_string(), format!("{}.toml", one.id)]);
        let _ = std::fs::remove_dir_all(root);
    }
}
