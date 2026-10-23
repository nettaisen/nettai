//! Folders against their rules, which are each game's (its rules'
//! `folder_check`: EXE6's are content/exe6/rules/folder/init.luau): one
//! check that a match file, a netplay offer, the build screens and live play's
//! random folder all go through (`Battle::check_folder`). The rules read
//! the side's stats as its round set them up (the NaviCust's and the patch
//! cards' folder limits), so a check runs on that side's battle.
//!
//! What a folder can be made of is what the rules accept a chip of alone
//! (`pool`), and live play's random folder (`random_folder`) is picked one
//! chip at a time, each kept if the rules still accept the chips so far.
//!
//! A folder editor asks the rules once for a folder as it stands
//! ([`View::of`]: the limits, what the folder holds against them, and why
//! each chip of the game can't go into it now), and then reads the answer
//! for every row it draws ([`View::refusal`], a lookup).
//!
//! A match keeps a folder as it is being made ([`Folder`]): entries may be
//! empty (a new match's all are). The rules see the chips there, and say
//! that a folder of fewer than 30 is no folder (EXE6's `size`); a round is
//! played only with a whole one ([`Folder::saved`]).

use crate::pick::Picks;
use nettai_battle::Battle;
use nettai_battle::content::{ChipCode, Content};
use nettai_battle::custom::folder::FOLDER_SIZE;
use nettai_battle::custom::{FolderChip, SavedFolder};
use nettai_content_api::{ChipHandle, Data};
use std::collections::HashMap;

use crate::check::Problem;
use crate::{Count, Wrong};

/// A side's folder as a match keeps it: its 30 entries, each a chip in one
/// of its codes or empty (a folder being made), and its Regular and tag
/// chips (entries).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Folder {
    pub chips: [Option<FolderChip>; FOLDER_SIZE],
    pub regular: Option<u8>,
    pub tags: Option<(u8, u8)>,
}

impl Folder {
    /// No chips: a new match's folder.
    pub const EMPTY: Folder = Folder { chips: [None; FOLDER_SIZE], regular: None, tags: None };

    /// The folder a save holds, when every entry has a chip.
    pub fn saved(&self) -> Option<SavedFolder> {
        let chips: Vec<FolderChip> = self.chips.iter().copied().collect::<Option<_>>()?;
        Some(SavedFolder { chips: chips.try_into().ok()?, regular: self.regular, tags: self.tags })
    }

    /// The chips there, in order.
    pub fn chips(&self) -> impl Iterator<Item = FolderChip> + '_ {
        self.chips.iter().flatten().copied()
    }

    /// Whether entry `i` has a chip.
    pub fn has(&self, i: u8) -> bool {
        self.chips.get(i as usize).is_some_and(|c| c.is_some())
    }
}

impl From<SavedFolder> for Folder {
    fn from(f: SavedFolder) -> Folder {
        Folder { chips: f.chips.map(Some), regular: f.regular, tags: f.tags }
    }
}

/// A folder as its rules are asked of it: the chips there, in order, the
/// Regular and tag chips by their place among them (one on an empty entry is
/// left out: the match's own checks say so), and each chip's entry in the
/// folder by its place.
struct Asked {
    chips: Vec<FolderChip>,
    regular: Option<u8>,
    tags: Option<(u8, u8)>,
    entries: Vec<usize>,
}

impl Asked {
    fn of(folder: &Folder) -> Asked {
        let chips: Vec<FolderChip> = folder.chips().collect();
        let entries: Vec<usize> = (0..FOLDER_SIZE).filter(|&i| folder.chips[i].is_some()).collect();
        let place = |i: u8| folder.has(i).then(|| folder.chips[..i as usize].iter().flatten().count() as u8);
        let regular = folder.regular.and_then(place);
        let tags = folder.tags.and_then(|(a, b)| Some((place(a)?, place(b)?)));
        Asked { chips, regular, tags, entries }
    }

