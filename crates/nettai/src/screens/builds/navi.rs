//! The Navi tab of a build's editor (docs/design/app-ui.md §6, "Navi"): the game's navis as cards, the build's
//! navi beside them (the face its round starts with, its HP then, its element), under it the build's other choices
//! (the game's builds data's: Light or Dark, of the game's own navi alone), each a segmented control of its presets
//! (docs/design/app-ui.md §2.63), then the selector of the forms the build has (docs/design/versions.md §2.1; the
//! user: "you probably need to have some kind of soul and cross and cybeast selector in the editor now right?":
//! `Builds::form_choices`).
//!
//! The selector: each fact of forms the build chooses, by its name in the game's rulesets' words (EXE6's "Beast
//! form", the user: "instead of "Cybeasts" in bn6 it should really be just "Beast form""), its forms in the battle's
//! order, both versions' (EXE6's Cybeast at its head, Gregar's or Falzar's, one always chosen; then its Crosses, as
//! many as the Cross window offers; EXE4's and EXE5's souls, as many as the soul button's list), each a row of the
//! editor's lists (the user: "the rows in the editor are too tall, they should match the height of the regular
//! editor"): checked where it is the build's, its face, its name, and the mark of the version it is one's own (the
//! user: "you should indicate in the editor version exclusives"). Enter, a click or a tap on one chooses it, or takes
//! it out of a list (`Builds::choose_form`). A list has ALL and NONE (docs/design/app-ui.md §2.69; the user: "the
//! souls/crosses should be able to select all/none") on each version's head, or on its heading where it shows no
//! versions: that group's forms put in while the list has room, the rest refused, or taken out (`Builds::choose_forms`,
//! one change). What is wrong with the build is marked where it is (docs/design/app-ui.md
//! §2.66; the user: "if there's a problem with e.g. crosses or souls there's nowhere in the ui that shows it"): a
//! fact's heading in the error red where it has a problem, and a form of the build's a problem is about, its name; a
//! form the ruleset doesn't allow says so at its row's end, as a library's row does. The tab has no strip (the user:
//! "i don't like having an info thing on the navi view, it's redundant"): what the build's problems have against the
//! form under the cursor or the pointer, the ruleset's among them (its own: one the ruleset holds back, a version's
//! in a build that mixes versions; then its fact's about no form of it), else against the navi (and the build where
//! no tab shows what it is about), is said in the note's place under the selector, kept for it, so nothing moves when
//! one appears (the user: "error messages for beast form/crosses ends up shifting layout"). The lists are the game's
//! own navi's alone; the Cybeast is every navi's (MstrCros plays its version's Crosses).
//!
//! Choosing a navi is `Draft::choose_navi` (nettai-match's `Builds::set_navi`, with what is the game's own
//! navi's alone kept aside in the build's file); a choice's preset is `Builds::take_preset`.

use crate::screens::builds::editor::{self, Draft, Tab};
use crate::shell::Shell;
use nettai_battle::Content;
use nettai_battle::content::{Element, PlayerFact};
use nettai_content_api::{FormHandle, NaviHandle, Registry};
use nettai_match::Wrong;
use nettai_match::build::TEXT_TABLE;
use nettai_match::check::Problem;
use nettai_match::ruleset::Item;
use nettai_ui::{EditorTab, NaviCard, NaviChoice, NaviForm, NaviFormGroup, NaviView};
use slint::{ComponentHandle, ModelRc, VecModel};

#[derive(Default)]
pub struct Navi {
    /// The game's navis, as the grid has them.
    navis: Vec<NaviHandle>,
    /// The selector's forms, each with the fact it is of, in the focus order.
    forms: Vec<(String, FormHandle)>,
    /// What each ALL and NONE of the selector is of (their focus keys' index): a fact of forms, and its version's
    /// group's forms (or the whole list's, where it shows no versions).
    sets: Vec<(String, Vec<FormHandle>)>,
    /// The form the pointer is on (its place in the focus order), whose problems the note's place says in place of the
    /// focused one's (`editor::hover`).
    hovered: Option<usize>,
}

