//! Each game's content (docs/design/content-deployment.md §5): the content directory its matches are played from,
//! which every reader of a game's content asks here (its loading, its names, its music's and backgrounds' names,
//! its look), and where other content of it is found by its content hash (a replay's, the other player's).
//!
//! **The store.** The bundles the app has are kept unpacked in the data folder's `content/<id>/`, each with its
//! record (`nettai_content::store`: its game, line and revision, and its content hashes). A game's content is its
//! newest bundle of the app's engine line ([`line`]) in the store: the highest revision.
//!
//! **Downloads.** The app ships no content, and knows one address: the catalog of games (`bundle::catalog_url`),
//! which gives each game's index, whose entries give each bundle's address. The games are the catalog's that its
//! extractor makes packs of. A game's newest bundle of the line is downloaded where the store has none of its line,
//! the first start included; and, unless Settings turned "Check for updates" off, at each start where its index has
//! a newer one (`crate::games` runs them: [`fetch_catalog`], [`fetch_newest`]). The catalog and the indexes last
//! read are kept beside the bundles (`games.json`, `<game>.index.json`), so a start without a connection plays
//! what the store has.
//! A bundle is checked against its id as it is stored, and the content hash it plays against its index's the first
//! time it is loaded with its game's pack ([`verify`]): one that differs is taken out of the store.
//!
//! **Development.** A build run from the repository, or with `$NETTAI_CONTENT`, plays every game from that one
//! content directory (`nettai_content::index::development`), and nothing is downloaded for it; other content (a
//! replay's) is found in the store and the sites all the same.

use nettai_battle::ContentHash;
use nettai_content::bundle::{self, Catalog, Index, Published};
use nettai_content::store::{Store, Stored};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// The most a game's index may be, and a bundle.
const MAX_INDEX: u64 = 1 << 20;
const MAX_BUNDLE: u64 = 64 << 20;

/// The engine line this app plays (`0.11`): the bundles of it are its.
pub fn line() -> &'static str {
    nettai_replay::line(env!("CARGO_PKG_VERSION"))
}

/// Where the games' content is.
struct Contents {
    /// Development's content directory, which every game is played from; none: each game's newest bundle in the
    /// store.
    development: Option<PathBuf>,
    store: Store,
}

/// The process's, made the first time it is asked for.
fn contents() -> &'static Contents {
    static CONTENTS: OnceLock<Contents> = OnceLock::new();
    CONTENTS.get_or_init(|| {
        // (The tests' store is a folder of their own, never the player's.)
        let data = if cfg!(test) { std::env::temp_dir().join(format!("nettai-tests-{}", std::process::id())) } else { crate::paths::data() };
        let store = Store::at(data.join("content"));
        let development = nettai_content::index::development();
        if development.is_none() {
            for why in store.tidy() {
                crate::errors::log("tidying the content store", &why);
            }
        }
        Contents { development, store }
    })
}

/// Run `store` alone: bundles are stored one at a time, whichever thread or task stores them.
fn storing<T>(store: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    static STORING: Mutex<()> = Mutex::new(());
    let _held = nettai_platform::thread::lock(&STORING).map_err(|_| "the store's lock was poisoned".to_string())?;
    store()
}

/// Whether every game is played from development's content directory.
pub fn development() -> bool {
    contents().development.is_some()
}

/// Game `game`'s content directory, if there is content of it: development's, else its newest bundle of the line.
pub fn dir(game: &str) -> Option<PathBuf> {
    let c = contents();
    match &c.development {
        Some(dir) => nettai_content::index::games(dir).ok()?.iter().any(|g| g == game).then(|| dir.clone()),
        None => Some(c.store.newest(game, line())?.dir),
    }
}

/// The games, by id: development's content's, else the catalog's as last read that this app plays (its extractor
/// makes their packs): each one's content is in the store, or is to be downloaded.
pub fn games() -> Result<Vec<String>, String> {
    match &contents().development {
        Some(dir) => nettai_content::index::games(dir).map_err(|e| format!("{}: {e}", dir.display())),
        None => Ok(catalog_games()),
    }
}

/// Each game with its content directory: what the background pool is read from (every game's backgrounds,
/// `nettai_content::locale::background_pool_of`).
pub fn pool() -> Vec<(String, PathBuf)> {
    games().unwrap_or_default().into_iter().filter_map(|game| Some((game.clone(), dir(&game)?))).collect()
}

