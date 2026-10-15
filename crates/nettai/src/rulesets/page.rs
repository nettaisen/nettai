//! A ruleset's page (docs/design/app-ui.md §2.59): what a ruleset allows, one category at a time, changed and
//! saved at once where it is the player's own; a preset's, or the host's ruleset a joiner views, read alone (its
//! DUPLICATE makes the player's own copy). The lobby and the build editor open it over themselves ([`Page::open`])
//! and hand it the controls whose names begin [`PREFIX`] and the answer to its question ([`DELETE`]); what it did
//! that the screen must hear of it says ([`Outcome`]): the ruleset it shows is another, or changed, or gone.
//!
//! The UI is ui/ruleset.slint (`RulesetPage`, `RulesetView`).

use crate::builds::Kit;
use crate::games::Names;
use crate::nav::Key;
use crate::rulesets::{NAME_LENGTH, Pickable, shown_name, store};
use crate::shell::{Dialog, Shell};
use crate::sound::Cue;
use nettai_content_api::{ChipHandle, EntryHandle, FormHandle, NaviHandle, Registry};
use nettai_match::ruleset::{Category, Name, Rulesets};
use nettai_ui::{ButtonKind, DialogButton, RulesetCategory, RulesetChoice, RulesetItem, RulesetView};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::rc::Rc;
use std::sync::Arc;

/// What the page's controls' names begin with: the focus's scope while it is open.
pub const PREFIX: &str = "rules.";

/// The tag of the question DELETE asks (`Screen::answered`: 1 deletes).
pub const DELETE: &str = "rules.delete";

/// What the page did that the screen that opened it hears of.
#[derive(Debug)]
pub enum Outcome {
    Nothing,
    /// The ruleset shown changed and was saved, or is another (a duplicate, now the player's own): it is the one
    /// the player picked, as it now stands.
    Changed(Pickable),
    /// The ruleset shown was deleted, and the page closed.
    Deleted,
}

/// A ruleset's page, open.
pub struct Page {
    kit: Rc<Kit>,
    rulesets: Arc<Rulesets>,
    shown: Pickable,
    /// The host's ruleset, viewed by a joiner: read alone, whatever it is.
    viewed: bool,
    /// The category shown, by its place, and what its items are filtered by.
    category: usize,
    search: String,
    /// The items listed of the category shown, by handle, in its list's order.
    rows: Vec<u16>,
    /// The quick choices' rows, changed in place where their count stays (their controls made again would drop the
    /// focus a press on one left there).
    choices: Rc<VecModel<RulesetChoice>>,
}

/// A name as it is typed: one line, no room at its start, [`NAME_LENGTH`] letters at most.
fn clean_name(text: &str) -> String {
    let line: String = text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    line.trim_start().chars().take(NAME_LENGTH).collect()
}

impl Page {
    /// Open `shown`'s page over the screen shown, its Back saying where it goes (`back`: the view's LOBBY or
    /// EDITOR), read alone where it is `viewed` (the host's ruleset, in a joiner's lobby). None where the game
    /// says nothing of its rulesets.
    pub fn open(shell: &mut Shell, kit: Rc<Kit>, shown: Pickable, viewed: bool, back: &str) -> Option<Page> {
        let rulesets = kit.rulesets()?;
        let choices = Rc::new(VecModel::default());
        let mut page = Page { kit, rulesets, shown, viewed, category: 0, search: String::new(), rows: Vec::new(), choices: choices.clone() };
        let ui = shell.ui();
        let view = ui.global::<RulesetView>();
        view.set_choices(ModelRc::from(choices));
        view.set_back(back.into());
        view.set_search(Default::default());
        page.show(shell);
        view.set_open(true);
        shell.nav.open(PREFIX, Key::new("rules.categories", 0));
        if shell.scale.phone {
            shell.nav.focus("rules.category", 0);
        }
        Some(page)
    }

    /// Close the page: the focus goes back where it was under it.
    pub fn close(self, shell: &mut Shell) {
        shell.ui().global::<RulesetView>().set_open(false);
        shell.nav.close(PREFIX);
    }

    /// Whether the controls `id` names are the page's.
    pub fn owns(id: &str) -> bool {
        id.starts_with(PREFIX)
    }

    /// The ruleset can be changed here: the player's own, not one viewed.
    fn editable(&self) -> bool {
        !self.viewed && self.shown.path.is_some()
    }

    fn category(&self) -> &Category {
        &self.rulesets.categories[self.category.min(self.rulesets.categories.len().saturating_sub(1))]
    }

