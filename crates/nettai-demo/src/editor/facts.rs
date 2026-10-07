//! A side's facts in the editor: what its game's rules take besides the
//! engine's own parts (`nettai_match::facts`: EXE6's version, its Crosses;
//! EXE5's karma, its souls), each shown by its setup field's type and
//! nothing else. The editor names no game's fact: a game that declares
//! another gets its control here without a line of this crate changing.
//!
//! - a flag: a checkbox;
//! - a number: a field to type it in (the rules' default its placeholder);
//! - an enum: a list of its variants, by the names the rules declare
//!   (nothing chosen until the side states one, where the rules give no
//!   default: a round doesn't start without);
//! - a few flags (`bool[5]`): a checkbox each, numbered;
//! - a list of definitions (`form[5]`, `form[16]`): a pane of its own,
//!   with a checkbox for each definition the list may hold
//!   (`nettai_match::facts::offered`). One the rules give no default is
//!   not stated until the side states it (an empty list is a statement:
//!   "None" states it), as an enum is.
//!
//! Each has a button back to the rules' default where it has one and isn't
//! that.
//!
//! The rules assume nothing a side must state, so the editor fills in what
//! it can for the person: when the side's version is chosen (the engine's
//! version fact), its form list (the engine's: EXE6's Crosses) becomes its
//! navi's own of that version (`Side::state_own_forms`) where it wasn't
//! stated, or was the last version's own.

use crate::editor::app::{Editor, Msg};
use crate::editor::view::{DIM, RED, SIDES, heading};
use iced::widget::{Column, Row, button, checkbox, column, image, pick_list, row, scrollable, space, text, text_input};
use iced::{Alignment, Element, Length};
use nettai_battle::Content;
use nettai_battle::content::PlayerFact;
use nettai_battle::rules::Fact;
use nettai_content_api::{ChipHandle, FieldType, FormHandle, NaviHandle, Registry, Value};
use nettai_match::Side;
use nettai_match::facts::{self, Field, Stated};

/// An edit of one of a side's facts.
#[derive(Clone, Debug)]
pub enum Edit {
    Flag(bool),
    /// An element of a list of flags.
    FlagAt(usize, bool),
    /// A number, as typed.
    Number(String),
    /// An enum's variant, by its name.
    Variant(String),
    /// A definition into a list of them, or out of it.
    Listed(u16, bool),
    /// A list of definitions stated as holding none.
    Empty,
    /// The form list as the side's navi's own of its version.
    Own,
    /// Back at what a side that says nothing has.
    Default,
}

/// A fact's name as a pane's label: `chaos_unison` as "Chaos unison".
pub fn title(name: &str) -> String {
    let spaced = name.replace('_', " ");
    let mut letters = spaced.chars();
    match letters.next() {
        Some(first) => first.to_uppercase().chain(letters).collect(),
        None => spaced,
    }
}

/// Whether `field` is a list of definitions: a pane of its own.
pub fn is_list(field: &Field) -> bool {
    matches!(field.ty, FieldType::Array(elem, _) if matches!(**elem, FieldType::Ref(..)))
}

/// The side's list facts with something to offer, each a pane: its place
/// among the game's facts and its title.
pub fn lists(content: &Content, game: &str, side: &Side) -> Vec<(usize, String)> {
    facts::fields(content)
        .iter()
        .enumerate()
        .filter(|(_, f)| facts::offered(content, game, side, f).is_some_and(|o| !o.is_empty()))
        .map(|(i, f)| (i, title(f.name)))
        .collect()
}

/// The place among the game's facts of the list fact named `name`.
pub fn list_named(content: &Content, name: &str) -> Option<usize> {
    facts::fields(content).iter().position(|f| f.name == name && is_list(f))
}