/// The stored bundle content directory `dir` is, if it is one.
#[allow(dead_code)] // (The lobby's, to come.)
pub fn stored_at(dir: &Path) -> Option<Stored> {
    stored_in(&contents().store, dir)
}

fn stored_in(store: &Store, dir: &Path) -> Option<Stored> {
    (dir.parent()? == store.dir).then_some(())?;
    store.get(dir.file_name()?.to_str()?.parse().ok()?)
}

/// Game `game`'s bundle of the line that plays `hash`, if the store has one.
#[allow(dead_code)] // (The replays' and the lobby's, to come.)
pub fn find(game: &str, hash: ContentHash) -> Option<Stored> {
    contents().store.playing(game, line(), hash)
}

/// The revision of game `game`'s content that plays `hash`, as far as is known: a stored bundle's, else its
/// index's (as last read).
#[allow(dead_code)] // (The lobby's, to come.)
pub fn revision_of(game: &str, hash: ContentHash) -> Option<u32> {
    find(game, hash).map(|s| s.record.revision).or_else(|| published(game, hash).map(|p| p.revision))
}

/// That the content loaded from content directory `dir`, whose hash is `hash`, is what it is published as: a
/// stored bundle's hash is its index's (where one gave it), and is recorded the first time it is loaded. One that
/// isn't is taken out of the store, and why is the error. (Development's content is what it is.)
pub fn verify(dir: &Path, hash: ContentHash) -> Result<(), String> {
    verify_in(&contents().store, dir, hash)
}

/// [`verify`], of store `store`.
fn verify_in(store: &Store, dir: &Path, hash: ContentHash) -> Result<(), String> {
    let Some(stored) = stored_in(store, dir) else { return Ok(()) };
    if let Some(published) = stored.record.published
        && published != hash
    {
        let taken = match store.remove(stored.id) {
            Ok(()) => "it is taken out of the store".to_string(),
            Err(why) => format!("taking it out of the store failed: {why}"),
        };
        return Err(format!(
            "{} r{} (bundle {}) plays {hash} with this pack, and its index says {published}: {taken}",
            stored.record.game, stored.record.revision, stored.id
        ));
    }
    if stored.record.hash != Some(hash) {
        store.record_hash(stored.id, hash)?;
    }
    Ok(())
}

// ---- The catalog and the games' indexes ---------------------------------------------------------------------

/// What was last read of the sites (the catalog, and each game's index), kept in the store's folder so that a start
/// without a connection plays what the store has: `games.json` and `<game>.index.json`.
#[derive(Default)]
struct Read {
    catalog: Option<Catalog>,
    indexes: HashMap<String, Index>,
}

/// Where the catalog read last is kept.
fn catalog_file() -> PathBuf {
    contents().store.dir.join("games.json")
}

/// Where game `game`'s index read last is kept.
fn index_file(game: &str) -> PathBuf {
    contents().store.dir.join(format!("{game}.index.json"))
}

/// What was read of the sites, as last kept, read from the store's folder the first time it is asked for.
fn read() -> &'static Mutex<Read> {
    static READ: OnceLock<Mutex<Read>> = OnceLock::new();
    READ.get_or_init(|| {
        let file = |path: PathBuf| nettai_platform::fs::read(path).ok();
        let catalog: Option<Catalog> = file(catalog_file()).and_then(|body| serde_json::from_slice(&body).ok());
        let indexes = catalog
            .iter()
            .flat_map(|c| c.games.keys())
            .filter_map(|game| Some((game.clone(), serde_json::from_slice::<Index>(&file(index_file(game))?).ok()?)))
            .collect();
        Mutex::new(Read { catalog, indexes })
    })
}

/// Keep `body` (what was read of a site) in the store's folder as `path`. (A failure is the log's: what was read is
/// used all the same.)
fn keep(path: PathBuf, body: &[u8]) {
    let written = path.parent().map_or(Ok(()), nettai_platform::fs::create_dir_all).and_then(|()| nettai_platform::fs::write(&path, body));
    if let Err(why) = written {
        crate::errors::log(&format!("keeping {}", path.display()), &why);
    }
}

