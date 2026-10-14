//! The Auto battle tab of a build's editor (docs/design/app-ui.md §6, "Auto battle"), in a game whose setup takes
//! auto battle data (EXE4's and EXE5's).
//!
//! The places and their lists are the game's data (`crate::builds::auto`: its rules/auto_battle/block's `LISTS`, each
//! named in its strings' `auto_battle_lists`; a game whose block states none has its places as one). A build states
//! the places alone (its pattern records are the rules' default, nothing learned), so a place holds a chip or nothing.
//! What a place may hold is the rules' (`validate`: a chip its navi can play, and in a list they hold to one class of
//! chip, one of that class): a chip they refuse isn't put, its row shakes and the strip says why. The library offers
//! what the place being filled can hold, a row a chip: in a list held to one class (the list's `class`: EXE4's and
//! EXE5's mega, giga and program advance lists), the game's chips of that class, the Program Advances among them,
//! which no folder holds; in another, the chips a folder of the build can hold.
//!
//! **Enter on a place goes to the library** (on a phone, its sheet); **Enter on a chip puts it in the place and goes
//! on to the next;** Enter (or Del, or a click) on the chip the place holds already takes it out again. Del empties
//! the place. **The mouse:** a click on a place marks it (the focus stays on it); a click on the marked one empties
//! it, as Del does. A place's chip dragged to another place of its list moves there, the places between shifting
//! toward where it was (`Edit::Move`); to a place of another list, the two trade what they hold (`Edit::Swap`), unless
//! the rules refuse either there: then nothing changes, the place dragged shakes, and the strip says why, as a chip
//! put from the library is refused. A pattern's places aren't shown: a build states no pattern records, so none of its places can point at
//! one (the game's own writer puts none there but a learned pattern's), and a match is sent them empty, a valid block.

use crate::builds::auto::{self, AutoBattle as Data, Edit, Entry, Holds};
use crate::builds::words::kind;
use crate::input::Action;
use crate::screens::builds::editor::{self, Draft, Tab};
use crate::shell::Shell;
use crate::sound::Cue;
use nettai_battle::Content;
use nettai_battle::content::ChipCode;
use nettai_content_api::ChipHandle;
use nettai_match::build::{self, ChipInfo, Sort};
use nettai_ui::{AutoBattleView, BuildWords, ChipCell, ChipDetail, EditorTab, LibraryRow, PlaceLine};
use slint::{ComponentHandle, Model, ModelRc, VecModel};

/// The sorts, in the order the library's select lists them (`AutoBattleView.sorts`).
const SORTS: [Sort; 5] = [Sort::Library, Sort::Name, Sort::Mb, Sort::Element, Sort::Attack];

/// The most letters a search has.
const SEARCH_LENGTH: usize = 24;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Described {
    Place(usize),
    Row(usize),
}

#[derive(Default)]
pub struct AutoBattle {
    /// The place being filled (from 0), and where each place is in the panel (its column and row).
    place: usize,
    at: Vec<(i32, i32)>,
    /// The places shown, in their order (a pattern's aren't).
    editable: Vec<usize>,
    in_library: bool,
    row: usize,
    search: String,
    sort: usize,
    descending: bool,
    /// The library's chips, what the places they are offered for hold, and its rows as sorted and searched (a row a
    /// chip).
    chips: Vec<ChipInfo>,
    chips_for: Option<Holds>,
    rows: Vec<ChipHandle>,
    described: Option<Described>,
    landed: i32,
    refused: i32,
    /// Why the last chip wasn't put in, until the cursor moves.
    notice: Option<String>,
}

impl AutoBattle {
    /// What the place being filled holds: its list's (any chip where the game lays out no lists).
    fn holds(&self, draft: &Draft) -> Holds {
        draft.kit.auto_layout().and_then(|l| l.list_of(self.place)).map_or(Holds::Any, |l| l.holds)
    }

    /// A chip as a row of the folder shows it, without a code (a place holds a chip in none); `flagged`: the rules
    /// have something against the place.
    fn cell(draft: &Draft, words: &crate::builds::words::Words, entry: Entry, flagged: bool) -> ChipCell {
        let Entry::Chip(chip) = entry else { return ChipCell { flagged, ..Default::default() } };
        let record = draft.content().chip(chip);
        ChipCell {
            there: true,
            icon: draft.kit.pictures.icon(chip),
            name: words.names.chip(chip).into(),
            kind: kind(record.class),
            element: draft.kit.pictures.chip_element(chip),
            flagged,
            ..Default::default()
        }
    }