    /// Everything the page shows, from the ruleset as it stands.
    fn show(&mut self, shell: &Shell) {
        let ui = shell.ui();
        let view = ui.global::<RulesetView>();
        let graphics = self.kit.ready.graphics(shell.lang);
        let kit = self.kit.clone();
        let names = Names::of(kit.content(), &graphics);
        let ruleset = &self.shown.ruleset;
        view.set_title(shown_name(&names, &ruleset.name).into());
        view.set_name(shown_name(&names, &ruleset.name).into());
        view.set_preset(matches!(ruleset.name, Name::Preset(_)));
        view.set_editable(self.editable());
        view.set_deletable(self.editable());
        view.set_versions(if ruleset.one_per_build { 0 } else { 1 });
        let categories: Vec<RulesetCategory> = self
            .rulesets
            .categories
            .iter()
            .map(|c| {
                let name = names.text(nettai_match::ruleset::TEXT_TABLE, &c.text_key()).unwrap_or_else(|| c.fact.clone());
                RulesetCategory { name: name.into(), note: view.invoke_allowed(ruleset.allowed_count(c) as i32, c.items.len() as i32) }
            })
            .collect();
        view.set_categories(ModelRc::new(VecModel::from(categories)));
        view.set_category(self.category as i32);
        let message = match (&ruleset.name, self.editable()) {
            (_, true) => Default::default(),
            _ if self.viewed => view.get_fixed_viewed(),
            _ => view.get_fixed_preset(),
        };
        view.set_message(message);
        view.set_message_quiet(true);
        self.show_items(shell, &names);
    }

    /// The category shown's quick choices and its items, filtered by the search.
    fn show_items(&mut self, shell: &Shell, names: &Names) {
        let ui = shell.ui();
        let view = ui.global::<RulesetView>();
        let kit = self.kit.clone();
        let rulesets = self.rulesets.clone();
        let category = self.category().clone();
        let ruleset = &self.shown.ruleset;
        let tone = |color: u32| {
            let [_, r, g, b] = color.to_be_bytes();
            slint::Color::from_rgb_u8(r, g, b)
        };
        // A version's own items as one choice each, then All: lit where they are exactly what is allowed of them.
        let groups = rulesets.groups_in(&category);
        let chosen = ruleset.groups_chosen(&rulesets, &category);
        let mut choices: Vec<RulesetChoice> = groups
            .iter()
            .map(|g| RulesetChoice { text: kit.version_name(names, &g.id).into(), version: true, tone: tone(g.color), chosen: chosen.as_deref() == Some(&[g.id.as_str()][..]) })
            .collect();
        if !groups.is_empty() {
            let all = chosen.as_ref().is_some_and(|c| c.len() == groups.len());
            choices.push(RulesetChoice { text: view.get_all_word(), version: false, tone: Default::default(), chosen: all });
        }
        if self.choices.row_count() == choices.len() {
            for (i, choice) in choices.into_iter().enumerate() {
                self.choices.set_row_data(i, choice);
            }
        } else {
            self.choices.set_vec(choices);
        }
        // The items in their lists' order: the chips the library's, the entries their collection's.
        let content = kit.content();
        let mut items: Vec<u16> = category.items.clone();
        match category.registry {
            Registry::Chip => {
                let mut sorted: Vec<((), ChipHandle)> = items.iter().map(|&h| ((), ChipHandle(h))).collect();
                kit.order.chips(content, &mut sorted);
                items = sorted.into_iter().map(|(_, h)| h.0).collect();
            }
            Registry::Entry => {
                if let Some(of) = &category.of {
                    let mut sorted: Vec<((), EntryHandle)> = items.iter().map(|&h| ((), EntryHandle(h))).collect();
                    kit.order.entries(content, of, &mut sorted);
                    items = sorted.into_iter().map(|(_, h)| h.0).collect();
                }
            }
            // (The game's own navi first, as the Navi tab has them.)
            Registry::Navi => items.sort_by_key(|&h| !content.navi(NaviHandle(h)).changes_form()),
            _ => {}
        }
        let search = self.search.to_lowercase();
        let rows: Vec<(u16, String)> =
            items.into_iter().map(|h| (h, names.def(category.registry, h))).filter(|(_, name)| search.is_empty() || name.to_lowercase().contains(&search)).collect();
        let lines: Vec<RulesetItem> = rows
            .iter()
            .map(|(h, name)| {
                let picture = match category.registry {
                    Registry::Chip => kit.pictures.icon(ChipHandle(*h)),
                    Registry::Navi => kit.pictures.navi(NaviHandle(*h)),
                    Registry::Form => kit.pictures.face(FormHandle(*h)),
                    _ => Default::default(),
                };
                let group = rulesets.group_of(category.registry, *h);
                RulesetItem {
                    picture,
                    name: name.as_str().into(),
                    note: group.map(|g| kit.version_name(names, &g.id).to_uppercase()).unwrap_or_default().into(),
                    tone: group.map(|g| tone(g.color)).unwrap_or_default(),
                    allowed: ruleset.allows(&category.fact, *h),
                }
            })
            .collect();
        self.rows = rows.into_iter().map(|(h, _)| h).collect();
        view.set_items(ModelRc::new(VecModel::from(lines)));
    }

