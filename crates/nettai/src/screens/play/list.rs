//! Play's builds (docs/design/app-ui.md §6, "Play"): what is done with the selected game's builds over the tiles
//! that practice with one or make a link. + NEW BUILD makes one of the game's own
//! (`nettai_match::build::Builds::new_build`: its own navi, the forms of the version chosen, the folder a new save starts
//! with; there are no random builds), or, by its dialog's FROM A SAVE FILE…, one of a save file chosen in the
//! system's dialog (as a save dropped on the window makes one: `crate::builds::import`); EDIT opens the chosen
//! build in Builds' editor (`crate::screens::builds::open`), whose Back comes back here with it still chosen;
//! PRACTICE goes straight into a practice battle with it (`crate::screens::practice::open`), whose EXIT comes
//! back here too; the
//! build's menu (⋯, or a right click on its row: `Shell::pick_menu`) has DUPLICATE, RENAME and, apart, DELETE
//! (which asks once, Cancel focused).

use super::{Play, remembered_build};
use crate::builds::store::{self, Build};
use crate::builds::{self, Kit, import};
use crate::errors::UserError;
use crate::games::Names;
use crate::nav::Key;
use crate::screens::{Screen, ScreenId};
use crate::shell::{Dialog, Shell};
use crate::sound::Cue;
use nettai_ui::{BuildDialog, ButtonKind, DialogButton, PlayView};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::path::Path;

/// The save files a build is made of: what the emulators and Tango name them.
const SAVES: [&str; 4] = ["sav", "sa1", "srm", "raw"];

/// The tag of the question DELETE asks (`Screen::answered`: 1 is DELETE).
pub(super) const DELETE: &str = "play.delete";

/// What the build's menu does, in its order: DELETE last, set apart.
#[derive(Clone, Copy, PartialEq)]
enum MenuAction {
    Duplicate,
    Rename,
    Delete,
}

/// The dialog open: a new build's name and version, or the chosen build's new name.
pub(super) enum Naming {
    New { name: String, version: usize },
    Rename { name: String },
}

impl Play {
    /// The chosen build, if it reads as one.
    fn chosen_build(&self) -> Option<(&Path, &Build)> {
        let entry = self.build()?;
        Some((entry.listed.path.as_path(), entry.build.as_ref()?))
    }

