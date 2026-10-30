//! The Folder tab of a build's editor (docs/design/app-ui.md §6, "Folder"; §2 items 11 to 14).
//!
//! Everything it says of the folder is the game's rules' (nettai-match): the limits and what the folder holds
//! against them and why each chip can't go in (`folders::View`, asked once for a folder as it stands and read
//! for every row), what is wrong with an entry (`folders::about_entry`), whether an entry can be the Regular
//! chip or a tag chip (`Checked::mark_problems`), the library's chips and their sorts (`build::library`,
//! `build::sorted`). The build's ruleset keeps out a chip it holds back (docs/design/app-ui.md §2.68): its
//! library row is greyed and says "Not allowed" at its end, and putting it in is refused as the rules' refusals are.
//! A build that holds one all the same (made under another ruleset) has each entry of it as its problem
//! (`Wrong::NotAllowed`, about every copy).
//!
//! **Enter on an entry goes to the library; Enter on a row puts that chip in the entry and goes on to the next
//! entry.** A chip put in takes the place of the entry's own, so a row's reason is asked of the folder without
//! the entry being filled: with the folder whole, a row still says only what would keep its chip out.
//!
//! **R** makes the entry under the cursor the Regular chip (or no longer), **T** tags it: the first T marks
//! one entry and the second its pair, and the two are the build's tag chips once the rules take them. **Del**
//! takes the entry's chip out. A mark the rules refuse isn't made, and the strip says why. **CLEAR** at the right end of
//! the folder's header (or **X**; docs/design/app-ui.md §2.71) empties every entry, the Regular and tag marks with
//! them, as one change, asked first (`editor::ask_clear`).
//!
//! **The mouse:** a click on an entry marks it (the focus stays on it); a right click takes its chip out, as Del does.
//! An entry dragged to another place moves there, those between shifting toward where it was, its Regular and tag
//! marks with it (`FolderEdit::Move`); dragged onto the library, its chip is taken out. A library's row dragged onto an
//! entry puts its chip in that entry, in place of what it held, as a click would put it in the marked one (refused as
//! a click's is); the entry is then the one marked.

use crate::builds::edit::{self, FolderEdit};
use crate::builds::overview::chip_cell;
use crate::builds::words::{Words, kind};
use crate::games::one_line;
use crate::input::{Action, KeyPress};
use crate::screens::builds::editor::{self, Draft, Tab};
use crate::shell::Shell;
use crate::sound::Cue;
use nettai_battle::Content;
use nettai_battle::content::PlayerFact;
use nettai_battle::content::strings::Strings;
use nettai_battle::custom::FolderChip;
use nettai_content_api::{ChipHandle, Registry};
use nettai_match::build::{self, ChipInfo, Sort};
use nettai_match::check::Problem;
use nettai_match::folders::{self, View};
use nettai_match::ruleset::Item;
use nettai_match::{Folder as Chips, Wrong};
use nettai_ui::{ChipCell, ChipDetail, ChipMark, EditorTab, FolderCounts, FolderView, LibraryRow};
use slint::winit_030::winit::keyboard::KeyCode;
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::collections::HashMap;

/// The sorts, in the order the library's select lists them (the UI's `FolderView.sorts`).
const SORTS: [Sort; 6] = [Sort::Library, Sort::Name, Sort::Code, Sort::Mb, Sort::Element, Sort::Attack];

/// The most letters a search has.
const SEARCH_LENGTH: usize = 24;