    /// The rules' problems of the folder, typed, each one's entry the
    /// folder's (a problem of the folder as a whole has none).
    fn typed(&self, content: &Content, side: u8, problems: Vec<nettai_battle::rules::Problem>) -> Vec<Problem> {
        problems
            .into_iter()
            .map(|p| {
                let mut p = Problem::of_rules(content, Some(side as usize), p);
                p.entry = p.entry.and_then(|place| self.entries.get(place).copied());
                p
            })
            .collect()
    }
}

/// What side `side`'s rules say of `folder`, each rule it breaks a problem
/// (typed, with the folder's entry it is of where it is of one; none: it
/// keeps them). `whole`: the folder is held to a folder's size too (a
/// folder being made isn't).
pub fn problems_of(b: &mut Battle, side: u8, folder: &Folder, whole: bool) -> Vec<Problem> {
    let asked = Asked::of(folder);
    let said = b.check_folder(side, &asked.chips, asked.regular, asked.tags, whole);
    let content = b.content.clone();
    asked.typed(&content, side, said)
}

/// What side `side`'s rules say of `folder`, a whole one ([`problems_of`]).
pub fn problems(b: &mut Battle, side: u8, folder: &Folder) -> Vec<Problem> {
    problems_of(b, side, folder, true)
}

/// Whether a problem says its chip is none the side's folders hold at all
/// (no folder's).
fn never(what: &Wrong) -> bool {
    matches!(what, Wrong::ChipNotAllowed { .. })
}

/// What a side's folder rules allow of a folder as it stands (the rules'
/// `folder_view`, asked once: [`View::of`]): a folder's size and the chips
/// this one holds, its Mega, Giga and dark chips against their limits (dark:
/// none in a game whose folders hold no dark chip), the Regular memory and
/// the tag memory (none in a game without tag chips), the rules it breaks
/// as it stands, and why each chip of the game can't go into it now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub size: u32,
    pub chips: u32,
    pub mega: Count,
    pub giga: Count,
    pub dark: Option<Count>,
    pub regular_memory: u32,
    pub tag_memory: Option<u32>,
    /// The rules the folder breaks as it stands (of a folder being made:
    /// its size isn't held against it), each with its entry where it is of
    /// one.
    pub problems: Vec<Problem>,
    refused: HashMap<ChipHandle, Wrong>,
}

impl View {
    /// Side `side`'s rules' view of `folder`, for the chips of `game`: one
    /// question of the rules (and one of what the folder breaks). None: the
    /// game's rules have no `folder_view`.
    pub fn of(content: &Content, game: &str, b: &mut Battle, side: u8, folder: &Folder) -> Option<View> {
        let offered: Vec<ChipHandle> =
            (0..content.defs.chips.len() as u16).map(ChipHandle).filter(|&id| crate::ids::in_game(content, game, &content.defs.chip(id).key)).collect();
        let asked = Asked::of(folder);
        let data = b.folder_view(side, &asked.chips, asked.regular, asked.tags, &offered)?;
        let number = |d: &Data| d.int().and_then(|n| u32::try_from(n).ok());
        let count = |d: &Data| Some(Count { used: number(d.field("used"))?, limit: number(d.field("limit"))? });
        let mut refused = HashMap::new();
        if let Data::List(items) = data.field("refused") {
            for p in items {
                let (Data::Str(kind), Data::Ref(_, key)) = (p.field("kind"), p.field("chip")) else { continue };
                if let Some(chip) = content.defs.chip_by_key(key) {
                    refused.insert(chip, Wrong::of_rules(content, kind, p));
                }
            }
        }
        let problems = problems_of(b, side, folder, false);
        Some(View {
            size: number(data.field("size"))?,
            chips: number(data.field("chips"))?,
            mega: count(data.field("mega"))?,
            giga: count(data.field("giga"))?,
            dark: count(data.field("dark")),
            regular_memory: number(data.field("regular_memory"))?,
            tag_memory: number(data.field("tag_memory")),
            problems,
            refused,
        })
    }

