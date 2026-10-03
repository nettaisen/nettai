//! The static content audit (`--audit-content`): every lookup the drawing
//! code and the audio make, through the same functions
//! ([`nettai_render::lookups`]), for everything the content defines, in every
//! language the content has strings in:
//!
//! - every chip's icon, picture, name glyphs, its name and code in the
//!   Program Advance animation, its window's class frame, element and code;
//!   its description in the dialogue font;
//! - every navi's face and emblem (on either game's console), its name and
//!   no-running message with its portrait; every form's face for each
//!   emotion; every Cross's name and colors and description;
//! - every asset of the loaded packs, by its qualified name (a superset of
//!   what the content names): each sprite with every animation and its
//!   frames, each sound's song, each banner's glyphs (a telop's banner its
//!   place, the judge's its numbers), each background and mugshot;
//! - the HUD's text lines, the custom screen, the chatbox and the warning
//!   marker, of each loaded game's pack (a console draws its own game's:
//!   another pack's for its game's chips' names, windows and descriptions,
//!   and its navis' emblems in each of its versions);
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
use nettai_battle::content::{BackgroundId, BannerId, BannerRole, ChipCode, ChipRole, MugshotId, PackId, RootId, SpriteId};
use nettai_battle::custom::GameVersion;
use nettai_battle::field::PanelType;
use nettai_battle::kinds::player::Emotion;
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
    /// The strings a language's table lacks (`ja: chips.bn6:hidden.name`),
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
        bundles[own.index()] = match b.in_language(lang) {
            Ok(b) => b,
            Err(e) => {
                out.problems.push(format!("({lang}) {e}"));
                continue;
            }
        };
        // (The marks one game's font lacks, another's lends it, as the
        // frontend loads them.)
        nettai_assets::lend_marks(&mut bundles);
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
                    "{game}'s pack has no {lang} lettering: a {game} console draws {lang} names and descriptions in its own (not checked)"
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
/// game's navis' emblems in each of its versions. Its problems, by game,
/// and the lookups made.
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
        let root = c.defs.roots.iter().position(|g| g == game).map(|r| RootId(r as u8));
        let of_game = |key: &str| root.is_some() && c.defs.root_of(key) == root;
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
        let versions: Vec<&str> =
            std::iter::once(a.versioned.base_version.as_str()).chain(a.versioned.versions.iter().map(|(v, _)| v.as_str())).collect();
        for k in (0..c.defs.navis.len()).filter(|&k| of_game(&c.defs.navi(NaviHandle(k as u16)).key)) {
            let navi = NaviHandle(k as u16);
            for &version in &versions {
                // (Each version's own run: the lookup is by the engine's
                // version, which tells BN5's apart by none.)
                let mut q = Problems::default();
                lookups::emblem(a, c, navi, GameVersion::Falzar, Some(version), &mut q);
                for (what, _) in q.iter() {
                    p.note(format!("{what} (on a {version} console)"));
                }
            }
        }
        let made = p.lookups().map(|l| l.describe(c)).collect();
        out.push((game.clone(), p.iter().map(|(what, _)| what.to_string()).collect(), made));
    }
    out
}

