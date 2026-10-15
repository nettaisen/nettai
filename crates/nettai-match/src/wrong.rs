//! What is wrong with a match or a side, typed: the kind of problem and its
//! numbers ([`Wrong`]), for a screen to word in its player's language. A
//! screen never shows a problem's sentence (`check::Problem::text` is for a
//! log and the terminal's tools).
//!
//! Most kinds are a game's rules' (their `validate` and `folder_check`
//! name each problem's kind and give its numbers: content/nettai/core.d.luau's
//! `SetupProblem` lists them); the rest are this crate's own checks'. The
//! kinds name nothing of one game: the folder's are the series' folder rules
//! in the engine's own words (a chip's class, its codes, the Regular and tag
//! chips), and the others say what is wrong with a fact of any setup (past
//! its range, there twice, over its budget), which the problem's field and
//! entry place.

use nettai_battle::content::{ChipClass, ChipCode, Content};
use nettai_content_api::{ChipHandle, Data, EntryHandle, Registry};

/// How much of something there is against the most there may be (a folder's
/// Mega chips against the navi's limit, a chip's MB against a memory).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Count {
    pub used: u32,
    pub limit: u32,
}

impl Count {
    /// What is left of the limit (none past it).
    pub fn free(self) -> u32 {
        self.limit.saturating_sub(self.used)
    }
}

/// What is wrong, typed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wrong {
    /// The match's game, its rounds, a stage or a background: nothing of a
    /// side's build.
    Match,
    /// The side's facts are another game's rules'.
    OtherGame,
    /// A fact the game's rules require is not stated (EXE6's Cybeast: none
    /// is assumed).
    Unstated,
    /// A definition the match's game hasn't (a navi, a chip, a form of
    /// another navi's lists).
    Foreign,

    /// The folder holds `used` chips; a folder is `limit`. (As why a chip
    /// can't be added: the folder is whole.)
    FolderSize(Count),
    /// A chip no folder holds.
    ChipNotAllowed { chip: ChipHandle },
    /// A chip in a code it doesn't come in.
    ChipCode { chip: ChipHandle, code: ChipCode },
    /// `used` copies of a chip, where a folder holds `limit`. (As why a
    /// chip can't be added: it holds `used` already.)
    Copies { chip: ChipHandle, count: Count },
    /// `used` chips of a class (Mega or Giga), past the navi's `limit`.
    Class { class: ChipClass, count: Count },
    /// `used` dark chips, past `limit`.
    Dark(Count),
    /// The Regular chip is `used` MB, and the navi's Regular memory `limit`.
    RegularMemory { chip: ChipHandle, count: Count },
    /// The tag chips are `used` MB together, and the tag memory `limit`.
    TagMemory(Count),
    /// The Regular chip is no entry with a chip.
    RegularEntry,
    /// The tag chips aren't two entries with a chip.
    TagEntries,
    /// The Regular chip is a tag chip.
    TagRegular,
    /// Tag chips, in a game whose folders have none.
    NoTags,

    /// A fact past what it may be: `value`, where it may be `least` to
    /// `most`.
    Range { value: i64, least: i64, most: i64 },
    /// A fact the side's navi takes none of (a NaviCust on a navi that
    /// compiles none, a level on a navi without levels).
    NotTaken,
    /// A fact the side's navi must state (a link navi's level).
    Missing,
    /// An entry of a list there twice, or a place taken twice.
    Twice,
    /// A list's entries cost `used` together, `limit` at most (the patch
    /// cards' MB).
    Budget(Count),
    /// A piece not on its board.
    OffBoard,
    /// A piece over the piece at entry `other` (from 0).
    Overlap { other: usize },
    /// An entry of a list whose entries go in slots sits in slot `slot`
    /// (from 0) and belongs in slot `own` (EXE4's patch card out of its
    /// catalog slot: [`crate::build::Slots`] names the slots). A build's
    /// alone: a match's checks leave it be.
    Slot { entry: EntryHandle, slot: u32, own: u32 },
    /// What the match's ruleset doesn't allow (`item` of `registry`: a chip,
    /// a navi, a form, a patch card, or a card's variant), held by the fact
    /// the problem's field names ([`crate::ruleset::Ruleset::problems`]). A
    /// build's under a ruleset alone: no game's rules say it.
    NotAllowed { registry: Registry, item: crate::ruleset::Item },
    /// Under a ruleset of one version per build, what the build holds is
    /// more than one version's (docs/design/versions.md §2.3): the groups it
    /// holds things of (`ruleset::Group` ids, in order) and every thing it
    /// holds of them, in the groups' order: none is taken for the build's
    /// own version and the rest for the mistake (the user: "if mixing and
    /// matching occurs and is not permitted you should just mark everything
    /// that's mixed and matched as an error").
    Mixed { groups: Vec<String>, items: Vec<(Registry, crate::ruleset::Item)> },
    /// Anything else no save holds.
    Invalid,

    /// The round doesn't start, or the rules stopped: nothing a player can
    /// mend. The problem's text has what happened, for the log.
    Internal,
}

