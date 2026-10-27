//! Content bundles (docs/design/content-deployment.md §3): a game pack and the support packs it depends on, as one
//! deterministic tar, named by the tar's SHA-256.
//!
//! ```text
//! bundle.toml          the game, the engine line, the revision, each pack's commit (`Manifest`)
//! <pack>/...           each pack's files (exe6/init.luau, exelib/regions.luau, ...)
//! ```
//!
//! The tar holds only files, sorted by path, each with mode 0644, no owner and time 0, so the same files make the
//! same tar and the same id on any machine. It travels gzipped (`.tar.gz`); the id is the tar's, not the gzip's.

use std::collections::BTreeMap;
use std::fmt;
use std::io::{Read, Write};
use std::path::{Component, Path};

use nettai_platform::fs;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A bundle's manifest's name in the bundle.
pub const MANIFEST: &str = "bundle.toml";

/// Files a pack's folder may hold that are no part of it: git's, the Finder's, and the repository's workflows
/// (`.github/`, which build the game's site: content-deployment.md §4).
const LITTER: [&str; 3] = [".git", ".DS_Store", ".github"];

/// A bundle's name: the SHA-256 of its tar.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BundleId(pub [u8; 32]);

impl BundleId {
    /// The id of a bundle's tar.
    pub fn of(tar: &[u8]) -> BundleId {
        BundleId(Sha256::digest(tar).into())
    }
}

impl fmt::Display for BundleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|b| write!(f, "{b:02x}"))
    }
}

impl fmt::Debug for BundleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BundleId({self})")
    }
}

impl std::str::FromStr for BundleId {
    type Err = String;

    /// 64 hex digits.
    fn from_str(s: &str) -> Result<BundleId, String> {
        let bad = || format!("a bundle id is 64 hex digits, not {s:?}");
        if s.len() != 64 || !s.is_ascii() {
            return Err(bad());
        }
        let mut id = [0u8; 32];
        for (i, b) in id.iter_mut().enumerate() {
            *b = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).map_err(|_| bad())?;
        }
        Ok(BundleId(id))
    }
}

/// `bundle.toml`: what a bundle is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// The game pack it plays (`exe6`).
    pub game: String,
    /// The engine line it is for: the engine's major and minor version (`0.11`).
    pub line: String,
    /// The game's publish count within the line (1 for `r1`).
    pub revision: u32,
    /// Each pack it holds, the game's and its support packs', by id: the commit of the pack's repository.
    pub packs: BTreeMap<String, String>,
}

impl Serialize for BundleId {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for BundleId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<BundleId, D::Error> {
        String::deserialize(d)?.parse().map_err(serde::de::Error::custom)
    }
}

/// The nettaisen organization's Pages domain. Its github.io addresses redirect here, without the cross-origin header a
/// browser needs on a redirect, so the app asks this domain directly.
pub const SITES: &str = "https://nettai.n1gp.net";

/// The catalog: the organization's site's `catalog.json` (the nettaisen.github.io repository), the one address the
/// app starts from (§5). Every other address comes from what it reads: the catalog gives each game's index and the
/// AI's, and each index entry its file.
pub fn catalog_url() -> String {
    format!("{SITES}/catalog.json")
}

/// The catalog: the published games' indexes and the practice dummy's network's, each by its full address. (Fields
/// an app doesn't know are skipped, so a later catalog still reads.)
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Catalog {
    /// Each game's site's `index.json`, by pack id.
    pub games: BTreeMap<String, String>,
    /// The nettai-ai repository's site's `index.json`: the network the practice dummy that fights plays.
    #[serde(default)]
    pub ai: Option<String>,
}

/// A game's site's `index.json`: its published bundles by engine line, each line's in publish order. (Fields an
/// app doesn't know are skipped, so a later index still reads.)
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Index {
    pub game: String,
    pub lines: BTreeMap<String, Vec<Published>>,
}

