//! Builds (docs/design/app-ui.md §6): the player's builds of the selected game, and a build's editor.
//!
//! **The list:** NEW BUILD and IMPORT SAVE… over the game's builds, the chosen one's overview beside them
//! (`crate::builds::overview`, which any screen shows), and EDIT, DUPLICATE, RENAME and DELETE. A build made
//! new is the game's own (`nettai_match::build::Builds::new_build`: its own navi, the version chosen, the
//! folder a new save starts with); there are no random builds. A save file chosen or dropped on the window is
//! made a build (`crate::builds::import`). On a phone the list is the page, and a build's overview and its
//! buttons a page of their own behind it.
//!
//! **The editor** (`editor`): the header's name, version and "✓ Ready to play", the tab bar, and the tab
//! shown. Each tab is a module here and a file in the UI's `screens/builds/`: [`navi`], [`folder`], and the
//! three to come ([`navicust`], [`patch_cards`], [`auto_battle`]).

use crate::builds::store::{self, Build};
use crate::builds::{self, Kit, import, overview, shelf};
use crate::errors::UserError;
use crate::games::Names;
use crate::input::{Action, KeyPress};
use crate::nav::Key;
use crate::screens::Screen;
use crate::shell::{Dialog, Shell};
use editor::Editor;
use nettai_ui::{BuildEntry, BuildsDialog, BuildsPage, BuildsView, ButtonKind, DialogButton, VersionTag};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::path::Path;
use std::rc::Rc;

pub mod auto_battle;
pub mod editor;
pub mod folder;
pub mod navi;
pub mod navicust;
pub mod patch_cards;
pub mod tour;

/// The save files a build is made of: what the emulators and Tango name them.
const SAVES: [&str; 4] = ["sav", "sa1", "srm", "raw"];

/// The dialog open: a new build's name and version, or the chosen build's new name.
enum Naming {
    New { name: String, version: usize },
    Rename { name: String },
}

#[derive(Default)]
pub struct Builds {
    /// The selected game's builds, and the game they are of.
    shelf: Vec<shelf::Shelved>,
    game: Option<String>,
    /// The build whose overview shows; `pointed`: the pointer just chose it (its click then only chose it, and
    /// the next one on it edits it).
    chosen: Option<usize>,
    pointed: bool,
    /// A phone's page of the chosen build is shown.
    sheet: bool,
    naming: Option<Naming>,
    editor: Option<Editor>,
    /// What the last import made, where that is worth saying.
    message: String,
}

impl Builds {
    /// The chosen build, if it reads as one.
    fn chosen_build(&self) -> Option<(&Path, &Build)> {
        let entry = self.shelf.get(self.chosen?)?;
        Some((entry.listed.path.as_path(), entry.build.as_ref()?))
    }

    /// Read the game's builds again, the one kept at `keep` chosen (else the one that was, by its place).
    fn load(&mut self, shell: &Shell, kit: &Kit, keep: Option<&Path>) {
        let kept = keep.map(Path::to_path_buf).or_else(|| self.chosen.and_then(|i| self.shelf.get(i)).map(|e| e.listed.path.clone()));
        self.shelf = shelf::load(shell, kit);
        self.game = Some(kit.game.clone());
        let at = kept.and_then(|path| self.shelf.iter().position(|e| e.listed.path == path));
        self.chosen = match (at, self.chosen) {
            _ if self.shelf.is_empty() => None,
            (Some(at), _) => Some(at),
            (None, Some(was)) => Some(was.min(self.shelf.len() - 1)),
            (None, None) => Some(0),
        };
    }