/// The chip family whose icon the game shows for `element`: the family of the element's name, a navi of no
/// element's the family of the chips of none.
fn family(content: &Content, element: Element) -> nettai_battle::content::ChipFamily {
    let families = &content.rules().chip_families;
    let named = match element {
        Element::Null => None,
        Element::Fire => families.by_name("fire"),
        Element::Aqua => families.by_name("aqua"),
        Element::Elec => families.by_name("elec"),
        Element::Wood => families.by_name("wood"),
    };
    named.unwrap_or(families.non_elemental)
}

/// Whether problem `problem` of the build's fact `fact` of forms (`chosen`, in order) is about form `form`: a form the
/// ruleset doesn't allow, one of a version's in a build that mixes versions, or the entry it names.
fn about(problem: &Problem, fact: &str, chosen: &[FormHandle], form: FormHandle) -> bool {
    match &problem.what {
        Wrong::NotAllowed { registry: Registry::Form, item, .. } => item.def == form.0,
        Wrong::Mixed { items, .. } => items.contains(&(Registry::Form, Item::of(form.0))),
        _ => problem.field.as_deref() == Some(fact) && problem.entry.and_then(|e| chosen.get(e)) == Some(&form),
    }
}

impl Navi {
    /// The navi's fact, by its setup field's name.
    fn navi_fact(content: &Content) -> Vec<String> {
        content.defs.fact_name(PlayerFact::Navi).map(str::to_string).into_iter().collect()
    }

    /// What the note's place says in the note's stead (docs/design/app-ui.md §2.66): what the build's problems have
    /// against the form under the pointer or the cursor, the ruleset's among them, where they have anything; else
    /// against the navi, and the build where no tab shows what it is about. Empty: the note.
    fn show_fault(&self, shell: &Shell, draft: &Draft) {
        let content = draft.content();
        let focused = shell.nav.focused().filter(|k| k.id == "form").map(|k| k.index.max(0) as usize);
        let of_form: Vec<&Problem> = match self.hovered.or(focused).and_then(|i| self.forms.get(i)) {
            Some((fact, form)) => {
                let picked = draft.kit.builds.chosen_forms(content, draft.side(), fact);
                // (Its own problems, then its fact's that are about no form of it: a fact a build made by hand states
                // wrong, which every form of it says.)
                let of_fact: Vec<&Problem> = draft.checked.problems.iter().filter(|p| draft.facts_of(p).contains(fact)).collect();
                let own = of_fact.iter().copied().filter(|p| about(p, fact, &picked, *form));
                let unplaced = of_fact.iter().copied().filter(|p| !picked.iter().any(|&f| about(p, fact, &picked, f)));
                own.chain(unplaced).collect()
            }
            None => Vec::new(),
        };
        let problems = if of_form.is_empty() { draft.pane_problems(&Navi::navi_fact(content), true) } else { of_form };
        shell.ui().global::<NaviView>().set_fault(draft.line(shell, problems).into());
    }
}

impl Tab for Navi {
    fn id(&self) -> EditorTab {
        EditorTab::Navi
    }

    /// The navi, and the forms the build chooses.
    fn fields(&self, content: &Content) -> Vec<String> {
        let navi = content.defs.fact_name(PlayerFact::Navi).map(str::to_string);
        navi.into_iter().chain(nettai_match::facts::form_facts(content).iter().map(|f| f.name.to_string())).collect()
    }