#[derive(Default)]
pub struct Folder {
    /// The entry being filled: the one the cursor was last on.
    slot: usize,
    /// The cursor is in the library, and the row it was last on there.
    in_library: bool,
    row: usize,
    search: String,
    sort: usize,
    descending: bool,
    /// The library's chips, and its rows as sorted and searched.
    chips: Vec<ChipInfo>,
    rows: Vec<FolderChip>,
    /// The folder's rules' view of the folder as it stands, and of it without the entry being filled (what a
    /// row's reason is read from).
    held: Option<View>,
    offered: Option<View>,
    /// The entry tagged first, waiting for its pair.
    tagging: Option<usize>,
    /// Why the last mark wasn't made, until the cursor moves.
    notice: Option<String>,
    /// The chip the strip describes: an entry's, or a row's; and the entry or row the pointer is on, which it
    /// describes in its place (`editor::hover`).
    described: Option<Described>,
    hovered: Option<Described>,
    landed: i32,
    refused: i32,
    /// The library's chip being dragged into the folder, as its ghost shows it.
    ghost: Option<FolderChip>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Described {
    Slot(usize),
    Row(usize),
}

/// A chip's description in the language shown, on one line; none where the strings have none.
fn description(words: &Words, chip: ChipHandle) -> String {
    let names = words.names;
    let key = &names.content.defs.chip(chip).key;
    let get = |s: &Strings| s.chip(key).and_then(|c| c.description.clone());
    names.strings.and_then(get).or_else(|| get(&names.content.strings)).map(|d| one_line(&d)).unwrap_or_default()
}

/// The build's problems that entry `i` of `folder` is part of (`folders::about_entry`: its own, its copies', its class's
/// limit; an empty one's, the folder's size), the ruleset's among them (a chip it doesn't allow, a version's Giga chip
/// in a build that mixes versions under one version per build): what the strip says of it, its name in the error
/// red (docs/design/app-ui.md §2.66). `fields` are the tab's.
fn entry_problems<'d>(draft: &'d Draft, fields: &[String], folder: &Chips, i: usize) -> Vec<&'d Problem> {
    let content = draft.content();
    let chip = folder.chips.get(i).copied().flatten();
    draft
        .pane_problems(fields, false)
        .into_iter()
        .filter(|p| match &p.what {
            Wrong::Mixed { items, .. } => chip.is_some_and(|c| items.contains(&(Registry::Chip, Item::of(c.id.0)))),
            _ => folders::about_entry(content, folder, i, p),
        })
        .collect()
}

/// The folder's problems no entry is part of and no count of its header says (a fact of it a build made by hand
/// states wrong): every entry's strip says them after its own (docs/design/app-ui.md §2.66).
fn unplaced<'d>(draft: &'d Draft, fields: &[String], folder: &Chips) -> Vec<&'d Problem> {
    let counted = |w: &Wrong| matches!(w, Wrong::FolderSize(_) | Wrong::Class { .. } | Wrong::RegularMemory { .. } | Wrong::TagMemory(_));
    let placed: Vec<&Problem> = (0..folder.chips.len()).flat_map(|i| entry_problems(draft, fields, folder, i)).collect();
    draft.pane_problems(fields, false).into_iter().filter(|p| !counted(&p.what) && !placed.iter().any(|q| std::ptr::eq(*q, *p))).collect()
}

impl Folder {
    /// The folder as the tab shows it: the build's, with the entry tagged first marked while it waits.
    fn shown(&self, draft: &Draft) -> Chips {
        let mut folder = draft.side().folder(draft.content());
        if let Some(first) = self.tagging.filter(|&i| folder.has(i as u8)) {
            folder.tags = Some((first as u8, first as u8));
        }
        folder
    }

    /// Ask the rules of the folder again: as it stands, and without the entry being filled.
    fn ask(&mut self, draft: &mut Draft) {
        let content = draft.content().clone();
        let folder = draft.side().folder(&content);
        self.held = draft.checked.folder_view(&content, &folder);
        let mut without = folder;
        if let Some(entry) = without.chips.get_mut(self.slot) {
            *entry = None;
        }
        self.offered = draft.checked.folder_view(&content, &without);
    }

    /// The library's rows, as sorted and searched.
    fn list(&mut self, shell: &Shell, draft: &Draft) {
        let content = draft.content();
        let kit = &draft.kit;
        if self.chips.is_empty()
            && let Some(view) = &self.held
        {
            self.chips = build::library(content, &kit.game, view, &|key| kit.order.chip_rank(key));
        }
        draft.with_words(shell, |words| {
            let wanted = self.search.to_lowercase();
            let found: Vec<ChipInfo> = self.chips.iter().filter(|c| wanted.is_empty() || words.names.chip(c.chip).to_lowercase().contains(&wanted)).cloned().collect();
            self.rows = build::sorted(content, &found, SORTS[self.sort.min(SORTS.len() - 1)], self.descending, &|chip| words.names.chip(chip));
        });
    }

