//! A build's edits, each through the facts' own writer (`nettai_match`):
//! an edit a fact doesn't take (a number past its type, a sixth Cross)
//! changes nothing. What the rules require and assume nothing of, the
//! creator fills in where it can: the version's own forms when the version
//! is chosen, a level where a navi must have one.

use crate::builds::layout::{element, room};
use nettai_battle::content::{ChipCode, Content, PlayerFact};
use nettai_battle::custom::FolderChip;
use nettai_battle::rules::Fact;
use nettai_content_api::{ChipHandle, FieldType, NaviHandle, Registry, Value};
use nettai_match::Side;
use nettai_match::facts::{self, Stated, fact_of};

/// An edit of one of a side's facts.
#[derive(Clone, Debug)]
pub enum FactEdit {
    /// A number, a flag, or an enum's next or last variant, a step on.
    Step(i64),
    Number(i64),
    /// A definition into a list of a few, or out of it.
    Listed(u16, bool),
    /// A list stated as holding none.
    Empty,
    /// The form list as the side's navi's own of its version.
    Own,
    /// Back at what a side that says nothing has.
    Default,
}

/// Apply `edit` to the side's fact `name`: whether the side changed.
pub fn fact(content: &Content, game: &str, side: &mut Side, name: &str, edit: &FactEdit) -> bool {
    let Some(field) = facts::field(content, name) else { return false };
    let before = side.facts.clone();
    // (The side's form list, if it is its navi's own of its version, follows
    // a change of the version. An empty one stays empty.)
    let own = |side: &Side| {
        let mut with_own = side.clone();
        with_own.state_own_forms(content) && with_own.facts == side.facts && !side.facts.form_list(content).is_empty()
    };
    let followed = facts::role_of(content, name) == Some(PlayerFact::Version) && (own(side) || side.facts.form_list(content).is_empty());
    let value = side.facts.get(content, name);
    let done = match edit {
        FactEdit::Default => {
            side.facts.reset(content, name);
            Ok(())
        }
        FactEdit::Empty => side.set_fact(content, name, &[]),
        FactEdit::Own => {
            side.state_own_forms(content);
            Ok(())
        }
        FactEdit::Number(n) => side.set_fact(content, name, &[Fact::Value(Value::Int(*n))]),
        FactEdit::Step(by) => match (field.ty, value) {
            (FieldType::Enum(variants), Some(Stated::Variant(now))) if !variants.is_empty() => {
                let n = variants.len() as i64;
                let at = match now.and_then(|v| variants.iter().position(|x| *x == v)) {
                    Some(i) => (i as i64 + by).rem_euclid(n),
                    None if *by < 0 => n - 1,
                    None => 0,
                };
                side.set_fact(content, name, &[Fact::Name(&variants[at as usize])])
            }
            (ty, Some(Stated::Number(now) | Stated::Optional(Some(now)))) => match facts::range(ty) {
                Some((lo, hi, _)) => side.set_fact(content, name, &[Fact::Value(Value::Int((now + by).clamp(lo, hi)))]),
                None => Ok(()),
            },
            (FieldType::OptionalU8, Some(Stated::Optional(None))) => side.set_fact(content, name, &[Fact::Value(Value::Int(0))]),
            (FieldType::Bool, Some(Stated::Flag(on))) => side.set_fact(content, name, &[Fact::Value(Value::Bool(!on))]),
            _ => Ok(()),
        },
        FactEdit::Listed(h, on) => match (field.ty, facts::offered(content, game, side, &field)) {
            (FieldType::Array(elem, capacity), Some(order)) => {
                let FieldType::Ref(registry, _) = **elem else { return false };
                let mut held = value.map(|v| v.defs()).unwrap_or_default();
                held.retain(|x| x != h);
                if *on && held.len() < *capacity as usize {
                    held.push(*h);
                }
                // (In the order they are offered: a form list's is its
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

/// Switch the side to `navi` as the game does: a navi that must have a
/// level keeps the side's (from none, 0). The pieces on the board are the
/// navi's that changes form alone: another's side has none (they, its
/// board and its base HP back at the rules' defaults). The form list
/// follows the navi.
pub fn switch_navi(content: &Content, side: &mut Side, navi: NaviHandle) -> bool {
    let before = side.facts.clone();
    if side.set_navi(content, navi).is_err() {
        return false;
    }
    let level = nettai_match::level_required(content, navi).then(|| side.level(content).unwrap_or(0));
    let _ = side.set_level(content, level);
    without_forms(content, side);
    side.state_own_forms(content);
    side.facts != before
}

/// What a navi that doesn't change form has none of, back at the rules'
/// defaults on a side of one: the pieces on the board, its board, and its
/// base HP (its HP is its level's or its story's).
pub fn without_forms(content: &Content, side: &mut Side) {
    if side.stated_navi(content).is_some_and(|n| content.navi(n).forms.is_none()) {
        for fact in [crate::builds::grid::PIECES_FIELD, crate::builds::grid::SIZE_FIELD, PlayerFact::BaseHp.name()] {
            side.facts.reset(content, fact);
        }
    }
}

/// An edit of a list of definitions.
#[derive(Clone, Copy, Debug)]
pub enum ListEdit {
    /// One added at its end.
    Add(u16),
    Remove(usize),
    /// An entry a place up (true) or down.
    Move(usize, bool),
}

/// Apply `edit` to the side's list `name`: whether it changed.
pub fn list(content: &Content, side: &mut Side, name: &str, edit: ListEdit) -> bool {
    let Some(field) = facts::field(content, name) else { return false };
    let Some(Stated::List(mut items)) = side.facts.get(content, name) else { return false };
    let FieldType::Ref(registry, _) = *element(field.ty) else { return false };
    match edit {
        ListEdit::Add(h) if items.len() < room(field.ty) && !items.iter().any(|x| x.defs() == [h]) => items.push(Stated::Def(registry, Some(h))),
        ListEdit::Remove(i) if i < items.len() => {
            items.remove(i);
        }
        ListEdit::Move(i, up) => {
            let j = if up { i.checked_sub(1) } else { (i + 1 < items.len()).then_some(i + 1) };
            let Some(j) = j.filter(|_| i < items.len()) else { return false };
            items.swap(i, j);
        }
        _ => return false,
    }
    write(content, side, name, &Stated::List(items))
}

/// Write `value` as the side's fact `name`: whether it took it.
fn write(content: &Content, side: &mut Side, name: &str, value: &Stated) -> bool {
    let values = match fact_of(value) {
        Fact::List(items) => items,
        f => vec![f],
    };
    side.set_fact(content, name, &values).is_ok()
}

/// The time of entry `i` of the side's list of times `name` (its record's
/// `frames`): whether it changed.
pub fn time(content: &Content, side: &mut Side, name: &str, i: usize, frames: u16) -> bool {
    let Some(Stated::List(mut items)) = side.facts.get(content, name) else { return false };
    let Some(Stated::Record(fields)) = items.get_mut(i) else { return false };
    let Some((_, v)) = fields.iter_mut().find(|(n, _)| n == "frames") else { return false };
    if *v == Stated::Number(frames as i64) {
        return false;
    }
    *v = Stated::Number(frames as i64);
    write(content, side, name, &Stated::List(items))
}

/// An edit of the folder, by entry (from 0).
#[derive(Clone, Copy, Debug)]
pub enum FolderEdit {
    Put(usize, ChipHandle, ChipCode),
    Clear(usize),
    /// The entry made the Regular chip, or no longer.
    Regular(usize),
    /// The entry made a tag chip (its pair the next one tagged), or no
    /// longer.
    Tag(usize),
}

/// Apply `edit` to the side's folder: whether it changed.
pub fn folder(content: &Content, side: &mut Side, edit: FolderEdit) -> bool {
    let mut f = side.folder(content);
    let before = f;
    match edit {
        FolderEdit::Put(i, chip, code) if i < f.chips.len() => f.chips[i] = Some(FolderChip::new(chip, code)),
        FolderEdit::Clear(i) if i < f.chips.len() => {
            f.chips[i] = None;
            let i = i as u8;
            if f.regular == Some(i) {
                f.regular = None;
            }
            if f.tags.is_some_and(|(a, b)| a == i || b == i) {
                f.tags = None;
            }
        }
        FolderEdit::Regular(i) if f.has(i as u8) => {
            let i = i as u8;
            f.regular = if f.regular == Some(i) { None } else { Some(i) };
        }
        FolderEdit::Tag(i) if f.has(i as u8) && content.defs.fact_field(PlayerFact::TagChips).is_some() => {
            let i = i as u8;
            f.tags = match f.tags {
                Some((a, b)) if a == i || b == i => None,
                // (A tag chip waiting for its pair is tagged with itself.)
                Some((a, b)) if a == b => Some((a.min(i), a.max(i))),
                _ => Some((i, i)),
            };
        }
        _ => return false,
    }
    f != before && side.set_folder(content, &f).is_ok()
}

/// The definitions of `registry` (of collection `of`) in `game`.
pub fn of_game(content: &Content, game: &str, registry: Registry, of: Option<&str>) -> Vec<u16> {
    nettai_match::ids::all_of(content, registry, of)
        .into_iter()
        .filter(|&h| nettai_match::ids::key_of(content, registry, h).is_some_and(|k| nettai_match::ids::in_game(content, game, k)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::testing::{exe5_content, exe6_content};

    /// Facts are edited by their types alone: EXE6's version stepped
    /// through its variants (its own Crosses following it), its Crosses
    /// checked up to five in the window's order, a number stepped and
    /// kept in its type; EXE5's souls.
    #[test]
    fn facts_are_edited_by_their_types() {
        let six = exe6_content();
        let mut m = nettai_match::Match::empty(&six, "exe6").unwrap();
        let side = &mut m.sides[0];
        let crosses = |side: &Side| side.facts.get(&six, "crosses").unwrap().defs();
        let offered = facts::offered(&six, "exe6", side, &facts::field(&six, "crosses").unwrap()).unwrap();
        assert!(fact(&six, "exe6", side, "version", &FactEdit::Step(1)));
        assert_eq!(side.version(&six), Some("gregar"));
        assert_eq!(crosses(side), offered[..5], "the version's own, chosen with it");
        assert!(fact(&six, "exe6", side, "version", &FactEdit::Step(1)));
        assert_eq!((side.version(&six), crosses(side)), (Some("falzar"), offered[5..].to_vec()));
        assert!(fact(&six, "exe6", side, "crosses", &FactEdit::Empty));
        for &h in [offered[7], offered[2], offered[9], offered[0], offered[4], offered[5]].iter() {
            fact(&six, "exe6", side, "crosses", &FactEdit::Listed(h, true));
        }
        assert_eq!(crosses(side), [offered[0], offered[2], offered[4], offered[7], offered[9]], "the sixth isn't taken");
        assert!(fact(&six, "exe6", side, "version", &FactEdit::Step(-1)));
        assert_eq!(crosses(side).len(), 5, "a list of the side's own choosing stays");
        assert!(fact(&six, "exe6", side, "bug_frags", &FactEdit::Step(3)));
        assert!(!fact(&six, "exe6", side, "bug_frags", &FactEdit::Number(-1)));
        assert!(fact(&six, "exe6", side, "bug_frags", &FactEdit::Step(-10)));
        assert_eq!(side.facts.get(&six, "bug_frags"), Some(Stated::Number(0)));
        assert!(fact(&six, "exe6", side, "beast_out", &FactEdit::Step(1)));
        assert!(fact(&six, "exe6", side, "beast_out", &FactEdit::Default));
        let protoman = nettai_match::ids::navi(&six, "exe6", "protoman").unwrap();
        assert!(switch_navi(&six, side, protoman));
        assert!(crosses(side).is_empty());

        let five = exe5_content();
        let mut m = nettai_match::Match::empty(&five, "exe5").unwrap();
        let side = &mut m.sides[0];
        let offered = facts::offered(&five, "exe5", side, &facts::field(&five, "souls").unwrap()).unwrap();
        assert!(fact(&five, "exe5", side, "souls", &FactEdit::Listed(offered[3], false)));
        assert_eq!(side.facts.get(&five, "souls").unwrap().defs().len(), offered.len() - 1);
        assert!(fact(&five, "exe5", side, "karma", &FactEdit::Number(100)));
        assert!(!fact(&five, "exe5", side, "karma", &FactEdit::Number(70000)), "past a u16");
    }

    /// The folder's entries, its Regular and tag chips; a list's entries
    /// added once each, moved and taken out; a time.
    #[test]
    fn folders_lists_and_times() {
        let six = exe6_content();
        let mut m = nettai_match::Match::empty(&six, "exe6").unwrap();
        let side = &mut m.sides[0];
        let cannon = nettai_match::ids::chip(&six, "exe6", "cannon").unwrap();
        let a = six.chip(cannon).codes[0];
        assert!(folder(&six, side, FolderEdit::Put(0, cannon, a)) && folder(&six, side, FolderEdit::Put(1, cannon, a)));
        assert!(!folder(&six, side, FolderEdit::Regular(2)), "an empty entry");
        assert!(folder(&six, side, FolderEdit::Regular(1)));
        assert!(folder(&six, side, FolderEdit::Tag(0)) && folder(&six, side, FolderEdit::Tag(1)));
        assert_eq!((side.folder(&six).regular, side.folder(&six).tags), (Some(1), Some((0, 1))));
        assert!(folder(&six, side, FolderEdit::Clear(1)));
        assert_eq!((side.folder(&six).regular, side.folder(&six).tags), (None, None));
        let cards = of_game(&six, "exe6", Registry::Entry, Some("patch_cards"));
        assert!(list(&six, side, "patch_cards", ListEdit::Add(cards[0])) && list(&six, side, "patch_cards", ListEdit::Add(cards[1])));
        assert!(!list(&six, side, "patch_cards", ListEdit::Add(cards[0])), "once");
        assert!(list(&six, side, "patch_cards", ListEdit::Move(1, true)));
        assert_eq!(side.facts.get(&six, "patch_cards").unwrap().defs(), [cards[1], cards[0]]);
        assert!(!list(&six, side, "patch_cards", ListEdit::Move(0, true)));
        assert!(list(&six, side, "patch_cards", ListEdit::Remove(0)));
        assert_eq!(side.facts.get(&six, "patch_cards").unwrap().defs(), [cards[0]]);
        let Some(Stated::List(times)) = side.facts.get(&six, "sp_times") else { panic!("times") };
        assert!(!times.is_empty());
        assert!(time(&six, side, "sp_times", 0, 741) && !time(&six, side, "sp_times", 0, 741));
    }
}
