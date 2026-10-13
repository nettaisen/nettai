//! A build's editor (docs/design/app-ui.md §6, "The editor"; docs/app.md §8): the build being edited
//! ([`Draft`]), the tabs ([`Tab`]), and the frame that hosts them ([`Editor`]: the header's name, version and
//! "✓ Ready to play", the tab bar, the tab shown).
//!
//! **A tab is one Slint file and one module here** (`ui/screens/builds/<tab>.slint`, `<tab>.rs`), as a screen
//! is one of each for the shell. Its Slint has its view (a global its module sets: its key hints and its
//! status among it) and its component, which the frame places under the tab bar. Its module has a struct that
//! implements [`Tab`]: the same few calls a `Screen` gets (`show`, `activate`, `step`, `choose`, `edit`,
//! `focused`, `key`, `button`, `action`, `back`, `answered`), each with the shell and the [`Draft`].
//!
//! **A tab changes the build through the draft** ([`Draft::edit`]: the side; [`Draft::choose_navi`]: its
//! navi). The draft is checked again at once, so the tab reads what is wrong and what the round starts with
//! straight after (`draft.checked`); and when the tab's call returns the frame saves the build's file and
//! shows the header and the tabs' marks anew. A tab neither saves nor checks.
//!
//! **A tab says which of the build's facts are its own** ([`Tab::fields`]): a problem with one of them marks
//! its word in the bar; a tab none of whose facts the game's setup takes isn't in the bar (auto battle data:
//! some games'); and the facts of a tab that is the game's own navi's alone ([`Tab::own_navi_only`]) are kept
//! aside in the build's file while its navi is another (`crate::builds::store`), the tab staying in its place,
//! greyed.

use crate::builds::store::{self, Build};
use crate::builds::words::Words;
use crate::builds::Kit;
use crate::errors::UserError;
use crate::games::Names;
use crate::input::{Action, KeyPress};
use crate::shell::Shell;
use nettai_battle::Content;
use nettai_content_api::NaviHandle;
use nettai_match::Side;
use nettai_match::build::Checked;
use nettai_ui::{BuildsView, EditorTab};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

/// The build being edited: what a tab is handed, with the shell.
pub struct Draft {
    /// The game's builds data, pictures and library order.
    pub kit: Rc<Kit>,
    pub build: Build,
    /// The build checked: what is wrong with it, and its round set up (the stats it starts with, the
    /// folder's view, the navi's first face). Made again at each change.
    pub checked: Checked,
    /// Where the build is kept.
    path: PathBuf,
    /// The facts that are the game's own navi's alone (the tabs say which).
    navi_only: Vec<String>,
    /// It changed since it was last saved.
    changed: bool,
}

impl Draft {
    pub fn content(&self) -> &Arc<Content> {
        self.kit.content()
    }

    /// The side a match is handed.
    pub fn side(&self) -> &Side {
        &self.build.side
    }

    /// Change the build's side: `change` says whether it did. The build is checked again at once; the frame
    /// saves it when the tab's call returns.
    pub fn edit(&mut self, change: impl FnOnce(&Content, &mut Side) -> bool) -> bool {
        let kit = self.kit.clone();
        if !change(kit.content(), &mut self.build.side) {
            return false;
        }
        self.recheck();
        true
    }

    /// Choose the build's navi, as a build is (`Build::set_navi`: what is the game's own navi's alone kept
    /// aside, or put back). Whether it changed.
    pub fn choose_navi(&mut self, navi: NaviHandle) -> bool {
        let kit = self.kit.clone();
        let before = self.build.clone();
        if let Err(why) = self.build.set_navi(kit.content(), &kit.builds, navi, &self.navi_only) {
            crate::errors::log("choosing a build's navi", &why);
            return false;
        }
        if self.build == before {
            return false;
        }
        self.recheck();
        true
    }

    fn recheck(&mut self) {
        self.checked = Checked::of(self.kit.content(), &self.kit.game, &self.build.side);
        self.changed = true;
    }