    /// The list, the chosen build's overview, and the dialog open.
    fn show_list(&self, shell: &Shell, kit: &Kit) {
        let ui = shell.ui();
        let view = ui.global::<BuildsView>();
        let rows: Vec<BuildEntry> = self.shelf.iter().map(|e| e.row.clone()).collect();
        view.set_builds(ModelRc::new(VecModel::from(rows)));
        view.set_chosen(self.chosen.map_or(-1, |i| i as i32));
        view.set_message(self.message.as_str().into());
        view.set_page(if self.sheet && shell.scale.phone && self.chosen.is_some() { BuildsPage::Sheet } else { BuildsPage::List });
        self.show_chosen(shell, kit);
        // The game's versions, for a new build: each by its tag, and what a version sets in the game's words.
        let graphics = kit.ready.graphics(shell.lang);
        let names = Names::of(kit.content(), &graphics);
        let versions: Vec<VersionTag> = kit
            .builds
            .versions
            .iter()
            .map(|v| {
                let [_, r, g, b] = v.color.to_be_bytes();
                VersionTag { text: kit.version_name(&names, &v.id).to_uppercase().into(), tone: slint::Color::from_rgb_u8(r, g, b) }
            })
            .collect();
        view.set_versions(ModelRc::new(VecModel::from(versions)));
        view.set_version_sets(names.text(nettai_match::build::TEXT_TABLE, "versions").unwrap_or_default().into());
        self.show_dialog(shell);
    }

    /// The chosen build's overview.
    fn show_chosen(&self, shell: &Shell, kit: &Kit) {
        let ui = shell.ui();
        let view = ui.global::<BuildsView>();
        view.set_chosen(self.chosen.map_or(-1, |i| i as i32));
        match self.chosen_build() {
            Some((_, build)) => {
                view.set_sheet(overview::sheet(shell, kit, "", &build.name, &build.side));
                view.set_readable(true);
                view.set_name(build.name.as_str().into());
            }
            None => {
                view.set_readable(false);
                view.set_name(self.chosen.and_then(|i| self.shelf.get(i)).map(|e| e.listed.name.clone()).unwrap_or_default().into());
            }
        }
    }

    fn show_dialog(&self, shell: &Shell) {
        let ui = shell.ui();
        let view = ui.global::<BuildsView>();
        match &self.naming {
            Some(Naming::New { name, version }) => {
                view.set_dialog_name(name.as_str().into());
                view.set_dialog_version(*version as i32);
                view.set_dialog(BuildsDialog::NewBuild);
            }
            Some(Naming::Rename { name }) => {
                view.set_dialog_name(name.as_str().into());
                view.set_dialog(BuildsDialog::Rename);
            }
            None => view.set_dialog(BuildsDialog::None),
        }
    }

    fn close_dialog(&mut self, shell: &mut Shell) {
        self.naming = None;
        shell.nav.close("name.");
        self.show_dialog(shell);
    }

    /// The names the game's builds have.
    fn taken(&self) -> Vec<String> {
        self.shelf.iter().map(|e| e.listed.name.clone()).collect()
    }

    /// Keep `build` as a new file, and choose it in the list.
    fn add(&mut self, shell: &mut Shell, kit: &Kit, build: &Build) {
        let path = store::new_path(&kit.game, &build.name);
        if let Err(why) = store::write(&path, kit.content(), &kit.game, build) {
            crate::errors::log("keeping a new build", &why);
            return shell.fail(UserError::Other);
        }
        self.load(shell, kit, Some(&path));
        self.show_list(shell, kit);
        if let Some(at) = self.chosen {
            shell.nav.focus("builds", at as i32);
        }
    }

    /// A new build of version `version`, named `name`.
    fn create(&mut self, shell: &mut Shell, kit: &Kit, name: &str, version: usize) {
        let Some(version) = kit.builds.versions.get(version) else { return };
        match kit.builds.new_build(kit.content(), &version.id) {
            Ok(side) => self.add(shell, kit, &Build::new(kit.content(), name, side)),
            Err(why) => {
                crate::errors::log("making a new build", &why);
                shell.fail(UserError::Other);
            }
        }
    }