/// The games of the catalog as last read, that this app plays (its extractor makes their packs), by id.
fn catalog_games() -> Vec<String> {
    let Ok(read) = nettai_platform::thread::lock(read()) else { return Vec::new() };
    read.catalog.iter().flat_map(|c| c.games.keys()).filter(|game| nettai_extract::Game::named(game).is_some()).cloned().collect()
}

/// Read the catalog of games from the organization's site (`bundle::catalog_url`, the one address the app knows),
/// and keep it: the games this app plays among them.
pub async fn fetch_catalog() -> Result<Vec<String>, String> {
    let url = bundle::catalog_url();
    let body = nettai_platform::http::get(&url, MAX_INDEX).await?;
    let catalog: Catalog = serde_json::from_slice(&body).map_err(|e| format!("{url}: a catalog that doesn't read: {e}"))?;
    keep(catalog_file(), &body);
    if let Ok(mut read) = nettai_platform::thread::lock(read()) {
        read.catalog = Some(catalog);
    }
    Ok(catalog_games())
}

/// Game `game`'s published bundle of the line that plays `hash`, in its index as last read.
#[allow(dead_code)] // (The replays' and the lobby's, to come.)
pub fn published(game: &str, hash: ContentHash) -> Option<Published> {
    nettai_platform::thread::lock(read()).ok()?.indexes.get(game)?.playing(line(), hash).cloned()
}

/// Read game `game`'s index from its site (where the catalog as last read says it is), and keep it.
pub async fn fetch_index(game: &str) -> Result<Index, String> {
    let url = {
        let read = nettai_platform::thread::lock(read()).map_err(|_| "the sites' lock was poisoned".to_string())?;
        let catalog = read.catalog.as_ref().ok_or_else(|| "no catalog of games has been read".to_string())?;
        catalog.games.get(game).map(|g| g.index.clone()).ok_or_else(|| format!("the catalog of games has no {game}"))?
    };
    let body = nettai_platform::http::get(&url, MAX_INDEX).await?;
    let index: Index = serde_json::from_slice(&body).map_err(|e| format!("{url}: an index that doesn't read: {e}"))?;
    if index.game != game {
        return Err(format!("{url}: the index of {}, not {game}", index.game));
    }
    keep(index_file(game), &body);
    if let Ok(mut read) = nettai_platform::thread::lock(read()) {
        read.indexes.insert(game.to_string(), index.clone());
    }
    Ok(index)
}

/// Published bundle `published`, from the store, else downloaded from where its index entry says (`progress` told
/// how far, 0 to 1, by its index's size), checked against its id and stored with its index's content hash, off the
/// window's thread.
pub async fn fetch(published: &Published, mut progress: impl FnMut(f32)) -> Result<Stored, String> {
    let store = contents().store.clone();
    if let Some(stored) = store.get(published.id) {
        return Ok(stored);
    }
    let url = published.url()?.to_string();
    let size = published.size.max(1) as f32;
    let gz = nettai_platform::http::get_reporting(&url, MAX_BUNDLE, |got| progress((got as f32 / size).min(1.0))).await?;
    let (id, hash) = (published.id, published.content_hash);
    nettai_platform::task::off_thread("storing content", move || storing(|| store.add(&gz, id, Some(hash))))
        .await?
        .map_err(|why| format!("{url}: {why}"))
}

/// Game `game`'s newest bundle of the line, from its site where the store hasn't it (`progress` told how far it is
/// downloaded): true if it was downloaded now, false if the store had it. An error where it can't be had, or the
/// site has no content of the line.
pub async fn fetch_newest(game: &str, progress: impl FnMut(f32)) -> Result<bool, String> {
    let index = fetch_index(game).await?;
    let Some(newest) = index.newest(line()) else {
        return Err(format!("{game}'s index has no content for nettai {}", line()));
    };
    if contents().store.get(newest.id).is_some() {
        return Ok(false);
    }
    let stored = fetch(newest, progress).await?;
    crate::errors::log(&format!("downloading {game}'s content"), &format_args!("r{} (bundle {})", stored.record.revision, stored.id));
    Ok(true)
}