/// A published bundle, as its game's index lists it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Published {
    pub id: BundleId,
    /// What it plays (`Content::hash`: the content alone), as 16 hex digits.
    #[serde(with = "content_hash_hex")]
    pub content_hash: nettai_battle::ContentHash,
    /// Its revision within the line (1 for `r1`).
    pub revision: u32,
    /// Each pack's commit, by pack id (its manifest's).
    pub packs: BTreeMap<String, String>,
    /// The day it was published (YYYY-MM-DD).
    pub date: String,
    /// The size of its `.tar.gz`, in bytes.
    pub size: u64,
    /// Where its `.tar.gz` is, in full: the site's index states it (its Pages workflow, which puts the bundle
    /// there); a release's `bundle.json`, written before the site has it, doesn't.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl Published {
    /// Where its `.tar.gz` is: an index entry without the address is refused.
    pub fn url(&self) -> Result<&str, String> {
        self.url.as_deref().ok_or_else(|| format!("bundle {} is listed without its address", self.id))
    }
}

impl Index {
    /// Line `line`'s newest bundle.
    pub fn newest(&self, line: &str) -> Option<&Published> {
        self.lines.get(line)?.last()
    }

    /// Line `line`'s newest bundle that plays `hash`.
    pub fn playing(&self, line: &str, hash: nettai_battle::ContentHash) -> Option<&Published> {
        self.lines.get(line)?.iter().rev().find(|p| p.content_hash == hash)
    }
}

mod content_hash_hex {
    use nettai_battle::ContentHash;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(h: &ContentHash, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{:016x}", h.0))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<ContentHash, D::Error> {
        let s = String::deserialize(d)?;
        u64::from_str_radix(&s, 16).map(ContentHash).map_err(|_| serde::de::Error::custom(format!("a content hash is 16 hex digits, not {s:?}")))
    }
}

/// A bundle: its tar and what names it.
#[derive(Clone, Debug)]
pub struct Bundle {
    pub manifest: Manifest,
    pub tar: Vec<u8>,
    pub id: BundleId,
}

/// Bundle `manifest` with each pack's folder (`packs`: the pack's id and its folder), which must be the manifest's
/// packs. Every file of the folders goes in, but `.git`, `.DS_Store` and `.github/` at any depth.
pub fn build(manifest: Manifest, packs: &[(&str, &Path)]) -> Result<Bundle, String> {
    let named: Vec<&str> = manifest.packs.keys().map(String::as_str).collect();
    let mut given: Vec<&str> = packs.iter().map(|(id, _)| *id).collect();
    given.sort_unstable();
    if named != given {
        return Err(format!("the manifest names the packs {named:?}, and the folders given are {given:?}"));
    }
    let mut files = BTreeMap::new();
    let text = toml::to_string(&manifest).map_err(|e| format!("{MANIFEST}: {e}"))?;
    files.insert(MANIFEST.to_string(), text.into_bytes());
    for (id, dir) in packs {
        if !fs::is_dir(dir) {
            return Err(format!("pack {id}'s folder {} isn't there", dir.display()));
        }
        pack_files(id, dir, &mut files)?;
    }
    let tar = write_tar(&files).map_err(|e| format!("writing the tar: {e}"))?;
    let id = BundleId::of(&tar);
    Ok(Bundle { manifest, tar, id })
}

/// The files of folder `dir` and its subfolders, under `prefix/`, by path.
fn pack_files(prefix: &str, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err(format!("{}: a file name that isn't UTF-8: {name:?}", dir.display()));
        };
        if LITTER.contains(&name) {
            continue;
        }
        let path = entry.path();
        let inner = format!("{prefix}/{name}");
        if fs::is_dir(&path) {
            pack_files(&inner, &path, out)?;
        } else {
            let data = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            out.insert(inner, data);
        }
    }
    Ok(())
}

/// The tar of `files`: each a regular file, in path order, mode 0644, no owner, time 0.
fn write_tar(files: &BTreeMap<String, Vec<u8>>) -> std::io::Result<Vec<u8>> {
    let mut builder = tar::Builder::new(Vec::new());
    for (path, data) in files {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        builder.append_data(&mut header, path, data.as_slice())?;
    }
    builder.into_inner()
}

