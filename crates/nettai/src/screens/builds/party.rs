//! The Party tab of a build's editor, in a game whose setup takes a party (EXE5's party battle, EXE5 DS's:
//! docs/design/exe5ds-map.md §4, §11): the navi's partners, each with its PartyCust programs filling its gauge (§4.10).
//!
//! **Its form is the setup's schema's** (`crate::builds::party::Shape`): the field is a list of records (its room the most
//! partners), each with a field naming a navi and a list of a collection's entries (the programs; its room the most a
//! partner holds). What may be a partner and what fits a gauge are the game's rules': the navis a side can play but its
//! own are offered, the build's problems say what the rules refuse, and the gauge is the rules' overview's (a list budget
//! of the partner's entry: `Overview::budget_of`), as is a program's cost and the round's navi changes
//! (`Overview::switches`).
//!
//! **The layout is the user's choice** (the verification workspace's docs/nettai-ui/party/chosen/party-tab-D9r.png), built
//! as the Folder and Patch cards tabs are: the strip describes what is under the cursor (or the pointer); the navis to
//! pick from, a row a version (the builds data's groups, in their colors), a chip each; PARTY framed at the left, its
//! counts and CLEAR in its head, a section a partner (its head, its programs); the library open at the right. **Enter
//! (or a click) on a chip** puts its navi in the party, at its end, or takes it out with its programs; one the build's
//! ruleset holds back (its `party` category) is dimmed and refused, and none goes in past the party's room. **The partner
//! the library adds to** is the one the cursor was last in (its head, its programs), marked as a list's chosen row is.
//! **Enter in the library** adds its program at the end of that partner's (a program may be in twice: the rules hold
//! them to the gauge alone), refused where the gauge or the list has no room ("Needs 3 blocks") or the ruleset holds
//! the program back (its `party.programs` category: "Not allowed"). **Del** takes out what is under the cursor: a
//! program, a partner (its head, its chip). **CLEAR** takes every partner out, asked first.
//!
//! **What a program does** is its effects (its definition's `effects`) in the words of the collection's own text table
//! (`<collection>_effects`: `partycust_programs_effects`), as the Patch cards tab words a card's lines; its kind is that
//! table's `kind.` and its definition's `kind`.
//!
//! **A program is dragged** as the Patch cards tab's cards are (`Drag`, `drag-over` and `drag-in-over`): from the library
//! into a partner's list, in a list to another place or into the other partner's, and onto the library, which takes it
//! out. Over a partner's section (its head or its programs) that partner is the one marked, its head lit, and a gap opens
//! in its list where the program would go (over its head, at the top); where it can't go in there, the dragged row says
//! why ("Needs 3 blocks") and no gap opens, and let go it is refused as Enter is. **A partner's head is dragged** onto
//! another's section (or above the first, below the last) to take its place, its programs with it; **a chip in the
//! party** onto another that is, to trade places.

use crate::builds::overview::{stat_line, stat_name};
use crate::builds::party::{FIELD, Partner, Shape, partners, tone, write};
use crate::input::Action;
use crate::screens::builds::editor::{self, Draft, Tab};
use crate::shell::Shell;
use crate::sound::Cue;
use nettai_battle::Content;
use nettai_content_api::{EntryHandle, NaviHandle, Registry};
use nettai_match::build::{Budget, Checked, EntryRefusal, NamedStat, Stat};
use nettai_match::check::Problem;
use nettai_match::ruleset::Item;
use nettai_ui::{BuildWords, EditorTab, PartyChip, PartyPanel, PartyRow, PartyTeam, PartyView, StatLine};
use slint::{ComponentHandle, Model, ModelRc, VecModel};

/// The list of partner `i`'s programs (its panel's).
fn installed_list(i: usize) -> String {
    format!("installed{i}")
}

/// The partner whose list `id` is, if it is one.
fn partner_of_list(id: &str) -> Option<usize> {
    id.strip_prefix("installed")?.parse().ok()
}

/// What program `h` does, on one line: its effects in text table `table`'s words, one after the other.
fn effect_of(names: &crate::games::Names, h: EntryHandle, table: &str) -> String {
    let named: crate::builds::cards::Named = &|_, _| None;
    crate::builds::cards::lines_in(names, h, table, named).unwrap_or_default().into_iter().map(|l| l.text).collect::<Vec<_>>().join(", ")
}

/// A program's row: its name, its blocks as budget `b` costs them, its color. (What it does is the strip's: the user,
/// "no need to show effects in the library column since they get shown on mouseover".)
fn program_row(words: &crate::builds::words::Words, content: &Content, h: EntryHandle, b: Option<&Budget>) -> PartyRow {
    PartyRow { name: words.names.entry(h).into(), blocks: b.map_or(0, |b| b.cost_of(content, h)) as i32, tone: tone(content, h), ..Default::default() }
}

/// Where a partner's section of PARTY is in the window: across, from its head's top to its end (the next's head's top;
/// the last's, a little past its list's), where its list begins, and its list's first row (as it is scrolled) and
/// pitch.
#[derive(Clone, Copy, Debug)]
struct Section {
    x: f32,
    right: f32,
    top: f32,
    list: f32,
    bottom: f32,
    first: f32,
    pitch: f32,
}

/// The sections of the first `n` partners, as the registry has their heads and lists (none it doesn't have).
fn sections(shell: &Shell, n: usize) -> Vec<Section> {
    let registry = shell.nav.registry();
    let registry = registry.borrow();
    let mut out: Vec<Section> = Vec::new();
    for i in 0..n {
        let Some(head) = registry.rect(&crate::nav::Key::new("target", i as i32)).filter(|r| r.w > 0.0) else { break };
        let list = registry.view(&installed_list(i));
        if let Some(last) = out.last_mut() {
            last.bottom = head.y;
        }
        let end = list.map_or(head.bottom(), |l| l.bottom().max(head.bottom()));
        let list_y = list.map_or(head.bottom(), |l| l.y);
        let (first, pitch) = registry.first_row(&installed_list(i)).unwrap_or((list_y, 0.0));
        out.push(Section { x: head.x, right: head.right(), top: head.y, list: list_y, bottom: end + 16.0, first, pitch });
    }
    out
}

