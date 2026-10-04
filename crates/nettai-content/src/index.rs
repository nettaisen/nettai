//! The content (docs/design/content-model-v2.md §4.0): a content directory
//! (content/) of packs, each a folder with a manifest:
//!
//! ```text
//! <pack>/manifest.toml            the pack: its kind (a game, or a support pack games share), the
//!                                 support packs it uses, and a game's modules that define what a
//!                                 person picks or a ruleset names (rules, chips, navis, forms,
//!                                 stages, patch cards, NaviCust programs), its `also` and its
//!                                 `unported` (nettai_content_api::packs)
//! <pack>/**/*.luau                the pack's scripts (bn6/chips/cannon/chips); every id local to the
//!                                 game (`minibomb`), every asset name its asset pack's (`bomb`)
//! <pack>/**/*.d.luau              its API declarations, which its modules (and its users') check
//!                                 against after the engine's
//! <pack>/locales/<language>.toml  a game's display text by id: the own language's (en) is the
//!                                 content's strings, the others a frontend's (crate::locale)
//! <game>/compat/                  the original's numbers by id: tools' data, not content
//! nettai/core.d.luau              the engine's API declarations
//! ```
//!
//! [`read`] reads the manifests of the games it loads and of the support
//! packs they use, and follows the listed modules' requires (each a literal
//! path: `require("@exelib/swords/slash")`, `./x`, `../x`), reading exactly
//! those modules; a module requires only its own pack's and those of the
//! support packs its pack uses (`packs::check_require`). [`read_all`]
//! loads every game. The assets the definitions name
//! (`asset.sprite("bomb")`) come from the extracted packs' asset
//! indices; `crate::pack::load_battle` puts the two together.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use nettai_content_api::{PackManifest, keys, packs};

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

/// What a load reads.
#[derive(Clone, Debug, Default)]
pub struct Read {
    /// Where it was read from.
    pub dir: PathBuf,
    /// The packs it loads (their manifests), in load order: the support
    /// packs the games use, then the games.
    pub packs: Vec<PackManifest>,
    /// The modules: the packs' listed modules and what they require, by
    /// module name (`keys::module_name`).
    pub modules: BTreeMap<String, String>,
    /// The games' own language's strings: their display text, which the
    /// define phase counts the chatbox's timing from (`Content::strings`).
    pub strings: crate::locale::Strings,
}

impl Read {
    /// The game packs it loads, in order.
    pub fn games(&self) -> Vec<String> {
        self.packs.iter().filter(|p| p.kind == nettai_content_api::PackKind::Game).map(|p| p.id.clone()).collect()
    }

    /// The modules of pack `pack` this load reads.
    pub fn modules_of<'a>(&'a self, pack: &'a str) -> impl Iterator<Item = (&'a String, &'a String)> + 'a {
        self.modules.iter().filter(move |(name, _)| keys::root_of(name) == Some(pack))
    }

    /// The scripts as the engine loads them.
    pub fn scripts(&self) -> nettai_battle::content::Scripts {
        nettai_battle::content::Scripts { modules: self.modules.clone(), packs: self.packs.clone(), ..Default::default() }
    }
}

/// The literal paths module source `text` requires, as written (outside its
/// line comments): `packs::requires`.
pub fn requires(text: &str) -> Vec<String> {
    packs::requires(text)
}

/// The manifests of every pack of content `dir`, by name.
pub fn manifests(dir: &Path) -> Result<BTreeMap<String, PackManifest>, String> {
    Ok(packs::packs(dir)?.into_iter().map(|p| (p.id.clone(), p)).collect())
}

/// Read the games `games` of content `dir`: the packs they load (the
/// support packs they use, then the games), each pack's listed modules and
/// what they require, and the games' own language's strings. A pack that
/// isn't there, a use of a game pack or a cycle of uses, a listed module or
/// a require of no module, a require across packs that the packs refuse,
/// an unported module that isn't there: errors, each naming the path.
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
    for p in &out.packs {
        for m in p.unported() {
            let file = format!("{}.luau", keys::module_path(&m));
            if !dir.join(&file).is_file() {
                report.error(format!("{}/{}", p.id, packs::MANIFEST), format!("unported {}: no module {file}", keys::local(&m)));
            }
        }
    }
    let start: Vec<(String, String)> =
        out.packs.iter().flat_map(|p| p.entries().into_iter().map(move |m| (m, p.id.clone()))).collect();
    follow(dir, &all, &start, &mut out.modules, report);
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