    /// The folder's pane: its entries, each marked where the build's problems are about it, its counts (one past its
    /// limit in the error red), and in a phone's header what the game's rules have against the folder or an entry of
    /// it (docs/design/app-ui.md §2.66).
    fn show_folder(&self, shell: &Shell, draft: &Draft) {
        let content = draft.content();
        let folder = self.shown(draft);
        let stood = draft.side().folder(content);
        let ui = shell.ui();
        let view = ui.global::<FolderView>();
        let megabytes = |entry: u8| folder.chips.get(entry as usize).copied().flatten().map_or(0, |c| content.chip(c.id).mb as i32);
        let fields = self.fields(content);
        let cells: Vec<ChipCell> = draft.with_words(shell, |words| {
            (0..folder.chips.len())
                .map(|i| {
                    let flagged = stood.chips[i].is_some() && !entry_problems(draft, &fields, &stood, i).is_empty();
                    let mut cell = chip_cell(words.kit, words.names, &folder, i, flagged);
                    // (A chip of one version's own is marked so: a Giga chip's version.)
                    if let Some(chip) = folder.chips.get(i).copied().flatten() {
                        cell.version = words.kit.version_tag_of(words.names, Registry::Chip, &Item::of(chip.id.0));
                    }
                    // (The entry tagged first, waiting for its pair.)
                    if self.tagging == Some(i) {
                        cell.mark = ChipMark::Tag1;
                    }
                    cell
                })
                .collect()
        });
        view.set_slots(ModelRc::new(VecModel::from(cells)));
        view.set_problems(draft.pane_line(shell, &fields, false).into());
        view.set_slot(self.slot as i32);
        if let Some(held) = &self.held {
            let tags = stood.tags.map_or(0, |(a, b)| megabytes(a) + megabytes(b));
            view.set_counts(FolderCounts {
                chips: held.chips as i32,
                size: held.size as i32,
                mega: held.mega.used as i32,
                mega_limit: held.mega.limit as i32,
                giga: held.giga.used as i32,
                giga_limit: held.giga.limit as i32,
                regular: stood.regular.map_or(0, megabytes),
                regular_limit: held.regular_memory as i32,
                tag: tags,
                tag_limit: held.tag_memory.unwrap_or(0) as i32,
                tags: held.tag_memory.is_some(),
            });
        }
    }

    /// The library's pane: each row with "Not allowed" where the build's ruleset holds its chip back, else
    /// why its chip can't go in (the game's rules'), greyed either way; else how many the folder holds.
    fn show_library(&self, shell: &Shell, draft: &Draft) {
        let content = draft.content();
        let kit = &draft.kit;
        let folder = draft.side().folder(content);
        let fact = content.defs.fact_name(PlayerFact::Folder).unwrap_or_default();
        let mut copies: HashMap<FolderChip, i32> = HashMap::new();
        for chip in folder.chips() {
            *copies.entry(chip).or_default() += 1;
        }
        let ui = shell.ui();
        let view = ui.global::<FolderView>();
        let say = ui.global::<nettai_ui::BuildWords>();
        let rows: Vec<LibraryRow> = draft.with_words(shell, |words| {
            // (What a row shows of its chip is the same in each of its codes: made once a chip.)
            let mut parts: HashMap<ChipHandle, LibraryRow> = HashMap::new();
            self.rows
                .iter()
                .map(|row| {
                    let part = parts.entry(row.id).or_insert_with(|| {
                        let record = content.chip(row.id);
                        let refusal = self.offered.as_ref().and_then(|v| v.refusal(row.id));
                        let banned = !draft.allows(fact, row.id.0);
                        LibraryRow {
                            icon: kit.pictures.icon(row.id),
                            name: words.names.chip(row.id).into(),
                            kind: kind(record.class),
                            note: match refusal {
                                _ if banned => say.invoke_not_allowed_here(),
                                Some(wrong) => words.refusal(row.id, wrong).into(),
                                None => Default::default(),
                            },
                            refused: banned || refusal.is_some(),
                            megabytes: say.invoke_megabytes(record.mb as i32),
                            attack: if record.damage > 0 { record.damage.to_string().into() } else { Default::default() },
                            element: kit.pictures.chip_element(row.id),
                            code: Default::default(),
                            version: kit.version_tag_of(words.names, Registry::Chip, &Item::of(row.id.0)),
                        }
                    });
                    let mut line = part.clone();
                    line.code = row.code.letter().to_string().into();
                    if !line.refused
                        && line.note.is_empty()
                        && let Some(&n) = copies.get(row)
                    {
                        line.note = say.invoke_in_folder(n);
                    }
                    line
                })
                .collect()
        });
        view.set_rows(ModelRc::new(VecModel::from(rows)));
        view.set_search(self.search.as_str().into());
        view.set_sort(self.sort as i32);
        view.set_descending(self.descending);
        view.set_in_library(self.in_library);
    }