/// The emotions a form's faces are by.
const EMOTIONS: [Emotion; 5] = [Emotion::Normal, Emotion::Tired, Emotion::FullSynchro, Emotion::Angry, Emotion::WornOut];

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

    // The chips: the Beast Out chip's picture is the Beast's (the custom
    // screen's), not its own.
    let beast_out: BTreeSet<ChipHandle> = c.defs.roles.iter().filter_map(|r| r.try_chip(ChipRole::BeastOut)).collect();
    let font = &hud.dialogue_font;
    for (i, d) in c.defs.chips.iter().enumerate() {
        let chip = ChipHandle(i as u16);
        lookups::chip_icon(packs, c, chip, p);
        if !beast_out.contains(&chip) {
            lookups::chip_art(packs, c, chip, p);
        }
        let name = text.chip_name(c, chip);
        lookups::chip_name(hud, c, chip, name, p);
        lookups::advance_code(c, chip, p);
        let code = d.record.codes.iter().map(|c| c.0).max().unwrap_or(ChipCode::ASTERISK.0);
        lookups::chip_window(a, c, chip, code, p);
        if let Some(said) = text.chip_description(c, chip) {
            lookups::dialogue(font, Lookup::ChipDescription(chip), said.text, p);
        }
    }

    // The navis, their forms and Crosses.
    for i in 0..c.defs.navis.len() {
        let navi = NaviHandle(i as u16);
        let data = c.navi(navi);
        if !data.changes_form() {
            lookups::navi_face(packs, c, navi, p);
        }
        for version in [GameVersion::Falzar, GameVersion::Gregar] {
            lookups::emblem(a, c, navi, version, None, p);
        }
        lookups::navi_name(hud, navi, text.navi_name(c, navi), p);
        if let Some(said) = text.run_message(c, navi) {
            lookups::dialogue(font, Lookup::RunMessage(navi), said.text, p);
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
        if let Some(forms) = &data.forms {
            for version in [GameVersion::Falzar, GameVersion::Gregar] {
                for &form in &forms.of(version).crosses {
                    lookups::cross_name(a, c, navi, form, p);
                    if let Some(said) = text.form_description(c, form) {
                        lookups::dialogue(font, Lookup::CrossDescription(form), said.text, p);
                    }
                }
            }
        }
    }
    for i in 0..c.defs.forms.len() {
        for emotion in EMOTIONS {
            lookups::form_face(packs, c, FormHandle(i as u16), emotion, p);
        }
    }

    // Every asset of the loaded packs, by its qualified name (a superset of
    // what the content names: a match file names a background of its own,
    // for one).
    let telops: HashSet<BannerId> = c
        .defs
        .roles
        .iter()
        .flat_map(|r| [BannerRole::Telop, BannerRole::TelopRemote].into_iter().filter_map(|role| r.banners.get(&role).copied()))
        .collect();
    let judges: HashSet<BannerId> = c.defs.roles.iter().filter_map(|r| r.banners.get(&BannerRole::Judge).copied()).collect();
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
    let named: HashSet<BannerId> = c.defs.roles.iter().flat_map(|r| r.banners.values().copied()).collect();
    for id in handles(AssetKind::Banner).map(BannerId) {
        if telops.contains(&id) || (!named.contains(&id) && lookups::is_telop(packs, c, id)) {
            lookups::telop(packs, c, id, p);
            continue;
        }
        lookups::banner(packs, c, id, p);
        if judges.contains(&id) && !lookups::is_judge(packs, c, id) {
            p.note(format!("the judge's {} is no judge's in the pack: its numbers aren't drawn", Lookup::Banner(id).describe(c)));
        }
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
/// blocks (BN6's field none of BN5's metal, lava or sea). In an arena of
/// each loaded game, every panel type a loaded game names and both
/// highlights are drawn as the stage draws them (`FieldArt`): from the
/// arena's field, another pack's, or as a tinted normal panel, which is
/// said, not counted.
fn field(c: &Content, packs: &Packs, p: &mut Problems) {
    // (The shared folder, content/common, is no game: it has no field.)
    let roots: Vec<RootId> = (0..c.rules.len())
        .map(|i| RootId(i as u8))
        .filter(|r| c.defs.roots.get(r.index()).is_none_or(|name| name != nettai_content_api::keys::SHARED))
        .collect();
    let names = |root: RootId, t: PanelType| c.rules_of(root).panels.types.get(t as usize).is_some_and(|r| r.named);
    let blocks = |pack: PackId, t: PanelType, p: &mut Problems| {
        for owner in 0..2 {
            for y in 1..=3 {
                lookups::panel_block(c, &packs.bundle(pack).field, pack, t as u8, owner, y, p);
            }
        }
    };
    for &root in &roots {
        let pack = packs.id_of_root(c, root);
        for t in PanelType::ALL.into_iter().filter(|&t| names(root, t)) {
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
    }
    for &arena in &roots {
        let art = FieldArt::of(c, packs, arena);
        for t in PanelType::ALL.into_iter().filter(|&t| roots.iter().any(|&r| names(r, t))) {
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
    /// game names, not for every type the engine has (BN5's metal, lava
    /// and sea, which BN6's field has none of).
    #[test]
    fn the_field_is_audited_for_the_panel_types_its_game_names() {
        let mut c = (*testing::content()).clone();
        let own = c.assets.pack(testing::ROOT).expect("the test pack");
        let missing = |c: &Content| {
            let found = audit(c, vec![Bundle::default()], own, None, &[("en".into(), None)]);
            found.problems.iter().filter(|p| p.contains("doesn't draw panel type")).count()
        };
        assert_eq!(missing(&c), PanelType::ALL.len());
        for t in [PanelType::Metal, PanelType::Lava, PanelType::Sea] {
            let test = c.defs.root_id(testing::ROOT).expect("the test game");
            c.rules[test.index()].panels.types[t as usize].named = false;
        }
        assert_eq!(missing(&c), PanelType::ALL.len() - 3);
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
    /// by its qualified key, where the pack names it by the key its root
    /// writes), not only for the chips a trace shows.
    #[test]
    fn a_lookup_by_the_wrong_key_fails_for_every_chip() {
        assert_eq!(chips_without_icons(|key| nettai_content_api::keys::local(key).to_string()), 0);
        assert_eq!(chips_without_icons(str::to_string), testing::content().defs.chips.len());
    }
}