/// The section of `sections` window point `(x, y)` is in, if it is in one.
fn section_at(sections: &[Section], (x, y): (f32, f32)) -> Option<usize> {
    sections.iter().position(|s| x >= s.x && x <= s.right && y >= s.top && y < s.bottom)
}

/// Where a program let go at window point `(x, y)` goes in, among partners laid out as `sections` with `counts`
/// programs: the partner whose section it is in, and the place between its programs nearest the pointer (over its
/// head, the first; past the last row, after it). None: in no section.
fn landing_in(sections: &[Section], counts: &[usize], (x, y): (f32, f32)) -> Option<(usize, usize)> {
    let i = section_at(sections, (x, y))?;
    let s = sections[i];
    let n = counts.get(i).copied().unwrap_or(0);
    if y < s.list || s.pitch <= 0.0 {
        return Some((i, 0));
    }
    Some((i, (((y - s.first) / s.pitch).round().max(0.0) as usize).min(n)))
}

/// What control `id` (at `index`) puts in the strip.
fn described(id: &str, index: i32) -> Option<Described> {
    let at = index.max(0) as usize;
    Some(match id {
        "chip" => Described::Chip(at),
        "target" => Described::Partner(at),
        "lib.rows" => Described::Row(at),
        id => Described::Installed(partner_of_list(id)?, at),
    })
}

/// What the strip says of a navi as a build would field it: its HP, its buster's attack and charge (as the games show
/// them, 1 to 5), the stats its game's rules give it (an EXE5 team navi's own chip's damage) and its gauge's blocks.
#[derive(Clone, Debug, Default)]
struct NaviFacts {
    hp: i32,
    attack: i32,
    charge: i32,
    named: Vec<NamedStat>,
    gauge: Option<u32>,
}

impl NaviFacts {
    /// Partner `i` of a checked build (its party's member `i + 1`).
    fn of(checked: &mut Checked, content: &std::sync::Arc<Content>, side: &nettai_match::Side, i: usize) -> Option<NaviFacts> {
        let s = *checked.members().get(i + 1)?;
        let o = checked.overview(content, side);
        Some(NaviFacts {
            hp: s.max_hp.into(),
            attack: i32::from(s.attack) + 1,
            charge: i32::from(s.charge) + 1,
            named: o.as_ref().and_then(|o| o.members.iter().find(|(m, _)| *m == i + 1)).map(|(_, s)| s.clone()).unwrap_or_default(),
            gauge: o.as_ref().and_then(|o| o.budget_of(FIELD, i)).map(|b| b.count.limit),
        })
    }

    /// Its stats as the strip says them, by what the navi `alone` (no programs) has: what its programs added in cyan,
    /// as the overview says it. (Their labels: HP, Attack, Charge the view's; a stat of the rules' own its name.)
    fn lines(&self, alone: &NaviFacts, words: &crate::builds::words::Words) -> Vec<StatLine> {
        let line = |with: i32, base: i32| stat_line(String::new(), Stat { base, value: with });
        [line(self.hp, alone.hp), line(self.attack, alone.attack), line(self.charge, alone.charge)]
            .into_iter()
            .chain(self.named.iter().map(|s| stat_line(stat_name(words, s), s.stat)))
            .collect()
    }
}

/// What the strip describes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Described {
    /// A chip of the picker, by its place.
    Chip(usize),
    /// A partner's section head.
    Partner(usize),
    /// A partner's program.
    Installed(usize, usize),
    /// A row of the library.
    Row(usize),
}

#[derive(Default)]
pub struct Party {
    shape: Option<Shape>,
    /// The partner the library adds to: the one the cursor was last in.
    target: usize,
    /// The navis offered, in the chips' order (the teams' rows, one after the other).
    navis: Vec<NaviHandle>,
    /// The library's programs, in the game's order.
    programs: Vec<EntryHandle>,
    refused: i32,
    /// What the strip describes: the focused thing, and the one under the pointer, which it describes in its place.
    described: Option<Described>,
    hovered: Option<Described>,
    /// Why the last thing asked wasn't done, until the cursor moves.
    notice: Option<String>,
    /// A navi's facts alone (no programs), as the build would field it: asked once a navi (a check of the build with it
    /// the one partner), for the strip's description of a navi, and what a partner's programs add to it.
    alone: std::collections::HashMap<NaviHandle, Option<NaviFacts>>,
    /// The partners' budgets, asked once until the build changes (`changed` forgets them): a drag asks them at every
    /// move.
    kept_budgets: Option<Vec<Option<Budget>>>,
    /// A budget the programs are costed by (`learn_costing`).
    costing: Option<Budget>,
}

impl Party {
    /// The party's shape and the library's programs, read the first time.
    fn read(&mut self, draft: &Draft) -> Option<Shape> {
        if self.shape.is_none() {
            let content = draft.content();
            let shape = Shape::of(content)?;
            let mut programs: Vec<((), EntryHandle)> = crate::builds::edit::of_game(content, &draft.kit.game, Registry::Entry, Some(&shape.collection)).into_iter().map(|h| ((), EntryHandle(h))).collect();
            draft.kit.order.entries(content, &shape.collection, &mut programs);
            self.programs = programs.into_iter().map(|(_, h)| h).collect();
            self.shape = Some(shape);
        }
        self.shape.clone()
    }

    fn partners(&self, draft: &Draft) -> Vec<Partner> {
        self.shape.as_ref().map(|s| partners(draft.content(), draft.side(), s)).unwrap_or_default()
    }

