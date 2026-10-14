//! The Navi tab of a build's editor (docs/design/app-ui.md §6, "Navi"): the game's navis as cards, the build's
//! navi beside them (the face its round starts with, its HP then, its element), the build's version as a stepper
//! (any navi's: the version decides the folder's Giga chips whatever the navi), with the forms it brings the game's
//! own navi under it while that navi is the build's (EXE6's Crosses, EXE4's and EXE5's souls: `Builds::forms_of`),
//! and the build's other choices as steppers (the game's builds data's: Light or Dark, of the game's own navi
//! alone).
//!
//! Choosing a navi is `Draft::choose_navi` (nettai-match's `Builds::set_navi`, with what is the game's own
//! navi's alone kept aside in the build's file); the version is `Draft::choose_version` (`Build::set_version`:
//! what the version sets is the new one's, the rest kept, docs/app.md §8); a choice's preset is
//! `Builds::take_preset`.

use crate::screens::builds::editor::{Draft, Tab};
use crate::shell::Shell;
use nettai_battle::Content;
use nettai_battle::content::{Element, PlayerFact};
use nettai_content_api::NaviHandle;
use nettai_match::build::TEXT_TABLE;
use nettai_ui::{EditorTab, NaviCard, NaviChoice, NaviView};
use slint::{ComponentHandle, ModelRc, VecModel};

#[derive(Default)]
pub struct Navi {
    /// The game's navis, as the grid has them.
    navis: Vec<NaviHandle>,
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

    /// The navi, and what the build's choices set.
    fn fields(&self, content: &Content) -> Vec<String> {
        content.defs.fact_name(PlayerFact::Navi).map(str::to_string).into_iter().collect()
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
            let cards: Vec<NaviCard> = self.navis.iter().map(|&n| NaviCard { name: names.navi(n).into(), face: kit.pictures.navi(n), chosen: n == chosen }).collect();
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
        // The version, any navi's; what it brings the game's own navi, while that is the build's.
        let version = kit.builds.version_of(&content, draft.side()).map(str::to_string);
        let name = version.as_deref().map(|v| draft.with_words(shell, |words| kit.version_name(words.names, v))).unwrap_or_default();
        view.set_version(name.into());
        let forms = match (&version, draft.build.own_navi(&content)) {
            (Some(v), true) => kit.builds.forms_of(&content, v).unwrap_or_default(),
            _ => Vec::new(),
        };
        let faces: Vec<slint::Image> = forms.iter().map(|&f| kit.pictures.face(f)).collect();
        view.set_forms(ModelRc::new(VecModel::from(faces)));
        view.set_face(face);
        view.set_hp(hp.into());
        view.set_element(kit.pictures.element(family(&content, content.navi(chosen).element)));
        // (The focus starts on the build's navi's card.)
        if shell.nav.focused().is_none_or(|k| !["navi", "version", "choice", "tab"].contains(&k.id.as_str()))
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
            // (Enter on the version or a choice takes the next, as Right does.)
            "version" | "choice" => self.step(shell, draft, id, index, 1),
            _ => {}
        }
    }

    fn step(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, by: i32) {
        let kit = draft.kit.clone();
        if id == "version" {
            // (The next version in the game's order; a build of none, the first.)
            let versions = &kit.builds.versions;
            let next = match kit.builds.version_of(draft.content(), draft.side()).and_then(|v| versions.iter().position(|x| x.id == v)) {
                Some(at) => (at as i32 + by).rem_euclid(versions.len() as i32) as usize,
                None => 0,
            };
            if draft.choose_version(&versions[next].id) {
                self.show(shell, draft);
            }
            return;
        }
        if id != "choice" {
            return;
        }
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
