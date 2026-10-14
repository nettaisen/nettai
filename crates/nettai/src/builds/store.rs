//! The player's builds on disk: each a TOML file of its own in the builds folder (`crate::paths::builds`), a
//! folder per game (`builds/exe6/<name>.toml`). A build file names its game and the build, then states the
//! side under `[side]` as a match file states one (`nettai_match::file`: the same names, written by the app or
//! by hand):
//!
//! ```toml
//! game = "exe6"
//! name = "Tengu rush"
//!
//! [side]
//! navi = "tenguman"
//! version = "falzar"
//! folder = [...]
//!
//! [aside]
//! navicust_programs = [...]
//! patch_cards = [...]
//! ```
//!
//! **The name is the player's own.** It is the file's alone: `[side]` is what a match, a lobby and a replay are
//! given, and it holds no name.
//!
//! **What is kept aside.** Some of a build's facts are its game's own navi's alone, the one that changes form
//! (MegaMan's NaviCust, his patch cards, his auto battle data: the editor's tabs say which facts are theirs,
//! `crate::screens::builds::tabs`). While a build's navi is another, those facts are kept under `[aside]`,
//! outside the side (the rules refuse a NaviCust on a navi that compiles none, and a side states nothing that
//! isn't its navi's), and they are put back when the game's own navi is chosen again ([`Build::set_navi`]).
//!
//! A build made new is `nettai_match::build::Builds::new_build` (the game's own navi, a version, the folder a
//! new save starts with); there are no random builds.

use nettai_battle::Content;
use nettai_match::build::Builds;
use nettai_match::facts::{Stated, fact_of};
use nettai_match::file::SideFile;
use nettai_match::{Facts, Side};
use nettai_content_api::NaviHandle;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The most letters a build's name has.
pub const NAME_LENGTH: usize = 32;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildFile {
    game: String,
    name: String,
    #[serde(default)]
    side: SideFile,
    #[serde(default)]
    aside: SideFile,
}

/// A build: its name, the side a match is handed, and the facts kept aside while its navi takes none of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Build {
    pub name: String,
    pub side: Side,
    /// The facts kept aside, stated; every other one the rules' default.
    aside: Facts,
}

/// A build's file, as it is listed: its path, its name, and its text (read against its game's content once
/// that is loaded: [`read`]).
#[derive(Clone, Debug)]
pub struct Listed {
    pub path: PathBuf,
    pub name: String,
    pub text: String,
}

/// A fact's value as the facts' writer takes it: an element each for a list.
fn values(value: &Stated) -> Vec<nettai_battle::rules::Fact<'_>> {
    match fact_of(value) {
        nettai_battle::rules::Fact::List(items) => items,
        fact => vec![fact],
    }
}

impl Build {
    /// A build named `name` of `side`, nothing kept aside.
    pub fn new(content: &Content, name: &str, side: Side) -> Build {
        Build { name: name.to_string(), side, aside: Facts::defaults(content) }
    }

    /// The build under another name.
    pub fn named(&self, name: &str) -> Build {
        Build { name: name.to_string(), ..self.clone() }
    }

    /// Whether the build's navi is its game's own, the one that changes form (the one a NaviCust, patch cards
    /// and auto battle data are of).
    pub fn own_navi(&self, content: &Content) -> bool {
        self.side.stated_navi(content).is_some_and(|n| content.navi(n).changes_form())
    }

    /// Whether fact `field` is kept aside.
    #[allow(dead_code)] // (The tabs to come show what they keep.)
    pub fn keeps_aside(&self, content: &Content, field: &str) -> bool {
        !self.aside.is_default(content, field)
    }

    /// Where each of `fields` (the facts that are the game's own navi's alone) belongs for the build's navi:
    /// in the side if the navi is the game's own (what was kept aside is put back), else aside (the side's
    /// own back at the rules' default). Whether anything moved.
    pub fn settle_aside(&mut self, content: &Content, fields: &[String]) -> bool {
        let own = self.own_navi(content);
        let (from, to) = if own { (&mut self.aside, &mut self.side.facts) } else { (&mut self.side.facts, &mut self.aside) };
        let mut moved = false;
        for field in fields {
            if from.is_default(content, field) {
                continue;
            }
            let Some(value) = from.get(content, field) else { continue };
            // (What was kept aside goes back only where the side states none of its own.)
            if (!own || to.is_default(content, field)) && to.set(content, field, &values(&value)).is_err() {
                continue;
            }
            from.reset(content, field);
            moved = true;
        }
        moved
    }

