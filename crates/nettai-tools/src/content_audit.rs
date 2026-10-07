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
//!   console of each of the pack's versions;
//! - every asset of the loaded packs, by its qualified name (a superset of
//!   what the content names): each sprite with every animation and its
//!   frames, each sound's song, each banner's glyphs (a telop's banner its
//!   place, the judge's its numbers), each background and mugshot;
//! - the HUD's text lines, the custom screen, the chatbox and the warning
//!   marker, of each loaded game's pack (a console draws its own game's:
//!   another pack's for its game's chips' names, windows and descriptions,
//!   and its navis' emblems);
//! - the field (docs/design/rules-in-luau.md §7.4): each loaded game's
//!   pack's blocks for the panel types its game names, and in an arena of
//!   each, every panel type a loaded game names and both highlights, as
//!   the stage draws them (`stage::FieldArt`: the arena's field, another
//!   pack's, or a tinted normal panel, which is said, not counted).
//!
//! It takes seconds and catches what a trace's frames would only catch for
//! the chips and the navis the trace shows: a lookup by the wrong key
//! fails here for every chip. What a battle asks for that the content
//! can't say beforehand (the animation and the palette an object picks)
//! is the trace audit's (`--audit`).

use nettai_assets::Bundle;
use nettai_battle::Content;
use nettai_battle::content::{BackgroundId, BannerId, BannerRole, ChipCode, ChipRole, MugshotId, PackId, SoundRole, SpriteId};
use nettai_battle::field::PanelType;
use nettai_content_api::{AssetKind, ChipHandle, FormHandle, NaviHandle};
use nettai_render::audit::{Lookup, Problems};
use nettai_render::packs::Packs;
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
    /// What is drawn otherwise by design (a panel type no loaded pack's
    /// field draws, tinted): said, not problems.
    pub notes: Vec<String>,
}

/// A language to check: its name and its strings table (none: the
/// content's own).
pub type Language = (String, Option<Arc<nettai_content::locale::Strings>>);

