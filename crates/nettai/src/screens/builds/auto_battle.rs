//! The Auto battle tab of a build's editor (docs/design/app-ui.md §6, "Auto battle"), in a game whose setup takes
//! auto battle data (EXE4's and EXE5's).
//!
//! The places and their lists are the game's data (`crate::builds::auto`: its rules/auto_battle/block's `LISTS`, each
//! named in its strings' `auto_battle_lists`; a game whose block states none has its places as one). A build states
//! the places alone (its pattern records are the rules' default, nothing learned), so a place holds a chip or nothing.
//! What a place may hold is the rules' (`validate`: a chip its navi can play): a chip they refuse isn't put, its row
//! shakes and the strip says why. The library is the chips a folder of the build can hold, a row a chip.
//!
//! **Enter on a place goes to the library** (on a phone, its sheet); **Enter on a chip puts it in the place and goes
//! on to the next.** Del empties the place.

use crate::builds::auto::{self, AutoBattle as Data, Edit, Entry, Layout};
use crate::builds::words::kind;
use crate::input::{Action, KeyPress};
use crate::screens::builds::editor::{Draft, Tab};
use crate::shell::Shell;
use crate::sound::Cue;
use nettai_battle::Content;
use nettai_content_api::ChipHandle;
use nettai_match::build::{self, ChipInfo, Sort};
use nettai_ui::{AutoBattleView, BuildWords, ChipCell, ChipDetail, EditorTab, LibraryRow, PlaceLine};
use slint::{ComponentHandle, Model, ModelRc, VecModel};

/// The sorts, in the order the library's select lists them (`AutoBattleView.sorts`).
const SORTS: [Sort; 5] = [Sort::Library, Sort::Name, Sort::Mb, Sort::Element, Sort::Attack];

/// The strings' text table of the lists' names.
const LISTS_TABLE: &str = "auto_battle_lists";

/// The most letters a search has.
const SEARCH_LENGTH: usize = 24;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Described {
    Place(usize),
    Row(usize),
}

#[derive(Default)]
pub struct AutoBattle {
    /// The data's lists, read once (`Some(None)`: the game's block isn't one this tab reads).
    layout: Option<Option<Layout>>,
    /// The place being filled (from 0), and where each place is in the panel (its column and row).
    place: usize,
    at: Vec<(i32, i32)>,
    in_library: bool,
    row: usize,
    search: String,
    sort: usize,
    descending: bool,
    /// The library's chips, and its rows as sorted and searched (a row a chip).
    chips: Vec<ChipInfo>,
    rows: Vec<ChipHandle>,
    described: Option<Described>,
    landed: i32,
    refused: i32,
    /// Why the last chip wasn't put in, until the cursor moves.
    notice: Option<String>,
}

impl AutoBattle {
    fn layout(&mut self, draft: &Draft) -> Option<&Layout> {
        if self.layout.is_none() {
            let content = draft.content();
            let block = crate::builds::module_data(content, &draft.kit.game, auto::MODULE);
            self.layout = Some(Layout::read(content, block.as_ref()));
        }
        self.layout.as_ref().and_then(Option::as_ref)
    }

    /// A chip as a row of the folder shows it, without a code (a place holds a chip in none).
    fn cell(draft: &Draft, words: &crate::builds::words::Words, entry: Entry) -> ChipCell {
        let Entry::Chip(chip) = entry else { return ChipCell::default() };
        let record = draft.content().chip(chip);
        ChipCell {
            there: true,
            icon: draft.kit.pictures.icon(chip),
            name: words.names.chip(chip).into(),
            kind: kind(record.class),
            element: draft.kit.pictures.element(record.family),
            ..Default::default()
        }
    }

    /// The places under their lists' titles, two columns of them (on a phone one), each place where it is.
    fn show_places(&mut self, shell: &Shell, draft: &Draft) {
        let columns = if shell.scale.phone { 1 } else { 2 };
        let Some(layout) = self.layout.clone().flatten() else { return };
        let data = Data::of_side(draft.content(), draft.side());
        self.at = vec![(0, 0); auto::PLACES];
        let mut row = 0;
        let lines: Vec<PlaceLine> = draft.with_words(shell, |words| {
            let mut lines = Vec::new();
            for list in &layout.lists {
                // (A list with no name in a game of one list has no title.)
                if !list.name.is_empty() {
                    let title = words.names.text(LISTS_TABLE, &list.name).unwrap_or_else(|| list.name.clone());
                    lines.push(PlaceLine { title: true, text: title.to_uppercase().into(), row, ..Default::default() });
                    row += 1;
                }
                for (k, place) in list.places().enumerate() {
                    let (column, r) = ((k % columns) as i32, row + (k / columns) as i32);
                    self.at[place] = (column, r);
                    lines.push(PlaceLine { chip: AutoBattle::cell(draft, words, data.places[place]), number: k as i32 + 1, place: place as i32, column, row: r, ..Default::default() });
                }
                row += list.len.div_ceil(columns) as i32;
            }
            lines
        });
        let ui = shell.ui();
        let view = ui.global::<AutoBattleView>();
        view.set_lines(ModelRc::new(VecModel::from(lines)));
        view.set_rows(row);
        view.set_place(self.place as i32);
        view.set_filled(data.entries() as i32);
        view.set_places(auto::PLACES as i32);
    }