    /// Whether the rules have something against place `place` (from 0).
    fn flagged(draft: &Draft, place: usize) -> bool {
        draft.checked.problems.iter().any(|p| p.field.as_deref() == Some(auto::PLACES_FIELD) && p.entry == Some(place))
    }

    /// The places under their lists' titles, two columns of them (on a phone one), each place where it is.
    fn show_places(&mut self, shell: &Shell, draft: &Draft) {
        let columns = if shell.scale.phone { 1 } else { 2 };
        let kit = draft.kit.clone();
        let Some(layout) = kit.auto_layout() else { return };
        let data = Data::of_side(draft.content(), draft.side());
        self.at = vec![(0, 0); auto::PLACES];
        self.editable.clear();
        let mut row = 0;
        let lines: Vec<PlaceLine> = draft.with_words(shell, |words| {
            let mut lines = Vec::new();
            // (A pattern's places aren't shown: a build states no pattern records, the rules' default being
            // nothing learned, so no place of it points at one, and the game never puts a chip there.)
            for list in layout.lists.iter().filter(|l| l.holds != Holds::Patterns) {
                // (A list with no name in a game of one list has no title.)
                if !list.name.is_empty() {
                    let title = words.names.text(auto::LISTS_TABLE, &list.name).unwrap_or_else(|| list.name.clone());
                    lines.push(PlaceLine { title: true, text: title.to_uppercase().into(), row, ..Default::default() });
                    row += 1;
                }
                for (k, place) in list.places().enumerate() {
                    let (column, r) = ((k % columns) as i32, row + (k / columns) as i32);
                    self.at[place] = (column, r);
                    self.editable.push(place);
                    let chip = AutoBattle::cell(draft, words, data.places[place], AutoBattle::flagged(draft, place));
                    lines.push(PlaceLine { chip, number: k as i32 + 1, place: place as i32, column, row: r, ..Default::default() });
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
        view.set_filled(self.editable.iter().filter(|&&p| data.places[p] != Entry::Empty).count() as i32);
        view.set_places(self.editable.len() as i32);
    }

    /// The library's rows, as sorted and searched: of the chips the place being filled can hold.
    fn list(&mut self, shell: &Shell, draft: &mut Draft) {
        let content = draft.content().clone();
        let holds = self.holds(draft);
        if self.chips_for != Some(holds) {
            let kit = draft.kit.clone();
            let rank = |key: &str| kit.order.chip_rank(key);
            let chips = match holds {
                // (A list the rules hold to one class: the game's chips of it, whether a folder holds them or not.)
                Holds::Chips(class) => Some(auto::of_class(&content, &kit.game, class).into_iter().map(|c| build::chip_info(&content, c, &rank)).collect()),
                _ => {
                    let folder = draft.side().folder(&content);
                    draft.checked.folder_view(&content, &folder).map(|view| build::library(&content, &kit.game, &view, &rank))
                }
            };
            // (A row a chip: in its first code alone, and a chip of no code, some Program Advances, in one. No
            // folder's view, the round not starting: no chips, asked again next time.)
            self.chips = chips
                .unwrap_or_default()
                .into_iter()
                .map(|mut c: ChipInfo| {
                    c.codes.truncate(1);
                    if c.codes.is_empty() {
                        c.codes.push(ChipCode::ASTERISK);
                    }
                    c
                })
                .collect();
            self.chips_for = (!self.chips.is_empty()).then_some(holds);
            self.row = 0;
        }
        let wanted = self.search.to_lowercase();
        self.rows = draft.with_words(shell, |words| {
            let found: Vec<ChipInfo> = self.chips.iter().filter(|c| wanted.is_empty() || words.names.chip(c.chip).to_lowercase().contains(&wanted)).cloned().collect();
            build::sorted(&content, &found, SORTS[self.sort.min(SORTS.len() - 1)], self.descending, &|chip| words.names.chip(chip)).into_iter().map(|c| c.id).collect()
        });
    }

    /// The library's row under the cursor is the chip the place being filled holds (Confirm takes it out again).
    fn on_placed(&self, shell: &Shell, draft: &Draft) -> bool {
        let Some(row) = shell.nav.focused().filter(|k| k.id == "lib.rows").map(|k| k.index.max(0) as usize) else { return false };
        let place = Data::of_side(draft.content(), draft.side()).places.get(self.place).copied();
        self.rows.get(row).is_some_and(|&chip| place == Some(Entry::Chip(chip)))
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
                        element: draft.kit.pictures.chip_element(chip),
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
        view.set_on_placed(self.on_placed(shell, draft));
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
            let problem = place.and_then(|i| draft.checked.problems.iter().find(|p| p.field.as_deref() == Some(auto::PLACES_FIELD) && p.entry == Some(i)).map(|p| words.problem(p)));
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
                element: draft.kit.pictures.chip_element(chip),
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

    /// The cursor is on place `i`: it is the one being filled. A place of a list that holds other chips than the last
    /// one's has its library of them, from its first row.
    fn on_place(&mut self, shell: &Shell, draft: &mut Draft, i: usize) {
        self.place = i.min(auto::PLACES - 1);
        self.in_library = false;
        self.described = Some(Described::Place(self.place));
        self.notice = None;
        let ui = shell.ui();
        let view = ui.global::<AutoBattleView>();
        view.set_place(self.place as i32);
        view.set_in_library(false);
        // (Confirm on a place goes to the library, whatever it holds.)
        view.set_on_placed(false);
        if self.chips_for != Some(self.holds(draft)) {
            self.list(shell, draft);
            self.show_library(shell, draft);
        }
        self.show_detail(shell, draft);
    }

    /// Put row `row`'s chip in the place being filled, and go on to the next. A chip the rules refuse there isn't
    /// put: its row shakes, and the strip says why.
    fn put(&mut self, shell: &mut Shell, draft: &mut Draft, row: usize) {
        let Some(&chip) = self.rows.get(row) else { return };
        let place = self.place;
        let before = draft.side().clone();
        let field = Some(auto::PLACES_FIELD);
        let had: Vec<String> = draft.checked.problems.iter().filter(|p| p.field.as_deref() == field && p.entry == Some(place)).map(|p| p.text.clone()).collect();
        // (The chip the place holds already: taken out again, from where the cursor is; it flies back to its row.)
        if Data::of_side(draft.content(), draft.side()).places.get(place) == Some(&Entry::Chip(chip)) {
            let (icon, name) = draft.with_words(shell, |w| (draft.kit.pictures.icon(chip), w.names.chip(chip)));
            shell.fly(("places.place", place as i32), ("lib.rows", row as i32), icon, &name);
            self.remove(shell, draft, place);
            return self.leave_library(shell);
        }
        if !draft.edit(|content, side| auto::update(content, side, Edit::Put(place, chip))) {
            return shell.sound.play(Cue::Refused);
        }
        let ui = shell.ui();
        let view = ui.global::<AutoBattleView>();
        let against = draft.checked.problems.iter().find(|p| p.field.as_deref() == field && p.entry == Some(place) && !had.contains(&p.text)).cloned();
        if let Some(problem) = against {
            draft.edit(|_, side| {
                *side = before;
                true
            });
            self.refused += 1;
            view.set_refused(row as i32);
            view.set_refused_place(-1);
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
        // (The next place shown: a pattern's are passed over.)
        let at = self.editable.iter().position(|&p| p == self.place);
        self.place = at.and_then(|i| self.editable.get(i + 1)).copied().unwrap_or(self.place);
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
        view.set_on_placed(false);
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

    /// The list place `place` is in (where the game lays out none, all are one).
    fn list_of(draft: &Draft, place: usize) -> Option<usize> {
        draft.kit.auto_layout().and_then(|l| l.lists.iter().position(|list| list.places().contains(&place)))
    }

    /// Where place `from`'s chip dragged to `at` (the window's point) would land: the place under the pointer, and
    /// whether it is another list's (the two trade what they hold); none off the places.
    fn drop_at(&self, shell: &Shell, draft: &Draft, from: usize, at: i32) -> Option<(usize, bool)> {
        let to = editor::nearest(shell, "places.place", self.editable.iter().copied(), editor::pointer_at(at), Some("places."))?;
        Some((to, AutoBattle::list_of(draft, to) != AutoBattle::list_of(draft, from)))
    }

    /// A drag of place `from` begun or moved on: the place, as the row under the pointer shows it (with the number of
    /// the place it would land in), and where it would land.
    fn drag_over(&self, shell: &Shell, draft: &Draft, from: usize, at: i32) {
        let ui = shell.ui();
        let view = ui.global::<AutoBattleView>();
        let drop = self.drop_at(shell, draft, from, at);
        let lines = view.get_lines();
        let line = |place: usize| lines.iter().find(|l| !l.title && l.place == place as i32);
        if let Some(mut lifted) = line(from) {
            lifted.number = drop.and_then(|d| line(d.0)).map_or(lifted.number, |l| l.number);
            view.set_lifted(lifted);
        }
        editor::show_drop(shell, drop.map(|d| d.0), drop.is_some_and(|d| d.1));
    }

    /// Place `from`'s chip let go over place `to`: moved there within its list, or traded with another list's, unless
    /// the rules refuse a chip where it would go (then it shakes, and the strip says why).
    fn reorder(&mut self, shell: &mut Shell, draft: &mut Draft, from: usize, to: usize, swap: bool) {
        let before = draft.side().clone();
        let field = Some(auto::PLACES_FIELD);
        let had: Vec<String> = draft.checked.problems.iter().filter(|p| p.field.as_deref() == field).map(|p| p.text.clone()).collect();
        let change = if swap { Edit::Swap(from, to) } else { Edit::Move(from, to) };
        if !draft.edit(|content, side| auto::update(content, side, change)) {
            return;
        }
        let ui = shell.ui();
        let view = ui.global::<AutoBattleView>();
        // (Within a list nothing is refused: its places hold the same chips. Across, each place the trade fills is
        // held to what the rules let it hold.)
        let against = draft.checked.problems.iter().find(|p| p.field.as_deref() == field && (p.entry == Some(from) || p.entry == Some(to)) && !had.contains(&p.text)).cloned();
        if let Some(problem) = against.filter(|_| swap) {
            draft.edit(|_, side| {
                *side = before;
                true
            });
            self.refused += 1;
            view.set_refused(-1);
            view.set_refused_place(from as i32);
            view.set_refused_seq(self.refused);
            shell.sound.play(Cue::Refused);
            self.notice = Some(draft.with_words(shell, |w| w.problem(&problem)));
            return self.show_detail(shell, draft);
        }
        shell.sound.play(Cue::Pick);
        self.place = to;
        self.described = Some(Described::Place(to));
        self.notice = None;
        self.changed(shell, draft);
        self.landed += 1;
        let (column, r) = self.at.get(to).copied().unwrap_or_default();
        view.set_landed_column(column);
        view.set_landed_row(r);
        view.set_landed_seq(self.landed);
        shell.nav.focus("places.place", to as i32);
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

    fn choose(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, choice: i32) {
        match id {
            "lib.sort" => {
                self.sort = (choice.max(0) as usize).min(SORTS.len() - 1);
                self.changed(shell, draft);
            }
            // A place's chip dragged: where it would land; let go there, it moves (off the places, it goes back).
            "drag-over" if index >= 0 => self.drag_over(shell, draft, index as usize, choice),
            "drag-drop" if index >= 0 => {
                editor::show_drop(shell, None, false);
                if let Some((to, swap)) = self.drop_at(shell, draft, index as usize, choice).filter(|d| d.0 != index as usize) {
                    self.reorder(shell, draft, index as usize, to, swap);
                }
            }
            _ => {}
        }
    }

    fn edit(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, _index: i32, text: &str, _done: bool) {
        if id == "lib.search" {
            self.search = text.chars().filter(|c| !c.is_control()).take(SEARCH_LENGTH).collect();
            self.changed(shell, draft);
            // (What the search finds is shown from its top.)
            let ui = shell.ui();
            shell.nav.list_to_top(&ui, "lib.rows");
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
                view.set_on_placed(self.on_placed(shell, draft));
                self.show_detail(shell, draft);
            }
            id if id.starts_with("lib.") => {
                view.set_on_placed(false);
                self.in_library = true;
                view.set_in_library(true);
            }
            _ => {}
        }
    }

    /// Delete empties the place.
    fn action(&mut self, shell: &mut Shell, draft: &mut Draft, action: Action) -> bool {
        match (action, AutoBattle::focused_place(shell)) {
            (Action::Delete, Some(i)) => self.remove(shell, draft, i),
            // (In the library, Del on the chip the place holds takes it out, as Confirm does.)
            (Action::Delete, None) if self.on_placed(shell, draft) => self.put(shell, draft, self.row),
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