    /// The catalogs' and the game's words, in the language shown.
    pub fn with_words<R>(&self, shell: &Shell, f: impl FnOnce(&Words) -> R) -> R {
        let ui = shell.ui();
        let graphics = self.kit.ready.graphics(shell.lang);
        let names = Names::of(self.kit.content(), &graphics);
        f(&Words { ui: &ui, kit: &self.kit, names: &names })
    }
}

/// One of the editor's tabs. Every method but [`Tab::show`] has a default that does nothing, as a
/// `Screen`'s; `id` and `index` are a control's name and index as the tab's Slint gives them.
pub trait Tab {
    /// Which tab it is: its place in the bar, and its component in the UI.
    fn id(&self) -> EditorTab;

    /// The build's facts that are the tab's own, by their setup fields' names.
    fn fields(&self, content: &Content) -> Vec<String>;

    /// Whether the tab is the game's own navi's alone (the navi that changes form: a NaviCust, patch cards
    /// and auto battle data are its).
    fn own_navi_only(&self) -> bool {
        false
    }

    /// The control the focus starts on, the first time the tab is shown.
    fn main(&self) -> &'static str;

    /// The tab is shown, or something it shows changed under it (the build, the language, the window turned
    /// upright): set the view from the draft.
    fn show(&mut self, shell: &mut Shell, draft: &mut Draft);

    /// Another tab is shown in its place, or the editor is left.
    fn hide(&mut self, _shell: &mut Shell, _draft: &mut Draft) {}

    fn activate(&mut self, _shell: &mut Shell, _draft: &mut Draft, _id: &str, _index: i32) {}

    fn step(&mut self, _shell: &mut Shell, _draft: &mut Draft, _id: &str, _index: i32, _by: i32) {}

    fn choose(&mut self, _shell: &mut Shell, _draft: &mut Draft, _id: &str, _index: i32, _choice: i32) {}

    fn edit(&mut self, _shell: &mut Shell, _draft: &mut Draft, _id: &str, _index: i32, _text: &str, _done: bool) {}

    fn focused(&mut self, _shell: &mut Shell, _draft: &mut Draft, _id: &str, _index: i32) {}

    /// A key the tab may want for itself (a letter that acts on the focused row). True if it was the tab's.
    fn key(&mut self, _shell: &mut Shell, _draft: &mut Draft, _key: &KeyPress) -> bool {
        false
    }

    fn button(&mut self, _shell: &mut Shell, _draft: &mut Draft, _button: gamepad_facade::Button) -> bool {
        false
    }

    /// Delete, and the pad's north button. (Previous and next are the frame's: they change the tab.)
    fn action(&mut self, _shell: &mut Shell, _draft: &mut Draft, _action: Action) -> bool {
        false
    }

    /// Esc or Back. True if the tab went back within itself (it shut something of its own).
    fn back(&mut self, _shell: &mut Shell, _draft: &mut Draft) -> bool {
        false
    }

    /// A dialog the tab asked (`Shell::ask`, by its `tag`) was answered.
    fn answered(&mut self, _shell: &mut Shell, _draft: &mut Draft, _tag: &str, _choice: usize) {}
}

/// Every tab's module, in the bar's order.
fn all() -> Vec<Box<dyn Tab>> {
    vec![
        Box::new(super::navi::Navi::default()),
        Box::new(super::folder::Folder::default()),
        Box::new(super::navicust::Navicust::default()),
        Box::new(super::patch_cards::PatchCards::default()),
        Box::new(super::auto_battle::AutoBattle::default()),
    ]
}

/// The facts of `tab` that the game's setup takes (a tab names every fact its view can edit; a game may take
/// only some: EXE4's NaviCust has its programs and no board to expand).
fn taken(content: &Content, tab: &dyn Tab) -> Vec<String> {
    tab.fields(content).into_iter().filter(|f| nettai_match::facts::takes(content, f)).collect()
}

/// Whether the game has tab `tab`: it names no facts of its own (the navi's, the folder's are every game's), or
/// the game's setup takes at least one of those it names.
fn has(content: &Content, tab: &dyn Tab) -> bool {
    tab.fields(content).is_empty() || !taken(content, tab).is_empty()
}

