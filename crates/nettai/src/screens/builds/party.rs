//! The Party tab of a build's editor, in a game whose setup takes a party (EXE5's party battle, EXE5 DS's:
//! docs/design/exe5ds-map.md §4, §11): the navi's partners, each with its PartyCust programs filling its gauge (§4.10).
//!
//! **Its form is the setup's schema's**: the field is a list of records (its room the most partners), each with a field
//! naming a navi and a list of entries of a collection (the programs; its room the most a partner holds). The tab reads
//! those from the schema and names neither. What may be a partner and what fits a gauge are the game's rules': the
//! navis a side can play but its own are offered, the build's problems say what the rules refuse (a navi twice, one
//! that is no team navi, programs past the gauge), and the gauge itself is the rules' overview's (a list budget of the
//! partner's entry: `Overview::budget_of`), as is a program's cost (the field the budget names). A partner's HP is the
//! round's as the build's check sets it up (`Checked::members`), its programs' bonus in.
//!
//! **The layout is the user's** (who found a tree of partners and programs with one library for both "confusing"):
//! two partner cards across the top, always there (an empty one "Add a partner"), the focused one the partner whose
//! PartyCust is below: its gauge, INSTALLED (its programs) and PROGRAMS (the library). **Enter on a card** opens the
//! navis to pick from (the shell's marked list, a phone's sheet): the partner's own checked, one the build's ruleset
//! holds back (its `party` category) or the other partner's dimmed and refused, and REMOVE, which takes the partner out
//! with its programs. **Enter in PROGRAMS** installs the program at the end of the partner's (a program may be in twice:
//! the rules hold them to the gauge alone), refused where the gauge or the list has no room ("Needs 5 blocks");
//! **Enter or Del in INSTALLED** takes it out; Del on a card takes the partner out; **CLEAR** (or X) takes every partner
//! out, asked first. Each block is in its program's color, a NaviCust program's (its definition's `color`). A program
//! the build's ruleset holds back (its category over the partners' programs, `party.programs`) is greyed in PROGRAMS
//! with "Not allowed" and refused; one installed all the same says so, red, and can be taken out.

use crate::input::{Action, KeyPress};
use crate::screens::builds::editor::{self, Draft, Tab};
use crate::shell::Shell;
use crate::sound::Cue;
use nettai_battle::Content;
use crate::builds::party::{FIELD, Partner, Shape, partners, tone, write};
use nettai_content_api::{EntryHandle, NaviHandle, Registry};
use nettai_match::build::{Budget, EntryRefusal};
use nettai_match::check::Problem;
use nettai_ui::{BuildWords, EditorTab, PartnerCard, PartyDetail, PartyRow, PartyView, PickMark};
use slint::{ComponentHandle, ModelRc, VecModel};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Described {
    Card(usize),
    Installed(usize),
    Row(usize),
}

#[derive(Default)]
pub struct Party {
    shape: Option<Shape>,
    /// The card chosen: the partner whose PartyCust is shown.
    chosen: usize,
    /// The navis offered, and what the navi picker's rows are (none: REMOVE).
    navis: Vec<NaviHandle>,
    picking: Vec<Option<NaviHandle>>,
    /// The library's programs, in the game's order.
    programs: Vec<EntryHandle>,
    described: Option<Described>,
    hovered: Option<Described>,
    refused: i32,
    /// Why the last thing asked wasn't done, until the cursor moves.
    notice: Option<String>,
}

impl Party {
    /// The party's shape and the library's programs, read the first time; the navis offered.
    fn read(&mut self, draft: &Draft) -> Option<Shape> {
        if self.shape.is_none() {
            let content = draft.content();
            let shape = Shape::of(content)?;
            let mut programs: Vec<((), EntryHandle)> = crate::builds::edit::of_game(content, &draft.kit.game, Registry::Entry, Some(&shape.collection)).into_iter().map(|h| ((), EntryHandle(h))).collect();
            draft.kit.order.entries(content, &shape.collection, &mut programs);
            self.programs = programs.into_iter().map(|(_, h)| h).collect();
            self.shape = Some(shape);
        }
        // (The navis a side can play but the build's own, in the game's order.)
        let content = draft.content();
        let own = draft.side().navi(content);
        self.navis = nettai_match::navis(content, &draft.kit.game).into_iter().filter(|&n| n != own).collect();
        self.shape.clone()
    }

