//! The Patch cards tab of a build's editor (docs/design/app-ui.md §6, "Patch cards"; §2 items 19 and 20).
//!
//! A card's number, name, MB, slot and lines are the game's data (`crate::builds::cards`: its effects' lines from
//! the strings, a bug's marked; its number its own, the game's compat tables' (`builds::import::card_number`); its
//! MB the field the rules' budget names). What the cards may be is the game's rules': the list's budget and why a card can't go in now
//! (nettai-match's `Overview::budget`, `Budget::refusal`), and in a game whose cards go in slots, each card's own
//! (`Builds::slots`: `Slots::own`, `held`, `put`, `clear`). What they add up to is the build's overview without its
//! NaviCust.
//!
//! **Enter on one of the build's cards goes to the library** (on a phone, its sheet); **Enter on a card of the
//! library puts it in**: at the end of the list, or in the slot it belongs in, in place of the card there, and the
//! cursor stays in the library; Enter (or Del, or a click) on a card of the library that is in takes it out again,
//! so a card is put in and taken out from where the cursor is. Del takes the build's card under the cursor out;
//! `[` and `]` (the pad's north button: up) move it in the list's order, which matters (a later card can undo an
//! earlier one's work).

use crate::builds::cards::{self, Line};
use crate::builds::edit::{self, ListEdit};
use crate::builds::grid;
use crate::builds::overview;
use crate::input::{Action, KeyPress};
use crate::screens::builds::editor::{Draft, Tab};
use crate::shell::Shell;
use crate::sound::Cue;
use nettai_battle::Content;
use nettai_content_api::{EntryHandle, FieldType, Registry};
use nettai_match::build::{Budget, Checked, EntryRefusal};
use nettai_match::facts::Stated;
use nettai_ui::{BuildWords, CardCounts, CardDetail, CardLine, CardRow, EditorTab, NavicustView, PatchCardsView};
use slint::winit_030::winit::keyboard::KeyCode;
use slint::{ComponentHandle, Model, ModelRc, VecModel};

/// The setup field of the cards installed.
pub const FIELD: &str = "patch_cards";

/// The most letters a search has.
const SEARCH_LENGTH: usize = 24;

/// The lines each of the strip's groups has room for.
const GROUP_LINES: usize = 4;

/// The sorts, in the order the library's select lists them (`PatchCardsView.sorts`): the number, the name, the MB
/// (in a game with slots, the slot).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Sort {
    Number,
    Name,
    Cost,
}
const SORTS: [Sort; 3] = [Sort::Number, Sort::Name, Sort::Cost];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Described {
    Installed(usize),
    Row(usize),
}

#[derive(Default)]
pub struct PatchCards {
    /// The collection the cards are of, and its cards in the library's order.
    collection: String,
    cards: Vec<EntryHandle>,
    /// The library's rows, as sorted and searched.
    rows: Vec<EntryHandle>,
    /// The build's row the cursor was last on (in a game with slots, the slot).
    slot: usize,
    in_library: bool,
    row: usize,
    search: String,
    sort: usize,
    descending: bool,
    described: Option<Described>,
    landed: i32,
    refused: i32,
    /// Why the last card wasn't put in, until the cursor moves.
    notice: Option<String>,
}

/// The collection of the definitions field `name`'s list holds (directly, or a field of its records).
fn collection_of(content: &Content, name: &str) -> Option<String> {
    let field = nettai_match::facts::field(content, name)?;
    let element = match field.ty {
        FieldType::List(e, _) | FieldType::Array(e, _) => e,
        t => t,
    };
    match element {
        FieldType::Ref(Registry::Entry, Some(c)) => Some(c.clone()),
        FieldType::Record(fields) => fields.fields().iter().find_map(|f| match &f.ty {
            FieldType::Ref(Registry::Entry, Some(c)) => Some(c.clone()),
            _ => None,
        }),
        _ => None,
    }
}