/// The build's facts that are the game's own navi's alone: those of the tabs that are (the NaviCust's, the
/// patch cards', the auto battle data's, where the game has them). What a build's file keeps aside while its
/// navi is another.
pub fn navi_only_fields(content: &Content) -> Vec<String> {
    all().iter().filter(|t| t.own_navi_only()).flat_map(|t| taken(content, t.as_ref())).collect()
}

/// The tab whose facts include fact `field`, if one's do (what a problem with it is about, for its sentence).
pub fn tab_of(content: &Content, field: &str) -> Option<EditorTab> {
    all().iter().find(|t| t.fields(content).iter().any(|f| f == field)).map(|t| t.id())
}

/// The key the last tab used is remembered under, in the settings.
const REMEMBERED: &str = "builds.tab";

/// A tab's name: the focus's memory's, and the settings'.
fn name(tab: EditorTab) -> &'static str {
    match tab {
        EditorTab::Navi => "builds.navi",
        EditorTab::Folder => "builds.folder",
        EditorTab::Navicust => "builds.navicust",
        EditorTab::PatchCards => "builds.patch-cards",
        EditorTab::AutoBattle => "builds.auto-battle",
    }
}

/// The editor's frame: the build, the game's tabs, and the one shown.
pub struct Editor {
    pub draft: Draft,
    tabs: Vec<Box<dyn Tab>>,
    at: usize,
}

impl Editor {
    /// Open `build` (kept at `path`) in the editor: settled as every build is, on the tab last used.
    pub fn open(shell: &mut Shell, kit: Rc<Kit>, path: PathBuf, mut build: Build) -> Editor {
        let content = kit.content().clone();
        let navi_only = navi_only_fields(&content);
        let settled = build.settle(&content, &kit.builds, &navi_only);
        let checked = Checked::of(&content, &kit.game, &build.side);
        let tabs: Vec<Box<dyn Tab>> = all().into_iter().filter(|t| has(&content, t.as_ref())).collect();
        let draft = Draft { kit, build, checked, path, navi_only, changed: settled };
        let last = shell.settings.remembered.get(REMEMBERED).cloned();
        let mut editor = Editor { draft, tabs, at: 0 };
        editor.at = editor.tabs.iter().position(|t| Some(name(t.id())) == last.as_deref()).filter(|&i| editor.enabled(i)).unwrap_or(0);
        editor.save(shell);
        editor
    }

    /// The file the build is kept in.
    pub fn path(&self) -> &std::path::Path {
        &self.draft.path
    }

    /// Whether tab `i` can be gone to: one that is the game's own navi's alone can't while the build's navi
    /// is another.
    fn enabled(&self, i: usize) -> bool {
        !self.tabs[i].own_navi_only() || self.draft.build.own_navi(self.draft.content())
    }

    fn tab(&mut self) -> (&mut dyn Tab, &mut Draft) {
        (self.tabs[self.at].as_mut(), &mut self.draft)
    }