/// Read into `modules` the modules `start` (by name, each with the pack
/// whose manifest lists it) and what they and the modules already there
/// require, from content `dir`, each module once (one already in `modules`
/// keeps its text: a tool's stand-in). A module that isn't there is an
/// error naming the module that requires it (or the manifest that lists
/// it); so is a require the packs `all` refuse.
pub fn follow(
    dir: &Path,
    all: &BTreeMap<String, PackManifest>,
    start: &[(String, String)],
    modules: &mut BTreeMap<String, String>,
    report: &mut Report,
) {
    let mut pending: Vec<(String, Result<(String, String), String>)> =
        start.iter().map(|(n, pack)| (n.clone(), Err(pack.clone()))).collect();
    for (name, _) in modules.iter() {
        pending.push((name.clone(), Err(String::new())));
    }
    let mut seen: BTreeSet<String> = pending.iter().map(|(n, _)| n.clone()).collect();
    while let Some((name, from)) = pending.pop() {
        let mut path = keys::module_path(&name);
        // (A folder names its `init` module: `bn6:rules` is rules/init.luau.)
        let init = keys::init_of(&name);
        let name = if !modules.contains_key(&name)
            && !dir.join(format!("{path}.luau")).is_file()
            && (modules.contains_key(&init) || dir.join(format!("{}.luau", keys::module_path(&init))).is_file())
        {
            path = keys::module_path(&init);
            if !seen.insert(init.clone()) {
                continue;
            }
            init
        } else {
            name
        };
        let text = match modules.get(&name) {
            Some(t) => t.clone(),
            None => match std::fs::read_to_string(dir.join(format!("{path}.luau"))) {
                Ok(t) => t,
                Err(e) => {
                    match from {
                        Ok((from, written)) => report.error(
                            format!("{}.luau", keys::module_path(&from)),
                            format!("require({written:?}): no module {path}.luau in {} ({e})", dir.display()),
                        ),
                        Err(pack) => report.error(
                            format!("{pack}/{}", packs::MANIFEST),
                            format!("lists {}, and no module {path}.luau is in {} ({e})", keys::local(&name), dir.display()),
                        ),
                    }
                    continue;
                }
            },
        };
        for written in requires(&text) {
            match keys::resolve(&name, &written) {
                Ok(target) => {
                    if let Err(e) = packs::check_require(all, &name, &written, &target) {
                        let file = format!("{path}.luau");
                        let said = e.strip_prefix(&format!("{file}: ")).unwrap_or(&e).to_string();
                        report.error(file, said);
                        continue;
                    }
                    if seen.insert(target.clone()) {
                        pending.push((target, Ok((name.clone(), written))));
                    }
                }
                Err(e) => report.error(format!("{path}.luau"), e),
            }
        }
        modules.insert(name, text);
    }
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

    #[test]
    fn requires_are_read_as_written() {
        let text = "local a = require(\"@exelib/x\")\nlocal b = require('./y') -- require(\"./z\")\nreturn {}\n";
        assert_eq!(requires(text), ["@exelib/x", "./y"]);
    }

    /// The repository's content reads: every game's listed modules and what
    /// they require, and nothing names a module that isn't there.
    #[test]
    fn the_content_reads_from_its_manifests() {
        let mut r = Report::default();
        let read = read_all(&content(), &mut r).unwrap_or_else(|| panic!("{r}"));
        assert_eq!(read.games(), ["bn5", "bn6"]);
        assert_eq!(read.packs[0].id, "exelib", "the support pack first");
        assert!(read.modules.contains_key("bn6:rules/init"), "the manifest's `rules` is the folder's init");
        assert!(read.modules.contains_key("exelib:swords/slash"), "the support pack's modules come in by their requires");
    }

    /// A listed module or a require of no module, and a require of another
    /// game's module, are errors naming where they are written.
    #[test]
    fn what_a_read_refuses() {
        let dir = std::env::temp_dir().join(format!("nettai-index-{}", std::process::id()));
        let write = |path: &str, text: &str| {
            let p = dir.join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        };
        write("lib/manifest.toml", "id = \"lib\"\nkind = \"support\"\n");
        write("lib/x.luau", "return {}\n");
        write("g/manifest.toml", "id = \"g\"\nkind = \"game\"\nuses = [\"lib\"]\n[definitions]\nrules = [\"rules\"]\n");
        write("g/rules.luau", "local x = require(\"@g/nowhere\")\nlocal y = require(\"@lib/x\")\nreturn {}\n");
        write("h/manifest.toml", "id = \"h\"\nkind = \"game\"\n[definitions]\nrules = [\"rules\", \"gone\"]\n");
        write("h/rules.luau", "local x = require(\"@g/rules\")\nreturn {}\n");
        let mut r = Report::default();
        assert!(read(&dir, &["g".into()], &mut r).is_none());
        assert!(r.to_string().contains("g/rules.luau") && r.to_string().contains("no module g/nowhere.luau"), "{r}");
        let mut r = Report::default();
        assert!(read(&dir, &["h".into()], &mut r).is_none());
        assert!(r.to_string().contains("h/manifest.toml") && r.to_string().contains("lists gone, and no module h/gone.luau"), "{r}");
        assert!(r.to_string().contains("require(\"@g/rules\"): g is a game pack"), "game to game: {r}");
        std::fs::remove_dir_all(&dir).ok();
    }
}
