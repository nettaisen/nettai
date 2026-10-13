//! A build's edits, each through the facts' own writer (`nettai_match`): an edit a fact doesn't take changes
//! nothing. The folder's ([`folder`]: a chip put in an entry or taken out, the Regular chip, the tag chips),
//! and a list of definitions' ([`list`]: the patch cards').
//!
//! What a build's version, its navi and its choices set is nettai-match's (`build::Builds`), not an edit here.

use nettai_battle::content::{ChipCode, Content, PlayerFact};
use nettai_battle::custom::FolderChip;
use nettai_battle::rules::Fact;
use nettai_content_api::{ChipHandle, FieldType, Registry};
use nettai_match::Side;
use nettai_match::facts::{self, Stated, fact_of};

/// The type of a list's element, else the type itself.
fn element(ty: &FieldType) -> &FieldType {
    match ty {
        FieldType::List(e, _) | FieldType::Array(e, _) => e,
        t => t,
    }
}

/// A list's room.
fn room(ty: &FieldType) -> usize {
    match ty {
        FieldType::List(_, n) => *n as usize,
        FieldType::Array(_, n) => *n as usize,
        _ => 0,
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

/// An edit of the folder, by entry (from 0).
#[derive(Clone, Copy, Debug)]
pub enum FolderEdit {
    Put(usize, ChipHandle, ChipCode),
    Clear(usize),
    /// The entry made the Regular chip, or no longer.
    Regular(usize),
    /// The entry made a tag chip (its pair the next one tagged), or no longer.
    Tag(usize),
}

/// The folder `edit` makes of `f`; none: it changes nothing (an entry past the folder, a mark on an empty
/// entry, a tag in a game whose folders have none).
pub fn folder_after(content: &Content, f: &nettai_match::Folder, edit: FolderEdit) -> Option<nettai_match::Folder> {
    let mut f = *f;
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
        _ => return None,
    }
    (f != before).then_some(f)
}

/// Apply `edit` to the side's folder: whether it changed.
pub fn folder(content: &Content, side: &mut Side, edit: FolderEdit) -> bool {
    folder_after(content, &side.folder(content), edit).is_some_and(|f| side.set_folder(content, &f).is_ok())
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
    use nettai_match::testing::exe6_content;

    /// The folder's entries, its Regular and tag chips; a list's entries added once each, moved and taken out.
    #[test]
    fn folders_and_lists() {
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
    }
}