    /// Each partner's gauge, by its entry: the rules' overview's budget of it; and the round's navi changes.
    fn budgets(draft: &mut Draft) -> (Vec<Option<Budget>>, Option<u32>) {
        let content = draft.content().clone();
        let side = draft.side().clone();
        let overview = draft.checked.overview(&content, &side);
        (crate::builds::party::budgets(&content, &side, overview.as_ref()), overview.and_then(|o| o.switches))
    }

    /// The ruleset category of the partners' programs: the party's field and its records' programs' field, by their
    /// path (`party.programs`: nettai-match's categories over a field of a list's records).
    fn programs_category(&self) -> String {
        format!("{FIELD}.{}", self.shape.as_ref().map_or("", |s| s.programs.as_str()))
    }

    /// Why program `h` can't go into the target partner's now (the build's ruleset, its gauge), if it can't.
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

    /// The build's problems with partner `i` (its own, and the party's about no partner).
    fn problems(draft: &Draft, i: usize) -> Vec<&Problem> {
        draft.checked.problems.iter().filter(|p| draft.facts_of(p).iter().any(|f| f == FIELD) && (p.at(i) || p.entry.is_none())).collect()
    }

    /// The library's rows: each program's blocks, its name, and why it can't go into the target partner's list (its
    /// budget `target`: red, the row greyed), where there are partners.
    fn library_rows(&self, shell: &Shell, draft: &Draft, words: &crate::builds::words::Words, partnered: bool, target: Option<&Budget>) -> Vec<PartyRow> {
        let content = draft.content();
        self.programs
            .iter()
            .map(|&h| {
                let mut r = program_row(words, content, h, target.or(self.costing.as_ref()));
                if partnered && let Some(why) = self.refusal(shell, draft, target, h) {
                    r.why = why.into();
                }
                r
            })
            .collect()
    }

    /// A budget to cost the programs by, learned once (its cost field is the rules' overview's, every partner's the
    /// same): a partner's, else a check's of the build with the first navi offered its one partner. The library's rows
    /// have their blocks with no partner too, and the blocks' column is as wide as the most a program fills.
    fn learn_costing(&mut self, draft: &Draft, budgets: &[Option<Budget>]) {
        if self.costing.is_some() {
            return;
        }
        self.costing = budgets.iter().flatten().next().cloned().or_else(|| {
            let shape = self.shape.clone()?;
            let n = *self.navis.first()?;
            let content = draft.content().clone();
            let mut side = draft.side().clone();
            write(&content, &mut side, &shape, &[Partner { navi: Some(n), programs: Vec::new() }]).then_some(())?;
            Checked::of(&content, &draft.kit.game, &side).overview(&content, &side)?.budget_of(FIELD, 0).cloned()
        });
    }

    /// The partners' budgets, asked once until the build changes.
    fn kept_budgets(&mut self, draft: &mut Draft) -> Vec<Option<Budget>> {
        if self.kept_budgets.is_none() {
            self.kept_budgets = Some(Party::budgets(draft).0);
        }
        self.kept_budgets.clone().unwrap_or_default()
    }

    /// Partner `i` is the one the library adds to (the cursor came into it, or a program dragged is over it): its head
    /// lit, the library's rows saying why they can't go into its list. The rows change in place and nothing is made
    /// anew: a row the pointer pressed or holds stays the one it holds (a model set anew would make every row anew,
    /// and the pointer let go of it).
    fn retarget(&mut self, shell: &Shell, draft: &mut Draft, i: usize) {
        if i == self.target {
            return;
        }
        self.target = i;
        let budgets = self.kept_budgets(draft);
        let target = budgets.get(i).cloned().flatten();
        let rows = draft.with_words(shell, |words| self.library_rows(shell, draft, words, true, target.as_ref()));
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        let model = view.get_rows();
        if model.row_count() == rows.len() {
            for (k, r) in rows.into_iter().enumerate() {
                model.set_row_data(k, r);
            }
        } else {
            view.set_rows(ModelRc::new(VecModel::from(rows)));
        }
        view.set_target(i as i32);
        view.set_free(target.map_or(0, |b| b.count.limit.saturating_sub(b.count.used)) as i32);
        self.show_detail(shell, draft);
    }

    /// Why program `h` can't go into partner `i`'s list now (its list full: no words), if it can't.
    fn why_not(&self, shell: &Shell, draft: &Draft, budgets: &[Option<Budget>], i: usize, h: EntryHandle) -> Option<String> {
        let room = self.shape.as_ref().map_or(0, |s| s.program_room);
        if self.partners(draft).get(i).is_none_or(|p| p.programs.len() >= room) {
            return Some(String::new());
        }
        self.refusal(shell, draft, budgets.get(i).and_then(Option::as_ref), h)
    }

    /// Tell the UI where a program dragged over partner `i`'s section would go in (`at`), or, where it can't go in, why
    /// (in the dragged row's place for what it does) and no gap.
    fn show_program_landing(&self, shell: &Shell, i: usize, at: usize, why: Option<String>) {
        let ui = shell.ui();
        ui.global::<PartyView>().set_drag_why(why.clone().unwrap_or_default().into());
        editor::show_landing(shell, &installed_list(i), if why.is_some() { None } else { Some(at) }, false);
    }

    /// Nothing shown of where a drag would land: into no list (the list it came from no longer scrolls near its
    /// edges for it).
    fn show_no_landing(&self, shell: &Shell) {
        shell.ui().global::<PartyView>().set_drag_why(Default::default());
        editor::show_landing(shell, "", None, false);
    }