    /// Choose the build's navi, as a build is ([`Builds::set_navi`]: its level at its last, its own forms of
    /// the build's version, its choices settled), each of `fields` kept aside or put back as the navi takes it
    /// ([`Build::settle_aside`]).
    pub fn set_navi(&mut self, content: &Content, builds: &Builds, navi: NaviHandle, fields: &[String]) -> Result<(), String> {
        let mut made = self.clone();
        // (Aside first where the navi takes none, back last where it does: the side is never one the rules
        // refuse on the way.)
        made.side.set_navi(content, navi)?;
        made.settle_aside(content, fields);
        builds.set_navi(content, &mut made.side, navi)?;
        *self = made;
        Ok(())
    }

    /// Switch the build to version `version` (docs/app.md §8, "A build's version"; the user: "you should allow
    /// switching a save between game versions actually"): the facts the version sets are the new version's
    /// (`Builds::take_version`: EXE6's version, and its MegaMan's Crosses with it; EXE4's and EXE5's souls and the
    /// Giga chips a save of it can have), and everything else is kept as it is: the navi, the folder, the
    /// NaviCust, the patch cards, the auto battle data, the choices, what is kept aside. What no longer fits (the
    /// other version's Giga chips in the folder) stays, and is the build's problem as any rule's break is.
    /// Whether it changed.
    pub fn set_version(&mut self, content: &Content, builds: &Builds, version: &str) -> Result<bool, String> {
        let before = self.side.clone();
        builds.take_version(content, &mut self.side, version)?;
        Ok(self.side != before)
    }

    /// Set the build to what every build is ([`Builds::as_built`]: the facts no build states back at the
    /// rules' defaults, its choices settled), with each of `fields` where its navi takes it. Whether it
    /// changed: a build read from a file or made from a save is settled before it is edited or kept.
    pub fn settle(&mut self, content: &Content, builds: &Builds, fields: &[String]) -> bool {
        let before = self.clone();
        self.settle_aside(content, fields);
        builds.as_built(content, &mut self.side);
        *self != before
    }
}

/// A build's name as it is typed: one line, no room at its ends, [`NAME_LENGTH`] letters at most.
pub fn clean_name(text: &str) -> String {
    let line: String = text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    line.trim_start().chars().take(NAME_LENGTH).collect()
}

/// The folder of game `game`'s builds.
pub fn folder(game: &str) -> PathBuf {
    crate::paths::builds().join(game)
}

/// Game `game`'s builds, by name: each file of its folder that is a build of it (a file that isn't one is
/// left out).
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
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then_with(|| a.path.cmp(&b.path)));
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

/// A build file's text read against its game's content: the build, or what keeps it from being one (a name
/// the game hasn't), in the log's words.
pub fn read(content: &Content, game: &str, text: &str) -> Result<Build, String> {
    let f: BuildFile = toml::from_str(text).map_err(|e| e.message().to_string())?;
    if f.game != game {
        return Err(format!("a build of {}, not of {game}", f.game));
    }
    let mut problems = Vec::new();
    let side = nettai_match::file::resolve_side(content, game, &f.side, "the build", &mut problems);
    let aside = nettai_match::file::resolve_facts(content, game, &f.aside, "what the build keeps aside", &mut problems);
    match (side, aside) {
        (Some(side), Some(aside)) => Ok(Build { name: f.name, side, aside }),
        _ => Err(problems.join("; ")),
    }
}

/// A build as its file's text.
pub fn text(content: &Content, game: &str, build: &Build) -> String {
    #[derive(Serialize)]
    struct Header<'a> {
        game: &'a str,
        name: &'a str,
    }
    let head = toml::to_string(&Header { game, name: &build.name }).expect("a header serializes");
    let mut text = format!("# A nettai build (docs/app.md §8).\n\n{head}\n{}", nettai_match::file::side_toml(content, "side", &build.side));
    if build.aside != Facts::defaults(content) {
        text.push_str("\n# Kept while the build's navi takes none of it.\n");
        text.push_str(&nettai_match::file::side_toml(content, "aside", &Side { facts: build.aside.clone() }));
    }
    text
}

/// Write a build to `path` (its folder made if it isn't there).
pub fn write(path: &Path, content: &Content, game: &str, build: &Build) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(path, text(content, game, build)).map_err(|e| format!("{}: {e}", path.display()))
}