impl Wrong {
    /// Whether it is the ruleset's alone ([`Wrong::NotAllowed`],
    /// [`Wrong::Mixed`]: `build::Checked::under` adds them), not the game's
    /// rules': a build has it only by the ruleset it is checked under.
    pub fn of_ruleset(&self) -> bool {
        matches!(self, Wrong::NotAllowed { .. } | Wrong::Mixed { .. })
    }

    /// A problem of the rules' (`nettai_battle::rules::Problem`: its kind's
    /// name and its table), typed. A kind this crate doesn't know, or one
    /// without its numbers, is [`Wrong::Invalid`] (the rules' sentence still
    /// says it in the log).
    pub fn of_rules(content: &Content, kind: &str, data: &Data) -> Wrong {
        let number = |name: &str| data.field(name).int();
        let count = || Some(Count { used: number("used")?.try_into().ok()?, limit: number("limit")?.try_into().ok()? });
        let chip = || match data.field("chip") {
            Data::Ref(Registry::Chip, key) => content.defs.chip_by_key(key),
            _ => None,
        };
        let entry = || match data.field("def") {
            Data::Ref(Registry::Entry, key) => content.defs.entries.iter().position(|e| e.key == *key).map(|i| EntryHandle(i as u16)),
            _ => None,
        };
        let typed = || -> Option<Wrong> {
            Some(match kind {
                "folder_size" => Wrong::FolderSize(count()?),
                "chip_not_allowed" => Wrong::ChipNotAllowed { chip: chip()? },
                "chip_code" => {
                    let Data::Str(code) = data.field("code") else { return None };
                    let mut letters = code.chars();
                    Wrong::ChipCode { chip: chip()?, code: ChipCode::from_letter(letters.next()?)? }
                }
                "copies" => Wrong::Copies { chip: chip()?, count: count()? },
                "mega" => Wrong::Class { class: ChipClass::Mega, count: count()? },
                "giga" => Wrong::Class { class: ChipClass::Giga, count: count()? },
                "dark" => Wrong::Dark(count()?),
                "regular_memory" => Wrong::RegularMemory { chip: chip()?, count: count()? },
                "tag_memory" => Wrong::TagMemory(count()?),
                "regular_entry" => Wrong::RegularEntry,
                "tag_entries" => Wrong::TagEntries,
                "tag_regular" => Wrong::TagRegular,
                "no_tags" => Wrong::NoTags,
                "range" => Wrong::Range { value: number("value")?, least: number("least")?, most: number("most")? },
                "not_taken" => Wrong::NotTaken,
                "missing" => Wrong::Missing,
                "twice" => Wrong::Twice,
                "budget" => Wrong::Budget(count()?),
                "off_board" => Wrong::OffBoard,
                "overlap" => Wrong::Overlap { other: usize::try_from(number("other")?).ok()?.checked_sub(1)? },
                "slot" => Wrong::Slot { entry: entry()?, slot: number("value")?.try_into().ok()?, own: number("own")?.try_into().ok()? },
                _ => return None,
            })
        };
        typed().unwrap_or(Wrong::Invalid)
    }