    /// The strip: the chip under the cursor, an entry's or a row's; while the pointer is on one, that one.
    fn show_detail(&self, shell: &Shell, draft: &Draft) {
        let content = draft.content();
        let kit = &draft.kit;
        let folder = draft.side().folder(content);
        let ui = shell.ui();
        let view = ui.global::<FolderView>();
        let say = ui.global::<nettai_ui::BuildWords>();
        let hovered = self.hovered.filter(|h| match *h {
            Described::Slot(i) => i < folder.chips.len(),
            Described::Row(r) => r < self.rows.len(),
        });
        // (Why the last mark wasn't made is the focused entry's.)
        let notice = if hovered.is_some_and(|h| Some(h) != self.described) { None } else { self.notice.clone() };
        let chip = match hovered.or(self.described) {
            Some(Described::Slot(i)) => folder.chips.get(i).copied().flatten(),
            Some(Described::Row(r)) => self.rows.get(r).copied(),
            None => None,
        };
        // (What the build's problems have against the entry described, the ruleset's among them, and after them what is
        // against the folder and said by none of its entries or counts: said where nothing was refused,
        // docs/design/app-ui.md §2.66.)
        let fault = match hovered.or(self.described) {
            Some(Described::Slot(i)) => {
                let fields = self.fields(content);
                draft.line(shell, entry_problems(draft, &fields, &folder, i).into_iter().chain(unplaced(draft, &fields, &folder)))
            }
            _ => String::new(),
        };
        let detail = draft.with_words(shell, |words| {
            // (Why the last mark asked of the entry wasn't made: the strip's refusal, said before what is against it.)
            let problem = notice.unwrap_or_default();
            let Some(chip) = chip else { return ChipDetail { problem: problem.into(), fault: fault.into(), ..Default::default() } };
            let record = content.chip(chip.id);
            let held = folder.chips().filter(|c| *c == chip).count() as i32;
            let dark = record.flags.has(nettai_battle::content::ChipFlags::DARK);
            let mut facts = vec![say.invoke_class(kind(record.class), dark), say.invoke_megabytes(record.mb as i32)];
            if record.damage > 0 {
                facts.push(say.invoke_attack(record.damage as i32));
            }
            if held > 0 {
                facts.push(say.invoke_in_folder(held));
            }
            ChipDetail {
                there: true,
                art: kit.pictures.art(chip.id),
                name: words.names.chip(chip.id).into(),
                kind: kind(record.class),
                code: chip.code.letter().to_string().into(),
                description: description(words, chip.id).into(),
                element: kit.pictures.chip_element(chip.id),
                facts: ModelRc::new(VecModel::from(facts)),
                problem: problem.into(),
                fault: fault.into(),
            }
        });
        view.set_detail(detail);
    }

    /// The build's folder changed: the rules are asked again, and everything is shown anew.
    fn changed(&mut self, shell: &mut Shell, draft: &mut Draft) {
        self.ask(draft);
        self.show_folder(shell, draft);
        self.show_library(shell, draft);
        self.show_detail(shell, draft);
    }

    /// The cursor is on entry `i`: it is the one being filled.
    fn on_slot(&mut self, shell: &mut Shell, draft: &mut Draft, i: usize) {
        let moved = self.slot != i;
        self.slot = i;
        self.in_library = false;
        self.described = Some(Described::Slot(i));
        self.notice = None;
        if moved {
            // (A row's reason is asked of the folder without this entry.)
            let before = self.offered.take();
            self.ask(draft);
            if before != self.offered {
                self.show_library(shell, draft);
            }
        }
        let ui = shell.ui();
        let view = ui.global::<FolderView>();
        view.set_slot(i as i32);
        view.set_in_library(false);
        self.show_detail(shell, draft);
    }

    /// Put row `row`'s chip in the entry being filled, and go on to the next entry. A chip the rules keep out
    /// isn't put: its row shakes.
    fn put(&mut self, shell: &mut Shell, draft: &mut Draft, row: usize) {
        if !self.put_in(shell, draft, row) {
            return;
        }
        let next = (self.slot + 1).min(nettai_battle::custom::folder::FOLDER_SIZE - 1);
        self.slot = next;
        self.changed(shell, draft);
        self.leave_library(shell, next);
    }

