//! The static content audit (`--audit-content`): every lookup the drawing
//! code and the audio make, through the same functions
//! ([`nettai_render::lookups`]), for everything the content defines, in every
//! language the content has strings in:
//!
//! - every chip's icon, picture, name glyphs, its name and code in the
//!   Program Advance animation, its window's class frame, element and code;
//!   its description in the dialogue font;
//! - every navi's face and emblem, its name and no-running message with
//!   its portrait; every form's face for each emotion; every Cross's name
//!   and colors and description; every custom-screen button's look, on a
//!   console of each of the game's versions;
//! - every asset the content names: each sprite with every animation and
//!   its frames, each sound's song, each banner's glyphs (a telop's banner
//!   its place, the judge's its numbers), each background and mugshot;
//! - the HUD's text lines, the custom screen, the chatbox and the warning
//!   marker of the game's graphics;
//! - the field (docs/design/rules-in-luau.md §7.4): the game's field's
//!   blocks for the panel types its game names, and every panel type and
//!   both highlights as the stage draws them (`stage::FieldArt`: the
//!   field's, or a tinted normal panel, which is said, not counted).
//!
//! It takes seconds and catches what a trace's frames would only catch for
//! the chips and the navis the trace shows: a lookup by the wrong key
//! fails here for every chip. What a battle asks for that the content
//! can't say beforehand (the animation and the palette an object picks)
//! is the trace audit's (`--audit`).

use nettai_assets::Bundle;
use nettai_battle::Content;
use nettai_battle::content::{AssetNumbers, BackgroundId, BannerId, BannerRole, ChipCode, ChipRole, MugshotId, SoundRole, SpriteId};
use nettai_battle::field::PanelType;
use nettai_content_api::{AssetKind, ChipHandle, FormHandle, NaviHandle};
use nettai_render::audit::{Lookup, Problems};
use nettai_render::graphics::Graphics;
use nettai_render::stage::{Art, FieldArt};
use nettai_render::strings::DisplayText;
use nettai_render::{hud, lookups};
use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

/// What the audit checked, and what it found.
#[derive(Clone, Debug, Default)]
pub struct ContentAudit {
    /// The definitions and assets it went through.
    pub chips: usize,
    pub navis: usize,
    pub forms: usize,
    /// The loaded packs' assets, by kind.
    pub assets: Vec<(AssetKind, usize)>,
    /// The languages it checked, the content's own first.
    pub languages: Vec<String>,
    /// The lookups made (in any language), each as `Lookup::describe` says
    /// it: what the trace cover needn't make again.
    pub made: BTreeSet<String>,
    /// What is missing: one line each, a language's other than the
    /// content's own marked with it.
    pub problems: Vec<String>,
    /// The strings a language's table lacks (`ja: chips.hidden.name`),
    /// which show in the content's own: not problems.
    pub untranslated: Vec<String>,
    /// What is drawn otherwise by design (a panel type the game's
    /// field draws, tinted): said, not problems.
    pub notes: Vec<String>,
}

/// A language to check: its name and its strings table (none: the
/// content's own).
pub type Language = (String, Option<Arc<nettai_content::locale::Strings>>);

/// Audit `c` against its game's graphics (`bundle`, whose lettering each
/// language swaps in; `numbers`, the game's numbers for the content's
/// assets) and sound (`bank`, if loaded), in each of `languages` (the
/// content's own first).
pub fn audit(c: &Content, bundle: Bundle, numbers: &AssetNumbers, bank: Option<&m4a::SoundBank>, languages: &[Language]) -> ContentAudit {
    let mut out = ContentAudit {
        chips: c.defs.chips.len(),
        navis: c.defs.navis.len(),
        forms: c.defs.forms.len(),
        assets: AssetKind::ALL.iter().map(|&k| (k, c.assets.names(k).len())).collect(),
        ..ContentAudit::default()
    };
    let mut seen = BTreeSet::new();
    let mut bundle = bundle;
    for (k, (lang, strings)) in languages.iter().enumerate() {
        // (A language the content has strings in and the graphics no
        // lettering for is shown with the graphics' own lettering, as a
        // frontend shows it, `Bundle::for_language`: its strings are the
        // vector font's to draw, not the graphics' fonts'.)
        let lettered = bundle.hud.has_language(lang);
        let own_language = bundle.hud.language().to_string();
        bundle = std::mem::take(&mut bundle).for_language(lang);
        let graphics = Graphics::of(&bundle).with_numbers(numbers);
        let text = DisplayText::new(strings.as_deref());
        let mut problems = Problems::default();
        check(c, &graphics, &text, lettered, if k == 0 { bank } else { None }, &mut problems);
        // (A string a language's table lacks shows in the content's own,
        // by design: not a lookup that fails.)
        out.untranslated.extend(text.take_missing().into_iter().map(|what| format!("{lang}: {what}")));
        out.made.extend(problems.lookups().map(|l| l.describe(c)));
        if k == 0 {
            out.notes = problems.said_lines();
        }
        if !lettered {
            out.notes.push(format!(
                "the graphics have no {lang} lettering: their pictures with words are drawn in their own ({own_language}), and {lang} names and descriptions with the frontend's font (not checked against the graphics')"
            ));
        }
        out.languages.push(lang.clone());
        // (A language's problems that the content's own has too are told
        // once.)
        for (what, _) in problems.iter() {
            if k == 0 {
                seen.insert(what.to_string());
                out.problems.push(what.to_string());
            } else if !seen.contains(what) {
                out.problems.push(format!("({lang}) {what}"));
            }
        }
    }
    out
}

