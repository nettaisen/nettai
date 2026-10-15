//! A game's builds as a list has them (Play's, and any other's that picks one: the UI's `BuildRow`,
//! build.slint): each file of the game's builds folder read on the game's content and checked, with its row's
//! words in the language shown ("MegaMan · ready", or how many problems, and its version's tag).

use crate::builds::store::{self, Build, Listed};
use crate::builds::words::Words;
use crate::builds::Kit;
use crate::games::Names;
use crate::shell::Shell;
use nettai_match::Side;
use nettai_match::build::Checked;
use nettai_ui::BuildEntry;
use slint::ComponentHandle;

/// One of a game's builds, as a list has it.
pub struct Shelved {
    /// Its file: where it is, and what it holds.
    pub listed: Listed,
    /// The build. None: its file doesn't read as one on this content (why is in the log).
    pub build: Option<Build>,
    /// Nothing is wrong with it: it can be played.
    pub ready: bool,
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

    /// It can be played: it reads, and nothing is wrong with it.
    pub fn usable(&self) -> bool {
        self.build.is_some() && self.ready
    }
}

/// `build` (the file `listed`'s) as a list has it.
pub fn shelved(words: &Words, listed: Listed, build: Option<Build>) -> Shelved {
    let kit = words.kit;
    let Some(build) = build else {
        let row = BuildEntry { name: listed.name.as_str().into(), status: words.ui.global::<nettai_ui::BuildWords>().invoke_unreadable(), bad: true, ..Default::default() };
        return Shelved { listed, build: None, ready: false, row };
    };
    let checked = Checked::of(kit.content(), &kit.game, &build.side);
    let navi = words.names.navi(build.side.navi(kit.content()));
    let (status, bad) = words.status(&navi, &checked);
    let row = BuildEntry { name: build.name.as_str().into(), status: status.into(), bad };
    Shelved { listed, ready: checked.ready(), build: Some(build), row }
}

/// The selected game's builds, by name, in the language shown.
pub fn load(shell: &Shell, kit: &Kit) -> Vec<Shelved> {
    let ui = shell.ui();
    let graphics = kit.ready.graphics(shell.lang);
    let names = Names::of(kit.content(), &graphics);
    let words = Words { ui: &ui, kit, names: &names };
    store::list(&kit.game)
        .into_iter()
        .map(|listed| {
            let build = store::read(kit.content(), &kit.game, &listed.text).map_err(|why| crate::errors::log(&format!("reading the build {}", listed.path.display()), &why)).ok();
            shelved(&words, listed, build)
        })
        .collect()
}