    /// The kinds of the rules' problems this crate types (what a game's
    /// rules may name a problem: core.d.luau's `SetupProblem`).
    pub const RULES_KINDS: [&'static str; 22] = [
        "folder_size",
        "chip_not_allowed",
        "chip_code",
        "copies",
        "mega",
        "giga",
        "dark",
        "regular_memory",
        "tag_memory",
        "regular_entry",
        "tag_entries",
        "tag_regular",
        "no_tags",
        "range",
        "not_taken",
        "missing",
        "twice",
        "budget",
        "off_board",
        "overlap",
        "slot",
        "invalid",
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every kind the rules may name types to its own variant when its
    /// numbers are there, and to `Invalid` when they aren't.
    #[test]
    fn every_kind_of_the_rules_is_typed() {
        let content = crate::testing::exe6_content();
        let cannon = crate::ids::chip(&content, "exe6", "cannon").unwrap();
        let whole = Data::map([
            ("chip", Data::Ref(Registry::Chip, content.defs.chip(cannon).key.clone())),
            ("code", Data::Str("B".into())),
            ("used", Data::Int(6)),
            ("limit", Data::Int(5)),
            ("value", Data::Int(9)),
            ("least", Data::Int(0)),
            ("most", Data::Int(3)),
            ("other", Data::Int(2)),
            ("def", Data::Ref(Registry::Entry, content.defs.entries[0].key.clone())),
            ("own", Data::Int(4)),
        ]);
        let count = Count { used: 6, limit: 5 };
        for kind in Wrong::RULES_KINDS {
            let typed = Wrong::of_rules(&content, kind, &whole);
            assert_eq!(typed == Wrong::Invalid, kind == "invalid", "{kind}");
        }
        assert_eq!(Wrong::of_rules(&content, "copies", &whole), Wrong::Copies { chip: cannon, count });
        assert_eq!(Wrong::of_rules(&content, "giga", &whole), Wrong::Class { class: ChipClass::Giga, count });
        assert_eq!(Wrong::of_rules(&content, "chip_code", &whole), Wrong::ChipCode { chip: cannon, code: ChipCode(1) });
        assert_eq!(Wrong::of_rules(&content, "range", &whole), Wrong::Range { value: 9, least: 0, most: 3 });
        assert_eq!(Wrong::of_rules(&content, "overlap", &whole), Wrong::Overlap { other: 1 });
        assert_eq!(Wrong::of_rules(&content, "budget", &whole), Wrong::Budget(count));
        assert_eq!(Wrong::of_rules(&content, "slot", &whole), Wrong::Slot { entry: EntryHandle(0), slot: 9, own: 4 });
        assert_eq!((count.free(), Count { used: 56, limit: 80 }.free()), (0, 24));
        // Without its numbers, or of no kind this crate knows: invalid.
        assert_eq!(Wrong::of_rules(&content, "copies", &Data::Nil), Wrong::Invalid);
        assert_eq!(Wrong::of_rules(&content, "a_new_kind", &whole), Wrong::Invalid);
    }

    /// Every problem the content's rules say is of a kind this crate types:
    /// each table of the content's Luau that has a `kind` and a `text` (a
    /// problem: no other table has both).
    #[test]
    fn the_contents_problems_are_of_known_kinds() {
        fn luau(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    luau(&path, out);
                } else if path.extension().is_some_and(|e| e == "luau") {
                    out.push(path);
                }
            }
        }
        let mut files = Vec::new();
        luau(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../content")), &mut files);
        let mut seen: Vec<String> = Vec::new();
        for file in files {
            let text = std::fs::read_to_string(&file).unwrap();
            for (at, _) in text.match_indices("kind = \"") {
                let (open, close) = (text[..at].rfind('{'), text[at..].find('}').map(|i| at + i));
                let (Some(open), Some(close)) = (open, close) else { continue };
                if !text[open..close].contains("text = ") {
                    continue;
                }
                let name = &text[at + 8..];
                let name = &name[..name.find('"').unwrap()];
                assert!(Wrong::RULES_KINDS.contains(&name), "{}: a problem of kind {name:?}, which nettai-match doesn't type", file.display());
                if !seen.iter().any(|k| k == name) {
                    seen.push(name.to_string());
                }
            }
        }
        // (The folder's kinds and the others are all said somewhere.)
        for kind in ["folder_size", "copies", "mega", "giga", "dark", "tag_memory", "range", "budget", "overlap"] {
            assert!(seen.iter().any(|k| k == kind), "no problem of kind {kind} in the content");
        }
    }
}