    fn partners(&self, draft: &Draft) -> Vec<Partner> {
        self.shape.as_ref().map(|s| partners(draft.content(), draft.side(), s)).unwrap_or_default()
    }

    /// Each partner's gauge, by its entry: the rules' overview's budget of it.
    fn budgets(draft: &mut Draft) -> Vec<Option<Budget>> {
        let content = draft.content().clone();
        let side = draft.side().clone();
        let overview = draft.checked.overview(&content, &side);
        crate::builds::party::budgets(&content, &side, overview.as_ref())
    }

    /// A partner's HP as its round starts: its member's, as the build's check sets the round up.
    fn hp(draft: &Draft, navi: NaviHandle) -> Option<u16> {
        crate::builds::party::hp(&draft.checked, navi)
    }

    /// The build's problems with partner `i` (its own, and the party's about no partner).
    fn problems(draft: &Draft, i: usize) -> Vec<&Problem> {
        draft.checked.problems.iter().filter(|p| draft.facts_of(p).iter().any(|f| f == FIELD) && (p.at(i) || p.entry.is_none())).collect()
    }

    /// The ruleset category of the partners' programs: the party's field and its records' programs' field, by their
    /// path (`party.programs`: nettai-match's categories over a field of a list's records).
    fn programs_category(&self) -> String {
        format!("{FIELD}.{}", self.shape.as_ref().map_or("", |s| s.programs.as_str()))
    }

    /// Why program `h` can't go into the chosen partner's now (the build's ruleset, its gauge), if it can't.
    fn refusal(&self, shell: &Shell, draft: &Draft, budget: Option<&Budget>, h: EntryHandle) -> Option<String> {
        let ui = shell.ui();
        if !draft.allows(&self.programs_category(), h.0) {
            return Some(ui.global::<BuildWords>().invoke_not_allowed_here().into());
        }
        if let Some(EntryRefusal::Needs { needs, .. }) = budget.and_then(|b| b.refusal(draft.content(), draft.side(), h)) {
            return Some(ui.global::<PartyView>().invoke_needs_blocks(needs as i32).into());
        }
        None
    }