    /// The library's rows, as sorted and searched.
    fn list(&mut self, shell: &Shell, draft: &mut Draft) {
        let content = draft.content().clone();
        if self.chips.is_empty() {
            let folder = draft.side().folder(&content);
            let kit = draft.kit.clone();
            if let Some(view) = draft.checked.folder_view(&content, &folder) {
                // (A row a chip: in its first code alone.)
                self.chips = build::library(&content, &kit.game, &view, &|key| kit.order.chip_rank(key)).into_iter().map(|mut c| {
                    c.codes.truncate(1);
                    c
                }).filter(|c| !c.codes.is_empty()).collect();
            }
        }
        let wanted = self.search.to_lowercase();
        self.rows = draft.with_words(shell, |words| {
            let found: Vec<ChipInfo> = self.chips.iter().filter(|c| wanted.is_empty() || words.names.chip(c.chip).to_lowercase().contains(&wanted)).cloned().collect();
            build::sorted(&content, &found, SORTS[self.sort.min(SORTS.len() - 1)], self.descending, &|chip| words.names.chip(chip)).into_iter().map(|c| c.id).collect()
        });
    }

    fn show_library(&self, shell: &Shell, draft: &Draft) {
        let content = draft.content();
        let data = Data::of_side(content, draft.side());
        let ui = shell.ui();
        let view = ui.global::<AutoBattleView>();
        let say = ui.global::<BuildWords>();
        let rows: Vec<LibraryRow> = draft.with_words(shell, |words| {
            self.rows
                .iter()
                .map(|&chip| {
                    let record = content.chip(chip);
                    let copies = data.places.iter().filter(|e| **e == Entry::Chip(chip)).count() as i32;
                    LibraryRow {
                        icon: draft.kit.pictures.icon(chip),
                        name: words.names.chip(chip).into(),
                        kind: kind(record.class),
                        note: if copies > 0 { view.invoke_in_places(copies) } else { Default::default() },
                        refused: false,
                        megabytes: say.invoke_megabytes(record.mb as i32),
                        attack: if record.damage > 0 { record.damage.to_string().into() } else { Default::default() },
                        element: draft.kit.pictures.element(record.family),
                        code: Default::default(),
                    }
                })
                .collect()
        });
        view.set_chips(ModelRc::new(VecModel::from(rows)));
        view.set_search(self.search.as_str().into());
        view.set_sort(self.sort as i32);
        view.set_descending(self.descending);
        view.set_in_library(self.in_library);
    }

    /// The strip: the chip under the cursor, a place's or a row's, and what the rules have against a place.
    fn show_detail(&self, shell: &Shell, draft: &Draft) {
        let content = draft.content();
        let data = Data::of_side(content, draft.side());
        let (chip, place) = match self.described {
            Some(Described::Place(i)) => (match data.places.get(i) {
                Some(Entry::Chip(c)) => Some(*c),
                _ => None,
            }, Some(i)),
            Some(Described::Row(r)) => (self.rows.get(r).copied(), None),
            None => (None, None),
        };
        let ui = shell.ui();
        let say = ui.global::<BuildWords>();
        let detail = draft.with_words(shell, |words| {
            let problem = place.and_then(|i| draft.checked.problems.iter().find(|p| p.field.as_deref() == Some(auto::PLACES_FIELD) && p.entry == Some(i + 1)).map(|p| words.problem(p)));
            let problem = self.notice.clone().or(problem).unwrap_or_default();
            let Some(chip) = chip else { return ChipDetail { problem: problem.into(), ..Default::default() } };
            let record = content.chip(chip);
            let dark = record.flags.has(nettai_battle::content::ChipFlags::DARK);
            let mut facts = vec![say.invoke_class(kind(record.class), dark), say.invoke_megabytes(record.mb as i32)];
            if record.damage > 0 {
                facts.push(say.invoke_attack(record.damage as i32));
            }
            let key = &content.defs.chip(chip).key;
            let get = |s: &nettai_battle::content::strings::Strings| s.chip(key).and_then(|c| c.description.clone());
            let description = words.names.strings.and_then(get).or_else(|| get(&content.strings)).map(|d| crate::games::one_line(&d)).unwrap_or_default();
            ChipDetail {
                there: true,
                art: draft.kit.pictures.art(chip),
                name: words.names.chip(chip).into(),
                kind: kind(record.class),
                description: description.into(),
                element: draft.kit.pictures.element(record.family),
                facts: ModelRc::new(VecModel::from(facts)),
                problem: problem.into(),
                ..Default::default()
            }
        });
        ui.global::<AutoBattleView>().set_detail(detail);
    }

