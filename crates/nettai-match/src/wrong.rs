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
use nettai_content_api::{ChipHandle, Data, Registry};

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
    /// A fact the game's rules require is not stated (EXE6's version: none
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
    /// A chip the saves of another version of the game hold, not this
    /// side's ([`crate::build::Builds::version_with`] says whose it is).
    ChipOfAnotherVersion { chip: ChipHandle },
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
    /// Anything else no save holds.
    Invalid,

    /// The round doesn't start, or the rules stopped: nothing a player can
    /// mend. The problem's text has what happened, for the log.
    Internal,
}

impl Wrong {
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
        let typed = || -> Option<Wrong> {
            Some(match kind {
                "folder_size" => Wrong::FolderSize(count()?),
                "chip_not_allowed" => Wrong::ChipNotAllowed { chip: chip()? },
                "chip_of_another_version" => Wrong::ChipOfAnotherVersion { chip: chip()? },
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
        "chip_of_another_version",
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
        "invalid",
    ];
}