    /// A build of the save at `path`, named for the file.
    fn import(&mut self, shell: &mut Shell, kit: &Kit, path: &Path) {
        let file = match std::fs::read(path) {
            Ok(file) => file,
            Err(why) => {
                crate::errors::log(&format!("reading the save {}", path.display()), &why);
                return shell.fail(UserError::Other);
            }
        };
        let imported = match import::side_of_save(kit.content(), &kit.builds, &file) {
            Ok(imported) => imported,
            Err(import::Refused::NotThisGames) => return shell.fail(UserError::WrongGameSave { game: Shell::game_name(&kit.game) }),
            Err(import::Refused::Failed) => return shell.fail(UserError::Other),
        };
        let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let name = store::new_name(store::clean_name(&stem).trim_end(), &self.taken());
        let mut build = Build::new(kit.content(), &name, imported.side);
        build.settle(kit.content(), &kit.builds, &editor::navi_only_fields(kit.content()));
        // What the save was put on, where that changed what it held: "Imported as Dark MegaMan".
        let ui = shell.ui();
        let graphics = kit.ready.graphics(shell.lang);
        let names = Names::of(kit.content(), &graphics);
        self.message = imported.taken.first().map_or_else(String::new, |taken| {
            let preset = kit.builds.choice(&taken.choice).and_then(|c| c.preset(&taken.preset).map(|p| c.text_key(p)));
            let preset = preset.and_then(|key| names.text(nettai_match::build::TEXT_TABLE, &key)).unwrap_or_else(|| taken.preset.clone());
            ui.global::<nettai_ui::BuildWords>().invoke_imported_as(preset.into(), names.navi(build.side.navi(kit.content())).into()).into()
        });
        self.add(shell, kit, &build);
    }

    /// Open the chosen build in the editor.
    fn edit_chosen(&mut self, shell: &mut Shell, kit: Rc<Kit>) {
        let Some((path, build)) = self.chosen_build().map(|(p, b)| (p.to_path_buf(), b.clone())) else { return };
        self.message.clear();
        let mut editor = Editor::open(shell, kit, path, build);
        shell.ui().global::<BuildsView>().set_page(BuildsPage::Editor);
        editor.show(shell);
        self.editor = Some(editor);
    }

    /// Leave the editor for the list, the build it edited chosen there.
    fn close_editor(&mut self, shell: &mut Shell) {
        let Some(mut editor) = self.editor.take() else { return };
        editor.close(shell);
        let path = editor.path().to_path_buf();
        let kit = editor.draft.kit.clone();
        self.load(shell, &kit, Some(&path));
        shell.nav.enter("builds", "builds");
        self.show_list(shell, &kit);
        if let Some(at) = self.chosen {
            shell.nav.focus("builds", at as i32);
        }
    }

    fn duplicate(&mut self, shell: &mut Shell, kit: &Kit) {
        let Some((_, build)) = self.chosen_build() else { return };
        let copy = build.named(&store::new_name(&build.name, &self.taken()));
        self.add(shell, kit, &copy);
    }

    /// Ask once whether the chosen build goes, Cancel focused.
    fn ask_delete(&mut self, shell: &mut Shell) {
        let Some(entry) = self.chosen.and_then(|i| self.shelf.get(i)) else { return };
        let ui = shell.ui();
        let view = ui.global::<BuildsView>();
        let button = |text: slint::SharedString, danger: bool| DialogButton { text, kind: ButtonKind::Ghost, danger };
        shell.ask(Dialog {
            tag: "delete",
            title: view.invoke_delete_title(entry.listed.name.as_str().into()).into(),
            message: String::new(),
            buttons: vec![button(view.get_cancel(), false), button(view.get_delete(), true)],
            focus: 0,
            cancel: 0,
        });
    }

    fn delete(&mut self, shell: &mut Shell, kit: &Kit) {
        let Some(entry) = self.chosen.and_then(|i| self.shelf.get(i)) else { return };
        if let Err(why) = std::fs::remove_file(&entry.listed.path) {
            crate::errors::log(&format!("deleting the build {}", entry.listed.path.display()), &why);
            return shell.fail(UserError::Other);
        }
        self.sheet = false;
        self.load(shell, kit, None);
        self.show_list(shell, kit);
        match self.chosen {
            Some(at) => shell.nav.focus("builds", at as i32),
            None => shell.nav.focus("new", 0),
        }
    }

