//! Each game's content (docs/design/content-deployment.md §5): the content directory its matches are played from,
//! which every reader of a game's content asks here (its loading, its names, its music's and backgrounds' names,
//! its look), and where other content of it is found by its content hash (a replay's, the other player's).
//!
//! **The store.** The bundles the app has are kept unpacked in the data folder's `content/<id>/`, each with its
//! record (`nettai_content::store`: its game, line and revision, and its content hashes). A game's content is its
//! newest bundle of the app's engine line ([`line`]) in the store: the highest revision.
//!
//! **Downloads.** The app ships no content, and knows one address: the catalog of games (`bundle::catalog_url`),
//! which gives each game's index, whose entries give each bundle's address. The games are the catalog's: a bundle
//! of the app's line has the recipe its assets are extracted from its ROMs by. A game's newest bundle of the line is downloaded where the store has none of its line,
//! the first start included; and, unless Settings turned "Check for updates" off, at each start where its index has
//! a newer one (`crate::games` runs them: [`fetch_catalog`], [`fetch_newest`]). An online match whose players'
//! background pools differ takes every game's newest whatever Settings say ([`fetch_every_newest`], `screens::play`).
//! The catalog and the indexes last read are kept beside the bundles (`catalog.json`, `<game>.index.json`), so a start
//! without a connection plays what the store has.
//! A bundle is checked against its id as it is stored, and the content hash it plays against its index's the first
//! time it is loaded ([`verify`]): one that differs is taken out of the store.
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

/// Where the games' content is, and what was read of the sites: the process's ([`contents`]), or a test's own.
struct Contents {
    /// Development's content directory, which every game is played from; none: each game's newest bundle in the
    /// store.
    development: Option<PathBuf>,
    store: Store,
    /// The catalog's address ([`catalog_url`]).
    catalog: String,
    /// What was read of the sites, as last kept.
    read: Mutex<Read>,
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
        Contents::new(development, store, catalog_url())
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

/// The games, by id: development's content's, else the catalog's as last read: each one's content is in the store,
/// or is to be downloaded.
pub fn games() -> Result<Vec<String>, String> {
    match &contents().development {
        Some(dir) => nettai_content::index::games(dir).map_err(|e| format!("{}: {e}", dir.display())),
        None => Ok(contents().catalog_games()),
    }
}

/// Each game with its content directory: what the background pool is read from (every game's backgrounds,
/// `nettai_content::locale::background_pool_of`). Development's games, else each game's newest bundle in the store.
pub fn pool() -> Vec<(String, PathBuf)> {
    match &contents().development {
        Some(dir) => nettai_content::index::games(dir).unwrap_or_default().into_iter().map(|game| (game, dir.clone())).collect(),
        None => contents().stored_pool(),
    }
}

/// The stored bundle content directory `dir` is, if it is one.
pub fn stored_at(dir: &Path) -> Option<Stored> {
    stored_in(&contents().store, dir)
}

fn stored_in(store: &Store, dir: &Path) -> Option<Stored> {
    (dir.parent()? == store.dir).then_some(())?;
    store.get(dir.file_name()?.to_str()?.parse().ok()?)
}

/// Game `game`'s bundle of the line that plays `hash`, if the store has one.
pub fn find(game: &str, hash: ContentHash) -> Option<Stored> {
    contents().store.playing(game, line(), hash)
}

/// The revision of game `game`'s content that plays `hash`, as far as is known: a stored bundle's, else its
/// index's (as last read).
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
/// without a connection plays what the store has: `catalog.json` and `<game>.index.json`.
#[derive(Default)]
struct Read {
    catalog: Option<Catalog>,
    indexes: HashMap<String, Index>,
}

/// Keep `body` (what was read of a site) in the store's folder as `path`. (A failure is the log's: what was read is
/// used all the same.)
fn keep(path: PathBuf, body: &[u8]) {
    let written = path.parent().map_or(Ok(()), nettai_platform::fs::create_dir_all).and_then(|()| nettai_platform::fs::write(&path, body));
    if let Err(why) = written {
        crate::errors::log(&format!("keeping {}", path.display()), &why);
    }
}

/// The catalog's address: `bundle::catalog_url`, the one address the app knows; `$NETTAI_CATALOG` names another
/// (development and tests: a catalog of indexes and bundles served from elsewhere).
fn catalog_url() -> String {
    std::env::var("NETTAI_CATALOG").ok().filter(|url| !url.is_empty()).unwrap_or_else(bundle::catalog_url)
}

/// Read the catalog of games, and keep it: the games this app plays among them.
pub async fn fetch_catalog() -> Result<Vec<String>, String> {
    contents().fetch_catalog().await
}

/// Game `game`'s published bundle of the line that plays `hash`, in its index as last read.
pub fn published(game: &str, hash: ContentHash) -> Option<Published> {
    contents().published(game, hash)
}

/// Read game `game`'s index from its site (where the catalog as last read says it is), and keep it.
pub async fn fetch_index(game: &str) -> Result<Index, String> {
    contents().fetch_index(game).await
}

/// What a game's index has of the app's line: its newest bundle, downloaded now or had already; or nothing yet (no
/// content of the game is published for this line).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Newest {
    Downloaded,
    Had,
    Unpublished,
}

