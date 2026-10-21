//! A ruleset's page (docs/design/app-ui.md §2.59): what a ruleset allows, one category at a time, changed and
//! saved at once where it is the player's own; a preset's, or the host's ruleset a joiner views, read alone (its
//! DUPLICATE makes the player's own copy). The lobby and the build editor open it over themselves ([`Page::open`])
//! and hand it the controls whose names begin [`PREFIX`] and the answer to its question ([`DELETE`]); what it did
//! that the screen must hear of it says ([`Outcome`]): the ruleset it shows is another, or changed, or gone.
//!
//! A category another is kept within (`nettai_match::ruleset::Category::within`: EXE5's party, its partners' programs
//! kept per navi) is one page, paired: its items (the navis) at the left, All first, each with its allow box and what
//! the inner category holds back for it alone; the one chosen's inner items beside them (the programs, every navi's ban
//! dashed). The inner category has no row of its own in the list.
//!
//! The UI is ui/ruleset.slint (`RulesetPage`, `RulesetView`).

use crate::builds::Kit;
use crate::games::Names;
use crate::nav::Key;
use crate::rulesets::{NAME_LENGTH, Pickable, shown_name, store};
use crate::shell::{Dialog, Shell};
use crate::sound::Cue;
use nettai_content_api::{ChipHandle, EntryHandle, FormHandle, NaviHandle, Registry};
use nettai_match::ruleset::{Category, Item, Name, Rulesets};
use nettai_ui::{ButtonKind, DialogButton, RulesetCategory, RulesetChoice, RulesetItem, RulesetKey, RulesetProgram, RulesetSection, RulesetView};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::rc::Rc;
use std::sync::Arc;

/// What the page's controls' names begin with: the focus's scope while it is open.
pub const PREFIX: &str = "rules.";

/// A ruleset file's extensions, as the system's file dialogs ask for them.
const EXTENSIONS: [&str; 1] = [store::EXTENSION];

/// What ruleset files chosen in the system's dialog are for, as the screen that shows the page hears them
/// (`Screen::chosen`).
pub const RULESET_FILES: &str = "rulesets";

/// The tag of the question DELETE asks (`Screen::answered`: 1 deletes).
pub const DELETE: &str = "rules.delete";

