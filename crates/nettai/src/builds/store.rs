//! The player's builds on disk: each a TOML file of its own in the builds folder (`crate::paths::builds`), a
//! folder per game (`builds/exe6/<name>.nettaibuild`). A build file names its game and the build, then states the
//! side under `[side]` as a match file states one (`nettai_match::file`: the same names, written by the app or
//! by hand):
//!
//! ```toml
//! game = "exe6"
//! name = "Tengu rush"
//! ruleset = "preset:masters"
//!
//! [side]
//! navi = "tenguman"
//! cybeast = "falzar-beast"
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
//! **The ruleset it is made for** (the user: "the build in the menu should indicate the ruleset it's for"): a
//! preset's id or the player's own ruleset's name, by kind (`preset:<id>`, `own:<name>`: `crate::rulesets::name_text`),
//! what it is checked under outside a lobby (`crate::rulesets::made_for`). It is the file's alone too, as the name
//! is. A file that names none (written before builds named theirs) is made for the game's first preset; one that
//! names a ruleset no longer there says it is missing, and is checked under that preset. The player's own renamed
//! is renamed in the builds made for it ([`rename_ruleset`]).
//!
//! **What is kept aside.** Some of a build's facts are its game's own navi's alone, the one that changes form
//! (MegaMan's NaviCust, his patch cards, his auto battle data: the editor's tabs say which facts are theirs,
//! `crate::screens::builds::tabs`). While a build's navi is another, those facts are kept under `[aside]`,
//! outside the side (the rules refuse a NaviCust on a navi that compiles none, and a side states nothing that
//! isn't its navi's), and they are put back when the game's own navi is chosen again ([`Build::set_navi`]).
//!
//! A build made new is `nettai_match::build::Builds::new_build` (the game's own navi, a version's group's forms, the
//! folder a new save starts with); there are no random builds. A build states no version (docs/design/versions.md
//! §2.1): a file written before it did (its `version`) is no build now, and the list says it can't be read (§3.2:
//! old builds are dropped, not converted).

use nettai_battle::Content;
use nettai_match::build::Builds;
use nettai_match::facts::{Stated, fact_of};
use nettai_match::file::SideFile;
use nettai_match::ruleset::Name;
use nettai_match::{Facts, Side};
use nettai_content_api::NaviHandle;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The most letters a build's name has.
pub const NAME_LENGTH: usize = 32;

/// A build file's extension (its text is TOML).
pub const EXTENSION: &str = "nettaibuild";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildFile {
    game: String,
    name: String,
    #[serde(default)]
    ruleset: Option<String>,
    #[serde(default)]
    side: SideFile,
    #[serde(default)]
    aside: SideFile,
}

/// A build: its name, the ruleset it is made for, the side a match is handed, and the facts kept aside while its
/// navi takes none of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Build {
    pub name: String,
    /// The ruleset it is made for, by name (`crate::rulesets::made_for`). None: the game's first preset (a file
    /// that names none).
    pub ruleset: Option<Name>,
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
    /// A build named `name` of `side`, made for the game's first preset, nothing kept aside.
    pub fn new(content: &Content, name: &str, side: Side) -> Build {
        Build { name: name.to_string(), ruleset: None, side, aside: Facts::defaults(content) }
    }

    /// The build made for ruleset `ruleset` (none: the game's first preset).
    pub fn made_for(self, ruleset: Option<Name>) -> Build {
        Build { ruleset, ..self }
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

    /// Choose the build's navi, as a build is ([`Builds::set_navi`]: its level at its last, its form list kept or
    /// none, its choices settled), each of `fields` kept aside or put back as the navi takes it
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
    list_in(&folder(game), game)
}