/// Game `game`'s newest bundle of the line, from its site where the store hasn't it (`progress` told how far it is
/// downloaded). An error where it can't be had.
pub async fn fetch_newest(game: &str, progress: impl FnMut(f32)) -> Result<Newest, String> {
    contents().fetch_newest(game, progress).await
}

/// Game `game`'s content that plays `hash`: the store's bundle, else its index's (read again where the one kept
/// hasn't it), downloaded and stored. None: no bundle of the line plays it.
pub async fn fetch_playing(game: &str, hash: ContentHash) -> Result<Option<Stored>, String> {
    contents().fetch_playing(game, hash).await
}

/// What taking every game's newest content came to ([`fetch_every_newest`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Updated {
    /// The games whose newest content was downloaded now.
    pub arrived: Vec<String>,
    /// What couldn't be had, and why: the catalog, or each game whose newest content couldn't be downloaded. None:
    /// the store has every game's newest content the sites publish for the line.
    pub failed: Option<String>,
    /// The background pool's identity (`BackgroundPool::identity`) as the store has it now: its every game's
    /// newest bundle's backgrounds, which the game is loaded with.
    pub pool: u64,
}

/// Every game's newest content of the line, whatever Settings say ("Check for updates" off too): the catalog read
/// again, then each of its games' newest bundle downloaded where the store hasn't it. Two players whose background
/// pools differ both take it (docs/design/content-deployment.md §5, "Netplay"). A game whose content can't be had
/// is said, and the others are taken all the same.
pub async fn fetch_every_newest() -> Updated {
    contents().fetch_every_newest().await
}

impl Contents {
    /// The content kept in `store`, read from the catalog at `catalog` (development's content directory, where
    /// there is one, played instead), with what was last read of the sites as the store's folder keeps it.
    fn new(development: Option<PathBuf>, store: Store, catalog: String) -> Contents {
        let file = |path: PathBuf| nettai_platform::fs::read(path).ok();
        let kept: Option<Catalog> = file(store.dir.join(CATALOG_FILE)).and_then(|body| serde_json::from_slice(&body).ok());
        let indexes = kept
            .iter()
            .flat_map(|c| c.games.keys())
            .filter_map(|game| Some((game.clone(), serde_json::from_slice::<Index>(&file(index_file(&store, game))?).ok()?)))
            .collect();
        Contents { development, store, catalog, read: Mutex::new(Read { catalog: kept, indexes }) }
    }

    /// The games of the catalog as last read, by id. (The app knows no game: a bundle of its line holds the
    /// recipe the game's assets are extracted from its ROMs by.)
    fn catalog_games(&self) -> Vec<String> {
        let Ok(read) = nettai_platform::thread::lock(&self.read) else { return Vec::new() };
        read.catalog.iter().flat_map(|c| c.games.keys()).cloned().collect()
    }

    /// Each game of the catalog as last read with its newest bundle of the line in the store: the background pool
    /// an app with a store plays ([`pool`]).
    fn stored_pool(&self) -> Vec<(String, PathBuf)> {
        self.catalog_games().into_iter().filter_map(|game| Some((game.clone(), self.store.newest(&game, line())?.dir))).collect()
    }

    /// Read the catalog of games ([`Contents::catalog`]), and keep it: its games.
    async fn fetch_catalog(&self) -> Result<Vec<String>, String> {
        let url = &self.catalog;
        let body = nettai_platform::http::get(url, MAX_INDEX).await?;
        let catalog: Catalog = serde_json::from_slice(&body).map_err(|e| format!("{url}: a catalog that doesn't read: {e}"))?;
        keep(self.store.dir.join(CATALOG_FILE), &body);
        if let Ok(mut read) = nettai_platform::thread::lock(&self.read) {
            read.catalog = Some(catalog);
        }
        Ok(self.catalog_games())
    }

    fn published(&self, game: &str, hash: ContentHash) -> Option<Published> {
        nettai_platform::thread::lock(&self.read).ok()?.indexes.get(game)?.playing(line(), hash).cloned()
    }