    /// Why `chip` can't go into the folder now, in any of its codes: no
    /// folder holds it ([`Wrong::ChipNotAllowed`]), the folder is whole
    /// ([`Wrong::FolderSize`]), the folder has its copies of it
    /// ([`Wrong::Copies`], `used` what it holds), or its class's limit is
    /// reached ([`Wrong::Class`], [`Wrong::Dark`]). None: it can. A lookup:
    /// cheap enough for every row of a list.
    pub fn refusal(&self, chip: ChipHandle) -> Option<&Wrong> {
        self.refused.get(&chip)
    }

    /// Whether a library lists `chip` for this side at all: every chip but
    /// those no folder holds.
    pub fn lists(&self, chip: ChipHandle) -> bool {
        !matches!(self.refused.get(&chip), Some(Wrong::ChipNotAllowed { .. }))
    }

    /// The problems of the folder that are entry `entry`'s ([`about_entry`]).
    pub fn entry_problems<'a>(&'a self, content: &Content, folder: &Folder, entry: usize) -> Vec<&'a Problem> {
        self.problems.iter().filter(|p| about_entry(content, folder, entry, p)).collect()
    }
}

/// Whether `problem`, one of `folder`'s, is entry `entry`'s: one the rules tie to it (or a ruleset: each entry of a
/// chip it doesn't allow, `check::Problem::at`); one of the folder as a whole
/// that its chip is part of (its copies, its class's limit, the dark chips', the tag memory of a tag chip, tags in a
/// game without); and of an empty entry, the folder's size, and the Regular or tag chip it is marked as. A screen says
/// it where it says the entry (a folder editor's strip, its name in the error red).
pub fn about_entry(content: &Content, folder: &Folder, entry: usize, problem: &Problem) -> bool {
    use nettai_battle::content::ChipFlags;
    let tagged = folder.tags.is_some_and(|(a, b)| a as usize == entry || b as usize == entry);
    let Some(held) = folder.chips.get(entry).copied().flatten() else {
        return match (&problem.what, problem.entry) {
            (_, Some(_)) => problem.at(entry),
            (Wrong::FolderSize(_), None) => entry < folder.chips.len(),
            (Wrong::RegularEntry, None) => folder.regular == Some(entry as u8),
            (Wrong::TagEntries, None) => tagged,
            _ => false,
        };
    };
    let record = content.chip(held.id);
    let dark = record.flags.has(ChipFlags::DARK);
    match (&problem.what, problem.entry) {
        (_, Some(_)) => problem.at(entry),
        (Wrong::Copies { chip, .. }, None) => *chip == held.id,
        (Wrong::Class { class, .. }, None) => !dark && *class == record.class,
        (Wrong::Dark(_), None) => dark,
        (Wrong::TagMemory(_) | Wrong::TagEntries | Wrong::TagRegular | Wrong::NoTags, None) => tagged,
        _ => false,
    }
}

/// What side `side`'s rules say of `folder`'s Regular and tag chips alone
/// (a folder being made): the problems of those marks, none when they
/// stand. A folder editor asks it of the folder with a mark as it would be,
/// to say whether an entry can be the Regular chip or a tag chip.
pub fn mark_problems(b: &mut Battle, side: u8, folder: &Folder) -> Vec<Problem> {
    let marks = |w: &Wrong| {
        matches!(w, Wrong::RegularMemory { .. } | Wrong::RegularEntry | Wrong::TagMemory(_) | Wrong::TagEntries | Wrong::TagRegular | Wrong::NoTags)
    };
    let mut out: Vec<Problem> = problems_of(b, side, folder, false).into_iter().filter(|p| marks(&p.what)).collect();
    // (A mark on an empty entry: the rules are asked of the chips there.)
    if folder.regular.is_some_and(|r| !folder.has(r)) {
        out.push(Problem { side: Some(side as usize), field: None, entry: None, what: Wrong::RegularEntry, text: "the Regular chip is an empty entry".into() });
    }
    if folder.tags.is_some_and(|(a, b)| !folder.has(a) || !folder.has(b)) {
        out.push(Problem { side: Some(side as usize), field: None, entry: None, what: Wrong::TagEntries, text: "a tag chip is an empty entry".into() });
    }
    out
}

