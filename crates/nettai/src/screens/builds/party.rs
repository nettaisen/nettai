//! The Party tab of a build's editor, in a game whose setup takes a party (EXE5's party battle, EXE5 DS's:
//! docs/design/exe5ds-map.md §4, §11): the navi's partners, each with its PartyCust programs filling its gauge (§4.10).
//!
//! **Its form is the setup's schema's**: the field is a list of records (its room the most partners), each with a field
//! naming a navi and a list of entries of a collection (the programs; its room the most a partner holds). The tab reads
//! those from the schema and names neither. What may be a partner and what fits a gauge are the game's rules': the
//! navis a side can play but its own are offered, the build's problems say what the rules refuse (a navi twice, one
//! that is no team navi, programs past the gauge), and the gauge itself is the rules' overview's (a list budget of the
//! partner's entry: `Overview::budget_of`), as is a program's cost (the field the budget names). A partner's HP is the
//! round's as the build's check sets it up (`Checked::members`).
//!
//! **The panel's lines**: each partner, its programs under it and a place for one more, then a place for another
//! partner. **Enter on a line goes to the library** (on a phone, its sheet), which holds what that line takes: the
//! navis for a partner's line, the programs for a program's. **Enter on a navi** makes it the partner (a new one on the
//! place for another); on the partner's own navi, takes the partner out; one of the other partner's is refused. **Enter
//! on a program** puts it in the partner's programs, at their end (a program may be in twice: the rules hold them to
//! the gauge alone), refused where the gauge or the list has no room. **Del** on a line takes out what it holds (a
//! partner with its programs); **CLEAR** (or X) takes every partner out, asked first. A navi the build's ruleset holds
//! back (its `party` category) is greyed with "Not allowed", and refused.

use crate::input::{Action, KeyPress};
use crate::screens::builds::editor::{self, Draft, Tab};
use crate::shell::Shell;
use crate::sound::Cue;
use nettai_battle::Content;
use nettai_battle::rules::Fact;
use nettai_content_api::{EntryHandle, FieldType, NaviHandle, Registry, Value};
use nettai_match::build::{Budget, EntryRefusal};
use nettai_match::check::Problem;
use nettai_match::facts::Stated;
use nettai_ui::{BuildWords, EditorTab, PartyDetail, PartyLine, PartyRow, PartyView};
use slint::{ComponentHandle, ModelRc, VecModel};

/// The setup field of the party.
pub const FIELD: &str = "party";

/// The field of a program's definition that names its color.
const COLOR: &str = "color";

/// The party's shape, as the setup's schema says it.
#[derive(Clone)]
struct Shape {
    /// How many partners the list holds.
    room: usize,
    /// The record's field naming the partner's navi.
    navi: String,
    /// The record's field listing its programs, their collection, and how many it holds.
    programs: String,
    collection: String,
    program_room: usize,
}

impl Shape {
    fn of(content: &Content) -> Option<Shape> {
        let field = nettai_match::facts::field(content, FIELD)?;
        let FieldType::List(elem, room) = field.ty else { return None };
        let FieldType::Record(fields) = &**elem else { return None };
        let navi = fields.fields().iter().find(|f| matches!(f.ty, FieldType::Ref(Registry::Navi, _)))?.name.clone();
        let (programs, collection, program_room) = fields.fields().iter().find_map(|f| match &f.ty {
            FieldType::List(e, n) => match &**e {
                FieldType::Ref(Registry::Entry, Some(c)) => Some((f.name.clone(), c.clone(), *n as usize)),
                _ => None,
            },
            _ => None,
        })?;
        Some(Shape { room: *room as usize, navi, programs, collection, program_room })
    }
}

/// A partner as the build holds it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Partner {
    navi: Option<NaviHandle>,
    programs: Vec<EntryHandle>,
}