    async fn fetch_index(&self, game: &str) -> Result<Index, String> {
        let url = {
            let read = nettai_platform::thread::lock(&self.read).map_err(|_| "the sites' lock was poisoned".to_string())?;
            let catalog = read.catalog.as_ref().ok_or_else(|| "no catalog of games has been read".to_string())?;
            catalog.games.get(game).cloned().ok_or_else(|| format!("the catalog of games has no {game}"))?
        };
        let body = nettai_platform::http::get(&url, MAX_INDEX).await?;
        let index: Index = serde_json::from_slice(&body).map_err(|e| format!("{url}: an index that doesn't read: {e}"))?;
        if index.game != game {
            return Err(format!("{url}: the index of {}, not {game}", index.game));
        }
        keep(index_file(&self.store, game), &body);
        if let Ok(mut read) = nettai_platform::thread::lock(&self.read) {
            read.indexes.insert(game.to_string(), index.clone());
        }
        Ok(index)
    }

    /// Published bundle `published`, from the store, else downloaded from where its index entry says (`progress`
    /// told how far, 0 to 1, by its index's size), checked against its id and stored with its index's content hash,
    /// off the window's thread.
    async fn fetch(&self, published: &Published, mut progress: impl FnMut(f32)) -> Result<Stored, String> {
        let store = self.store.clone();
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

    async fn fetch_newest(&self, game: &str, progress: impl FnMut(f32)) -> Result<Newest, String> {
        let index = self.fetch_index(game).await?;
        let Some(newest) = index.newest(line()) else { return Ok(Newest::Unpublished) };
        if self.store.get(newest.id).is_some() {
            return Ok(Newest::Had);
        }
        let stored = self.fetch(newest, progress).await?;
        crate::errors::log(&format!("downloading {game}'s content"), &format_args!("r{} (bundle {})", stored.record.revision, stored.id));
        Ok(Newest::Downloaded)
    }

    async fn fetch_playing(&self, game: &str, hash: ContentHash) -> Result<Option<Stored>, String> {
        if let Some(stored) = self.store.playing(game, line(), hash) {
            return Ok(Some(stored));
        }
        let published = match self.published(game, hash) {
            Some(p) => p,
            None => match self.fetch_index(game).await?.playing(line(), hash) {
                Some(p) => p.clone(),
                None => return Ok(None),
            },
        };
        self.fetch(&published, |_| {}).await.map(Some)
    }

    async fn fetch_every_newest(&self) -> Updated {
        let (mut arrived, mut failed) = (Vec::new(), Vec::new());
        match self.fetch_catalog().await {
            Ok(games) => {
                for game in games {
                    match self.fetch_newest(&game, |_| {}).await {
                        Ok(Newest::Downloaded) => arrived.push(game),
                        Ok(Newest::Had | Newest::Unpublished) => {}
                        Err(why) => failed.push(format!("{game}'s newest content couldn't be downloaded ({why})")),
                    }
                }
            }
            Err(why) => failed.push(format!("the catalog of games couldn't be read ({why})")),
        }
        // (A few small tables, read where the game's loading reads them: each game's own language's.)
        let pool = nettai_content::locale::background_pool_of(&self.stored_pool()).identity();
        Updated { arrived, failed: (!failed.is_empty()).then(|| failed.join("; ")), pool }
    }
}

/// The catalog read last, as the store's folder keeps it.
const CATALOG_FILE: &str = "catalog.json";

/// Where game `game`'s index read last is kept in `store`'s folder.
fn index_file(store: &Store, game: &str) -> PathBuf {
    store.dir.join(format!("{game}.index.json"))
}

/// What the tests put in the store and the indexes (the tests' own: a folder of their own).
#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    /// A bundle of game `game` (one pack, holding `text`), stored as one that plays `hash`.
    pub(crate) fn store(game: &str, text: &str, hash: ContentHash) -> Stored {
        let src = contents().store.dir.with_file_name(format!("src-{game}-{}", hash.0));
        nettai_platform::fs::create_dir_all(&src).unwrap();
        nettai_platform::fs::write(src.join("manifest.toml"), format!("id = \"{game}\"\nkind = \"game\"\n")).unwrap();
        nettai_platform::fs::write(src.join("init.luau"), text).unwrap();
        let manifest = nettai_content::bundle::Manifest { game: game.into(), line: line().into(), revision: 1, packs: [(game.to_string(), "c0ffee".to_string())].into() };
        let bundle = nettai_content::bundle::build(manifest, &[(game, &src)]).unwrap();
        storing(|| contents().store.add(&nettai_content::bundle::gzip(&bundle.tar), bundle.id, Some(hash))).unwrap()
    }

