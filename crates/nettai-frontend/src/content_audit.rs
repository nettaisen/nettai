//! The static content audit (`--audit-content`): every lookup the drawing
//! code and the audio make, through the same functions
//! ([`crate::lookups`]), for everything the content defines, in every
//! language the content has strings in:
//!
//! - every chip's icon, picture, name glyphs, its name and code in the
//!   Program Advance animation, its window's class frame, element and code;
//!   its description in the dialogue font;
//! - every navi's face and emblem (on either game's console), its name and
//!   no-running message with its portrait; every form's face for each
//!   emotion; every Cross's name and colours and description;
//! - every asset of the loaded packs, by its qualified name (a superset of
//!   what the content names): each sprite with every animation and its
//!   frames, each sound's song, each banner's glyphs (a telop's banner its
//!   place, the judge's its numbers), each background and mugshot;
//! - the HUD's text lines, the field's panel blocks, the custom screen,
//!   the chatbox and the warning marker.
//!
//! It takes seconds and catches what a trace's frames would only catch for
//! the chips and the navis the trace shows: a lookup by the wrong key
//! fails here for every chip. What a battle asks for that the content
//! can't say beforehand (the animation and the palette an object picks)
//! is the trace audit's (`--audit`).

use crate::audit::{Lookup, Problems};
use crate::lookups;
use crate::packs::Packs;
use crate::strings::DisplayText;
use nettai_assets::Bundle;
use nettai_battle::Content;
use nettai_battle::content::{BackgroundId, BannerId, BannerRole, ChipCode, ChipRole, MugshotId, PackId, SpriteId};
use nettai_battle::custom::GameVersion;
use nettai_battle::field::PanelType;
use nettai_battle::kinds::player::Emotion;
use nettai_content_api::{AssetKind, ChipHandle, FormHandle, NaviHandle};
use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

/// What the audit checked, and what it found.
#[derive(Clone, Debug, Default)]
pub struct ContentAudit {
    /// The definitions and assets it went through.
    pub chips: usize,
    pub navis: usize,
    pub forms: usize,
    /// The assets the content's modules name, by kind.
    pub assets: Vec<(AssetKind, usize)>,
    /// The languages it checked, the content's own first.
    pub languages: Vec<String>,
    /// Lookups made (over every language).
    pub lookups: usize,
    /// What is missing: one line each, a language's other than the
    /// content's own marked with it.
    pub problems: Vec<String>,
    /// The strings a language's table lacks (`ja: chips.bn6:hidden.name`),
    /// which show in the content's own: not problems.
    pub untranslated: Vec<String>,
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
        let packs = Packs::new(bundles.iter().collect(), own);
        let text = DisplayText::new(strings.as_deref());
        let mut problems = Problems::default();
        check(c, &packs, &text, if k == 0 { banks } else { None }, &mut problems);
        // (A string a language's table lacks shows in the content's own,
        // by design: not a lookup that fails.)
        out.untranslated.extend(text.take_missing().into_iter().map(|what| format!("{lang}: {what}")));
        out.lookups += problems.lookups().count();
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

/// The emotions a form's faces are by.
const EMOTIONS: [Emotion; 5] = [Emotion::Normal, Emotion::Tired, Emotion::FullSynchro, Emotion::Angry, Emotion::WornOut];

/// Every lookup, in one language (`text`, the HUD's lettering in `packs`).
fn check(c: &Content, packs: &Packs, text: &DisplayText, banks: Option<&[Arc<m4a::SoundBank>]>, p: &mut Problems) {
    let own = packs.own();
    let (hud, a) = (&own.hud, &own.custom);
    lookups::chatbox_graphics(hud, p);
    lookups::custom_graphics(a, p);
    lookups::warning(hud, p);
    for line in (crate::hud::TEXT_TIME_UP..=crate::hud::TEXT_TIME_UP + 10).chain([crate::hud::TEXT_COUNTER_HIT]) {
        lookups::text_line(hud, line, p);
    }
    for kind in PanelType::ALL {
        for owner in 0..2 {
            for y in 1..=3 {
                lookups::panel_block(&own.field, kind as usize, owner, y, p);
            }
        }
    }

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
            lookups::emblem(a, c, navi, version, p);
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
            lookups::sound(c, banks, h, true, p);
        }
    }
    for id in handles(AssetKind::Banner).map(BannerId) {
        if telops.contains(&id) {
            lookups::telop(packs, c, id, p);
            continue;
        }
        lookups::banner(packs, c, id, p);
        if judges.contains(&id) && !lookups::is_judge(packs, c, id) {
            p.note(format!("the judge's banner {} is no judge's in the pack: its numbers aren't drawn", Lookup::Banner(id).describe(c)));
        }
    }
    for h in handles(AssetKind::Background) {
        lookups::background(packs, c, BackgroundId(h), p);
    }
    for h in handles(AssetKind::Mugshot) {
        lookups::mugshot(packs, c, MugshotId(h), p);
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