    /// Everything shown anew.
    fn changed(&mut self, shell: &Shell, draft: &mut Draft) {
        self.kept_budgets = None;
        let Some(shape) = self.read(draft) else { return };
        let content = draft.content().clone();
        let partners = partners(&content, draft.side(), &shape);
        let (budgets, changes) = Party::budgets(draft);
        self.target = self.target.min(partners.len().saturating_sub(1));
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        let say = ui.global::<BuildWords>();
        let kit = draft.kit.clone();
        let budget = |i: usize| budgets.get(i).cloned().flatten();
        let category = self.programs_category();
        // The navis offered, a row a version (the builds data's groups, in their order); those of none in a row last.
        let own = draft.side().navi(&content);
        let offered: Vec<NaviHandle> = nettai_match::navis(&content, &kit.game).into_iter().filter(|&n| n != own).collect();
        let rulesets = kit.rulesets();
        let group_of = |n: NaviHandle| rulesets.as_ref().and_then(|r| r.group_of(Registry::Navi, &Item::of(n.0))).map(|g| g.id.clone());
        let mut teams: Vec<(Option<String>, Vec<NaviHandle>)> = kit.builds.groups.iter().map(|g| (Some(g.id.clone()), Vec::new())).collect();
        for &n in &offered {
            let id = group_of(n);
            match teams.iter_mut().find(|(g, _)| *g == id) {
                Some((_, row)) => row.push(n),
                None => teams.push((id, vec![n])),
            }
        }
        teams.retain(|(_, row)| !row.is_empty());
        self.navis = teams.iter().flat_map(|(_, row)| row.iter().copied()).collect();
        let full = partners.len() >= shape.room;
        self.learn_costing(draft, &budgets);
        let most = self.costing.as_ref().and_then(|b| self.programs.iter().map(|&h| b.cost_of(&content, h)).max()).unwrap_or(1);
        let (team_rows, panels, rows) = draft.with_words(shell, |words| {
            let firsts = teams.iter().scan(0, |first, (_, row)| {
                let at = *first;
                *first += row.len();
                Some(at as i32)
            });
            let team_rows: Vec<PartyTeam> = teams
                .iter()
                .zip(firsts)
                .map(|((id, row), first)| {
                    let tag = id.as_ref().and_then(|id| kit.builds.group(id)).map(|_| kit.version_tag_of(words.names, Registry::Navi, &Item::of(row[0].0))).unwrap_or_default();
                    let chips: Vec<PartyChip> = row
                        .iter()
                        .map(|&n| {
                            let place = partners.iter().position(|p| p.navi == Some(n)).map_or(0, |at| at as i32 + 1);
                            PartyChip { face: kit.pictures.navi(n), name: words.names.navi_long(n).into(), place, full: full && place == 0, held: !draft.allows(FIELD, n.0) }
                        })
                        .collect();
                    PartyTeam { name: id.as_ref().map(|id| kit.version_name(words.names, id)).unwrap_or_default().into(), tone: tag.tone, chips: ModelRc::new(VecModel::from(chips)), first }
                })
                .collect();
            let row = |h: EntryHandle, b: Option<&Budget>| program_row(words, &content, h, b);
            let panels: Vec<PartyPanel> = partners
                .iter()
                .enumerate()
                .filter_map(|(i, p)| {
                    let navi = p.navi?;
                    let b = budget(i);
                    // (Each block's program, in order; the empty blocks one run after them.)
                    let mut runs: Vec<i32> = p.programs.iter().enumerate().flat_map(|(k, &h)| std::iter::repeat_n(k as i32, b.as_ref().map_or(0, |b| b.cost_of(&content, h)) as usize)).collect();
                    let gauge = b.as_ref().map_or(0, |b| b.count.limit as usize);
                    let last = if p.programs.is_empty() { 0 } else { p.programs.len() as i32 };
                    while runs.len() < gauge {
                        runs.push(last);
                    }
                    let rows: Vec<PartyRow> = p
                        .programs
                        .iter()
                        .map(|&h| {
                            let mut r = row(h, b.as_ref());
                            // (One the build's ruleset holds back all the same says so, red: it can be taken out.)
                            if !draft.allows(&category, h.0) {
                                r.note = say.invoke_not_allowed_here();
                                r.bad = true;
                            }
                            r
                        })
                        .collect();
                    Some(PartyPanel {
                        face: kit.pictures.navi(navi),
                        name: words.names.navi_long(navi).into(),
                        hp: crate::builds::party::hp(&draft.checked, navi).map(|hp| view.invoke_hp_word(hp as i32)).unwrap_or_default(),
                        used: b.as_ref().map_or(0, |b| b.count.used as i32),
                        gauge: gauge as i32,
                        tones: ModelRc::new(VecModel::from(crate::builds::party::tones(&content, p, b.as_ref()))),
                        runs: ModelRc::new(VecModel::from(runs)),
                        bad: !Party::problems(draft, i).is_empty(),
                        rows: ModelRc::new(VecModel::from(rows)),
                    })
                })
                .collect();
            let rows = self.library_rows(shell, draft, words, !partners.is_empty(), budget(self.target).as_ref());
            (team_rows, panels, rows)
        });
        let target = budget(self.target);
        view.set_most_blocks(most as i32);
        view.set_partners(partners.len() as i32);
        view.set_room(shape.room as i32);
        view.set_changes(changes.unwrap_or(0) as i32);
        view.set_teams(ModelRc::new(VecModel::from(team_rows)));
        view.set_panels(ModelRc::new(VecModel::from(panels)));
        view.set_target(self.target as i32);
        view.set_marked(!partners.is_empty());
        view.set_free(target.map_or(0, |b| b.count.limit.saturating_sub(b.count.used)) as i32);
        view.set_rows(ModelRc::new(VecModel::from(rows)));
        view.set_problems(draft.pane_line(shell, &[FIELD.to_string()], false).into());
        self.show_detail(shell, draft);
    }

    /// The collection's own text table of its programs' effects and kinds (`<collection>_effects`).
    fn effects_table(&self) -> String {
        format!("{}_effects", self.shape.as_ref().map_or("", |s| s.collection.as_str()))
    }