/// How many entries field `name`'s list holds at most.
fn room_of(content: &Content, name: &str) -> usize {
    match nettai_match::facts::field(content, name).map(|f| f.ty) {
        Some(FieldType::List(_, n)) => *n as usize,
        Some(FieldType::Array(_, n)) => *n as usize,
        _ => 0,
    }
}

/// A card's lines as a row shows them, each after its lead: a bar where its card screen's group changes, else a dot.
fn row_lines(lines: &[Line]) -> Vec<CardLine> {
    lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let lead = match i {
                0 => "",
                _ if l.group.is_some() && l.group != lines[i - 1].group => "  |  ",
                _ => " · ",
            };
            CardLine { text: l.text.as_str().into(), bug: l.bug, lead: lead.into() }
        })
        .collect()
}

impl PatchCards {
    /// The cards' collection and the library, read the first time.
    fn read(&mut self, draft: &Draft) {
        if !self.collection.is_empty() {
            return;
        }
        let content = draft.content();
        let Some(collection) = collection_of(content, FIELD) else { return };
        let mut cards: Vec<((), EntryHandle)> = edit::of_game(content, &draft.kit.game, Registry::Entry, Some(&collection)).into_iter().map(|h| ((), EntryHandle(h))).collect();
        draft.kit.order.entries(content, &collection, &mut cards);
        self.cards = cards.into_iter().map(|(_, h)| h).collect();
        self.collection = collection;
    }

    /// The game's cards go in slots (EXE4's).
    fn slotted(draft: &Draft) -> bool {
        draft.kit.builds.slots.as_ref().is_some_and(|s| s.field == FIELD)
    }

    /// What the build holds: its cards in order and, while the list has room, an empty row after them; in a game
    /// with slots, what each slot holds.
    fn installed(draft: &Draft) -> Vec<Option<EntryHandle>> {
        let content = draft.content();
        if let Some(slots) = draft.kit.builds.slots.as_ref().filter(|s| s.field == FIELD) {
            return slots.held(content, draft.side());
        }
        let mut out: Vec<Option<EntryHandle>> = match draft.side().facts.get(content, FIELD) {
            Some(Stated::List(items)) => items.iter().filter_map(|i| i.defs().first().map(|&h| Some(EntryHandle(h)))).collect(),
            _ => Vec::new(),
        };
        if out.len() < room_of(content, FIELD) {
            out.push(None);
        }
        out
    }

    /// The card's number as its game shows it (`074`).
    fn number(&self, draft: &Draft, h: EntryHandle) -> String {
        let id = draft.content().defs.entry(h).id();
        crate::builds::import::card_number(&draft.kit.game, id).map(|n| format!("{n:03}")).unwrap_or_default()
    }