    /// Row `row`'s chip dragged onto entry `to`: put in it, in place of what it held, as a click puts one in the
    /// marked entry, and the entry is then the one marked. A chip the rules keep out there isn't put (its row shakes,
    /// the strip says why), and the marked entry stays as it was.
    fn fill(&mut self, shell: &mut Shell, draft: &mut Draft, row: usize, to: usize) {
        let was = self.slot;
        // (A row's reason is asked of the folder without the entry it would go in.)
        self.slot = to;
        self.ask(draft);
        if !self.put_in(shell, draft, row) {
            self.slot = was;
            self.ask(draft);
            return;
        }
        self.changed(shell, draft);
        self.leave_library(shell, to);
    }

    /// Put row `row`'s chip in the entry being filled: whether it went in. A chip the rules keep out, or the build's
    /// ruleset, isn't put: its row shakes, and the strip says why.
    fn put_in(&mut self, shell: &mut Shell, draft: &mut Draft, row: usize) -> bool {
        let Some(&chip) = self.rows.get(row) else { return false };
        let ui = shell.ui();
        let view = ui.global::<FolderView>();
        let fact = draft.content().defs.fact_name(PlayerFact::Folder).unwrap_or_default();
        let banned = !draft.allows(fact, chip.id.0);
        if banned || self.offered.as_ref().is_none_or(|v| v.refusal(chip.id).is_some()) {
            shell.sound.play(Cue::Refused);
            self.refused += 1;
            view.set_refused(row as i32);
            view.set_refused_seq(self.refused);
            let wrong = self.offered.as_ref().and_then(|v| v.refusal(chip.id));
            self.notice = match banned {
                true => Some(ui.global::<nettai_ui::BuildWords>().invoke_not_allowed_here().to_string()),
                false => wrong.map(|wrong| draft.with_words(shell, |words| words.refusal(chip.id, wrong))),
            };
            self.show_detail(shell, draft);
            return false;
        }
        let slot = self.slot;
        let content = draft.content().clone();
        let before = draft.side().folder(&content);
        // (The entry's own chip in its own code put again changes nothing, and still goes on.)
        let mut after = edit::folder_after(&content, &before, FolderEdit::Put(slot, chip.id, chip.code)).unwrap_or(before);
        // (An entry that was the Regular chip or a tag chip keeps its mark only if the rules take the new chip
        // as one.)
        let marked = after.regular == Some(slot as u8) || after.tags.is_some_and(|(a, b)| a as usize == slot || b as usize == slot);
        if marked && !draft.checked.mark_problems(&after).is_empty() {
            if after.regular == Some(slot as u8) {
                after.regular = None;
            }
            if after.tags.is_some_and(|(a, b)| a as usize == slot || b as usize == slot) {
                after.tags = None;
            }
        }
        if after != before && !draft.edit(|content, side| side.set_folder(content, &after).is_ok()) {
            return false;
        }
        // The chip flies from its row to its entry, which flashes.
        let (icon, name) = draft.with_words(shell, |words| (words.kit.pictures.icon(chip.id), words.names.chip(chip.id)));
        shell.fly(("lib.rows", row as i32), ("slot", slot as i32), icon, &name);
        self.landed += 1;
        view.set_landed(slot as i32);
        view.set_landed_seq(self.landed);
        self.notice = None;
        true
    }

    /// The ghost of row `row`'s chip, dragged into the folder: as an entry would show it.
    fn show_ghost(&mut self, shell: &Shell, draft: &Draft, row: usize) {
        let Some(&chip) = self.rows.get(row) else { return };
        if self.ghost == Some(chip) {
            return;
        }
        self.ghost = Some(chip);
        let content = draft.content();
        let cell = draft.with_words(shell, |words| ChipCell {
            there: true,
            icon: words.kit.pictures.icon(chip.id),
            name: words.names.chip(chip.id).into(),
            kind: kind(content.chip(chip.id).class),
            element: words.kit.pictures.chip_element(chip.id),
            code: chip.code.letter().to_string().into(),
            version: words.kit.version_tag_of(words.names, Registry::Chip, &Item::of(chip.id.0)),
            ..Default::default()
        });
        shell.ui().global::<FolderView>().set_ghost(cell);
    }

    /// Back from the library to entry `to` (a phone's sheet shuts).
    fn leave_library(&mut self, shell: &mut Shell, to: usize) {
        let ui = shell.ui();
        let view = ui.global::<FolderView>();
        if view.get_sheet() {
            view.set_sheet(false);
            shell.nav.close("lib.");
        }
        self.in_library = false;
        self.described = Some(Described::Slot(to));
        view.set_in_library(false);
        shell.nav.focus("slot", to as i32);
    }