/// Apply `edit` to the side's fact `name`. Whether the side changed: an
/// edit the fact doesn't take (a number that isn't one, or past its type)
/// changes nothing.
pub fn apply(content: &Content, game: &str, side: &mut Side, name: &str, edit: &Edit) -> bool {
    let Some(field) = facts::field(content, name) else { return false };
    let before = side.facts.clone();
    // (The side's form list, if it is its navi's own of its version: it
    // follows a change of the version. An empty one stays empty.)
    let own = |side: &Side| {
        let mut with_own = side.clone();
        with_own.state_own_forms(content) && with_own.facts == side.facts && !side.facts.form_list(content).is_empty()
    };
    let followed = facts::role_of(content, name) == Some(PlayerFact::Version) && own(side);
    let done = match edit {
        Edit::Default => {
            side.facts.reset(content, name);
            Ok(())
        }
        Edit::Empty => side.set_fact(content, name, &[]),
        Edit::Own => {
            side.state_own_forms(content);
            Ok(())
        }
        Edit::Flag(on) => side.set_fact(content, name, &[Fact::Value(Value::Bool(*on))]),
        Edit::FlagAt(k, on) => match side.facts.get(content, name) {
            Some(Stated::List(items)) => {
                let flags: Vec<Fact> =
                    items.iter().enumerate().map(|(i, v)| Fact::Value(Value::Bool(if i == *k { *on } else { *v == Stated::Flag(true) }))).collect();
                side.set_fact(content, name, &flags)
            }
            _ => Ok(()),
        },
        Edit::Number(typed) => match typed.trim().parse::<i64>() {
            Ok(n) => side.set_fact(content, name, &[Fact::Value(Value::Int(n))]),
            Err(e) => Err(e.to_string()),
        },
        Edit::Variant(variant) => side.set_fact(content, name, &[Fact::Name(variant)]),
        Edit::Listed(h, on) => match (field.ty, facts::offered(content, game, side, &field)) {
            (FieldType::Array(elem, capacity), Some(order)) => {
                let FieldType::Ref(registry, _) = **elem else { return false };
                let mut held = side.facts.get(content, name).map(|v| v.defs()).unwrap_or_default();
                held.retain(|x| x != h);
                if *on && held.len() < *capacity as usize {
                    held.push(*h);
                }
                // (In the order the pane offers them: a form list's is its
                // window's.)
                held.sort_by_key(|h| order.iter().position(|x| x == h));
                let list: Vec<Fact> = held.iter().map(|&h| Fact::Value(Value::Def(registry, h))).collect();
                side.set_fact(content, name, &list)
            }
            _ => Ok(()),
        },
    };
    if done.is_err() {
        side.facts = before;
        return false;
    }
    if followed {
        side.state_own_forms(content);
    }
    side.facts != before
}

/// The side's facts that are one value, or a few flags: a row each.
pub fn rows(e: &Editor, s: usize) -> Column<'_, Msg> {
    let c = &e.content;
    let side = e.side(s);
    let defaults = nettai_match::Facts::defaults(c);
    let mut col = Column::new().spacing(10);
    let in_panes: Vec<&str> = e.panes.iter().flat_map(|p| p.fields.iter().map(|f| f.field.as_str())).collect();
    for f in facts::fields(c) {
        // (The level has its own field, with the navi; a pane shows its
        // own.)
        if facts::role_of(c, f.name) == Some(PlayerFact::Level) || in_panes.contains(&f.name) {
            continue;
        }
        let Some(value) = side.facts.get(c, f.name) else { continue };
        let name = f.name.to_string();
        let msg = move |edit: Edit| Msg::Fact(s, name.clone(), edit);
        let control: Element<Msg> = match (&value, f.ty) {
            (Stated::Flag(on), _) => {
                let msg = msg.clone();
                checkbox(*on).on_toggle(move |b| msg(Edit::Flag(b))).into()
            }
            (Stated::Number(n), _) => {
                let shown = e.fact_typed.get(&(s, f.name.to_string())).cloned().unwrap_or_else(|| n.to_string());
                let default = match defaults.get(c, f.name) {
                    Some(Stated::Number(d)) => d.to_string(),
                    _ => String::new(),
                };
                let msg = msg.clone();
                text_input(&default, &shown).on_input(move |t| msg(Edit::Number(t))).width(Length::Fixed(100.0)).into()
            }
            (Stated::Variant(chosen), FieldType::Enum(variants)) => {
                let msg = msg.clone();
                pick_list(variants.clone(), chosen.clone(), move |v| msg(Edit::Variant(v))).placeholder("choose one").into()
            }
            (Stated::List(items), _) if !items.is_empty() && items.iter().all(|v| matches!(v, Stated::Flag(_))) => {
                let mut flags = Row::new().spacing(12);
                for (k, v) in items.iter().enumerate() {
                    let msg = msg.clone();
                    flags = flags.push(checkbox(*v == Stated::Flag(true)).label((k + 1).to_string()).on_toggle(move |b| msg(Edit::FlagAt(k, b))));
                }
                flags.into()
            }
            // (A list of definitions has a pane of its own.)
            _ => continue,
        };
        let mut line = row![text(title(f.name)).size(14).width(Length::Fixed(160.0)), control].spacing(8).align_y(Alignment::Center);
        let unstated = value == Stated::Variant(None);
        let required = defaults.get(c, f.name) == Some(Stated::Variant(None));
        if !unstated && !side.facts.is_default(c, f.name) && !required {
            line = line.push(button(text("Default").size(13)).on_press(msg(Edit::Default)).style(button::secondary));
        }
        col = col.push(line);
        for p in crate::editor::panes::said(e, s, f.name, None) {
            col = col.push(text(p.to_string()).size(13).color(RED));
        }
        if unstated {
            col = col.push(text(format!("No {} is chosen: a side states its own, and a round doesn't start without.", f.name)).size(13).color(RED));
        }
    }
    col
}

