//! About, Settings' subscreen (docs/app.md §18): the app's version and license; Slint's "Made with Slint", which
//! Slint's royalty-free license asks of an app made with it (an About screen reached from the top menu: SETTINGS,
//! then the About group's SHOW LICENSES); and the software the app is made of, the fonts it bundles and every
//! package built into it, each with its license's texts as its package ships them (`build.rs` finds them). It is
//! shown in the settings' place, a view of its own to the focus; Back from its list goes back to them. A phone
//! reads a license on a sheet over the list.

use crate::input::Action;
use crate::nav::Key;
use crate::shell::Shell;
use crate::sound::Cue;
use nettai_ui::{AboutView, LicensePart, SettingsView, SoftwareRow};
use slint::{ComponentHandle, ModelRc, VecModel};

/// What the app is made of, as `build.rs` wrote it.
mod licenses {
    /// A font or a package: its name, its version (a font's: none), its license as it states it (an SPDX
    /// expression), its repository, and its license files, each a name and its text (in `TEXTS`).
    pub struct Software {
        pub name: &'static str,
        pub version: &'static str,
        pub license: &'static str,
        pub repository: &'static str,
        pub texts: &'static [(&'static str, usize)],
    }

    include!(concat!(env!("OUT_DIR"), "/licenses.rs"));
}

use licenses::{FONTS, PACKAGES, Software, TEXTS};

/// How far Up and Down scroll a license.
const STEP: f32 = 56.0;

/// Its view's name, where the focus is remembered apart from the settings'.
const VIEW: &str = "settings.about";

/// The list: the fonts, then the packages by name.
fn software(at: usize) -> Option<&'static Software> {
    FONTS.get(at).or_else(|| PACKAGES.get(at.checked_sub(FONTS.len())?))
}

/// Where `name` is in the list (the screenshot pass reads Slint's license).
#[cfg(any(test, feature = "tour"))]
pub(crate) fn place_of(name: &str) -> i32 {
    (0..).map_while(software).position(|s| s.name == name).map_or(0, |at| at as i32)
}

/// A license's pieces: each file's name, then its text a paragraph a piece (the lines between blank ones, as
/// they are).
fn parts(software: &Software) -> Vec<LicensePart> {
    let mut parts = Vec::new();
    for &(file, text) in software.texts {
        parts.push(LicensePart { heading: true, text: file.into() });
        let mut paragraph: Vec<&str> = Vec::new();
        for line in TEXTS[text].lines().chain([""]) {
            if line.trim().is_empty() {
                if !paragraph.is_empty() {
                    parts.push(LicensePart { heading: false, text: paragraph.join("\n").into() });
                    paragraph.clear();
                }
            } else {
                paragraph.push(line.trim_end());
            }
        }
    }
    parts
}

#[derive(Default)]
pub struct About {
    /// The one whose license shows, in the list's order.
    chosen: usize,
    /// A phone: its sheet is open.
    sheet: bool,
    /// It is shown in the settings' place.
    open: bool,
}