/// Every lookup, in one language (`text`, the HUD's lettering in
/// `graphics`; `lettered`: the graphics have fonts of the language, so its
/// strings are looked up in them).
fn check(c: &Content, graphics: &Graphics, text: &DisplayText, lettered: bool, bank: Option<&m4a::SoundBank>, p: &mut Problems) {
    let own = graphics.bundle();
    let (hud, a) = (&own.hud, &own.custom);
    lookups::chatbox_graphics(hud, p);
    lookups::custom_graphics(a, p);
    lookups::warning(hud, p);
    // (The letters the enhanced mode draws its words and numbers by.)
    lookups::pack_letters(c, hud, a, text, p);
    for line in (hud::TEXT_TIME_UP..=hud::TEXT_TIME_UP + 10).chain([hud::TEXT_COUNTER_HIT]) {
        lookups::text_line(hud, line, p);
    }
    field(c, graphics, p);

    // The chips: the button chip's picture is its button's (EXE6's
    // BeastOut's, the Beast's: the custom screen's), not its own.
    let beast_out: BTreeSet<ChipHandle> = std::iter::once(c.defs.roles()).filter_map(|r| r.try_chip(ChipRole::ButtonChip)).collect();
    let font = &hud.dialogue_font;
    for (i, d) in c.defs.chips.iter().enumerate() {
        let chip = ChipHandle(i as u16);
        lookups::chip_icon(graphics, c, chip, p);
        if !beast_out.contains(&chip) {
            lookups::chip_art(graphics, c, chip, p);
        }
        if lettered {
            lookups::chip_name(hud, c, chip, text.chip_name(c, chip), p);
        }
        let code = d.record.codes.iter().map(|c| c.0).max().unwrap_or(ChipCode::ASTERISK.0);
        lookups::chip_window(a, c, chip, code, p);
        if let Some(said) = text.chip_description(c, chip).filter(|_| lettered) {
            lookups::dialogue(font, Lookup::ChipDescription(chip), said.text, p);
        }
    }

    // The custom screen's buttons' looks: the game's, and each of its
    // versions' own (EXE6's Beast Out button is its version's Beast's).
    for i in 0..c.defs.buttons.len() {
        let button = nettai_battle::content::ButtonHandle(i as u16);
        lookups::button(a, &a.versioned.base, c, button, p);
        for (version, own) in &a.versioned.versions {
            let mut q = Problems::default();
            lookups::button(a, own, c, button, &mut q);
            for (what, _) in q.iter() {
                p.note(format!("{what} (on a {version} console)"));
            }
        }
    }

    // The navis, their forms and Crosses.
    for i in 0..c.defs.navis.len() {
        let navi = NaviHandle(i as u16);
        let data = c.navi(navi);
        if !data.changes_form() {
            lookups::navi_face(graphics, c, navi, p);
        }
        lookups::emblem(a, c, navi, p);
        if lettered {
            lookups::navi_name(hud, navi, false, text.navi_name(c, navi), p);
        }
        if let Some(name) = text.navi_variant_name(c, navi).filter(|_| lettered) {
            lookups::navi_name(hud, navi, true, name, p);
        }
        if let Some(said) = text.run_message(c, navi).filter(|_| lettered) {
            lookups::dialogue(font, Lookup::RunMessage(navi), said.text, p);
        }
        // A game whose custom screen has the no-running message (its roles
        // fill the message's sound) has one for every navi a side can
        // start, said by a portrait: the screen opens no box for a navi
        // whose content states no words (`custom::Screen`), so a missing
        // one shows as L doing nothing.
        if c.defs.roles().try_sound(SoundRole::CustomRunMessage).is_some() && data.fresh.is_some() {
            let key = &c.defs.navi(navi).key;
            if data.run_message.counts.is_empty() {
                p.note(format!("navi {key} has no no-running message (`run_message` in the content's strings): L on the custom screen opens nothing"));
            }
            if data.run_message.portrait.is_none() {
                p.note(format!("navi {key}'s no-running message has no speaker (`run_message.portrait` in its definition)"));
            }
        }
        if let Some(id) = data.run_message.portrait {
            let who = || "(a portrait)".to_string();
            if let Some(sheet) = lookups::sprite(graphics, c, id, &who, p) {
                // (Its faces: still, idle and talking.)
                for anim in 0..3 {
                    lookups::animation(sheet, c, id, anim, &who, p);
                }
            }
        }
    }
    // The forms a window lists (EXE6's Crosses: the ones a navi lists
    // among a version's): each one's name and colors, and its description.
    // (Rules without the form list's window list none: EXE4's souls, which
    // its navi lists by version too.)
    let form_window = c.defs.rules().is_some_and(|r| r.views.form_list.is_some());
    for n in 0..c.defs.navis.len() {
        if !form_window {
            break;
        }
        let navi = NaviHandle(n as u16);
        for i in 0..c.defs.forms.len() {
            let form = FormHandle(i as u16);
            if nettai_render::custom::form_name_picture(c, a, navi, form).is_none() {
                continue;
            }
            lookups::form_name(a, c, navi, form, p);
            lookups::form_name_letters(a, c, navi, form, text, p);
            if let Some(said) = text.form_description(c, form).filter(|_| lettered) {
                lookups::dialogue(font, Lookup::FormDescription(form), said.text, p);
            }
        }
    }
    // (A form's faces by each of its game's emotions.)
    for i in 0..c.defs.forms.len() {
        for e in 0..c.rules().emotion.names.len() {
            for variant in [false, true] {
                lookups::form_face(graphics, c, FormHandle(i as u16), nettai_battle::content::Emotion(e as u8), variant, p);
            }
        }
    }

    // Every asset of the loaded packs, by its qualified name (a superset of
    // what the content names: a match file names a background of its own,
    // for one).
    let telops: HashSet<BannerId> = std::iter::once(c.defs.roles())
        .flat_map(|r| [BannerRole::Telop, BannerRole::TelopRemote].into_iter().filter_map(|role| r.banners.get(&role).copied()))
        .collect();
    let handles = |kind: AssetKind| 0..c.assets.names(kind).len() as u16;
    for h in handles(AssetKind::Sprite) {
        sprite(c, graphics, SpriteId(h), p);
    }
    if let Some(bank) = bank {
        for h in handles(AssetKind::Sound) {
            // (The no-music song stops the music: it has none.)
            crate::sound_lookups::sound(c, graphics.numbers(), bank, h, true, p);
        }
    }
    // (A banner the game's roles don't name is checked as its HUD lays it
    // out: a telop's layout, which has no glyphs of its own, as a telop. A
    // HUD that draws telops in its own look, EXE4's, draws its telop roles'
    // banner as a plain banner when it shows one.)
    let named: HashSet<BannerId> = c.defs.roles().banners.values().copied().collect();
    for id in handles(AssetKind::Banner).map(BannerId) {
        let own_telop = graphics.banner(id).is_some_and(|(hud, _)| hud.layout.telop.is_some());
        if !own_telop && (telops.contains(&id) || (!named.contains(&id) && lookups::is_telop(graphics, id))) {
            lookups::telop(graphics, c, id, p);
            continue;
        }
        // (The judge's banner draws its numbers by its role: `lookups::is_judge`.)
        lookups::banner(graphics, c, id, p);
        lookups::banner_letters(graphics, c, id, text, p);
    }
    for h in handles(AssetKind::Background) {
        lookups::background(graphics, c, BackgroundId(h), p);
    }
    for h in handles(AssetKind::Mugshot) {
        lookups::mugshot(graphics, c, MugshotId(h), p);
    }
}