    fn changed(&mut self, shell: &Shell, draft: &mut Draft) {
        self.list(shell, draft);
        self.show_places(shell, draft);
        self.show_library(shell, draft);
        self.show_detail(shell, draft);
    }

    /// The cursor is on place `i`: it is the one being filled.
    fn on_place(&mut self, shell: &Shell, draft: &Draft, i: usize) {
        self.place = i.min(auto::PLACES - 1);
        self.in_library = false;
        self.described = Some(Described::Place(self.place));
        self.notice = None;
        let ui = shell.ui();
        let view = ui.global::<AutoBattleView>();
        view.set_place(self.place as i32);
        view.set_in_library(false);
        self.show_detail(shell, draft);
    }

    /// Put row `row`'s chip in the place being filled, and go on to the next. A chip the rules refuse there isn't
    /// put: its row shakes, and the strip says why.
    fn put(&mut self, shell: &mut Shell, draft: &mut Draft, row: usize) {
        let Some(&chip) = self.rows.get(row) else { return };
        let place = self.place;
        let before = draft.side().clone();
        let field = Some(auto::PLACES_FIELD);
        let had: Vec<String> = draft.checked.problems.iter().filter(|p| p.field.as_deref() == field && p.entry == Some(place + 1)).map(|p| p.text.clone()).collect();
        if !draft.edit(|content, side| auto::update(content, side, Edit::Put(place, chip))) {
            // (The place holds it already: on to the next.)
            return self.next(shell, draft);
        }
        let ui = shell.ui();
        let view = ui.global::<AutoBattleView>();
        let against = draft.checked.problems.iter().find(|p| p.field.as_deref() == field && p.entry == Some(place + 1) && !had.contains(&p.text)).cloned();
        if let Some(problem) = against {
            draft.edit(|_, side| {
                *side = before;
                true
            });
            self.refused += 1;
            view.set_refused(row as i32);
            view.set_refused_seq(self.refused);
            shell.sound.play(Cue::Refused);
            self.notice = Some(draft.with_words(shell, |w| w.problem(&problem)));
            return self.show_detail(shell, draft);
        }
        let (icon, name) = draft.with_words(shell, |w| (draft.kit.pictures.icon(chip), w.names.chip(chip)));
        shell.fly(("lib.rows", row as i32), ("places.place", place as i32), icon, &name);
        self.landed += 1;
        let (column, r) = self.at.get(place).copied().unwrap_or_default();
        view.set_landed_column(column);
        view.set_landed_row(r);
        view.set_landed_seq(self.landed);
        shell.sound.play(Cue::Pick);
        self.next(shell, draft);
    }

    /// On to the next place, back from the library (a phone's sheet shuts).
    fn next(&mut self, shell: &mut Shell, draft: &mut Draft) {
        self.place = (self.place + 1).min(auto::PLACES - 1);
        self.notice = None;
        self.changed(shell, draft);
        self.leave_library(shell);
    }

    /// Back from the library to the place being filled.
    fn leave_library(&mut self, shell: &mut Shell) {
        let ui = shell.ui();
        let view = ui.global::<AutoBattleView>();
        if view.get_sheet() {
            view.set_sheet(false);
            shell.nav.close("lib.");
        }
        self.in_library = false;
        self.described = Some(Described::Place(self.place));
        view.set_in_library(false);
        shell.nav.focus("places.place", self.place as i32);
    }

    /// Empty place `i`.
    fn remove(&mut self, shell: &mut Shell, draft: &mut Draft, i: usize) {
        if draft.edit(|content, side| auto::update(content, side, Edit::Empty(i))) {
            shell.sound.play(Cue::Back);
            self.changed(shell, draft);
        } else {
            shell.sound.play(Cue::Refused);
        }
    }

    fn focused_place(shell: &Shell) -> Option<usize> {
        shell.nav.focused().filter(|k| k.id == "places.place").map(|k| k.index.max(0) as usize)
    }
}

impl Tab for AutoBattle {
    fn id(&self) -> EditorTab {
        EditorTab::AutoBattle
    }

