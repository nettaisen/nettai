//! A game's builds as a list has them (Play's, and any other's that picks one: the UI's `BuildRow`,
//! build.slint): each file of the game's builds folder read on the game's content and checked under a ruleset
//! (the one the player plays by, or in a lobby the host's), with its row's words in the language shown
//! ("MegaMan · ready", or how many problems).

use crate::builds::store::{self, Build, Listed};
use crate::builds::words::Words;
use crate::builds::Kit;
use crate::games::Names;
use crate::shell::Shell;
use nettai_match::Side;
use nettai_match::build::Checked;
use nettai_match::check::Problem;
use nettai_match::ruleset::{Ruleset, Rulesets};
use nettai_ui::BuildEntry;
use slint::ComponentHandle;

/// The ruleset a list checks its builds under, one of the game's `rulesets`.
#[derive(Clone, Copy)]
pub struct Under<'a> {
    pub rulesets: &'a Rulesets,
    pub ruleset: &'a Ruleset,
}

/// `f` given the ruleset the player plays by in `kit`'s game (`rulesets::played_by`), where the game has
/// rulesets: what a list outside a lobby checks its builds under.
pub fn played_by<R>(shell: &Shell, kit: &Kit, f: impl FnOnce(Option<Under>) -> R) -> R {
    let (rulesets, played) = (kit.rulesets(), crate::rulesets::played_by(shell, kit));
    f(rulesets.as_deref().zip(played.as_ref()).map(|(rulesets, p)| Under { rulesets, ruleset: &p.ruleset }))
}

/// One of a game's builds, as a list has it.
pub struct Shelved {
    /// Its file: where it is, and what it holds.
    pub listed: Listed,
    /// The build. None: its file doesn't read as one on this content (why is in the log).
    pub build: Option<Build>,
    /// What is wrong with it by the game's rules alone (`Checked::of`'s problems), which a ruleset's are added
    /// to: kept, so that another ruleset checks it without its round set up again.
    rules: Vec<Problem>,
    /// What is wrong with it under the ruleset it is checked by (`Checked::under`'s problems). Empty: it can be
    /// played there.
    pub problems: Vec<Problem>,
    /// Its row.
    pub row: BuildEntry,
}

impl Shelved {
    /// Its name: the build's, or its file's where that doesn't read as one.
    pub fn name(&self) -> &str {
        self.build.as_ref().map_or(self.listed.name.as_str(), |b| b.name.as_str())
    }

    /// Its side, where its file reads on the game's content.
    pub fn side(&self) -> Option<&Side> {
        self.build.as_ref().map(|b| &b.side)
    }

    /// It can be played under the ruleset it is checked by: it reads, and nothing is wrong with it.
    pub fn usable(&self) -> bool {
        self.build.is_some() && self.problems.is_empty()
    }

    /// It can be played by the game's rules, whatever a ruleset holds back: what joining a lobby asks of a
    /// build, as the lobby's ruleset is the host's and the lobby checks the build under it.
    pub fn playable(&self) -> bool {
        self.build.is_some() && self.rules.is_empty()
    }

    /// Check it again under `under` (none: the game's rules alone), and its row's words with it.
    pub fn check_under(&mut self, words: &Words, under: Option<Under>) {
        let Some(build) = &self.build else { return };
        let kit = words.kit;
        let mut problems = self.rules.clone();
        if let Some(u) = under {
            problems.extend(u.ruleset.problems(u.rulesets, kit.content(), &build.side));
        }
        let navi = words.names.navi(build.side.navi(kit.content()));
        let (status, bad) = words.status(&navi, &problems);
        self.row = BuildEntry { name: build.name.as_str().into(), status: status.into(), bad };
        self.problems = problems;
    }
}

/// `build` (the file `listed`'s) as a list has it, checked under `under` (none: the game's rules alone).
pub fn shelved(words: &Words, listed: Listed, build: Option<Build>, under: Option<Under>) -> Shelved {
    let kit = words.kit;
    let Some(build) = build else {
        let row = BuildEntry { name: listed.name.as_str().into(), status: words.ui.global::<nettai_ui::BuildWords>().invoke_unreadable(), bad: true, ..Default::default() };
        return Shelved { listed, build: None, rules: Vec::new(), problems: Vec::new(), row };
    };
    let rules = Checked::of(kit.content(), &kit.game, &build.side).problems;
    let mut shelved = Shelved { listed, build: Some(build), rules, problems: Vec::new(), row: BuildEntry::default() };
    shelved.check_under(words, under);
    shelved
}

/// The selected game's builds, by name, in the language shown, checked under `under` (none: the game's rules
/// alone).
pub fn load(shell: &Shell, kit: &Kit, under: Option<Under>) -> Vec<Shelved> {
    let ui = shell.ui();
    let graphics = kit.ready.graphics(shell.lang);
    let names = Names::of(kit.content(), &graphics);
    let words = Words { ui: &ui, kit, names: &names };
    store::list(&kit.game)
        .into_iter()
        .map(|listed| {
            let build = store::read(kit.content(), &kit.game, &listed.text).map_err(|why| crate::errors::log(&format!("reading the build {}", listed.path.display()), &why)).ok();
            shelved(&words, listed, build, under)
        })
        .collect()
}
