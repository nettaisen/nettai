//! A build's editor (docs/design/app-ui.md §6, "The editor"; docs/app.md §8): the build being edited
//! ([`Draft`]), the tabs ([`Tab`]), and the frame that hosts them ([`Editor`]: the header's name, the tab bar, the
//! ruleset's select, the tab shown).
//!
//! **A build's problems are said where they are, in one system** (docs/design/app-ui.md §2.66): each marks the tabs its
//! facts are (`Editor::tabs_of`: the tab bar's marks). Each is said at the top right of the strip (a tab's info box) of
//! every item it is about, the ruleset's (`Wrong::of_ruleset`: a banned item, a mix of versions) with the game's rules'
//! ([`Draft::line`]): an item's own, and one of a whole part of the build in each item it counts (the folder's size in
//! an empty entry, a class's limit in each chip of the class), whose count is in the error red too; one of a part that
//! is about no item and has no count, in every item of its pane ([`Draft::pane_problems`]). An item a problem is about
//! has its name in the error red on its row, and nothing more there; a strip says first why what was asked of its item
//! wasn't done (a refusal). A phone has no strip: its panes' headers say the game's rules' problems with their part
//! ([`Draft::pane_line`]). Several in one place are the first and "· +n more" (`Words::summary`).
//!
//! **A tab is one Slint file and one module here** (`ui/screens/builds/<tab>.slint`, `<tab>.rs`), as a screen
//! is one of each for the shell. Its Slint has its view (a global its module sets: its key hints and its
//! status among it) and its component, which the frame places under the tab bar. Its module has a struct that
//! implements [`Tab`]: the same few calls a `Screen` gets (`show`, `activate`, `step`, `choose`, `edit`,
//! `focused`, `key`, `button`, `action`, `back`, `answered`), each with the shell and the [`Draft`].
//!
//! **A tab changes the build through the draft** ([`Draft::edit`]: the side; [`Draft::choose_navi`]: its
//! navi). The draft is checked again at once, so the tab reads what is wrong and what the round starts with
//! straight after (`draft.checked`); and when the tab's call returns the frame saves the build's file and
//! shows the header and the tabs' marks anew. A tab neither saves nor checks.
//!
//! **The ruleset the build is made for** (docs/design/app-ui.md §2.59, §2.73; `Build::ruleset`) is the frame's too: a
//! select at the tab bar's right end (on a phone, at the top of the Navi tab) with EDIT, which opens its page over the
//! editor (`crate::rulesets::page`). The select changes the build's ruleset (and the one a new build is made for: the
//! last chosen, `rulesets::remember`); one it names that is no longer there (the player's own, deleted) is named with
//! "(missing)", and the build is checked under the game's first preset. The build is checked under it
//! (`Checked::under`), so what it doesn't allow is among
//! the build's problems, and each tab marks it where it shows it. Every tab refuses to add it (docs/design/app-ui.md
//! §2.68; the user: "okay actually rulesets should prevent adding unallowed ncps/chips/patch cards/etc"): a library's
//! row, a navi's card or a form's row says "Not allowed" in the error red, dimmed ([`Draft::allows`]), and taking it is
//! refused as the game's rules' refusals are. A build that holds it all the same (made under another ruleset, or the
//! ruleset changed since) has it as a problem about every entry that holds it, red; it can be taken out.
//!
//! **A tab says which of the build's facts are its own** ([`Tab::fields`]): a problem with one of them marks
//! its word in the bar; a tab none of whose facts the game's setup takes isn't in the bar (auto battle data:
//! some games'); and the facts of a tab that is the game's own navi's alone ([`Tab::own_navi_only`]) are kept
//! aside in the build's file while its navi is another (`crate::builds::store`), the tab staying in its place,
//! greyed.

use crate::builds::store::{self, Build};
use crate::builds::words::Words;
use crate::builds::Kit;
use crate::errors::UserError;
use crate::games::Names;
use crate::input::{Action, KeyPress};
use crate::rulesets::page::{self as rules_page, Outcome, Page as RulesPage};
use crate::rulesets::{self, MadeFor, Pickable};
use crate::shell::Shell;
use nettai_battle::Content;
use nettai_content_api::NaviHandle;
use nettai_match::Side;
use nettai_match::Wrong;
use nettai_match::build::Checked;
use nettai_match::check::Problem;
use nettai_ui::{BuildsView, EditorTab};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

/// The build being edited: what a tab is handed, with the shell.
pub struct Draft {
    /// The game's builds data, pictures and library order.
    pub kit: Rc<Kit>,
    pub build: Build,
    /// The build checked: what is wrong with it, and its round set up (the stats it starts with, the
    /// folder's view, the navi's first face). Made again at each change.
    pub checked: Checked,
    /// The ruleset the build is made for (`rulesets::made_for`), which it is checked under; none where the game
    /// says nothing of its rulesets.
    pub ruleset: Option<MadeFor>,
    /// Where the build is kept.
    path: PathBuf,
    /// The facts that are the game's own navi's alone (the tabs say which).
    navi_only: Vec<String>,
    /// It changed since it was last saved.
    changed: bool,
}

impl Draft {
    pub fn content(&self) -> &Arc<Content> {
        self.kit.content()
    }

    /// The side a match is handed.
    pub fn side(&self) -> &Side {
        &self.build.side
    }

    /// Change the build's side: `change` says whether it did. The build is checked again at once; the frame
    /// saves it when the tab's call returns.
    pub fn edit(&mut self, change: impl FnOnce(&Content, &mut Side) -> bool) -> bool {
        let kit = self.kit.clone();
        if !change(kit.content(), &mut self.build.side) {
            return false;
        }
        self.recheck();
        true
    }

    /// Choose the build's navi, as a build is (`Build::set_navi`: what is the game's own navi's alone kept
    /// aside, or put back). Whether it changed.
    pub fn choose_navi(&mut self, navi: NaviHandle) -> bool {
        let kit = self.kit.clone();
        let before = self.build.clone();
        if let Err(why) = self.build.set_navi(kit.content(), &kit.builds, navi, &self.navi_only) {
            crate::errors::log("choosing a build's navi", &why);
            return false;
        }
        if self.build == before {
            return false;
        }
        self.recheck();
        true
    }