    /// Make entry `i` the Regular chip, or no longer. The rules may refuse it (its MB against the Regular
    /// memory): the strip says why.
    fn regular(&mut self, shell: &mut Shell, draft: &mut Draft, i: usize) {
        let content = draft.content().clone();
        let folder = draft.side().folder(&content);
        let Some(after) = edit::folder_after(&content, &folder, FolderEdit::Regular(i)) else { return shell.sound.play(Cue::Refused) };
        self.mark(shell, draft, after, |w| matches!(w, Wrong::RegularMemory { .. } | Wrong::RegularEntry | Wrong::TagRegular));
    }

    /// Tag entry `i`: the first of a pair, or the second (the two are then the folder's tag chips); a tagged
    /// one is untagged.
    fn tag(&mut self, shell: &mut Shell, draft: &mut Draft, i: usize) {
        let content = draft.content().clone();
        let folder = draft.side().folder(&content);
        if content.defs.fact_field(PlayerFact::TagChips).is_none() || !folder.has(i as u8) {
            return shell.sound.play(Cue::Refused);
        }
        if self.tagging == Some(i) {
            self.tagging = None;
            return self.changed(shell, draft);
        }
        let mut after = folder;
        if folder.tags.is_some_and(|(a, b)| a as usize == i || b as usize == i) {
            after.tags = None;
        } else if let Some(first) = self.tagging.take() {
            after.tags = Some((first.min(i) as u8, first.max(i) as u8));
        } else {
            self.tagging = Some(i);
            shell.sound.play(Cue::Pick);
            return self.changed(shell, draft);
        }
        self.mark(shell, draft, after, |w| matches!(w, Wrong::TagMemory(_) | Wrong::TagEntries | Wrong::TagRegular | Wrong::NoTags));
    }

    /// Make the folder `after` (a mark made or taken off), unless the rules have something of `kinds` against
    /// its marks: then nothing changes, and the strip says why.
    fn mark(&mut self, shell: &mut Shell, draft: &mut Draft, after: Chips, kinds: impl Fn(&Wrong) -> bool) {
        let against = draft.checked.mark_problems(&after).into_iter().find(|p| kinds(&p.what));
        match against {
            Some(problem) => {
                self.notice = Some(draft.with_words(shell, |words| words.problem(&problem)));
                shell.sound.play(Cue::Refused);
                self.show_folder(shell, draft);
                self.show_detail(shell, draft);
            }
            None => {
                self.notice = None;
                if draft.edit(|content, side| side.set_folder(content, &after).is_ok()) {
                    shell.sound.play(Cue::Pick);
                }
                self.changed(shell, draft);
            }
        }
    }

    /// Take entry `i`'s chip out.
    fn remove(&mut self, shell: &mut Shell, draft: &mut Draft, i: usize) {
        if self.tagging == Some(i) {
            self.tagging = None;
        }
        if draft.edit(|content, side| edit::folder(content, side, FolderEdit::Clear(i))) {
            shell.sound.play(Cue::Back);
            self.changed(shell, draft);
        }
    }

    /// CLEAR, answered: every entry emptied and its marks with it, as one change (`Folder::EMPTY`). The cursor goes to
    /// the entry being filled.
    fn clear(&mut self, shell: &mut Shell, draft: &mut Draft) {
        self.tagging = None;
        self.notice = None;
        if draft.edit(|content, side| side.folder(content) != Chips::EMPTY && side.set_folder(content, &Chips::EMPTY).is_ok()) {
            shell.sound.play(Cue::Back);
        }
        self.changed(shell, draft);
        shell.nav.focus("slot", self.slot as i32);
    }

    /// Where entry `from` dragged to `at` (the window's point) would land: the entry under the pointer; none off the
    /// folder.
    fn drop_at(shell: &Shell, at: i32) -> Option<usize> {
        let point = editor::pointer_at(at);
        let size = nettai_battle::custom::folder::FOLDER_SIZE;
        if shell.scale.phone { editor::list_row(shell, "slot", point, size) } else { editor::nearest(shell, "slot", 0..size, point, None) }
    }