/// Audit `c` against its packs' graphics (`bundles`, by `PackId`; `own` the
/// content's own, whose lettering each language swaps in) and sound
/// (`banks`, by `PackId`, if loaded), in each of `languages` (the content's
/// own first).
pub fn audit(c: &Content, mut bundles: Vec<Bundle>, own: PackId, banks: Option<&[Arc<m4a::SoundBank>]>, languages: &[Language]) -> ContentAudit {
    let mut out = ContentAudit {
        chips: c.defs.chips.len(),
        navis: c.defs.navis.len(),
        forms: c.defs.forms.len(),
        assets: AssetKind::ALL.iter().map(|&k| (k, c.assets.names(k).len())).collect(),
        ..ContentAudit::default()
    };
    let mut seen = BTreeSet::new();
    for (k, (lang, strings)) in languages.iter().enumerate() {
        let b = std::mem::take(&mut bundles[own.index()]);
        // (A language the content has strings in and its pack no lettering
        // for is a problem: a console in it can't be shown.)
        let have = b.hud.languages().join(", ");
        if k > 0 && !b.hud.languages().contains(&lang.as_str()) {
            out.problems.push(format!("({lang}) the pack has no lettering in {lang:?} (it has {have}): extract it again"));
            bundles[own.index()] = b;
            continue;
        }
        bundles[own.index()] = match b.in_language(lang) {
            Ok(b) => b,
            Err(e) => {
                out.problems.push(format!("({lang}) {e}"));
                continue;
            }
        };
        let packs = Packs::new(bundles.iter().collect(), own);
        let text = DisplayText::new(strings.as_deref());
        let mut problems = Problems::default();
        check(c, &packs, &text, if k == 0 { banks } else { None }, &mut problems);
        for (game, lines, made) in other_packs(c, &packs, lang, &text) {
            for what in lines {
                problems.note(format!("({game}'s pack) {what}"));
            }
            out.made.extend(made);
        }
        // (A string a language's table lacks shows in the content's own,
        // by design: not a lookup that fails.)
        out.untranslated.extend(text.take_missing().into_iter().map(|what| format!("{lang}: {what}")));
        out.made.extend(problems.lookups().map(|l| l.describe(c)));
        if k == 0 {
            out.notes = problems.said_lines();
        }
        out.languages.push(lang.clone());
        // (Another game's pack without this language's lettering draws its
        // console's names in its own: said.)
        for (i, game) in c.assets.packs.iter().enumerate() {
            let pack = PackId(i as u8);
            if pack != own && is_loaded(&packs, pack) && !packs.bundle(pack).hud.languages().contains(&lang.as_str()) {
                out.notes.push(format!(
                    "{game}'s pack has no {lang} lettering: an {game} console draws {lang} names and descriptions in its own (not checked)"
                ));
            }
        }
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

/// Whether pack `pack` is loaded (the own pack stands in for one that
/// isn't).
fn is_loaded(packs: &Packs, pack: PackId) -> bool {
    pack == packs.own_pack() || !std::ptr::eq(packs.bundle(pack), packs.own())
}

/// Each other loaded game's pack: what a console of its game draws from it
/// (the local side's game's HUD, custom screen and chatbox), as `check`
/// does the own pack's: its graphics, its game's chips' names (in its font,
/// in a language it has lettering in), windows and descriptions, and its
/// game's navis' emblems. Its problems, by game, and the lookups made.
fn other_packs(c: &Content, packs: &Packs, lang: &str, text: &DisplayText) -> Vec<(String, Vec<String>, Vec<String>)> {
    let mut out = Vec::new();
    for (i, game) in c.assets.packs.iter().enumerate() {
        let pack = PackId(i as u8);
        if pack == packs.own_pack() || !is_loaded(packs, pack) {
            continue;
        }
        let bundle = packs.bundle(pack);
        let (hud, a) = (&bundle.hud, &bundle.custom);
        let mut p = Problems::default();
        lookups::chatbox_graphics(hud, &mut p);
        lookups::custom_graphics(a, &mut p);
        lookups::warning(hud, &mut p);
        for line in (hud::TEXT_TIME_UP..=hud::TEXT_TIME_UP + 10).chain([hud::TEXT_COUNTER_HIT]) {
            lookups::text_line(hud, line, &mut p);
        }
        let lettered = hud.languages().contains(&lang);
        let of_game = |_key: &str| c.game() == game.as_str();
        for (k, d) in c.defs.chips.iter().enumerate().filter(|(_, d)| of_game(&d.key)) {
            let chip = ChipHandle(k as u16);
            if lettered {
                lookups::chip_name(hud, c, chip, text.chip_name(c, chip), &mut p);
                if let Some(said) = text.chip_description(c, chip) {
                    lookups::dialogue(&hud.dialogue_font, Lookup::ChipDescription(chip), said.text, &mut p);
                }
            }
            let code = d.record.codes.iter().map(|c| c.0).max().unwrap_or(ChipCode::ASTERISK.0);
            lookups::chip_window(a, c, chip, code, &mut p);
        }
        for k in (0..c.defs.navis.len()).filter(|&k| of_game(&c.defs.navi(NaviHandle(k as u16)).key)) {
            lookups::emblem(a, c, NaviHandle(k as u16), &mut p);
        }
        let made = p.lookups().map(|l| l.describe(c)).collect();
        out.push((game.clone(), p.iter().map(|(what, _)| what.to_string()).collect(), made));
    }
    out
}

/// Every lookup, in one language (`text`, the HUD's lettering in `packs`).
fn check(c: &Content, packs: &Packs, text: &DisplayText, banks: Option<&[Arc<m4a::SoundBank>]>, p: &mut Problems) {
    let own = packs.own();
    let (hud, a) = (&own.hud, &own.custom);
    lookups::chatbox_graphics(hud, p);
    lookups::custom_graphics(a, p);
    lookups::warning(hud, p);
    for line in (hud::TEXT_TIME_UP..=hud::TEXT_TIME_UP + 10).chain([hud::TEXT_COUNTER_HIT]) {
        lookups::text_line(hud, line, p);
    }
    field(c, packs, p);

    // The chips: the button chip's picture is its button's (EXE6's
    // BeastOut's, the Beast's: the custom screen's), not its own.
    let beast_out: BTreeSet<ChipHandle> = std::iter::once(c.defs.roles()).filter_map(|r| r.try_chip(ChipRole::ButtonChip)).collect();
    let font = &hud.dialogue_font;
    for (i, d) in c.defs.chips.iter().enumerate() {
        let chip = ChipHandle(i as u16);
        lookups::chip_icon(packs, c, chip, p);
        if !beast_out.contains(&chip) {
            lookups::chip_art(packs, c, chip, p);
        }
        let name = text.chip_name(c, chip);
        lookups::chip_name(hud, c, chip, name, p);
        let code = d.record.codes.iter().map(|c| c.0).max().unwrap_or(ChipCode::ASTERISK.0);
        lookups::chip_window(a, c, chip, code, p);
        if let Some(said) = text.chip_description(c, chip) {
            lookups::dialogue(font, Lookup::ChipDescription(chip), said.text, p);
        }
    }

    // The custom screen's buttons' looks: the pack's, and each of its
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
            lookups::navi_face(packs, c, navi, p);
        }
        lookups::emblem(a, c, navi, p);
        lookups::navi_name(hud, navi, false, text.navi_name(c, navi), p);
        if let Some(name) = text.navi_variant_name(c, navi) {
            lookups::navi_name(hud, navi, true, name, p);
        }
        if let Some(said) = text.run_message(c, navi) {
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
            if let Some(sheet) = lookups::sprite(packs, c, id, &who, p) {
                // (Its faces: still, idle and talking.)
                for anim in 0..3 {
                    lookups::animation(sheet, c, id, anim, &who, p);
                }
            }
        }
    }
    // The forms a window lists (EXE6's Crosses: the ones a navi lists
    // among a version's): each one's name and colors, and its description.
    for n in 0..c.defs.navis.len() {
        let navi = NaviHandle(n as u16);
        for i in 0..c.defs.forms.len() {
            let form = FormHandle(i as u16);
            if nettai_render::custom::form_name_picture(c, a, navi, form).is_none() {
                continue;
            }
            lookups::form_name(a, c, navi, form, p);
            if let Some(said) = text.form_description(c, form) {
                lookups::dialogue(font, Lookup::FormDescription(form), said.text, p);
            }
        }
    }
    // (A form's faces by each of its game's emotions.)
    for i in 0..c.defs.forms.len() {
        for e in 0..c.rules().emotion.names.len() {
            for variant in [false, true] {
                lookups::form_face(packs, c, FormHandle(i as u16), nettai_battle::content::Emotion(e as u8), variant, p);
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
        sprite(c, packs, SpriteId(h), p);
    }
    if let Some(banks) = banks {
        for h in handles(AssetKind::Sound) {
            // (The no-music song stops the music: it has none.)
            crate::sound_lookups::sound(c, banks, h, true, p);
        }
    }
    // (A banner no loaded game's roles name is checked as its pack lays it
    // out: a telop's layout, which has no glyphs of its own, as a telop.)
    let named: HashSet<BannerId> = c.defs.roles().banners.values().copied().collect();
    for id in handles(AssetKind::Banner).map(BannerId) {
        if telops.contains(&id) || (!named.contains(&id) && lookups::is_telop(packs, c, id)) {
            lookups::telop(packs, c, id, p);
            continue;
        }
        // (The judge's banner draws its numbers by its role: `lookups::is_judge`.)
        lookups::banner(packs, c, id, p);
    }
    for h in handles(AssetKind::Background) {
        lookups::background(packs, c, BackgroundId(h), p);
    }
    for h in handles(AssetKind::Mugshot) {
        lookups::mugshot(packs, c, MugshotId(h), p);
    }
}

/// The field (docs/design/rules-in-luau.md §7.4). Each loaded game's pack
/// draws the panel types its game's `panels` section names, with their
/// blocks (EXE6's field none of EXE5's metal, lava or sea). In an arena of
/// each loaded game, every panel type a loaded game names and both
/// highlights are drawn as the stage draws them (`FieldArt`): from the
/// arena's field, another pack's, or as a tinted normal panel, which is
/// said, not counted.
fn field(c: &Content, packs: &Packs, p: &mut Problems) {
    let names = |t: PanelType| c.rules().panels.types.get(t as usize).is_some_and(|r| r.named);
    let blocks = |pack: PackId, t: PanelType, p: &mut Problems| {
        for owner in 0..2 {
            for y in 1..=3 {
                lookups::panel_block(c, &packs.bundle(pack).field, pack, t as u8, owner, y, p);
            }
        }
    };
    let pack = packs.game_id(c);
    for t in PanelType::ALL.into_iter().filter(|&t| names(t)) {
        if packs.bundle(pack).field.draws(t as u8) {
            blocks(pack, t, p);
        } else {
            p.note(format!(
                "{}'s field doesn't draw panel type {} ({t:?}), which its game names (extract the pack again)",
                lookups::pack_name(c, pack),
                t as u8
            ));
        }
    }
    let art = FieldArt::of(c, packs);
    for t in PanelType::ALL.into_iter().filter(|&t| names(t)) {
        match art.panel(t) {
            Art::Field(from) => blocks(from, t, p),
            Art::Tint => lookups::panel_tint(c, art.arena, t as u8, p),
        }
    }
    for h in 1..=2 {
        if art.highlight(h) == Art::Tint {
            lookups::panel_tint(c, art.arena, lookups::HIGHLIGHT_TINT + h, p);
        }
    }
}

/// A sprite the content names: its sheet, every animation with its frames,
/// as many as the engine's timing for it has, and each frame's own palette
/// (its first part's, which an object's palette 0 picks).
fn sprite(c: &Content, packs: &Packs, id: SpriteId, p: &mut Problems) {
    let who = || "(named by the content)".to_string();
    let Some(sheet) = lookups::sprite(packs, c, id, &who, p) else { return };
    let timed = c.animations.sprites.get(&id).map_or(0, Vec::len);
    if timed > sheet.animations.len() {
        p.note(format!(
            "{} has {} animations in the pack's graphics, and {timed} in its timing (the engine's)",
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

    /// The static audit's chips without an icon, with the pack's icons
    /// named by `name` of each chip's key.
    fn chips_without_icons(name: impl Fn(&str) -> String) -> usize {
        let c = testing::content();
        let mut b = Bundle::default();
        for d in &c.defs.chips {
            b.hud.chip_icons.push(ChipIcon { key: name(&d.key), tiles: Tiles { pixels: vec![1; 4 * Tiles::TILE] } });
        }
        let own = c.assets.pack(testing::ROOT).expect("the test pack");
        let found = audit(&c, vec![b], own, None, &[("en".into(), None)]);
        found.problems.iter().filter(|p| p.ends_with("has no icon in the pack")).count()
    }

    /// The field's blocks are audited for the panel types the own pack's
    /// game names, not for every type the engine has (EXE5's metal, lava
    /// and sea, which EXE6's field has none of; EXE4's pitfall and hole,
    /// which the test content names none of).
    #[test]
    fn the_field_is_audited_for_the_panel_types_its_game_names() {
        let mut c = (*testing::content()).clone();
        let own = c.assets.pack(testing::ROOT).expect("the test pack");
        let missing = |c: &Content| {
            let found = audit(c, vec![Bundle::default()], own, None, &[("en".into(), None)]);
            found.problems.iter().filter(|p| p.contains("doesn't draw panel type")).count()
        };
        let named = PanelType::ALL.len() - 2;
        assert_eq!(missing(&c), named);
        for t in [PanelType::Metal, PanelType::Lava, PanelType::Sea] {
            c.rules_mut().panels.types[t as usize].named = false;
        }
        assert_eq!(missing(&c), named - 3);
    }

    /// A navi a side can start that has no no-running message is listed
    /// (L on the custom screen would open nothing for it), in a game whose
    /// roles fill the message's sound.
    #[test]
    fn a_navi_without_its_no_running_message_is_listed() {
        let mut c = (*testing::content()).clone();
        let own = c.assets.pack(testing::ROOT).expect("the test pack");
        let silent = |c: &Content| {
            let found = audit(c, vec![Bundle::default()], own, None, &[("en".into(), None)]);
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

    /// Another loaded game's pack is audited as its console draws it (its
    /// HUD's, custom screen's and chatbox's graphics), named by its game;
    /// one that isn't loaded is not.
    #[test]
    fn another_games_pack_is_audited_as_its_console_draws_it() {
        let mut c = (*testing::content()).clone();
        let own = c.assets.pack(testing::ROOT).expect("the test pack");
        c.assets.packs.push("other".into());
        let others = |bundles: Vec<Bundle>| {
            let found = audit(&c, bundles, own, None, &[("en".into(), None)]);
            found.problems.into_iter().filter(|p| p.starts_with("(other's pack)")).collect::<Vec<_>>()
        };
        let found = others(vec![Bundle::default(), Bundle::default()]);
        assert!(found.iter().any(|p| p.contains("no chatbox graphics")), "{found:?}");
        assert!(found.iter().any(|p| p.contains("no custom screen graphics")), "{found:?}");
        assert!(others(vec![Bundle::default()]).is_empty());
    }

    /// A lookup by the wrong key fails for every chip (R1's: a chip's icon
    /// by another key than the pack names it by, here one written with its
    /// game), not only for the chips a trace shows.
    #[test]
    fn a_lookup_by_the_wrong_key_fails_for_every_chip() {
        assert_eq!(chips_without_icons(str::to_string), 0);
        assert_eq!(chips_without_icons(|key| format!("{}:{key}", testing::ROOT)), testing::content().defs.chips.len());
    }
}
