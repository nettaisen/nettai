//! A build's typed problems, in the player's words (docs/design/app-ui.md §5 rule 12, §7): every
//! `nettai_match::Wrong` has a sentence of the catalogs (the UI's `BuildWords`, build.slint), short, with its
//! numbers. A problem's own `text` is English for the log, and is never shown.
//!
//! One kind reads two ways. As what is wrong with a build ([`Words::problem`]: "2 Giga chips, limit 1"), in the
//! editor's pane whose part of the build it is about or by the ruleset's select, and after "This build can't be used:";
//! several in a line as [`Words::summary`] says them ("… +2 more"). And as why a chip can't go into the folder now
//! ([`Words::refusal`]: "Giga limit reached"), at the end of its library row and in the strip (docs/design/app-ui.md
//! §2.66).
//!
//! A problem's sentence says what it is about where the problem names a fact ([`part_of`]): "No navi
//! chosen", "No form chosen", "Something in the patch cards is there twice", "This navi isn't EXE6's". A kind
//! alone ([`Words::wrong`], [`wrong`]) has the general sentence.

use crate::builds::Kit;
use crate::builds::auto::{Holds, LISTS_TABLE};
use crate::games::Names;
use nettai_battle::content::{ChipClass, PlayerFact};
use nettai_content_api::{ChipHandle, Registry};
use nettai_match::Wrong;
use nettai_match::ruleset::Item;
use nettai_match::build::{EntryRefusal, Extent};
use nettai_match::check::Problem;
use nettai_ui::{AppWindow, BuildPart, BuildWords, ChipKind, EditorTab};
use slint::ComponentHandle;

/// A chip's class as a row colors its name and the strip words it: a class with no color of its own is colored as
/// a Standard chip's.
pub fn kind(class: ChipClass) -> ChipKind {
    match class {
        ChipClass::Mega => ChipKind::Mega,
        ChipClass::Giga => ChipKind::Giga,
        ChipClass::ProgramAdvance => ChipKind::ProgramAdvance,
        _ => ChipKind::Standard,
    }
}