/// A tar gzipped, as a bundle travels (no name, time 0).
pub fn gzip(tar: &[u8]) -> Vec<u8> {
    let mut gz = flate2::GzBuilder::new().mtime(0).write(Vec::new(), flate2::Compression::best());
    gz.write_all(tar).expect("gzip into memory");
    gz.finish().expect("gzip into memory")
}

/// A gzipped tar's tar.
pub fn gunzip(gz: &[u8]) -> Result<Vec<u8>, String> {
    let mut tar = Vec::new();
    flate2::read::GzDecoder::new(gz).read_to_end(&mut tar).map_err(|e| format!("the bundle isn't gzip: {e}"))?;
    Ok(tar)
}

/// A bundle's tar's manifest and files, by path. Each path must be relative and stay inside the bundle, and every
/// file must be under a pack the manifest names (or be the manifest).
pub fn read(tar: &[u8]) -> Result<(Manifest, BTreeMap<String, Vec<u8>>), String> {
    let mut files = BTreeMap::new();
    let mut archive = tar::Archive::new(tar);
    for entry in archive.entries().map_err(|e| format!("the bundle's tar: {e}"))? {
        let mut entry = entry.map_err(|e| format!("the bundle's tar: {e}"))?;
        if entry.header().entry_type() != tar::EntryType::Regular {
            return Err("the bundle's tar holds something other than files".to_string());
        }
        let path = entry.path().map_err(|e| format!("the bundle's tar: {e}"))?.into_owned();
        if path.components().any(|c| !matches!(c, Component::Normal(_))) {
            return Err(format!("the bundle names a path outside it: {}", path.display()));
        }
        let Some(path) = path.to_str().map(|p| p.replace('\\', "/")) else {
            return Err(format!("the bundle names a path that isn't UTF-8: {}", path.display()));
        };
        let mut data = Vec::new();
        entry.read_to_end(&mut data).map_err(|e| format!("the bundle's {path}: {e}"))?;
        files.insert(path, data);
    }
    let text = files.remove(MANIFEST).ok_or_else(|| format!("the bundle has no {MANIFEST}"))?;
    let text = String::from_utf8(text).map_err(|_| format!("the bundle's {MANIFEST} isn't UTF-8"))?;
    let manifest: Manifest = toml::from_str(&text).map_err(|e| format!("the bundle's {MANIFEST}: {e}"))?;
    if let Some(stray) = files.keys().find(|p| !manifest.packs.contains_key(p.split('/').next().unwrap_or(""))) {
        return Err(format!("the bundle's {stray} is under no pack its manifest names"));
    }
    Ok((manifest, files))
}