/// Game `game`'s content that plays `hash`: the store's bundle, else its index's (read again where the one kept
/// hasn't it), downloaded and stored. None: no bundle of the line plays it.
#[allow(dead_code)] // (The replays' and the lobby's, to come.)
pub async fn fetch_playing(game: &str, hash: ContentHash) -> Result<Option<Stored>, String> {
    if let Some(stored) = find(game, hash) {
        return Ok(Some(stored));
    }
    let published = match published(game, hash) {
        Some(p) => p,
        None => match fetch_index(game).await?.playing(line(), hash) {
            Some(p) => p.clone(),
            None => return Ok(None),
        },
    };
    fetch(&published, |_| {}).await.map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_content::bundle::{Manifest, build, gzip};

    /// A store in a folder of its own, with a bundle of `game` holding `text`.
    fn store_with(name: &str, published: Option<ContentHash>) -> (tempfile::TempDir, Store, Stored) {
        let dir = tempfile::tempdir().unwrap();
        let pack = dir.path().join("src").join(name);
        nettai_platform::fs::create_dir_all(&pack).unwrap();
        nettai_platform::fs::write(pack.join("manifest.toml"), "id = \"exe6\"\nkind = \"game\"\n").unwrap();
        nettai_platform::fs::write(pack.join("init.luau"), name).unwrap();
        let manifest = Manifest { game: "exe6".into(), line: line().into(), revision: 1, packs: [("exe6".to_string(), "c0ffee".to_string())].into() };
        let bundle = build(manifest, &[("exe6", &pack)]).unwrap();
        let store = Store::at(dir.path().join("content"));
        let stored = store.add(&gzip(&bundle.tar), bundle.id, published).unwrap();
        (dir, store, stored)
    }

    /// A stored bundle that plays its index's hash has that hash recorded the first time it is loaded; one that
    /// plays another is taken out of the store, and why is said; content that is no stored bundle is what it is.
    #[test]
    fn a_bundle_plays_what_its_index_says() {
        let (_dir, store, stored) = store_with("good", Some(ContentHash(0xAB)));
        assert_eq!(verify_in(&store, &stored.dir, ContentHash(0xAB)), Ok(()));
        assert_eq!(store.get(stored.id).unwrap().record.hash, Some(ContentHash(0xAB)));
        let (_dir, store, stored) = store_with("bad", Some(ContentHash(0xAB)));
        let why = verify_in(&store, &stored.dir, ContentHash(0xCD)).unwrap_err();
        assert!(why.contains("00000000000000cd") || why.contains("cd"), "{why}");
        assert!(why.contains("taken out of the store"), "{why}");
        assert!(store.get(stored.id).is_none() && !stored.dir.exists());
        // (One stored without an index's hash: the hash computed is recorded.)
        let (_dir, store, stored) = store_with("unlisted", None);
        assert_eq!(verify_in(&store, &stored.dir, ContentHash(0xEF)), Ok(()));
        assert_eq!(store.get(stored.id).unwrap().content_hash(), Some(ContentHash(0xEF)));
        assert_eq!(verify_in(&store, Path::new("/somewhere/content"), ContentHash(1)), Ok(()), "development's");
    }

    /// The sites, live (`cargo test -p nettai --lib -- --ignored the_sites_give_each_games_content`): the catalog
    /// read, and each game's newest bundle of the line downloaded, checked against its id and stored (into the
    /// tests' store, a folder of their own); asked again, the store has it.
    #[test]
    #[ignore = "reads the live sites"]
    fn the_sites_give_each_games_content() {
        let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
        runtime.block_on(async {
            let games = fetch_catalog().await.unwrap();
            assert!(!games.is_empty(), "the catalog gives games this app plays");
            for game in &games {
                let mut said = Vec::new();
                fetch_newest(game, |p| said.push(p)).await.unwrap_or_else(|why| panic!("{game}: {why}"));
                let stored = contents().store.newest(game, line()).unwrap_or_else(|| panic!("{game}: nothing stored"));
                assert!(stored.record.published.is_some(), "{game}: stored with its index's hash");
                assert!(said.windows(2).all(|w| w[0] <= w[1]) && said.last().is_some_and(|p| *p > 0.99), "{game}: {said:?}");
                assert!(!fetch_newest(game, |_| {}).await.unwrap(), "{game}: the store has it");
                let hash = stored.content_hash().unwrap();
                assert_eq!(fetch_playing(game, hash).await.unwrap().map(|s| s.id), Some(stored.id), "{game}: found by its hash");
            }
        });
    }
}