    /// Refused: the ruleset can't be changed here, which the reserved line says, in the error red.
    fn refuse(&self, shell: &mut Shell) -> Outcome {
        shell.sound.play(Cue::Refused);
        shell.ui().global::<RulesetView>().set_message_quiet(false);
        Outcome::Nothing
    }

    /// Change the ruleset by `f`, where it can be changed: saved at once, and shown.
    fn change(&mut self, shell: &mut Shell, f: impl FnOnce(&mut nettai_match::ruleset::Ruleset, &Rulesets)) -> Outcome {
        if !self.editable() {
            return self.refuse(shell);
        }
        let before = self.shown.ruleset.clone();
        f(&mut self.shown.ruleset, &self.rulesets);
        if self.shown.ruleset == before {
            return Outcome::Nothing;
        }
        self.save(shell);
        self.show(shell);
        Outcome::Changed(self.shown.clone())
    }

    /// Write the ruleset shown to its file.
    fn save(&self, shell: &mut Shell) {
        let Some(path) = &self.shown.path else { return };
        if let Err(why) = store::write(path, self.kit.content(), &self.kit.game, &self.shown.ruleset) {
            crate::errors::log("saving a ruleset", &why);
            shell.fail(crate::errors::UserError::Other);
        }
    }

    /// The player's own rulesets' names, but the one shown.
    fn taken(&self) -> Vec<String> {
        store::list(&self.kit.game).into_iter().filter(|l| Some(&l.path) != self.shown.path.as_ref()).map(|l| l.name).collect()
    }

    /// DUPLICATE: the player's own copy of the ruleset shown, named after it (numbered past the presets' names and
    /// the player's own), saved and shown in its place.
    fn duplicate(&mut self, shell: &mut Shell) -> Outcome {
        let graphics = self.kit.ready.graphics(shell.lang);
        let kit = self.kit.clone();
        let names = Names::of(kit.content(), &graphics);
        let mut taken = self.taken();
        taken.extend(self.rulesets.presets.iter().map(|p| shown_name(&names, &p.name)));
        if let Name::Own(own) = &self.shown.ruleset.name {
            taken.push(own.clone());
        }
        let name = crate::builds::store::new_name(&shown_name(&names, &self.shown.ruleset.name), &taken);
        let mut copy = self.shown.ruleset.clone();
        copy.name = Name::Own(name.clone());
        self.shown = Pickable { ruleset: copy, path: Some(store::new_path(&self.kit.game, &name)) };
        self.viewed = false;
        self.save(shell);
        self.show(shell);
        shell.sound.play(Cue::Pick);
        Outcome::Changed(self.shown.clone())
    }

    /// Enter, a click or a tap on one of the page's controls.
    pub fn activate(&mut self, shell: &mut Shell, id: &str, index: i32) -> Outcome {
        match id {
            "rules.categories" => {
                self.choose_category(shell, index);
                if !self.rows.is_empty() {
                    shell.nav.focus("rules.items", 0);
                }
                Outcome::Nothing
            }
            "rules.category" => {
                let ui = shell.ui();
                let names: Vec<String> = ui.global::<RulesetView>().get_categories().iter().map(|c| c.name.to_string()).collect();
                shell.pick("rules.category", 0, names, self.category);
                Outcome::Nothing
            }
            "rules.items" => {
                let Some(&h) = self.rows.get(index.max(0) as usize) else { return Outcome::Nothing };
                let fact = self.category().fact.clone();
                self.change(shell, |r, _| {
                    let allowed = r.allows(&fact, h);
                    r.set_allowed(&fact, h, !allowed);
                })
            }
            "rules.all" | "rules.none" => {
                let category = self.category().clone();
                let all = id == "rules.all";
                self.change(shell, |r, _| r.allow_only(&category, if all { &category.items } else { &[] }))
            }
            "rules.set" => {
                let category = self.category().clone();
                let groups: Vec<String> = self.rulesets.groups_in(&category).iter().map(|g| g.id.clone()).collect();
                let chosen: Vec<String> = match groups.get(index.max(0) as usize) {
                    Some(one) => vec![one.clone()],
                    None => groups.clone(),
                };
                self.change(shell, |r, rulesets| r.allow_groups(rulesets, &category, &chosen.iter().map(String::as_str).collect::<Vec<_>>()))
            }
            "rules.versions" => {
                let one = !self.shown.ruleset.one_per_build;
                self.change(shell, |r, _| r.one_per_build = one)
            }
            "rules.duplicate" => self.duplicate(shell),
            "rules.delete" if self.editable() => {
                let ui = shell.ui();
                let view = ui.global::<RulesetView>();
                let play = ui.global::<nettai_ui::PlayView>();
                let button = |text: slint::SharedString, danger: bool| DialogButton { text, kind: ButtonKind::Ghost, danger };
                shell.ask(Dialog {
                    tag: DELETE,
                    title: view.invoke_delete_title(view.get_name()).into(),
                    message: String::new(),
                    buttons: vec![button(play.get_cancel_word(), false), button(play.get_delete_word(), true)],
                    focus: 0,
                    cancel: 0,
                });
                Outcome::Nothing
            }
            _ => Outcome::Nothing,
        }
    }

