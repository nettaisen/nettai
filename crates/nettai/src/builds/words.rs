//! A build's typed problems, in the player's words (docs/design/app-ui.md §5 rule 12, §7): every
//! `nettai_match::Wrong` has a sentence of the catalogs (the UI's `BuildWords`, build.slint), short, with its
//! numbers. A problem's own `text` is English for the log, and is never shown.
//!
//! One kind reads two ways. As what is wrong with a build ([`Words::problem`]: "2 Giga chips, limit 1"), in the
//! folder editor's strip and after "This build can't be used:". And as why a chip can't go into the folder now
//! ([`Words::refusal`]: "Giga limit reached"), at the end of its library row.

use crate::builds::Kit;
use crate::games::Names;
use nettai_battle::content::ChipClass;
use nettai_content_api::ChipHandle;
use nettai_match::Wrong;
use nettai_match::build::{Checked, Extent};
use nettai_match::check::Problem;
use nettai_ui::{AppWindow, BuildWords, ChipKind};
use slint::ComponentHandle;

/// A chip's class as a row colors its name: a class with no color of its own is a Standard chip's.
pub fn kind(class: ChipClass) -> ChipKind {
    match class {
        ChipClass::Mega => ChipKind::Mega,
        ChipClass::Giga => ChipKind::Giga,
        _ => ChipKind::Standard,
    }
}

/// The catalogs' words for a game's builds, in the language shown.
pub struct Words<'a> {
    pub ui: &'a AppWindow,
    pub kit: &'a Kit,
    pub names: &'a Names<'a>,
}

impl Words<'_> {
    fn say(&self) -> BuildWords<'_> {
        self.ui.global::<BuildWords>()
    }

    /// The versions whose saves hold `chip`, by name ("Gregar"; two as "Gregar, Falzar"); none: every
    /// version's, or the rules don't say whose.
    fn holders(&self, chip: ChipHandle) -> Option<String> {
        let ids = self.kit.versions_of(chip).filter(|ids| !ids.is_empty())?;
        Some(ids.iter().map(|id| self.kit.version_name(self.names, id)).collect::<Vec<_>>().join(", "))
    }

    /// What is wrong, as a build's problem: its kind's sentence with its numbers.
    pub fn problem(&self, problem: &Problem) -> String {
        let say = self.say();
        let chip = |c: &ChipHandle| slint::SharedString::from(self.names.chip(*c));
        let n = |v: u32| v as i32;
        let said = match &problem.what {
            Wrong::Match | Wrong::Invalid => say.invoke_invalid(),
            Wrong::OtherGame => say.invoke_other_game(),
            Wrong::Unstated => say.invoke_unstated(),
            Wrong::Foreign => say.invoke_foreign(),
            Wrong::FolderSize(c) => say.invoke_folder_size(n(c.used), n(c.limit)),
            Wrong::ChipNotAllowed { chip: c } => say.invoke_chip_not_allowed(chip(c)),
            Wrong::ChipOfAnotherVersion { chip: c } => match self.holders(*c) {
                Some(version) => say.invoke_chip_only(chip(c), version.into()),
                None => say.invoke_chip_other_version(chip(c)),
            },
            Wrong::ChipCode { chip: c, code } => say.invoke_chip_code(chip(c), code.letter().to_string().into()),
            Wrong::Copies { chip: c, count } => say.invoke_copies(chip(c), n(count.used), n(count.limit)),
            Wrong::Class { class, count } => say.invoke_class_over(kind(*class), n(count.used), n(count.limit)),
            Wrong::Dark(c) => say.invoke_dark_over(n(c.used), n(c.limit)),
            Wrong::RegularMemory { chip: c, count } => say.invoke_regular_memory(chip(c), n(count.used), n(count.limit)),
            Wrong::TagMemory(c) => say.invoke_tag_memory(n(c.used), n(c.limit)),
            Wrong::RegularEntry => say.invoke_regular_entry(),
            Wrong::TagEntries => say.invoke_tag_entries(),
            Wrong::TagRegular => say.invoke_tag_regular(),
            Wrong::NoTags => say.invoke_no_tags(),
            Wrong::Range { value, least, most } => say.invoke_range(*value as i32, *least as i32, *most as i32),
            Wrong::NotTaken => say.invoke_not_taken(),
            Wrong::Missing => say.invoke_missing(),
            Wrong::Twice => say.invoke_twice(),
            Wrong::Budget(c) => say.invoke_budget(n(c.used), n(c.limit)),
            Wrong::OffBoard => say.invoke_off_board(),
            Wrong::Overlap { .. } => say.invoke_overlap(),
            Wrong::Slot { entry, own, .. } => {
                let slot = self.kit.builds.slots.as_ref().and_then(|s| s.name(*own as usize)).map(str::to_string);
                say.invoke_wrong_slot(self.names.entry(*entry).into(), slot.unwrap_or_else(|| (own + 1).to_string()).into())
            }
            Wrong::Internal => say.invoke_internal(),
        };
        said.into()
    }

    /// Why a chip can't go into the folder now (`nettai_match::folders::View::refusal`), for the end of its
    /// library row.
    pub fn refusal(&self, chip: ChipHandle, wrong: &Wrong) -> String {
        let say = self.say();
        let said = match wrong {
            Wrong::ChipOfAnotherVersion { .. } => match self.holders(chip) {
                Some(version) => say.invoke_only(version.into()),
                None => say.invoke_other_version(),
            },
            Wrong::FolderSize(_) => say.invoke_folder_full(),
            Wrong::Copies { count, .. } => say.invoke_copies_at_most(count.limit as i32),
            Wrong::Class { class, .. } => say.invoke_class_limit(kind(*class)),
            Wrong::Dark(_) => say.invoke_dark_limit(),
            // (No other kind is a reason of a library's row: said as the problem it is.)
            other => return self.problem(&Problem { side: None, field: None, entry: None, what: other.clone(), text: String::new() }),
        };
        said.into()
    }

    /// How much of a bug there is: its level, or the amount a card writes.
    pub fn extent(&self, extent: Extent) -> String {
        match extent {
            Extent::Level(level) => self.say().invoke_level(level as i32).into(),
            Extent::Amount { amount, of: Some(of) } => self.say().invoke_amount_of(amount as i32, of as i32).into(),
            Extent::Amount { amount, of: None } => amount.to_string(),
        }
    }

    /// A build's line in a list: its navi and whether it is ready ("MegaMan · ready", "MegaMan · 2 problems"),
    /// and whether that is something wrong.
    pub fn status(&self, navi: &str, checked: &Checked) -> (String, bool) {
        match checked.problems.len() {
            0 => (self.say().invoke_ready(navi.into()).into(), false),
            n => (self.say().invoke_problems(navi.into(), n as i32).into(), true),
        }
    }

    /// The editor's header's: "✓ Ready to play", or how many problems.
    pub fn header(&self, checked: &Checked) -> (String, bool) {
        match checked.problems.len() {
            0 => (self.say().invoke_ready_to_play().into(), true),
            n => (self.say().invoke_problem_count(n as i32).into(), false),
        }
    }

    /// The reason a build can't be played, for `UserError::BuildUnusable`: its first problem's sentence. None:
    /// nothing is wrong with it.
    pub fn first_problem(&self, checked: &Checked) -> Option<String> {
        checked.problems.first().map(|p| self.problem(p))
    }
}