/// Unpack gzipped bundle `gz`, whose id must be `expected`, into folder `dest`, which mustn't exist yet: the files go
/// to a folder beside it first, renamed to `dest` once they are all written, so `dest` is whole or absent. The
/// result: the bundle's manifest; `dest` then holds `bundle.toml` and a folder for each pack, a content directory.
pub fn unpack(gz: &[u8], expected: BundleId, dest: &Path) -> Result<Manifest, String> {
    let tar = gunzip(gz)?;
    let id = BundleId::of(&tar);
    if id != expected {
        return Err(format!("the bundle's SHA-256 is {id}, not {expected}"));
    }
    let (manifest, files) = read(&tar)?;
    if fs::exists(dest) {
        return Err(format!("{} is there already", dest.display()));
    }
    let mut part = dest.as_os_str().to_owned();
    part.push(".part");
    let part = std::path::PathBuf::from(part);
    if fs::exists(&part) {
        fs::remove_dir_all(&part).map_err(|e| format!("{}: {e}", part.display()))?;
    }
    let written = (|| -> std::io::Result<()> {
        fs::create_dir_all(&part)?;
        let text = toml::to_string(&manifest).expect("a manifest read is written");
        fs::write(part.join(MANIFEST), text)?;
        for (path, data) in &files {
            let file = part.join(path);
            if let Some(dir) = file.parent() {
                fs::create_dir_all(dir)?;
            }
            fs::write(file, data)?;
        }
        fs::rename(&part, dest)
    })();
    written.map_err(|e| format!("unpacking into {}: {e}", dest.display()))?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A new empty folder for one test.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nettai-bundle-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn put(dir: &Path, path: &str, text: &str) {
        let file = dir.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }

    fn manifest() -> Manifest {
        Manifest {
            game: "exe6".into(),
            line: "0.11".into(),
            revision: 3,
            packs: [("exe6".to_string(), "aaaa".to_string()), ("exelib".to_string(), "bbbb".to_string())].into(),
        }
    }

    /// Two packs, with litter that stays out.
    fn packs(root: &Path) -> (PathBuf, PathBuf) {
        let game = root.join("exe6");
        let lib = root.join("exelib");
        put(&game, "manifest.toml", "id = \"exe6\"\n");
        put(&game, "chips/cannon/init.luau", "return {}\n");
        put(&game, ".DS_Store", "litter");
        put(&game, ".git", "gitdir: ../../.git/modules/content/exe6\n");
        put(&game, ".github/workflows/pages.yml", "name: Pages\n");
        put(&lib, "manifest.toml", "id = \"exelib\"\n");
        put(&lib, "a/very/long/path/that/goes/on/and/on/past/a/hundred/characters/of/tar/header/name/room/regions.luau", "x");
        (game, lib)
    }

    #[test]
    fn the_same_files_make_the_same_id() {
        let root = scratch("same");
        let (game, lib) = packs(&root);
        let a = build(manifest(), &[("exe6", &game), ("exelib", &lib)]).unwrap();
        let b = build(manifest(), &[("exelib", &lib), ("exe6", &game)]).unwrap();
        assert_eq!(a.id, b.id);
        assert_eq!(a.tar, b.tar);
        put(&game, "chips/cannon/init.luau", "return { changed = true }\n");
        let c = build(manifest(), &[("exe6", &game), ("exelib", &lib)]).unwrap();
        assert_ne!(a.id, c.id);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_bundle_unpacks_to_its_files() {
        let root = scratch("unpack");
        let (game, lib) = packs(&root);
        let bundle = build(manifest(), &[("exe6", &game), ("exelib", &lib)]).unwrap();
        let (m, files) = read(&bundle.tar).unwrap();
        assert_eq!(m, manifest());
        assert!(files.keys().all(|p| !p.contains(".DS_Store") && !p.ends_with("/.git") && !p.contains(".github")), "{:?}", files.keys());
        let dest = root.join("store").join(bundle.id.to_string());
        let m = unpack(&gzip(&bundle.tar), bundle.id, &dest).unwrap();
        assert_eq!(m, manifest());
        assert_eq!(std::fs::read_to_string(dest.join("exe6/chips/cannon/init.luau")).unwrap(), "return {}\n");
        assert!(dest.join(MANIFEST).is_file());
        assert!(!dest.join("exe6/.DS_Store").exists());
        // Unpacked again, it is refused: the store keeps one copy.
        assert!(unpack(&gzip(&bundle.tar), bundle.id, &dest).is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_bundle_of_another_id_is_refused() {
        let root = scratch("id");
        let (game, lib) = packs(&root);
        let bundle = build(manifest(), &[("exe6", &game), ("exelib", &lib)]).unwrap();
        let other = BundleId([7; 32]);
        let err = unpack(&gzip(&bundle.tar), other, &root.join("x")).unwrap_err();
        assert!(err.contains("SHA-256"), "{err}");
        assert!(!root.join("x").exists() && !root.join("x.part").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_path_outside_the_bundle_is_refused() {
        let mut files = BTreeMap::new();
        files.insert(MANIFEST.to_string(), toml::to_string(&manifest()).unwrap().into_bytes());
        files.insert("exe6/ok.luau".to_string(), b"x".to_vec());
        let mut builder = tar::Builder::new(Vec::new());
        for (path, data) in &files {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            builder.append_data(&mut header, path, data.as_slice()).unwrap();
        }
        // (A header whose name is written by hand: append_data refuses `..` itself.)
        let mut header = tar::Header::new_old();
        header.as_old_mut().name[..10].copy_from_slice(b"../evil.lu");
        header.set_size(1);
        header.set_cksum();
        builder.append(&header, &b"x"[..]).unwrap();
        let tar = builder.into_inner().unwrap();
        let err = read(&tar).unwrap_err();
        assert!(err.contains("outside"), "{err}");
    }

    #[test]
    fn a_file_under_no_pack_is_refused() {
        let mut files = BTreeMap::new();
        files.insert(MANIFEST.to_string(), toml::to_string(&manifest()).unwrap().into_bytes());
        files.insert("exe5/sneaky.luau".to_string(), b"x".to_vec());
        let err = read(&write_tar(&files).unwrap()).unwrap_err();
        assert!(err.contains("under no pack"), "{err}");
    }

    #[test]
    fn the_packs_given_must_be_the_manifests() {
        let root = scratch("packs");
        let (game, _) = packs(&root);
        let err = build(manifest(), &[("exe6", &game)]).unwrap_err();
        assert!(err.contains("manifest names the packs"), "{err}");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn an_index_reads_back_and_finds_by_content_hash() {
        let entry = |rev: u32, id: u8, hash: u64| Published {
            id: BundleId([id; 32]),
            content_hash: nettai_battle::ContentHash(hash),
            revision: rev,
            packs: [("exe6".to_string(), "aaaa".to_string())].into(),
            date: "2026-10-26".into(),
            size: 1234,
            url: Some(format!("https://example.org/exe6/bundles/{}.tar.gz", BundleId([id; 32]))),
        };
        let index = Index { game: "exe6".into(), lines: [("0.11".to_string(), vec![entry(1, 1, 0xA), entry(2, 2, 0xB), entry(3, 3, 0xA)])].into() };
        let json = serde_json::to_string(&index).unwrap();
        assert!(json.contains("\"content_hash\":\"000000000000000a\""), "{json}");
        let back: Index = serde_json::from_str(&json).unwrap();
        assert_eq!(back, index);
        assert_eq!(back.newest("0.11").unwrap().revision, 3);
        assert_eq!(back.playing("0.11", nettai_battle::ContentHash(0xB)).unwrap().revision, 2);
        assert_eq!(back.playing("0.11", nettai_battle::ContentHash(0xA)).unwrap().revision, 3);
        assert!(back.playing("0.12", nettai_battle::ContentHash(0xA)).is_none());
        // A later index's fields an app doesn't know are skipped.
        let later = json.replace("\"size\":1234", "\"size\":1234,\"notes\":\"new\"");
        assert_eq!(serde_json::from_str::<Index>(&later).unwrap(), index);
        assert!(back.newest("0.11").unwrap().url().unwrap().ends_with(&format!("{}.tar.gz", BundleId([3; 32]))));
        // A release's bundle.json has no address, and writes none.
        let release = Published { url: None, ..entry(1, 1, 0xA) };
        assert!(!serde_json::to_string(&release).unwrap().contains("url"));
        assert!(release.url().unwrap_err().contains("without its address"));
    }

    #[test]
    fn the_catalog_lists_games() {
        let catalog: Catalog =
            serde_json::from_str(r#"{"games": {"exe6": "https://example.org/exe6/index.json"}, "ai": "https://example.org/ai/index.json", "later": 1}"#)
                .unwrap();
        assert_eq!(catalog.games["exe6"], "https://example.org/exe6/index.json");
        assert_eq!(catalog.ai.as_deref(), Some("https://example.org/ai/index.json"));
        assert_eq!(serde_json::from_str::<Catalog>(r#"{"games": {}}"#).unwrap().ai, None);
        assert_eq!(catalog_url(), "https://nettai.n1gp.net/catalog.json");
    }

    #[test]
    fn an_id_reads_back_from_its_hex() {
        let id = BundleId::of(b"bundle");
        assert_eq!(id.to_string().parse::<BundleId>().unwrap(), id);
        assert!("abc".parse::<BundleId>().is_err());
        assert!("zz".repeat(32).parse::<BundleId>().is_err());
    }
}