/// A definition's name in the editor's language, else its key.
fn def_name(e: &Editor, registry: Registry, h: u16) -> String {
    let c = &e.content;
    match registry {
        Registry::Form => e.names.form(c, FormHandle(h)),
        Registry::Chip => e.names.chip(c, ChipHandle(h)),
        Registry::Navi => e.names.navi(c, NaviHandle(h)),
        _ => nettai_match::ids::key_of(c, registry, h).map_or_else(|| h.to_string(), |k| nettai_match::ids::local(k).to_string()),
    }
}

/// What a definition says of itself beside its name: a form's version (as
/// the rules name it), the chips a soul is for.
fn about(c: &Content, registry: Registry, h: u16) -> String {
    if registry != Registry::Form {
        return String::new();
    }
    let form = c.form(FormHandle(h));
    let mut said: Vec<String> = Vec::new();
    said.extend(form.version.clone());
    // (EXE5's souls: a form's `soul`, its rules' extension.)
    let soul = c.defs.extension(Registry::Form, &c.defs.form(FormHandle(h)).key, "soul");
    said.extend(soul.and_then(|s| s.field("family").str()).map(|family| format!("for {family} chips")));
    said.join(", ")
}

/// The pane of the side's list fact at `index` among the game's facts: a
/// checkbox for each definition it may hold (`facts::offered`), those it
/// holds checked, in the list's order.
pub fn list(e: &Editor, s: usize, index: usize) -> Element<'_, Msg> {
    let c = &e.content;
    let side = e.side(s);
    let fields = facts::fields(c);
    let Some(f) = fields.get(index).filter(|f| is_list(f)) else { return space().into() };
    let (FieldType::Array(elem, capacity), Some(offered)) = (f.ty, facts::offered(c, e.m.game(), side, f)) else { return space().into() };
    let FieldType::Ref(registry, _) = **elem else { return space().into() };
    let held = side.facts.get(c, f.name).map(|v| v.defs()).unwrap_or_default();
    let name = f.name.to_string();
    let msg = move |edit: Edit| Msg::Fact(s, name.clone(), edit);
    // (The list's own order is the pane's once it is edited here; a file's
    // may be another, and is said.)
    let mut in_order = held.clone();
    in_order.sort_by_key(|h| offered.iter().position(|x| x == h));
    let count = if in_order == held {
        format!("{} of {capacity}, in this order.", held.len())
    } else {
        let names: Vec<String> = held.iter().map(|&h| def_name(e, registry, h)).collect();
        format!("{} of {capacity}, listed in another order than here: {}.", held.len(), names.join(", "))
    };
    let mut top = row![text(count).size(13).color(DIM)].spacing(12).align_y(Alignment::Center);
    // (A list whose default is empty: its default is "None".)
    let default_empty = nettai_match::Facts::defaults(c).get(c, f.name).is_none_or(|v| v.defs().is_empty());
    if side.facts.is_default(c, f.name) {
        top = top.push(text("The rules' default: what a side that says nothing has.").size(13).color(DIM));
    } else {
        let label = if default_empty { "None" } else { "Default" };
        top = top.push(button(text(label).size(13)).on_press(msg(Edit::Default)).style(button::secondary));
    }
    if !held.is_empty() && !default_empty {
        top = top.push(button(text("None").size(13)).on_press(msg(Edit::Empty)).style(button::secondary));
    }
    if facts::role_of(c, f.name) == Some(PlayerFact::FormList) && side.version(c).is_some() {
        top = top.push(button(text("Its version's own").size(13)).on_press(msg(Edit::Own)).style(button::secondary));
    }
    let mut col = column![heading(format!("{}: {}", SIDES[s], title(f.name))), top].spacing(8);
    for p in crate::editor::panes::said(e, s, f.name, None) {
        col = col.push(text(p.to_string()).size(13).color(RED));
    }
    for h in offered {
        let on = held.contains(&h);
        let key = nettai_match::ids::key_of(c, registry, h).unwrap_or_default();
        let face: Element<Msg> = match e.pictures.face(key) {
            Some(picture) => image(picture.clone()).width(64).height(32).filter_method(image::FilterMethod::Nearest).into(),
            None => space().width(64).height(32).into(),
        };
        let msg = msg.clone();
        let mut tick = checkbox(on);
        // (A full list takes no more.)
        if on || held.len() < *capacity as usize {
            tick = tick.on_toggle(move |b| msg(Edit::Listed(h, b)));
        }
        col = col.push(
            row![tick, face, text(def_name(e, registry, h)).size(15).width(Length::Fixed(160.0)), text(about(c, registry, h)).size(13).color(DIM)]
                .spacing(10)
                .align_y(Alignment::Center),
        );
    }
    scrollable(col).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::testing::{exe5_content, exe6_content};

    /// Each game's facts get their controls from their types alone: EXE6's
    /// version a list of two, its Crosses a pane of MegaMan's ten; EXE5's
    /// karma a number, its souls a pane of twelve. Edits go through the
    /// facts' own writer, and one a fact doesn't take changes nothing. What
    /// the rules require and assume nothing of, the editor fills in where
    /// it can: the version's own Crosses when the version is chosen.
    #[test]
    fn facts_are_edited_by_their_types() {
        let six = exe6_content();
        let mut m = nettai_match::Match::empty(&six, "exe6").unwrap();
        let side = &mut m.sides[0];
        // (The facts in the setup's order, their names': beast_out,
        // bug_frags, crosses, ...)
        assert_eq!(lists(&six, "exe6", side), [(2, "Crosses".to_string())]);
        assert_eq!(list_named(&six, "crosses"), Some(2));
        assert_eq!(list_named(&six, "version"), None, "an enum: a row, no pane");
        let field = facts::field(&six, "crosses").unwrap();
        let offered = facts::offered(&six, "exe6", side, &field).unwrap();
        assert_eq!(offered.len(), 10, "MegaMan's Crosses of both versions: Gregar's five, then Falzar's");
        let crosses = |side: &Side| side.facts.get(&six, "crosses").unwrap().defs();
        // A new side states no version and has no Crosses (the list's
        // default). Choosing the version (a variant by its name) leaves
        // them none; its own five, once stated, follow a change of the
        // version while they are the version's own.
        assert_eq!(side.facts.get(&six, "crosses"), Some(Stated::List(vec![Stated::Def(Registry::Form, None); 5])));
        assert!(side.facts.is_default(&six, "crosses"));
        assert!(apply(&six, "exe6", side, "version", &Edit::Variant("falzar".into())));
        assert_eq!(side.version(&six), Some("falzar"));
        assert!(crosses(side).is_empty());
        assert!(apply(&six, "exe6", side, "crosses", &Edit::Own));
        assert_eq!(crosses(side), offered[5..]);
        assert!(apply(&six, "exe6", side, "version", &Edit::Variant("gregar".into())));
        assert_eq!(crosses(side), offered[..5]);
        assert!(!apply(&six, "exe6", side, "version", &Edit::Variant("azure".into())));
        assert_eq!(side.version(&six), Some("gregar"));
        // The Crosses: none, which is a statement; then checked in any
        // order they keep the window's order, and hold five.
        assert!(apply(&six, "exe6", side, "crosses", &Edit::Empty));
        assert_eq!(side.facts.get(&six, "crosses"), Some(Stated::List(vec![Stated::Def(Registry::Form, None); 5])));
        for &h in [offered[7], offered[2], offered[9], offered[0], offered[4], offered[5]].iter() {
            apply(&six, "exe6", side, "crosses", &Edit::Listed(h, true));
        }
        let held = crosses(side);
        assert_eq!(held, [offered[0], offered[2], offered[4], offered[7], offered[9]], "the sixth isn't taken");
        // (Crosses of the side's own choosing stay when the version changes.)
        assert!(apply(&six, "exe6", side, "version", &Edit::Variant("falzar".into())));
        assert_eq!(crosses(side), held);
        assert!(apply(&six, "exe6", side, "crosses", &Edit::Listed(offered[2], false)));
        assert_eq!(crosses(side).len(), 4);
        assert!(apply(&six, "exe6", side, "crosses", &Edit::Own));
        assert_eq!(crosses(side), offered[5..]);
        // A flag, a number as typed.
        assert!(apply(&six, "exe6", side, "beast_out", &Edit::Flag(false)));
        assert!(apply(&six, "exe6", side, "bug_frags", &Edit::Number(" 12 ".into())));
        assert!(!apply(&six, "exe6", side, "bug_frags", &Edit::Number("many".into())));
        assert!(!apply(&six, "exe6", side, "bug_frags", &Edit::Number("-1".into())));
        assert_eq!(side.facts.get(&six, "bug_frags"), Some(Stated::Number(12)));
        // Back at the defaults, for the facts that have one.
        for name in ["beast_out", "bug_frags"] {
            assert!(apply(&six, "exe6", side, name, &Edit::Default), "{name}");
            assert!(side.facts.is_default(&six, name), "{name}");
        }
        assert_eq!(nettai_match::check::check_side_alone(&six, &m.game, &m.sides[0]).iter().filter(|p| !p.contains("folder")).count(), 0);
        // A navi that doesn't change form is offered no form list.
        let side = &mut m.sides[0];
        side.set_navi(&six, nettai_match::ids::navi(&six, "exe6", "protoman").unwrap()).unwrap();
        assert!(lists(&six, "exe6", side).is_empty());

        let five = exe5_content();
        let mut m = nettai_match::Match::empty(&five, "exe5").unwrap();
        let side = &mut m.sides[0];
        let panes = lists(&five, "exe5", side);
        assert_eq!(panes.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>(), ["Souls"]);
        let field = facts::field(&five, "souls").unwrap();
        let offered = facts::offered(&five, "exe5", side, &field).unwrap();
        assert_eq!(offered.len(), 12, "the souls the rules' default lists");
        assert!(apply(&five, "exe5", side, "souls", &Edit::Listed(offered[3], false)));
        assert_eq!(side.facts.get(&five, "souls").unwrap().defs().len(), 11);
        assert_eq!(facts::offered(&five, "exe5", side, &field).unwrap(), offered, "an unchecked soul is still offered");
        assert!(apply(&five, "exe5", side, "karma", &Edit::Number("100".into())));
        assert!(!apply(&five, "exe5", side, "karma", &Edit::Number("70000".into())), "past a u16");
        assert_eq!(side.facts.get(&five, "karma"), Some(Stated::Number(100)));
        assert_eq!(title("chaos_unison"), "Chaos unison");
    }
}
