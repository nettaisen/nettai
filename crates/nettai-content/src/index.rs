//! The content (docs/design/content-model-v2.md §4.0): a content directory
//! (content/) of packs, each a folder with a manifest:
//!
//! ```text
//! <pack>/manifest.toml            the pack: its name, its kind (a game, or a support pack games
//!                                 share) and the support packs it depends on
//!                                 (nettai_content_api::packs)
//! <game>/init.luau                a game's top module: it requires the game's rules and folders
//! <game>/<folder>/init.luau       a folder's index: it requires the folder's modules that define
//!                                 what the game has (its chips, navis, forms, stages, patch cards,
//!                                 NaviCust programs, and what only an id names)
//! <pack>/**/*.luau                the pack's scripts (exe6/chips/cannon/init.luau: a folder's main
//!                                 module, which the folder's name stands for); every id local to
//!                                 the game (`minibomb`), every asset name its asset pack's (`bomb`)
//! <pack>/**/*.d.luau              its API declarations, which its modules (and its users') check
//!                                 against after the engine's
//! <pack>/locales/<language>.toml  a game's display text by id: the own language's (en) is the
//!                                 content's strings, the others a frontend's (crate::locale)
//! <game>/compat/                  the original's numbers by id: tools' data, not content
//! nettai/core.d.luau              the engine's API declarations
//! ```
//!
//! [`read`] reads the manifests of the games it loads and of the support
//! packs they depend on, and no module: the define phase reads each as a
//! `require` reaches it, from the packs' folders (`Scripts::dirs`,
//! `packs::find`), starting at each game's top module. A module requires
//! only its own pack's and those of the support packs its pack depends on
//! (`packs::check_require`). [`read_all`] loads every game. The assets the
//! definitions name (`asset.sprite("bomb")`) come from the extracted packs'
//! asset indices; `crate::pack::load_battle` puts the two together.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use nettai_content_api::{PackManifest, packs};

use crate::report::Report;

/// This repository's content directory.
const CONTENT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content");

/// The engine's declarations' directory in a content directory.
pub const DECLARATIONS: &str = packs::ENGINE;

/// The content directory: `$NETTAI_CONTENT`, else this repository's
/// content/.
pub fn content() -> PathBuf {
    std::env::var_os("NETTAI_CONTENT").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(CONTENT))
}

/// The engine's API declarations for content (content/nettai).
pub fn engine_declarations(content: &Path) -> PathBuf {
    content.join(DECLARATIONS)
}

/// The game packs of content `dir`, by name: the games it can play.
pub fn games(dir: &Path) -> Result<Vec<String>, String> {
    packs::games(dir)
}

/// What a load starts from.
#[derive(Clone, Debug, Default)]
pub struct Read {
    /// Where its packs are.
    pub dir: PathBuf,
    /// The packs it loads (their manifests), in load order: the support
    /// packs the games depend on, then the games.
    pub packs: Vec<PackManifest>,
    /// The games' own language's strings: their display text, which the
    /// define phase counts the chatbox's timing from (`Content::strings`).
    pub strings: crate::locale::Strings,
}

impl Read {
    /// The game packs it loads, in order.
    pub fn games(&self) -> Vec<String> {
        self.packs.iter().filter(|p| p.kind == nettai_content_api::PackKind::Game).map(|p| p.id.clone()).collect()
    }

    /// The scripts as the engine loads them: the packs, and their folders,
    /// which the define phase reads each module from as it is required (no
    /// module is read here).
    pub fn scripts(&self) -> nettai_battle::content::Scripts {
        let dirs = packs::Dirs::of_content(&self.dir, self.packs.iter().map(|p| p.id.as_str()));
        nettai_battle::content::Scripts { packs: self.packs.clone(), dirs: nettai_battle::content::ModuleDirs(dirs), ..Default::default() }
    }
}

/// The manifests of every pack of content `dir`, by name.
pub fn manifests(dir: &Path) -> Result<BTreeMap<String, PackManifest>, String> {
    Ok(packs::packs(dir)?.into_iter().map(|p| (p.id.clone(), p)).collect())
}