    /// A navi's facts as the build would field it, and alone (no programs; kept, a navi once): a partner's its own,
    /// its programs' in; a navi not in the party's as a check of the build with it the one partner says.
    fn navi_facts(&mut self, draft: &mut Draft, n: NaviHandle) -> Option<(NaviFacts, NaviFacts)> {
        if !self.alone.contains_key(&n) {
            let shape = self.shape.clone()?;
            let content = draft.content().clone();
            let mut side = draft.side().clone();
            let facts = write(&content, &mut side, &shape, &[Partner { navi: Some(n), programs: Vec::new() }])
                .then(|| NaviFacts::of(&mut Checked::of(&content, &draft.kit.game, &side), &content, &side, 0))
                .flatten();
            self.alone.insert(n, facts);
        }
        let alone = self.alone.get(&n).cloned().flatten()?;
        let partners = self.partners(draft);
        let with = match partners.iter().position(|p| p.navi == Some(n)) {
            Some(i) => {
                let (content, side) = (draft.content().clone(), draft.side().clone());
                NaviFacts::of(&mut draft.checked, &content, &side, i)?
            }
            None => alone.clone(),
        };
        Some((with, alone))
    }

    /// The strip: what the cursor is on; while the pointer is on a chip or a row, that one.
    fn show_detail(&mut self, shell: &Shell, draft: &mut Draft) {
        let content = draft.content().clone();
        let partners = self.partners(draft);
        let (budgets, _) = Party::budgets(draft);
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        let say = ui.global::<BuildWords>();
        let notice = if self.hovered.is_some_and(|h| Some(h) != self.described) { None } else { self.notice.clone() };
        let kit = draft.kit.clone();
        let table = self.effects_table();
        let described = self.hovered.or(self.described);
        // (A navi: its face, its name and version, its HP; its version and its gauge's blocks as its columns.)
        let navi = match described {
            Some(Described::Chip(k)) => self.navis.get(k).copied(),
            Some(Described::Partner(i)) => partners.get(i).and_then(|p| p.navi),
            _ => None,
        };
        let facts = navi.and_then(|n| self.navi_facts(draft, n));
        let element = navi.map(|n| kit.pictures.element(super::navi::family(&content, content.navi(n).element)));
        let detail = draft.with_words(shell, |words| {
            let mut d = match (described, navi) {
                (_, Some(n)) => {
                    let tag = kit.version_tag_of(words.names, Registry::Navi, &Item::of(n.0));
                    let stats = facts.as_ref().map(|(with, alone)| with.lines(alone, words)).unwrap_or_default();
                    let facts: Vec<slint::SharedString> = facts.as_ref().and_then(|(with, _)| with.gauge).map(|g| view.invoke_gauge_word(g as i32)).into_iter().collect();
                    let i = partners.iter().position(|p| p.navi == Some(n));
                    nettai_ui::PartyDetail {
                        there: true,
                        face: kit.pictures.navi(n),
                        name: words.names.navi_long(n).into(),
                        version: tag,
                        stats: ModelRc::new(VecModel::from(stats)),
                        element: element.clone().unwrap_or_default(),
                        facts: ModelRc::new(VecModel::from(facts)),
                        fault: i.map(|i| draft.line(shell, Party::problems(draft, i))).unwrap_or_default().into(),
                        problem: if matches!(described, Some(Described::Chip(_))) && !draft.allows(FIELD, n.0) { say.invoke_not_allowed_here() } else { Default::default() },
                        ..Default::default()
                    }
                }
                (Some(Described::Installed(i, k)), _) => match partners.get(i).and_then(|p| p.programs.get(k)) {
                    Some(&h) => {
                        let mut d = self.program_detail(shell, draft, words, &table, &budgets, i, h, false);
                        d.fault = draft.line(shell, Party::problems(draft, i)).into();
                        d
                    }
                    None => Default::default(),
                },
                (Some(Described::Row(r)), _) => match self.programs.get(r) {
                    Some(&h) => self.program_detail(shell, draft, words, &table, &budgets, self.target, h, !partners.is_empty()),
                    None => Default::default(),
                },
                _ => Default::default(),
            };
            if let Some(n) = &notice {
                d.problem = n.as_str().into();
            }
            d
        });
        view.set_detail(detail);
    }

    /// A program as the strip describes it, as the Folder tab's strip a chip: its blocks and name, what it does; its
    /// blocks, its kind, and whether they fit partner `i`'s gauge (`fitting`: it is the library's, which adds to it).
    #[allow(clippy::too_many_arguments)]
    fn program_detail(&self, shell: &Shell, draft: &Draft, words: &crate::builds::words::Words, table: &str, budgets: &[Option<Budget>], i: usize, h: EntryHandle, fitting: bool) -> nettai_ui::PartyDetail {
        let content = draft.content();
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        let b = budgets.get(i).cloned().flatten();
        let cost = b.as_ref().map_or(0, |b| b.cost_of(content, h)) as i32;
        let kind = content.defs.definitions.get(Registry::Entry, &content.defs.entry(h).key).and_then(|d| d.spec.field("kind").str().map(str::to_string));
        let kind = kind.and_then(|k| words.names.text(table, &format!("kind.{k}")));
        let why = if fitting { self.refusal(shell, draft, b.as_ref(), h) } else { None };
        let fits = (fitting && why.is_none()).then(|| view.invoke_fits_word(b.as_ref().map_or(0, |b| b.count.free()) as i32));
        let facts: Vec<slint::SharedString> = [Some(view.invoke_blocks_word(cost)), kind.map(Into::into), fits].into_iter().flatten().collect();
        nettai_ui::PartyDetail {
            there: true,
            name: words.names.entry(h).into(),
            blocks: cost,
            tone: tone(content, h),
            description: effect_of(words.names, h, table).into(),
            facts: ModelRc::new(VecModel::from(facts)),
            problem: why.unwrap_or_default().into(),
            ..Default::default()
        }
    }

    /// Change the build's partners: whether they changed.
    fn change(&self, draft: &mut Draft, change: impl FnOnce(&mut Vec<Partner>) -> bool) -> bool {
        let Some(shape) = self.shape.clone() else { return false };
        draft.edit(|content, side| {
            let mut ps = partners(content, side, &shape);
            change(&mut ps) && write(content, side, &shape, &ps)
        })
    }