impl About {
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Shown in the settings' place: the focus on the chosen one's row, the list scrolled to it (the row it
    /// was on, the last time; a list made anew starts at its top, and a remembered focus alone doesn't scroll
    /// it).
    pub fn open(&mut self, shell: &mut Shell) {
        self.open = true;
        shell.ui().global::<SettingsView>().set_about(true);
        shell.nav.enter(VIEW, "list");
        self.show(shell);
        shell.nav.focus("list", self.chosen as i32);
    }

    /// The settings shown again. (Their focus, where it was, is the settings' to bring back.)
    pub fn close(&mut self, shell: &mut Shell) {
        self.close_sheet(shell);
        self.open = false;
        shell.ui().global::<SettingsView>().set_about(false);
    }

    fn show_chosen(&self, shell: &Shell) {
        let ui = shell.ui();
        let view = ui.global::<AboutView>();
        let software = software(self.chosen).expect("the list's");
        view.set_chosen(self.chosen as i32);
        view.set_repository(software.repository.into());
        view.set_parts(ModelRc::new(VecModel::from(parts(software))));
        view.set_text_y(0.0);
    }

    fn open_sheet(&mut self, shell: &mut Shell) {
        if !self.sheet {
            self.sheet = true;
            shell.ui().global::<AboutView>().set_sheet(true);
            shell.nav.open("sheet.", Key::new("sheet.text", 0));
        }
    }

    fn close_sheet(&mut self, shell: &mut Shell) {
        if std::mem::take(&mut self.sheet) {
            shell.nav.close("sheet.");
            shell.ui().global::<AboutView>().set_sheet(false);
        }
    }

    /// Whether the focus is on the license (beside the list, or on the sheet).
    fn reading(shell: &Shell) -> bool {
        shell.nav.focused().is_some_and(|k| k.id == "text" || k.id == "sheet.text")
    }

    // What Settings passes on to it while it is open, as a screen's own.

    /// Set the view from the state (opened, or the language changed under it).
    pub fn show(&mut self, shell: &mut Shell) {
        if !shell.scale.phone {
            self.close_sheet(shell);
        }
        let rows: Vec<SoftwareRow> = FONTS
            .iter()
            .chain(PACKAGES)
            .map(|s| SoftwareRow { name: s.name.into(), version: s.version.into(), license: s.license.into() })
            .collect();
        let ui = shell.ui();
        let view = ui.global::<AboutView>();
        view.set_rows(ModelRc::new(VecModel::from(rows)));
        view.set_fonts(FONTS.len() as i32);
        self.show_chosen(shell);
    }

    /// A row: Enter goes to its license, to scroll it with the keys (a phone opens its sheet). A click
    /// chooses it alone.
    pub fn activate(&mut self, shell: &mut Shell, id: &str, index: i32) {
        match id {
            "list" => {
                if let Some(at) = usize::try_from(index).ok().filter(|&at| software(at).is_some())
                    && at != self.chosen
                {
                    self.chosen = at;
                    self.show_chosen(shell);
                }
                if shell.scale.phone {
                    self.open_sheet(shell);
                } else if !shell.nav.pointing() {
                    shell.nav.focus("text", 0);
                }
            }
            "sheet.close" => self.close_sheet(shell),
            _ => {}
        }
    }

    pub fn focused(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if id != "list" || shell.scale.phone {
            return;
        }
        if let Some(at) = usize::try_from(index).ok().filter(|&at| software(at).is_some())
            && at != self.chosen
        {
            self.chosen = at;
            self.show_chosen(shell);
        }
    }

    /// The license with the focus on it: Up and Down scroll it a step, Page Up and Down a page less a step,
    /// Home and End to its ends. It takes them all, at an end too (its view is the one control there).
    pub fn scroll(&mut self, shell: &mut Shell, action: Action) -> bool {
        if !Self::reading(shell) {
            return false;
        }
        let ui = shell.ui();
        let view = ui.global::<AboutView>();
        let (at, span, page) = (-view.get_text_y(), view.get_text_span(), (view.get_text_view() - STEP).max(STEP));
        let to = match action {
            Action::Up => at - STEP,
            Action::Down => at + STEP,
            Action::PageUp => at - page,
            Action::PageDown => at + page,
            Action::Home => 0.0,
            Action::End => span,
            _ => return false,
        }
        .clamp(0.0, span.max(0.0));
        if to != at {
            view.set_text_y(-to);
            shell.sound.play(Cue::Cursor);
        }
        true
    }

    /// Back from the license beside the list goes to the list; from the sheet, shuts it. False from the list:
    /// Settings closes it.
    pub fn back(&mut self, shell: &mut Shell) -> bool {
        if self.sheet {
            self.close_sheet(shell);
            return true;
        }
        if Self::reading(shell) {
            shell.nav.focus("list", self.chosen as i32);
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_holds_the_fonts_and_slint_with_their_licenses() {
        // (The UI's and the battle's drawing's: build.rs's list, in its order.)
        let names: Vec<&str> = FONTS.iter().map(|f| f.name).collect();
        assert_eq!(names, ["Bungee", "Chakra Petch", "Corporate Logo", "M PLUS 1 Code", "Noto Sans CJK", "Nettai Banner Digits", "Nettai Chip Codes", "Nettai Digits", "Nettai Slot Codes"]);
        // (The bought-in fonts under the OFL; the digit and chip code fonts nettai's own, under its license.)
        assert!(FONTS.iter().all(|f| !f.texts.is_empty() && f.license == if f.name.starts_with("Nettai") { "MPL-2.0" } else { "OFL-1.1" }));
        let slint = PACKAGES.iter().find(|p| p.name == "slint").expect("Slint is built in");
        assert!(slint.license.contains("LicenseRef-Slint-Royalty-free-2.0"));
        assert!(slint.texts.iter().any(|(file, _)| file.contains("Royalty-free")));
        // (Only what the app is built from: no workspace crate, nothing a build of the app alone doesn't take.)
        assert!(!PACKAGES.iter().any(|p| p.name.starts_with("nettai") || p.name.starts_with("skia")));
    }

    #[test]
    fn a_license_is_its_files_names_over_their_paragraphs() {
        let slint = PACKAGES.iter().find(|p| p.name == "slint").expect("Slint is built in");
        let parts = parts(slint);
        assert!(parts[0].heading);
        assert_eq!(parts.iter().filter(|p| p.heading).count(), slint.texts.len());
        assert!(parts.iter().all(|p| !p.text.is_empty() && !p.text.contains("\n\n")));
    }
}