    /// The name dialog's main button: the build made, or renamed.
    fn confirm(&mut self, shell: &mut Shell, kit: &Kit) {
        match self.naming.take() {
            Some(Naming::New { name, version }) if !name.trim().is_empty() => {
                shell.nav.close("name.");
                self.create(shell, kit, name.trim_end(), version);
            }
            Some(Naming::Rename { name }) if !name.trim().is_empty() => {
                shell.nav.close("name.");
                let Some((path, build)) = self.chosen_build().map(|(p, b)| (p.to_path_buf(), b.clone())) else { return self.show_dialog(shell) };
                let renamed = build.named(name.trim_end());
                // (Its file is named for it: the old one goes once the new one is kept.)
                let to = store::new_path(&kit.game, &renamed.name);
                match store::write(&to, kit.content(), &kit.game, &renamed) {
                    Ok(()) => {
                        let _ = std::fs::remove_file(&path);
                        self.load(shell, kit, Some(&to));
                    }
                    Err(why) => {
                        crate::errors::log("renaming a build", &why);
                        shell.fail(UserError::Other);
                    }
                }
                self.show_list(shell, kit);
                if let Some(at) = self.chosen {
                    shell.nav.focus("builds", at as i32);
                }
            }
            // (No name: the dialog stays.)
            unnamed => self.naming = unnamed,
        }
    }
}