    /// Enter on chip `k`: its navi put in the party, at its end, or taken out with its programs.
    fn toggle(&mut self, shell: &mut Shell, draft: &mut Draft, k: usize) {
        let Some(&n) = self.navis.get(k) else { return };
        let room = self.shape.as_ref().map_or(0, |s| s.room);
        let partners = self.partners(draft);
        if let Some(at) = partners.iter().position(|p| p.navi == Some(n)) {
            return self.remove_partner(shell, draft, at);
        }
        if partners.len() >= room || !draft.allows(FIELD, n.0) {
            return shell.sound.play(Cue::Refused);
        }
        if self.change(draft, |ps| {
            ps.push(Partner { navi: Some(n), programs: Vec::new() });
            true
        }) {
            shell.sound.play(Cue::Pick);
            self.target = partners.len();
            self.changed(shell, draft);
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
            self.changed(shell, draft);
        } else {
            shell.sound.play(Cue::Refused);
        }
    }

    /// Refuse the library's row `row`: it shakes, with the refusal's sound.
    fn refuse(&mut self, shell: &mut Shell, row: usize) {
        shell.sound.play(Cue::Refused);
        self.refused += 1;
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        view.set_refused(row as i32);
        view.set_refused_seq(self.refused);
    }

    /// PROGRAMS' row `row`'s program installed among partner `i`'s at `at` (past their end: at it), refused where the
    /// list or the gauge has no room or the ruleset holds it back.
    fn install(&mut self, shell: &mut Shell, draft: &mut Draft, row: usize, i: usize, at: usize) {
        let Some(&h) = self.programs.get(row) else { return };
        let partners = self.partners(draft);
        let Some(p) = partners.get(i) else { return shell.sound.play(Cue::Refused) };
        let room = self.shape.as_ref().map_or(0, |s| s.program_room);
        let (budgets, _) = Party::budgets(draft);
        if p.programs.len() >= room || self.refusal(shell, draft, budgets.get(i).and_then(Option::as_ref), h).is_some() {
            return self.refuse(shell, row);
        }
        if self.change(draft, |ps| {
            let at = at.min(ps[i].programs.len());
            ps[i].programs.insert(at, h);
            true
        }) {
            shell.sound.play(Cue::Pick);
            self.target = i;
            self.changed(shell, draft);
        }
    }

    /// Program `k` of partner `i`'s taken out.
    fn uninstall(&mut self, shell: &mut Shell, draft: &mut Draft, i: usize, k: usize) {
        if self.change(draft, |ps| match ps.get_mut(i) {
            Some(p) if k < p.programs.len() => {
                p.programs.remove(k);
                true
            }
            _ => false,
        }) {
            shell.sound.play(Cue::Back);
            self.changed(shell, draft);
            // (The focus stays in the list, on the program now there, or the last; else on the partner's head.)
            let left = self.partners(draft).get(i).map_or(0, |p| p.programs.len());
            if left > 0 {
                shell.nav.focus(&installed_list(i), k.min(left - 1) as i32);
            } else {
                shell.nav.focus("target", i as i32);
            }
        } else {
            shell.sound.play(Cue::Refused);
        }
    }

    /// Program `k` of partner `from`'s moved to place `to` of partner `into`'s (the same or the other); into the other,
    /// refused as an install is.
    fn shift(&mut self, shell: &mut Shell, draft: &mut Draft, from: usize, k: usize, into: usize, to: usize) {
        let partners = self.partners(draft);
        let Some(&h) = partners.get(from).and_then(|p| p.programs.get(k)) else { return };
        if into != from {
            let room = self.shape.as_ref().map_or(0, |s| s.program_room);
            let (budgets, _) = Party::budgets(draft);
            let fits = partners.get(into).is_some_and(|p| p.programs.len() < room) && self.refusal(shell, draft, budgets.get(into).and_then(Option::as_ref), h).is_none();
            if !fits {
                return shell.sound.play(Cue::Refused);
            }
        }
        if self.change(draft, |ps| {
            let h = ps[from].programs.remove(k);
            let at = to.min(ps[into].programs.len());
            ps[into].programs.insert(at, h);
            from != into || at != k
        }) {
            shell.sound.play(Cue::Pick);
            self.target = into;
            self.changed(shell, draft);
        }
    }

    /// Where a program dragged to `at` (the UI's `Drag` place) would go: the partner whose section it is over (its head
    /// or its list), and the place among its programs nearest the pointer (over its head, the first). None: over no
    /// partner.
    fn landing(&self, shell: &Shell, draft: &Draft, at: i32) -> Option<(usize, usize)> {
        let counts: Vec<usize> = self.partners(draft).iter().map(|p| p.programs.len()).collect();
        landing_in(&sections(shell, counts.len()), &counts, editor::pointer_at(at))
    }

    /// Where a partner's head dragged to `at` would put it: the place of the partner whose section it is over (above
    /// the first, the first's; below the last, the last's). None: beside them.
    fn partner_landing(&self, shell: &Shell, draft: &Draft, at: i32) -> Option<usize> {
        let (x, y) = editor::pointer_at(at);
        let sections = sections(shell, self.partners(draft).len());
        let (first, last) = (sections.first()?, sections.last()?);
        if x < first.x || x > first.right {
            return None;
        }
        Some(if y < first.top { 0 } else if y >= last.bottom { sections.len() - 1 } else { section_at(&sections, (x, y))? })
    }

    /// Partner `from` moved to place `to` among them, its programs with it, or (`trade`) the two trading places.
    fn move_partner(&mut self, shell: &mut Shell, draft: &mut Draft, from: usize, to: usize, trade: bool) {
        if self.change(draft, |ps| {
            if from == to || from >= ps.len() || to >= ps.len() {
                return false;
            }
            if trade {
                ps.swap(from, to);
            } else {
                let p = ps.remove(from);
                ps.insert(to, p);
            }
            true
        }) {
            shell.sound.play(Cue::Pick);
            self.target = to;
            self.changed(shell, draft);
            shell.nav.focus("target", to as i32);
        }
    }