    /// Choose form `form` for the build's fact `fact` of forms (the Navi tab's selector: `Builds::choose_form`, its
    /// Cybeast, a soul or a Cross in or out). Whether it changed.
    pub fn choose_form(&mut self, fact: &str, form: nettai_content_api::FormHandle) -> bool {
        let kit = self.kit.clone();
        self.edit(|content, side| kit.builds.choose_form(content, side, fact, form))
    }

    /// Put forms `forms` of the build's list `fact` of forms all in (`on`: ALL), or take them all out (NONE), as one
    /// change (`Builds::choose_forms`). Whether it changed, and how many were left out for want of room.
    pub fn choose_forms(&mut self, fact: &str, forms: &[nettai_content_api::FormHandle], on: bool) -> (bool, usize) {
        let kit = self.kit.clone();
        let mut left = 0;
        let changed = self.edit(|content, side| {
            let (changed, l) = kit.builds.choose_forms(content, side, fact, forms, on);
            left = l;
            changed
        });
        (changed, left)
    }

    fn recheck(&mut self) {
        self.check();
        self.changed = true;
    }

    /// Check the build as it stands, under the ruleset it is made for.
    fn check(&mut self) {
        self.checked = checked(&self.kit, &self.build.side, self.under());
    }

    /// The ruleset the build is checked under: the one it is made for, else (that one missing) the game's first
    /// preset.
    pub fn under(&self) -> Option<&Pickable> {
        self.ruleset.as_ref().map(|made| &made.pick)
    }

    /// The build's ruleset as the select names it, in the language shown, and whether it is missing.
    pub fn ruleset_shown(&self, shell: &Shell) -> (String, bool) {
        self.with_words(shell, |words| self.ruleset.as_ref().map(|made| made.shown(words.names))).unwrap_or_default()
    }

    /// Whether the build's ruleset allows definition `h` of the build's fact `fact` (a navi, a chip, a program, a
    /// soul, a Cross, a Cybeast, an auto battle place's chip): where it holds nothing of that fact back, or there is
    /// none, yes. What it doesn't allow is marked where it is offered, and refused where it would be added.
    pub fn allows(&self, fact: &str, h: u16) -> bool {
        self.under().is_none_or(|p| p.ruleset.allows(fact, h))
    }

    /// Whether it allows item `item` of the build's fact `fact` (a card's variant too: docs/design/versions.md
    /// §2.1).
    pub fn allows_item(&self, fact: &str, item: &nettai_match::ruleset::Item) -> bool {
        self.under().is_none_or(|p| p.ruleset.allows_item(fact, item))
    }

    /// Whether it allows definition `h` of the build's fact `fact` in a record of key `key` (a category kept by a
    /// field of its records: EXE5's partners' programs, by the partner's navi): neither held back for every record nor
    /// for that key's. With no key (a partner without its navi yet), what it holds back for every record alone.
    pub fn allows_for(&self, fact: &str, key: Option<u16>, h: u16) -> bool {
        let item = nettai_match::ruleset::Item::of(h);
        self.under().is_none_or(|p| match key {
            Some(key) => p.ruleset.allows_for(fact, key, &item),
            None => p.ruleset.allows_item(fact, &item),
        })
    }

    /// The build's ruleset found again among those the player can pick (one may have gone: it is missing).
    fn find_ruleset(&mut self) {
        self.ruleset = rulesets::made_for(&rulesets::pickable(&self.kit), self.build.ruleset.as_ref());
    }

    /// The build's facts problem `problem` is about: the one it names; a mix of versions (which names none), each
    /// fact that holds something of its list (the ruleset categories' that the build holds it in: the folder's Giga
    /// chips, the forms' facts, the patch cards, EXE5's partners' navis, not the build's navi, whose category has them
    /// too). None: it is about no fact (the first tab's, then).
    pub fn facts_of(&self, problem: &Problem) -> Vec<String> {
        if let Some(field) = &problem.field {
            return vec![field.clone()];
        }
        let (Wrong::Mixed { items, .. }, Some(rulesets)) = (&problem.what, self.kit.rulesets()) else { return Vec::new() };
        let content = self.content();
        rulesets
            .categories
            .iter()
            .filter(|c| {
                let of = |(registry, item): &&(nettai_content_api::Registry, nettai_match::ruleset::Item)| c.registry == *registry && c.items.contains(item);
                let mut mine = items.iter().filter(of).peekable();
                mine.peek().is_some() && {
                    let held = nettai_match::ruleset::held(content, self.side(), &c.fact);
                    mine.any(|(_, item)| held.contains(item))
                }
            })
            .map(|c| c.fact.clone())
            .collect()
    }

    /// The build's mix of versions (a ruleset of one version per build's problem) where item `item` of `registry`
    /// is among what it holds of the versions mixed, every one of which is (a Giga chip in the folder, a form, a
    /// card's variant: none is taken for the build's own version): where a tab shows the item, it is marked, and
    /// says so.
    pub fn mixing(&self, registry: nettai_content_api::Registry, item: &nettai_match::ruleset::Item) -> Option<&Problem> {
        self.checked.problems.iter().find(|p| matches!(&p.what, Wrong::Mixed { items, .. } if items.iter().any(|(r, i)| *r == registry && i == item)))
    }

    /// The problems with the build's facts `fields` (a tab's: `Tab::fields`), the ruleset's among them; the first tab's
    /// (`first`) also those of no fact a tab has.
    pub fn pane_problems(&self, fields: &[String], first: bool) -> Vec<&Problem> {
        let content = self.content();
        self.checked
            .problems
            .iter()
            .filter(|p| {
                let facts = self.facts_of(p);
                facts.iter().any(|f| fields.contains(f)) || (first && facts.iter().all(|f| tab_of(content, f).is_none()))
            })
            .collect()
    }