    /// The library's rows, as sorted and searched.
    fn list(&mut self, shell: &Shell, draft: &Draft, budget: Option<&Budget>) {
        let content = draft.content();
        let slots = draft.kit.builds.slots.clone();
        let wanted = self.search.to_lowercase();
        let rows: Vec<EntryHandle> = draft.with_words(shell, |words| {
            let mut found: Vec<(String, u32, EntryHandle)> = self
                .cards
                .iter()
                .filter_map(|&h| {
                    let name = words.names.entry(h);
                    (wanted.is_empty() || name.to_lowercase().contains(&wanted)).then(|| {
                        let cost = match (&slots, budget) {
                            (Some(s), _) if s.field == FIELD => s.own(content, h).map_or(u32::MAX, |n| n as u32),
                            (_, Some(b)) => b.cost_of(content, h),
                            _ => 0,
                        };
                        (name, cost, h)
                    })
                })
                .collect();
            // (The library's order is the cards' numbers': a sort by name or cost keeps it among equals.)
            match SORTS[self.sort.min(SORTS.len() - 1)] {
                Sort::Number => {}
                Sort::Name => found.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase())),
                Sort::Cost => found.sort_by_key(|f| f.1),
            }
            if self.descending {
                found.reverse();
            }
            found.into_iter().map(|f| f.2).collect()
        });
        self.rows = rows;
    }

    /// A card as a row shows it (none: an empty row or slot).
    fn row_of(&self, shell: &Shell, draft: &Draft, card: Option<EntryHandle>, slot: Option<usize>, budget: Option<&Budget>) -> CardRow {
        let content = draft.content();
        let slots = draft.kit.builds.slots.as_ref().filter(|s| s.field == FIELD);
        let slot_name = |n: usize| slots.and_then(|s| s.name(n)).unwrap_or_default().to_string();
        let ui = shell.ui();
        let say = ui.global::<BuildWords>();
        let Some(h) = card else { return CardRow { slot: slot.map(slot_name).unwrap_or_default().into(), ..Default::default() } };
        draft.with_words(shell, |words| {
            let named = cards::named_by(words.kit);
            let lines = cards::lines(words.names, h, &named).unwrap_or_default();
            let own = slots.and_then(|s| s.own(content, h));
            CardRow {
                there: true,
                number: self.number(draft, h).into(),
                name: words.names.entry(h).into(),
                megabytes: budget.map(|b| say.invoke_megabytes(b.cost_of(content, h) as i32)).unwrap_or_default(),
                slot: slot.or(own).map(slot_name).unwrap_or_default().into(),
                lines: ModelRc::new(VecModel::from(row_lines(&lines))),
                ..Default::default()
            }
        })
    }

    /// The build's cards (or slots), their counts and what they add up to.
    fn show_installed(&self, shell: &Shell, draft: &mut Draft, budget: Option<&Budget>) {
        let installed = PatchCards::installed(draft);
        let slotted = PatchCards::slotted(draft);
        let rows: Vec<CardRow> = installed.iter().enumerate().map(|(i, &c)| self.row_of(shell, draft, c, slotted.then_some(i), budget)).collect();
        let ui = shell.ui();
        let view = ui.global::<PatchCardsView>();
        let cards = installed.iter().flatten().count() as i32;
        view.set_installed(ModelRc::new(VecModel::from(rows)));
        view.set_slot(self.slot.min(installed.len().saturating_sub(1)) as i32);
        view.set_counts(CardCounts {
            cards,
            megabytes: budget.map_or(0, |b| b.count.used as i32),
            limit: budget.map_or(0, |b| b.count.limit as i32),
            budget: budget.is_some(),
            slotted,
            filled: cards,
            slots: if slotted { installed.len() as i32 } else { 0 },
        });
        // What the cards add up to: the build without its NaviCust, overviewed.
        let content = draft.content().clone();
        let mut bare = draft.side().clone();
        let _ = bare.set_fact(&content, grid::PIECES_FIELD, &[]);
        let alone = Checked::of(&content, &draft.kit.game, &bare).overview(&content, &bare);
        let numbers_view = ui.global::<NavicustView>();
        let say = ui.global::<BuildWords>();
        let (numbers, abilities, bugs) = draft.with_words(shell, |words| {
            let Some(o) = &alone else { return (Vec::new(), Vec::new(), Vec::new()) };
            let stats = [o.hp, o.attack, o.rapid, o.charge, o.custom, o.mega, o.giga, o.regular_memory];
            let numbers: Vec<String> = stats
                .iter()
                .enumerate()
                .filter(|(_, s)| s.added() != 0)
                .map(|(i, s)| {
                    let n = s.added();
                    let by = if i == 7 { format!("{}{}", if n > 0 { "+" } else { "" }, say.invoke_megabytes(n)) } else { format!("{n:+}") };
                    numbers_view.invoke_stat(i as i32, by.into()).to_string()
                })
                .collect();
            let abilities: Vec<String> = o.abilities.iter().map(|&s| overview::source(words, s)).collect();
            let bugs: Vec<String> = o.bugs.iter().map(|b| overview::bug(words, b)).map(|(n, e)| if e.is_empty() { n } else { format!("{n} {e}") }).collect();
            (numbers, abilities, bugs)
        });
        view.set_together_numbers(numbers.join(" · ").into());
        view.set_together_abilities(abilities.join(" · ").into());
        view.set_together_bugs(bugs.join(" · ").into());
    }

    /// The library's pane: each card with whether it is in, or why it can't go in.
    fn show_library(&self, shell: &Shell, draft: &Draft, budget: Option<&Budget>) {
        let content = draft.content();
        let slots = draft.kit.builds.slots.as_ref().filter(|s| s.field == FIELD);
        let held: Vec<Option<EntryHandle>> = PatchCards::installed(draft);
        let rows: Vec<CardRow> = self
            .rows
            .iter()
            .map(|&h| {
                let mut row = self.row_of(shell, draft, Some(h), None, budget);
                // (A card that is in says so, and isn't greyed: Enter takes it out again. In a game with slots a
                // card goes in its own slot whatever it holds: no card is refused.)
                if held.contains(&Some(h)) {
                    row.note = shell.ui().global::<BuildWords>().invoke_installed();
                } else if slots.is_some() {
                } else if let Some(refusal) = budget.and_then(|b| b.refusal(content, draft.side(), h)) {
                    row.note = draft.with_words(shell, |w| w.entry_refusal(refusal)).into();
                    row.refused = true;
                    row.bad = matches!(refusal, EntryRefusal::Needs { .. });
                }
                row
            })
            .collect();
        let ui = shell.ui();
        let view = ui.global::<PatchCardsView>();
        view.set_rows(ModelRc::new(VecModel::from(rows)));
        view.set_search(self.search.as_str().into());
        view.set_sort(self.sort as i32);
        view.set_descending(self.descending);
        view.set_in_library(self.in_library);
        view.set_on_installed(self.on_installed(shell, draft).is_some());
    }

    /// The library's row under the cursor, if its card is in: its place among the build's (Confirm and Del take it
    /// out again).
    fn on_installed(&self, shell: &Shell, draft: &Draft) -> Option<(usize, usize)> {
        let row = shell.nav.focused().filter(|k| k.id == "lib.rows").map(|k| k.index.max(0) as usize)?;
        let h = *self.rows.get(row)?;
        PatchCards::installed(draft).iter().position(|c| *c == Some(h)).map(|i| (row, i))
    }

    /// The strip: the card under the cursor, the build's or the library's.
    fn show_detail(&self, shell: &Shell, draft: &Draft, budget: Option<&Budget>) {
        let content = draft.content();
        let installed = PatchCards::installed(draft);
        let slots = draft.kit.builds.slots.as_ref().filter(|s| s.field == FIELD);
        let ui = shell.ui();
        let view = ui.global::<PatchCardsView>();
        let say = ui.global::<BuildWords>();
        let (card, at) = match self.described {
            Some(Described::Installed(i)) => (installed.get(i).copied().flatten(), Some(i)),
            Some(Described::Row(r)) => (self.rows.get(r).copied(), None),
            None => (None, None),
        };
        let detail = draft.with_words(shell, |words| {
            // Why it can't go in (the library's), or what the rules have against it (the build's: its place in the
            // list's records, from 1, as the rules count).
            let problem = match (card, at) {
                (Some(h), None) if slots.is_none() => budget.and_then(|b| b.refusal(content, draft.side(), h)).filter(|r| matches!(r, EntryRefusal::Needs { .. })).map(|r| words.entry_refusal(r)),
                (Some(_), Some(i)) => {
                    let place = installed[..i].iter().flatten().count() + 1;
                    draft.checked.problems.iter().find(|p| p.field.as_deref() == Some(FIELD) && p.entry == Some(place)).map(|p| words.problem(p))
                }
                _ => None,
            };
            let problem = self.notice.clone().or(problem).unwrap_or_default();
            let Some(h) = card else { return CardDetail { problem: problem.into(), ..Default::default() } };
            let named = cards::named_by(words.kit);
            let lines = cards::lines(words.names, h, &named).unwrap_or_default();
            // The two groups: the card screen's, by the lines' groups in their order; a game whose cards have none,
            // what it does and the bugs it adds to or clears.
            let groups: Vec<Option<String>> = lines.iter().map(|l| l.group.clone()).fold(Vec::new(), |mut g, x| {
                if !g.contains(&x) {
                    g.push(x);
                }
                g
            });
            let (first, second, titles): (Vec<&Line>, Vec<&Line>, (String, String)) = if groups.iter().any(Option::is_some) {
                // (A title is in capitals, as a label is.)
                let title = |g: Option<&Option<String>>| g.and_then(|g| g.as_deref()).map(|g| words.names.text(cards::GROUPS, g).unwrap_or_else(|| g.to_string()).to_uppercase()).unwrap_or_default();
                let a = groups.first().cloned().flatten();
                (lines.iter().filter(|l| l.group == a).collect(), lines.iter().filter(|l| l.group != a).collect(), (title(groups.first()), title(groups.get(1))))
            } else {
                (lines.iter().filter(|l| !l.bug).collect(), lines.iter().filter(|l| l.bug).collect(), (view.get_effects_title().into(), view.get_bugs_title().into()))
            };
            let fit = |ls: Vec<&Line>| -> Vec<CardLine> {
                let mut out: Vec<CardLine> = ls.iter().take(if ls.len() > GROUP_LINES { GROUP_LINES - 1 } else { GROUP_LINES }).map(|l| CardLine { text: l.text.as_str().into(), bug: l.bug, lead: "".into() }).collect();
                if ls.len() > GROUP_LINES {
                    out.push(CardLine { text: say.invoke_more((ls.len() - (GROUP_LINES - 1)) as i32), bug: false, lead: "".into() });
                }
                out
            };
            let own = slots.and_then(|s| s.own(content, h)).and_then(|n| slots?.name(n));
            let facts: Vec<String> = [budget.map(|b| say.invoke_megabytes(b.cost_of(content, h) as i32).to_string()), own.map(|n| view.invoke_slot_word(n.into()).to_string())].into_iter().flatten().collect();
            CardDetail {
                there: true,
                number: self.number(draft, h).into(),
                name: words.names.entry(h).into(),
                facts: facts.join(" · ").into(),
                problem: problem.into(),
                first_title: titles.0.into(),
                first: ModelRc::new(VecModel::from(fit(first))),
                second_title: titles.1.into(),
                second: ModelRc::new(VecModel::from(fit(second))),
            }
        });
        view.set_detail(detail);
    }

    /// The list's budget, as the build stands.
    fn budget(draft: &mut Draft) -> Option<Budget> {
        let content = draft.content().clone();
        let side = draft.side().clone();
        draft.checked.overview(&content, &side).and_then(|o| o.budget(FIELD).cloned())
    }

    /// The build's cards changed (or the library's order, or the search): everything shown anew.
    fn changed(&mut self, shell: &Shell, draft: &mut Draft) {
        let budget = PatchCards::budget(draft);
        self.list(shell, draft, budget.as_ref());
        self.show_installed(shell, draft, budget.as_ref());
        self.show_library(shell, draft, budget.as_ref());
        self.show_detail(shell, draft, budget.as_ref());
    }

    /// Put row `row`'s card in: at the end of the list, or in its own slot in place of what that slot held; one
    /// already in is taken out again (it flies back to its row). A card the list can't take isn't put in: its row
    /// shakes, and the strip says why.
    fn put(&mut self, shell: &mut Shell, draft: &mut Draft, row: usize) {
        let Some(&h) = self.rows.get(row) else { return };
        if let Some(i) = PatchCards::installed(draft).iter().position(|c| *c == Some(h)) {
            let name = draft.with_words(shell, |w| w.names.entry(h));
            shell.fly(("card", i as i32), ("lib.rows", row as i32), slint::Image::default(), &name);
            return self.remove(shell, draft, i);
        }
        let budget = PatchCards::budget(draft);
        let ui = shell.ui();
        let view = ui.global::<PatchCardsView>();
        let refusal = (!PatchCards::slotted(draft)).then(|| budget.as_ref().and_then(|b| b.refusal(draft.content(), draft.side(), h))).flatten();
        let room = PatchCards::slotted(draft) || PatchCards::installed(draft).iter().flatten().count() < room_of(draft.content(), FIELD);
        if refusal.is_some() || !room {
            self.refused += 1;
            view.set_refused(row as i32);
            view.set_refused_seq(self.refused);
            shell.sound.play(Cue::Refused);
            self.notice = refusal.map(|r| draft.with_words(shell, |w| w.entry_refusal(r)));
            return self.show_detail(shell, draft, budget.as_ref());
        }
        let slots = draft.kit.builds.slots.clone().filter(|s| s.field == FIELD);
        let mut into = None;
        let changed = draft.edit(|content, side| match &slots {
            Some(s) => match s.put(content, side, h) {
                Ok(slot) => {
                    into = Some(slot);
                    true
                }
                Err(why) => {
                    crate::errors::log("putting a patch card in its slot", &why);
                    false
                }
            },
            None => edit::list(content, side, FIELD, ListEdit::Add(h.0)),
        });
        if !changed {
            return shell.sound.play(Cue::Refused);
        }
        let landed = into.unwrap_or_else(|| PatchCards::installed(draft).iter().position(|c| *c == Some(h)).unwrap_or(0));
        let name = draft.with_words(shell, |w| w.names.entry(h));
        shell.fly(("lib.rows", row as i32), ("card", landed as i32), slint::Image::default(), &name);
        self.landed += 1;
        view.set_landed(landed as i32);
        view.set_landed_seq(self.landed);
        shell.sound.play(Cue::Pick);
        self.slot = landed;
        self.notice = None;
        self.changed(shell, draft);
    }

    /// Take the build's card in row `i` out (in a game with slots, empty the slot).
    fn remove(&mut self, shell: &mut Shell, draft: &mut Draft, i: usize) {
        let installed = PatchCards::installed(draft);
        if installed.get(i).copied().flatten().is_none() {
            return shell.sound.play(Cue::Refused);
        }
        let slots = draft.kit.builds.slots.clone().filter(|s| s.field == FIELD);
        let changed = draft.edit(|content, side| match &slots {
            Some(s) => s.clear(content, side, i).is_ok(),
            None => edit::list(content, side, FIELD, ListEdit::Remove(i)),
        });
        if changed {
            shell.sound.play(Cue::Back);
            self.changed(shell, draft);
        }
    }

    /// Move the build's card in row `i` up or down the list.
    fn shift(&mut self, shell: &mut Shell, draft: &mut Draft, i: usize, up: bool) {
        if PatchCards::slotted(draft) || PatchCards::installed(draft).get(i).copied().flatten().is_none() {
            return shell.sound.play(Cue::Refused);
        }
        if !draft.edit(|content, side| edit::list(content, side, FIELD, ListEdit::Move(i, up))) {
            return shell.sound.play(Cue::Refused);
        }
        let to = if up { i - 1 } else { i + 1 };
        self.slot = to;
        self.described = Some(Described::Installed(to));
        shell.sound.play(Cue::Cursor);
        self.changed(shell, draft);
        if !shell.ui().global::<PatchCardsView>().get_sheet() {
            shell.nav.focus("card", to as i32);
        }
    }

    /// The cursor is on the build's row `i`.
    fn on_card(&mut self, shell: &mut Shell, draft: &mut Draft, i: usize) {
        self.slot = i;
        self.in_library = false;
        self.described = Some(Described::Installed(i));
        self.notice = None;
        let ui = shell.ui();
        let view = ui.global::<PatchCardsView>();
        view.set_slot(i as i32);
        view.set_in_library(false);
        let budget = PatchCards::budget(draft);
        self.show_detail(shell, draft, budget.as_ref());
    }

    /// Back from the library to the build's cards (a phone's sheet shuts).
    fn leave_library(&mut self, shell: &mut Shell) {
        let ui = shell.ui();
        let view = ui.global::<PatchCardsView>();
        if view.get_sheet() {
            view.set_sheet(false);
            shell.nav.close("lib.");
        }
        self.in_library = false;
        self.described = Some(Described::Installed(self.slot));
        view.set_in_library(false);
        shell.nav.focus("card", self.slot as i32);
    }

    /// The build's row the cursor is on, if it is on one.
    fn focused_card(shell: &Shell) -> Option<usize> {
        shell.nav.focused().filter(|k| k.id == "card").map(|k| k.index.max(0) as usize)
    }
}

