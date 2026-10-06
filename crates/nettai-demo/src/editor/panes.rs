//! The panes of a side's setup the editor lays out (`crate::editor::layout`),
//! drawn by their views' kinds: a flag, a number, a pick of one, a list, and
//! the NaviCust's grid (`crate::editor::navicust`). Each problem the rules'
//! `validate` ties to a field and an entry shows beside its row.

use crate::editor::app::{Choice, Editor, Msg};
use crate::editor::layout::{FieldPane, List, Pick, View, default_view, element, record_field, room};
use crate::editor::navicust::GridEdit;
use crate::editor::view::{DIM, RED, SIDES, heading};
use iced::widget::{Column, Row, button, checkbox, column, pick_list, row, scrollable, space, text, text_input};
use iced::{Alignment, Element, Length};
use nettai_battle::rules::Fact;
use nettai_content_api::{FieldType, Registry, Value};
use nettai_match::Side;
use nettai_match::facts::{Stated, fact_of};

/// The view at `path` among the panes, and the type of its setup field.
fn view_at<'a>(e: &'a Editor, path: &str) -> Option<(&'a FieldPane, FieldType)> {
    let f = e.panes.iter().flat_map(|p| &p.fields).find(|f| f.path == path)?;
    Some((f, nettai_match::facts::field(&e.content, &f.field)?.ty.clone()))
}

/// What the rules' `validate` says of side `s`'s field `field` at `entry`
/// (none: the field as a whole).
pub fn said<'a>(e: &'a Editor, s: usize, field: &'a str, entry: Option<usize>) -> impl Iterator<Item = &'a str> + 'a {
    e.problems.iter().filter(move |p| p.side == Some(s) && p.field.as_deref() == Some(field) && p.entry == entry).map(|p| p.text.as_str())
}

// ---- Edits ----------------------------------------------------------------------------

/// An edit of a fact through its view.
#[derive(Clone, Debug)]
pub enum Edit {
    Flag(bool),
    /// A number, as typed.
    Number(String),
    /// One picked: an enum's variant's place, a definition's handle, a
    /// number, an entry's place in another list; none.
    Pick(Option<i64>),
    /// A definition into a list, at its end.
    Add(u16),
    Remove(usize),
    /// A row up (true) or down.
    Move(usize, bool),
    /// A list's row's column, by its field.
    Cell(usize, String, Box<Edit>),
    Grid(GridEdit),
    /// The search of a list's or a grid's pieces.
    Search(String),
    /// The field back at the rules' default (what a side that says nothing
    /// has), or holding nothing.
    Default,
    Empty,
}

