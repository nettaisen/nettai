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
//!
//! **The mouse:** a click on one of the build's cards marks it (the focus stays on it); a right click takes it out, as
//! Del does (a right click on a library card that is in, too). A card dragged to another place in the list moves
//! there, those between shifting toward where it was (the keys' edit, `ListEdit::Move`); dragged onto the library, it
//! is taken out. A library's card dragged onto the list goes in at that place, those from there on shifting on
//! (`ListEdit::Insert`; one that is in already moves there), refused as a click's is. In a game whose cards go in
//! slots each card's slot is its own: a slot's card dragged goes back to its slot anywhere but onto the library, which
//! empties the slot, and a library's card let go anywhere over the slots goes in its own, as a click puts it.
//!
//! **The strip** describes the card under the cursor, the build's or the library's: its number, name, MB or slot,
//! and its lines in its card screen's two groups. A row shows the card's number, name and MB alone (the user: "the
//! name isn't very visible on the patch cards view"); while the pointer is on a row, the strip describes that one, and
//! the focused one again once it has left (docs/design/app-ui.md §2.64).
//!
//! **A card with variants** (one each version's ROM gives effects of its own: Bass-Cross MegaMan, docs/design/versions.md
//! §2.1) is in the library once for each, as a NaviCust program is once for each of its colors: its name with the
//! variant's (the group's), its lines that variant's. Putting one in chooses it for the card; the card is in the
//! build once, so putting the other in while one is chooses the other in its place.

use crate::builds::cards::{self, Line};
use crate::builds::edit::{self, ListEdit};
use crate::builds::grid;
use crate::builds::overview;
use crate::input::{Action, KeyPress};
use crate::screens::builds::editor::{self, Draft, Tab};
use crate::shell::Shell;
use crate::sound::Cue;
use nettai_battle::Content;
use nettai_content_api::{EntryHandle, FieldType, Registry};
use nettai_match::Wrong;
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

/// A card as a row shows it: the card, and its variant where it has them (a version's id).
type Card = (EntryHandle, Option<String>);

/// Choose `variant` for card `h` in the side's list (none: it has none to choose): whether it took.
fn with_variant(content: &Content, side: &mut nettai_match::Side, h: EntryHandle, variant: Option<&str>) -> bool {
    let Some(v) = variant else { return true };
    let Some(Stated::List(items)) = side.facts.get(content, FIELD) else { return false };
    let Some(i) = items.iter().position(|x| x.defs().first() == Some(&h.0)) else { return false };
    edit::variant(content, side, FIELD, i, Some(v))
}