    /// Move entry `from` to `to`, those between shifting toward where it was; its marks go with it (and the entry
    /// tagged first, waiting for its pair). It lands with a flash, and is the one marked.
    fn reorder(&mut self, shell: &mut Shell, draft: &mut Draft, from: usize, to: usize) {
        let content = draft.content().clone();
        let before = draft.side().folder(&content);
        let Some(after) = edit::folder_after(&content, &before, FolderEdit::Move(from, to)) else { return };
        if !draft.edit(|content, side| side.set_folder(content, &after).is_ok()) {
            return shell.sound.play(Cue::Refused);
        }
        self.tagging = self.tagging.map(|t| edit::moved(t, from, to));
        shell.sound.play(Cue::Pick);
        self.landed += 1;
        let ui = shell.ui();
        let view = ui.global::<FolderView>();
        view.set_landed(to as i32);
        view.set_landed_seq(self.landed);
        self.slot = to;
        self.described = Some(Described::Slot(to));
        self.notice = None;
        self.changed(shell, draft);
        shell.nav.focus("slot", to as i32);
    }

    /// The entry the cursor is on, if it is on one.
    fn focused_slot(shell: &Shell) -> Option<usize> {
        shell.nav.focused().filter(|k| k.id == "slot").map(|k| k.index.max(0) as usize)
    }

    fn sort_by(&mut self, shell: &mut Shell, draft: &mut Draft, sort: usize) {
        self.sort = sort.min(SORTS.len() - 1);
        self.list(shell, draft);
        self.show_library(shell, draft);
    }
}

impl Tab for Folder {
    fn id(&self) -> EditorTab {
        EditorTab::Folder
    }

    /// The folder, its Regular chip and its tag chips.
    fn fields(&self, content: &Content) -> Vec<String> {
        [PlayerFact::Folder, PlayerFact::RegularChip, PlayerFact::TagChips].into_iter().filter_map(|role| content.defs.fact_name(role).map(str::to_string)).collect()
    }