    /// Everything shown anew.
    fn changed(&mut self, shell: &Shell, draft: &mut Draft) {
        let Some(shape) = self.read(draft) else { return };
        let content = draft.content().clone();
        let partners = partners(&content, draft.side(), &shape);
        let budgets = Party::budgets(draft);
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        let pictures = &draft.kit.pictures;
        let budget = |i: usize| budgets.get(i).cloned().flatten();
        let cards: Vec<PartnerCard> = draft.with_words(shell, |words| {
            (0..shape.room)
                .map(|i| {
                    let Some(p) = partners.get(i) else { return PartnerCard::default() };
                    let b = budget(i);
                    // (Each block its program's color, the programs in their order.)
                    let tones = crate::builds::party::tones(&content, p, b.as_ref());
                    PartnerCard {
                        there: true,
                        face: p.navi.map(|n| pictures.navi(n)).unwrap_or_default(),
                        name: p.navi.map(|n| words.names.navi(n)).unwrap_or_default().into(),
                        hp: p.navi.and_then(|n| Party::hp(draft, n)).map(|hp| view.invoke_hp_word(hp as i32)).unwrap_or_default(),
                        used: b.as_ref().map_or(0, |b| b.count.used as i32),
                        gauge: b.as_ref().map_or(0, |b| b.count.limit as i32),
                        tones: ModelRc::new(VecModel::from(tones)),
                        bad: !Party::problems(draft, i).is_empty(),
                    }
                })
                .collect()
        });
        self.chosen = self.chosen.min(shape.room.saturating_sub(1));
        let chosen = partners.get(self.chosen);
        let b = budget(self.chosen);
        let (title, installed, rows) = draft.with_words(shell, |words| {
            let title = chosen.and_then(|p| p.navi).map(|n| view.invoke_partycust_of(words.names.navi(n).into())).unwrap_or_default();
            let row = |h: EntryHandle| PartyRow { name: words.names.entry(h).into(), blocks: b.as_ref().map_or(0, |b| b.cost_of(&content, h)) as i32, tone: tone(&content, h), ..Default::default() };
            // (One the build's ruleset holds back all the same says so, red: it can be taken out.)
            let category = self.programs_category();
            let installed: Vec<PartyRow> = chosen
                .map(|p| {
                    p.programs
                        .iter()
                        .map(|&h| {
                            let mut r = row(h);
                            if !draft.allows(&category, h.0) {
                                r.note = ui.global::<BuildWords>().invoke_not_allowed_here();
                                r.bad = true;
                            }
                            r
                        })
                        .collect()
                })
                .unwrap_or_default();
            let rows: Vec<PartyRow> = self
                .programs
                .iter()
                .map(|&h| {
                    let mut r = row(h);
                    if let Some(why) = self.refusal(shell, draft, b.as_ref(), h) {
                        r.note = why.into();
                        r.refused = true;
                        r.bad = true;
                    }
                    r
                })
                .collect();
            (title, installed, rows)
        });
        view.set_partners(partners.len() as i32);
        view.set_cards(ModelRc::new(VecModel::from(cards)));
        view.set_chosen(self.chosen as i32);
        view.set_title(title);
        view.set_installed(ModelRc::new(VecModel::from(installed)));
        view.set_rows(ModelRc::new(VecModel::from(rows)));
        view.set_problems(draft.pane_line(shell, &[FIELD.to_string()], false).into());
        self.show_detail(shell, draft);
    }

    /// The strip: what is under the cursor (or the pointer): a partner's navi, or a program.
    fn show_detail(&self, shell: &Shell, draft: &mut Draft) {
        let content = draft.content().clone();
        let partners = self.partners(draft);
        let budgets = Party::budgets(draft);
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        let chosen = partners.get(self.chosen);
        let b = budgets.get(self.chosen).cloned().flatten();
        let notice = if self.hovered.is_some_and(|h| Some(h) != self.described) { None } else { self.notice.clone() };
        let fault = |i: usize| draft.line(shell, Party::problems(draft, i));
        let detail = draft.with_words(shell, |words| {
            let program = |h: EntryHandle| {
                let cost = b.as_ref().map_or(0, |b| b.cost_of(&content, h));
                PartyDetail {
                    there: true,
                    name: words.names.entry(h).into(),
                    blocks: cost as i32,
                    tone: tone(&content, h),
                    facts: view.invoke_blocks_word(cost as i32),
                    text: words.names.description(h).map(|d| crate::games::one_line(&d)).unwrap_or_default().into(),
                    ..Default::default()
                }
            };
            let mut detail = match self.hovered.or(self.described) {
                Some(Described::Card(i)) => match partners.get(i).and_then(|p| p.navi) {
                    Some(n) => {
                        let b = budgets.get(i).cloned().flatten();
                        let facts: Vec<String> = [Party::hp(draft, n).map(|hp| view.invoke_hp_word(hp as i32).to_string()), b.map(|b| format!("{}/{}", b.count.used, b.count.limit))].into_iter().flatten().collect();
                        PartyDetail { there: true, face: draft.kit.pictures.navi(n), name: words.names.navi(n).into(), facts: facts.join(" · ").into(), fault: fault(i).into(), ..Default::default() }
                    }
                    None => PartyDetail::default(),
                },
                Some(Described::Installed(k)) => match chosen.and_then(|p| p.programs.get(k)) {
                    Some(&h) => PartyDetail { fault: fault(self.chosen).into(), ..program(h) },
                    None => PartyDetail::default(),
                },
                Some(Described::Row(r)) => match self.programs.get(r) {
                    Some(&h) => PartyDetail { problem: self.refusal(shell, draft, b.as_ref(), h).unwrap_or_default().into(), ..program(h) },
                    None => PartyDetail::default(),
                },
                None => PartyDetail::default(),
            };
            if let Some(n) = &notice {
                detail.problem = n.as_str().into();
            }
            detail
        });
        view.set_detail(detail);
    }