/// Read what a load of the games `games` of content `dir` starts from: the
/// packs they load (the support packs they depend on, then the games) and
/// the games' own language's strings. A pack that isn't there, a game pack
/// depended on or a cycle: errors, each naming the path. (A game without
/// its init.luau, a require of no module and a require across packs that
/// the packs refuse are the define phase's errors: it reads the modules.)
pub fn read(dir: &Path, games: &[String], report: &mut Report) -> Option<Read> {
    let all = match manifests(dir) {
        Ok(a) => a,
        Err(e) => {
            report.error(dir.display().to_string(), e);
            return None;
        }
    };
    let order = match packs::load_order(&all, games) {
        Ok(o) => o,
        Err(e) => {
            report.error(dir.display().to_string(), e);
            return None;
        }
    };
    let mut out = Read { dir: dir.to_path_buf(), packs: order.into_iter().cloned().collect(), ..Default::default() };
    match crate::locale::load_for(dir, &out.games(), crate::locale::OWN) {
        Ok(Some(s)) => out.strings = s,
        Ok(None) => report.warn(
            format!("{}/{}", crate::locale::DIR, crate::locale::OWN),
            "the content has no strings: its chips, navis and forms show by their keys",
        ),
        Err(e) => report.error(format!("{}/{}", crate::locale::DIR, crate::locale::OWN), e),
    }
    (!report.has_errors()).then_some(out)
}

/// Read every game of content `dir`.
pub fn read_all(dir: &Path, report: &mut Report) -> Option<Read> {
    let games = match games(dir) {
        Ok(g) => g,
        Err(e) => {
            report.error(dir.display().to_string(), e);
            return None;
        }
    };
    read(dir, &games, report)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The repository's content reads: the games' packs after the support
    /// pack, and no module before a load requires it.
    #[test]
    fn the_content_reads_from_its_games_packs() {
        let mut r = Report::default();
        let read = read_all(&content(), &mut r).unwrap_or_else(|| panic!("{r}"));
        assert_eq!(read.games(), ["exe4", "exe5", "exe6"]);
        assert_eq!(read.packs[0].id, "exelib", "the support pack first");
        let scripts = read.scripts();
        assert!(scripts.modules.is_empty(), "nothing is read ahead");
        assert_eq!(scripts.dirs.0.0.keys().collect::<Vec<_>>(), ["exe4", "exe5", "exe6", "exelib"]);
    }

    /// A game without its top module, a require of no module, and a require
    /// of another game's module, are errors of the load, naming where they
    /// are written.
    #[test]
    fn what_a_load_refuses() {
        let dir = std::env::temp_dir().join(format!("nettai-index-{}", std::process::id()));
        let write = |path: &str, text: &str| {
            let p = dir.join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        };
        write("lib/manifest.toml", "id = \"lib\"\nkind = \"support\"\n");
        write("lib/x.luau", "return {}\n");
        write("g/manifest.toml", "id = \"g\"\nkind = \"game\"\ndepends = [\"lib\"]\n");
        write("g/init.luau", "require(\"@self/rules\")\n");
        write("g/rules.luau", "local y = require(\"@lib/x\")\nlocal x = require(\"@g/nowhere\")\nreturn {}\n");
        write("h/manifest.toml", "id = \"h\"\nkind = \"game\"\n");
        write("h/init.luau", "require(\"@self/rules\")\n");
        write("h/rules.luau", "local x = require(\"@g/rules\")\nreturn {}\n");
        write("i/manifest.toml", "id = \"i\"\nkind = \"game\"\n");
        write("i/rules.luau", "return {}\n");
        let load = |game: &str| -> String {
            let mut r = Report::default();
            let read = read(&dir, &[game.into()], &mut r).unwrap_or_else(|| panic!("{r}"));
            let mut c = nettai_battle::Content { scripts: read.scripts(), ..Default::default() };
            c.define().expect_err("a load the content refuses").message
        };
        let e = load("g");
        assert!(e.contains("g/rules.luau: require(\"@g/nowhere\"): no module g/nowhere.luau"), "{e}");
        // (Another game is no pack of the load: a content is one game.)
        let e = load("h");
        assert!(e.contains("h/rules.luau: require(\"@g/rules\"): g is no pack of this load"), "game to game: {e}");
        let e = load("i");
        assert!(e.contains("i/init.luau: game pack i has no top module"), "{e}");
        // A pack that isn't there is the read's.
        let mut r = Report::default();
        assert!(read(&dir, &["j".into()], &mut r).is_none());
        assert!(r.to_string().contains("no pack j"), "{r}");
        std::fs::remove_dir_all(&dir).ok();
    }
}