/// The field (docs/design/rules-in-luau.md §7.4). The game's field draws
/// the panel types its game's `panels` section numbers, with their blocks.
/// Every panel type the game numbers and both highlights are drawn as the
/// stage draws them (`FieldArt`): from the game's field, or as a tinted
/// normal panel, which is said, not counted.
fn field(c: &Content, graphics: &Graphics, p: &mut Problems) {
    let types = || (0..c.rules().panels.types.len() as u8).map(PanelType);
    let field = &graphics.bundle().field;
    let blocks = |t: PanelType, p: &mut Problems| {
        for owner in 0..2 {
            for y in 1..=3 {
                lookups::panel_block(c, field, t, owner, y, p);
            }
        }
    };
    for t in types() {
        let name = c.rules().panels.name(t);
        if field.draws(name) {
            blocks(t, p);
        } else {
            p.note(format!("{}'s field doesn't draw panel type {} ({name}), which its game numbers", c.game(), t.0));
        }
    }
    let art = FieldArt::of(c, graphics);
    for t in types() {
        match art.panel(t) {
            Art::Field => blocks(t, p),
            Art::Tint => lookups::panel_tint(c, t.0, p),
        }
    }
    for h in 1..=2 {
        if art.highlight(h) == Art::Tint {
            lookups::panel_tint(c, lookups::HIGHLIGHT_TINT + h, p);
        }
    }
}