    /// What a phone's pane says in its header, where it has no strip (docs/design/app-ui.md §2.66), as `Words::summary`
    /// says them: the game's rules' problems among [`Draft::pane_problems`], those about an item of it (its name red on
    /// its row) with the rest; the ruleset's are said by its items' red names alone (the user: "panel header doesn't
    /// need to show ruleset violation"). Empty: none.
    pub fn pane_line(&self, shell: &Shell, fields: &[String], first: bool) -> String {
        let said: Vec<&Problem> = self.pane_problems(fields, first).into_iter().filter(|p| !p.what.of_ruleset()).collect();
        self.with_words(shell, |words| words.summary(said)).unwrap_or_default()
    }

    /// What a strip says at its top right against the item it describes, where nothing was refused (docs/design/app-ui.md
    /// §2.66): `problems`, the ruleset's among them, each once, as `Words::summary` says them. Empty: none.
    pub fn line<'p>(&self, shell: &Shell, problems: impl IntoIterator<Item = &'p Problem>) -> String {
        let mut said: Vec<&Problem> = Vec::new();
        for p in problems {
            if !said.iter().any(|s| std::ptr::eq(*s, p)) {
                said.push(p);
            }
        }
        self.with_words(shell, |words| words.summary(said)).unwrap_or_default()
    }

    /// The catalogs' and the game's words, in the language shown.
    pub fn with_words<R>(&self, shell: &Shell, f: impl FnOnce(&Words) -> R) -> R {
        let ui = shell.ui();
        let graphics = self.kit.ready.graphics(shell.i18n.code);
        let names = Names::of(self.kit.content(), &graphics);
        f(&Words { ui: &ui, kit: &self.kit, names: &names, i18n: &shell.i18n })
    }
}

/// `side` checked as a build of `kit`'s game, under `ruleset` where there is one.
fn checked(kit: &Kit, side: &Side, ruleset: Option<&Pickable>) -> Checked {
    match (ruleset, kit.rulesets()) {
        (Some(picked), Some(rulesets)) => Checked::under(kit.content(), &kit.game, side, &rulesets, &picked.ruleset),
        _ => Checked::of(kit.content(), &kit.game, side),
    }
}

/// One of the editor's tabs. Every method but [`Tab::show`] has a default that does nothing, as a
/// `Screen`'s; `id` and `index` are a control's name and index as the tab's Slint gives them.
pub trait Tab {
    /// Which tab it is: its place in the bar, and its component in the UI.
    fn id(&self) -> EditorTab;

    /// The build's facts that are the tab's own, by their setup fields' names.
    fn fields(&self, content: &Content) -> Vec<String>;

    /// Whether the tab is the game's own navi's alone (the navi that changes form: a NaviCust, patch cards
    /// and auto battle data are its).
    fn own_navi_only(&self) -> bool {
        false
    }

    /// The control the focus starts on, the first time the tab is shown.
    fn main(&self) -> &'static str;

    /// The tab is shown, or something it shows changed under it (the build, the language, the window turned
    /// upright): set the view from the draft.
    fn show(&mut self, shell: &mut Shell, draft: &mut Draft);

    /// Another tab is shown in its place, or the editor is left.
    fn hide(&mut self, _shell: &mut Shell, _draft: &mut Draft) {}

    fn activate(&mut self, _shell: &mut Shell, _draft: &mut Draft, _id: &str, _index: i32) {}

    fn step(&mut self, _shell: &mut Shell, _draft: &mut Draft, _id: &str, _index: i32, _by: i32) {}

    fn choose(&mut self, _shell: &mut Shell, _draft: &mut Draft, _id: &str, _index: i32, _choice: i32) {}

    fn edit(&mut self, _shell: &mut Shell, _draft: &mut Draft, _id: &str, _index: i32, _text: &str, _done: bool) {}

    fn focused(&mut self, _shell: &mut Shell, _draft: &mut Draft, _id: &str, _index: i32) {}

    /// The pointer came onto one of the tab's rows (`on`) or left it: the strip describes that row while the pointer
    /// is on it, and the focused one's again after (`hover`).
    fn hovered(&mut self, _shell: &mut Shell, _draft: &mut Draft, _id: &str, _index: i32, _on: bool) {}

    /// An arrow, before the focus moves by it: true if the tab took it (the NaviCust's Left and Right on a row choose
    /// its color).
    fn route(&mut self, _shell: &mut Shell, _draft: &mut Draft, _action: Action) -> bool {
        false
    }

    /// A key the tab may want for itself (a letter that acts on the focused row). True if it was the tab's.
    fn key(&mut self, _shell: &mut Shell, _draft: &mut Draft, _key: &KeyPress) -> bool {
        false
    }

    fn button(&mut self, _shell: &mut Shell, _draft: &mut Draft, _button: nettai_gamepad::Button) -> bool {
        false
    }

    /// Delete, and the pad's north button. (Previous and next are the frame's: they change the tab.)
    fn action(&mut self, _shell: &mut Shell, _draft: &mut Draft, _action: Action) -> bool {
        false
    }

    /// Esc or Back. True if the tab went back within itself (it shut something of its own).
    fn back(&mut self, _shell: &mut Shell, _draft: &mut Draft) -> bool {
        false
    }

    /// A dialog the tab asked (`Shell::ask`, by its `tag`) was answered.
    fn answered(&mut self, _shell: &mut Shell, _draft: &mut Draft, _tag: &str, _choice: usize) {}
}

// ---- CLEAR ------------------------------------------------------------------------------------------------------

/// The question a tab's CLEAR asks (docs/design/app-ui.md §2.71; the user: "you should add clear for navicust/folder/
/// patch cards/abd"): whether its part of the build is emptied. Its answer is the tab's (`Tab::answered`), CLEAR the
/// second button.
pub const CLEAR: &str = "builds.clear";

/// CLEAR, at the right end of a pane's header (`clear`), or its key: asked first whether the part of the build is
/// emptied, `question` its words, Cancel focused (the editor has no undo, as a build's DELETE asks); where the part
/// holds nothing (`empty`), refused.
pub fn ask_clear(shell: &mut Shell, question: slint::SharedString, empty: bool) {
    if empty {
        return shell.sound.play(crate::sound::Cue::Refused);
    }
    let ui = shell.ui();
    let play = ui.global::<nettai_ui::PlayView>();
    let button = |text: slint::SharedString, danger: bool| nettai_ui::DialogButton { text, kind: nettai_ui::ButtonKind::Ghost, danger };
    let clear = ui.global::<BuildsView>().get_clear_word();
    shell.ask(crate::shell::Dialog { tag: CLEAR, title: question.into(), message: String::new(), buttons: vec![button(play.get_cancel_word(), false), button(clear, true)], focus: 0, cancel: 0 });
}