/// The part of a build fact `field` is (a problem's, for its sentence): the navi, its level, the folder (and
/// its Regular and tag chips), by their roles; the forms, by the facts of forms a build chooses (EXE6's Cybeast
/// and Crosses, EXE4's and EXE5's souls); else the editor's tab whose facts include it. None: no fact, or one with
/// no part a player knows by a word.
pub fn part_of(content: &nettai_battle::Content, _builds: &nettai_match::build::Builds, field: Option<&str>) -> BuildPart {
    let Some(field) = field else { return BuildPart::None };
    match nettai_match::facts::role_of(content, field) {
        Some(PlayerFact::Navi) => return BuildPart::Navi,
        Some(PlayerFact::Level) => return BuildPart::Level,
        Some(PlayerFact::Folder | PlayerFact::RegularChip | PlayerFact::TagChips) => return BuildPart::Folder,
        _ => {}
    }
    if nettai_match::facts::form_facts(content).iter().any(|f| f.name == field) {
        return BuildPart::Forms;
    }
    match crate::screens::builds::editor::tab_of(content, field) {
        Some(EditorTab::Navicust) => BuildPart::Navicust,
        Some(EditorTab::PatchCards) => BuildPart::PatchCards,
        Some(EditorTab::AutoBattle) => BuildPart::AutoBattle,
        Some(EditorTab::Folder) => BuildPart::Folder,
        Some(EditorTab::Navi) => BuildPart::Navi,
        None => BuildPart::None,
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

    /// Item `item` of `registry` (a ruleset's: a definition, or a card's variant) by name: a variant after its
    /// definition's, its version's name ("Bass-Cross MegaMan (Gregar)").
    fn item(&self, registry: Registry, item: &Item) -> String {
        let name = self.names.def(registry, item.def);
        match &item.variant {
            Some(v) => self.say().invoke_card_variant(name.into(), self.kit.version_name(self.names, v).into()).into(),
            None => name,
        }
    }

    /// What is wrong, as a build's problem: its kind's sentence with its numbers, about the part of the build
    /// the fact it names is ([`part_of`]).
    pub fn problem(&self, problem: &Problem) -> String {
        let part = part_of(self.kit.content(), &self.kit.builds, problem.field.as_deref());
        // (A chip in a place of the auto battle data whose list the rules hold to another class: the list says why.)
        if let (Wrong::ChipNotAllowed { chip }, BuildPart::AutoBattle) = (&problem.what, part)
            && let Some(list) = problem.entry.and_then(|e| self.kit.auto_layout()?.list_of(e))
            && let Holds::Chips(class) = list.holds
            && self.kit.content().chip(*chip).class != class
        {
            let title = self.names.text(LISTS_TABLE, &list.name).unwrap_or_else(|| list.name.clone());
            return self.say().invoke_chip_not_in_list(self.names.chip(*chip).into(), title.into()).into();
        }
        self.said(&problem.what, part)
    }

    /// The sentence of what is wrong, by its kind alone, with its numbers ("2 Giga chips, limit 1": no full
    /// stop, it goes after "can't be used:" as well as on a line of its own). A kind that is about a part of the
    /// build has its general sentence here: [`Words::problem`] says the part.
    pub fn wrong(&self, wrong: &Wrong) -> String {
        self.said(wrong, BuildPart::None)
    }

    /// The sentence of `wrong`, about `part`.
    fn said(&self, wrong: &Wrong, part: BuildPart) -> String {
        let say = self.say();
        let game = || slint::SharedString::from(crate::shell::Shell::game_name(&self.kit.game));
        let chip = |c: &ChipHandle| slint::SharedString::from(self.names.chip(*c));
        let n = |v: u32| v as i32;
        let said = match wrong {
            Wrong::Match => say.invoke_invalid(BuildPart::None),
            Wrong::Invalid => say.invoke_invalid(part),
            Wrong::OtherGame => say.invoke_other_game(),
            Wrong::Unstated | Wrong::Missing => say.invoke_missing(part),
            Wrong::Foreign => say.invoke_foreign(part, game()),
            Wrong::FolderSize(c) => say.invoke_folder_size(n(c.used), n(c.limit)),
            // (A chip the auto battle data can't hold is one its navi can't play.)
            Wrong::ChipNotAllowed { chip: c } if part == BuildPart::AutoBattle => say.invoke_chip_not_playable(chip(c)),
            Wrong::ChipNotAllowed { chip: c } => say.invoke_chip_not_allowed(chip(c)),
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
            Wrong::NotTaken => say.invoke_not_taken(part),
            Wrong::Twice => say.invoke_twice(part),
            Wrong::Budget(c) => say.invoke_budget(n(c.used), n(c.limit)),
            Wrong::OffBoard => say.invoke_off_board(),
            Wrong::Overlap { .. } => say.invoke_overlap(),
            Wrong::Slot { entry, own, .. } => {
                let slot = self.kit.builds.slots.as_ref().and_then(|s| s.name(*own as usize)).map(str::to_string);
                say.invoke_wrong_slot(self.names.entry(*entry).into(), slot.unwrap_or_else(|| (own + 1).to_string()).into())
            }
            Wrong::NotAllowed { registry, item } => say.invoke_not_allowed(self.item(*registry, item).into()),
            // (By the versions alone: every thing of them the build holds is in the mix, and none is the one at fault.)
            Wrong::Mixed { groups, .. } => {
                let name = |id: &String| self.kit.version_name(self.names, id);
                let versions = match groups.as_slice() {
                    [a, b] => say.invoke_and_of(name(a).into(), name(b).into()).to_string(),
                    many => many.iter().map(name).collect::<Vec<_>>().join(", "),
                };
                say.invoke_mixed(versions.into())
            }
            Wrong::Internal => say.invoke_internal(),
        };
        said.into()
    }

    /// Why a chip can't go into the folder now (`nettai_match::folders::View::refusal`), for the end of its
    /// library row.
    pub fn refusal(&self, _chip: ChipHandle, wrong: &Wrong) -> String {
        let say = self.say();
        let said = match wrong {
            Wrong::FolderSize(_) => say.invoke_folder_full(),
            Wrong::Copies { count, .. } => say.invoke_copies_at_most(count.limit as i32),
            Wrong::Class { class, .. } => say.invoke_class_limit(kind(*class)),
            Wrong::Dark(_) => say.invoke_dark_limit(),
            Wrong::NotAllowed { .. } => say.invoke_not_allowed_here(),
            // (No other kind is a reason of a library's row: said as the problem it is.)
            other => return self.wrong(other),
        };
        said.into()
    }

    /// Why an entry can't go into a list of the build now (`nettai_match::build::Budget::refusal`: a patch
    /// card), for the end of its library row: it is in already, or what it needs.
    pub fn entry_refusal(&self, refusal: EntryRefusal) -> String {
        match refusal {
            EntryRefusal::Installed => self.say().invoke_installed().into(),
            EntryRefusal::Needs { needs, .. } => self.say().invoke_needs_megabytes(needs as i32).into(),
        }
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
    pub fn status(&self, navi: &str, problems: &[Problem]) -> (String, bool) {
        match problems.len() {
            0 => (self.say().invoke_ready(navi.into()).into(), false),
            n => (self.say().invoke_problems(navi.into(), n as i32).into(), true),
        }
    }

    /// Problems said in one line (docs/design/app-ui.md §2.66): the first's sentence, and how many more there are
    /// after it ("Mixes Gregar and Falzar · +1 more"). Wherever a build's problems are said in a line they are
    /// said so: the editor's (a pane's header, a strip's, the ruleset's select), and the reason a build can't be
    /// played (`UserError::BuildUnusable`: Play's, the lobby's, Practice's). None: there are none.
    pub fn summary<'p>(&self, problems: impl IntoIterator<Item = &'p Problem>) -> Option<String> {
        let mut problems = problems.into_iter();
        let first = self.problem(problems.next()?);
        Some(match problems.count() {
            0 => first,
            more => format!("{first} · {}", self.say().invoke_more(more as i32)),
        })
    }
}

/// What is wrong with a build of `game`, in the language shown: its problems as [`Words::summary`] says them, the
/// first's sentence of the catalogs with its numbers, the chip's and the version's names, and what it is about where
/// it names a fact ("2 Giga chips, limit 1", "Gregar is Gregar only", "No version chosen"), and "+n more". It has no
/// full stop: it goes after "This build can't be used:" or on a line of its own. For any screen, whichever is shown: a
/// lobby's reserved line (`Reason::YourBuild { problems }`). Empty where the game says nothing of its builds (the line
/// then says only that the build can't be used; why is in the log), or there are none.
pub fn problems(shell: &crate::shell::Shell, game: &str, ready: &std::rc::Rc<crate::games::Ready>, problems: &[Problem]) -> String {
    let Some(kit) = crate::builds::kit_of(game, ready) else { return String::new() };
    let ui = shell.ui();
    let graphics = ready.graphics(shell.lang);
    let names = Names::of(ready.content(), &graphics);
    Words { ui: &ui, kit: &kit, names: &names }.summary(problems).unwrap_or_default()
}

/// A kind alone said, where the problem's fact isn't at hand: a kind that is about a part of the build has its
/// general sentence ("Something isn't chosen"). Ask [`problems`] where there is a `Problem`.
#[allow(dead_code)] // (Kept for a screen that has only a kind: Play's lobby was told of it.)
pub fn wrong(shell: &crate::shell::Shell, game: &str, ready: &std::rc::Rc<crate::games::Ready>, wrong: &Wrong) -> String {
    let Some(kit) = crate::builds::kit_of(game, ready) else { return String::new() };
    let ui = shell.ui();
    let graphics = ready.graphics(shell.lang);
    let names = Names::of(ready.content(), &graphics);
    Words { ui: &ui, kit: &kit, names: &names }.wrong(wrong)
}

/// `problems` (a build's of `kit`'s game, as a list checked it: `shelf::Shelved::problems`) as [`Words::summary`] says
/// them, in the language shown: the reason of `UserError::BuildUnusable` ("This build can't be used: 2 Giga chips,
/// limit 1 · +1 more."), for a screen that starts a match with a build. None: nothing is wrong with it.
pub fn summary(shell: &crate::shell::Shell, kit: &Kit, problems: &[Problem]) -> Option<String> {
    let ui = shell.ui();
    let graphics = kit.ready.graphics(shell.lang);
    let names = Names::of(kit.content(), &graphics);
    Words { ui: &ui, kit, names: &names }.summary(problems)
}

#[cfg(test)]
mod tests {
    use super::part_of;
    use nettai_battle::content::PlayerFact;
    use nettai_match::Wrong;
    use nettai_match::build::Builds;
    use nettai_match::check::side_alone;
    use nettai_match::testing::{exe4_content, exe5_content, exe6_content};
    use nettai_ui::BuildPart;

    /// A problem that names its fact is about the part of the build the fact is, which its sentence says
    /// (the catalog's `BuildWords`: "No navi chosen", "No form chosen", "Something in the patch cards is there
    /// twice"). A side with no navi is the navi's problem; a new side of EXE6, which states no Cybeast, the forms';
    /// the folder's, the souls' and Crosses' (the forms'), a link navi's level, the NaviCust's, the patch cards' and
    /// the auto battle data's facts are theirs; a problem of no fact is of no part.
    #[test]
    fn a_problem_is_about_its_part() {
        let six = exe6_content();
        let builds = Builds::of(&six, "exe6").unwrap();
        let mut side = nettai_match::Side::fresh(&six, "exe6").unwrap();
        let first = |side: &nettai_match::Side| side_alone(&six, "exe6", side).into_iter().next().unwrap();
        let p = first(&side);
        assert_eq!((&p.what, part_of(&six, &builds, p.field.as_deref())), (&Wrong::Unstated, BuildPart::Forms), "{p:?}");
        side.facts.reset(&six, six.defs.fact_name(PlayerFact::Navi).unwrap());
        let p = first(&side);
        assert_eq!((&p.what, part_of(&six, &builds, p.field.as_deref())), (&Wrong::Missing, BuildPart::Navi), "{p:?}");
        fn part(content: &nettai_battle::Content, builds: &Builds, field: &str) -> BuildPart {
            part_of(content, builds, Some(field))
        }
        assert_eq!(part(&six, &builds, "folder"), BuildPart::Folder);
        assert_eq!(part(&six, &builds, "crosses"), BuildPart::Forms);
        assert_eq!(part(&six, &builds, "cybeast"), BuildPart::Forms);
        assert_eq!(part(&six, &builds, "navicust_programs"), BuildPart::Navicust);
        assert_eq!(part(&six, &builds, "patch_cards"), BuildPart::PatchCards);
        assert_eq!(part_of(&six, &builds, None), BuildPart::None);
        for (content, game) in [(exe5_content(), "exe5"), (exe4_content(), "exe4")] {
            let builds = Builds::of(&content, game).unwrap();
            assert_eq!(part(&content, &builds, "souls"), BuildPart::Forms, "{game}");
            assert_eq!(part(&content, &builds, "auto_battle_places"), BuildPart::AutoBattle, "{game}");
        }
        // (A link navi's level: EXE6's and EXE5's. EXE4 has none, and no such fact.)
        assert_eq!(part(&six, &builds, "level"), BuildPart::Level);
        let five = exe5_content();
        assert_eq!(part(&five, &Builds::of(&five, "exe5").unwrap(), "level"), BuildPart::Level);
    }

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