    fn main(&self) -> &'static str {
        "slot"
    }

    fn show(&mut self, shell: &mut Shell, draft: &mut Draft) {
        self.slot = self.slot.min(nettai_battle::custom::folder::FOLDER_SIZE - 1);
        if self.described.is_none() {
            self.described = Some(Described::Slot(self.slot));
        }
        self.ask(draft);
        self.list(shell, draft);
        self.show_folder(shell, draft);
        self.show_library(shell, draft);
        self.show_detail(shell, draft);
    }

    fn hide(&mut self, shell: &mut Shell, _draft: &mut Draft) {
        // (A tag waiting for its pair is dropped; a phone's sheet shuts.)
        self.tagging = None;
        self.notice = None;
        self.hovered = None;
        let ui = shell.ui();
        if ui.global::<FolderView>().get_sheet() {
            ui.global::<FolderView>().set_sheet(false);
            shell.nav.close("lib.");
        }
    }

    fn activate(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32) {
        let ui = shell.ui();
        let view = ui.global::<FolderView>();
        match id {
            // An entry: on to the library, for a chip to put in it (on a phone, its sheet).
            "slot" => {
                self.on_slot(shell, draft, index.max(0) as usize);
                self.in_library = true;
                view.set_in_library(true);
                // (Its row is the one the cursor was last on there; with no rows, the search that left none.)
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
                self.list(shell, draft);
                self.show_library(shell, draft);
            }
            // A phone's sheet: what a slot's keys do, and shutting it.
            "lib.regular" => self.regular(shell, draft, self.slot),
            "lib.tag" => self.tag(shell, draft, self.slot),
            "lib.remove" => {
                self.remove(shell, draft, self.slot);
                self.leave_library(shell, self.slot);
            }
            "lib.close" => self.leave_library(shell, self.slot),
            "clear" => {
                let empty = draft.side().folder(draft.content()).chips().next().is_none();
                editor::ask_clear(shell, view.get_clear_question(), empty);
            }
            _ => {}
        }
    }

    fn step(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, _index: i32, by: i32) {
        if id == "lib.sort" {
            self.sort_by(shell, draft, (self.sort as i32 + by).rem_euclid(SORTS.len() as i32) as usize);
        }
    }

    fn choose(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, choice: i32) {
        match id {
            "lib.sort" => self.sort_by(shell, draft, choice.max(0) as usize),
            // An entry dragged: where it would land; let go there, it moves (over the library, its chip is taken
            // out; elsewhere off the folder, it goes back).
            "drag-over" if editor::over_library(shell, choice) => editor::show_remove(shell),
            "drag-over" => editor::show_drop(shell, Folder::drop_at(shell, choice), false),
            "drag-drop" => {
                editor::show_drop(shell, None, false);
                if index >= 0 && editor::over_library(shell, choice) {
                    self.remove(shell, draft, index as usize);
                } else if let Some(to) = Folder::drop_at(shell, choice).filter(|&to| to as i32 != index && index >= 0) {
                    self.reorder(shell, draft, index as usize, to);
                }
            }
            // A library's row dragged: the entry it would go in, in place of its chip (none shift); let go there, it
            // goes in, as a click puts it in the marked one; anywhere else, nothing.
            "drag-in-over" if index >= 0 => {
                self.show_ghost(shell, draft, index as usize);
                editor::show_landing(shell, "slot", Folder::drop_at(shell, choice), true);
            }
            "drag-in-drop" => {
                editor::show_drop(shell, None, false);
                self.ghost = None;
                if let Some(to) = Folder::drop_at(shell, choice).filter(|_| index >= 0) {
                    self.fill(shell, draft, index as usize, to);
                }
            }
            _ => {}
        }
    }

    fn edit(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, _index: i32, text: &str, _done: bool) {
        if id != "lib.search" {
            return;
        }
        self.search = text.chars().filter(|c| !c.is_control()).take(SEARCH_LENGTH).collect();
        self.list(shell, draft);
        self.show_library(shell, draft);
        // (What the search finds is shown from its top.)
        let ui = shell.ui();
        shell.nav.list_to_top(&ui, "lib.rows");
    }

    /// The pointer on an entry or a row of the library: the strip describes its chip.
    fn hovered(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, on: bool) {
        let what = match id {
            "slot" => Described::Slot(index.max(0) as usize),
            "lib.rows" => Described::Row(index.max(0) as usize),
            _ => return,
        };
        if editor::hover(&mut self.hovered, what, on) {
            self.show_detail(shell, draft);
        }
    }

    /// (The strip follows the focus, the last thing moved, until the pointer goes onto another row.)
    fn focused(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32) {
        if self.hovered.take().is_some() && !matches!(id, "slot" | "lib.rows") {
            self.show_detail(shell, draft);
        }
        let ui = shell.ui();
        let view = ui.global::<FolderView>();
        match id {
            "slot" => self.on_slot(shell, draft, index.max(0) as usize),
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

    /// R and T on an entry; X, CLEAR.
    fn key(&mut self, shell: &mut Shell, draft: &mut Draft, key: &KeyPress) -> bool {
        if editor::clear_key(shell, key) {
            self.activate(shell, draft, "clear", 0);
            return true;
        }
        if let Some(slot) = Folder::focused_slot(shell) {
            match key.code {
                KeyCode::KeyR if !key.repeat => self.regular(shell, draft, slot),
                KeyCode::KeyT if !key.repeat => self.tag(shell, draft, slot),
                KeyCode::KeyR | KeyCode::KeyT => {}
                _ => return false,
            }
            return true;
        }
        false
    }

    /// The pad's Select tags.
    fn button(&mut self, shell: &mut Shell, draft: &mut Draft, button: nettai_gamepad::Button) -> bool {
        match (button, Folder::focused_slot(shell)) {
            (nettai_gamepad::Button::Back, Some(slot)) => self.tag(shell, draft, slot),
            _ => return false,
        }
        true
    }

    /// Delete takes the entry's chip out; the pad's north button makes it the Regular chip.
    fn action(&mut self, shell: &mut Shell, draft: &mut Draft, action: Action) -> bool {
        let Some(slot) = Folder::focused_slot(shell) else { return false };
        match action {
            Action::Delete => self.remove(shell, draft, slot),
            Action::Alternate => self.regular(shell, draft, slot),
            _ => return false,
        }
        true
    }

    /// Esc in the library goes back to the entry being filled.
    fn back(&mut self, shell: &mut Shell, _draft: &mut Draft) -> bool {
        if shell.nav.focused().is_some_and(|k| k.id.starts_with("lib.")) || shell.ui().global::<FolderView>().get_sheet() {
            self.leave_library(shell, self.slot);
            return true;
        }
        false
    }

    fn answered(&mut self, shell: &mut Shell, draft: &mut Draft, tag: &str, choice: usize) {
        if editor::cleared(tag, choice) {
            self.clear(shell, draft);
        }
    }
}
