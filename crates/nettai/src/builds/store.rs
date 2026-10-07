//! The player's builds on disk: each a TOML file of its own in the builds
//! folder (`crate::paths::builds`), a folder per game
//! (`builds/exe6/<name>.toml`). A build file names its game and the
//! build, then states the side under `[side]` as a match file states one
//! (`nettai_match::file`: the same names, written by the creator or by
//! hand):
//!
//! ```toml
//! game = "exe6"
//! name = "Falzar heat"
//!
//! [side]
//! navi = "megaman"
//! version = "falzar"
//! folder = [...]
//! ```

use nettai_battle::Content;
use nettai_match::Side;
use nettai_match::file::SideFile;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildFile {
    game: String,
    name: String,
    #[serde(default)]
    side: SideFile,
}

/// A build's file, as it is listed: its path, its name, and its text
/// (read against its game's content once that is loaded: [`read`]).
#[derive(Clone, Debug)]
pub struct Listed {
    pub path: PathBuf,
    pub name: String,
    pub text: String,
}

/// The folder of game `game`'s builds.
pub fn folder(game: &str) -> PathBuf {
    crate::paths::builds().join(game)
}

/// Game `game`'s builds, by name: each file of its folder that is a build
/// of it (a file that isn't one is left out).
pub fn list(game: &str) -> Vec<Listed> {
    let Ok(dir) = std::fs::read_dir(folder(game)) else { return Vec::new() };
    let mut out: Vec<Listed> = dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "toml"))
        .filter_map(|path| {
            let text = std::fs::read_to_string(&path).ok()?;
            let head = header(&text).ok().filter(|(g, _)| g == game)?;
            Some(Listed { path, name: head.1, text })
        })
        .collect();
    out.sort_by_key(|b| b.name.to_lowercase());
    out
}

/// A build file's game and name, read before its game's content is.
fn header(text: &str) -> Result<(String, String), String> {
    #[derive(Deserialize)]
    struct Header {
        game: String,
        name: String,
    }
    let h: Header = toml::from_str(text).map_err(|e| e.message().to_string())?;
    Ok((h.game, h.name))
}

/// A build file's text read against its game's content: its name and its
/// side, or what keeps it from being one (a name the game hasn't).
pub fn read(content: &Content, game: &str, text: &str) -> Result<(String, Side), String> {
    let f: BuildFile = toml::from_str(text).map_err(|e| e.message().to_string())?;
    if f.game != game {
        return Err(format!("a build of {}, not of {game}", f.game));
    }
    let mut problems = Vec::new();
    let side = nettai_match::file::resolve_side(content, game, &f.side, "the build", &mut problems);
    side.map(|s| (f.name, s)).ok_or_else(|| problems.join("; "))
}

/// A build as its file's text.
pub fn text(content: &Content, game: &str, name: &str, side: &Side) -> String {
    #[derive(Serialize)]
    struct Header<'a> {
        game: &'a str,
        name: &'a str,
    }
    let head = toml::to_string(&Header { game, name }).expect("a header serializes");
    format!("# A nettai build (docs/app.md §8).\n\n{head}\n{}", nettai_match::file::side_toml(content, "side", side))
}

/// Write a build to `path` (its folder made if it isn't there).
pub fn write(path: &Path, content: &Content, game: &str, name: &str, side: &Side) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(path, text(content, game, name, side)).map_err(|e| format!("{}: {e}", path.display()))
}

/// A path for a new build of `game` named `name` that no file has: the
/// name's letters and digits (`falzar-heat.toml`), numbered if it is taken.
pub fn new_path(game: &str, name: &str) -> PathBuf {
    let mut stem: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_lowercase().next().unwrap_or(c) } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if stem.is_empty() {
        stem = "build".into();
    }
    let dir = folder(game);
    let mut path = dir.join(format!("{stem}.toml"));
    for n in 2.. {
        if !path.exists() {
            break;
        }
        path = dir.join(format!("{stem}-{n}.toml"));
    }
    path
}

/// A name for a new build that none of `taken` has: `base`, else `base 2`,
/// and so on.
pub fn new_name(base: &str, taken: &[String]) -> String {
    let free = |n: &str| !taken.iter().any(|t| t.eq_ignore_ascii_case(n));
    if free(base) {
        return base.to_string();
    }
    (2..).map(|n| format!("{base} {n}")).find(|n| free(n)).expect("a free name")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A build's file reads back as the build it was written from; one of
    /// another game, or naming what the game hasn't, isn't one.
    #[test]
    fn a_build_reads_back() {
        let content = nettai_match::testing::exe6_content();
        let m = nettai_match::pick::live(&content, "exe6", 11, None).unwrap();
        let text = text(&content, "exe6", "Falzar \"heat\"", &m.sides[0]);
        assert!(text.contains("[side]"), "{text}");
        assert_eq!(header(&text), Ok(("exe6".into(), "Falzar \"heat\"".into())));
        assert_eq!(read(&content, "exe6", &text), Ok(("Falzar \"heat\"".to_string(), m.sides[0].clone())));
        assert!(read(&content, "exe5", &text).unwrap_err().contains("not of exe5"));
        let wrong = text.replacen("navi = \"megaman\"", "navi = \"nobody\"", 1);
        assert_ne!(wrong, text);
        assert!(read(&content, "exe6", &wrong).is_err());
    }

    #[test]
    fn new_names_and_paths() {
        assert_eq!(new_name("Falzar", &["falzar".into(), "Falzar 2".into()]), "Falzar 3");
        assert_eq!(new_name("Gregar", &["falzar".into()]), "Gregar");
        let path = new_path("exe6", "  Heat / Elec!! ");
        assert_eq!(path.file_name().and_then(|n| n.to_str()), Some("heat-elec.toml"));
    }
}