/// [`list`] of folder `dir`.
fn list_in(dir: &Path, game: &str) -> Vec<Listed> {
    let Ok(dir) = nettai_files::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<Listed> = dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == EXTENSION))
        .filter_map(|path| {
            let text = nettai_files::read_to_string(&path).ok()?;
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
    let ruleset = match f.ruleset {
        Some(text) => Some(crate::rulesets::name_of_text(&text).ok_or_else(|| format!("ruleset {text:?}: \"preset:<id>\" or \"own:<name>\""))?),
        None => None,
    };
    match (side, aside) {
        (Some(side), Some(aside)) => Ok(Build { name: f.name, ruleset, side, aside }),
        _ => Err(problems.join("; ")),
    }
}

/// A build as its file's text.
pub fn text(content: &Content, game: &str, build: &Build) -> String {
    #[derive(Serialize)]
    struct Header<'a> {
        game: &'a str,
        name: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        ruleset: Option<String>,
    }
    let ruleset = build.ruleset.as_ref().map(crate::rulesets::name_text);
    let head = toml::to_string(&Header { game, name: &build.name, ruleset }).expect("a header serializes");
    let mut text = format!("{head}\n{}", nettai_match::file::side_toml(content, "side", &build.side));
    if build.aside != Facts::defaults(content) {
        text.push_str("\n# Kept while the build's navi takes none of it.\n");
        text.push_str(&nettai_match::file::side_toml(content, "aside", &Side { facts: build.aside.clone() }));
    }
    text
}

/// Write a build to `path` (its folder made if it isn't there).
pub fn write(path: &Path, content: &Content, game: &str, build: &Build) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        nettai_files::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    nettai_files::write(path, text(content, game, build)).map_err(|e| format!("{}: {e}", path.display()))
}

/// Ruleset `from` renamed `to` in each of game `game`'s builds made for it: its file read on `content` and written
/// again. Each file that names it and couldn't be, with why (it doesn't read as a build on this content, or
/// can't be written).
pub fn rename_ruleset(content: &Content, game: &str, from: &Name, to: &Name) -> Vec<(PathBuf, String)> {
    rename_ruleset_in(&folder(game), content, game, from, to)
}

/// [`rename_ruleset`] in folder `dir`.
fn rename_ruleset_in(dir: &Path, content: &Content, game: &str, from: &Name, to: &Name) -> Vec<(PathBuf, String)> {
    let mut failed = Vec::new();
    for listed in list_in(dir, game) {
        let named = toml::from_str::<BuildFile>(&listed.text).ok().and_then(|f| f.ruleset).and_then(|r| crate::rulesets::name_of_text(&r));
        if named.as_ref() != Some(from) {
            continue;
        }
        let renamed = read(content, game, &listed.text).and_then(|b| write(&listed.path, content, game, &b.made_for(Some(to.clone()))));
        if let Err(why) = renamed {
            failed.push((listed.path, why));
        }
    }
    failed
}

/// A path for a new build of `game` named `name` that no file has: the name's letters and digits
/// (`falzar-heat.nettaibuild`), numbered if it is taken.
pub fn new_path(game: &str, name: &str) -> PathBuf {
    path_for(game, name, None)
}

/// [`new_path`] for build `name` of `game` kept at `own` until now (renamed): its own file counts as free, so a name
/// whose letters and digits are the same keeps it.
pub fn renamed_path(game: &str, name: &str, own: &Path) -> PathBuf {
    path_for(game, name, Some(own))
}