    fn main(&self) -> &'static str {
        "navi"
    }

    fn show(&mut self, shell: &mut Shell, draft: &mut Draft) {
        let content = draft.content().clone();
        let kit = draft.kit.clone();
        // (The game's own navi first: the content states no order of the others, which are by their keys.)
        self.navis = nettai_match::navis(&content, &kit.game);
        self.navis.sort_by_key(|&n| !content.navi(n).changes_form());
        let chosen = draft.side().navi(&content);
        let ui = shell.ui();
        let view = ui.global::<NaviView>();
        let face = draft.checked.face().map(|f| kit.pictures.shown(&f)).unwrap_or_else(|| kit.pictures.navi(chosen));
        let hp = draft.checked.stats().map(|s| s.max_hp.to_string()).unwrap_or_default();
        let offered = kit.builds.choices_of(&content, draft.side());
        draft.with_words(shell, |words| {
            let names = words.names;
            // (A navi the ruleset the player plays by doesn't allow is marked: the navi fact's category.)
            let navi_fact = content.defs.fact_name(PlayerFact::Navi).unwrap_or_default();
            let cards: Vec<NaviCard> =
                self.navis.iter().map(|&n| NaviCard { name: names.navi(n).into(), face: kit.pictures.navi(n), chosen: n == chosen, banned: !draft.allows(navi_fact, n.0) }).collect();
            view.set_navis(ModelRc::new(VecModel::from(cards)));
            view.set_name(names.navi(chosen).into());
            // Each of the game's choices the build's navi has (a navi with none of one has nothing in its place): its
            // presets by their names in the game's strings, in capitals, a segment each, and the one the build is on.
            let choices: Vec<NaviChoice> = kit
                .builds
                .choices
                .iter()
                .enumerate()
                .filter(|(_, c)| offered.iter().any(|o| o.id == c.id))
                .filter_map(|(i, c)| {
                    let now = kit.builds.preset_of(&content, draft.side(), &c.id)?;
                    let presets: Vec<slint::SharedString> =
                        c.presets.iter().map(|p| names.text(TEXT_TABLE, &c.text_key(p)).unwrap_or_else(|| p.id.clone()).to_uppercase().into()).collect();
                    let chosen = c.presets.iter().position(|p| p.id == now.id).unwrap_or(0) as i32;
                    Some(NaviChoice { presets: ModelRc::new(VecModel::from(presets)), chosen, index: i as i32 })
                })
                .collect();
            view.set_choices(ModelRc::new(VecModel::from(choices)));
            let own = nettai_match::first_navi(&content, &kit.game).map(|n| names.navi(n)).unwrap_or_default();
            view.set_own(own.into());
        });
        // The selector: each fact of forms the build chooses, named as the game's rulesets name it ("CROSSES"), with how
        // many it has of its room (none said of the one always chosen), in the error red where it has a problem; its
        // forms checked where they are the build's, under their versions' heads, a form's name in the error red where a
        // problem is about it, and "Not allowed" at the end of one the ruleset holds back.
        let choices = kit.builds.form_choices(&content, draft.side());
        self.forms = choices.iter().flat_map(|c| c.forms.iter().map(|&f| (c.fact.clone(), f))).collect();
        // (One column of rows on a desktop, two on a phone.)
        let columns = if shell.scale.phone { 2 } else { 1 };
        let mut sets: Vec<(String, Vec<FormHandle>)> = Vec::new();
        let groups: Vec<NaviFormGroup> = draft.with_words(shell, |words| {
            let mut at = 0;
            choices
                .iter()
                .map(|c| {
                    let chosen = kit.builds.chosen_forms(&content, draft.side(), &c.fact);
                    let problems: Vec<&Problem> = draft.checked.problems.iter().filter(|p| draft.facts_of(p).contains(&c.fact)).collect();
                    // (Grouped by version, in the groups' order, the battle's within each, as `form_choices` sorts
                    // them, those of none last: each group headed by its version's name, its rows from a row of
                    // their own, in the columns the panel has. Where each of two versions has one form, as EXE6's
                    // Beast form's GBeast and FBeast, the two groups are side by side, a column each, one row under
                    // their heads.)
                    let versions: Vec<Option<String>> = c.forms.iter().map(|f| content.defs.forms.get(f.index()).and_then(|d| d.record.version.clone())).collect();
                    let side_by_side = versions.len() == 2 && versions[0].is_some() && versions[1].is_some() && versions[0] != versions[1];
                    // (A list's ALL and NONE: on each version's head where its forms are grouped by version, a group of
                    // none's too, else on the fact's heading; whether each would do anything, ALL where the list has
                    // room for one of them it lacks, NONE where it has one. A fact of one form has none.)
                    let mut set_of = |forms: Vec<FormHandle>| -> (i32, bool, bool) {
                        if c.one {
                            return (-1, false, false);
                        }
                        let all = chosen.len() < c.room && forms.iter().any(|f| !chosen.contains(f));
                        let none = forms.iter().any(|f| chosen.contains(f));
                        sets.push((c.fact.clone(), forms));
                        (sets.len() as i32 - 1, all, none)
                    };
                    let by_version = versions.iter().any(Option::is_some);
                    let whole = if by_version { (-1, false, false) } else { set_of(c.forms.clone()) };
                    let mut forms: Vec<NaviForm> = Vec::new();
                    let (mut rows, mut heads, mut first_row, mut place) = (0, 0, 0, 0);
                    let mut last: Option<Option<String>> = None;
                    for (n, (&f, version)) in c.forms.iter().zip(versions.clone()).enumerate() {
                        let tag = kit.version_tag_of(words.names, Registry::Form, &Item::of(f.0));
                        let starts = last.as_ref() != Some(&version);
                        let (set, all, none) = if starts && by_version {
                            set_of(c.forms.iter().zip(&versions).skip(n).take_while(|(_, v)| **v == version).map(|(&f, _)| f).collect())
                        } else {
                            (-1, false, false)
                        };
                        if starts && !side_by_side {
                            // (A group of none after the versions' has a head's room, and no name.)
                            if version.is_some() || last.is_some() {
                                heads += 1;
                            }
                            first_row = rows;
                            place = 0;
                        }
                        last = Some(version);
                        let (column, row) = if side_by_side { (n as i32, 0) } else { (place % columns, first_row + place / columns) };
                        if side_by_side {
                            heads = 1;
                        }
                        rows = rows.max(row + 1);
                        forms.push(NaviForm {
                            face: kit.pictures.face(f),
                            name: words.names.form(f).into(),
                            head: if starts { tag } else { Default::default() },
                            column,
                            row,
                            heads,
                            chosen: chosen.contains(&f),
                            flagged: problems.iter().any(|p| about(p, &c.fact, &chosen, f)),
                            banned: !draft.allows(&c.fact, f.0),
                            set,
                            span: if side_by_side { 1 } else { columns },
                            all,
                            none,
                        });
                        place += 1;
                    }
                    let name = words.names.text(nettai_match::ruleset::TEXT_TABLE, &format!("category.{}", c.fact)).unwrap_or_else(|| c.fact.clone());
                    let count = if c.one { String::new() } else { format!("{}/{}", chosen.len(), c.room) };
                    let columns = if side_by_side { 2 } else { columns };
                    let group = NaviFormGroup {
                        name: name.to_uppercase().into(),
                        count: count.into(),
                        flagged: !problems.is_empty(),
                        columns,
                        rows,
                        heads,
                        forms: ModelRc::new(VecModel::from(forms)),
                        at,
                        set: whole.0,
                        all: whole.1,
                        none: whole.2,
                    };
                    at += c.forms.len() as i32;
                    group
                })
                .collect()
        });
        self.sets = sets;
        view.set_form_groups(ModelRc::new(VecModel::from(groups)));
        let ruleset = draft.with_words(shell, |words| draft.ruleset.as_ref().map(|p| crate::rulesets::shown_name(words.names, &p.ruleset.name)));
        view.set_ruleset(ruleset.unwrap_or_default().into());
        view.set_face(face);
        view.set_hp(hp.into());
        view.set_element(kit.pictures.element(family(&content, content.navi(chosen).element)));
        // (The focus starts on the build's navi's card.)
        if shell.nav.focused().is_none_or(|k| !["navi", "form", "forms-all", "forms-none", "choice", "tab", "ruleset", "ruleset-edit"].contains(&k.id.as_str()))
            && let Some(at) = self.navis.iter().position(|&n| n == chosen)
        {
            shell.nav.focus("navi", at as i32);
        }
        self.show_fault(shell, draft);
    }

    fn hide(&mut self, _shell: &mut Shell, _draft: &mut Draft) {
        self.hovered = None;
    }

    /// The pointer came onto a form's row, or left it: the note's place says the form's problems while the pointer is
    /// on it.
    fn hovered(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, on: bool) {
        if id != "form" {
            return;
        }
        if editor::hover(&mut self.hovered, index.max(0) as usize, on) {
            self.show_fault(shell, draft);
        }
    }

    /// (The note's place follows the focus, the last thing moved, until the pointer goes onto another form.)
    fn focused(&mut self, shell: &mut Shell, draft: &mut Draft, _id: &str, _index: i32) {
        self.hovered = None;
        self.show_fault(shell, draft);
    }

    fn activate(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32) {
        match id {
            "navi" => {
                let Some(&navi) = self.navis.get(index.max(0) as usize) else { return };
                if draft.choose_navi(navi) {
                    self.show(shell, draft);
                }
            }
            // (A form of the selector: chosen, or in a list taken out.)
            "form" => {
                let Some((fact, form)) = self.forms.get(index.max(0) as usize).cloned() else { return };
                if draft.choose_form(&fact, form) {
                    shell.sound.play(crate::sound::Cue::Pick);
                    self.show(shell, draft);
                } else {
                    shell.sound.play(crate::sound::Cue::Refused);
                }
            }
            // (ALL or NONE of a version's group of forms, or of a whole list: as many put in as the list has room for,
            // in order, and the rest refused; or all taken out. One change.)
            "forms-all" | "forms-none" => {
                let Some((fact, forms)) = self.sets.get(index.max(0) as usize).cloned() else { return };
                let (changed, left) = draft.choose_forms(&fact, &forms, id == "forms-all");
                let cue = if changed && left == 0 { crate::sound::Cue::Pick } else { crate::sound::Cue::Refused };
                shell.sound.play(cue);
                if changed {
                    self.show(shell, draft);
                }
            }
            // (Enter on a choice takes its next preset, after the last its first: of two, the other.)
            "choice" => {
                if let Some(at) = preset_at(draft, index) {
                    take(self, shell, draft, index, at + 1);
                }
            }
            _ => {}
        }
    }

    /// Left or Right on a choice: its preset that way, none past its ends (a segmented control's).
    fn step(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, by: i32) {
        if id != "choice" {
            return;
        }
        let Some(at) = preset_at(draft, index) else { return };
        let count = draft.kit.builds.choices.get(index.max(0) as usize).map_or(0, |c| c.presets.len() as i32);
        let to = (at + by).clamp(0, count - 1);
        if to != at {
            take(self, shell, draft, index, to);
        }
    }

    /// A click or a tap on a choice's segment: its preset.
    fn choose(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, choice: i32) {
        if id == "choice" {
            take(self, shell, draft, index, choice);
        }
    }
}

/// Which of choice `index`'s presets the build is on (None: the build's navi has none of the choice).
fn preset_at(draft: &Draft, index: i32) -> Option<i32> {
    let kit = &draft.kit;
    let choice = kit.builds.choices.get(index.max(0) as usize)?;
    let now = kit.builds.preset_of(draft.content(), draft.side(), &choice.id)?;
    Some(choice.presets.iter().position(|p| p.id == now.id).unwrap_or(0) as i32)
}

/// The build takes preset `preset` of choice `index` (past the last, the first), and the tab shows it.
fn take(tab: &mut Navi, shell: &mut Shell, draft: &mut Draft, index: i32, preset: i32) {
    let kit = draft.kit.clone();
    let Some(choice) = kit.builds.choices.get(index.max(0) as usize) else { return };
    let Some(next) = choice.presets.get(preset.rem_euclid(choice.presets.len().max(1) as i32) as usize) else { return };
    let changed = draft.edit(|content, side| {
        let before = side.clone();
        match kit.builds.take_preset(content, side, &choice.id, &next.id) {
            Ok(()) => *side != before,
            Err(why) => {
                crate::errors::log("taking a build's preset", &why);
                false
            }
        }
    });
    if changed {
        tab.show(shell, draft);
    }
}