/// The first thing wrong with `side` as a build of `kit`'s game, in the language shown: the reason of
/// `UserError::BuildUnusable` ("This build can't be used: 2 Giga chips, limit 1."), for a screen that starts a
/// match with a build. None: nothing is wrong with it.
#[allow(dead_code)] // (The screens that play a build ask it.)
pub fn first_problem(shell: &crate::shell::Shell, kit: &Kit, side: &nettai_match::Side) -> Option<String> {
    let ui = shell.ui();
    let graphics = kit.ready.graphics(shell.lang);
    let names = Names::of(kit.content(), &graphics);
    let checked = Checked::of(kit.content(), &kit.game, side);
    Words { ui: &ui, kit, names: &names }.first_problem(&checked)
}

#[cfg(test)]
mod tests {
    use nettai_match::Wrong;

    /// Every kind of problem the rules can name has a sentence here: `Words::problem` matches `Wrong` without
    /// a catch-all, so a kind added to nettai-match doesn't compile until it is worded. This holds the kinds
    /// the rules name to that list.
    #[test]
    fn every_kind_the_rules_name_is_typed() {
        let content = nettai_match::testing::exe6_content();
        for kind in Wrong::RULES_KINDS {
            let typed = Wrong::of_rules(&content, kind, &nettai_content_api::Data::Nil);
            // (Without its numbers a kind is `Invalid`, which has its sentence too.)
            assert!(matches!(typed, Wrong::Invalid | Wrong::RegularEntry | Wrong::TagEntries | Wrong::TagRegular | Wrong::NoTags | Wrong::NotTaken | Wrong::Missing | Wrong::Twice | Wrong::OffBoard), "{kind}: {typed:?}");
        }
    }
}