    /// Left or Right on one of the page's steppers.
    pub fn step(&mut self, shell: &mut Shell, id: &str, _index: i32, by: i32) -> Outcome {
        match id {
            "rules.versions" => self.change(shell, |r, _| r.one_per_build = by < 0),
            "rules.category" => {
                let count = self.rulesets.categories.len() as i32;
                if count > 0 {
                    self.choose_category(shell, (self.category as i32 + by).rem_euclid(count));
                }
                Outcome::Nothing
            }
            _ => Outcome::Nothing,
        }
    }

    /// A choice made outright: a side of Versions, a category from its select's list.
    pub fn choose(&mut self, shell: &mut Shell, id: &str, _index: i32, choice: i32) -> Outcome {
        match id {
            "rules.versions" => self.change(shell, |r, _| r.one_per_build = choice == 0),
            "rules.category" => {
                self.choose_category(shell, choice);
                Outcome::Nothing
            }
            _ => Outcome::Nothing,
        }
    }

    /// A field's text: the name (saved when it is confirmed, a name another of the player's own has taken
    /// numbered), the search (each change).
    pub fn edit(&mut self, shell: &mut Shell, id: &str, text: &str, done: bool) -> Outcome {
        let ui = shell.ui();
        let view = ui.global::<RulesetView>();
        match id {
            "rules.search" => {
                self.search = text.to_string();
                view.set_search(text.into());
                let graphics = self.kit.ready.graphics(shell.lang);
                let kit = self.kit.clone();
        let names = Names::of(kit.content(), &graphics);
                self.show_items(shell, &names);
                Outcome::Nothing
            }
            "rules.name" if self.editable() => {
                let name = clean_name(text);
                view.set_name(name.as_str().into());
                if !done {
                    return Outcome::Nothing;
                }
                let name = name.trim_end();
                if name.is_empty() {
                    self.show(shell);
                    return Outcome::Nothing;
                }
                let name = crate::builds::store::new_name(name, &self.taken());
                self.change(shell, |r, _| r.name = Name::Own(name))
            }
            _ => Outcome::Nothing,
        }
    }

    /// The focus moved: along the categories, the category under it is shown.
    pub fn focused(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if id == "rules.categories" && index as usize != self.category {
            self.choose_category(shell, index);
        }
    }

    /// Show category `index`.
    fn choose_category(&mut self, shell: &mut Shell, index: i32) {
        let index = (index.max(0) as usize).min(self.rulesets.categories.len().saturating_sub(1));
        if index == self.category {
            return;
        }
        self.category = index;
        let ui = shell.ui();
        ui.global::<RulesetView>().set_category(index as i32);
        let graphics = self.kit.ready.graphics(shell.lang);
        let kit = self.kit.clone();
        let names = Names::of(kit.content(), &graphics);
        self.show_items(shell, &names);
    }

    /// The page's question was answered: DELETE's (1 deletes the ruleset's file, and the page closes).
    pub fn answered(self, shell: &mut Shell, choice: usize) -> Result<Outcome, Page> {
        if choice != 1 {
            return Err(self);
        }
        let Some(path) = &self.shown.path else { return Err(self) };
        if let Err(why) = std::fs::remove_file(path) {
            crate::errors::log(&format!("deleting the ruleset {}", path.display()), &why);
            shell.fail(crate::errors::UserError::Other);
            return Err(self);
        }
        self.close(shell);
        Ok(Outcome::Deleted)
    }
}