/// Card `card` as a ruleset holds it back.
fn item((h, variant): &Card) -> nettai_match::ruleset::Item {
    nettai_match::ruleset::Item { def: h.0, variant: variant.clone() }
}

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
    /// The library's rows, as sorted and searched: each card, in each of its variants.
    rows: Vec<Card>,
    /// The build's row the cursor was last on (in a game with slots, the slot).
    slot: usize,
    in_library: bool,
    row: usize,
    search: String,
    sort: usize,
    descending: bool,
    described: Option<Described>,
    /// The row the pointer is on, which the strip describes in its place (`editor::hover`).
    hovered: Option<Described>,
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

    /// What the build holds: its cards in order, each with its variant, and, while the list has room, an empty row
    /// after them; in a game with slots, what each slot holds.
    fn installed(draft: &Draft) -> Vec<Option<Card>> {
        let content = draft.content();
        if let Some(slots) = draft.kit.builds.slots.as_ref().filter(|s| s.field == FIELD) {
            return slots.held(content, draft.side()).into_iter().map(|h| h.map(|h| (h, None))).collect();
        }
        let variant = |i: &Stated| match i {
            Stated::Record(fields) => fields.iter().find(|(n, _)| n == nettai_match::ruleset::VARIANT).and_then(|(_, v)| if let Stated::Variant(v) = v { v.clone() } else { None }),
            _ => None,
        };
        let mut out: Vec<Option<Card>> = match draft.side().facts.get(content, FIELD) {
            Some(Stated::List(items)) => items.iter().filter_map(|i| i.defs().first().map(|&h| Some((EntryHandle(h), variant(i))))).collect(),
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
        let rows: Vec<Card> = draft.with_words(shell, |words| {
            let mut found: Vec<(String, u32, Card)> = self
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
                        // (Each of its variants, in their order; a card without, itself.)
                        let variants = nettai_match::ruleset::variants_of(content, Registry::Entry, h.0);
                        let variants: Vec<Option<String>> = if variants.is_empty() { vec![None] } else { variants.into_iter().map(Some).collect() };
                        variants.into_iter().map(move |v| (name.clone(), cost, (h, v))).collect::<Vec<_>>()
                    })
                })
                .flatten()
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

    /// A card's name as the strip shows it: with its variant's, the group's name, where it has one (a row shows the
    /// card's own and the version's mark).
    fn name_of(shell: &Shell, words: &crate::builds::words::Words, (h, variant): &Card) -> slint::SharedString {
        let name = words.names.entry(*h);
        match variant {
            Some(v) => shell.ui().global::<BuildWords>().invoke_card_variant(name.into(), words.kit.version_name(words.names, v).into()),
            None => name.into(),
        }
    }

    /// A card as a row shows it (none: an empty row or slot).
    fn row_of(&self, shell: &Shell, draft: &Draft, card: Option<&Card>, slot: Option<usize>, budget: Option<&Budget>) -> CardRow {
        let content = draft.content();
        let slots = draft.kit.builds.slots.as_ref().filter(|s| s.field == FIELD);
        let slot_name = |n: usize| slots.and_then(|s| s.name(n)).unwrap_or_default().to_string();
        let ui = shell.ui();
        let say = ui.global::<BuildWords>();
        let Some(card) = card else { return CardRow { slot: slot.map(slot_name).unwrap_or_default().into(), ..Default::default() } };
        let h = card.0;
        draft.with_words(shell, |words| {
            let own = slots.and_then(|s| s.own(content, h));
            CardRow {
                there: true,
                number: self.number(draft, h).into(),
                // (Its variant's version is its mark, after the card's own name: the strip says both in words.)
                name: words.names.entry(h).into(),
                version: words.kit.version_tag_of(words.names, Registry::Entry, &item(card)),
                megabytes: budget.map(|b| say.invoke_megabytes(b.cost_of(content, h) as i32)).unwrap_or_default(),
                slot: slot.or(own).map(slot_name).unwrap_or_default().into(),
                // (One the ruleset the player plays by doesn't allow, or a version's variant in a build that mixes
                // versions under one version per build: its name in the error red.)
                bad: !draft.allows_item(FIELD, &item(card)) || draft.mixing(Registry::Entry, &item(card)).is_some(),
                ..Default::default()
            }
        })
    }

    /// The build's problems with its card at `i` of `installed` (none: an empty slot), the ruleset's among them (a card
    /// or a variant it holds back, a version's variant in a build that mixes versions under one version per build): its
    /// own, by its place in the list's records, from 0; and the cards' MB past the list's, which every card counts in.
    fn problems<'d>(draft: &'d Draft, installed: &[Option<Card>], i: usize) -> Vec<&'d nettai_match::check::Problem> {
        let Some(Some(card)) = installed.get(i) else { return Vec::new() };
        let place = installed[..i].iter().flatten().count();
        let own = draft.checked.problems.iter().filter(|p| p.field.as_deref() == Some(FIELD) && (p.entry == Some(place) || matches!(p.what, Wrong::Budget(_))));
        own.chain(draft.mixing(Registry::Entry, &item(card))).collect()
    }

    /// The cards' problems about no card of the build's (a fact of them a build made by hand states wrong): every card's
    /// strip says them after its own (docs/design/app-ui.md §2.66).
    fn unplaced<'d>(draft: &'d Draft, installed: &[Option<Card>]) -> Vec<&'d nettai_match::check::Problem> {
        let cards = installed.iter().flatten().count();
        let placed = |p: &nettai_match::check::Problem| matches!(p.what, Wrong::Budget(_) | Wrong::Mixed { .. }) || p.entry.is_some_and(|e| e < cards);
        draft.pane_problems(&[FIELD.to_string()], false).into_iter().filter(|p| !placed(p)).collect()
    }

    /// The build's cards (or slots), each marked where the build's problems are about it, their counts (the MB past the
    /// list's in the error red) and in a phone's header what the game's rules have against them (one, or all), and what
    /// they add up to (docs/design/app-ui.md §2.66).
    fn show_installed(&self, shell: &Shell, draft: &mut Draft, budget: Option<&Budget>) {
        let installed = PatchCards::installed(draft);
        let slotted = PatchCards::slotted(draft);
        let rows: Vec<CardRow> = installed
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let mut row = self.row_of(shell, draft, c.as_ref(), slotted.then_some(i), budget);
                row.bad |= !PatchCards::problems(draft, &installed, i).is_empty();
                row
            })
            .collect();
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
        view.set_problems(draft.pane_line(shell, &[FIELD.to_string()], false).into());
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
                .chain(o.stats.iter().filter(|s| s.stat.added() != 0).map(|s| format!("{} {:+}", overview::stat_name(words, s), s.stat.added())))
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
        let held: Vec<Option<Card>> = PatchCards::installed(draft);
        let rows: Vec<CardRow> = self
            .rows
            .iter()
            .map(|card| {
                let h = card.0;
                let mut row = self.row_of(shell, draft, Some(card), None, budget);
                let is_in = held.contains(&Some(card.clone()));
                // (Its card in, of another variant: putting this one in chooses it, nothing refuses it.)
                let other = !is_in && held.iter().flatten().any(|c| c.0 == h);
                // (A card that is in says so, and isn't greyed: Enter takes it out again. In a game with slots a
                // card goes in its own slot whatever it holds: no card is refused.)
                // (One the ruleset the player plays by doesn't allow says so, in the error red, whether it is in or not.)
                if !draft.allows_item(FIELD, &item(card)) {
                    row.note = shell.ui().global::<BuildWords>().invoke_not_allowed_here();
                    row.refused = !is_in;
                    row.bad = true;
                } else if is_in {
                    // (Its note says no reason: the build's card a problem is about is marked in the build's list, its
                    // name in the error red, docs/design/app-ui.md §2.66.)
                    row.note = shell.ui().global::<BuildWords>().invoke_installed();
                    row.bad = false;
                } else if slots.is_some() || other {
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
        let card = self.rows.get(row)?.clone();
        PatchCards::installed(draft).iter().position(|c| *c == Some(card.clone())).map(|i| (row, i))
    }

    /// The strip: the card under the cursor, the build's or the library's; while the pointer is on a row, that one.
    fn show_detail(&self, shell: &Shell, draft: &Draft, budget: Option<&Budget>) {
        let content = draft.content();
        let installed = PatchCards::installed(draft);
        let slots = draft.kit.builds.slots.as_ref().filter(|s| s.field == FIELD);
        let ui = shell.ui();
        let view = ui.global::<PatchCardsView>();
        let say = ui.global::<BuildWords>();
        let hovered = self.hovered.filter(|h| match *h {
            Described::Installed(i) => i < installed.len(),
            Described::Row(r) => r < self.rows.len(),
        });
        // (Why the focused card wasn't put in is the focused card's.)
        let notice = if hovered.is_some_and(|h| Some(h) != self.described) { None } else { self.notice.clone() };
        let (card, at) = match hovered.or(self.described) {
            Some(Described::Installed(i)) => (installed.get(i).cloned().flatten(), Some(i)),
            Some(Described::Row(r)) => (self.rows.get(r).cloned(), None),
            None => (None, None),
        };
        // (What the build's problems have against its card described, the ruleset's among them, then what is against the
        // cards and about none of them: said where nothing was refused, docs/design/app-ui.md §2.66.)
        let fault = at
            .filter(|&i| installed.get(i).is_some_and(Option::is_some))
            .map(|i| draft.line(shell, PatchCards::problems(draft, &installed, i).into_iter().chain(PatchCards::unplaced(draft, &installed))))
            .unwrap_or_default();
        let detail = draft.with_words(shell, |words| {
            // Why a library's card can't go in, the strip's refusal, said before what is against it.
            let problem = match (&card, at) {
                (Some((h, _)), None) if slots.is_none() => budget.and_then(|b| b.refusal(content, draft.side(), *h)).filter(|r| matches!(r, EntryRefusal::Needs { .. })).map(|r| words.entry_refusal(r)),
                _ => None,
            };
            let problem = notice.or(problem).unwrap_or_default();
            let Some(card) = card else { return CardDetail { problem: problem.into(), fault: fault.into(), ..Default::default() } };
            let h = card.0;
            let named = cards::named_by(words.kit);
            let lines = cards::lines(words.names, h, card.1.as_deref(), &named).unwrap_or_default();
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
                let mut out: Vec<CardLine> = ls.iter().take(if ls.len() > GROUP_LINES { GROUP_LINES - 1 } else { GROUP_LINES }).map(|l| CardLine { text: l.text.as_str().into(), bug: l.bug }).collect();
                if ls.len() > GROUP_LINES {
                    out.push(CardLine { text: say.invoke_more((ls.len() - (GROUP_LINES - 1)) as i32), bug: false });
                }
                out
            };
            let own = slots.and_then(|s| s.own(content, h)).and_then(|n| slots?.name(n));
            let facts: Vec<String> = [budget.map(|b| say.invoke_megabytes(b.cost_of(content, h) as i32).to_string()), own.map(|n| view.invoke_slot_word(n.into()).to_string())].into_iter().flatten().collect();
            CardDetail {
                there: true,
                number: self.number(draft, h).into(),
                name: PatchCards::name_of(shell, words, &card),
                facts: facts.join(" · ").into(),
                problem: problem.into(),
                fault: fault.into(),
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
    /// already in is taken out again (it flies back to its row), and one in of another variant is this one now. A
    /// card the list can't take isn't put in: its row shakes, and the strip says why.
    fn put(&mut self, shell: &mut Shell, draft: &mut Draft, row: usize) {
        let Some(card) = self.rows.get(row).cloned() else { return };
        let h = card.0;
        let installed = PatchCards::installed(draft);
        if let Some(i) = installed.iter().position(|c| *c == Some(card.clone())) {
            let name = draft.with_words(shell, |w| w.names.entry(h));
            shell.fly(("card", i as i32), ("lib.rows", row as i32), slint::Image::default(), &name);
            return self.remove(shell, draft, i);
        }
        if let Some(i) = installed.iter().position(|c| c.as_ref().is_some_and(|c| c.0 == h)) {
            if !draft.allows_item(FIELD, &item(&card)) {
                self.refuse(shell, draft, row, &card);
                return;
            }
            if draft.edit(|content, side| edit::variant(content, side, FIELD, i, card.1.as_deref())) {
                self.land(shell, draft, row, h, i);
            }
            return;
        }
        if self.refuse(shell, draft, row, &card) {
            return;
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
            None => edit::list(content, side, FIELD, ListEdit::Add(h.0)) && with_variant(content, side, h, card.1.as_deref()),
        });
        if !changed {
            return shell.sound.play(Cue::Refused);
        }
        let landed = into.unwrap_or_else(|| PatchCards::installed(draft).iter().position(|c| c.as_ref().is_some_and(|c| c.0 == h)).unwrap_or(0));
        self.land(shell, draft, row, h, landed);
    }

    /// Whether the list can't take row `row`'s card `h` (its MB, or no room): if so its row shakes, and the strip says
    /// why. (In a game with slots a card goes in its own slot, whatever it holds: none is refused.)
    fn refuse(&mut self, shell: &mut Shell, draft: &mut Draft, row: usize, card: &Card) -> bool {
        let h = card.0;
        let budget = PatchCards::budget(draft);
        let refusal = (!PatchCards::slotted(draft)).then(|| budget.as_ref().and_then(|b| b.refusal(draft.content(), draft.side(), h))).flatten();
        let room = PatchCards::slotted(draft) || PatchCards::installed(draft).iter().flatten().count() < room_of(draft.content(), FIELD);
        // (A card the ruleset the player plays by doesn't allow doesn't go in either: the strip says so.)
        let allowed = draft.allows_item(FIELD, &item(card));
        if refusal.is_none() && room && allowed {
            return false;
        }
        let ui = shell.ui();
        let view = ui.global::<PatchCardsView>();
        self.refused += 1;
        view.set_refused(row as i32);
        view.set_refused_seq(self.refused);
        shell.sound.play(Cue::Refused);
        self.notice = match allowed {
            false => Some(shell.ui().global::<BuildWords>().invoke_not_allowed_here().to_string()),
            true => refusal.map(|r| draft.with_words(shell, |w| w.entry_refusal(r))),
        };
        self.show_detail(shell, draft, budget.as_ref());
        true
    }

    /// Row `row`'s card `h` went in at the build's row `landed`: it flies there from its row, and that row flashes and
    /// is the one marked.
    fn land(&mut self, shell: &mut Shell, draft: &mut Draft, row: usize, h: EntryHandle, landed: usize) {
        let name = draft.with_words(shell, |w| w.names.entry(h));
        shell.fly(("lib.rows", row as i32), ("card", landed as i32), slint::Image::default(), &name);
        self.landed += 1;
        let ui = shell.ui();
        let view = ui.global::<PatchCardsView>();
        view.set_landed(landed as i32);
        view.set_landed_seq(self.landed);
        shell.sound.play(Cue::Pick);
        self.slot = landed;
        self.notice = None;
        self.changed(shell, draft);
    }

    /// Row `row`'s card dragged onto the build's: in at place `at` of the list, those from there on shifting on (one
    /// that is in already moves there), refused as a click's is; in a game with slots, in its own slot, as a click puts
    /// it (one that is in already stays).
    fn insert(&mut self, shell: &mut Shell, draft: &mut Draft, row: usize, at: usize) {
        let Some(card) = self.rows.get(row).cloned() else { return };
        let h = card.0;
        let installed = PatchCards::installed(draft);
        let held = installed.iter().position(|c| c.as_ref().is_some_and(|c| c.0 == h));
        if PatchCards::slotted(draft) {
            if held.is_none() {
                self.put(shell, draft, row);
            }
            return;
        }
        let at = at.min(installed.iter().flatten().count());
        if let Some(i) = held {
            // (Of another variant: this one is chosen first.)
            if installed[i].as_ref() != Some(&card) {
                if !draft.allows_item(FIELD, &item(&card)) {
                    self.refuse(shell, draft, row, &card);
                    return;
                }
                draft.edit(|content, side| edit::variant(content, side, FIELD, i, card.1.as_deref()));
            }
            let to = if at > i { at - 1 } else { at };
            if to == i || self.move_card(shell, draft, i, to) {
                self.land(shell, draft, row, h, to);
            }
            return;
        }
        if self.refuse(shell, draft, row, &card) {
            return;
        }
        if !draft.edit(|content, side| edit::list(content, side, FIELD, ListEdit::Insert(at, h.0)) && with_variant(content, side, h, card.1.as_deref())) {
            return shell.sound.play(Cue::Refused);
        }
        self.land(shell, draft, row, h, at);
    }

    /// Take the build's card in row `i` out (in a game with slots, empty the slot).
    fn remove(&mut self, shell: &mut Shell, draft: &mut Draft, i: usize) {
        let installed = PatchCards::installed(draft);
        if installed.get(i).cloned().flatten().is_none() {
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
        let Some(to) = (if up { i.checked_sub(1) } else { Some(i + 1) }) else { return shell.sound.play(Cue::Refused) };
        if self.move_card(shell, draft, i, to) {
            shell.sound.play(Cue::Cursor);
        }
    }

    /// Move the build's card in row `i` to row `to`, those between shifting toward where it was: whether it moved
    /// (if not, the refusal's sound is played).
    fn move_card(&mut self, shell: &mut Shell, draft: &mut Draft, i: usize, to: usize) -> bool {
        let cards = PatchCards::installed(draft).iter().flatten().count();
        if PatchCards::slotted(draft) || i >= cards || to >= cards || !draft.edit(|content, side| edit::list(content, side, FIELD, ListEdit::Move(i, to))) {
            shell.sound.play(Cue::Refused);
            return false;
        }
        self.slot = to;
        self.described = Some(Described::Installed(to));
        self.notice = None;
        self.changed(shell, draft);
        if !shell.ui().global::<PatchCardsView>().get_sheet() {
            shell.nav.focus("card", to as i32);
        }
        true
    }

    /// Where card `from` dragged to `at` (the window's point) would land: the card under the pointer (past the last,
    /// the last); none off the list. In a game with slots, its own slot (it can be in no other) while over the list.
    fn drop_at(shell: &Shell, draft: &Draft, from: usize, at: i32) -> Option<usize> {
        let slotted = PatchCards::slotted(draft);
        let rows = if slotted { PatchCards::installed(draft).len() } else { PatchCards::installed(draft).iter().flatten().count() };
        editor::list_row(shell, "card", editor::pointer_at(at), rows).map(|to| if slotted { from } else { to })
    }

    /// Where library row `row`'s card dragged to `at` would go in: the place under the pointer (the empty row after
    /// the cards, at their end), and whether it takes the place of what is there (a game with slots: over the list
    /// anywhere, its own slot), else those from there on shift on; none off the list.
    fn landing(&self, shell: &Shell, draft: &Draft, row: usize, at: i32) -> Option<(usize, bool)> {
        let installed = PatchCards::installed(draft);
        let to = editor::list_row(shell, "card", editor::pointer_at(at), installed.len())?;
        let slots = draft.kit.builds.slots.as_ref().filter(|s| s.field == FIELD);
        match slots {
            Some(s) => s.own(draft.content(), self.rows.get(row)?.0).map(|own| (own, true)),
            None => Some((to.min(installed.iter().flatten().count()), false)),
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
        self.hovered = None;
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

    fn choose(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, choice: i32) {
        match id {
            "lib.sort" => {
                self.sort = (choice.max(0) as usize).min(SORTS.len() - 1);
                self.changed(shell, draft);
            }
            // A card dragged: where it would land; let go there, it moves (over the library, it is taken out;
            // elsewhere off the list, it goes back).
            "drag-over" if editor::over_library(shell, choice) => editor::show_remove(shell),
            "drag-over" => editor::show_drop(shell, PatchCards::drop_at(shell, draft, index.max(0) as usize, choice), false),
            "drag-drop" => {
                editor::show_drop(shell, None, false);
                if index >= 0 && editor::over_library(shell, choice) {
                    self.remove(shell, draft, index as usize);
                } else if let Some(to) = PatchCards::drop_at(shell, draft, index.max(0) as usize, choice).filter(|&to| to as i32 != index && index >= 0)
                    && self.move_card(shell, draft, index as usize, to)
                {
                    shell.sound.play(Cue::Pick);
                    self.landed += 1;
                    let ui = shell.ui();
                    let view = ui.global::<PatchCardsView>();
                    view.set_landed(to as i32);
                    view.set_landed_seq(self.landed);
                }
            }
            // A library's card dragged: where it would go in (a gap, those from there on shifting; a game with slots,
            // its own slot outlined); let go there, it goes in; anywhere else, nothing.
            "drag-in-over" if index >= 0 => {
                let landing = self.landing(shell, draft, index as usize, choice);
                editor::show_landing(shell, "card", landing.map(|l| l.0), landing.is_some_and(|l| l.1));
            }
            "drag-in-drop" => {
                editor::show_drop(shell, None, false);
                if let Some((at, _)) = self.landing(shell, draft, index.max(0) as usize, choice).filter(|_| index >= 0) {
                    self.insert(shell, draft, index as usize, at);
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
        self.changed(shell, draft);
        // (What the search finds is shown from its top.)
        let ui = shell.ui();
        shell.nav.list_to_top(&ui, "lib.rows");
    }

    /// The pointer on one of the build's cards or the library's: the strip describes it.
    fn hovered(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, on: bool) {
        let what = match id {
            "card" => Described::Installed(index.max(0) as usize),
            "lib.rows" => Described::Row(index.max(0) as usize),
            _ => return,
        };
        if editor::hover(&mut self.hovered, what, on) {
            let budget = PatchCards::budget(draft);
            self.show_detail(shell, draft, budget.as_ref());
        }
    }

    /// (The strip follows the focus, the last thing moved, until the pointer goes onto another row.)
    fn focused(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32) {
        if self.hovered.take().is_some() && !matches!(id, "card" | "lib.rows") {
            let budget = PatchCards::budget(draft);
            self.show_detail(shell, draft, budget.as_ref());
        }
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

    /// `[` and `]` on one of the build's cards move it.
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

    /// Every card of each game has its own number (the game's compat tables'), each number one card's; the library
    /// lists them in that order.
    #[test]
    fn every_card_has_its_own_number() {
        let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../content"));
        for (content, game, count) in [(nettai_match::testing::exe6_content(), "exe6", 117), (nettai_match::testing::exe5_content(), "exe5", 111), (nettai_match::testing::exe4_content(), "exe4", 133)] {
            let order = crate::builds::order::Order::load(dir, game);
            let collection = collection_of(&content, FIELD).unwrap();
            let mut cards: Vec<((), EntryHandle)> = edit::of_game(&content, game, Registry::Entry, Some(&collection)).into_iter().map(|h| ((), EntryHandle(h))).collect();
            order.entries(&content, &collection, &mut cards);
            let numbers: Vec<u8> = cards.iter().map(|(_, h)| crate::builds::import::card_number(game, content.defs.entry(*h).id()).unwrap_or_else(|| panic!("{game}: {} has no number", content.defs.entry(*h).id()))).collect();
            assert_eq!(numbers.len(), count, "{game}");
            assert!(numbers.windows(2).all(|w| w[0] <= w[1]), "{game}: the library's order is the numbers'");
            let mut distinct = numbers.clone();
            distinct.dedup();
            assert_eq!(numbers.len(), distinct.len(), "{game}: numbers twice");
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
            let written = cards.iter().filter(|&&h| !cards::lines(&names, EntryHandle(h), None, none).unwrap_or_default().is_empty()).count();
            assert!(written * 10 > cards.len() * 9, "{game}: {written} of {} cards write lines", cards.len());
            if game == "exe4" {
                let pair = nettai_match::ids::entry(&content, game, &collection, "074-triple-supporter-1-2").unwrap();
                let lines = cards::lines(&names, pair, None, none).unwrap();
                let texts: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
                assert_eq!(texts, ["With {pair.card} in 2", "Rush, Beat and Tango", "Clears Support Bug"]);
                assert_eq!(lines.iter().map(|l| l.bug).collect::<Vec<_>>(), [false, false, true]);
            }
        }
    }

    /// The Bass-Cross MegaMan card is one card of each game whose lines are the build's version's (the user: "it's
    /// actually just the same card, but with different effects depending on version of the game"): the effects of no
    /// version's, and those of the build's.
    #[test]
    fn bass_cross_megamans_lines_are_the_versions() {
        for (content, game, versions) in [
            (nettai_match::testing::exe6_content(), "exe6", [("gregar", ["HP+200", "NormBody", "MegFld-2", "SprArmr", "TriBustr", "B↓Sbustr"]), ("falzar", ["HP+20%", "NormBody", "MegFld-2", "FlotShoe", "TriBustr", "B↓HelzR"])]),
            (nettai_match::testing::exe5_content(), "exe5", [("protoman", ["HP+200", "NormBody", "MegFld-2", "SprArmr", "TriBustr", "B↓Sbustr"]), ("colonel", ["HP+20%", "NormBody", "MegFld-2", "FlotShoe", "TriBustr", "B↓HelzR"])]),
        ] {
            let collection = collection_of(&content, FIELD).unwrap();
            let card = nettai_match::ids::entry(&content, game, &collection, "bass-cross-megaman").unwrap_or_else(|| panic!("{game}: one Bass-Cross MegaMan"));
            let names = crate::games::Names { content: &content, strings: None };
            let none: cards::Named = &|_, _| None;
            for (version, want) in versions {
                let lines = cards::lines(&names, card, Some(version), none).unwrap();
                let texts: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
                assert_eq!(texts, want, "{game} {version}");
            }
        }
    }
}
