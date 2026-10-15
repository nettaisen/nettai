//! A game's builds as a list has them (Play's, and any other's that picks one: the UI's `BuildRow`,
//! build.slint): each file of the game's builds folder read on the game's content and checked under a ruleset
//! (the one it is made for, or in a lobby the host's), with its row's words in the language shown ("MegaMan ·
//! ready", or how many problems, and the name of the ruleset it is made for).

use crate::builds::store::{self, Build, Listed};
use crate::builds::words::Words;
use crate::builds::Kit;
use crate::games::Names;
use crate::rulesets::{MadeFor, Pickable};
use crate::shell::Shell;
use nettai_match::Side;
use nettai_match::build::Checked;
use nettai_match::check::Problem;
use nettai_match::ruleset::{Name, Ruleset, Rulesets};
use nettai_ui::BuildEntry;
use slint::ComponentHandle;
use std::sync::Arc;

/// A ruleset a list checks its builds under, one of the game's `rulesets`: a lobby's.
#[derive(Clone, Copy)]
pub struct Under<'a> {
    pub rulesets: &'a Rulesets,
    pub ruleset: &'a Ruleset,
}

/// The rulesets a list's builds are made for, read once for the list: the game's (`Kit::rulesets`), and those the
/// player can pick (`rulesets::pickable`), of which each build's is (`rulesets::made_for`).
pub struct Own {
    rulesets: Arc<Rulesets>,
    picks: Vec<Pickable>,
}

impl Own {
    /// `kit`'s game's. None where it has no rulesets: its builds are checked by the game's rules alone.
    pub fn of(kit: &Kit) -> Option<Own> {
        Some(Own { rulesets: kit.rulesets()?, picks: crate::rulesets::pickable(kit) })
    }
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
    /// The ruleset it is made for, which its row names and it is checked under outside a lobby; and the game's
    /// rulesets. None where the game has none, or its file doesn't read.
    made_for: Option<(Arc<Rulesets>, MadeFor)>,
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

    /// The ruleset it is made for (none where the game has none, or its file doesn't read).
    pub fn made_for(&self) -> Option<&MadeFor> {
        self.made_for.as_ref().map(|(_, made)| made)
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

    /// Check it again under `under` (a lobby's ruleset; none: the one it is made for, or where there is none the
    /// game's rules alone), and its row's words with it. Its row names the ruleset it is made for, whichever.
    pub fn check_under(&mut self, words: &Words, under: Option<Under>) {
        let Some(build) = &self.build else { return };
        let kit = words.kit;
        let mut problems = self.rules.clone();
        let own = self.made_for.as_ref().map(|(rulesets, made)| Under { rulesets, ruleset: &made.pick.ruleset });
        if let Some(u) = under.or(own) {
            problems.extend(u.ruleset.problems(u.rulesets, kit.content(), &build.side));
        }
        let navi = words.names.navi(build.side.navi(kit.content()));
        let (status, bad) = words.status(&navi, &problems);
        let (ruleset, missing) = self.made_for.as_ref().map(|(_, made)| made.shown(words.names)).unwrap_or_default();
        self.row = BuildEntry { name: build.name.as_str().into(), status: status.into(), bad, ruleset: ruleset.into(), ruleset_missing: missing };
        self.problems = problems;
    }

    /// The player's own ruleset `from` renamed `to` (its files are: `rulesets::rename`): it is made for it by its
    /// new name where it was made for it; its row says so once it is checked again.
    pub fn follow_rename(&mut self, from: &Name, to: &Pickable) {
        let Some(build) = self.build.as_mut().filter(|b| b.ruleset.as_ref() == Some(from)) else { return };
        build.ruleset = Some(to.ruleset.name.clone());
        if let Some((_, made)) = &mut self.made_for {
            *made = MadeFor { pick: to.clone(), missing: None };
        }
    }
}

/// `build` (the file `listed`'s) as a list has it, made for its ruleset among `own`'s (none: the game has no
/// rulesets) and checked under it.
pub fn shelved(words: &Words, listed: Listed, build: Option<Build>, own: Option<&Own>) -> Shelved {
    let kit = words.kit;
    let Some(build) = build else {
        let row = BuildEntry { name: listed.name.as_str().into(), status: words.ui.global::<nettai_ui::BuildWords>().invoke_unreadable(), bad: true, ..Default::default() };
        return Shelved { listed, build: None, rules: Vec::new(), problems: Vec::new(), made_for: None, row };
    };
    let rules = Checked::of(kit.content(), &kit.game, &build.side).problems;
    let made_for = own.and_then(|own| Some((own.rulesets.clone(), crate::rulesets::made_for(&own.picks, build.ruleset.as_ref())?)));
    let mut shelved = Shelved { listed, build: Some(build), rules, problems: Vec::new(), made_for, row: BuildEntry::default() };
    shelved.check_under(words, None);
    shelved
}

/// The selected game's builds, by name, in the language shown, each checked under the ruleset it is made for.
pub fn load(shell: &Shell, kit: &Kit) -> Vec<Shelved> {
    let ui = shell.ui();
    let graphics = kit.ready.graphics(shell.lang);
    let names = Names::of(kit.content(), &graphics);
    let words = Words { ui: &ui, kit, names: &names };
    let own = Own::of(kit);
    store::list(&kit.game)
        .into_iter()
        .map(|listed| {
            let build = store::read(kit.content(), &kit.game, &listed.text).map_err(|why| crate::errors::log(&format!("reading the build {}", listed.path.display()), &why)).ok();
            shelved(&words, listed, build, own.as_ref())
        })
        .collect()
}
