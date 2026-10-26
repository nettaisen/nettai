//! The network the dummy that fights plays with (docs/design/training-ai.md §10.5). None is built in: the nettai-ai
//! repository (github.com/nettaisen/nettai-ai) publishes them as releases, one line at a time ("0.11": what nettai
//! 0.11.x plays), and its site serves each line's newest: `index.json` gives each line's SHA-256 (so a check for a
//! new one is a small file, not the network) and `weights/<sha256>.safetensors` the network. So a better network
//! reaches players without a release of the app, and a release that changes what the network sees (a new minor
//! version) waits for one trained for it.
//!
//! It is asked for when Fights is first picked in a run ([`Weights::wanted`]), then the same for every dummy that
//! fights. The data folder keeps the last one downloaded (`ai/<line>/nettai-ai.safetensors`): where it is the
//! index's, it is read from there; else the index's is downloaded (`nettai_platform::http`, reqwest's, in a future on
//! the window's loop, `slint::spawn_local`, aborted if the app quits first; the files read, hashed and written off
//! that thread, `nettai_platform::task::off_thread`), checked against its SHA-256 and kept. With no
//! index (no connection, or no line for this version), the one kept plays if there is one.
//!
//! `NETTAI_AI_WEIGHTS=<file>` (the desktop's) names weights to play instead, for testing them: nothing is
//! downloaded. Weights this encoder can't play are refused either way (`Net::from_safetensors`: the observation
//! spec they were trained on), and the dummy stands still, the reason in the log.

use nettai_platform::task::off_thread;
use nettai_training::ai::Arriving;
use nettai_training::net::Net;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// The nettai-ai repository's site (its Pages workflow puts the releases there).
const SITE: &str = "https://nettai.n1gp.net/nettai-ai";
/// The most a network's file may be (they are about 11 MB), and the index's.
const MAX_BYTES: u64 = 64 << 20;
const MAX_INDEX: u64 = 1 << 20;
/// The variable naming weights to play instead (the desktop's).
pub(crate) const OVERRIDE: &str = "NETTAI_AI_WEIGHTS";

/// The process's network, once asked for, and the future bringing it (aborted if this goes first: the app quits).
#[derive(Default)]
pub(crate) struct Weights(Option<(Arriving, Option<slint::JoinHandle<()>>)>);

impl Weights {
    /// The network the dummy that fights plays with: asked for the first time this is called (on its way after),
    /// the same one every time after. Called on the window's thread.
    pub(crate) fn wanted(&mut self) -> Arriving {
        self.0.get_or_insert_with(fetch).0.clone()
    }
}

impl Drop for Weights {
    fn drop(&mut self) {
        if let Some(bringing) = self.0.as_mut().and_then(|(_, bringing)| bringing.take()).filter(|b| !b.is_finished()) {
            bringing.abort();
        }
    }
}

/// The weights' line, nettai's major and minor version ("0.11" for 0.11.2): weights are made for one.
fn line_of(version: &str) -> String {
    version.split('.').take(2).collect::<Vec<_>>().join(".")
}

fn line() -> String {
    line_of(env!("CARGO_PKG_VERSION"))
}

/// A line's weights: their file on the site (named by its SHA-256), and its SHA-256 (hex).
#[derive(Clone, Debug, PartialEq)]
struct Entry {
    file: String,
    sha256: String,
}