/// Apply `edit` to side `s`'s fact through the view at `path`; whether the
/// match changed.
pub fn apply(e: &mut Editor, s: usize, path: &str, edit: Edit) -> bool {
    let Some((f, ty)) = view_at(e, path).map(|(f, ty)| (f.clone(), ty)) else { return false };
    let content = e.content.clone();
    let c = &*content;
    let side = &mut e.m.sides[s];
    let current = side.facts.get(c, &f.field);
    let write = |side: &mut Side, value: &Stated| side.set_fact(c, &f.field, &values(value)).is_ok();
    match (edit, &f.view) {
        (Edit::Search(t), View::Grid(_)) => {
            e.grids[s].entry(path.to_string()).or_default().search = t;
            false
        }
        (Edit::Search(t), _) => {
            e.search = t;
            false
        }
        (Edit::Flag(on), _) => side.set_fact(c, &f.field, &[Fact::Value(Value::Bool(on))]).is_ok(),
        (Edit::Default, _) => {
            let game = e.m.game.clone();
            crate::editor::facts::apply(c, &game, &mut e.m.sides[s], &f.field, &crate::editor::facts::Edit::Default)
        }
        (Edit::Empty, _) => side.set_fact(c, &f.field, &[]).is_ok(),
        (Edit::Number(t), View::Number { time }) => {
            let key = (s, path.to_string());
            let parsed = number(&t, *time, &ty);
            e.pane_typed.insert(key.clone(), t);
            match parsed {
                Some(v) if side.set_fact(c, &f.field, &[Fact::Value(v)]).is_ok() => {
                    e.pane_typed.remove(&key);
                    true
                }
                _ => false,
            }
        }
        (Edit::Pick(v), View::Pick(_)) => side.set_fact(c, &f.field, &[pick_fact(&ty, v)]).is_ok(),
        (edit, View::List(l)) => {
            let Some(Stated::List(mut items)) = current else { return false };
            let room = room(&ty);
            let registry = match element(&ty) {
                FieldType::Ref(r, _) => Some(*r),
                _ => None,
            };
            match edit {
                Edit::Add(h) if !l.fixed && items.len() < room => items.push(Stated::Def(registry.unwrap_or(Registry::Chip), Some(h))),
                Edit::Remove(i) if !l.fixed && i < items.len() => {
                    items.remove(i);
                }
                Edit::Move(i, up) if !l.fixed => {
                    let j = if up { i.checked_sub(1) } else { (i + 1 < items.len()).then_some(i + 1) };
                    let Some(j) = j else { return false };
                    items.swap(i, j);
                }
                Edit::Cell(i, column, cell) => {
                    let Some(Stated::Record(fields)) = items.get_mut(i) else { return false };
                    let Some(cty) = record_field(&ty, &column).cloned() else { return false };
                    let view = l.columns.iter().find(|(n, _)| *n == column).map(|(_, v)| v.clone());
                    let new = match (*cell, view) {
                        (Edit::Number(t), Some(View::Number { time })) => {
                            let key = (s, format!("{path}.{i}.{column}"));
                            let parsed = number(&t, time, &cty);
                            e.pane_typed.insert(key.clone(), t);
                            let Some(v) = parsed else { return false };
                            e.pane_typed.remove(&key);
                            stated_of(&cty, v)
                        }
                        (Edit::Number(t), _) => {
                            let Some(v) = number(&t, false, &cty) else { return false };
                            stated_of(&cty, v)
                        }
                        (Edit::Flag(b), _) => Stated::Flag(b),
                        (Edit::Pick(v), _) => match (&cty, v) {
                            (FieldType::Enum(names), Some(i)) => Stated::Variant(names.get(i as usize).cloned()),
                            (FieldType::Ref(r, _), v) => Stated::Def(*r, v.map(|h| h as u16)),
                            (_, Some(n)) => Stated::Number(n),
                            (_, None) => Stated::Optional(None),
                        },
                        _ => return false,
                    };
                    match fields.iter_mut().find(|(n, _)| *n == column) {
                        Some((_, v)) => *v = new,
                        None => return false,
                    }
                }
                _ => return false,
            }
            write(side, &Stated::List(items))
        }
        (Edit::Grid(g), View::Grid(grid)) => {
            let look = crate::editor::navicust::look(e.navicust.as_ref(), c, side, grid);
            let state = e.grids[s].entry(path.to_string()).or_default();
            crate::editor::navicust::grid_edit(c, side, &f.field, &ty, grid, &look, state, g)
        }
        _ => false,
    }
}

/// A number as typed into a field of type `ty` (`time`: mm:ss.cc, its
/// frames); none: it reads as none of its.
fn number(t: &str, time: bool, ty: &FieldType) -> Option<Value> {
    let t = t.trim();
    if t.is_empty() && matches!(ty, FieldType::OptionalU8) {
        return Some(Value::Nil);
    }
    if time {
        return nettai_match::sp_times::parse(t).ok().map(|f| Value::Int(f as i64));
    }
    let n: i64 = t.parse().ok()?;
    match nettai_match::facts::range(ty) {
        Some((lo, hi, _)) if !(lo..=hi).contains(&n) => None,
        _ => Some(Value::Int(n)),
    }
}

/// A value as a record's field reads it.
fn stated_of(ty: &FieldType, v: Value) -> Stated {
    match (ty, v) {
        (FieldType::OptionalU8, Value::Nil) => Stated::Optional(None),
        (FieldType::OptionalU8, Value::Int(n)) => Stated::Optional(Some(n)),
        (_, Value::Int(n)) => Stated::Number(n),
        (_, Value::Bool(b)) => Stated::Flag(b),
        _ => Stated::Other,
    }
}