    /// Change the build's partners: whether they changed.
    fn change(&self, draft: &mut Draft, change: impl FnOnce(&mut Vec<Partner>) -> bool) -> bool {
        let Some(shape) = self.shape.clone() else { return false };
        draft.edit(|content, side| {
            let mut ps = partners(content, side, &shape);
            change(&mut ps) && write(content, side, &shape, &ps)
        })
    }

    /// Refuse what was asked of library row `row`, saying `why`: the row shakes.
    fn refuse(&mut self, shell: &mut Shell, draft: &mut Draft, row: usize, why: String) {
        shell.sound.play(Cue::Refused);
        self.refused += 1;
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        view.set_refused(row as i32);
        view.set_refused_seq(self.refused);
        self.notice = Some(why);
        self.show_detail(shell, draft);
    }

    /// Open the navis to pick from for card `i`: the partner's own checked, those that can't be it dimmed with why, and
    /// REMOVE where there is a partner.
    fn pick_navi(&mut self, shell: &mut Shell, draft: &Draft, i: usize) {
        let partners = self.partners(draft);
        let mine = partners.get(i).and_then(|p| p.navi);
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        let say = ui.global::<BuildWords>();
        let mut rows: Vec<(String, PickMark)> = draft.with_words(shell, |words| {
            self.navis
                .iter()
                .map(|&n| {
                    let other = mine != Some(n) && partners.iter().any(|p| p.navi == Some(n));
                    let allowed = draft.allows(FIELD, n.0);
                    let value = if !allowed { say.invoke_not_allowed_here() } else if other { view.invoke_in_party() } else { Default::default() };
                    (words.names.navi(n), PickMark { check: mine == Some(n), dim: !allowed || other, line: false, value })
                })
                .collect()
        });
        self.picking = self.navis.iter().map(|&n| Some(n)).collect();
        if mine.is_some() {
            rows.push((view.get_remove_word().into(), PickMark { line: true, ..Default::default() }));
            self.picking.push(None);
        }
        shell.pick_marked("navi", i as i32, &crate::nav::Key::new("partner", i as i32), rows);
    }

    /// The navi picker's row `choice` for card `i`: the navi made the partner (a new partner where the card had none),
    /// or (REMOVE) the partner taken out.
    fn picked(&mut self, shell: &mut Shell, draft: &mut Draft, i: usize, choice: usize) {
        let Some(&pick) = self.picking.get(choice) else { return };
        let partners = self.partners(draft);
        let Some(n) = pick else { return self.remove_partner(shell, draft, i) };
        if !draft.allows(FIELD, n.0) || partners.iter().enumerate().any(|(at, p)| at != i && p.navi == Some(n)) {
            return shell.sound.play(Cue::Refused);
        }
        let at = i.min(partners.len());
        if self.change(draft, |ps| {
            match ps.get_mut(at) {
                Some(p) => p.navi = Some(n),
                None => ps.push(Partner { navi: Some(n), programs: Vec::new() }),
            }
            true
        }) {
            shell.sound.play(Cue::Pick);
            self.chosen = at;
            self.described = Some(Described::Card(at));
            self.notice = None;
            self.changed(shell, draft);
            shell.nav.focus("partner", at as i32);
        }
    }