/// `line`'s entry in the index `body` (each line's SHA-256, by line). Err: an index that can't be read, or none for
/// the line.
fn entry(body: &[u8], line: &str) -> Result<Entry, String> {
    let index: HashMap<String, String> = serde_json::from_slice(body).map_err(|e| format!("the AI's index: {e}"))?;
    let sha256 = index.get(line).cloned().ok_or_else(|| format!("the AI's index has no weights for nettai {line} yet"))?;
    Ok(Entry { file: format!("weights/{sha256}.safetensors"), sha256 })
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn read(bytes: &[u8]) -> Result<Arc<Net>, String> {
    Net::from_safetensors(bytes).map(Arc::new).map_err(|why| format!("the AI's weights: {why}"))
}

/// Weights downloaded for `entry`: refused unless they are its, by its SHA-256.
fn checked(bytes: &[u8], entry: &Entry) -> Result<Arc<Net>, String> {
    if sha256(bytes) != entry.sha256 {
        return Err(format!("the AI's weights ({}) aren't the index's: another SHA-256", entry.file));
    }
    read(bytes)
}

/// Where the data folder keeps the line's weights.
fn kept() -> PathBuf {
    crate::paths::data().join("ai").join(line()).join("nettai-ai.safetensors")
}

/// The network asked for, on its way: `NETTAI_AI_WEIGHTS`'s file if it names one, else the repository's for this
/// line (the copy kept where it is that one), and the future bringing it, on the window's loop (none: it couldn't be
/// started, and the network has arrived as that error).
fn fetch() -> (Arriving, Option<slint::JoinHandle<()>>) {
    let arriving = Arriving::default();
    let done = arriving.clone();
    match slint::spawn_local(async move { done.arrive(network().await) }) {
        Ok(bringing) => (arriving, Some(bringing)),
        Err(why) => {
            arriving.arrive(Err(format!("asking for the AI's weights: {why}")));
            (arriving, None)
        }
    }
}

/// The network: `NETTAI_AI_WEIGHTS`'s file, else the repository's for this line, from the copy kept where it is the
/// index's, else downloaded, checked and kept; with no index, the copy kept. Files are read, hashed and written off
/// the window's thread.
async fn network() -> Result<Arc<Net>, String> {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(path) = std::env::var_os(OVERRIDE).map(PathBuf::from) {
        ::log::info!(target: "nettai", "the dummy's AI plays {} ({OVERRIDE})", path.display());
        return off_thread("ai-weights", move || read(&std::fs::read(&path).map_err(|e| format!("{OVERRIDE}, {}: {e}", path.display()))?)).await?;
    }
    let kept = kept();
    let entry = match nettai_platform::http::get(&format!("{SITE}/index.json"), MAX_INDEX).await.and_then(|body| entry(&body, &line())) {
        Ok(entry) => entry,
        Err(why) => {
            return off_thread("ai-weights", move || match nettai_platform::fs::read(&kept) {
                Ok(bytes) => {
                    ::log::warn!(target: "nettai", "{why}: the dummy's AI plays the weights kept ({})", kept.display());
                    read(&bytes)
                }
                Err(_) => Err(why),
            })
            .await?;
        }
    };
    // (The copy kept, where it is the index's.)
    let (k, sha) = (kept.clone(), entry.sha256.clone());
    if let Some(net) = off_thread("ai-weights", move || nettai_platform::fs::read(&k).ok().filter(|bytes| sha256(bytes) == sha).map(|bytes| read(&bytes))).await? {
        return net;
    }
    let bytes = nettai_platform::http::get(&format!("{SITE}/{}", entry.file), MAX_BYTES).await?;
    off_thread("ai-weights", move || {
        let net = checked(&bytes, &entry)?;
        let written = kept.parent().map_or(Ok(()), nettai_platform::fs::create_dir_all).and_then(|_| nettai_platform::fs::write(&kept, &bytes));
        if let Err(why) = written {
            crate::errors::log("keeping the AI's weights", &format!("{}: {why}", kept.display()));
        }
        Ok(net)
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A version's line is its major and minor version.
    #[test]
    fn a_versions_line_is_its_major_and_minor() {
        assert_eq!(line_of("0.11.0"), "0.11");
        assert_eq!(line_of("1.2.13"), "1.2");
        assert_eq!(line(), line_of(env!("CARGO_PKG_VERSION")));
    }

    /// The index gives each line's SHA-256, its file named by it; a line it hasn't is said so, and an index that
    /// isn't one is refused.
    #[test]
    fn the_index_gives_a_lines_weights() {
        let body = br#"{"0.11": "ab", "0.12": "cd"}"#;
        assert_eq!(entry(body, "0.11"), Ok(Entry { file: "weights/ab.safetensors".into(), sha256: "ab".into() }));
        assert!(entry(body, "0.13").unwrap_err().contains("no weights for nettai 0.13"));
        assert!(entry(b"<html>", "0.11").is_err());
    }

    /// The site serves this line's weights: downloaded, checked and kept, then read from the copy kept. Not run
    /// unless asked (`--ignored`): it needs the network, and a release of this line's.
    #[test]
    #[ignore]
    fn the_repository_serves_this_lines_weights() {
        let data = tempfile::tempdir().unwrap();
        // (Alone in its process when asked for: `--ignored` with its name.)
        unsafe {
            std::env::set_var("NETTAI_DATA", data.path());
            std::env::remove_var(OVERRIDE);
        }
        // (No window's loop in a test: the future run on tokio's, as the program's runs under it.)
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let arrived = || {
            let got = runtime.block_on(network());
            assert!(got.is_ok(), "{:?}", got.err());
        };
        arrived();
        let kept = std::fs::read(kept()).expect("kept");
        arrived();
        assert_eq!(std::fs::read(super::kept()).unwrap(), kept, "read from the copy kept");
    }

    /// Downloaded weights are played only when they are the index's, by SHA-256.
    #[test]
    fn downloaded_weights_must_be_the_indexs() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../nettai-training/tests/fixtures/net.safetensors");
        let bytes = std::fs::read(path).unwrap();
        let entry = Entry { file: format!("weights/{}.safetensors", sha256(&bytes)), sha256: sha256(&bytes) };
        assert!(checked(&bytes, &entry).is_ok());
        let other = Entry { sha256: sha256(b"other"), ..entry };
        assert!(checked(&bytes, &other).err().is_some_and(|why| why.contains("aren't the index's")));
    }
}