    /// What the build's menu offers: for a build that reads, DUPLICATE, RENAME and DELETE; for one whose file
    /// doesn't, DELETE alone; with none, nothing.
    fn menu_actions(&self) -> &'static [MenuAction] {
        match (self.build(), self.chosen_build()) {
            (_, Some(_)) => &[MenuAction::Duplicate, MenuAction::Rename, MenuAction::Delete],
            (Some(_), None) => &[MenuAction::Delete],
            (None, None) => &[],
        }
    }

    /// The build's menu, by the control `at` (⋯, or the row a right click was on): its choice is heard as
    /// `more`'s (`menu_chosen`).
    pub(super) fn open_menu(&mut self, shell: &mut Shell, at: &Key) {
        let actions = self.menu_actions();
        if actions.is_empty() {
            return shell.sound.play(Cue::Refused);
        }
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        let words = actions
            .iter()
            .map(|a| match a {
                MenuAction::Duplicate => view.get_duplicate_word(),
                MenuAction::Rename => view.get_rename_word(),
                MenuAction::Delete => view.get_delete_word(),
            })
            .map(|w| w.to_string())
            .collect();
        let apart = actions.iter().position(|a| *a == MenuAction::Delete);
        shell.pick_menu("more", 0, at, words, apart);
    }

    /// A choice of the build's menu, by its place in it.
    pub(super) fn menu_chosen(&mut self, shell: &mut Shell, choice: i32) {
        match usize::try_from(choice).ok().and_then(|at| self.menu_actions().get(at)) {
            Some(MenuAction::Duplicate) => self.duplicate(shell),
            Some(MenuAction::Rename) => self.ask_rename(shell),
            Some(MenuAction::Delete) => self.ask_delete(shell),
            None => {}
        }
    }

    /// The names the game's builds have.
    fn taken(&self) -> Vec<String> {
        self.builds.iter().map(|b| b.listed.name.clone()).collect()
    }

    /// The game's versions, for a new build: each by its tag, and what a version sets in the game's words.
    pub(super) fn show_versions(&self, shell: &Shell, kit: &Kit) {
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        let graphics = kit.ready.graphics(shell.lang);
        let names = Names::of(kit.content(), &graphics);
        view.set_versions(ModelRc::new(VecModel::from(kit.version_tags(&names))));
        view.set_version_sets(names.text(nettai_match::build::TEXT_TABLE, "versions").unwrap_or_default().into());
    }

    pub(super) fn show_dialog(&self, shell: &Shell) {
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        match &self.naming {
            Some(Naming::New { name, version }) => {
                view.set_dialog_name(name.as_str().into());
                view.set_dialog_version(*version as i32);
                view.set_dialog(BuildDialog::NewBuild);
            }
            Some(Naming::Rename { name }) => {
                view.set_dialog_name(name.as_str().into());
                view.set_dialog(BuildDialog::Rename);
            }
            None => view.set_dialog(BuildDialog::None),
        }
    }

    pub(super) fn close_dialog(&mut self, shell: &mut Shell) {
        if self.naming.take().is_some() {
            shell.nav.close("name.");
        }
        self.show_dialog(shell);
    }

    /// Open the dialog for a name, `naming`, its field ready to type in.
    fn open_dialog(&mut self, shell: &mut Shell, naming: Naming) {
        self.naming = Some(naming);
        self.show_dialog(shell);
        shell.nav.open_editing("name.", Key::new("name.name", 0));
    }

    /// NEW BUILD: a name not taken, and the game's first version.
    pub(super) fn ask_new(&mut self, shell: &mut Shell) {
        let name = store::new_name(&shell.ui().global::<PlayView>().get_new_name(), &self.taken());
        self.open_dialog(shell, Naming::New { name, version: 0 });
    }

    /// RENAME: the chosen build's name, to change.
    pub(super) fn ask_rename(&mut self, shell: &mut Shell) {
        let Some((_, build)) = self.chosen_build() else { return };
        let name = build.name.clone();
        self.open_dialog(shell, Naming::Rename { name });
    }

    /// The new build's version stepped by `by`, around.
    pub(super) fn step_version(&mut self, shell: &Shell, by: i32) {
        let count = builds::kit(shell).map_or(1, |kit| kit.builds.groups.len().max(1)) as i32;
        if let Some(Naming::New { version, .. }) = &mut self.naming {
            *version = (*version as i32 + by).rem_euclid(count) as usize;
            self.show_dialog(shell);
        }
    }

    /// The new build's version chosen outright (a click on one).
    pub(super) fn choose_version(&mut self, shell: &Shell, choice: usize) {
        if let Some(Naming::New { version, .. }) = &mut self.naming {
            *version = choice;
            self.show_dialog(shell);
        }
    }

    /// The name typed: as a build's file can be named, its end trimmed once it is left.
    pub(super) fn edit_name(&mut self, shell: &Shell, text: &str, done: bool) {
        if let Some(Naming::New { name, .. } | Naming::Rename { name }) = &mut self.naming {
            *name = store::clean_name(text);
            if done {
                *name = name.trim_end().to_string();
            }
            self.show_dialog(shell);
        }
    }

    /// The name dialog's main button: the build made, or renamed. (With no name the dialog stays.)
    pub(super) fn confirm(&mut self, shell: &mut Shell) {
        let Some(kit) = builds::kit(shell) else { return };
        match self.naming.take() {
            Some(Naming::New { name, version }) if !name.trim().is_empty() => {
                shell.nav.close("name.");
                self.show_dialog(shell);
                self.create(shell, &kit, name.trim_end(), version);
            }
            Some(Naming::Rename { name }) if !name.trim().is_empty() => {
                shell.nav.close("name.");
                self.show_dialog(shell);
                self.rename(shell, &kit, name.trim_end());
            }
            unnamed => self.naming = unnamed,
        }
    }

    /// Show the game's builds read anew, the one named `name` chosen (and remembered); the cursor on it, or with
    /// none on NEW BUILD.
    fn reload(&mut self, shell: &mut Shell, name: Option<String>) {
        if let (Some(game), Some(name)) = (self.builds_of.clone(), name) {
            shell.settings.remembered.insert(remembered_build(&game), name);
            shell.save_settings();
        }
        // (No build is chosen until they are read: then the one remembered is.)
        self.chosen = usize::MAX;
        self.show(shell);
        match (self.builds.is_empty(), shell.scale.phone) {
            (true, _) => shell.nav.focus("new", 0),
            (false, true) => shell.nav.focus("build", 0),
            (false, false) => shell.nav.focus("builds", self.chosen as i32),
        }
    }

    /// Keep `build` as a new file, and choose it.
    fn add(&mut self, shell: &mut Shell, kit: &Kit, build: &Build) {
        let path = store::new_path(&kit.game, &build.name);
        if let Err(why) = store::write(&path, kit.content(), &kit.game, build) {
            crate::errors::log("keeping a new build", &why);
            return shell.fail(UserError::Other);
        }
        self.reload(shell, Some(build.name.clone()));
    }

    /// A new build from the game's `version`th group (its version's forms), named `name`.
    fn create(&mut self, shell: &mut Shell, kit: &Kit, name: &str, version: usize) {
        let Some(version) = kit.builds.groups.get(version) else { return };
        self.notice.clear();
        match kit.builds.new_build(kit.content(), &version.id) {
            Ok(side) => self.add(shell, kit, &Build::new(kit.content(), name, side)),
            Err(why) => {
                crate::errors::log("making a new build", &why);
                shell.fail(UserError::Other);
            }
        }
    }

    /// The chosen build named `name`: its file is named for it, the old one going once the new one is kept.
    fn rename(&mut self, shell: &mut Shell, kit: &Kit, name: &str) {
        let Some((path, build)) = self.chosen_build().map(|(p, b)| (p.to_path_buf(), b.clone())) else { return };
        self.notice.clear();
        let renamed = build.named(name);
        let to = store::new_path(&kit.game, &renamed.name);
        match store::write(&to, kit.content(), &kit.game, &renamed) {
            Ok(()) => {
                let _ = std::fs::remove_file(&path);
                self.reload(shell, Some(renamed.name));
            }
            Err(why) => {
                crate::errors::log("renaming a build", &why);
                shell.fail(UserError::Other);
            }
        }
    }

    /// The NEW BUILD dialog's FROM A SAVE FILE…: a save file chosen in the system's dialog.
    pub(super) fn import_chosen_file(&mut self, shell: &mut Shell) {
        let kind = shell.ui().global::<PlayView>().get_save_files().to_string();
        if let Some(path) = shell.choose_file(&kind, &SAVES) {
            self.import(shell, &path);
        }
    }

    /// A build of the save at `path`, named for the file.
    pub(super) fn import(&mut self, shell: &mut Shell, path: &Path) {
        let Some(kit) = builds::kit(shell) else { return };
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
        build.settle(kit.content(), &kit.builds, &crate::screens::builds::editor::navi_only_fields(kit.content()));
        // What the save was put on, where that changed what it held: "Imported as Dark MegaMan".
        let ui = shell.ui();
        let graphics = kit.ready.graphics(shell.lang);
        let names = Names::of(kit.content(), &graphics);
        self.notice = imported.taken.first().map_or_else(String::new, |taken| {
            let preset = kit.builds.choice(&taken.choice).and_then(|c| c.preset(&taken.preset).map(|p| c.text_key(p)));
            let preset = preset.and_then(|key| names.text(nettai_match::build::TEXT_TABLE, &key)).unwrap_or_else(|| taken.preset.clone());
            ui.global::<nettai_ui::BuildWords>().invoke_imported_as(preset.into(), names.navi(build.side.navi(kit.content())).into()).into()
        });
        self.add(shell, &kit, &build);
    }

    /// EDIT: the chosen build in Builds' editor (by its place in the list, `store::list`'s order), whose Back
    /// comes back here.
    pub(super) fn edit_chosen(&mut self, shell: &mut Shell) {
        if self.chosen_build().is_none() {
            return shell.sound.play(Cue::Refused);
        }
        self.notice.clear();
        crate::screens::builds::open(shell, ScreenId::Play, self.chosen);
    }

    /// PRACTICE: straight into a practice battle, the player's build the chosen one (`practice::open`), whose EXIT
    /// comes back here with it still chosen. With no build, or one that can't be played, the reserved line says
    /// why, as HOST's does.
    pub(super) fn practice(&mut self, shell: &mut Shell) {
        match (self.side(shell), self.builds_of.clone(), self.build().map(|b| b.name().to_string())) {
            (Ok(_), Some(game), Some(name)) => {
                self.notice.clear();
                crate::screens::practice::open(shell, &game, &name);
            }
            (Err(problem), _, _) => {
                self.problem = Some(problem);
                self.refuse_or_show(shell);
            }
            _ => {}
        }
    }

    /// DUPLICATE: a copy of the chosen build, named for it.
    pub(super) fn duplicate(&mut self, shell: &mut Shell) {
        let (Some(kit), Some((_, build))) = (builds::kit(shell), self.chosen_build()) else { return };
        let copy = build.named(&store::new_name(&build.name, &self.taken()));
        self.notice.clear();
        self.add(shell, &kit, &copy);
    }

    /// DELETE: asked once whether the chosen build goes, Cancel focused.
    pub(super) fn ask_delete(&mut self, shell: &mut Shell) {
        let Some(entry) = self.build() else { return };
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        let button = |text: slint::SharedString, danger: bool| DialogButton { text, kind: ButtonKind::Ghost, danger };
        shell.ask(Dialog {
            tag: DELETE,
            title: view.invoke_delete_title(entry.listed.name.as_str().into()).into(),
            message: String::new(),
            buttons: vec![button(view.get_cancel_word(), false), button(view.get_delete_word(), true)],
            focus: 0,
            cancel: 0,
        });
    }

    /// The chosen build's file goes; the one after it is chosen, or the one before where it was the last.
    pub(super) fn delete(&mut self, shell: &mut Shell) {
        let Some(entry) = self.build() else { return };
        if let Err(why) = std::fs::remove_file(&entry.listed.path) {
            crate::errors::log(&format!("deleting the build {}", entry.listed.path.display()), &why);
            return shell.fail(UserError::Other);
        }
        let at = self.chosen;
        let next = self.builds.get(at + 1).or_else(|| at.checked_sub(1).and_then(|i| self.builds.get(i))).map(|b| b.name().to_string());
        self.notice.clear();
        self.reload(shell, next);
    }
}