/// A sprite the content names: its sheet, every animation with its frames,
/// as many as the engine's timing for it has, and each frame's own palette
/// (its first part's, which an object's palette 0 picks).
fn sprite(c: &Content, graphics: &Graphics, id: SpriteId, p: &mut Problems) {
    let who = || "(named by the content)".to_string();
    let Some(sheet) = lookups::sprite(graphics, c, id, &who, p) else { return };
    let timed = c.animations.sprites.get(id.index()).map_or(0, |d| d.animations.len());
    if timed > sheet.animations.len() {
        p.note(format!(
            "{} has {} animations in the game's graphics, and {timed} in its timing (the content's)",
            lookups::sprite_name(c, id),
            sheet.animations.len()
        ));
    }
    for anim in 0..sheet.animations.len().min(0x100) {
        let Some(frames) = lookups::animation(sheet, c, id, anim as u8, &who, p) else { continue };
        for f in frames {
            let Some((parts, _, set)) = lookups::frame_parts(sheet, f) else { continue };
            let first = parts.first().map_or(0, |part| part.palette) as usize;
            let what = || format!("its animation {anim}'s own");
            lookups::palette(set, c, id, f, first, &what, p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_assets::{ChipIcon, Tiles};
    use nettai_battle::content::testing;

    /// The static audit's chips without an icon, with the game's icons
    /// named by `name` of each chip's key.
    fn chips_without_icons(name: impl Fn(&str) -> String) -> usize {
        let c = testing::content();
        let mut b = Bundle::default();
        for d in &c.defs.chips {
            b.hud.chip_icons.push(ChipIcon { key: name(&d.key), tiles: Tiles { pixels: vec![1; 4 * Tiles::TILE] } });
        }
        let found = audit(&c, b, &testing::asset_numbers(&c), None, &[("en".into(), None)]);
        found.problems.iter().filter(|p| p.ends_with("has no icon in the game's graphics")).count()
    }

    /// The field's blocks are audited for the panel types the game numbers,
    /// each of them.
    #[test]
    fn the_field_is_audited_for_the_panel_types_its_game_names() {
        let mut c = (*testing::content()).clone();
        let missing = |c: &Content| {
            let found = audit(c, Bundle::default(), &testing::asset_numbers(c), None, &[("en".into(), None)]);
            found.problems.iter().filter(|p| p.contains("doesn't draw panel type")).count()
        };
        let numbered = c.rules().panels.types.len();
        assert_eq!(missing(&c), numbered);
        // (A game of one type fewer.)
        let panels = &mut c.rules_mut().panels;
        panels.types.pop();
        panels.names.pop();
        assert_eq!(missing(&c), numbered - 1);
    }

    /// A navi a side can start that has no no-running message is listed
    /// (L on the custom screen would open nothing for it), in a game whose
    /// roles fill the message's sound.
    #[test]
    fn a_navi_without_its_no_running_message_is_listed() {
        let mut c = (*testing::content()).clone();
        let silent = |c: &Content| {
            let found = audit(c, Bundle::default(), &testing::asset_numbers(c), None, &[("en".into(), None)]);
            found.problems.into_iter().filter(|p| p.contains("has no no-running message")).collect::<Vec<_>>()
        };
        // (The test content's navis that say nothing are listed already.)
        let before = silent(&c);
        let navi = c.defs.navis.iter().position(|d| d.record.fresh.is_some() && !d.record.run_message.counts.is_empty()).expect("a navi with a message");
        let named = format!("navi {} has no no-running message", c.defs.navis[navi].key);
        assert!(!before.iter().any(|p| p.starts_with(&named)), "{before:?}");
        c.defs.navis[navi].record.run_message.counts.clear();
        let found = silent(&c);
        assert_eq!(found.len(), before.len() + 1, "{found:?}");
        assert!(found.iter().any(|p| p.starts_with(&named)), "{found:?}");
        // A game with no such message (no sound for it) has none to miss.
        c.defs.roles.sounds.remove(&SoundRole::CustomRunMessage);
        assert_eq!(silent(&c), Vec::<String>::new());
    }

    /// A lookup by the wrong key fails for every chip (R1's: a chip's icon
    /// by another key than the game's graphics name it by, here one written with its
    /// game), not only for the chips a trace shows.
    #[test]
    fn a_lookup_by_the_wrong_key_fails_for_every_chip() {
        assert_eq!(chips_without_icons(str::to_string), 0);
        assert_eq!(chips_without_icons(|key| format!("{}:{key}", testing::ROOT)), testing::content().defs.chips.len());
    }
}