    /// The partner chip `k` of the picker is in the party as, if it is.
    fn partner_of_chip(&self, draft: &Draft, k: usize) -> Option<usize> {
        let n = *self.navis.get(k)?;
        self.partners(draft).iter().position(|p| p.navi == Some(n))
    }

    fn close_sheet(&mut self, shell: &mut Shell) {
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        if view.get_sheet() {
            view.set_sheet(false);
            shell.nav.close("lib.");
        }
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
        "chip"
    }

    fn show(&mut self, shell: &mut Shell, draft: &mut Draft) {
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
            "chip" => self.toggle(shell, draft, index),
            // (Enter on a partner's head: the library, which adds to it; a phone's sheet.)
            "target" => {
                self.target = index;
                self.changed(shell, draft);
                if shell.scale.phone {
                    shell.ui().global::<PartyView>().set_sheet(true);
                    shell.nav.open("lib.", crate::nav::Key::new("lib.rows", 0));
                } else {
                    shell.nav.focus("lib.rows", 0);
                }
            }
            "lib.rows" => {
                let at = usize::MAX;
                self.install(shell, draft, index, self.target, at);
            }
            "programs" => {
                shell.ui().global::<PartyView>().set_sheet(true);
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
            id => {
                if let Some(i) = partner_of_list(id)
                    && self.target != i
                {
                    self.target = i;
                    self.changed(shell, draft);
                }
            }
        }
    }

    fn choose(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, choice: i32) {
        let list = shell.ui().global::<nettai_ui::Drag>().get_list().to_string();
        match (id, list.as_str()) {
            // A partner's head dragged: over another partner's section (or above the first, below the last), it takes
            // that one's place; let go there, the partners move, their programs with them.
            ("drag-over", "target") => match self.partner_landing(shell, draft, choice).filter(|&to| to != index.max(0) as usize) {
                Some(to) => editor::show_landing(shell, "target", Some(to), true),
                None => self.show_no_landing(shell),
            },
            ("drag-drop", "target") => {
                self.show_no_landing(shell);
                if let Some(to) = self.partner_landing(shell, draft, choice) {
                    self.move_partner(shell, draft, index.max(0) as usize, to, false);
                }
            }
            // A chip of the picker that is in the party dragged onto another that is: the two trade places.
            ("drag-over", "chip") => {
                let placed: Vec<usize> = (0..self.navis.len()).filter(|&k| k != index.max(0) as usize && self.partner_of_chip(draft, k).is_some()).collect();
                match editor::nearest(shell, "chip", placed.into_iter(), editor::pointer_at(choice), None) {
                    Some(k) => editor::show_landing(shell, "chip", Some(k), true),
                    None => self.show_no_landing(shell),
                }
            }
            ("drag-drop", "chip") => {
                self.show_no_landing(shell);
                let placed: Vec<usize> = (0..self.navis.len()).filter(|&k| k != index.max(0) as usize && self.partner_of_chip(draft, k).is_some()).collect();
                if let Some(k) = editor::nearest(shell, "chip", placed.into_iter(), editor::pointer_at(choice), None)
                    && let (Some(from), Some(to)) = (self.partner_of_chip(draft, index.max(0) as usize), self.partner_of_chip(draft, k))
                {
                    self.move_partner(shell, draft, from, to, true);
                }
            }
            // A partner's program dragged: over the library, taken out if let go; over a partner's section (its head or
            // its list), that partner is the one marked, and where it would go in there (a gap, those from there on
            // shifting), or why it can't (no gap); elsewhere, nowhere (it goes back).
            ("drag-over", _) if editor::over_library(shell, choice) => {
                shell.ui().global::<PartyView>().set_drag_why(Default::default());
                editor::show_remove(shell);
            }
            ("drag-over", _) => {
                let (Some(from), Ok(k)) = (partner_of_list(&list), usize::try_from(index)) else { return };
                match self.landing(shell, draft, choice) {
                    Some((i, at)) => {
                        self.retarget(shell, draft, i);
                        if i == from {
                            // (In its own list a program moves to a row there is: past those before the place, less itself.)
                            self.show_program_landing(shell, i, if at > k { at - 1 } else { at }, None);
                        } else {
                            let budgets = self.kept_budgets(draft);
                            let why = self.partners(draft).get(from).and_then(|p| p.programs.get(k)).and_then(|&h| self.why_not(shell, draft, &budgets, i, h));
                            self.show_program_landing(shell, i, at, why);
                        }
                    }
                    None => self.show_no_landing(shell),
                }
            }
            ("drag-drop", _) => {
                self.show_no_landing(shell);
                let (Some(from), Ok(k)) = (partner_of_list(&list), usize::try_from(index)) else { return };
                if editor::over_library(shell, choice) {
                    self.uninstall(shell, draft, from, k);
                } else if let Some((into, at)) = self.landing(shell, draft, choice) {
                    let to = if into == from && at > k { at - 1 } else { at };
                    self.shift(shell, draft, from, k, into, to);
                }
            }
            // A library's program dragged: over a partner's section, that partner is the one marked, and where it would
            // go in among its programs (a gap, those from there on shifting), or why it can't (no gap); let go there, it
            // goes in, refused as Enter's is; anywhere else, nothing.
            ("drag-in-over", _) if index >= 0 => match self.landing(shell, draft, choice) {
                Some((i, at)) => {
                    self.retarget(shell, draft, i);
                    let budgets = self.kept_budgets(draft);
                    let why = self.programs.get(index as usize).and_then(|&h| self.why_not(shell, draft, &budgets, i, h));
                    self.show_program_landing(shell, i, at, why);
                }
                None => self.show_no_landing(shell),
            },
            ("drag-in-drop", _) => {
                self.show_no_landing(shell);
                if let (Some((i, at)), Ok(row)) = (self.landing(shell, draft, choice), usize::try_from(index)) {
                    self.install(shell, draft, row, i, at);
                }
            }
            _ => {}
        }
    }