/// Whether `key` is CLEAR's (X, as the tabs' hints say; not a letter typed on a text field, nor a key the menus
/// take: the one bound to L or R, which change the tab).
pub fn clear_key(shell: &Shell, key: &KeyPress) -> bool {
    key.code == slint::winit_030::winit::keyboard::KeyCode::KeyX && !key.repeat && !shell.nav.on_field() && crate::input::menu_action(key.code, key.shift, &shell.bindings).is_none()
}

/// Whether the dialog answered (`tag`, `choice`) said CLEAR.
pub fn cleared(tag: &str, choice: usize) -> bool {
    tag == CLEAR && choice == 1
}

/// The row the strip describes in place of the focused one while the pointer is on it (docs/design/app-ui.md §2.64):
/// `what`, the pointer come onto it (`on`) or gone off it, becomes `hovered` (none: the focused row again). Whether
/// it changed: the strip is shown anew. (Going from one row to the next, the next's coming may be told before the
/// last's going: a row left that isn't the one hovered changes nothing.)
pub fn hover<D: PartialEq>(hovered: &mut Option<D>, what: D, on: bool) -> bool {
    let now = match on {
        true => Some(what),
        false if hovered.as_ref() == Some(&what) => None,
        false => return false,
    };
    if *hovered == now {
        return false;
    }
    *hovered = now;
    true
}

// ---- A row dragged to another place ---------------------------------------------------------------------------

/// How near a list the pointer may be, past its edge, and still be over it (a drag let go there lands on the
/// nearest row; past it, the row goes back).
const NEAR: f32 = 16.0;

/// Where in the window a drag's pointer is: `at` as the UI's `Drag` says it (x * 65536 + y).
pub fn pointer_at(at: i32) -> (f32, f32) {
    ((at.max(0) / 65536) as f32, (at.max(0) % 65536) as f32)
}

/// Tell the UI where the row being dragged would land (`None`: nowhere, it goes back), and whether it trades places
/// with what is there.
pub fn show_drop(shell: &Shell, to: Option<usize>, swap: bool) {
    let ui = shell.ui();
    let drag = ui.global::<nettai_ui::Drag>();
    drag.set_to(to.map_or(-1, |t| t as i32));
    drag.set_swap(swap && to.is_some());
    drag.set_removing(false);
}

/// The library's rows, the list a library's row is dragged from (`Drag.list`) into the build's list beside it.
pub const LIBRARY: &str = "lib.rows";

/// Whether the drag under way is a library's row's, into the build's list: the tab is told it as `drag-in-over` and
/// `drag-in-drop` (the editor's `choose`).
fn from_library(shell: &Shell) -> bool {
    shell.ui().global::<nettai_ui::Drag>().get_list() == LIBRARY
}

/// Tell the UI where the library's row being dragged into list `into` (its name: the list that scrolls near its edges)
/// would land (`None`: nowhere, let go nothing happens), and whether it takes the place of what is there (else those
/// from there on shift to leave it a gap).
pub fn show_landing(shell: &Shell, into: &str, to: Option<usize>, swap: bool) {
    show_drop(shell, to, swap);
    shell.ui().global::<nettai_ui::Drag>().set_into(into.into());
}

/// Tell the UI the row being dragged is over the library: let go, what it holds is taken out (the remove look).
pub fn show_remove(shell: &Shell) {
    show_drop(shell, None, false);
    shell.ui().global::<nettai_ui::Drag>().set_removing(true);
}

/// Whether window point `(x, y)` is over the area the controls `ids` (each a control's first, or a list's view) take
/// together, or within `NEAR` of it.
pub fn over(shell: &Shell, ids: &[&str], (x, y): (f32, f32)) -> bool {
    let registry = shell.nav.registry();
    let registry = registry.borrow();
    let rects: Vec<crate::nav::Rect> = ids.iter().filter_map(|id| registry.view(id).or_else(|| registry.rect(&crate::nav::Key::new(id, 0)))).filter(|r| r.w > 0.0 && r.h > 0.0).collect();
    let (left, top) = rects.iter().fold((f32::MAX, f32::MAX), |(l, t), r| (l.min(r.x), t.min(r.y)));
    let (right, bottom) = rects.iter().fold((f32::MIN, f32::MIN), |(l, t), r| (l.max(r.right()), t.max(r.bottom())));
    !rects.is_empty() && x >= left - NEAR && x <= right + NEAR && y >= top - NEAR && y <= bottom + NEAR
}

/// Whether a row of the build's dragged to `at` (as the UI's `Drag` says it) is over the library beside the list, its
/// search, its sort and its rows: let go there, what it holds is taken out, as Del does ("drag it back to where it
/// came from"). Never on a phone, whose library is a sheet, shut while a row is dragged.
pub fn over_library(shell: &Shell, at: i32) -> bool {
    !shell.scale.phone && over(shell, &["lib.search", "lib.sort", "lib.direction", "lib.rows"], pointer_at(at))
}

/// The row of list `id` (a `ListView`) under `(x, y)`, of the first `rows` of its rows: past the last, the last;
/// none where the pointer is off the list.
pub fn list_row(shell: &Shell, id: &str, (x, y): (f32, f32), rows: usize) -> Option<usize> {
    let registry = shell.nav.registry();
    let registry = registry.borrow();
    let view = registry.view(id)?;
    let near = x >= view.x - NEAR && x <= view.right() + NEAR && y >= view.y - NEAR && y <= view.bottom() + NEAR;
    let row = registry.row_at(id, y.clamp(view.y, view.bottom() - 1.0))?;
    (near && rows > 0).then(|| row.clamp(0, rows as i32 - 1) as usize)
}