    /// Game `game`'s index as last read: one bundle of the line, playing `hash`.
    pub(crate) fn list(game: &str, hash: ContentHash) {
        let published = Published { id: nettai_content::bundle::BundleId([hash.0 as u8; 32]), content_hash: hash, revision: 2, packs: Default::default(), date: "2026-10-26".into(), size: 1, url: None };
        let index = Index { game: game.into(), lines: [(line().to_string(), vec![published])].into() };
        nettai_platform::thread::lock(&contents().read).unwrap().indexes.insert(game.to_string(), index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_content::bundle::{Catalog, Manifest, build, gzip};
    use std::sync::Arc;

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

    /// Folder `root` served over HTTP on this machine, on threads of its own, as long as the tests run: its address.
    fn serve(root: PathBuf) -> String {
        use std::io::{Read as _, Write as _};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let root = root.clone();
                std::thread::spawn(move || {
                    let mut request = Vec::new();
                    let mut byte = [0u8; 1];
                    while !request.ends_with(b"\r\n\r\n") && stream.read(&mut byte).is_ok_and(|n| n == 1) {
                        request.push(byte[0]);
                    }
                    let request = String::from_utf8_lossy(&request);
                    let path = request.split_whitespace().nth(1).unwrap_or("/").trim_start_matches('/').to_string();
                    let answer = match std::fs::read(root.join(&path)) {
                        Ok(body) => [format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).into_bytes(), body].concat(),
                        Err(_) => b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
                    };
                    let _ = stream.write_all(&answer);
                });
            }
        });
        address
    }

    /// A game's content comes from where the catalog and its index say (a site of the tests' own, served here:
    /// `$NETTAI_CATALOG` names such a catalog to the app): the catalog's games this app plays; a game none of whose
    /// content is published for the line yet; the newest bundle downloaded (how far told), checked and stored with
    /// its index's hash, then had; found by its hash; and a bundle that isn't its id, or isn't there, is an error,
    /// and nothing of it is stored.
    #[test]
    fn a_games_content_comes_from_its_index() {
        let site = tempfile::tempdir().unwrap();
        let url = serve(site.path().to_path_buf());
        // (A store and sites of the test's own.)
        let store = tempfile::tempdir().unwrap();
        let contents = Contents::new(None, Store::at(store.path().join("content")), format!("{url}/catalog.json"));
        let put = |path: &str, bytes: &[u8]| {
            let file = site.path().join(path);
            nettai_platform::fs::create_dir_all(file.parent().unwrap()).unwrap();
            nettai_platform::fs::write(file, bytes).unwrap();
        };
        let bundle = |text: &str, revision: u32| {
            let src = site.path().join("src").join(text);
            nettai_platform::fs::create_dir_all(&src).unwrap();
            nettai_platform::fs::write(src.join("manifest.toml"), "id = \"exe6\"\nkind = \"game\"\n").unwrap();
            nettai_platform::fs::write(src.join("init.luau"), text).unwrap();
            let manifest = Manifest { game: "exe6".into(), line: line().into(), revision, packs: [("exe6".to_string(), "c0ffee".to_string())].into() };
            build(manifest, &[("exe6", &src)]).unwrap()
        };
        let entry = |id: nettai_content::bundle::BundleId, hash: u64, revision: u32, size: usize, url: String| Published {
            id,
            content_hash: ContentHash(hash),
            revision,
            packs: Default::default(),
            date: "2026-10-26".into(),
            size: size as u64,
            url: Some(url),
        };
        let index = |game: &str, entries: Vec<Published>| {
            let lines = if entries.is_empty() { Default::default() } else { [(line().to_string(), entries)].into() };
            put(&format!("{game}/index.json"), serde_json::to_string(&Index { game: game.into(), lines }).unwrap().as_bytes());
        };
        // exe6: one bundle; exe5: nothing for the line yet.
        let one = bundle("return 1", 1);
        let gz = gzip(&one.tar);
        put(&format!("exe6/bundles/{}.tar.gz", one.id), &gz);
        index("exe6", vec![entry(one.id, 0x6A, 1, gz.len(), format!("{url}/exe6/bundles/{}.tar.gz", one.id))]);
        index("exe5", Vec::new());
        let games = [("exe5", "exe5"), ("exe6", "exe6")].map(|(g, at)| (g.to_string(), format!("{url}/{at}/index.json")));
        put("catalog.json", serde_json::to_string(&Catalog { games: games.into(), ai: None }).unwrap().as_bytes());
        let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
        runtime.block_on(async {
            assert_eq!(contents.fetch_catalog().await.unwrap(), ["exe5", "exe6"]);
            assert_eq!(contents.fetch_newest("exe5", |_| {}).await.unwrap(), Newest::Unpublished);
            let mut said = Vec::new();
            assert_eq!(contents.fetch_newest("exe6", |p| said.push(p)).await.unwrap(), Newest::Downloaded);
            assert!(said.windows(2).all(|w| w[0] <= w[1]) && said.last() == Some(&1.0), "{said:?}");
            let stored = contents.store.get(one.id).expect("stored");
            assert_eq!((stored.record.revision, stored.record.published, stored.record.hash), (1, Some(ContentHash(0x6A)), None));
            assert_eq!(nettai_platform::fs::read_to_string(stored.dir.join("exe6/init.luau")).unwrap(), "return 1");
            assert_eq!(contents.fetch_newest("exe6", |_| {}).await.unwrap(), Newest::Had);
            assert_eq!(contents.fetch_playing("exe6", ContentHash(0x6A)).await.unwrap().map(|s| s.id), Some(one.id));
            assert_eq!(contents.fetch_playing("exe6", ContentHash(0x77)).await.unwrap(), None, "no bundle plays it");
            // r2, whose file is another bundle's: refused by its id, nothing stored.
            let (two, other) = (bundle("return 2", 2), bundle("return 3", 3));
            let other_gz = gzip(&other.tar);
            put(&format!("exe6/bundles/{}.tar.gz", two.id), &other_gz);
            index("exe6", vec![entry(one.id, 0x6A, 1, gz.len(), format!("{url}/exe6/bundles/{}.tar.gz", one.id)), entry(two.id, 0x6B, 2, other_gz.len(), format!("{url}/exe6/bundles/{}.tar.gz", two.id))]);
            let why = contents.fetch_newest("exe6", |_| {}).await.unwrap_err();
            assert!(why.contains("SHA-256"), "{why}");
            assert!(contents.store.get(two.id).is_none());
            // r3, not on the site.
            index("exe6", vec![entry(other.id, 0x6C, 3, other_gz.len(), format!("{url}/exe6/bundles/gone.tar.gz"))]);
            let why = contents.fetch_newest("exe6", |_| {}).await.unwrap_err();
            assert!(why.contains("404"), "{why}");
            assert!(contents.store.get(other.id).is_none());
        });
    }

    /// The repository's exe6 as a bundle (`cargo test -p nettai --lib -- --ignored a_real_bundle`), published on a
    /// site of the tests' own with its content hash: downloaded, stored, and loaded from the store, it plays that
    /// hash, which is recorded.
    #[test]
    #[ignore = "loads the repository's EXE6 twice"]
    fn a_real_bundle_plays_its_published_hash() {
        let repository = nettai_content::index::content();
        let manifest = Manifest { game: "exe6".into(), line: line().into(), revision: 1, packs: [("exe6".to_string(), "a".to_string()), ("exelib".to_string(), "b".to_string())].into() };
        let bundle = build(manifest, &[("exe6", &repository.join("exe6")), ("exelib", &repository.join("exelib"))]).unwrap();
        let hash = nettai_content::store::content_hash(&repository, "exe6").unwrap_or_else(|r| panic!("{r}"));
        let site = tempfile::tempdir().unwrap();
        let url = serve(site.path().to_path_buf());
        // (A store and sites of the test's own.)
        let store = tempfile::tempdir().unwrap();
        let contents = Contents::new(None, Store::at(store.path().join("content")), format!("{url}/catalog.json"));
        let gz = gzip(&bundle.tar);
        let put = |path: &str, bytes: &[u8]| {
            let file = site.path().join(path);
            nettai_platform::fs::create_dir_all(file.parent().unwrap()).unwrap();
            nettai_platform::fs::write(file, bytes).unwrap();
        };
        put(&format!("exe6/{}.tar.gz", bundle.id), &gz);
        let entry = Published { id: bundle.id, content_hash: hash, revision: 1, packs: bundle.manifest.packs.clone(), date: "2026-10-26".into(), size: gz.len() as u64, url: Some(format!("{url}/exe6/{}.tar.gz", bundle.id)) };
        put("exe6/index.json", serde_json::to_string(&Index { game: "exe6".into(), lines: [(line().to_string(), vec![entry])].into() }).unwrap().as_bytes());
        put("catalog.json", serde_json::to_string(&Catalog { games: [("exe6".to_string(), format!("{url}/exe6/index.json"))].into(), ai: None }).unwrap().as_bytes());
        let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
        runtime.block_on(async {
            contents.fetch_catalog().await.unwrap();
            assert_ne!(contents.fetch_newest("exe6", |_| {}).await.unwrap(), Newest::Unpublished);
        });
        let stored = contents.store.get(bundle.id).expect("stored");
        let computed = nettai_content::store::content_hash(&stored.dir, "exe6").unwrap_or_else(|r| panic!("{r}"));
        assert_eq!(verify_in(&contents.store, &stored.dir, computed), Ok(()));
        assert_eq!(contents.store.get(bundle.id).unwrap().record.hash, Some(hash));
    }

    /// The sites, live (`cargo test -p nettai --lib -- --ignored the_sites_give_each_games_content`): the catalog
    /// read, and each game's newest bundle of the line downloaded, checked against its id and stored (into the
    /// tests' store, a folder of their own); asked again, the store has it, and it plays its index's hash.
    #[test]
    #[ignore = "reads the live sites"]
    fn the_sites_give_each_games_content() {
        let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
        runtime.block_on(async {
            let games = fetch_catalog().await.unwrap();
            assert!(!games.is_empty(), "the catalog gives games this app plays");
            for game in &games {
                let mut said = Vec::new();
                if fetch_newest(game, |p| said.push(p)).await.unwrap_or_else(|why| panic!("{game}: {why}")) == Newest::Unpublished {
                    continue;
                }
                let stored = contents().store.newest(game, line()).unwrap_or_else(|| panic!("{game}: nothing stored"));
                assert!(stored.record.published.is_some(), "{game}: stored with its index's hash");
                assert!(said.windows(2).all(|w| w[0] <= w[1]) && said.last().is_some_and(|p| *p > 0.99), "{game}: {said:?}");
                assert_eq!(fetch_newest(game, |_| {}).await.unwrap(), Newest::Had, "{game}: the store has it");
                let hash = stored.content_hash().unwrap();
                assert_eq!(fetch_playing(game, hash).await.unwrap().map(|s| s.id), Some(stored.id), "{game}: found by its hash");
                // (The bundle plays its index's hash, as a game's first load from it checks.)
                let (dir, id) = (stored.dir.clone(), game.clone());
                let computed = nettai_platform::task::off_thread("hashing", move || nettai_content::store::content_hash(&dir, &id).map_err(|r| r.to_string())).await.unwrap();
                let computed = computed.unwrap_or_else(|why| panic!("{why}"));
                assert_eq!(verify(&stored.dir, computed), Ok(()), "{game}");
                assert_eq!(contents().store.get(stored.id).unwrap().record.hash, Some(computed));
            }
        });
    }

    // ---- Two players whose background pools differ ---------------------------------------------------------------

    /// A site of the tests' own, served here, publishing bundles that each name their backgrounds.
    struct Site {
        dir: tempfile::TempDir,
        url: String,
    }

    impl Site {
        /// A site with a catalog of `games`, whose indexes are at `<game>/index.json`.
        fn new(games: &[&str]) -> Site {
            let dir = tempfile::tempdir().unwrap();
            let url = serve(dir.path().to_path_buf());
            let site = Site { dir, url };
            let games = games.iter().map(|g| (g.to_string(), format!("{}/{g}/index.json", site.url))).collect();
            site.put("catalog.json", serde_json::to_string(&Catalog { games, ai: None }).unwrap().as_bytes());
            site
        }

        fn catalog(&self) -> String {
            format!("{}/catalog.json", self.url)
        }

        fn put(&self, path: &str, bytes: &[u8]) {
            let file = self.dir.path().join(path);
            nettai_platform::fs::create_dir_all(file.parent().unwrap()).unwrap();
            nettai_platform::fs::write(file, bytes).unwrap();
        }

        /// Game `game`'s bundle at `revision`, its content naming `backgrounds` (its own language's
        /// `[backgrounds]`, what the pool is read from), put on the site: its index entry, and the file served.
        fn bundle(&self, game: &str, revision: u32, backgrounds: &[&str]) -> (Published, String) {
            let pack = self.dir.path().join("src").join(format!("{game}-r{revision}")).join(game);
            nettai_platform::fs::create_dir_all(pack.join("locales")).unwrap();
            nettai_platform::fs::write(pack.join("manifest.toml"), format!("id = \"{game}\"\nkind = \"game\"\n")).unwrap();
            nettai_platform::fs::write(pack.join("init.luau"), format!("return {revision}")).unwrap();
            let table: String = backgrounds.iter().map(|b| format!("\"{b}\" = {{ name = \"{b}\" }}\n")).collect();
            nettai_platform::fs::write(pack.join("locales").join("en.toml"), format!("language = \"en\"\n\n[backgrounds]\n{table}")).unwrap();
            let manifest = Manifest { game: game.into(), line: line().into(), revision, packs: [(game.to_string(), "c0ffee".to_string())].into() };
            let bundle = build(manifest, &[(game, &pack)]).unwrap();
            let gz = gzip(&bundle.tar);
            let path = format!("{game}/bundles/{}.tar.gz", bundle.id);
            self.put(&path, &gz);
            let url = format!("{}/{path}", self.url);
            let entry = Published { id: bundle.id, content_hash: ContentHash(revision.into()), revision, packs: Default::default(), date: "2026-10-26".into(), size: gz.len() as u64, url: Some(url) };
            (entry, path)
        }

        /// Game `game`'s index: `entries`, of the line.
        fn index(&self, game: &str, entries: &[&Published]) {
            let lines = [(line().to_string(), entries.iter().map(|e| (*e).clone()).collect())].into();
            self.put(&format!("{game}/index.json"), serde_json::to_string(&Index { game: game.into(), lines }).unwrap().as_bytes());
        }
    }

    /// One player of an online match, for the tests: their own store, reading the site at `catalog`
    /// (`Contents`), the content they play (EXE6's, its background pool as their store gives it) and their lobby,
    /// which waits on content as the app's does.
    struct Peer {
        _store: tempfile::TempDir,
        contents: Contents,
        content: Arc<nettai_battle::Content>,
        side: nettai_match::Side,
        lobby: nettai_frontend::lobby::Lobby,
    }

    impl Peer {
        /// A peer in `role` whose store has every game's newest content `site` publishes now.
        fn new(role: nettai_frontend::lobby::Role, site: &Site, runtime: &tokio::runtime::Runtime) -> Peer {
            let store = tempfile::tempdir().unwrap();
            let contents = Contents::new(None, Store::at(store.path().join("content")), site.catalog());
            assert_eq!(runtime.block_on(contents.fetch_every_newest()).failed, None);
            let content = Peer::played(&contents);
            let side = nettai_match::Side::picked(&content, "exe6", &mut nettai_match::Picks::new(role as u32 + 1)).unwrap();
            let rulesets = nettai_match::testing::rulesets("exe6");
            let settings = nettai_frontend::lobby::Settings::new("exe6", 1, rulesets.preset("unrestricted").unwrap().clone());
            let entropy = 0x1111 * (role as u64 + 1);
            let mut lobby = nettai_frontend::lobby::Lobby::new(role, &content, rulesets, settings, side.clone(), entropy, std::time::Duration::from_secs(10));
            lobby.wait_on_content(true);
            lobby.set_ready(true);
            Peer { _store: store, contents, content, side, lobby }
        }

        /// EXE6's content, loaded with the background pool of `contents`'s store: every game's newest bundle's
        /// backgrounds (as `game::Game::load` reads them, from `pool`).
        fn played(contents: &Contents) -> Arc<nettai_battle::Content> {
            let mut content = (*nettai_match::testing::exe6_content()).clone();
            content.background_pool = nettai_content::locale::background_pool_of(&contents.stored_pool());
            Arc::new(content)
        }

        /// What the app's host does where the other's pool isn't this one's (`Play::follow_content`): every game's
        /// newest content taken; the game loaded again on it where the pool changed, the lobby playing it, the
        /// player ready again; the lobby told what couldn't be had. What the taking came to, where it was asked.
        fn follow(&mut self, runtime: &tokio::runtime::Runtime) -> Option<Updated> {
            let theirs = self.lobby.their_backgrounds()?;
            let updated = runtime.block_on(self.contents.fetch_every_newest());
            if updated.pool != self.content.background_pool.identity() {
                self.content = Peer::played(&self.contents);
                assert_eq!(self.content.background_pool.identity(), updated.pool);
                self.lobby.replace_content(&self.content, nettai_match::testing::rulesets("exe6"), self.side.clone());
                self.lobby.set_ready(true);
            }
            self.lobby.updated_backgrounds(theirs, updated.failed.clone());
            Some(updated)
        }
    }

    /// Two peers' lobbies, each hearing what the other said, every `RESEND`, their hosts following the other's
    /// pool as the app's do, until neither is still agreeing (or `rounds` have passed): what each one's takings of
    /// every game's newest content came to.
    fn meet(peers: &mut [Peer; 2], runtime: &tokio::runtime::Runtime, rounds: usize) -> [Vec<Updated>; 2] {
        use nettai_frontend::lobby::Status;
        let mut now = nettai_platform::time::Instant::now();
        let mut took = [Vec::new(), Vec::new()];
        for _ in 0..rounds {
            now += nettai_frontend::lobby::RESEND;
            for from in 0..2 {
                let _ = peers[from].lobby.poll(now);
                for d in peers[from].lobby.outgoing() {
                    peers[1 - from].lobby.receive(now, &d);
                }
            }
            for (peer, took) in peers.iter_mut().zip(&mut took) {
                took.extend(peer.follow(runtime));
            }
            if peers.iter_mut().all(|p| !matches!(p.lobby.poll(now), Status::Pending)) {
                break;
            }
        }
        // (What the last polls said reaches the other: a refusal, say.)
        for from in 0..2 {
            for d in peers[from].lobby.outgoing() {
                peers[1 - from].lobby.receive(now, &d);
            }
        }
        took
    }

    /// Two players whose background pools differ through another game's revision (the joiner took BN5's r11, the
    /// host has r10, and both play BN6's r10, of the same hash): neither is refused. Each takes every game's newest
    /// content from the sites: the host downloads BN5's r11, its game loaded again on the pool that gives, and
    /// plays the joiner's pool; the joiner, which had it, changes nothing and waits for the host. Both agree the
    /// match, ready again.
    #[test]
    fn peers_whose_backgrounds_differ_take_every_games_newest() {
        use nettai_frontend::lobby::{Role, Status};
        let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
        let site = Site::new(&["exe5", "exe6"]);
        let (exe6, _) = site.bundle("exe6", 10, &["comp", "lans-hp"]);
        site.index("exe6", &[&exe6]);
        let (old, _) = site.bundle("exe5", 10, &["acdc-area"]);
        site.index("exe5", &[&old]);
        let host = Peer::new(Role::Host, &site, &runtime);
        let (new, _) = site.bundle("exe5", 11, &["acdc-area", "airport"]);
        site.index("exe5", &[&old, &new]);
        let joiner = Peer::new(Role::Join, &site, &runtime);
        let mut peers = [host, joiner];
        assert_eq!(peers[0].content.hash(), peers[1].content.hash(), "the game's content is the same");
        let pools = peers.each_ref().map(|p| p.content.background_pool.identity());
        assert_ne!(pools[0], pools[1], "the pools differ: BN5's backgrounds");
        let [host_took, joiner_took] = meet(&mut peers, &runtime, 40);
        assert_eq!(host_took.iter().map(|u| (u.arrived.clone(), u.failed.clone(), u.pool)).collect::<Vec<_>>(), [(vec!["exe5".to_string()], None, pools[1])]);
        assert_eq!(joiner_took.iter().map(|u| (u.arrived.clone(), u.failed.clone(), u.pool)).collect::<Vec<_>>(), [(vec![], None, pools[1])], "the joiner waits");
        assert!(peers[0].content.background_pool.keys().any(|k| k == "exe5:airport"), "the host's game plays r11's backgrounds");
        for p in &mut peers {
            assert!(matches!(p.lobby.poll(nettai_platform::time::Instant::now()), Status::Loading(_)), "agreed");
        }
    }

    /// A player who can't download what the other has (BN5's r12, which the joiner took and the sites can no
    /// longer give) is refused: it takes what it can, its pool still differs, and its lobby refuses with what it
    /// couldn't have, the other side told the same. The joiner, which has it, waits until then, refusing nobody.
    #[test]
    fn a_peer_that_cant_download_is_refused() {
        use nettai_frontend::lobby::{Reason, Role, Status};
        let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
        let site = Site::new(&["exe5", "exe6"]);
        let (exe6, _) = site.bundle("exe6", 10, &["comp"]);
        site.index("exe6", &[&exe6]);
        let (old, _) = site.bundle("exe5", 10, &["acdc-area"]);
        site.index("exe5", &[&old]);
        let host = Peer::new(Role::Host, &site, &runtime);
        let (new, path) = site.bundle("exe5", 12, &["acdc-area", "airport"]);
        site.index("exe5", &[&old, &new]);
        let joiner = Peer::new(Role::Join, &site, &runtime);
        std::fs::remove_file(site.dir.path().join(path)).unwrap();
        let mut peers = [host, joiner];
        let [host_took, joiner_took] = meet(&mut peers, &runtime, 40);
        assert_eq!(host_took.len(), 1, "{host_took:?}");
        let failed = host_took[0].failed.clone().expect("the host couldn't have BN5's r12");
        assert!(failed.starts_with("exe5's newest content couldn't be downloaded") && failed.contains("404"), "{failed}");
        assert!(host_took[0].arrived.is_empty() && joiner_took.iter().all(|u| u.failed.is_none()), "{joiner_took:?}");
        let now = nettai_platform::time::Instant::now();
        let failure = |peer: &mut Peer| match peer.lobby.poll(now) {
            Status::Failed(f) => f.clone(),
            _ => panic!("not refused"),
        };
        let (h, j) = (failure(&mut peers[0]), failure(&mut peers[1]));
        assert_eq!((h.reason.clone(), j.reason.clone()), (Reason::DifferentContent, Reason::DifferentContent));
        assert!(h.detail.starts_with("can't play: the other side has other backgrounds") && h.detail.contains(&failed), "{h:?}");
        assert!(j.detail.starts_with("the other side refused: can't play:") && j.detail.contains(&failed), "{j:?}");
    }
}