    /// The header and the tab bar: the build's name, its version's tag, whether it is ready, the tabs.
    fn show_frame(&self, shell: &Shell) {
        let ui = shell.ui();
        let view = ui.global::<BuildsView>();
        let content = self.draft.content();
        let names = view.get_tab_names();
        let problems = &self.draft.checked.problems;
        let fields: Vec<Vec<String>> = self.tabs.iter().map(|t| t.fields(content)).collect();
        let owned = |field: &str| fields.iter().any(|f| f.iter().any(|x| x == field));
        let tabs: Vec<nettai_ui::Tab> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                // A problem is its field's tab's; one of a fact no tab says is its own, or of none, the first's.
                let flagged = problems.iter().any(|p| match p.field.as_deref() {
                    Some(field) if owned(field) => fields[i].iter().any(|x| x == field),
                    _ => i == 0,
                });
                let word = slint::Model::row_data(&names, tab.id() as usize).unwrap_or_default();
                nettai_ui::Tab { text: word, enabled: self.enabled(i), flagged }
            })
            .collect();
        let (state, good) = self.draft.with_words(shell, |words| words.header(&self.draft.checked));
        let version = self.draft.with_words(shell, |words| words.kit.version_tag(words.names, self.draft.side()));
        view.set_name(self.draft.build.name.as_str().into());
        view.set_version(version);
        view.set_state(state.into());
        view.set_state_good(good);
        view.set_tabs(ModelRc::new(VecModel::from(tabs)));
        view.set_tab(self.at as i32);
        view.set_shown(self.tabs[self.at].id());
    }

    /// The editor is shown, or something changed under it: the frame, and the tab.
    pub fn show(&mut self, shell: &mut Shell) {
        shell.lock_game(true);
        self.show_frame(shell);
        let at = self.at as i32;
        let (tab, draft) = self.tab();
        shell.nav.enter(name(tab.id()), tab.main());
        // (A tab with nothing of its own to focus has the focus on its word in the bar.)
        if tab.main() == "tab" {
            shell.nav.focus("tab", at);
        }
        tab.show(shell, draft);
    }

    /// The editor is left.
    pub fn close(&mut self, shell: &mut Shell) {
        let (tab, draft) = self.tab();
        tab.hide(shell, draft);
        self.save(shell);
        shell.lock_game(false);
    }

    /// Keep the build if it changed, and show what follows from it: the header, the tabs' marks.
    fn save(&mut self, shell: &mut Shell) {
        if !std::mem::take(&mut self.draft.changed) {
            return;
        }
        let draft = &self.draft;
        if let Err(why) = store::write(&draft.path, draft.content(), &draft.kit.game, &draft.build) {
            crate::errors::log("keeping a build", &why);
            shell.fail(UserError::Other);
        }
    }

    /// After a tab's call: the build kept if it changed, and the frame shown anew.
    fn after(&mut self, shell: &mut Shell) {
        if self.draft.changed {
            self.save(shell);
            self.show_frame(shell);
        }
    }

    /// Go to tab `to`, if it can be gone to; `way` is the side it is reached from (-1: from the right).
    fn go(&mut self, shell: &mut Shell, to: usize, way: i32) {
        if to == self.at || to >= self.tabs.len() || !self.enabled(to) {
            return;
        }
        let (tab, draft) = self.tab();
        tab.hide(shell, draft);
        self.at = to;
        shell.settings.remembered.insert(REMEMBERED.into(), name(self.tabs[to].id()).into());
        shell.save_settings();
        shell.ui().global::<BuildsView>().set_tab_way(way);
        self.show(shell);
    }

    /// The tab before or after the one shown that can be gone to, around from the last to the first.
    fn step_tab(&mut self, shell: &mut Shell, by: i32) {
        let n = self.tabs.len() as i32;
        let mut to = self.at as i32;
        for _ in 0..n {
            to = (to + by).rem_euclid(n);
            if self.enabled(to as usize) {
                break;
            }
        }
        self.go(shell, to as usize, by.signum());
    }

    pub fn activate(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if id == "tab" {
            let way = if index as usize >= self.at { 1 } else { -1 };
            return self.go(shell, index.max(0) as usize, way);
        }
        let (tab, draft) = self.tab();
        tab.activate(shell, draft, id, index);
        // (A click on one of the build's own rows, an entry, a card, a slot or a place, chooses it as Enter does,
        // which takes the cursor on to the library for a chip or a card to put in it; but where the library is
        // beside it (not a phone's sheet), the focus stays on what was clicked, so the next key acts on it (R, T,
        // Del, the arrows) and a click on the library's row puts what it holds in there. The pointer down took the
        // focus there already: this keeps it from going on.)
        let moved_on = shell.nav.focused().is_some_and(|k| k.id.starts_with("lib.") && !id.starts_with("lib."));
        if shell.nav.pointing() && !shell.scale.phone && moved_on {
            shell.nav.focus(id, index);
            tab.focused(shell, draft, id, index);
        }
        self.after(shell);
    }

    pub fn step(&mut self, shell: &mut Shell, id: &str, index: i32, by: i32) {
        let (tab, draft) = self.tab();
        tab.step(shell, draft, id, index, by);
        self.after(shell);
    }

    /// A right click on a row that holds something of the build (its `removable` touch, which brought the focus to
    /// it) removes it, as Del there does.
    pub fn choose(&mut self, shell: &mut Shell, id: &str, index: i32, choice: i32) {
        let (tab, draft) = self.tab();
        if id == "remove" {
            tab.action(shell, draft, Action::Delete);
        } else {
            tab.choose(shell, draft, id, index, choice);
        }
        self.after(shell);
    }

    pub fn edit(&mut self, shell: &mut Shell, id: &str, index: i32, text: &str, done: bool) {
        let (tab, draft) = self.tab();
        tab.edit(shell, draft, id, index, text, done);
        self.after(shell);
    }

    pub fn focused(&mut self, shell: &mut Shell, id: &str, index: i32) {
        let (tab, draft) = self.tab();
        tab.focused(shell, draft, id, index);
        self.after(shell);
    }

    pub fn key(&mut self, shell: &mut Shell, key: &KeyPress) -> bool {
        let (tab, draft) = self.tab();
        let taken = tab.key(shell, draft, key);
        self.after(shell);
        taken
    }

    pub fn button(&mut self, shell: &mut Shell, button: gamepad_facade::Button) -> bool {
        let (tab, draft) = self.tab();
        let taken = tab.button(shell, draft, button);
        self.after(shell);
        taken
    }

    /// Previous and next change the tab; the rest is the tab's.
    pub fn action(&mut self, shell: &mut Shell, action: Action) -> bool {
        match action {
            Action::Previous => self.step_tab(shell, -1),
            Action::Next => self.step_tab(shell, 1),
            _ => {
                let (tab, draft) = self.tab();
                let taken = tab.action(shell, draft, action);
                self.after(shell);
                return taken;
            }
        }
        true
    }

    /// Esc or Back: true if the tab went back within itself.
    pub fn back(&mut self, shell: &mut Shell) -> bool {
        let (tab, draft) = self.tab();
        let within = tab.back(shell, draft);
        self.after(shell);
        within
    }

    pub fn answered(&mut self, shell: &mut Shell, tag: &str, choice: usize) {
        let (tab, draft) = self.tab();
        tab.answered(shell, draft, tag, choice);
        self.after(shell);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::testing::{exe4_content, exe5_content, exe6_content};

    /// Each game has the tabs its setup has something for: a NaviCust and patch cards in all three (EXE4's
    /// NaviCust has its programs and no board to expand: a tab is there for any of the facts it names), auto
    /// battle data in EXE4 and EXE5. What is the game's own navi's alone, and kept aside for another, is those
    /// tabs' facts that the game takes.
    #[test]
    fn each_game_has_its_tabs() {
        let tabs = |content: &Content| -> Vec<EditorTab> { all().iter().filter(|t| has(content, t.as_ref())).map(|t| t.id()).collect() };
        let four = [EditorTab::Navi, EditorTab::Folder, EditorTab::Navicust, EditorTab::PatchCards];
        let five = [EditorTab::Navi, EditorTab::Folder, EditorTab::Navicust, EditorTab::PatchCards, EditorTab::AutoBattle];
        assert_eq!(tabs(&exe6_content()), four);
        assert_eq!(tabs(&exe5_content()), five);
        assert_eq!(tabs(&exe4_content()), five);
        assert_eq!(navi_only_fields(&exe6_content()), ["navicust_programs", "navicust_expansions", "patch_cards"]);
        assert_eq!(navi_only_fields(&exe5_content()), ["navicust_programs", "navicust_expansions", "patch_cards", "auto_battle_places"]);
        assert_eq!(navi_only_fields(&exe4_content()), ["navicust_programs", "patch_cards", "auto_battle_places"]);
    }
}