/// The chips of `game` side `side`'s rules let a folder hold, in handle
/// order: those they accept alone (in their first code).
pub fn pool(content: &Content, game: &str, b: &mut Battle, side: u8) -> Vec<ChipHandle> {
    (0..content.defs.chips.len() as u16)
        .map(ChipHandle)
        .filter(|&id| crate::ids::in_game(content, game, &content.defs.chip(id).key))
        .filter(|&id| {
            let code = content.chip(id).codes.first().copied().unwrap_or(ChipCode(0));
            let said = b.check_folder(side, &[FolderChip::new(id, code)], None, None, false);
            !said.iter().any(|p| never(&Wrong::of_rules(content, &p.kind, &p.data)))
        })
        .collect()
}

/// A random folder of `game`'s chips that side `side`'s rules accept: chips
/// picked one at a time from the pool, each kept if the rules still accept
/// the chips so far
/// (the copies of a chip, the Mega and Giga limits), in one of its codes.
/// The codes lean to two the folder favors (and `*`), as a player's would,
/// so that a hand often has chips to pick together. Its Regular chip is one
/// the rules accept as Regular, if any is.
pub fn random_folder(content: &Content, game: &str, b: &mut Battle, side: u8, picks: &mut Picks) -> SavedFolder {
    let pool = pool(content, game, b, side);
    random_folder_of(content, &pool, b, side, picks)
}

/// [`random_folder`] of the chips of `pool` (some of [`pool`]'s: a
/// training's builds leave chips out of theirs).
pub fn random_folder_of(content: &Content, pool: &[ChipHandle], b: &mut Battle, side: u8, picks: &mut Picks) -> SavedFolder {
    assert!(!pool.is_empty(), "the rules let a folder hold no chip");
    let favored = [ChipCode(picks.below(26) as u8), ChipCode(picks.below(26) as u8)];
    let mut chips: Vec<FolderChip> = Vec::with_capacity(FOLDER_SIZE);
    let mut tries = 0;
    while chips.len() < FOLDER_SIZE {
        tries += 1;
        assert!(tries < 100_000, "no folder the rules accept in the content's chips");
        let id = pool[picks.below(pool.len())];
        let d = content.chip(id);
        chips.push(FolderChip::new(id, d.codes[0]));
        let kept = b.check_folder(side, &chips, None, None, false).is_empty();
        chips.pop();
        if !kept {
            continue;
        }
        let liked: Vec<ChipCode> =
            d.codes.iter().copied().filter(|c| *c == ChipCode::ASTERISK || favored.contains(c)).collect();
        let codes = if liked.is_empty() { &d.codes } else { &liked };
        let code = codes[picks.below(codes.len())];
        chips.push(FolderChip::new(id, code));
    }
    let fits: Vec<u8> =
        (0..FOLDER_SIZE as u8).filter(|&i| b.check_folder(side, &chips, Some(i), None, true).is_empty()).collect();
    let regular = (!fits.is_empty()).then(|| fits[picks.below(fits.len())]);
    SavedFolder { chips: chips.try_into().expect("30 chips"), regular, tags: None }
}