impl Tab for PatchCards {
    fn id(&self) -> EditorTab {
        EditorTab::PatchCards
    }

    fn fields(&self, _content: &Content) -> Vec<String> {
        vec![FIELD.into()]
    }

    fn own_navi_only(&self) -> bool {
        true
    }

    fn main(&self) -> &'static str {
        "card"
    }

    fn show(&mut self, shell: &mut Shell, draft: &mut Draft) {
        self.read(draft);
        if self.described.is_none() {
            self.described = Some(Described::Installed(self.slot));
        }
        self.changed(shell, draft);
    }

    fn hide(&mut self, shell: &mut Shell, _draft: &mut Draft) {
        self.notice = None;
        let ui = shell.ui();
        if ui.global::<PatchCardsView>().get_sheet() {
            ui.global::<PatchCardsView>().set_sheet(false);
            shell.nav.close("lib.");
        }
    }

    fn activate(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32) {
        let ui = shell.ui();
        let view = ui.global::<PatchCardsView>();
        match id {
            // One of the build's cards: on to the library (on a phone, its sheet).
            "card" if index >= 0 => {
                self.on_card(shell, draft, index as usize);
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
                self.remove(shell, draft, self.slot);
                self.leave_library(shell);
            }
            "lib.up" => self.shift(shell, draft, self.slot, true),
            "lib.down" => self.shift(shell, draft, self.slot, false),
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
        if id != "lib.search" {
            return;
        }
        self.search = text.chars().filter(|c| !c.is_control()).take(SEARCH_LENGTH).collect();
        self.changed(shell, draft);
        // (What the search finds is shown from its top.)
        let ui = shell.ui();
        shell.nav.list_to_top(&ui, "lib.rows");
    }

    fn focused(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32) {
        let ui = shell.ui();
        let view = ui.global::<PatchCardsView>();
        match id {
            "card" => self.on_card(shell, draft, index.max(0) as usize),
            "lib.rows" => {
                self.in_library = true;
                self.row = index.max(0) as usize;
                self.described = Some(Described::Row(self.row));
                self.notice = None;
                view.set_in_library(true);
                view.set_on_installed(self.on_installed(shell, draft).is_some());
                let budget = PatchCards::budget(draft);
                self.show_detail(shell, draft, budget.as_ref());
            }
            id if id.starts_with("lib.") => {
                view.set_on_installed(false);
                self.in_library = true;
                view.set_in_library(true);
            }
            _ => {}
        }
    }

    /// `[` and `]` on one of the build's cards move it; in the library, a letter typed starts a search.
    fn key(&mut self, shell: &mut Shell, draft: &mut Draft, key: &KeyPress) -> bool {
        if let Some(i) = PatchCards::focused_card(shell) {
            match key.code {
                KeyCode::BracketLeft if !key.repeat => self.shift(shell, draft, i, true),
                KeyCode::BracketRight if !key.repeat => self.shift(shell, draft, i, false),
                KeyCode::BracketLeft | KeyCode::BracketRight => {}
                _ => return false,
            }
            return true;
        }
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

    /// Delete takes the card out; the pad's north button moves it up.
    fn action(&mut self, shell: &mut Shell, draft: &mut Draft, action: Action) -> bool {
        // (In the library, Del on a card that is in takes it out, as Confirm does.)
        if let (Action::Delete, Some((row, _))) = (action, self.on_installed(shell, draft)) {
            self.put(shell, draft, row);
            return true;
        }
        let Some(i) = PatchCards::focused_card(shell) else { return false };
        match action {
            Action::Delete => self.remove(shell, draft, i),
            Action::Alternate => self.shift(shell, draft, i, true),
            _ => return false,
        }
        true
    }

    /// Esc in the library goes back to the build's cards.
    fn back(&mut self, shell: &mut Shell, _draft: &mut Draft) -> bool {
        if shell.nav.focused().is_some_and(|k| k.id.starts_with("lib.")) || shell.ui().global::<PatchCardsView>().get_sheet() {
            self.leave_library(shell);
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every card of each game has its own number (the game's compat tables'), each number one card's but EXE5's 111,
    /// each team's own; the library lists them in that order (its place is the number but for the second 111, which
    /// is why the number isn't taken from it).
    #[test]
    fn every_card_has_its_own_number() {
        let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../content"));
        for (content, game, count) in [(nettai_match::testing::exe6_content(), "exe6", 117), (nettai_match::testing::exe5_content(), "exe5", 112), (nettai_match::testing::exe4_content(), "exe4", 133)] {
            let order = crate::builds::order::Order::load(dir, game);
            let collection = collection_of(&content, FIELD).unwrap();
            let mut cards: Vec<((), EntryHandle)> = edit::of_game(&content, game, Registry::Entry, Some(&collection)).into_iter().map(|h| ((), EntryHandle(h))).collect();
            order.entries(&content, &collection, &mut cards);
            let numbers: Vec<u8> = cards.iter().map(|(_, h)| crate::builds::import::card_number(game, content.defs.entry(*h).id()).unwrap_or_else(|| panic!("{game}: {} has no number", content.defs.entry(*h).id()))).collect();
            assert_eq!(numbers.len(), count, "{game}");
            assert!(numbers.windows(2).all(|w| w[0] <= w[1]), "{game}: the library's order is the numbers'");
            let mut distinct = numbers.clone();
            distinct.dedup();
            assert_eq!(numbers.len() - distinct.len(), usize::from(game == "exe5"), "{game}: numbers twice");
        }
    }

    /// Each game's cards are a collection the tab finds by the field's type, and each card of it writes lines (an
    /// EXE4 card's from the content's own table, a two-part card's naming its other half), a bug's marked.
    #[test]
    fn each_games_cards_have_lines() {
        for (content, game) in [(nettai_match::testing::exe6_content(), "exe6"), (nettai_match::testing::exe5_content(), "exe5"), (nettai_match::testing::exe4_content(), "exe4")] {
            let collection = collection_of(&content, FIELD).unwrap_or_else(|| panic!("{game}: no cards"));
            let names = crate::games::Names { content: &content, strings: None };
            let cards = edit::of_game(&content, game, Registry::Entry, Some(&collection));
            assert!(cards.len() > 100, "{game}: {} cards", cards.len());
            let none: cards::Named = &|_, _| None;
            // (A card whose every effect is outside a battle (EXE4's PET menu colors) writes none.)
            let written = cards.iter().filter(|&&h| !cards::lines(&names, EntryHandle(h), none).unwrap_or_default().is_empty()).count();
            assert!(written * 10 > cards.len() * 9, "{game}: {written} of {} cards write lines", cards.len());
            if game == "exe4" {
                let pair = nettai_match::ids::entry(&content, game, &collection, "074-triple-supporter-1-2").unwrap();
                let lines = cards::lines(&names, pair, none).unwrap();
                let texts: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
                assert_eq!(texts, ["With {pair.card} in 2", "Rush, Beat and Tango", "Clears Support Bug"]);
                assert_eq!(lines.iter().map(|l| l.bug).collect::<Vec<_>>(), [false, false, true]);
            }
        }
    }
}