/// Program `h`'s color as a NaviCust program's is drawn (its definition's `color`, a name of the NaviCust's colors:
/// `crate::builds::grid::PALETTE`).
fn tone(content: &Content, h: EntryHandle) -> slint::Color {
    let key = &content.defs.entry(h).key;
    let name = content.defs.definitions.get(Registry::Entry, key).and_then(|d| d.spec.field(COLOR).str().map(str::to_string));
    let c = crate::builds::grid::rgb(name.as_deref());
    slint::Color::from_rgb_u8((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

/// The build's partners, in order.
fn partners(content: &Content, side: &nettai_match::Side, shape: &Shape) -> Vec<Partner> {
    let Some(Stated::List(items)) = side.facts.get(content, FIELD) else { return Vec::new() };
    items
        .iter()
        .map(|item| {
            let Stated::Record(fields) = item else { return Partner::default() };
            let get = |name: &str| fields.iter().find(|(n, _)| n == name).map(|(_, v)| v);
            let navi = match get(&shape.navi) {
                Some(Stated::Def(Registry::Navi, Some(h))) => Some(NaviHandle(*h)),
                _ => None,
            };
            let programs = match get(&shape.programs) {
                Some(Stated::List(xs)) => xs.iter().filter_map(|x| if let Stated::Def(Registry::Entry, Some(h)) = x { Some(EntryHandle(*h)) } else { None }).collect(),
                _ => Vec::new(),
            };
            Partner { navi, programs }
        })
        .collect()
}

/// Write the build's partners: whether the facts took them.
fn write(content: &Content, side: &mut nettai_match::Side, shape: &Shape, partners: &[Partner]) -> bool {
    let records: Vec<Fact> = partners
        .iter()
        .map(|p| {
            let navi = p.navi.map_or(Fact::Value(Value::Nil), |n| Fact::Value(Value::Def(Registry::Navi, n.0)));
            let programs = Fact::List(p.programs.iter().map(|h| Fact::Value(Value::Def(Registry::Entry, h.0))).collect());
            Fact::Record(vec![(shape.navi.as_str(), navi), (shape.programs.as_str(), programs)])
        })
        .collect();
    match side.set_fact(content, FIELD, &records) {
        Ok(()) => true,
        Err(why) => {
            crate::errors::log("changing a build's party", &why);
            false
        }
    }
}

/// A line of the panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Line {
    Partner(usize),
    Program(usize, usize),
    /// The place for one more program of the partner.
    AddProgram(usize),
    /// The place for another partner.
    AddPartner,
}

impl Line {
    /// The partner it is of (none: the place for another).
    fn partner(self) -> Option<usize> {
        match self {
            Line::Partner(i) | Line::Program(i, _) | Line::AddProgram(i) => Some(i),
            Line::AddPartner => None,
        }
    }

    /// What the library holds for it: the navis (else the programs).
    fn takes_navi(self) -> bool {
        matches!(self, Line::Partner(_) | Line::AddPartner)
    }
}

/// The panel's lines for `partners`.
fn lines(shape: &Shape, partners: &[Partner]) -> Vec<Line> {
    let mut out = Vec::new();
    for (i, p) in partners.iter().enumerate() {
        out.push(Line::Partner(i));
        out.extend((0..p.programs.len()).map(|k| Line::Program(i, k)));
        if p.programs.len() < shape.program_room {
            out.push(Line::AddProgram(i));
        }
    }
    if partners.len() < shape.room {
        out.push(Line::AddPartner);
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Described {
    Line(usize),
    Row(usize),
}

#[derive(Default)]
pub struct Party {
    shape: Option<Shape>,
    /// The panel's line the cursor was last on (the one the library fills).
    line: usize,
    in_library: bool,
    row: usize,
    /// The library's rows: the navis offered, or the programs, by what the line takes.
    navis: Vec<NaviHandle>,
    programs: Vec<EntryHandle>,
    described: Option<Described>,
    hovered: Option<Described>,
    refused: i32,
    /// Why the last thing asked wasn't done, until the cursor moves.
    notice: Option<String>,
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
        // (The navis a side can play but the build's own, in the game's order.)
        let content = draft.content();
        let own = draft.side().navi(content);
        self.navis = nettai_match::navis(content, &draft.kit.game).into_iter().filter(|&n| n != own).collect();
        self.shape.clone()
    }

    fn partners(&self, draft: &Draft) -> Vec<Partner> {
        self.shape.as_ref().map(|s| partners(draft.content(), draft.side(), s)).unwrap_or_default()
    }

    fn lines(&self, draft: &Draft) -> Vec<Line> {
        self.shape.as_ref().map(|s| lines(s, &self.partners(draft))).unwrap_or_default()
    }

    /// The line being filled.
    fn current(&self, draft: &Draft) -> Option<Line> {
        let lines = self.lines(draft);
        lines.get(self.line.min(lines.len().saturating_sub(1))).copied()
    }

    /// Each partner's gauge, by its entry: the rules' overview's budget of it.
    fn budgets(draft: &mut Draft) -> Vec<Option<Budget>> {
        let content = draft.content().clone();
        let side = draft.side().clone();
        let Some(o) = draft.checked.overview(&content, &side) else { return Vec::new() };
        (0..side.facts.get(&content, FIELD).map_or(0, |v| if let Stated::List(xs) = v { xs.len() } else { 0 })).map(|i| o.budget_of(FIELD, i).cloned()).collect()
    }

    /// A partner's HP as its round starts: its member's, as the build's check sets the round up.
    fn hp(draft: &Draft, navi: NaviHandle) -> Option<u16> {
        draft.checked.members().iter().skip(1).find(|s| s.navi == navi).map(|s| s.max_hp)
    }

    /// The build's problems with partner `i` (its own, and the party's about no partner).
    fn problems(draft: &Draft, i: usize) -> Vec<&Problem> {
        draft.checked.problems.iter().filter(|p| draft.facts_of(p).iter().any(|f| f == FIELD) && (p.at(i) || p.entry.is_none())).collect()
    }

    /// Everything shown anew.
    fn changed(&mut self, shell: &Shell, draft: &mut Draft) {
        let Some(shape) = self.read(draft) else { return };
        let content = draft.content().clone();
        let partners = partners(&content, draft.side(), &shape);
        let lines = lines(&shape, &partners);
        self.line = self.line.min(lines.len().saturating_sub(1));
        let budgets = Party::budgets(draft);
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        let pictures = &draft.kit.pictures;
        let shown: Vec<PartyLine> = draft.with_words(shell, |words| {
            lines
                .iter()
                .map(|&line| match line {
                    Line::Partner(i) => {
                        let p = &partners[i];
                        let budget = budgets.get(i).cloned().flatten();
                        // (Each block its program's color, the programs in their order.)
                        let tones: Vec<slint::Color> = p
                            .programs
                            .iter()
                            .flat_map(|&h| std::iter::repeat_n(tone(&content, h), budget.as_ref().map_or(0, |b| b.cost_of(&content, h)) as usize))
                            .collect();
                        PartyLine {
                            partner: true,
                            there: true,
                            tones: ModelRc::new(VecModel::from(tones)),
                            face: p.navi.map(|n| pictures.navi(n)).unwrap_or_default(),
                            name: p.navi.map(|n| words.names.navi(n)).unwrap_or_default().into(),
                            blocks: budget.as_ref().map_or(0, |b| b.count.used as i32),
                            gauge: budget.as_ref().map_or(0, |b| b.count.limit as i32),
                            note: p.navi.and_then(|n| Party::hp(draft, n)).map(|hp| view.invoke_hp_word(hp as i32)).unwrap_or_default(),
                            bad: !Party::problems(draft, i).is_empty(),
                        }
                    }
                    Line::Program(i, k) => {
                        let h = partners[i].programs[k];
                        let cost = budgets.get(i).cloned().flatten().map_or(0, |b| b.cost_of(&content, h));
                        PartyLine { there: true, name: words.names.entry(h).into(), blocks: cost as i32, tones: ModelRc::new(VecModel::from(vec![tone(&content, h); cost as usize])), ..Default::default() }
                    }
                    Line::AddProgram(_) => PartyLine::default(),
                    Line::AddPartner => PartyLine { partner: true, ..Default::default() },
                })
                .collect()
        });
        view.set_partners(partners.len() as i32);
        view.set_lines(ModelRc::new(VecModel::from(shown)));
        view.set_line(self.line as i32);
        view.set_problems(draft.pane_line(shell, &[FIELD.to_string()], false).into());
        self.show_library(shell, draft, &partners, &budgets);
        self.show_detail(shell, draft);
    }

    /// The library: what the line being filled takes, each with whether it is in or why it can't go in.
    fn show_library(&self, shell: &Shell, draft: &Draft, partners: &[Partner], budgets: &[Option<Budget>]) {
        let content = draft.content();
        let line = self.current(draft).unwrap_or(Line::AddPartner);
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        let say = ui.global::<BuildWords>();
        let rows: Vec<PartyRow> = draft.with_words(shell, |words| {
            if line.takes_navi() {
                let mine = line.partner().and_then(|i| partners.get(i)?.navi);
                self.navis
                    .iter()
                    .map(|&n| {
                        let mut row = PartyRow {
                            face: draft.kit.pictures.navi(n),
                            name: words.names.navi(n).into(),
                            version: words.kit.version_tag_of(words.names, Registry::Navi, &nettai_match::ruleset::Item { def: n.0, variant: None }),
                            ..Default::default()
                        };
                        let is_in = partners.iter().any(|p| p.navi == Some(n));
                        if !draft.allows(FIELD, n.0) {
                            row.note = say.invoke_not_allowed_here();
                            row.refused = !is_in;
                            row.bad = true;
                        } else if is_in {
                            row.note = view.invoke_in_party();
                            row.refused = mine != Some(n);
                        }
                        row
                    })
                    .collect()
            } else {
                let budget = line.partner().and_then(|i| budgets.get(i).cloned().flatten());
                self.programs
                    .iter()
                    .map(|&h| {
                        let mut row = PartyRow { name: words.names.entry(h).into(), tone: tone(content, h), ..Default::default() };
                        if let Some(b) = &budget {
                            row.blocks = b.cost_of(content, h) as i32;
                            if let Some(EntryRefusal::Needs { needs, .. }) = b.refusal(content, draft.side(), h) {
                                row.note = view.invoke_needs_blocks(needs as i32);
                                row.refused = true;
                                row.bad = true;
                            }
                        }
                        row
                    })
                    .collect()
            }
        });
        view.set_navis(line.takes_navi());
        view.set_rows(ModelRc::new(VecModel::from(rows)));
        view.set_in_library(self.in_library);
    }

    /// The strip: the line or the library's row under the cursor; while the pointer is on one, that one.
    fn show_detail(&self, shell: &Shell, draft: &mut Draft) {
        let content = draft.content().clone();
        let partners = self.partners(draft);
        let lines = self.lines(draft);
        let budgets = Party::budgets(draft);
        let line = self.current(draft).unwrap_or(Line::AddPartner);
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        let say = ui.global::<BuildWords>();
        let hovered = self.hovered.filter(|h| match *h {
            Described::Line(l) => l < lines.len(),
            Described::Row(r) => r < if line.takes_navi() { self.navis.len() } else { self.programs.len() },
        });
        let notice = if hovered.is_some_and(|h| Some(h) != self.described) { None } else { self.notice.clone() };
        let described = hovered.or(self.described);
        // (What the build's problems have against the partner the line is of.)
        let fault = |i: Option<usize>| i.map(|i| draft.line(shell, Party::problems(draft, i))).unwrap_or_default();
        let navi_detail = |n: NaviHandle, words: &crate::builds::words::Words| {
            let i = partners.iter().position(|p| p.navi == Some(n));
            let facts: Vec<String> = [
                Party::hp(draft, n).map(|hp| view.invoke_hp_word(hp as i32).to_string()),
                i.and_then(|i| budgets.get(i).cloned().flatten()).map(|b| view.invoke_gauge_word(b.count.used as i32, b.count.limit as i32).to_string()),
            ]
            .into_iter()
            .flatten()
            .collect();
            PartyDetail { there: true, face: draft.kit.pictures.navi(n), name: words.names.navi(n).into(), facts: facts.join(" · ").into(), ..Default::default() }
        };
        let program_detail = |h: EntryHandle, budget: Option<&Budget>, words: &crate::builds::words::Words| {
            let cost = budget.map_or(0, |b| b.cost_of(&content, h));
            PartyDetail {
                there: true,
                name: words.names.entry(h).into(),
                blocks: cost as i32,
                tone: tone(&content, h),
                facts: if budget.is_some() { view.invoke_blocks_word(cost as i32) } else { Default::default() },
                text: words.names.description(h).map(|d| crate::games::one_line(&d)).unwrap_or_default().into(),
                ..Default::default()
            }
        };
        let detail = draft.with_words(shell, |words| {
            let mut detail = match described {
                Some(Described::Line(l)) => match lines.get(l).copied() {
                    Some(Line::Partner(i)) => {
                        let mut d = partners[i].navi.map(|n| navi_detail(n, words)).unwrap_or_default();
                        d.fault = fault(Some(i)).into();
                        d
                    }
                    Some(Line::Program(i, k)) => {
                        let mut d = program_detail(partners[i].programs[k], budgets.get(i).and_then(Option::as_ref), words);
                        d.fault = fault(Some(i)).into();
                        d
                    }
                    Some(Line::AddProgram(i)) => PartyDetail { fault: fault(Some(i)).into(), ..Default::default() },
                    _ => PartyDetail::default(),
                },
                Some(Described::Row(r)) if line.takes_navi() => match self.navis.get(r) {
                    Some(&n) => {
                        let mut d = navi_detail(n, words);
                        if !draft.allows(FIELD, n.0) {
                            d.problem = say.invoke_not_allowed_here();
                        }
                        d
                    }
                    None => PartyDetail::default(),
                },
                Some(Described::Row(r)) => match self.programs.get(r) {
                    Some(&h) => {
                        let budget = line.partner().and_then(|i| budgets.get(i).cloned().flatten());
                        let mut d = program_detail(h, budget.as_ref(), words);
                        if let Some(EntryRefusal::Needs { needs, .. }) = budget.as_ref().and_then(|b| b.refusal(&content, draft.side(), h)) {
                            d.problem = view.invoke_needs_blocks(needs as i32);
                        }
                        d
                    }
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

    /// Refuse what was asked of library row `row` (or of the line, with none), saying `why`: the row shakes.
    fn refuse(&mut self, shell: &mut Shell, draft: &mut Draft, row: Option<usize>, why: String) {
        shell.sound.play(Cue::Refused);
        self.refused += 1;
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        view.set_refused(row.map_or(self.line as i32, |r| r as i32));
        view.set_refused_seq(self.refused);
        self.notice = Some(why);
        self.show_detail(shell, draft);
    }

    /// Enter on library row `row`: what the line being filled takes, put in.
    fn put(&mut self, shell: &mut Shell, draft: &mut Draft, row: usize) {
        let Some(line) = self.current(draft) else { return };
        let ui = shell.ui();
        let partners = self.partners(draft);
        if line.takes_navi() {
            let Some(&n) = self.navis.get(row) else { return };
            let mine = line.partner();
            if !draft.allows(FIELD, n.0) {
                let why = ui.global::<BuildWords>().invoke_not_allowed_here().to_string();
                return self.refuse(shell, draft, Some(row), why);
            }
            // (The partner's own navi: it is taken out. Another partner's: refused.)
            if let Some(at) = partners.iter().position(|p| p.navi == Some(n)) {
                if Some(at) == mine {
                    return self.remove(shell, draft, Line::Partner(at));
                }
                let why = ui.global::<PartyView>().invoke_in_party().to_string();
                return self.refuse(shell, draft, Some(row), why);
            }
            let changed = self.change(draft, |ps| {
                match mine {
                    Some(i) => ps[i].navi = Some(n),
                    None => ps.push(Partner { navi: Some(n), programs: Vec::new() }),
                }
                true
            });
            if changed {
                shell.sound.play(Cue::Pick);
                // (The line filled is the partner's: a new one's is where the place for another was.)
                self.line = self.lines(draft).iter().position(|l| *l == Line::Partner(mine.unwrap_or(partners.len()))).unwrap_or(self.line);
                self.notice = None;
                self.changed(shell, draft);
            }
            return;
        }
        let Some(&h) = self.programs.get(row) else { return };
        let Some(i) = line.partner() else { return };
        let budgets = Party::budgets(draft);
        let room = self.shape.as_ref().map_or(0, |s| s.program_room);
        if partners[i].programs.len() >= room {
            return shell.sound.play(Cue::Refused);
        }
        if let Some(EntryRefusal::Needs { needs, .. }) = budgets.get(i).cloned().flatten().and_then(|b| b.refusal(draft.content(), draft.side(), h)) {
            let why = ui.global::<PartyView>().invoke_needs_blocks(needs as i32).to_string();
            return self.refuse(shell, draft, Some(row), why);
        }
        if self.change(draft, |ps| {
            ps[i].programs.push(h);
            true
        }) {
            shell.sound.play(Cue::Pick);
            // (The cursor's line stays the place for one more, which moved one down.)
            if let Some(at) = self.lines(draft).iter().position(|l| *l == Line::AddProgram(i)) {
                self.line = at;
            }
            self.notice = None;
            self.changed(shell, draft);
        }
    }

    /// Take out what line `line` holds: a partner, with its programs; a program.
    fn remove(&mut self, shell: &mut Shell, draft: &mut Draft, line: Line) {
        let changed = self.change(draft, |ps| match line {
            Line::Partner(i) if i < ps.len() => {
                ps.remove(i);
                true
            }
            Line::Program(i, k) if ps.get(i).is_some_and(|p| k < p.programs.len()) => {
                ps[i].programs.remove(k);
                true
            }
            _ => false,
        });
        if !changed {
            return shell.sound.play(Cue::Refused);
        }
        shell.sound.play(Cue::Back);
        self.notice = None;
        self.changed(shell, draft);
    }

    /// The cursor came onto line `l`: the library holds what it takes.
    fn on_line(&mut self, shell: &Shell, draft: &mut Draft, l: usize) {
        self.line = l;
        self.in_library = false;
        self.notice = None;
        self.described = Some(Described::Line(l));
        self.changed(shell, draft);
    }

    fn leave_library(&mut self, shell: &mut Shell) {
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        if view.get_sheet() {
            view.set_sheet(false);
            shell.nav.close("lib.");
        }
        self.in_library = false;
        self.described = Some(Described::Line(self.line));
        view.set_in_library(false);
        shell.nav.focus("party.line", self.line as i32);
    }

    fn focused_line(shell: &Shell) -> Option<usize> {
        shell.nav.focused().filter(|k| k.id == "party.line").map(|k| k.index.max(0) as usize)
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
        "party.line"
    }

    fn show(&mut self, shell: &mut Shell, draft: &mut Draft) {
        if self.described.is_none() {
            self.described = Some(Described::Line(self.line));
        }
        self.changed(shell, draft);
    }

    fn hide(&mut self, shell: &mut Shell, _draft: &mut Draft) {
        self.notice = None;
        self.hovered = None;
        let ui = shell.ui();
        if ui.global::<PartyView>().get_sheet() {
            ui.global::<PartyView>().set_sheet(false);
            shell.nav.close("lib.");
        }
    }

    fn activate(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32) {
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        match id {
            "party.line" => {
                self.on_line(shell, draft, index.max(0) as usize);
                self.in_library = true;
                view.set_in_library(true);
                // (The navi a partner's line holds, else the library's first row.)
                let line = self.current(draft);
                let partners = self.partners(draft);
                let at = match line {
                    Some(Line::Partner(i)) => partners[i].navi.and_then(|n| self.navis.iter().position(|&m| m == n)).unwrap_or(0),
                    _ => self.row.min(if line.is_some_and(Line::takes_navi) { self.navis.len() } else { self.programs.len() }.saturating_sub(1)),
                };
                let to = crate::nav::Key::new("lib.rows", at as i32);
                if shell.scale.phone {
                    view.set_sheet(true);
                    shell.nav.open("lib.", to);
                } else {
                    shell.nav.focus(&to.id, to.index);
                }
            }
            "lib.rows" => self.put(shell, draft, index.max(0) as usize),
            "lib.remove" => {
                if let Some(line) = self.current(draft) {
                    self.remove(shell, draft, line);
                }
                self.leave_library(shell);
            }
            "lib.close" => self.leave_library(shell),
            "clear" => editor::ask_clear(shell, view.get_clear_question(), self.partners(draft).is_empty()),
            _ => {}
        }
    }

    fn hovered(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, on: bool) {
        let what = match id {
            "party.line" => Described::Line(index.max(0) as usize),
            "lib.rows" => Described::Row(index.max(0) as usize),
            _ => return,
        };
        if editor::hover(&mut self.hovered, what, on) {
            self.show_detail(shell, draft);
        }
    }

    fn focused(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32) {
        if self.hovered.take().is_some() && !matches!(id, "party.line" | "lib.rows") {
            self.show_detail(shell, draft);
        }
        match id {
            "party.line" => self.on_line(shell, draft, index.max(0) as usize),
            "lib.rows" => {
                self.in_library = true;
                self.row = index.max(0) as usize;
                self.described = Some(Described::Row(self.row));
                self.notice = None;
                shell.ui().global::<PartyView>().set_in_library(true);
                self.show_detail(shell, draft);
            }
            _ => {}
        }
    }

    /// X, CLEAR.
    fn key(&mut self, shell: &mut Shell, draft: &mut Draft, key: &KeyPress) -> bool {
        if editor::clear_key(shell, key) {
            self.activate(shell, draft, "clear", 0);
            return true;
        }
        false
    }

    /// Delete takes out what the line holds.
    fn action(&mut self, shell: &mut Shell, draft: &mut Draft, action: Action) -> bool {
        if action != Action::Delete {
            return false;
        }
        let at = Party::focused_line(shell).unwrap_or(self.line);
        match self.lines(draft).get(at).copied() {
            Some(line @ (Line::Partner(_) | Line::Program(..))) => self.remove(shell, draft, line),
            _ => shell.sound.play(Cue::Refused),
        }
        true
    }

    /// Esc in the library goes back to the line being filled.
    fn back(&mut self, shell: &mut Shell, _draft: &mut Draft) -> bool {
        if shell.nav.focused().is_some_and(|k| k.id.starts_with("lib.")) || shell.ui().global::<PartyView>().get_sheet() {
            self.leave_library(shell);
            return true;
        }
        false
    }

    fn answered(&mut self, shell: &mut Shell, draft: &mut Draft, tag: &str, choice: usize) {
        if editor::cleared(tag, choice) && self.change(draft, |ps| !std::mem::take(ps).is_empty()) {
            self.line = 0;
            self.changed(shell, draft);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::testing::exe5_content;

    /// EXE5's party read from its schema (two partners, each a navi and ten programs of PartyCust's), its lines, and a
    /// party written and read back.
    #[test]
    fn the_party_is_the_schemas() {
        let content = exe5_content();
        let shape = Shape::of(&content).unwrap();
        assert_eq!((shape.room, shape.navi.as_str(), shape.programs.as_str(), shape.collection.as_str(), shape.program_room), (2, "navi", "programs", "partycust_programs", 10));
        let builds = nettai_match::build::Builds::of(&content, "exe5").unwrap();
        let mut side = builds.new_build(&content, "protoman").unwrap();
        assert_eq!(partners(&content, &side, &shape), Vec::new());
        assert_eq!(lines(&shape, &[]), [Line::AddPartner]);
        let navi = |n: &str| Some(nettai_match::ids::navi(&content, "exe5", n).unwrap());
        let program = |n: &str| nettai_match::ids::entry(&content, "exe5", "partycust_programs", n).unwrap();
        let party = vec![Partner { navi: navi("toadman"), programs: vec![program("p-spport"), program("p-spport")] }, Partner { navi: navi("protoman"), programs: Vec::new() }];
        assert!(write(&content, &mut side, &shape, &party));
        assert_eq!(partners(&content, &side, &shape), party);
        assert_eq!(lines(&shape, &party), [Line::Partner(0), Line::Program(0, 0), Line::Program(0, 1), Line::AddProgram(0), Line::Partner(1), Line::AddProgram(1)]);
    }
}