impl Screen for Builds {
    fn main(&self) -> &'static str {
        "builds"
    }

    fn show(&mut self, shell: &mut Shell) {
        let Some(kit) = builds::kit(shell) else { return };
        // (The editor's build is its game's: the game can't change under it.)
        if let Some(editor) = &mut self.editor {
            shell.ui().global::<BuildsView>().set_page(BuildsPage::Editor);
            return editor.show(shell);
        }
        if self.game.as_deref() != Some(kit.game.as_str()) {
            self.chosen = None;
            self.sheet = false;
            self.message.clear();
        }
        self.load(shell, &kit, None);
        self.show_list(shell, &kit);
        // (With no builds the focus starts on what makes one.)
        if self.shelf.is_empty() && self.naming.is_none() && shell.nav.focused().is_none_or(|k| k.id == "builds") {
            shell.nav.focus("new", 0);
        }
    }

    fn hide(&mut self, shell: &mut Shell) {
        if let Some(mut editor) = self.editor.take() {
            editor.close(shell);
        }
        if self.naming.take().is_some() {
            shell.nav.close("name.");
        }
        self.sheet = false;
        self.message.clear();
        shell.ui().global::<BuildsView>().set_dialog(BuildsDialog::None);
        shell.ui().global::<BuildsView>().set_page(BuildsPage::List);
    }

    fn activate(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if let Some(editor) = &mut self.editor {
            return editor.activate(shell, id, index);
        }
        let Some(kit) = builds::kit(shell) else { return };
        match id {
            "new" => {
                let ui = shell.ui();
                let name = store::new_name(&ui.global::<BuildsView>().get_new_name(), &self.taken());
                self.naming = Some(Naming::New { name, version: 0 });
                self.show_dialog(shell);
                shell.nav.open("name.", Key::new("name.name", 0));
            }
            "import" => {
                let kind = shell.ui().global::<BuildsView>().get_save_files().to_string();
                if let Some(path) = shell.choose_file(&kind, &SAVES) {
                    self.import(shell, &kit, &path);
                }
            }
            "builds" => {
                let at = index.max(0) as usize;
                // A click chooses a row, and a click on the chosen row edits it; Enter edits the row it is on.
                let just_chosen = self.chosen != Some(at) || std::mem::take(&mut self.pointed);
                self.chosen = Some(at);
                if shell.scale.phone {
                    self.sheet = true;
                    shell.nav.enter("builds.one", "one.edit");
                    self.show_list(shell, &kit);
                } else if shell.nav.pointing() && just_chosen {
                    self.show_chosen(shell, &kit);
                } else {
                    self.edit_chosen(shell, kit);
                }
            }
            "edit" | "one.edit" => self.edit_chosen(shell, kit),
            "duplicate" | "one.duplicate" => self.duplicate(shell, &kit),
            "rename" | "one.rename" => {
                let Some((_, build)) = self.chosen_build() else { return };
                self.naming = Some(Naming::Rename { name: build.name.clone() });
                self.show_dialog(shell);
                shell.nav.open("name.", Key::new("name.name", 0));
            }
            "delete" | "one.delete" => self.ask_delete(shell),
            "name.confirm" => {
                self.confirm(shell, &kit);
                self.show_dialog(shell);
            }
            "name.cancel" => self.close_dialog(shell),
            // (Enter on the versions takes the other one.)
            "name.version" => self.step(shell, id, index, 1),
            _ => {}
        }
    }

    fn step(&mut self, shell: &mut Shell, id: &str, index: i32, by: i32) {
        if let Some(editor) = &mut self.editor {
            return editor.step(shell, id, index, by);
        }
        let Some(kit) = builds::kit(shell) else { return };
        if let ("name.version", Some(Naming::New { version, .. })) = (id, &mut self.naming) {
            *version = (*version as i32 + by).rem_euclid(kit.builds.versions.len().max(1) as i32) as usize;
            self.show_dialog(shell);
        }
    }

    fn choose(&mut self, shell: &mut Shell, id: &str, index: i32, choice: i32) {
        if let Some(editor) = &mut self.editor {
            return editor.choose(shell, id, index, choice);
        }
        if let ("name.version", Some(Naming::New { version, .. })) = (id, &mut self.naming) {
            *version = choice.max(0) as usize;
            self.show_dialog(shell);
        }
    }

    fn edit(&mut self, shell: &mut Shell, id: &str, index: i32, text: &str, done: bool) {
        if let Some(editor) = &mut self.editor {
            return editor.edit(shell, id, index, text, done);
        }
        if let ("name.name", Some(Naming::New { name, .. } | Naming::Rename { name })) = (id, &mut self.naming) {
            *name = store::clean_name(text);
            if done {
                *name = name.trim_end().to_string();
            }
            self.show_dialog(shell);
        }
    }

    fn focused(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if let Some(editor) = &mut self.editor {
            return editor.focused(shell, id, index);
        }
        if id == "builds" && self.chosen != Some(index.max(0) as usize) {
            let Some(kit) = builds::kit(shell) else { return };
            self.chosen = Some(index.max(0) as usize);
            self.pointed = shell.nav.pointing();
            self.show_chosen(shell, &kit);
        }
    }

    fn key(&mut self, shell: &mut Shell, key: &KeyPress) -> bool {
        self.editor.as_mut().is_some_and(|editor| editor.key(shell, key))
    }

    fn button(&mut self, shell: &mut Shell, button: gamepad_facade::Button) -> bool {
        self.editor.as_mut().is_some_and(|editor| editor.button(shell, button))
    }

    /// In the editor the shoulders change the tab; on the list Delete asks whether the chosen build goes.
    fn action(&mut self, shell: &mut Shell, action: Action) -> bool {
        if let Some(editor) = &mut self.editor {
            return editor.action(shell, action);
        }
        if self.naming.is_some() {
            return true;
        }
        if action == Action::Delete && shell.nav.focused().is_some_and(|k| k.id == "builds") {
            self.ask_delete(shell);
            return true;
        }
        false
    }

    fn back(&mut self, shell: &mut Shell) -> bool {
        if self.naming.is_some() {
            self.close_dialog(shell);
            return true;
        }
        if let Some(editor) = &mut self.editor {
            if !editor.back(shell) {
                self.close_editor(shell);
            }
            return true;
        }
        if self.sheet {
            self.sheet = false;
            shell.nav.enter("builds", "builds");
            if let Some(kit) = builds::kit(shell) {
                self.show_list(shell, &kit);
            }
            return true;
        }
        false
    }

    fn answered(&mut self, shell: &mut Shell, tag: &str, choice: usize) {
        if let Some(editor) = &mut self.editor {
            return editor.answered(shell, tag, choice);
        }
        if tag == "delete"
            && choice == 1
            && let Some(kit) = builds::kit(shell)
        {
            self.delete(shell, &kit);
        }
    }

    /// A save dropped on the list is made a build.
    fn dropped(&mut self, shell: &mut Shell, path: &Path) {
        if self.editor.is_some() || self.naming.is_some() {
            return;
        }
        if let Some(kit) = builds::kit(shell) {
            self.import(shell, &kit, path);
        }
    }
}