/// The control of `id` among `indices` nearest `(x, y)`, where the pointer is among them (within their bounds, and
/// within page `page`'s view if they are on one); none where it is off them.
pub fn nearest(shell: &Shell, id: &str, indices: impl Iterator<Item = usize>, (x, y): (f32, f32), page: Option<&str>) -> Option<usize> {
    let registry = shell.nav.registry();
    let registry = registry.borrow();
    let view = page.and_then(|p| registry.view(p));
    if let Some(view) = view
        && (x < view.x - NEAR || x > view.right() + NEAR || y < view.y - NEAR || y > view.bottom() + NEAR)
    {
        return None;
    }
    // (On a page, those scrolled out of its view aren't among them.)
    let shown = |r: &crate::nav::Rect| r.w > 0.0 && view.is_none_or(|v| r.bottom() > v.y && r.y < v.bottom());
    let rects: Vec<(usize, crate::nav::Rect)> = indices.filter_map(|i| registry.rect(&crate::nav::Key::new(id, i as i32)).filter(shown).map(|r| (i, r))).collect();
    let (left, top) = rects.iter().fold((f32::MAX, f32::MAX), |(l, t), (_, r)| (l.min(r.x), t.min(r.y)));
    let (right, bottom) = rects.iter().fold((f32::MIN, f32::MIN), |(l, t), (_, r)| (l.max(r.right()), t.max(r.bottom())));
    if x < left - NEAR || x > right + NEAR || y < top - NEAR || y > bottom + NEAR {
        return None;
    }
    // (How far the pointer is from a row: across counted twice, so a row beside it in another column is farther
    // than one above it in its own.)
    let off = |r: &crate::nav::Rect| 2.0 * (r.x - x).max(x - r.right()).max(0.0) + (r.y - y).max(y - r.bottom()).max(0.0);
    rects.iter().min_by(|a, b| off(&a.1).total_cmp(&off(&b.1))).map(|(i, _)| *i)
}

/// Every tab's module, in the bar's order.
fn all() -> Vec<Box<dyn Tab>> {
    vec![
        Box::new(super::navi::Navi::default()),
        Box::new(super::party::Party::default()),
        Box::new(super::folder::Folder::default()),
        Box::new(super::navicust::Navicust::default()),
        Box::new(super::patch_cards::PatchCards::default()),
        Box::new(super::auto_battle::AutoBattle::default()),
    ]
}

/// The facts of `tab` that the game's setup takes (a tab names every fact its view can edit; a game may take
/// only some: EXE4's NaviCust has its programs and no board to expand).
fn taken(content: &Content, tab: &dyn Tab) -> Vec<String> {
    tab.fields(content).into_iter().filter(|f| nettai_match::facts::takes(content, f)).collect()
}

/// Whether the game has tab `tab`: it names no facts of its own (the navi's, the folder's are every game's), or
/// the game's setup takes at least one of those it names.
fn has(content: &Content, tab: &dyn Tab) -> bool {
    tab.fields(content).is_empty() || !taken(content, tab).is_empty()
}

/// The build's facts that are the game's own navi's alone: those of the tabs that are (the NaviCust's, the
/// patch cards', the auto battle data's, where the game has them). What a build's file keeps aside while its
/// navi is another.
pub fn navi_only_fields(content: &Content) -> Vec<String> {
    all().iter().filter(|t| t.own_navi_only()).flat_map(|t| taken(content, t.as_ref())).collect()
}

/// The tab whose facts include fact `field`, if one's do (what a problem with it is about, for its sentence).
pub fn tab_of(content: &Content, field: &str) -> Option<EditorTab> {
    all().iter().find(|t| t.fields(content).iter().any(|f| f == field)).map(|t| t.id())
}

/// A tab's name: the focus's memory's.
fn name(tab: EditorTab) -> &'static str {
    match tab {
        EditorTab::Navi => "builds.navi",
        EditorTab::Folder => "builds.folder",
        EditorTab::Navicust => "builds.navicust",
        EditorTab::PatchCards => "builds.patch-cards",
        EditorTab::AutoBattle => "builds.auto-battle",
        EditorTab::Party => "builds.party",
    }
}

/// The editor's frame: the build, the game's tabs, and the one shown.
pub struct Editor {
    pub draft: Draft,
    tabs: Vec<Box<dyn Tab>>,
    at: usize,
    /// The ruleset's page, open over the editor.
    rules: Option<RulesPage>,
    /// The row a drag was let go over, which the pointer rests on (`hovered`).
    rest: Option<crate::nav::Key>,
}

impl Editor {
    /// Open `build` (kept at `path`) in the editor: settled as every build is, on its first tab, the Navi's,
    /// every time (the user: "don't remember the tab").
    pub fn open(shell: &mut Shell, kit: Rc<Kit>, path: PathBuf, mut build: Build) -> Editor {
        let content = kit.content().clone();
        let navi_only = navi_only_fields(&content);
        let settled = build.settle(&content, &kit.builds, &navi_only);
        let ruleset = rulesets::made_for(&rulesets::pickable(&kit), build.ruleset.as_ref());
        let checked = checked(&kit, &build.side, ruleset.as_ref().map(|made| &made.pick));
        let tabs: Vec<Box<dyn Tab>> = all().into_iter().filter(|t| has(&content, t.as_ref())).collect();
        let draft = Draft { kit, build, checked, ruleset, path, navi_only, changed: settled };
        let mut editor = Editor { draft, tabs, at: 0, rules: None, rest: None };
        editor.save(shell);
        editor
    }

    /// The tab shown, by its place in the bar.
    pub fn tab_at(&self) -> usize {
        self.at
    }

    /// Start on tab `at` in place of the first, where it can be gone to (back from Practice, on the tab its
    /// PRACTICE was pressed on). Before the editor is first shown.
    pub fn start_on(&mut self, at: usize) {
        if at < self.tabs.len() && self.enabled(at) {
            self.at = at;
        }
    }

    /// Whether tab `i` can be gone to: one that is the game's own navi's alone can't while the build's navi
    /// is another.
    fn enabled(&self, i: usize) -> bool {
        !self.tabs[i].own_navi_only() || self.draft.build.own_navi(self.draft.content())
    }