/// What the page did that the screen that opened it hears of.
#[derive(Debug)]
pub enum Outcome {
    Nothing,
    /// The ruleset shown changed and was saved, or is another (a duplicate, now the player's own): it is the one
    /// the player picked, as it now stands.
    Changed(Pickable),
    /// The ruleset shown (the player's own) was renamed `to` from `from`, and saved: the builds made for it are made
    /// for it by its new name already (`rulesets::rename`), on disk; it is the one the player picked.
    Renamed { from: Name, to: Pickable },
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
    /// The category shown, by its place in the list (the categories but those within another), and what its items
    /// are filtered by.
    category: usize,
    search: String,
    /// The items listed of the category shown, by handle, in its list's order.
    rows: Vec<Item>,
    /// In a category in sections, each shown row's section (by its place in `RulesetView.sections`).
    row_sections: Vec<usize>,
    /// The quick choices' rows, changed in place where their count stays (their controls made again would drop the
    /// focus a press on one left there).
    choices: Rc<VecModel<RulesetChoice>>,
    /// Paired: the keys' rows shown (None the row of every key, first), the key chosen (None: every key's), the inner
    /// category's items shown, in order.
    keys: Vec<Option<u16>>,
    key: Option<u16>,
    programs: Vec<Item>,
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
        let mut page = Page {
            kit,
            rulesets,
            shown,
            viewed,
            category: 0,
            search: String::new(),
            rows: Vec::new(),
            row_sections: Vec::new(),
            choices: choices.clone(),
            keys: Vec::new(),
            key: None,
            programs: Vec::new(),
        };
        let ui = shell.ui();
        let view = ui.global::<RulesetView>();
        view.set_choices(ModelRc::from(choices));
        view.set_back(back.into());
        view.set_search(Default::default());
        page.show(shell);
        shell.swap_page(true);
        shell.nav.open(PREFIX, Key::new("rules.categories", 0));
        if shell.scale.phone {
            shell.nav.focus("rules.category", 0);
        }
        Some(page)
    }

    /// Close the page: the focus goes back where it was under it.
    pub fn close(self, shell: &mut Shell) {
        shell.swap_page(false);
        shell.nav.close(PREFIX);
    }

    /// Whether the controls `id` names are the page's.
    pub fn owns(id: &str) -> bool {
        id.starts_with(PREFIX)
    }

    /// The lobby's ruleset as it stands now, where this page views it (a joiner's view of the host's): shown as
    /// the host changes it, as they change it (the user: "show live edits of host's ruleset"). The category, the
    /// search and the cursor stay where the player has them.
    pub fn follow(&mut self, shell: &Shell, ruleset: &nettai_match::ruleset::Ruleset) {
        if self.viewed && self.shown.ruleset != *ruleset {
            self.shown.ruleset = ruleset.clone();
            self.show(shell);
        }
    }

    /// The ruleset can be changed here: the player's own, not one viewed.
    fn editable(&self) -> bool {
        !self.viewed && self.shown.path.is_some()
    }

    /// The categories the list shows (those within another are shown on its page), by their places among the
    /// game's.
    fn listed(&self) -> Vec<usize> {
        (0..self.rulesets.categories.len()).filter(|&i| self.rulesets.categories[i].within.is_none()).collect()
    }

    fn category(&self) -> &Category {
        let listed = self.listed();
        &self.rulesets.categories[listed[self.category.min(listed.len().saturating_sub(1))]]
    }

    /// The category kept within category `outer` (its page's inner one), if there is one.
    fn inner_of(&self, outer: &Category) -> Option<&Category> {
        self.rulesets.categories.iter().find(|c| c.within.as_deref() == Some(outer.fact.as_str()) && c.by.is_some())
    }

    /// Everything the page shows, from the ruleset as it stands.
    fn show(&mut self, shell: &Shell) {
        let ui = shell.ui();
        let view = ui.global::<RulesetView>();
        let graphics = self.kit.ready.graphics(shell.lang);
        let kit = self.kit.clone();
        let names = Names::of(kit.content(), &graphics);
        let ruleset = &self.shown.ruleset;
        view.set_name(shown_name(&names, &ruleset.name).into());
        view.set_preset(matches!(ruleset.name, Name::Preset(_)));
        view.set_editable(self.editable());
        view.set_deletable(self.editable());
        view.set_versions(if ruleset.one_per_build { 0 } else { 1 });
        let categories: Vec<RulesetCategory> = self
            .listed()
            .into_iter()
            .map(|i| {
                let c = &self.rulesets.categories[i];
                let name = names.text(nettai_match::ruleset::TEXT_TABLE, &c.text_key()).unwrap_or_else(|| c.fact.clone());
                let (n, of) = (ruleset.allowed_count(c) as i32, c.items.len() as i32);
                let note = match self.inner_of(c) {
                    Some(inner) => view.invoke_paired_note(n, of, ruleset.ban_count(&inner.fact) as i32),
                    None => view.invoke_allowed(n, of),
                };
                RulesetCategory { name: name.into(), note }
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
        // A category of forms whose every one is a version's own (Crosses, Souls, Beast Out) is shown in a section a
        // version, as the Navi tab has them (the user: "for crosses and souls can you separate them into sections
        // instead like on the navi screen?"), in place of the quick choices.
        let groups = rulesets.groups_in(&category);
        let sectioned = category.registry == Registry::Form && !groups.is_empty() && category.items.iter().all(|i| rulesets.group_of(category.registry, i).is_some());
        // A version's own items as one choice each, then All: lit where they are exactly what is allowed of them.
        let chosen = ruleset.groups_chosen(&rulesets, &category);
        let mut choices: Vec<RulesetChoice> = groups
            .iter()
            .filter(|_| !sectioned)
            .map(|g| RulesetChoice { text: kit.version_name(names, &g.id).into(), version: true, tone: tone(g.color), chosen: chosen.as_deref() == Some(&[g.id.as_str()][..]) })
            .collect();
        if !groups.is_empty() && !sectioned {
            let all = chosen.as_ref().is_some_and(|c| c.len() == groups.len());
            choices.push(RulesetChoice { text: view.get_both_word(), version: false, tone: Default::default(), chosen: all });
        }
        if self.choices.row_count() == choices.len() {
            for (i, choice) in choices.into_iter().enumerate() {
                self.choices.set_row_data(i, choice);
            }
        } else {
            self.choices.set_vec(choices);
        }
        if let Some(inner) = self.inner_of(&category).cloned() {
            self.show_paired(shell, names, &category, &inner);
            return;
        }
        view.set_paired(false);
        view.set_keys(Default::default());
        view.set_programs(Default::default());
        self.keys.clear();
        self.programs.clear();
        // The items in their lists' order: the chips the library's, the entries their collection's (a card's variants
        // together, in their order).
        let content = kit.content();
        let mut items: Vec<Item> = category.items.clone();
        let place = |sorted: Vec<u16>| move |i: &Item| sorted.iter().position(|&h| h == i.def).unwrap_or(usize::MAX);
        match category.registry {
            Registry::Chip => {
                let mut sorted: Vec<((), ChipHandle)> = items.iter().map(|i| ((), ChipHandle(i.def))).collect();
                kit.order.chips(content, &mut sorted);
                items.sort_by_key(place(sorted.into_iter().map(|(_, h)| h.0).collect()));
            }
            Registry::Entry => {
                if let Some(of) = &category.of {
                    let mut sorted: Vec<((), EntryHandle)> = items.iter().map(|i| ((), EntryHandle(i.def))).collect();
                    sorted.dedup();
                    kit.order.entries(content, of, &mut sorted);
                    items.sort_by_key(place(sorted.into_iter().map(|(_, h)| h.0).collect()));
                }
            }
            // (The game's own navi first, as the Navi tab has them.)
            Registry::Navi => items.sort_by_key(|i| !content.navi(NaviHandle(i.def)).changes_form()),
            _ => {}
        }
        let search = self.search.to_lowercase();
        // (A card's variant by its card's name and its version's: "Bass-Cross MegaMan (Gregar)".)
        let shown = |i: &Item| {
            let name = names.def(category.registry, i.def);
            match &i.variant {
                Some(v) => shell.ui().global::<nettai_ui::BuildWords>().invoke_card_variant(name.into(), kit.version_name(names, v).into()).to_string(),
                None => name,
            }
        };
        let mut rows: Vec<(Item, String)> = items.into_iter().map(|i| {
            let name = shown(&i);
            (i, name)
        }).filter(|(_, name)| search.is_empty() || name.to_lowercase().contains(&search)).collect();
        // (In sections: the rows by their versions' order, each version's in its own order; a version none of whose
        // items the search leaves has no section.)
        let section_of = |item: &Item| rulesets.group_of(category.registry, item).and_then(|g| groups.iter().position(|o| o.id == g.id)).unwrap_or(0);
        if sectioned {
            rows.sort_by_key(|(item, _)| section_of(item));
        }
        let shown_groups: Vec<usize> = if sectioned {
            let mut seen: Vec<usize> = rows.iter().map(|(item, _)| section_of(item)).collect();
            seen.dedup();
            seen
        } else {
            Vec::new()
        };
        self.row_sections = rows.iter().map(|(item, _)| shown_groups.iter().position(|&g| g == section_of(item)).unwrap_or(0)).collect();
        let sections: Vec<RulesetSection> = shown_groups
            .iter()
            .enumerate()
            .map(|(k, &g)| {
                let group = groups[g];
                let of: Vec<&Item> = rows.iter().zip(&self.row_sections).filter(|(_, s)| **s == k).map(|((item, _), _)| item).collect();
                let start = self.row_sections.iter().filter(|s| **s < k).count();
                let allowed = of.iter().filter(|item| ruleset.allows_item(&category.fact, item)).count();
                RulesetSection {
                    name: kit.version_name(names, &group.id).into(),
                    tone: tone(group.color),
                    start: start as i32,
                    count: of.len() as i32,
                    all: allowed < of.len(),
                    none: allowed > 0,
                }
            })
            .collect();
        let section_rows = sections.iter().map(|s| s.count).max().unwrap_or(0);
        let mut slots = vec![0; sections.len()];
        let lines: Vec<RulesetItem> = rows
            .iter()
            .zip(&self.row_sections)
            .map(|((item, name), &section)| {
                let slot = slots.get_mut(section).map_or(0, |n| {
                    *n += 1;
                    *n - 1
                });
                let h = item.def;
                let picture = match category.registry {
                    Registry::Chip => kit.pictures.icon(ChipHandle(h)),
                    Registry::Navi => kit.pictures.navi(NaviHandle(h)),
                    Registry::Form => kit.pictures.face(FormHandle(h)),
                    _ => Default::default(),
                };
                let group = rulesets.group_of(category.registry, item);
                RulesetItem {
                    picture,
                    name: name.as_str().into(),
                    // (None in a version's section, whose head says it.)
                    note: group.filter(|_| !sectioned).map(|g| kit.version_name(names, &g.id)).unwrap_or_default().into(),
                    tone: group.map(|g| tone(g.color)).unwrap_or_default(),
                    allowed: ruleset.allows_item(&category.fact, item),
                    section: if sectioned { section as i32 } else { -1 },
                    slot,
                }
            })
            .collect();
        self.rows = rows.into_iter().map(|(h, _)| h).collect();
        view.set_items(ModelRc::new(VecModel::from(lines)));
        view.set_sections(ModelRc::new(VecModel::from(sections)));
        let allowed = ruleset.allowed_count(&category);
        view.set_category_all(allowed < category.items.len());
        view.set_category_none(allowed > 0);
        view.set_section_rows(section_rows);
    }

    /// A paired category's page: its items (the navis) at the left, the row of every key first, each a version's
    /// under its head, filtered by the search; the chosen key's inner items (its programs) beside them.
    fn show_paired(&mut self, shell: &Shell, names: &Names, outer: &Category, inner: &Category) {
        let ui = shell.ui();
        let view = ui.global::<RulesetView>();
        let kit = self.kit.clone();
        let rulesets = self.rulesets.clone();
        let ruleset = &self.shown.ruleset;
        let tone = |color: u32| {
            let [_, r, g, b] = color.to_be_bytes();
            slint::Color::from_rgb_u8(r, g, b)
        };
        let key_name = |h: u16| match outer.registry {
            Registry::Navi => names.navi_long(NaviHandle(h)),
            r => names.def(r, h),
        };
        // (The keys a version's, by the versions' order; those of none last; navis each in the game's order, by their
        // actor records' numbers.)
        let search = self.search.to_lowercase();
        let groups = &rulesets.groups;
        let content = kit.content();
        let group_of = |i: &Item| rulesets.group_of(outer.registry, i).and_then(|g| groups.iter().position(|o| o.id == g.id)).unwrap_or(groups.len());
        let number = |i: &Item| match outer.registry {
            Registry::Navi => content.identity(content.navi(NaviHandle(i.def)).identity).record.ai_index,
            _ => 0,
        };
        let mut items: Vec<&Item> = outer.items.iter().filter(|i| search.is_empty() || key_name(i.def).to_lowercase().contains(&search)).collect();
        items.sort_by_key(|i| (group_of(i), number(i)));
        let own_bans = |k: u16| ruleset.banned_for(&inner.fact, k).count() as i32;
        let mut rows = vec![RulesetKey {
            all: true,
            name: view.get_all_keys_word(),
            allowed: true,
            note: view.invoke_banned_count(ruleset.banned(&inner.fact).count() as i32),
            ..Default::default()
        }];
        self.keys = vec![None];
        let (mut heads, mut last) = (0, None);
        for item in items {
            let g = group_of(item);
            let head = (last != Some(g)).then(|| groups.get(g)).flatten();
            if last != Some(g) {
                last = Some(g);
                heads += i32::from(head.is_some());
            }
            let allowed = ruleset.allows_item(&outer.fact, item);
            rows.push(RulesetKey {
                all: false,
                picture: if outer.registry == Registry::Navi { kit.pictures.navi(NaviHandle(item.def)) } else { Default::default() },
                name: key_name(item.def).into(),
                allowed,
                note: if allowed { view.invoke_banned_count(own_bans(item.def)) } else { view.get_not_allowed_word() },
                head: head.map(|g| kit.version_name(names, &g.id)).unwrap_or_default().into(),
                tone: head.map(|g| tone(g.color)).unwrap_or_default(),
                heads,
            });
            self.keys.push(Some(item.def));
        }
        // (The key chosen stays where the search leaves it; else every key's.)
        if self.key.is_some_and(|k| !self.keys.contains(&Some(k))) {
            self.key = None;
        }
        let chosen = self.keys.iter().position(|k| *k == self.key).unwrap_or(0);
        // The chosen key's items, in their collection's order.
        let mut programs: Vec<Item> = inner.items.clone();
        if let (Registry::Entry, Some(of)) = (inner.registry, &inner.of) {
            let mut sorted: Vec<((), EntryHandle)> = programs.iter().map(|i| ((), EntryHandle(i.def))).collect();
            sorted.dedup();
            kit.order.entries(content, of, &mut sorted);
            let sorted: Vec<u16> = sorted.into_iter().map(|(_, h)| h.0).collect();
            programs.sort_by_key(|i| sorted.iter().position(|&h| h == i.def).unwrap_or(usize::MAX));
        }
        let lines: Vec<RulesetProgram> = programs
            .iter()
            .map(|item| {
                let all = ruleset.allows_item(&inner.fact, item);
                match self.key {
                    None => RulesetProgram { name: names.def(inner.registry, item.def).into(), allowed: all, inherited: false, note: Default::default() },
                    Some(k) => RulesetProgram {
                        name: names.def(inner.registry, item.def).into(),
                        allowed: all && ruleset.allows_for(&inner.fact, k, item),
                        inherited: !all,
                        note: if all { Default::default() } else { view.get_inherited_note() },
                    },
                }
            })
            .collect();
        let allowed = lines.iter().filter(|l| l.allowed).count() as i32;
        view.set_programs_title(match self.key {
            None => view.get_all_keys_title(),
            Some(k) => view.invoke_key_title(key_name(k).into()),
        });
        view.set_programs_note(view.invoke_allowed(allowed, lines.len() as i32));
        self.programs = programs;
        view.set_keys(ModelRc::new(VecModel::from(rows)));
        view.set_key(chosen as i32);
        view.set_programs(ModelRc::new(VecModel::from(lines)));
        view.set_paired(true);
        view.set_items(Default::default());
        view.set_sections(Default::default());
        view.set_section_rows(0);
        self.rows.clear();
        self.row_sections.clear();
        let n = ruleset.allowed_count(outer);
        view.set_category_all(n < outer.items.len());
        view.set_category_none(n > 0);
        // (The head's note: the navis allowed, and the inner category's bans.)
        view.set_head_note(view.invoke_paired_head(n as i32, outer.items.len() as i32, ruleset.ban_count(&inner.fact) as i32));
    }

    /// Paired: show key row `index`'s items (the row of every key's, or a navi's).
    fn choose_key(&mut self, shell: &Shell, index: i32) {
        let Some(&key) = self.keys.get(index.max(0) as usize) else { return };
        if key == self.key {
            return;
        }
        self.key = key;
        let graphics = self.kit.ready.graphics(shell.lang);
        let kit = self.kit.clone();
        let names = Names::of(kit.content(), &graphics);
        self.show_items(shell, &names);
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

    /// The ruleset files' kind and extensions, as the system's file dialogs ask for them.
    fn files_kind(shell: &Shell) -> String {
        shell.ui().global::<RulesetView>().get_ruleset_files().to_string()
    }

    /// EXPORT: the ruleset shown written to a file where the player says (a ruleset's file, as the player's own are
    /// kept: `store`), named as it is shown (a preset's in the language shown), to be imported elsewhere.
    fn export(&self, shell: &mut Shell) -> Outcome {
        let graphics = self.kit.ready.graphics(shell.lang);
        let names = Names::of(self.kit.content(), &graphics);
        let name = shown_name(&names, &self.shown.ruleset.name);
        let Some(path) = shell.choose_save(&Self::files_kind(shell), &EXTENSIONS, &format!("{}.{}", store::stem(&name), store::EXTENSION)) else { return Outcome::Nothing };
        let mut ruleset = self.shown.ruleset.clone();
        ruleset.name = Name::Own(name);
        match store::write(&path, self.kit.content(), &self.kit.game, &ruleset) {
            Ok(()) => shell.saved(&path),
            Err(why) => {
                crate::errors::log("exporting a ruleset", &why);
                shell.fail(crate::errors::UserError::Other);
            }
        }
        Outcome::Nothing
    }

    /// IMPORT: ruleset files chosen in the system's dialog, one or more, each the player's own (named as its file
    /// says, or for the file where it names none; numbered past the presets' names and the player's own), saved; the last shown in this one's place, as a
    /// duplicate is. A file that isn't a ruleset of the game (another game's, or naming what it hasn't) doesn't stop
    /// the others: the first such is said once they are in.
    fn import(&mut self, shell: &mut Shell) -> Outcome {
        // (The files are heard as `Screen::chosen`, by the screen that shows this page: `import_paths`.)
        shell.choose_files(&Self::files_kind(shell), &EXTENSIONS, RULESET_FILES);
        Outcome::Nothing
    }

    /// IMPORT's files, as [`Page::import`] says.
    pub fn import_paths(&mut self, shell: &mut Shell, paths: &[std::path::PathBuf]) -> Outcome {
        let graphics = self.kit.ready.graphics(shell.lang);
        let kit = self.kit.clone();
        let names = Names::of(kit.content(), &graphics);
        let mut taken: Vec<String> = store::list(&kit.game).into_iter().map(|l| l.name).collect();
        taken.extend(self.rulesets.presets.iter().map(|p| shown_name(&names, &p.name)));
        let (mut made, mut refused) = (None, None);
        for path in paths {
            let read = nettai_platform::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|text| store::read(&self.rulesets, kit.content(), &text));
            let mut ruleset = match read {
                Ok(ruleset) => ruleset,
                Err(why) => {
                    crate::errors::log(&format!("importing the ruleset {}", path.display()), &why);
                    refused.get_or_insert(crate::errors::UserError::NotARuleset { game: Shell::game_name(&kit.game) });
                    continue;
                }
            };
            let own = match &ruleset.name {
                Name::Own(own) => clean_name(own),
                _ => String::new(),
            };
            // (A file that names none is named for itself.)
            let stem = path.file_stem().map(|s| clean_name(&s.to_string_lossy())).unwrap_or_default();
            let name = crate::builds::store::new_name(Some(own.trim_end()).filter(|n| !n.is_empty()).unwrap_or(stem.trim_end()), &taken);
            ruleset.name = Name::Own(name.clone());
            let path = store::new_path(&kit.game, &name);
            if let Err(why) = store::write(&path, kit.content(), &kit.game, &ruleset) {
                crate::errors::log("importing a ruleset", &why);
                refused.get_or_insert(crate::errors::UserError::Other);
                continue;
            }
            taken.push(name);
            made = Some(Pickable { ruleset, path: Some(path) });
        }
        let outcome = match made {
            Some(made) => {
                self.shown = made;
                self.viewed = false;
                self.show(shell);
                shell.sound.play(Cue::Pick);
                Outcome::Changed(self.shown.clone())
            }
            None => Outcome::Nothing,
        };
        if let Some(error) = refused {
            shell.fail(error);
        }
        outcome
    }

    /// Enter, a click or a tap on one of the page's controls.
    pub fn activate(&mut self, shell: &mut Shell, id: &str, index: i32) -> Outcome {
        match id {
            "rules.categories" => {
                self.choose_category(shell, index);
                if !self.keys.is_empty() {
                    shell.nav.focus("rules.pair.keys", 0);
                } else if !self.rows.is_empty() {
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
            // Paired: a key's allow box (the row of every key has none: it is chosen), and an item of the key chosen,
            // held back for it alone or for every key (one every key's ban holds back can't be changed for one).
            "rules.pair.keys" => {
                self.choose_key(shell, index);
                let Some(Some(key)) = self.keys.get(index.max(0) as usize).copied() else { return Outcome::Nothing };
                let fact = self.category().fact.clone();
                let item = Item::of(key);
                self.change(shell, |r, _| {
                    let allowed = r.allows_item(&fact, &item);
                    r.set_allowed(&fact, &item, !allowed);
                })
            }
            "rules.pair.programs" => {
                let Some(item) = self.programs.get(index.max(0) as usize).cloned() else { return Outcome::Nothing };
                let Some(inner) = self.inner_of(self.category()).cloned() else { return Outcome::Nothing };
                match self.key {
                    None => self.change(shell, |r, _| {
                        let allowed = r.allows_item(&inner.fact, &item);
                        r.set_allowed(&inner.fact, &item, !allowed);
                    }),
                    Some(_) if !self.shown.ruleset.allows_item(&inner.fact, &item) && self.editable() => {
                        shell.sound.play(Cue::Refused);
                        Outcome::Nothing
                    }
                    Some(key) => self.change(shell, |r, _| {
                        let allowed = r.allows_for(&inner.fact, key, &item);
                        r.set_allowed_for(&inner, key, &item, !allowed);
                    }),
                }
            }
            "rules.items" => {
                let Some(item) = self.rows.get(index.max(0) as usize).cloned() else { return Outcome::Nothing };
                let fact = self.category().fact.clone();
                self.change(shell, |r, _| {
                    let allowed = r.allows_item(&fact, &item);
                    r.set_allowed(&fact, &item, !allowed);
                })
            }
            // A section's ALL or NONE: its rows shown, all allowed or none.
            "rules.items.all" | "rules.items.none" => {
                let fact = self.category().fact.clone();
                let allow = id == "rules.items.all";
                let of: Vec<Item> = self.rows.iter().zip(&self.row_sections).filter(|(_, s)| **s == index.max(0) as usize).map(|(item, _)| item.clone()).collect();
                self.change(shell, |r, _| {
                    for item in &of {
                        r.set_allowed(&fact, item, allow);
                    }
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
            "rules.export" => self.export(shell),
            "rules.import" => self.import(shell),
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
                let count = self.listed().len() as i32;
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

    /// A field's text: the name, the header's title (saved when it is confirmed, a name another of the player's own
    /// has taken numbered), the search (each change).
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
                let from = self.shown.ruleset.name.clone();
                match self.change(shell, |r, _| r.name = Name::Own(name)) {
                    Outcome::Changed(to) => {
                        crate::rulesets::rename(shell, &self.kit, &from, &to.ruleset.name);
                        Outcome::Renamed { from, to }
                    }
                    // (The same name, or Esc: the header's field shows it as it is kept.)
                    outcome => {
                        self.show(shell);
                        outcome
                    }
                }
            }
            _ => Outcome::Nothing,
        }
    }

    /// The focus moved: along the categories, the category under it is shown.
    pub fn focused(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if id == "rules.categories" && index as usize != self.category {
            self.choose_category(shell, index);
        }
        // (Paired: along the keys, the key under the focus is chosen.)
        if id == "rules.pair.keys" {
            self.choose_key(shell, index);
        }
    }

    /// Show category `index`.
    fn choose_category(&mut self, shell: &mut Shell, index: i32) {
        let index = (index.max(0) as usize).min(self.listed().len().saturating_sub(1));
        if index == self.category {
            return;
        }
        self.category = index;
        self.key = None;
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
        if let Err(why) = nettai_platform::fs::remove_file(path) {
            crate::errors::log(&format!("deleting the ruleset {}", path.display()), &why);
            shell.fail(crate::errors::UserError::Other);
            return Err(self);
        }
        self.close(shell);
        Ok(Outcome::Deleted)
    }
}