    /// The data's places (its records are none of a build's: the rules' default, nothing learned).
    fn fields(&self, _content: &Content) -> Vec<String> {
        vec![auto::PLACES_FIELD.into()]
    }

    fn own_navi_only(&self) -> bool {
        true
    }

    fn main(&self) -> &'static str {
        "places.place"
    }

    fn show(&mut self, shell: &mut Shell, draft: &mut Draft) {
        self.layout(draft);
        if self.described.is_none() {
            self.described = Some(Described::Place(self.place));
        }
        self.changed(shell, draft);
    }

    fn hide(&mut self, shell: &mut Shell, _draft: &mut Draft) {
        self.notice = None;
        let ui = shell.ui();
        if ui.global::<AutoBattleView>().get_sheet() {
            ui.global::<AutoBattleView>().set_sheet(false);
            shell.nav.close("lib.");
        }
    }

    fn activate(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32) {
        let ui = shell.ui();
        let view = ui.global::<AutoBattleView>();
        match id {
            "places.place" => {
                self.on_place(shell, draft, index.max(0) as usize);
                self.in_library = true;
                view.set_in_library(true);
                let row = self.row.min(self.rows.len().saturating_sub(1)) as i32;
                let to = if self.rows.is_empty() { crate::nav::Key::new("lib.search", 0) } else { crate::nav::Key::new("lib.rows", row) };
                if shell.scale.phone {
                    view.set_sheet(true);
                    shell.nav.open("lib.", to);
                } else {
                    shell.nav.focus(&to.id, to.index);
                }
            }
            "lib.rows" => self.put(shell, draft, index.max(0) as usize),
            "lib.sort" => {
                let options = view.get_sorts().iter().map(|s| s.to_string()).collect();
                shell.pick(id, 0, options, self.sort);
            }
            "lib.direction" => {
                self.descending = !self.descending;
                self.changed(shell, draft);
            }
            "lib.remove" => {
                self.remove(shell, draft, self.place);
                self.leave_library(shell);
            }
            "lib.close" => self.leave_library(shell),
            _ => {}
        }
    }

    fn step(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, _index: i32, by: i32) {
        if id == "lib.sort" {
            self.sort = (self.sort as i32 + by).rem_euclid(SORTS.len() as i32) as usize;
            self.changed(shell, draft);
        }
    }

    fn choose(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, _index: i32, choice: i32) {
        if id == "lib.sort" {
            self.sort = (choice.max(0) as usize).min(SORTS.len() - 1);
            self.changed(shell, draft);
        }
    }

    fn edit(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, _index: i32, text: &str, _done: bool) {
        if id == "lib.search" {
            self.search = text.chars().filter(|c| !c.is_control()).take(SEARCH_LENGTH).collect();
            self.changed(shell, draft);
        }
    }

    fn focused(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32) {
        let ui = shell.ui();
        let view = ui.global::<AutoBattleView>();
        match id {
            "places.place" => self.on_place(shell, draft, index.max(0) as usize),
            "lib.rows" => {
                self.in_library = true;
                self.row = index.max(0) as usize;
                self.described = Some(Described::Row(self.row));
                self.notice = None;
                view.set_in_library(true);
                self.show_detail(shell, draft);
            }
            id if id.starts_with("lib.") => {
                self.in_library = true;
                view.set_in_library(true);
            }
            _ => {}
        }
    }

    /// In the library, a letter typed starts a search.
    fn key(&mut self, shell: &mut Shell, _draft: &mut Draft, key: &KeyPress) -> bool {
        let in_rows = shell.nav.focused().is_some_and(|k| k.id == "lib.rows");
        let typed = key.text.as_deref().filter(|t| t.chars().count() == 1 && t.chars().all(|c| c.is_alphanumeric()));
        if in_rows
            && let Some(typed) = typed
            && !matches!(shell.bindings.of_key(key.code), Some(nettai_battle::input::keys::L | nettai_battle::input::keys::R))
        {
            shell.nav.focus("lib.search", 0);
            shell.nav.begin_edit(&shell.ui(), typed);
            return true;
        }
        false
    }

    /// Delete empties the place.
    fn action(&mut self, shell: &mut Shell, draft: &mut Draft, action: Action) -> bool {
        match (action, AutoBattle::focused_place(shell)) {
            (Action::Delete, Some(i)) => self.remove(shell, draft, i),
            _ => return false,
        }
        true
    }

    /// Esc in the library goes back to the place being filled.
    fn back(&mut self, shell: &mut Shell, _draft: &mut Draft) -> bool {
        if shell.nav.focused().is_some_and(|k| k.id.starts_with("lib.")) || shell.ui().global::<AutoBattleView>().get_sheet() {
            self.leave_library(shell);
            return true;
        }
        false
    }
}