/// A pick as the facts' writer takes it.
fn pick_fact(ty: &FieldType, v: Option<i64>) -> Fact<'_> {
    match (ty, v) {
        (FieldType::Enum(names), Some(i)) => names.get(i as usize).map_or(Fact::Value(Value::Nil), |n| Fact::Name(n)),
        (FieldType::Ref(r, _), Some(h)) => Fact::Value(Value::Def(*r, h as u16)),
        (_, Some(n)) => Fact::Value(Value::Int(n)),
        (_, None) => Fact::Value(Value::Nil),
    }
}

/// A read value's facts for the writer: a list's entries, else the value.
pub(crate) fn values(v: &Stated) -> Vec<Fact<'_>> {
    match fact_of(v) {
        Fact::List(items) => items,
        f => vec![f],
    }
}

// ---- Drawing --------------------------------------------------------------------------

/// A definition's name in the editor's language.
pub(crate) fn name_of(e: &Editor, registry: Registry, h: u16) -> String {
    let c = &e.content;
    match registry {
        Registry::Chip => e.names.chip(c, nettai_content_api::ChipHandle(h)),
        Registry::Form => e.names.form(c, nettai_content_api::FormHandle(h)),
        Registry::Navi => e.names.navi(c, nettai_content_api::NaviHandle(h)),
        Registry::Entry => e.names.entry(c, nettai_content_api::EntryHandle(h)),
        _ => nettai_match::ids::name_of(c, registry, h).map_or_else(|| h.to_string(), str::to_string),
    }
}

/// The definitions a view offers: every one of the field's registry (or
/// collection) of the match's game, in the library's order where it has
/// one.
pub(crate) fn offered(e: &Editor, registry: Registry, of: Option<&str>) -> Vec<u16> {
    let c = &e.content;
    let mut all: Vec<(String, u16)> = nettai_match::ids::all_of(c, registry, of)
        .into_iter()
        .filter(|&h| nettai_match::ids::key_of(c, registry, h).is_some_and(|k| nettai_match::ids::in_game(c, e.m.game(), k)))
        .map(|h| (name_of(e, registry, h), h))
        .collect();
    match (registry, of) {
        (Registry::Entry, Some(collection)) => {
            let mut by: Vec<(String, nettai_content_api::EntryHandle)> = all.iter().map(|(n, h)| (n.clone(), nettai_content_api::EntryHandle(*h))).collect();
            e.order.entries(c, collection, &mut by);
            all = by.into_iter().map(|(n, h)| (n, h.0)).collect();
        }
        (Registry::Chip, _) => {
            let mut by: Vec<(String, nettai_content_api::ChipHandle)> = all.iter().map(|(n, h)| (n.clone(), nettai_content_api::ChipHandle(*h))).collect();
            e.order.chips(c, &mut by);
            all = by.into_iter().map(|(n, h)| (n, h.0)).collect();
        }
        _ => {}
    }
    all.into_iter().map(|(_, h)| h).collect()
}

/// Pane `i` of side `s`.
pub fn view(e: &Editor, s: usize, i: usize) -> Element<'_, Msg> {
    let Some(p) = e.panes.get(i) else { return space().into() };
    let mut col = column![heading(format!("{}: {}", SIDES[s], p.title))].spacing(10);
    let mut grid: Option<Element<Msg>> = None;
    for f in &p.fields {
        let Some((_, ty)) = view_at(e, &f.path) else { continue };
        let label = f.label.clone();
        match &f.view {
            View::Grid(g) => grid = Some(crate::editor::navicust::grid_view(e, s, f, g, &ty)),
            View::List(l) => col = col.push(list_view(e, s, f, l, &ty, &label)),
            _ => {
                col = col.push(row![text(label).size(14).width(Length::Fixed(160.0)), control(e, s, f, &f.view, &ty, None)].spacing(8).align_y(Alignment::Center));
                for p in said(e, s, &f.field, None) {
                    col = col.push(text(p.to_string()).size(13).color(RED));
                }
            }
        }
    }
    match grid {
        Some(g) => column![col, g].spacing(10).into(),
        None => scrollable(col).into(),
    }
}