/// A folder in a line: each chip's name and code, the Regular chip marked,
/// an empty entry a dash.
pub fn describe(content: &Content, folder: &Folder) -> String {
    folder
        .chips
        .iter()
        .enumerate()
        .map(|(i, c)| match c {
            Some(c) => {
                let mark = if folder.regular == Some(i as u8) { " (Regular)" } else { "" };
                format!("{} {}{mark}", crate::names::chip(content, c.id), c.code.letter())
            }
            None => "-".into(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::exe6_content;
    use nettai_battle::content::ChipClass;

    /// A battle of the live navi on both sides, to ask EXE6's rules.
    fn rules(content: &std::sync::Arc<Content>) -> Battle {
        crate::check::set_up(content, &crate::pick::live(content, "exe6", 1, None).unwrap()).unwrap()
    }

    /// Random folders keep EXE6's folder rules, with a Regular chip that
    /// fits; the pool is the chips a save of the side's version can collect.
    #[test]
    fn random_folders_are_legal() {
        let content = exe6_content();
        let mut b = rules(&content);
        let pool = pool(&content, "exe6", &mut b, 0);
        // No Program Advance, no dark chip, none past the pack's (the
        // BeastOut chip), none of the arm chips. The Japanese
        // game's chips are folder chips (GunDelEX and Django's aren't in
        // the US games').
        let keys: Vec<&str> = pool.iter().map(|&c| nettai_content_api::keys::local(&content.defs.chip(c).key)).collect();
        assert!(keys.len() > 250, "{} chips", keys.len());
        for key in ["cannon", "roll", "gundels1", "gundelex", "areagrab", "otenko", "count", "django", "dblbeast"] {
            assert!(keys.contains(&key), "{key}");
        }
        for key in ["drksword", "beastout", "gigacan1", "batcan1", "puncharm", "synctrgr"] {
            assert!(!keys.contains(&key), "{key}");
        }
        // (The Regular chip: one within the fresh navi's Regular memory,
        // when the folder drew one.)
        let memory = b.stats[0].reg_up;
        let mut regulars = 0;
        for seed in 0..20 {
            let f = random_folder(&content, "exe6", &mut b, 0, &mut Picks::new(seed));
            assert_eq!(problems(&mut b, 0, &f.into()), Vec::new(), "seed {seed}: {}", describe(&content, &f.into()));
            if let Some(r) = f.regular {
                assert!(content.chip(f.chips[r as usize].id).mb <= memory);
                regulars += 1;
            }
        }
        assert!(regulars > 0, "no folder of twenty has a chip within the Regular memory ({memory} MB)");
        // Another seed, another folder; the same seed, the same.
        let mut one = |seed| random_folder(&content, "exe6", &mut b, 0, &mut Picks::new(seed));
        assert_ne!(one(1).chips, one(2).chips);
        assert_eq!(one(3).chips, one(3).chips);
    }

    /// What EXE6's rules refuse, each problem typed with its numbers and the
    /// folder's entry it is of.
    #[test]
    fn the_rules_refuse() {
        let content = exe6_content();
        let mut b = rules(&content);
        let id = |key: &str| content.defs.chip_by_key(key).unwrap();
        let chip = |key: &str| FolderChip::new(id(key), content.chip(id(key)).codes[0]);
        let base = |chips: [FolderChip; FOLDER_SIZE]| SavedFolder { chips, regular: None, tags: None };
        let mut said = |f: &SavedFolder| -> Vec<(Wrong, Option<usize>)> { problems(&mut b, 0, &(*f).into()).into_iter().map(|p| (p.what, p.entry)).collect() };
        // Five Recov10s (4 MB) and the rest plain chips is legal; a sixth
        // isn't.
        let mut chips = [chip("recov10"); FOLDER_SIZE];
        let mut plain = ["cannon", "airshot", "vulcan1", "spreadr1", "minibomb", "sword"].iter().cycle();
        for c in chips.iter_mut().skip(5) {
            *c = chip(plain.next().unwrap());
        }
        assert_eq!(said(&base(chips)), Vec::new());
        let mut six = chips;
        six[5] = chip("recov10");
        assert!(said(&base(six)).contains(&(Wrong::Copies { chip: id("recov10"), count: Count { used: 6, limit: 5 } }, None)));
        // A code the chip doesn't come in.
        let mut code = chips;
        code[0].code = ChipCode(25);
        assert_eq!(said(&base(code)), [(Wrong::ChipCode { chip: id("recov10"), code: ChipCode(25) }, Some(0))]);
        // Six Mega chips; two Giga chips.
        let mut megas = chips;
        for (i, key) in ["roll", "roll2", "heatman", "elecman", "slashman", "eraseman"].iter().enumerate() {
            megas[10 + i] = chip(key);
        }
        assert_eq!(said(&base(megas)), [(Wrong::Class { class: ChipClass::Mega, count: Count { used: 6, limit: 5 } }, None)]);
        let mut gigas = chips;
        gigas[10] = chip("bass");
        gigas[11] = chip("deltaray");
        assert!(said(&base(gigas)).contains(&(Wrong::Class { class: ChipClass::Giga, count: Count { used: 2, limit: 1 } }, None)));
        // A Regular chip past the Regular memory (Roll3 is 60 MB).
        let mut regular = base(chips);
        regular.chips[10] = chip("roll3");
        regular.regular = Some(10);
        assert_eq!(said(&regular), [(Wrong::RegularMemory { chip: id("roll3"), count: Count { used: 60, limit: 50 } }, Some(10))]);
        // Tag chips: two entries of 60 MB together at most, not the Regular.
        let mut tags = base(chips);
        tags.tags = Some((5, 6));
        assert_eq!(said(&tags), Vec::new());
        tags.chips[6] = chip("roll3");
        assert_eq!(said(&tags), [(Wrong::TagMemory(Count { used: 60 + content.chip(id("cannon")).mb as u32, limit: 60 }), None)]);
        tags.chips[6] = chip("airshot");
        tags.regular = Some(5);
        assert_eq!(said(&tags), [(Wrong::TagRegular, None)]);
        // A chip no folder holds, and the sentence a log gets.
        let mut dark = chips;
        dark[0] = chip("drksword");
        assert_eq!(said(&base(dark)), [(Wrong::ChipNotAllowed { chip: id("drksword") }, Some(0))]);
        let logged = problems(&mut b, 0, &base(dark).into());
        assert_eq!(logged[0].text, "entry 0: drksword is no chip a folder can hold");
    }

    /// A folder being made: an empty entry is none of the rules', and a
    /// problem's entry is the folder's, whatever is empty before it.
    #[test]
    fn a_problems_entry_is_the_folders() {
        let content = exe6_content();
        let mut b = rules(&content);
        let cannon = content.defs.chip_by_key("cannon").unwrap();
        let mut f = Folder::EMPTY;
        f.chips[7] = Some(FolderChip::new(cannon, ChipCode(25)));
        let said = problems_of(&mut b, 0, &f, false);
        assert_eq!(said.iter().map(|p| (&p.what, p.entry)).collect::<Vec<_>>(), [(&Wrong::ChipCode { chip: cannon, code: ChipCode(25) }, Some(7))]);
        // Whole, it is short of a folder's size too.
        assert!(problems(&mut b, 0, &f).iter().any(|p| p.what == Wrong::FolderSize(Count { used: 1, limit: 30 })));
    }

    /// The folder's view: the limits, what the folder holds against them,
    /// and why a chip can't go in now, read for any chip without asking the
    /// rules again.
    #[test]
    fn a_view_says_what_a_folder_can_take() {
        let content = exe6_content();
        let mut b = rules(&content);
        let id = |key: &str| content.defs.chip_by_key(key).unwrap();
        let chip = |key: &str| Some(FolderChip::new(id(key), content.chip(id(key)).codes[0]));
        let mut f = Folder::EMPTY;
        for (i, key) in ["recov10", "recov10", "recov10", "recov10", "recov10", "roll", "roll2", "heatman", "elecman", "slashman"].iter().enumerate() {
            f.chips[i] = chip(key);
        }
        // (A Giga chip: a build may hold either version's.)
        f.chips[10] = chip("bass");
        let v = View::of(&content, "exe6", &mut b, 0, &f).unwrap();
        assert_eq!((v.size, v.chips, v.mega, v.giga, v.dark), (30, 11, Count { used: 5, limit: 5 }, Count { used: 1, limit: 1 }, None));
        assert_eq!((v.regular_memory, v.tag_memory), (b.stats[0].reg_up as u32, Some(60)));
        assert_eq!(v.problems, Vec::new());
        assert_eq!(v.refusal(id("cannon")), None);
        assert_eq!(v.refusal(id("recov10")), Some(&Wrong::Copies { chip: id("recov10"), count: Count { used: 5, limit: 5 } }));
        assert_eq!(v.refusal(id("eraseman")), Some(&Wrong::Class { class: ChipClass::Mega, count: Count { used: 5, limit: 5 } }));
        assert_eq!(v.refusal(id("drksword")), Some(&Wrong::ChipNotAllowed { chip: id("drksword") }));
        assert!(!v.lists(id("drksword")) && !v.lists(id("puncharm")) && v.lists(id("cannon")));
        // A Giga chip: the folder has its one copy of it, and any other,
        // of either version, is past the Giga limit.
        assert!(v.lists(id("bass")) && matches!(v.refusal(id("bass")), Some(Wrong::Copies { .. })));
        for key in ["bassanly", "bighook", "metrknuk"] {
            assert!(v.lists(id(key)));
            assert!(matches!(v.refusal(id(key)), Some(Wrong::Class { class: ChipClass::Giga, .. })), "{key}");
        }
        // A whole folder takes none.
        let mut whole = f;
        for c in whole.chips.iter_mut().filter(|c| c.is_none()) {
            *c = chip("cannon");
        }
        let v = View::of(&content, "exe6", &mut b, 0, &whole).unwrap();
        assert_eq!(v.refusal(id("airshot")), Some(&Wrong::FolderSize(Count { used: 30, limit: 30 })));
        // (Twenty Cannons: its copies' problem is each Cannon entry's.)
        assert!(v.problems.iter().any(|p| matches!(p.what, Wrong::Copies { .. })));
        assert_eq!(v.entry_problems(&content, &whole, 29).len(), 1);
        assert_eq!(v.entry_problems(&content, &whole, 0), Vec::<&Problem>::new());
    }

    /// The Regular and tag marks, asked of the folder with a mark as it
    /// would be.
    #[test]
    fn marks_are_asked_as_they_would_be() {
        let content = exe6_content();
        let mut b = rules(&content);
        let id = |key: &str| content.defs.chip_by_key(key).unwrap();
        let chip = |key: &str| Some(FolderChip::new(id(key), content.chip(id(key)).codes[0]));
        let mut f = Folder::EMPTY;
        (f.chips[0], f.chips[1], f.chips[2]) = (chip("cannon"), chip("roll3"), chip("airshot"));
        let with = |regular: Option<u8>, tags: Option<(u8, u8)>| Folder { regular, tags, ..f };
        assert_eq!(mark_problems(&mut b, 0, &with(Some(0), None)), Vec::new());
        assert!(matches!(mark_problems(&mut b, 0, &with(Some(1), None))[0].what, Wrong::RegularMemory { .. }));
        assert_eq!(mark_problems(&mut b, 0, &with(None, Some((0, 2)))), Vec::new());
        assert!(matches!(mark_problems(&mut b, 0, &with(None, Some((0, 1))))[0].what, Wrong::TagMemory(_)));
        assert_eq!(mark_problems(&mut b, 0, &with(Some(0), Some((0, 2))))[0].what, Wrong::TagRegular);
        assert_eq!(mark_problems(&mut b, 0, &with(Some(9), None))[0].what, Wrong::RegularEntry);
        assert_eq!(mark_problems(&mut b, 0, &with(None, Some((0, 9))))[0].what, Wrong::TagEntries);
    }
}
