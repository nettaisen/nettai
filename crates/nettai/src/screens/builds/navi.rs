//! The Navi tab of a build's editor (docs/design/app-ui.md §6, "Navi"): the game's navis as cards, the build's
//! navi beside them (the face its round starts with, its HP then, its element), under it the selector of the
//! forms the build has (docs/design/versions.md §2.1; the user: "you probably need to have some kind of soul and
//! cross and cybeast selector in the editor now right?": `Builds::form_choices`), and the build's other choices
//! as steppers (the game's builds data's: Light or Dark, of the game's own navi alone).
//!
//! The selector: each fact of forms the build chooses, its forms' faces in the battle's order, both versions'
//! (EXE6's Cybeast at its head, Gregar's or Falzar's, one always chosen; then its Crosses, as many as the Cross
//! window offers; EXE4's and EXE5's souls, as many as the soul button's list). A face lit is the build's; Enter, a
//! click or a tap on one chooses it, or takes it out of a list (`Builds::choose_form`). A face the ruleset the
//! player plays by doesn't allow has a ring in the error red. The lists are the game's own navi's alone; the
//! Cybeast is every navi's (MstrCros plays its version's Crosses).
//!
//! Choosing a navi is `Draft::choose_navi` (nettai-match's `Builds::set_navi`, with what is the game's own
//! navi's alone kept aside in the build's file); a choice's preset is `Builds::take_preset`.

use crate::screens::builds::editor::{Draft, Tab};
use crate::shell::Shell;
use nettai_battle::Content;
use nettai_battle::content::{Element, PlayerFact};
use nettai_content_api::{FormHandle, NaviHandle};
use nettai_match::build::TEXT_TABLE;
use nettai_ui::{EditorTab, NaviCard, NaviChoice, NaviForm, NaviFormGroup, NaviView};
use slint::{ComponentHandle, ModelRc, VecModel};

#[derive(Default)]
pub struct Navi {
    /// The game's navis, as the grid has them.
    navis: Vec<NaviHandle>,
    /// The selector's forms, each with the fact it is of, in the focus order.
    forms: Vec<(String, FormHandle)>,
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
            // Each of the game's choices: the preset the build is on, by its name in the game's strings.
            let choices: Vec<NaviChoice> = kit
                .builds
                .choices
                .iter()
                .map(|c| {
                    let preset = offered.iter().any(|o| o.id == c.id).then(|| kit.builds.preset_of(&content, draft.side(), &c.id)).flatten();
                    let value = preset.map(|p| names.text(TEXT_TABLE, &c.text_key(p)).unwrap_or_else(|| p.id.clone()));
                    NaviChoice { offered: value.is_some(), value: value.unwrap_or_default().into() }
                })
                .collect();
            view.set_choices(ModelRc::new(VecModel::from(choices)));
            let own = nettai_match::first_navi(&content, &kit.game).map(|n| names.navi(n)).unwrap_or_default();
            view.set_own(own.into());
        });
        // The selector: each fact of forms the build chooses, named as the game's rulesets name it ("CROSSES"), with how
        // many it has of its room (none said of the one always chosen), its forms lit where they are the build's.
        let choices = kit.builds.form_choices(&content, draft.side());
        self.forms = choices.iter().flat_map(|c| c.forms.iter().map(|&f| (c.fact.clone(), f))).collect();
        let groups: Vec<NaviFormGroup> = draft.with_words(shell, |words| {
            let mut at = 0;
            choices
                .iter()
                .map(|c| {
                    let chosen = kit.builds.chosen_forms(&content, draft.side(), &c.fact);
                    let forms: Vec<NaviForm> = c.forms.iter().map(|&f| NaviForm { face: kit.pictures.face(f), chosen: chosen.contains(&f), banned: !draft.allows(&c.fact, f.0) }).collect();
                    let name = words.names.text(nettai_match::ruleset::TEXT_TABLE, &format!("category.{}", c.fact)).unwrap_or_else(|| c.fact.clone());
                    let count = if c.one { String::new() } else { format!("{}/{}", chosen.len(), c.room) };
                    let group = NaviFormGroup { name: name.to_uppercase().into(), count: count.into(), forms: ModelRc::new(VecModel::from(forms)), at };
                    at += c.forms.len() as i32;
                    group
                })
                .collect()
        });
        view.set_form_groups(ModelRc::new(VecModel::from(groups)));
        let ruleset = draft.with_words(shell, |words| draft.ruleset.as_ref().map(|p| crate::rulesets::shown_name(words.names, &p.ruleset.name)));
        view.set_ruleset(ruleset.unwrap_or_default().into());
        view.set_face(face);
        view.set_hp(hp.into());
        view.set_element(kit.pictures.element(family(&content, content.navi(chosen).element)));
        // (The focus starts on the build's navi's card.)
        if shell.nav.focused().is_none_or(|k| !["navi", "form", "choice", "tab", "ruleset", "ruleset-edit"].contains(&k.id.as_str()))
            && let Some(at) = self.navis.iter().position(|&n| n == chosen)
        {
            shell.nav.focus("navi", at as i32);
        }
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
            // (Enter on a choice takes its next preset, as Right does.)
            "choice" => self.step(shell, draft, id, index, 1),
            _ => {}
        }
    }

    fn step(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, by: i32) {
        if id != "choice" {
            return;
        }
        let kit = draft.kit.clone();
        let Some(choice) = kit.builds.choices.get(index.max(0) as usize) else { return };
        let Some(now) = kit.builds.preset_of(draft.content(), draft.side(), &choice.id) else { return };
        let at = choice.presets.iter().position(|p| p.id == now.id).unwrap_or(0) as i32;
        let next = &choice.presets[(at + by).rem_euclid(choice.presets.len() as i32) as usize];
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
            self.show(shell, draft);
        }
    }
}