/// A one-value view's control: of the side's fact, or (`cell`: its row and
/// column) of a list's record's field.
fn control<'a>(e: &'a Editor, s: usize, f: &'a FieldPane, view: &View, ty: &FieldType, cell: Option<(usize, String, Stated)>) -> Element<'a, Msg> {
    let c = &e.content;
    let path = f.path.clone();
    let value = match &cell {
        Some((_, _, v)) => Some(v.clone()),
        None => e.side(s).facts.get(c, &f.field),
    };
    let msg = {
        let cell = cell.clone();
        move |edit: Edit| match &cell {
            Some((i, column, _)) => Msg::Pane(s, path.clone(), Edit::Cell(*i, column.clone(), Box::new(edit))),
            None => Msg::Pane(s, path.clone(), edit),
        }
    };
    match view {
        View::Flag => {
            let on = matches!(value, Some(Stated::Flag(true)));
            checkbox(on).on_toggle(move |b| msg(Edit::Flag(b))).into()
        }
        View::Number { time } => {
            let key = match &cell {
                Some((i, column, _)) => format!("{}.{i}.{column}", f.path),
                None => f.path.clone(),
            };
            let shown = e.pane_typed.get(&(s, key)).cloned().unwrap_or_else(|| match &value {
                Some(Stated::Number(n)) if *time => nettai_match::sp_times::format(*n as u16),
                Some(Stated::Number(n)) => n.to_string(),
                Some(Stated::Optional(Some(n))) => n.to_string(),
                _ => String::new(),
            });
            let hint = if *time { "00:00.00" } else { "" };
            text_input(hint, &shown).on_input(move |t| msg(Edit::Number(t))).width(Length::Fixed(100.0)).into()
        }
        View::Pick(p) => {
            let choices = pick_choices(e, p, ty);
            let now = match &value {
                Some(Stated::Variant(Some(n))) => match ty {
                    FieldType::Enum(names) => names.iter().position(|x| x == n).map(|i| i as i64),
                    _ => None,
                },
                Some(Stated::Def(_, h)) => h.map(|h| h as i64),
                Some(Stated::Number(n)) => Some(*n),
                Some(Stated::Optional(n)) => *n,
                _ => None,
            };
            let picked = choices.iter().find(|x| x.value == now).cloned();
            pick_list(choices, picked, move |x: Choice<Option<i64>>| msg(Edit::Pick(x.value))).placeholder("choose one").into()
        }
        View::List(_) | View::Grid(_) => space().into(),
    }
}

/// What a pick offers, each by its label: an enum's variants, the
/// definitions offered, the numbers by name; "none" first where the field
/// may hold none.
fn pick_choices(e: &Editor, p: &Pick, ty: &FieldType) -> Vec<Choice<Option<i64>>> {
    let mut out = Vec::new();
    let none = Choice { label: "none".to_string(), value: None };
    match ty {
        FieldType::Enum(names) => out.extend(names.iter().enumerate().map(|(i, n)| Choice { label: n.clone(), value: Some(i as i64) })),
        FieldType::Ref(r, of) => {
            out.push(none);
            out.extend(offered(e, *r, of.as_deref()).into_iter().map(|h| Choice { label: name_of(e, *r, h), value: Some(h as i64) }));
        }
        _ => {
            if matches!(ty, FieldType::OptionalU8) {
                out.push(none);
            }
            out.extend(p.choices.iter().map(|(n, name)| Choice { label: name.clone(), value: Some(*n) }));
        }
    }
    out
}