    /// The pointer on a chip, a head or a row: the strip describes it.
    fn hovered(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, on: bool) {
        let Some(what) = described(id, index) else { return };
        if editor::hover(&mut self.hovered, what, on) {
            self.show_detail(shell, draft);
        }
    }

    /// The strip follows the cursor; the cursor in a partner, its head or its programs, makes it the one the library adds
    /// to; in the library, its title is lit.
    fn focused(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32) {
        self.hovered = None;
        self.notice = None;
        let ui = shell.ui();
        ui.global::<PartyView>().set_in_library(id.starts_with("lib."));
        if let Some(what) = described(id, index) {
            self.described = Some(what);
        }
        let i = match id {
            "target" => Some(index.max(0) as usize),
            id => partner_of_list(id),
        };
        match i {
            Some(i) if self.target != i => self.retarget(shell, draft, i),
            _ => self.show_detail(shell, draft),
        }
    }

    fn answered(&mut self, shell: &mut Shell, draft: &mut Draft, tag: &str, choice: usize) {
        if editor::cleared(tag, choice) && self.change(draft, |ps| !std::mem::take(ps).is_empty()) {
            self.target = 0;
            self.changed(shell, draft);
        }
    }

    /// X, CLEAR.
    fn key(&mut self, shell: &mut Shell, draft: &mut Draft, key: &crate::input::KeyPress) -> bool {
        if editor::clear_key(shell, key) {
            self.activate(shell, draft, "clear", 0);
            return true;
        }
        false
    }

    /// Delete takes out what is under the cursor: an installed program, a chip's partner.
    fn action(&mut self, shell: &mut Shell, draft: &mut Draft, action: Action) -> bool {
        if action != Action::Delete {
            return false;
        }
        let Some(key) = shell.nav.focused() else { return false };
        let at = key.index.max(0) as usize;
        if let Some(i) = partner_of_list(&key.id) {
            self.uninstall(shell, draft, i, at);
        } else if key.id == "target" {
            self.remove_partner(shell, draft, at);
        } else if key.id == "chip" {
            match self.navis.get(at).and_then(|&n| self.partners(draft).iter().position(|p| p.navi == Some(n))) {
                Some(i) => self.remove_partner(shell, draft, i),
                None => shell.sound.play(Cue::Refused),
            }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::testing::exe5_content;

    /// Two partners laid out as PARTY lays them out (heads 34 tall, rows 30 apart, an empty list its Empty row), the
    /// first with `a` programs, the second with `b`; the first's section a row taller while a program would go into it
    /// (`filling`).
    fn two(a: usize, b: usize, filling: bool) -> Vec<Section> {
        let section = |top: f32, n: usize, grown: bool| {
            let list = top + 34.0;
            let end = list + (n + usize::from(grown)).max(1) as f32 * 30.0;
            Section { x: 0.0, right: 560.0, top, list, bottom: end, first: list, pitch: 30.0 }
        };
        let first = section(100.0, a, filling);
        let mut second = section(first.bottom, b, false);
        second.bottom += 16.0;
        vec![first, second]
    }

    /// A program dragged over the first partner while the second is there lands in the first's list where the pointer
    /// is (its top, between its rows, its end), never in the other's; over the second's head, at the second's top.
    #[test]
    fn a_program_lands_where_the_pointer_is() {
        let counts = [3, 2];
        let s = two(3, 2, false);
        // (Its head: the top; its first row's upper half: before it; between rows 1 and 2: there; its last row's lower
        // half and past it, up to the second's head: the end.)
        assert_eq!(landing_in(&s, &counts, (200.0, 110.0)), Some((0, 0)));
        assert_eq!(landing_in(&s, &counts, (200.0, 134.0 + 10.0)), Some((0, 0)));
        assert_eq!(landing_in(&s, &counts, (200.0, 134.0 + 58.0)), Some((0, 2)));
        assert_eq!(landing_in(&s, &counts, (200.0, 134.0 + 80.0)), Some((0, 3)));
        assert_eq!(landing_in(&s, &counts, (200.0, s[1].top - 1.0)), Some((0, 3)));
        // (The second's head, and below its last row a little.)
        assert_eq!(landing_in(&s, &counts, (200.0, s[1].top + 1.0)), Some((1, 0)));
        assert_eq!(landing_in(&s, &counts, (200.0, s[1].bottom - 1.0)), Some((1, 2)));
        assert_eq!(landing_in(&s, &counts, (200.0, s[1].bottom + 1.0)), None);
        assert_eq!(landing_in(&s, &counts, (600.0, 150.0)), None);
        // (The first grown by the gap at its end: the pointer in the gap is still its end; the second, moved down,
        // still its own.)
        let grown = two(3, 2, true);
        assert_eq!(landing_in(&grown, &counts, (200.0, 134.0 + 3.0 * 30.0 + 20.0)), Some((0, 3)));
        assert_eq!(landing_in(&grown, &counts, (200.0, grown[1].top + 1.0)), Some((1, 0)));
        // (An empty first: anywhere in it, the top.)
        let empty = two(0, 2, false);
        assert_eq!(landing_in(&empty, &[0, 2], (200.0, 120.0)), Some((0, 0)));
        assert_eq!(landing_in(&empty, &[0, 2], (200.0, empty[0].bottom - 1.0)), Some((0, 0)));
    }

    /// EXE5's party read from its schema (two partners, each a navi and ten programs of PartyCust's), and a party
    /// written and read back; a partner's list by its name.
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
        assert_eq!((partner_of_list(&installed_list(1)), partner_of_list("lib.rows")), (Some(1), None));
    }
}