    fn remove_partner(&mut self, shell: &mut Shell, draft: &mut Draft, i: usize) {
        if self.change(draft, |ps| {
            if i >= ps.len() {
                return false;
            }
            ps.remove(i);
            true
        }) {
            shell.sound.play(Cue::Back);
            self.notice = None;
            self.changed(shell, draft);
        } else {
            shell.sound.play(Cue::Refused);
        }
    }

    /// Enter on PROGRAMS' row `row`: its program installed at the end of the chosen partner's.
    fn install(&mut self, shell: &mut Shell, draft: &mut Draft, row: usize) {
        let Some(&h) = self.programs.get(row) else { return };
        let partners = self.partners(draft);
        let i = self.chosen;
        let Some(p) = partners.get(i) else { return shell.sound.play(Cue::Refused) };
        let room = self.shape.as_ref().map_or(0, |s| s.program_room);
        if p.programs.len() >= room {
            return shell.sound.play(Cue::Refused);
        }
        let budgets = Party::budgets(draft);
        if let Some(why) = self.refusal(shell, draft, budgets.get(i).and_then(Option::as_ref), h) {
            return self.refuse(shell, draft, row, why);
        }
        if self.change(draft, |ps| {
            ps[i].programs.push(h);
            true
        }) {
            shell.sound.play(Cue::Pick);
            self.notice = None;
            self.changed(shell, draft);
        }
    }

    /// Program `k` of the chosen partner's taken out.
    fn uninstall(&mut self, shell: &mut Shell, draft: &mut Draft, k: usize) {
        let i = self.chosen;
        if self.change(draft, |ps| match ps.get_mut(i) {
            Some(p) if k < p.programs.len() => {
                p.programs.remove(k);
                true
            }
            _ => false,
        }) {
            shell.sound.play(Cue::Back);
            self.notice = None;
            self.changed(shell, draft);
            // (The focus stays in the list, on the program now there, or the last.)
            let left = self.partners(draft).get(i).map_or(0, |p| p.programs.len());
            if left > 0 {
                shell.nav.focus("installed", k.min(left - 1) as i32);
            } else {
                shell.nav.focus("partner", i as i32);
            }
        } else {
            shell.sound.play(Cue::Refused);
        }
    }

    fn close_sheet(&mut self, shell: &mut Shell) {
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        if view.get_sheet() {
            view.set_sheet(false);
            shell.nav.close("lib.");
        }
    }

    fn focused_at(shell: &Shell, id: &str) -> Option<usize> {
        shell.nav.focused().filter(|k| k.id == id).map(|k| k.index.max(0) as usize)
    }
}

impl Tab for Party {
    fn id(&self) -> EditorTab {
        EditorTab::Party
    }

    fn fields(&self, _content: &Content) -> Vec<String> {
        vec![FIELD.into()]
    }

    /// A party is the game's own navi's (EXE5's MegaMan's, as the DS's member 0).
    fn own_navi_only(&self) -> bool {
        true
    }