    fn tab(&mut self) -> (&mut dyn Tab, &mut Draft) {
        (self.tabs[self.at].as_mut(), &mut self.draft)
    }

    /// The tabs problem `problem` marks in the bar, in its order: its facts' tabs (a mix of versions', each tab that
    /// shows something of it: the Navi tab for a form, the Folder for a Giga chip); one of facts no tab says are its
    /// own, or of none, the first, whose strip says it of the navi (`Draft::pane_problems`; docs/design/app-ui.md §2.66).
    fn tabs_of(&self, problem: &Problem) -> Vec<usize> {
        let content = self.draft.content();
        let fields: Vec<Vec<String>> = self.tabs.iter().map(|t| t.fields(content)).collect();
        let facts: Vec<String> = self.draft.facts_of(problem).into_iter().filter(|f| fields.iter().any(|t| t.contains(f))).collect();
        match facts.is_empty() {
            true => vec![0],
            false => (0..self.tabs.len()).filter(|&i| facts.iter().any(|f| fields[i].contains(f))).collect(),
        }
    }

    /// The header and the tab bar: the build's name, the tabs, each marked where it has problems, and the ruleset.
    fn show_frame(&self, shell: &Shell) {
        let ui = shell.ui();
        let view = ui.global::<BuildsView>();
        let names = view.get_tab_names();
        let problems = &self.draft.checked.problems;
        let shown_on: Vec<Vec<usize>> = problems.iter().map(|p| self.tabs_of(p)).collect();
        let tabs: Vec<nettai_ui::Tab> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                let flagged = shown_on.iter().any(|on| on.contains(&i));
                let word = slint::Model::row_data(&names, tab.id() as usize).unwrap_or_default();
                nettai_ui::Tab { text: word, enabled: self.enabled(i), flagged }
            })
            .collect();
        let (ruleset, missing) = self.draft.ruleset_shown(shell);
        view.set_ruleset(ruleset.into());
        view.set_ruleset_missing(missing);
        view.set_name(self.draft.build.name.as_str().into());
        view.set_tabs(ModelRc::new(VecModel::from(tabs)));
        view.set_tab(self.at as i32);
        view.set_shown(self.tabs[self.at].id());
    }

    /// The editor is shown, or something changed under it: the frame, and the tab.
    pub fn show(&mut self, shell: &mut Shell) {
        shell.lock_game(true);
        self.show_frame(shell);
        let at = self.at as i32;
        let (tab, draft) = self.tab();
        shell.nav.enter(name(tab.id()), tab.main());
        // (A tab with nothing of its own to focus has the focus on its word in the bar.)
        if tab.main() == "tab" {
            shell.nav.focus("tab", at);
        }
        tab.show(shell, draft);
    }

    /// The editor is left.
    pub fn close(&mut self, shell: &mut Shell) {
        if let Some(page) = self.rules.take() {
            page.close(shell);
        }
        let (tab, draft) = self.tab();
        tab.hide(shell, draft);
        self.save(shell);
        shell.lock_game(false);
    }

    /// Keep the build if it changed, and show what follows from it: the header, the tabs' marks.
    fn save(&mut self, shell: &mut Shell) {
        if !std::mem::take(&mut self.draft.changed) {
            return;
        }
        let draft = &self.draft;
        if let Err(why) = store::write(&draft.path, draft.content(), &draft.kit.game, &draft.build) {
            crate::errors::log("keeping a build", &why);
            shell.fail(UserError::Other);
        }
    }

    /// The header's name, a field (the user: "you should just be able to edit the build name/ruleset name from the
    /// edit screen in the header"): as it is typed, as a build's file can be named; confirmed, the build is named it
    /// (numbered where another build has it) and its file named for it, the old one going once the new one is kept,
    /// Play and Practice following it; left empty, or as it was (Esc), it stays as it was.
    fn rename(&mut self, shell: &mut Shell, text: &str, done: bool) {
        let name = store::clean_name(text);
        if !done {
            shell.ui().global::<BuildsView>().set_name(name.as_str().into());
            return;
        }
        let name = name.trim_end();
        let game = self.draft.kit.game.clone();
        let from = self.draft.build.name.clone();
        if !name.is_empty() && name != from {
            let taken: Vec<String> = store::list(&game).into_iter().filter(|l| l.path != self.draft.path).map(|l| l.name).collect();
            let renamed = self.draft.build.named(&store::new_name(name, &taken));
            let to = store::renamed_path(&game, &renamed.name, &self.draft.path);
            match store::write(&to, self.draft.content(), &game, &renamed) {
                Ok(()) => {
                    if to != self.draft.path {
                        let _ = nettai_platform::fs::remove_file(&self.draft.path);
                    }
                    crate::screens::play::follow_rename(shell, &game, &from, &renamed.name);
                    self.draft.path = to;
                    self.draft.build = renamed;
                    self.draft.changed = false;
                }
                Err(why) => {
                    crate::errors::log("renaming a build", &why);
                    shell.fail(UserError::Other);
                }
            }
        }
        self.show_frame(shell);
    }

    /// After a tab's call: the build kept if it changed, and the frame shown anew.
    fn after(&mut self, shell: &mut Shell) {
        if self.draft.changed {
            self.save(shell);
            self.show_frame(shell);
        }
    }

    /// Go to tab `to`, if it can be gone to; `way` is the side it is reached from (-1: from the right).
    fn go(&mut self, shell: &mut Shell, to: usize, way: i32) {
        if to == self.at || to >= self.tabs.len() || !self.enabled(to) {
            return;
        }
        let (tab, draft) = self.tab();
        tab.hide(shell, draft);
        self.rest = None;
        self.at = to;
        shell.ui().global::<BuildsView>().set_tab_way(way);
        self.show(shell);
    }

    /// The tab before or after the one shown that can be gone to, around from the last to the first.
    fn step_tab(&mut self, shell: &mut Shell, by: i32) {
        let n = self.tabs.len() as i32;
        let mut to = self.at as i32;
        for _ in 0..n {
            to = (to + by).rem_euclid(n);
            if self.enabled(to as usize) {
                break;
            }
        }
        self.go(shell, to as usize, by.signum());
    }

    /// The build made for `picked` (remembered for the game, the ruleset a new build is made for): it is checked under
    /// it and kept, and the frame and the tab shown show its marks.
    fn make_for(&mut self, shell: &mut Shell, picked: Pickable) {
        let game = self.draft.kit.game.clone();
        rulesets::remember(shell, &game, &picked.ruleset.name);
        self.draft.build.ruleset = Some(picked.ruleset.name.clone());
        self.draft.ruleset = Some(MadeFor { pick: picked, missing: None });
        self.draft.recheck();
        self.save(shell);
        self.show_marks(shell);
    }

    /// The frame and the tab shown, after the build's ruleset changed or was found again.
    fn show_marks(&mut self, shell: &mut Shell) {
        self.show_frame(shell);
        // (Under the ruleset's page the tab is shown again as the page closes: its focus is the page's meanwhile.)
        if self.rules.is_none() {
            let (tab, draft) = self.tab();
            tab.show(shell, draft);
        }
    }

    /// The rulesets the player can pick, and which of them the build is checked under.
    fn ruleset_choices(&self) -> (Vec<Pickable>, usize) {
        let picks = rulesets::pickable(&self.draft.kit);
        let at = self.draft.under().and_then(|now| picks.iter().position(|p| p.ruleset.name == now.ruleset.name)).unwrap_or(0);
        (picks, at)
    }

    /// What the ruleset's page did: the ruleset it shows is changed, renamed or another (a copy), and is the one the
    /// build is made for; or it was deleted, and the build still names it, missing (checked under the game's first
    /// preset).
    fn ruleset_outcome(&mut self, shell: &mut Shell, outcome: Outcome) {
        match outcome {
            Outcome::Nothing => {}
            Outcome::Changed(picked) | Outcome::Renamed { to: picked, .. } => self.make_for(shell, picked),
            Outcome::Deleted => {
                self.rules = None;
                self.draft.find_ruleset();
                self.draft.check();
                self.show_marks(shell);
            }
        }
    }

    /// The ruleset's controls: the select (its list, or a step through the rulesets), and EDIT (its page).
    fn ruleset_control(&mut self, shell: &mut Shell, id: &str, index: i32, by: Option<i32>) {
        let (picks, at) = self.ruleset_choices();
        if picks.is_empty() {
            return;
        }
        match (id, by) {
            ("ruleset", Some(by)) => {
                let next = picks[(at as i32 + by).rem_euclid(picks.len() as i32) as usize].clone();
                self.make_for(shell, next);
            }
            ("ruleset", None) => {
                let names = self.draft.with_words(shell, |words| picks.iter().map(|p| rulesets::shown_name(words.names, &p.ruleset.name)).collect());
                shell.pick(id, index, names, at);
            }
            _ => {
                let Some(shown) = self.draft.under().cloned() else { return };
                let back = shell.ui().global::<nettai_ui::RulesetView>().get_editor_word().to_string();
                shell.sound.play(crate::sound::Cue::Pick);
                self.rules = RulesPage::open(shell, self.draft.kit.clone(), shown, false, &back);
            }
        }
    }

    pub fn activate(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if RulesPage::owns(id) {
            let outcome = self.rules.as_mut().map_or(Outcome::Nothing, |page| page.activate(shell, id, index));
            return self.ruleset_outcome(shell, outcome);
        }
        if id == "ruleset" || id == "ruleset-edit" {
            return self.ruleset_control(shell, id, index, None);
        }
        if id == "tab" {
            let way = if index as usize >= self.at { 1 } else { -1 };
            return self.go(shell, index.max(0) as usize, way);
        }
        let (tab, draft) = self.tab();
        tab.activate(shell, draft, id, index);
        // (A click on one of the build's own rows, an entry, a card, a slot or a place, chooses it as Enter does,
        // which takes the cursor on to the library for a chip or a card to put in it; but where the library is
        // beside it (not a phone's sheet), the focus stays on what was clicked, so the next key acts on it (R, T,
        // Del, the arrows) and a click on the library's row puts what it holds in there. The pointer down took the
        // focus there already: this keeps it from going on.)
        let moved_on = shell.nav.focused().is_some_and(|k| k.id.starts_with("lib.") && !id.starts_with("lib."));
        if shell.nav.pointing() && !shell.scale.phone && moved_on {
            shell.nav.focus(id, index);
            tab.focused(shell, draft, id, index);
        }
        self.after(shell);
    }

    pub fn step(&mut self, shell: &mut Shell, id: &str, index: i32, by: i32) {
        if RulesPage::owns(id) {
            let outcome = self.rules.as_mut().map_or(Outcome::Nothing, |page| page.step(shell, id, index, by));
            return self.ruleset_outcome(shell, outcome);
        }
        if id == "ruleset" {
            return self.ruleset_control(shell, id, index, Some(by));
        }
        let (tab, draft) = self.tab();
        tab.step(shell, draft, id, index, by);
        self.after(shell);
    }

    /// A right click (`remove`: on a row of the build's, a library's row that is in, the NaviCust's) removes what it
    /// is on, as Del there does ("sure right click should remove everywhere"); a click removes nothing ("remove that
    /// too"). A row dragged (`drag-over`, `drag-drop`: let go over the
    /// library, it is removed) is the tab's, as its other choices are; a library's row dragged into the build's list
    /// is told it as `drag-in-over` and `drag-in-drop` ("you should allow drag and dropping from the library into a
    /// slot"), `index` the library's row.
    pub fn choose(&mut self, shell: &mut Shell, id: &str, index: i32, choice: i32) {
        let id = match id {
            "drag-over" if from_library(shell) => "drag-in-over",
            "drag-drop" if from_library(shell) => "drag-in-drop",
            id => id,
        };
        if matches!(id, "drag-drop" | "drag-in-drop") {
            self.rest = shell.nav.registry().borrow().at(pointer_at(choice));
        }
        if RulesPage::owns(id) {
            let outcome = self.rules.as_mut().map_or(Outcome::Nothing, |page| page.choose(shell, id, index, choice));
            return self.ruleset_outcome(shell, outcome);
        }
        // (A ruleset chosen from the select's list.)
        if id == "ruleset" {
            let (picks, _) = self.ruleset_choices();
            if let Some(picked) = picks.into_iter().nth(choice.max(0) as usize) {
                self.make_for(shell, picked);
            }
            return;
        }
        if id == "remove" {
            let (tab, draft) = self.tab();
            tab.action(shell, draft, Action::Delete);
        } else {
            let (tab, draft) = self.tab();
            tab.choose(shell, draft, id, index, choice);
        }
        self.after(shell);
    }

    pub fn edit(&mut self, shell: &mut Shell, id: &str, index: i32, text: &str, done: bool) {
        if RulesPage::owns(id) {
            let outcome = self.rules.as_mut().map_or(Outcome::Nothing, |page| page.edit(shell, id, text, done));
            return self.ruleset_outcome(shell, outcome);
        }
        if id == "name" {
            return self.rename(shell, text, done);
        }
        let (tab, draft) = self.tab();
        tab.edit(shell, draft, id, index, text, done);
        self.after(shell);
    }

    /// The pointer on one of the tab's rows, or gone off it (a ruleset's page has no strip).
    /// (The row a drag was let go over isn't one the pointer went to: the strip keeps the focused row, and why the
    /// drop was refused, until the pointer goes on to another.)
    pub fn hovered(&mut self, shell: &mut Shell, id: &str, index: i32, on: bool) {
        if self.rules.is_some() {
            return;
        }
        let key = crate::nav::Key::new(id, index);
        if self.rest.as_ref() == Some(&key) {
            if on {
                return;
            }
            self.rest = None;
        } else if on {
            self.rest = None;
        }
        let (tab, draft) = self.tab();
        tab.hovered(shell, draft, id, index, on);
    }

    pub fn focused(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if let Some(page) = self.rules.as_mut().filter(|_| RulesPage::owns(id)) {
            return page.focused(shell, id, index);
        }
        let (tab, draft) = self.tab();
        tab.focused(shell, draft, id, index);
        self.after(shell);
    }

    pub fn route(&mut self, shell: &mut Shell, action: Action) -> bool {
        if self.rules.is_some() {
            return false;
        }
        let (tab, draft) = self.tab();
        tab.route(shell, draft, action)
    }

    pub fn key(&mut self, shell: &mut Shell, key: &KeyPress) -> bool {
        if self.rules.is_some() {
            return false;
        }
        let (tab, draft) = self.tab();
        let taken = tab.key(shell, draft, key);
        self.after(shell);
        taken
    }

    pub fn button(&mut self, shell: &mut Shell, button: nettai_gamepad::Button) -> bool {
        if self.rules.is_some() {
            return false;
        }
        let (tab, draft) = self.tab();
        let taken = tab.button(shell, draft, button);
        self.after(shell);
        taken
    }

    /// Previous and next change the tab; the rest is the tab's.
    pub fn action(&mut self, shell: &mut Shell, action: Action) -> bool {
        // (Over the ruleset's page the shoulders change nothing under it.)
        if self.rules.is_some() {
            return true;
        }
        match action {
            Action::Previous => self.step_tab(shell, -1),
            Action::Next => self.step_tab(shell, 1),
            _ => {
                let (tab, draft) = self.tab();
                let taken = tab.action(shell, draft, action);
                self.after(shell);
                return taken;
            }
        }
        true
    }

    /// Esc or Back: true if the ruleset's page closed, or the tab went back within itself.
    pub fn back(&mut self, shell: &mut Shell) -> bool {
        if let Some(page) = self.rules.take() {
            page.close(shell);
            let (tab, draft) = self.tab();
            tab.show(shell, draft);
            return true;
        }
        let (tab, draft) = self.tab();
        let within = tab.back(shell, draft);
        self.after(shell);
        within
    }

    pub fn answered(&mut self, shell: &mut Shell, tag: &str, choice: usize) {
        if tag == rules_page::DELETE {
            if let Some(page) = self.rules.take() {
                match page.answered(shell, choice) {
                    Ok(outcome) => self.ruleset_outcome(shell, outcome),
                    Err(page) => self.rules = Some(page),
                }
            }
            return;
        }
        let (tab, draft) = self.tab();
        tab.answered(shell, draft, tag, choice);
        self.after(shell);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::testing::{exe4_content, exe5_content, exe6_content};

    /// Each game has the tabs its setup has something for: a NaviCust and patch cards in all three (EXE4's
    /// NaviCust has its programs and no board to expand: a tab is there for any of the facts it names), auto
    /// battle data in EXE4 and EXE5, a party in EXE5 (EXE5 DS's party battle). What is the game's own navi's alone,
    /// and kept aside for another, is those tabs' facts that the game takes.
    #[test]
    fn each_game_has_its_tabs() {
        let tabs = |content: &Content| -> Vec<EditorTab> { all().iter().filter(|t| has(content, t.as_ref())).map(|t| t.id()).collect() };
        let four = [EditorTab::Navi, EditorTab::Folder, EditorTab::Navicust, EditorTab::PatchCards];
        let five = [EditorTab::Navi, EditorTab::Folder, EditorTab::Navicust, EditorTab::PatchCards, EditorTab::AutoBattle];
        let six = [EditorTab::Navi, EditorTab::Party, EditorTab::Folder, EditorTab::Navicust, EditorTab::PatchCards, EditorTab::AutoBattle];
        assert_eq!(tabs(&exe6_content()), four);
        assert_eq!(tabs(&exe5_content()), six);
        assert_eq!(tabs(&exe4_content()), five);
        assert_eq!(navi_only_fields(&exe6_content()), ["navicust_programs", "navicust_expansions", "patch_cards"]);
        assert_eq!(navi_only_fields(&exe5_content()), ["party", "navicust_programs", "navicust_expansions", "patch_cards", "auto_battle_places"]);
        assert_eq!(navi_only_fields(&exe4_content()), ["navicust_programs", "patch_cards", "auto_battle_places"]);
    }
}