/// A list's view: its count and its entries' total; its rows (each a
/// definition's name, or a record's fields side by side), each moved up or
/// down or taken out, with what the rules say of it; and what may be
/// added.
fn list_view<'a>(e: &'a Editor, s: usize, f: &'a FieldPane, l: &'a List, ty: &FieldType, label: &str) -> Element<'a, Msg> {
    let c = &e.content;
    let path = f.path.clone();
    let msg = move |edit: Edit| Msg::Pane(s, path.clone(), edit);
    let items = match e.side(s).facts.get(c, &f.field) {
        Some(Stated::List(items)) => items,
        _ => Vec::new(),
    };
    let room = room(ty);
    let mut col = Column::new().spacing(6);
    let mut top = row![text(label.to_string()).size(16)].spacing(12).align_y(Alignment::Center);
    top = top.push(text(format!("{} of {room}", items.len())).size(13).color(DIM));
    // (Its entries' total: each patch card's MB, say.)
    if let Some(total) = &l.total {
        let sum: i64 = items
            .iter()
            .flat_map(Stated::defs)
            .filter_map(|h| {
                let key = &c.defs.entry(nettai_content_api::EntryHandle(h)).key;
                c.defs.definitions.get(Registry::Entry, key).and_then(|d| d.spec.field(total).int())
            })
            .sum();
        top = top.push(text(format!("{sum} {} in all", total.to_uppercase())).size(13).color(DIM));
    }
    // Back to the rules' default ("None" where that holds nothing), or none.
    let side = e.side(s);
    let default_empty = nettai_match::Facts::defaults(c).get(c, &f.field).is_none_or(|v| matches!(&v, Stated::List(d) if d.iter().all(|x| x.defs().is_empty())));
    if !side.facts.is_default(c, &f.field) {
        let label = if default_empty { "None" } else { "Default" };
        top = top.push(button(text(label).size(13)).on_press(msg(Edit::Default)).style(button::secondary));
    }
    if !items.is_empty() && !default_empty && !l.fixed {
        top = top.push(button(text("None").size(13)).on_press(msg(Edit::Empty)).style(button::secondary));
    }
    col = col.push(top);
    for p in said(e, s, &f.field, None) {
        col = col.push(text(p.to_string()).size(13).color(RED));
    }
    // (What the rules say of a row, below it.)
    let row_said = |col: Column<'a, Msg>, i: usize| said(e, s, &f.field, Some(i)).fold(col, |col, p| col.push(text(p.to_string()).size(13).color(RED)));
    let elem = element(ty).clone();
    match &elem {
        FieldType::Ref(registry, of) => {
            for (i, item) in items.iter().enumerate() {
                let name = item.defs().first().map(|&h| name_of(e, *registry, h)).unwrap_or_default();
                let mut line = row![text(name).size(14).width(Length::Fill)].spacing(6).align_y(Alignment::Center);
                if !l.fixed {
                    line = line
                        .push(button(text("↑").size(12)).on_press(msg(Edit::Move(i, true))).style(button::text))
                        .push(button(text("↓").size(12)).on_press(msg(Edit::Move(i, false))).style(button::text))
                        .push(button(text("remove").size(12)).on_press(msg(Edit::Remove(i))).style(button::danger));
                }
                col = row_said(col.push(line), i);
            }
            if !l.fixed && items.len() < room {
                let needle = e.search.to_lowercase();
                let held: Vec<u16> = items.iter().flat_map(Stated::defs).collect();
                let mut add = Column::new().spacing(1);
                for h in offered(e, *registry, of.as_deref()) {
                    let name = name_of(e, *registry, h);
                    if held.contains(&h) || !(needle.is_empty() || name.to_lowercase().contains(&needle)) {
                        continue;
                    }
                    add = add.push(row![button(text("add").size(12)).on_press(msg(Edit::Add(h))).style(button::secondary), text(name).size(14)].spacing(6).align_y(Alignment::Center));
                }
                col = col.push(text_input("search", &e.search).on_input(move |t| Msg::Pane(s, f.path.clone(), Edit::Search(t))));
                col = col.push(scrollable(add).height(Length::Fixed(260.0)));
            }
        }
        FieldType::Record(fields) => {
            for (i, item) in items.iter().enumerate() {
                let Stated::Record(values) = item else { continue };
                let mut line = Row::new().spacing(8).align_y(Alignment::Center);
                for fd in fields.fields() {
                    let v = values.iter().find(|(n, _)| *n == fd.name).map(|(_, v)| v.clone()).unwrap_or(Stated::Other);
                    let declared = l.columns.iter().find(|(n, _)| *n == fd.name).map(|(_, v)| v.clone());
                    // (A fixed list's definitions are its rows' names.)
                    let cell: Element<Msg> = match (&v, &declared) {
                        (Stated::Def(r, h), None) if l.fixed => text(h.map(|h| name_of(e, *r, h)).unwrap_or_default()).size(14).width(Length::Fixed(160.0)).into(),
                        _ => {
                            let view = declared.unwrap_or_else(|| default_view(&fd.ty));
                            control(e, s, f, &view, &fd.ty, Some((i, fd.name.clone(), v)))
                        }
                    };
                    line = line.push(cell);
                }
                if !l.fixed {
                    line = line
                        .push(button(text("↑").size(12)).on_press(msg(Edit::Move(i, true))).style(button::text))
                        .push(button(text("↓").size(12)).on_press(msg(Edit::Move(i, false))).style(button::text))
                        .push(button(text("remove").size(12)).on_press(msg(Edit::Remove(i))).style(button::danger));
                }
                col = row_said(col.push(line), i);
            }
        }
        _ => {}
    }
    scrollable(col).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The editor's layout of each game's setup: EXE6's the NaviCust (the
    /// board a pick, the programs a grid), the patch cards (a list with its
    /// MB in all) and the SP times (fixed rows, each a time); EXE5's the
    /// same, its auto battle data its own pane's; no game's field without
    /// a view, and no pane for what the core's panes show.
    #[test]
    fn each_games_setup_is_laid_out() {
        for (content, game) in [(nettai_match::testing::exe6_content(), "exe6"), (nettai_match::testing::exe5_content(), "exe5")] {
            let modules = crate::editor::layout::modules(&content, game, &[crate::editor::navicust::MODULE, crate::editor::auto_battle::MODULE]);
            let data = crate::editor::navicust::read(&content, modules[0].as_ref());
            let auto_battle = crate::editor::auto_battle::Layout::read(modules[1].as_ref());
            assert_eq!(auto_battle.is_some(), game == "exe5", "{game}");
            let panes = crate::editor::layout::layout(&content, data.as_ref(), auto_battle.as_ref());
            let keys: Vec<&str> = panes.iter().map(|p| p.key.as_str()).collect();
            assert_eq!(keys, ["navicust", "patch_cards", "sp_times"], "{game}");
            assert!(data.is_some(), "{game}: the NaviCust's data fit the grid");
            let view = |field: &str| panes.iter().flat_map(|p| &p.fields).find(|f| f.field == field).map(|f| f.view.clone());
            assert!(matches!(view("navicust_programs"), Some(View::Grid(_))), "{game}");
            assert!(matches!(view("patch_cards"), Some(View::List(List { total: Some(ref t), fixed: false, .. })) if t == "mb"), "{game}");
            assert!(matches!(view("sp_times"), Some(View::List(List { fixed: true, ref columns, .. })) if columns[0].1 == View::Number { time: true }), "{game}");
            // (Without the NaviCust's data, the programs are a list.)
            let plain = crate::editor::layout::layout(&content, None, auto_battle.as_ref());
            assert!(matches!(plain.iter().flat_map(|p| &p.fields).find(|f| f.field == "navicust_programs").map(|f| &f.view), Some(View::List(_))), "{game}");
        }
    }

    /// A number as typed: a time's frames, a value past its type refused.
    #[test]
    fn numbers_read_as_typed() {
        assert_eq!(number("00:01.00", true, &FieldType::U16), Some(Value::Int(60)));
        assert_eq!(number("300", false, &FieldType::U8), None);
        assert_eq!(number(" 12 ", false, &FieldType::U8), Some(Value::Int(12)));
        assert_eq!(number("", false, &FieldType::OptionalU8), Some(Value::Nil));
    }
}