    fn main(&self) -> &'static str {
        "partner"
    }

    fn show(&mut self, shell: &mut Shell, draft: &mut Draft) {
        if self.described.is_none() {
            self.described = Some(Described::Card(self.chosen));
        }
        self.changed(shell, draft);
    }

    fn hide(&mut self, shell: &mut Shell, _draft: &mut Draft) {
        self.notice = None;
        self.hovered = None;
        self.close_sheet(shell);
    }

    fn activate(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32) {
        let index = index.max(0) as usize;
        match id {
            "partner" => {
                self.chosen = index;
                self.changed(shell, draft);
                self.pick_navi(shell, draft, index);
            }
            "installed" => self.uninstall(shell, draft, index),
            "lib.rows" => self.install(shell, draft, index),
            "programs" => {
                let ui = shell.ui();
                ui.global::<PartyView>().set_sheet(true);
                shell.nav.open("lib.", crate::nav::Key::new("lib.rows", 0));
            }
            "lib.close" => {
                self.close_sheet(shell);
                shell.nav.focus("programs", 0);
            }
            "clear" => {
                let question = shell.ui().global::<PartyView>().get_clear_question();
                editor::ask_clear(shell, question, self.partners(draft).is_empty());
            }
            _ => {}
        }
    }

    fn choose(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, choice: i32) {
        if id == "navi" {
            self.picked(shell, draft, index.max(0) as usize, choice.max(0) as usize);
        }
    }

    fn hovered(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, on: bool) {
        let index = index.max(0) as usize;
        let what = match id {
            "partner" => Described::Card(index),
            "installed" => Described::Installed(index),
            "lib.rows" => Described::Row(index),
            _ => return,
        };
        if editor::hover(&mut self.hovered, what, on) {
            self.show_detail(shell, draft);
        }
    }

    fn focused(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32) {
        let index = index.max(0) as usize;
        self.hovered = None;
        self.notice = None;
        match id {
            // (The card focused is the partner whose PartyCust is shown.)
            "partner" => {
                self.described = Some(Described::Card(index));
                if self.chosen != index {
                    self.chosen = index;
                    return self.changed(shell, draft);
                }
            }
            "installed" => self.described = Some(Described::Installed(index)),
            "lib.rows" => self.described = Some(Described::Row(index)),
            _ => {}
        }
        self.show_detail(shell, draft);
    }

    /// X, CLEAR.
    fn key(&mut self, shell: &mut Shell, draft: &mut Draft, key: &KeyPress) -> bool {
        if editor::clear_key(shell, key) {
            self.activate(shell, draft, "clear", 0);
            return true;
        }
        false
    }

    /// Delete takes out what is under the cursor: a program installed, a partner.
    fn action(&mut self, shell: &mut Shell, draft: &mut Draft, action: Action) -> bool {
        if action != Action::Delete {
            return false;
        }
        if let Some(k) = Party::focused_at(shell, "installed") {
            self.uninstall(shell, draft, k);
        } else if let Some(i) = Party::focused_at(shell, "partner") {
            self.remove_partner(shell, draft, i);
        } else {
            shell.sound.play(Cue::Refused);
        }
        true
    }

    /// Esc in a phone's sheet shuts it.
    fn back(&mut self, shell: &mut Shell, _draft: &mut Draft) -> bool {
        if shell.ui().global::<PartyView>().get_sheet() {
            self.close_sheet(shell);
            shell.nav.focus("programs", 0);
            return true;
        }
        false
    }

    fn answered(&mut self, shell: &mut Shell, draft: &mut Draft, tag: &str, choice: usize) {
        if editor::cleared(tag, choice) && self.change(draft, |ps| !std::mem::take(ps).is_empty()) {
            self.chosen = 0;
            self.changed(shell, draft);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::testing::exe5_content;

    /// EXE5's party read from its schema (two partners, each a navi and ten programs of PartyCust's), and a party
    /// written and read back.
    #[test]
    fn the_party_is_the_schemas() {
        let content = exe5_content();
        let shape = Shape::of(&content).unwrap();
        assert_eq!((shape.room, shape.navi.as_str(), shape.programs.as_str(), shape.collection.as_str(), shape.program_room), (2, "navi", "programs", "partycust_programs", 10));
        let builds = nettai_match::build::Builds::of(&content, "exe5").unwrap();
        let mut side = builds.new_build(&content, "protoman").unwrap();
        assert_eq!(partners(&content, &side, &shape), Vec::new());
        let navi = |n: &str| Some(nettai_match::ids::navi(&content, "exe5", n).unwrap());
        let program = |n: &str| nettai_match::ids::entry(&content, "exe5", "partycust_programs", n).unwrap();
        let party = vec![Partner { navi: navi("toadman"), programs: vec![program("p-spport"), program("p-spport")] }, Partner { navi: navi("protoman"), programs: Vec::new() }];
        assert!(write(&content, &mut side, &shape, &party));
        assert_eq!(partners(&content, &side, &shape), party);
        assert_eq!(tone(&content, program("p-spport")), slint::Color::from_rgb_u8(0xEA, 0xF3, 0xFF));
    }
}