/// A path for a new build of `game` named `name` that no file has: the name's letters and digits
/// (`falzar-heat.toml`), numbered if it is taken.
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

/// A name for a new build that none of `taken` has: `base`, else `base 2`, and so on.
pub fn new_name(base: &str, taken: &[String]) -> String {
    let free = |n: &str| !taken.iter().any(|t| t.to_lowercase() == n.to_lowercase());
    if free(base) {
        return base.to_string();
    }
    (2..).map(|n| format!("{base} {n}")).find(|n| free(n)).expect("a free name")
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::Wrong;
    use nettai_match::build::Checked;
    use nettai_match::testing::{PlacedProgram, exe5_content, exe6_content, patch_cards, set_navicust};

    /// The facts the tests keep aside: what the editor's tabs name for the game's own navi.
    fn fields() -> Vec<String> {
        ["navicust_programs", "patch_cards", "auto_battle_places"].map(String::from).to_vec()
    }

    /// A build's file reads back as the build it was written from; one of another game, or naming what the
    /// game hasn't, isn't one. Its name is the file's alone: the side says none.
    #[test]
    fn a_build_reads_back() {
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let build = Build::new(&content, "Falzar \"heat\"", builds.new_build(&content, "falzar").unwrap());
        let text = text(&content, "exe6", &build);
        assert!(text.contains("[side]") && !text.contains("[aside]"), "{text}");
        assert_eq!(header(&text), Ok(("exe6".into(), "Falzar \"heat\"".into())));
        assert_eq!(read(&content, "exe6", &text), Ok(build.clone()));
        assert!(read(&content, "exe5", &text).unwrap_err().contains("not of exe5"));
        let wrong = text.replacen("navi = \"megaman\"", "navi = \"nobody\"", 1);
        assert_ne!(wrong, text);
        assert!(read(&content, "exe6", &wrong).is_err());
        let side = nettai_match::file::side_toml(&content, "side", &build.side);
        assert!(!side.contains("Falzar") && !side.contains("name"), "the side a match is given says no name: {side}");
    }

    /// A new build is its game's own navi's, of the version asked for, with the folder a new save starts
    /// with, and nothing wrong with it.
    #[test]
    fn a_new_build_is_ready() {
        for (content, game) in [(exe6_content(), "exe6"), (exe5_content(), "exe5")] {
            let builds = Builds::of(&content, game).unwrap();
            for version in &builds.versions {
                let build = Build::new(&content, "New", builds.new_build(&content, &version.id).unwrap());
                assert!(build.own_navi(&content));
                assert_eq!(builds.version_of(&content, &build.side), Some(version.id.as_str()));
                assert_eq!(build.side.folder(&content), builds.new_folder);
                assert!(Checked::of(&content, game, &build.side).ready());
            }
        }
    }

    /// While another navi is chosen, the game's own navi's NaviCust and patch cards are kept aside, outside
    /// the side (which the rules then find nothing wrong with), written to the file and read back; they are
    /// in the side again when the game's own navi is chosen.
    #[test]
    fn the_own_navis_facts_are_kept_aside() {
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let mut side = builds.new_build(&content, "falzar").unwrap();
        let program = nettai_match::ids::entry(&content, "exe6", "navicust_programs", "suprarmr").unwrap();
        set_navicust(&content, &mut side, &[PlacedProgram { program, color: 0, x: 3, y: 3, rotation: 0, compressed: true }], 2);
        side.set_fact(&content, "patch_cards", &patch_cards(&content, "exe6", "skarab")).unwrap();
        let megaman = side.navi(&content);
        let with = Build::new(&content, "Heat", side);
        let stated = |b: &Build, field: &str| !b.side.facts.is_default(&content, field);
        assert!(stated(&with, "navicust_programs") && stated(&with, "patch_cards"));

        // Left in the side, the rules refuse the NaviCust on a navi that compiles none.
        let tenguman = nettai_match::ids::navi(&content, "exe6", "tenguman").unwrap();
        let mut left = with.side.clone();
        builds.set_navi(&content, &mut left, tenguman).unwrap();
        assert!(Checked::of(&content, "exe6", &left).problems.iter().any(|p| p.what == Wrong::NotTaken));

        let mut build = with.clone();
        build.set_navi(&content, &builds, tenguman, &fields()).unwrap();
        assert!(!build.own_navi(&content));
        assert!(!stated(&build, "navicust_programs") && !stated(&build, "patch_cards"), "outside the side");
        assert!(build.keeps_aside(&content, "navicust_programs") && build.keeps_aside(&content, "patch_cards"));
        assert!(Checked::of(&content, "exe6", &build.side).ready());
        let text = text(&content, "exe6", &build);
        assert!(text.contains("[aside]"), "{text}");
        assert_eq!(read(&content, "exe6", &text), Ok(build.clone()));
        // (Another navi again: what is aside stays.)
        let protoman = nettai_match::ids::navi(&content, "exe6", "protoman").unwrap();
        build.set_navi(&content, &builds, protoman, &fields()).unwrap();
        assert!(build.keeps_aside(&content, "navicust_programs"));

        build.set_navi(&content, &builds, megaman, &fields()).unwrap();
        assert_eq!(build, with, "back as it was");
        assert!(!super::text(&content, "exe6", &build).contains("[aside]"));
    }

    /// A build read with a navi that takes none of what its side states (written by hand) is settled: those
    /// facts aside.
    #[test]
    fn a_build_is_settled() {
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let mut side = builds.new_build(&content, "gregar").unwrap();
        side.set_fact(&content, "patch_cards", &patch_cards(&content, "exe6", "skarab")).unwrap();
        let heatman = nettai_match::ids::navi(&content, "exe6", "heatman").unwrap();
        builds.set_navi(&content, &mut side, heatman).unwrap();
        let mut build = Build::new(&content, "By hand", side);
        assert!(build.settle(&content, &builds, &fields()));
        assert!(build.side.facts.is_default(&content, "patch_cards") && build.keeps_aside(&content, "patch_cards"));
        assert!(!build.settle(&content, &builds, &fields()), "once is enough");
    }

    /// A folder's first entry made `chip` (in its first code).
    fn with_first(content: &Content, side: &mut Side, chip: nettai_content_api::ChipHandle) {
        let mut folder = side.folder(content);
        folder.chips[0] = Some(nettai_battle::custom::FolderChip::new(chip, content.chip(chip).codes[0]));
        side.set_folder(content, &folder).unwrap();
    }

    /// The forms a side states of `fact`, by their keys.
    fn forms(content: &Content, side: &Side, fact: &str) -> Vec<String> {
        let list = side.facts.get(content, fact).map(|v| v.defs()).unwrap_or_default();
        list.iter().map(|&h| nettai_match::ids::local(&content.defs.form(nettai_content_api::FormHandle(h)).key).to_string()).collect()
    }

    /// EXE6: a Gregar build with Gregar's own Giga chip (ColForce) switched to Falzar keeps its folder, its
    /// NaviCust and its patch cards; its Crosses are Falzar's (the version's own, as a new Falzar build's); the
    /// Giga chip stays, and is the build's one problem, the folder's first entry's. Switched back, it is as it
    /// was, with nothing wrong. A build of another navi switches too, its Crosses none and what is aside kept,
    /// and a switched build's file reads back as it.
    #[test]
    fn exe6_a_version_switch_keeps_the_build() {
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let mut side = builds.new_build(&content, "gregar").unwrap();
        let colforce = nettai_match::ids::chip(&content, "exe6", "colforce").unwrap();
        with_first(&content, &mut side, colforce);
        let program = nettai_match::ids::entry(&content, "exe6", "navicust_programs", "suprarmr").unwrap();
        set_navicust(&content, &mut side, &[PlacedProgram { program, color: 0, x: 3, y: 3, rotation: 0, compressed: true }], 2);
        side.set_fact(&content, "patch_cards", &patch_cards(&content, "exe6", "skarab")).unwrap();
        let gregar = Build::new(&content, "Gregar", side);
        assert!(Checked::of(&content, "exe6", &gregar.side).ready());

        let mut build = gregar.clone();
        assert_eq!(build.set_version(&content, &builds, "falzar"), Ok(true));
        assert_eq!(builds.version_of(&content, &build.side), Some("falzar"));
        assert_eq!(forms(&content, &build.side, "crosses"), forms(&content, &builds.new_build(&content, "falzar").unwrap(), "crosses"));
        for kept in ["navi", "folder", "navicust_programs", "navicust_expansions", "patch_cards", "bug_frags", "beast_out"] {
            assert_eq!(build.side.facts.get(&content, kept), gregar.side.facts.get(&content, kept), "{kept} is kept");
        }
        let problems = Checked::of(&content, "exe6", &build.side).problems;
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert_eq!((&problems[0].what, problems[0].entry, problems[0].field.as_deref()), (&Wrong::ChipOfAnotherVersion { chip: colforce }, Some(0), Some("folder")));
        assert_eq!(read(&content, "exe6", &text(&content, "exe6", &build)), Ok(build.clone()));
        assert_eq!(build.set_version(&content, &builds, "falzar"), Ok(false), "the same version changes nothing");
        assert_eq!(build.set_version(&content, &builds, "gregar"), Ok(true));
        assert_eq!(build, gregar, "back as it was");
        assert!(build.set_version(&content, &builds, "nowhere").is_err());

        // Another navi's build: the version alone, what is aside kept aside.
        let tenguman = nettai_match::ids::navi(&content, "exe6", "tenguman").unwrap();
        build.set_navi(&content, &builds, tenguman, &fields()).unwrap();
        let aside = build.clone();
        assert_eq!(build.set_version(&content, &builds, "falzar"), Ok(true));
        assert!(forms(&content, &build.side, "crosses").is_empty() && build.keeps_aside(&content, "patch_cards"));
        assert_eq!(build.side.folder(&content), aside.side.folder(&content));
        assert!(Checked::of(&content, "exe6", &build.side).problems.iter().all(|p| p.what == Wrong::ChipOfAnotherVersion { chip: colforce }));
    }

    /// EXE5 and EXE4: a build of one version with its own Giga chip and a dark MegaMan switched to the other has
    /// the other's souls and Giga chips, keeps its folder, its navi and its light or dark; its Giga chip stays,
    /// the build's one problem. Switched back, it is as it was.
    #[test]
    fn exe5_and_exe4_a_version_switch_keeps_the_build() {
        let four = nettai_match::testing::exe4_content();
        for (content, game, [from, to], giga) in [(exe5_content(), "exe5", ["protoman", "colonel"], "dethphnx"), (four, "exe4", ["redsun", "bluemoon"], "redsun")] {
            let builds = Builds::of(&content, game).unwrap();
            let mut side = builds.new_build(&content, from).unwrap();
            let chip = nettai_match::ids::chip(&content, game, giga).unwrap();
            with_first(&content, &mut side, chip);
            builds.take_preset(&content, &mut side, "light_dark", "dark").unwrap();
            let before = Build::new(&content, "Team", side);
            assert!(Checked::of(&content, game, &before.side).ready(), "{game}");

            let mut build = before.clone();
            assert_eq!(build.set_version(&content, &builds, to), Ok(true));
            let fresh = builds.new_build(&content, to).unwrap();
            assert_eq!(builds.version_of(&content, &build.side), Some(to), "{game}");
            assert_eq!(forms(&content, &build.side, "souls"), forms(&content, &fresh, "souls"), "{game}");
            assert_eq!(build.side.facts.get(&content, "giga_chips"), fresh.facts.get(&content, "giga_chips"), "{game}");
            assert_eq!(build.side.folder(&content), before.side.folder(&content), "{game}");
            assert_eq!(builds.preset_of(&content, &build.side, "light_dark").map(|p| p.id.as_str()), Some("dark"), "{game}");
            let problems = Checked::of(&content, game, &build.side).problems;
            assert_eq!(problems.len(), 1, "{game}: {problems:?}");
            assert_eq!((&problems[0].what, problems[0].entry), (&Wrong::ChipOfAnotherVersion { chip }, Some(0)), "{game}");
            assert_eq!(build.set_version(&content, &builds, from), Ok(true));
            assert_eq!(build, before, "{game}: back as it was");
        }
    }

    #[test]
    fn new_names_and_paths() {
        assert_eq!(new_name("Falzar", &["falzar".into(), "Falzar 2".into()]), "Falzar 3");
        assert_eq!(new_name("Gregar", &["falzar".into()]), "Gregar");
        let path = new_path("exe6", "  Heat / Elec!! ");
        assert_eq!(path.file_name().and_then(|n| n.to_str()), Some("heat-elec.toml"));
        assert_eq!(clean_name("  A\tname\n"), "A name ");
        assert_eq!(clean_name(&"x".repeat(80)).chars().count(), NAME_LENGTH);
    }
}