fn path_for(game: &str, name: &str, own: Option<&Path>) -> PathBuf {
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
    let mut path = dir.join(format!("{stem}.{EXTENSION}"));
    for n in 2.. {
        if !nettai_files::exists(&path) || own == Some(path.as_path()) {
            break;
        }
        path = dir.join(format!("{stem}-{n}.{EXTENSION}"));
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

    /// A new build is its game's own navi's, from the group asked for, with the folder a new save starts with,
    /// and nothing wrong with it.
    #[test]
    fn a_new_build_is_ready() {
        for (content, game) in [(exe6_content(), "exe6"), (exe5_content(), "exe5")] {
            let builds = Builds::of(&content, game).unwrap();
            for group in &builds.groups {
                let build = Build::new(&content, "New", builds.new_build(&content, &group.id).unwrap());
                assert!(build.own_navi(&content));
                assert_eq!(build.side.folder(&content), builds.new_folder);
                assert!(Checked::of(&content, game, &build.side).ready());
            }
        }
    }

    /// A build file written before builds stated no version (its side's `version`, a card a name and no record)
    /// is no build: it can't be read, and says why; nothing stops (docs/design/versions.md §3.2).
    #[test]
    fn a_stale_build_is_unreadable() {
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let text = text(&content, "exe6", &Build::new(&content, "Old", builds.new_build(&content, "falzar").unwrap()));
        let old = text.replacen("cybeast = \"falzar-beast\"", "version = \"falzar\"", 1);
        assert_ne!(old, text);
        assert!(read(&content, "exe6", &old).unwrap_err().contains("version"));
        let cards = text.replacen("[side]\n", "[side]\npatch_cards = [\"canodumb\"]\n", 1);
        assert!(read(&content, "exe6", &cards).is_err());
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

    /// The selector's choices keep the rest of the build: EXE6's Cybeast changed, the build keeps its Crosses, its
    /// folder, its NaviCust and its patch cards (a card's variant its own), and its file reads back as it.
    #[test]
    fn a_chosen_form_keeps_the_build() {
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let mut side = builds.new_build(&content, "gregar").unwrap();
        let program = nettai_match::ids::entry(&content, "exe6", "navicust_programs", "suprarmr").unwrap();
        set_navicust(&content, &mut side, &[PlacedProgram { program, color: 0, x: 3, y: 3, rotation: 0, compressed: true }], 2);
        side.set_fact(&content, "patch_cards", &patch_cards(&content, "exe6", "skarab,bass-cross-megaman/gregar")).unwrap();
        let gregar = Build::new(&content, "Gregar", side);
        let mut build = gregar.clone();
        let falzar_beast = nettai_match::ids::form(&content, "exe6", "falzar-beast").unwrap();
        assert!(builds.choose_form(&content, &mut build.side, "cybeast", falzar_beast));
        assert_eq!(build.side.version(&content), Some("falzar"));
        for kept in ["navi", "folder", "crosses", "navicust_programs", "navicust_expansions", "patch_cards", "bug_frags", "beast_out"] {
            assert_eq!(build.side.facts.get(&content, kept), gregar.side.facts.get(&content, kept), "{kept} is kept");
        }
        assert_eq!(read(&content, "exe6", &text(&content, "exe6", &build)), Ok(build.clone()));
        let gregar_beast = nettai_match::ids::form(&content, "exe6", "gregar-beast").unwrap();
        assert!(builds.choose_form(&content, &mut build.side, "cybeast", gregar_beast));
        assert_eq!(build, gregar, "back as it was");
    }

    /// The Navi tab's ALL and NONE: a version's Crosses taken out at once, the other's put in up to the Cross
    /// window's five, the rest refused for want of room; nothing done is no change.
    #[test]
    fn all_and_none_of_a_version() {
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let mut side = builds.new_build(&content, "gregar").unwrap();
        let version_of = |f: &nettai_content_api::FormHandle| content.defs.forms[f.index()].record.version.clone();
        let crosses = builds.form_choices(&content, &side).into_iter().find(|c| c.fact == "crosses").unwrap();
        let of = |v: &str| -> Vec<_> { crosses.forms.iter().copied().filter(|f| version_of(f).as_deref() == Some(v)).collect() };
        let (gregar, falzar) = (of("gregar"), of("falzar"));
        assert_eq!(builds.chosen_forms(&content, &side, "crosses"), gregar);
        assert_eq!(builds.choose_forms(&content, &mut side, "crosses", &falzar, true), (false, 5), "no room");
        assert_eq!(builds.choose_forms(&content, &mut side, "crosses", &gregar, false), (true, 0));
        assert!(builds.chosen_forms(&content, &side, "crosses").is_empty());
        assert_eq!(builds.choose_forms(&content, &mut side, "crosses", &gregar, false), (false, 0));
        assert!(builds.choose_form(&content, &mut side, "crosses", gregar[4]));
        assert_eq!(builds.choose_forms(&content, &mut side, "crosses", &falzar, true), (true, 1));
        // (In the battle's order: Gregar's, then Falzar's first four.)
        assert_eq!(builds.chosen_forms(&content, &side, "crosses"), [&gregar[4..], &falzar[..4]].concat());
        let cybeast = builds.chosen_forms(&content, &side, "cybeast");
        assert_eq!(builds.choose_forms(&content, &mut side, "cybeast", &cybeast, false), (false, 0), "one form is no list");
    }

    /// A build names the ruleset it is made for in its file, by kind, and reads back made for it; a file that names
    /// none (written before builds named theirs) reads as made for the game's first preset (none named), and one
    /// that names it in no known way isn't a build.
    #[test]
    fn a_build_names_its_ruleset() {
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let plain = Build::new(&content, "Heat", builds.new_build(&content, "falzar").unwrap());
        let text_of = |b: &Build| text(&content, "exe6", b);
        assert!(!text_of(&plain).contains("ruleset"), "{}", text_of(&plain));
        assert_eq!(read(&content, "exe6", &text_of(&plain)).unwrap().ruleset, None);
        for name in [Name::Preset("senior".into()), Name::Own("Gregar \"cup\"".into())] {
            let build = plain.clone().made_for(Some(name.clone()));
            let text = text_of(&build);
            assert!(text.contains("\nruleset = ") && text.contains(&crate::rulesets::name_text(&name)), "{text}");
            assert_eq!(read(&content, "exe6", &text), Ok(build.clone()));
            assert_eq!(read(&content, "exe6", &text).unwrap().named("Copy").ruleset, Some(name), "a duplicate keeps it");
        }
        let senior = text_of(&plain.clone().made_for(Some(Name::Preset("senior".into()))));
        assert!(read(&content, "exe6", &senior.replacen("preset:senior", "senior", 1)).unwrap_err().contains("ruleset"));
    }

    /// The player's own ruleset renamed is renamed in the files of the builds made for it, and in no other.
    #[test]
    fn a_renamed_ruleset_is_followed() {
        let dir = tempfile::tempdir().unwrap();
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let side = builds.new_build(&content, "gregar").unwrap();
        let (cup, renamed, other) = (Name::Own("Cup".into()), Name::Own("Spring cup".into()), Name::Own("Other".into()));
        let made = [("For the cup", Some(cup.clone())), ("For another", Some(other.clone())), ("For the first", None)];
        for (at, (name, ruleset)) in made.iter().enumerate() {
            let build = Build::new(&content, name, side.clone()).made_for(ruleset.clone());
            write(&dir.path().join(format!("{at}.{EXTENSION}")), &content, "exe6", &build).unwrap();
        }
        assert!(rename_ruleset_in(dir.path(), &content, "exe6", &cup, &renamed).is_empty());
        let now: Vec<(String, Option<Name>)> = list_in(dir.path(), "exe6").iter().map(|l| (l.name.clone(), read(&content, "exe6", &l.text).unwrap().ruleset)).collect();
        assert_eq!(now, [("For another".into(), Some(other)), ("For the cup".into(), Some(renamed)), ("For the first".into(), None)]);
    }

    #[test]
    fn new_names_and_paths() {
        assert_eq!(new_name("Falzar", &["falzar".into(), "Falzar 2".into()]), "Falzar 3");
        assert_eq!(new_name("Gregar", &["falzar".into()]), "Gregar");
        let path = new_path("exe6", "  Heat / Elec!! ");
        assert_eq!(path.file_name().and_then(|n| n.to_str()), Some("heat-elec.nettaibuild"));
        assert_eq!(clean_name("  A\tname\n"), "A name ");
        assert_eq!(clean_name(&"x".repeat(80)).chars().count(), NAME_LENGTH);
    }
}
