//! The lookups the drawing code makes of the packs and the content, each a
//! function that notes its [`Lookup`] and, the first time a run makes it,
//! what is missing (`crate::audit`; the audio's, a cue's song, is
//! nettai-frontend's `sound_lookups`). A frame draws through these, and the
//! static audit (nettai-frontend's `content_audit`) makes the same
//! lookups for everything the content defines, so a lookup by the wrong
//! key (a chip's icon by its qualified key, a banner by its handle) fails
//! both the same way.

use crate::audit::{Graphics, Lookup, Problems, emotion_number};
use crate::packs::Packs;
use nettai_assets::{BannerLayout, ChipArt, CustomScreen, DialogueFont, Hud, Palette, SpriteFrame, SpritePart, SpriteSheet, Tiles};
use nettai_battle::content::{BackgroundId, BannerId, ChipClass, ChipFlags, Content, MugshotId, PackId, SpriteId};
use nettai_battle::custom::GameVersion;
use nettai_battle::field::PanelType;
use nettai_battle::kinds::player::Emotion;
use nettai_content_api::{AssetKind, ChipHandle, FormHandle, NaviHandle};

/// A sprite's name for a problem's text (`sprite "bomb"`).
pub fn sprite_name(c: &Content, id: SpriteId) -> String {
    match crate::packs::name(c, AssetKind::Sprite, id.0) {
        Some(name) => format!("sprite {name:?}"),
        None => format!("sprite handle {}", id.0),
    }
}

/// Sprite `id`'s sheet, from its pack; `who` says who asks (`of kind
/// "bomb"`, `(a portrait)`).
pub fn sprite<'a>(packs: &Packs<'a>, c: &Content, id: SpriteId, who: &dyn Fn() -> String, problems: &mut Problems) -> Option<&'a SpriteSheet> {
    let sheet = packs.sprite(c, id);
    if problems.lookup(Lookup::Sprite(id)) && sheet.is_none() {
        problems.note(format!("{} {} is not in the pack's graphics", sprite_name(c, id), who()));
    }
    sheet
}

/// Animation `anim` of sprite `id`'s sheet, if it has frames.
pub fn animation<'a>(
    sheet: &'a SpriteSheet,
    c: &Content,
    id: SpriteId,
    anim: u8,
    who: &dyn Fn() -> String,
    problems: &mut Problems,
) -> Option<&'a [SpriteFrame]> {
    let frames = sheet.animations.get(anim as usize).map(Vec::as_slice);
    if problems.lookup(Lookup::Animation(id, anim)) {
        match frames {
            None => problems.note(format!(
                "{} has no animation {anim} ({}; it has {})",
                sprite_name(c, id),
                who(),
                sheet.animations.len()
            )),
            Some([]) => problems.note(format!("{} animation {anim} has no frames ({})", sprite_name(c, id), who())),
            Some(frames) => {
                // (Every frame's parts, tiles and palettes are the sheet's:
                // drawing indexes them.)
                for (k, f) in frames.iter().enumerate() {
                    if frame_parts(sheet, f).is_none() {
                        problems.note(format!(
                            "{} animation {anim} frame {k} names parts {}, tiles {} or palettes {} the sheet doesn't have",
                            sprite_name(c, id),
                            f.parts,
                            f.tileset,
                            f.palette_set
                        ));
                    }
                }
            }
        }
    }
    frames.filter(|f| !f.is_empty())
}

/// A frame's parts, tiles and palette set in its sheet.
pub fn frame_parts<'a>(sheet: &'a SpriteSheet, f: &SpriteFrame) -> Option<(&'a [SpritePart], &'a Tiles, &'a [Palette])> {
    Some((
        sheet.part_lists.get(f.parts as usize)?,
        sheet.tilesets.get(f.tileset as usize)?,
        sheet.palette_sets.get(f.palette_set as usize)?,
    ))
}

/// Palette `index` of a frame's palette set (`what` says who asks it and
/// on which animation).
pub fn palette(
    set: &[Palette],
    c: &Content,
    id: SpriteId,
    f: &SpriteFrame,
    index: usize,
    what: &dyn Fn() -> String,
    problems: &mut Problems,
) -> Option<Palette> {
    let p = set.get(index).copied();
    if problems.lookup(Lookup::Palette(id, f.palette_set as u8, index.min(0xFF) as u8)) && p.is_none() {
        problems.note(format!("{} has no palette {index} ({}; the set has {})", sprite_name(c, id), what(), set.len()));
    }
    p
}

/// A background, from its pack.
pub fn background<'a>(packs: &Packs<'a>, c: &Content, id: BackgroundId, problems: &mut Problems) -> Option<&'a nettai_assets::Background> {
    let found = packs.background(c, id).and_then(|(pack, n)| pack.background(n));
    if problems.lookup(Lookup::Background(id)) && found.is_none() {
        let name = crate::packs::name(c, AssetKind::Background, id.0).map_or(format!("handle {}", id.0), |n| format!("{n:?}"));
        problems.note(format!("background {name} is not in the pack's graphics"));
    }
    found
}

/// A pack's name for a problem's text (`bn6's pack`).
pub fn pack_name(c: &Content, pack: PackId) -> String {
    c.assets.packs.get(pack.index()).map_or(format!("pack {}", pack.index()), |g| format!("{g}'s pack"))
}

/// Pack `pack`'s field's panel block for a panel type (`kind`, the
/// engine's number), owner (0 the viewer's) and row (`y`, 1..=3): a field
/// that draws the type (`stage::FieldArt`).
pub fn panel_block<'a>(
    c: &Content,
    field: &'a nettai_assets::Field,
    pack: PackId,
    kind: u8,
    owner: usize,
    y: u8,
    problems: &mut Problems,
) -> Option<&'a [nettai_assets::MapEntry]> {
    let block = field.panel(kind, owner, y).map(|b| &b[..]);
    if problems.lookup(Lookup::Panel(pack, kind, owner as u8, y)) && block.is_none() {
        problems.note(format!("{}'s field has no block for panel type {kind}, owner {owner}, row {y}", pack_name(c, pack)));
    }
    block
}

/// The number [`Lookup::PanelTint`] gives highlight `h` (1 or 2):
/// `HIGHLIGHT_TINT + h`.
pub const HIGHLIGHT_TINT: u8 = 16;

/// A panel type (`kind`, the engine's number; a highlight,
/// `HIGHLIGHT_TINT + h`) that no loaded pack's field draws, in an arena of
/// pack `arena`'s: drawn as a tinted normal panel, by design
/// (docs/design/rules-in-luau.md §7.4). Said, not counted.
pub fn panel_tint(c: &Content, arena: PackId, kind: u8, problems: &mut Problems) {
    if problems.lookup(Lookup::PanelTint(arena, kind)) {
        let what = match kind.checked_sub(HIGHLIGHT_TINT) {
            Some(h) => format!("highlight {h}"),
            None => match PanelType::ALL.get(kind as usize) {
                Some(t) => format!("panel type {kind} ({t:?})"),
                None => format!("panel type {kind}"),
            },
        };
        problems.say(format!("in an arena of {}, {what} is in no loaded pack's field: drawn as a tinted normal panel", pack_name(c, arena)));
    }
}

/// A chip's key.
fn key(c: &Content, chip: ChipHandle) -> &str {
    &c.defs.chip(chip).key
}

/// A chip's icon: its game's pack's, under its key there; with that HUD's
/// icon palette.
pub fn chip_icon<'a>(packs: &Packs<'a>, c: &Content, chip: ChipHandle, problems: &mut Problems) -> Option<(&'a Tiles, &'a Palette)> {
    let key = key(c, chip);
    let icon = packs.chip_icon(c, key);
    if problems.lookup(Lookup::ChipIcon(chip)) && icon.is_none() {
        problems.note(format!("chip {key:?} has no icon in the pack"));
    }
    Some((icon?, &packs.game(c).hud.icon_palette))
}

/// A chip's picture in the chip window: its game's pack's, under its key
/// there.
pub fn chip_art<'a>(packs: &Packs<'a>, c: &Content, chip: ChipHandle, problems: &mut Problems) -> Option<&'a ChipArt> {
    let key = key(c, chip);
    let art = packs.chip_art(c, key);
    if problems.lookup(Lookup::ChipArt(chip)) && art.is_none() {
        problems.note(format!("chip {key:?} has no picture in the pack"));
    }
    art
}

/// A chip's name (`name`, its display text) in the 8x16 font's glyphs, all
/// of them (the HUD shows eight at most).
pub fn chip_name(hud: &Hud, c: &Content, chip: ChipHandle, name: &str, problems: &mut Problems) -> Vec<u16> {
    let (glyphs, missing) = crate::fonts::cell_glyphs(hud, name);
    if problems.lookup(Lookup::ChipName(chip)) && !missing.is_empty() {
        problems.note(format!("chip {:?} is named {name:?}, but the pack's font has no glyph for {missing:?}", key(c, chip)));
    }
    glyphs
}

/// What the chip window shows of a chip besides its name and picture: its
/// frame's colors (`frame_palettes`, by class: a dark chip's dark), its
/// element's icon and colors, its code's glyph. Returns the frame's
/// number.
pub fn chip_window(a: &CustomScreen, c: &Content, chip: ChipHandle, code: u8, problems: &mut Problems) -> usize {
    let data = c.chip(chip);
    let class = match data.class {
        ChipClass::Standard => Some(0),
        ChipClass::Mega => Some(1),
        ChipClass::Giga => Some(2),
        _ => None,
    };
    let frame = match class {
        Some(_) if data.flags.has(ChipFlags::DARK) => 3,
        Some(n) => n,
        None => 0,
    };
    if problems.lookup(Lookup::ChipWindow(chip)) {
        let key = key(c, chip);
        if a.frame_palettes.get(frame).is_none() {
            problems.note(format!("chip {key:?}: the custom screen has no window colors for its class ({frame})"));
        }
        // (A family past the elements with colors shows none: the
        // original's.)
        let family = data.family as usize;
        if family < a.element_colors.len() && a.elements.len() < 4 * (family + 1) {
            problems.note(format!("chip {key:?}: the custom screen has no icon for its element ({family})"));
        }
        if a.codes.len() < 2 * (code.min(crate::custom::NO_CODE) as usize + 1) {
            problems.note(format!("chip {key:?}: the custom screen has no glyph for its code ({code})"));
        }
    }
    frame
}

/// Whether the Program Advance animation shows a chip's code after its
/// name (`sub_802B80C`: not for the original's chips from 0x160 on, its
/// navi chips' and the like), by the chip's number in BN6's compat; a
/// chip of another root shows none.
pub fn advance_code(c: &Content, chip: ChipHandle, problems: &mut Problems) -> bool {
    let key = key(c, chip);
    let compat = bn6_compat::Compat::bn6_for(c);
    let Some(local) = compat.compat_key(c, key) else { return false };
    let number = compat.chips.get(local).map(|e| e.id);
    if problems.lookup(Lookup::AdvanceName(chip)) && number.is_none() {
        problems.note(format!("chip {key:?} has no number in BN6's compat: the Program Advance animation can't tell whether its code shows"));
    }
    number.is_some_and(|n| n < crate::custom::ADVANCE_NO_CODE_FROM)
}

/// A name in the Program Advance animation (`name`, a chip's display
/// text) in the 8x16 font's glyphs.
pub fn advance_name(hud: &Hud, chip: ChipHandle, name: &str, problems: &mut Problems) -> Vec<u16> {
    let (glyphs, missing) = crate::fonts::cell_glyphs(hud, name);
    // (Its own lookup: the chip's name, as the HUD's.)
    if problems.lookup(Lookup::ChipName(chip)) && !missing.is_empty() {
        problems.note(format!("the Program Advance animation's {name:?}: the pack's font has no glyph for {missing:?}"));
    }
    glyphs
}

/// A mugshot, from its pack's HUD: the picture and its palettes.
pub fn mugshot<'a>(packs: &Packs<'a>, c: &Content, id: MugshotId, problems: &mut Problems) -> Option<(&'a Tiles, &'a [Palette])> {
    let face = crate::packs::mugshot(c, id).and_then(|m| {
        let (hud, n) = packs.mugshot(m);
        hud.mugshot(n)
    });
    if problems.lookup(Lookup::Mugshot(id)) && face.is_none() {
        let name = crate::packs::name(c, AssetKind::Mugshot, id.0).map_or(format!("handle {}", id.0), |n| format!("{n:?}"));
        problems.note(format!("mugshot {name} is not in the pack's graphics"));
    }
    face
}

/// A link navi's own face (its definition's mugshot): the picture and its
/// palettes (the second Full Synchro's).
pub fn navi_face<'a>(packs: &Packs<'a>, c: &Content, navi: NaviHandle, problems: &mut Problems) -> Option<(&'a Tiles, &'a [Palette], u8)> {
    let picture = c.navi(navi).mugshot.and_then(|m| crate::packs::mugshot(c, m));
    let face = picture.and_then(|m| {
        let (hud, n) = packs.mugshot(m);
        hud.mugshot(n).map(|(t, p)| (t, p, m.id))
    });
    if problems.lookup(Lookup::NaviFace(navi)) && face.is_none() {
        problems.note(format!("navi {:?} has no mugshot in the pack", c.defs.navi(navi).key));
    }
    face
}

/// The face a form shows for an emotion (its definition's mugshot for it, of
/// its second set when `variant`): the mugshot's pack and number there,
/// and the picture with its palettes.
#[allow(clippy::type_complexity)]
pub fn form_face<'a>(
    packs: &Packs<'a>,
    c: &Content,
    form: FormHandle,
    emotion: Emotion,
    variant: bool,
    problems: &mut Problems,
) -> (Option<nettai_battle::content::InPack<u8>>, Option<(&'a Tiles, &'a [Palette])>) {
    let picture = c.form(form).mugshot.and_then(|faces| crate::packs::mugshot(c, faces.shown(emotion, variant)));
    let face = picture.and_then(|m| {
        let (hud, n) = packs.mugshot(m);
        hud.mugshot(n)
    });
    if problems.lookup(Lookup::FormFace(form, emotion_number(emotion) | if variant { 0x10 } else { 0 })) && face.is_none() {
        problems.note(format!("form {:?} has no mugshot in the pack", c.defs.form(form).key));
    }
    (picture, face)
}

/// A navi's number in BN6's compat: its emblem and its emblem's palette
/// (the cursor's too) are by it (`sub_802812C`); 0, MegaMan's, for a navi
/// of another root.
pub fn navi_number(c: &Content, navi: NaviHandle, problems: &mut Problems) -> usize {
    let (number, known) = compat_navi_number(c, navi);
    if problems.lookup(Lookup::NaviNumber(navi)) && !known {
        problems.note(format!("navi {:?} has no number in BN6's compat: the custom screen shows MegaMan's emblem", c.defs.navi(navi).key));
    }
    number
}

/// [`navi_number`], once its lookup has been made.
pub fn navi_number_of(c: &Content, navi: NaviHandle) -> usize {
    compat_navi_number(c, navi).0
}

/// A navi's number, and false if it is BN6's but compat hasn't one.
fn compat_navi_number(c: &Content, navi: NaviHandle) -> (usize, bool) {
    let compat = bn6_compat::Compat::bn6_for(c);
    match compat.compat_key(c, &c.defs.navi(navi).key) {
        Some(local) => compat.navis.get(local).map_or((0, false), |n| (n.navi as usize, true)),
        None => (0, true),
    }
}

/// A navi's emblem (four 8x8 tiles) on a console of `version` (the pack's
/// version `console` names, for a game whose versions the engine doesn't
/// tell apart: `Renderer::console_version`), as the
/// custom screen's 4x4 sprite holds it (the middle four tiles).
pub fn emblem(a: &CustomScreen, c: &Content, navi: NaviHandle, version: GameVersion, console: Option<&str>, problems: &mut Problems) -> Tiles {
    let number = navi_number(c, navi, problems);
    let e = a.emblem_of.get(number).copied();
    let pictures = &a.versioned.get(console.unwrap_or(crate::custom::game_name(version))).emblems;
    let mut t = Tiles { pixels: vec![0; 16 * Tiles::TILE] };
    for (k, place) in [5usize, 6, 9, 10].into_iter().enumerate() {
        if let Some(src) = pictures.get(4 * e.unwrap_or(0) as usize + k) {
            t.pixels[place * Tiles::TILE..(place + 1) * Tiles::TILE].copy_from_slice(src);
        }
    }
    if problems.lookup(Lookup::Emblem(navi, version)) {
        let palette = a.emblem_palette_of.get(number).and_then(|&i| a.emblem_palettes.get(i as usize));
        if !e.is_some_and(|e| pictures.len() >= 4 * (e as usize + 1)) || palette.is_none() {
            problems.note(format!(
                "navi {:?} (number {number}) has no emblem or emblem colors on the {} custom screen",
                c.defs.navi(navi).key,
                crate::custom::game_name(version)
            ));
        }
    }
    t
}

/// A navi's name (`name`, its display text: its variant name if
/// `variant`) in the 8x16 font's glyphs (the custom screen's enemy names).
pub fn navi_name(hud: &Hud, navi: NaviHandle, variant: bool, name: &str, problems: &mut Problems) -> Vec<u16> {
    let (glyphs, missing) = crate::fonts::cell_glyphs(hud, name);
    let lookup = if variant { Lookup::NaviVariantName(navi) } else { Lookup::NaviName(navi) };
    if problems.lookup(lookup) && !missing.is_empty() {
        problems.note(format!("the navi named {name:?}: the pack's font has no glyph for {missing:?}"));
    }
    glyphs
}

/// A string the chatbox prints (`lookup` says whose), each of its first
/// three lines in the dialogue font's glyphs.
pub fn dialogue(font: &DialogueFont, lookup: Lookup, string: &str, problems: &mut Problems) -> Vec<Vec<u16>> {
    let first = problems.lookup(lookup);
    string
        .split('\n')
        .take(3)
        .map(|line| {
            let (glyphs, missing) = crate::fonts::dialogue_glyphs(font, line);
            if first && !missing.is_empty() {
                problems.note(format!("the chatbox's {line:?}: the pack's dialogue font has no glyph for {missing:?}"));
            }
            glyphs
        })
        .collect()
}

/// The chatbox's graphics and the dialogue font, if the pack has them.
pub fn chatbox_graphics(hud: &Hud, problems: &mut Problems) -> bool {
    let there = !hud.chatbox.is_empty() && !hud.dialogue_font.is_empty();
    if problems.lookup(Lookup::Graphics(Graphics::Chatbox)) && !there {
        problems.note("the pack has no chatbox graphics or dialogue font (extract it again)".into());
    }
    there
}

/// The custom screen's graphics, if the pack has them.
pub fn custom_graphics(a: &CustomScreen, problems: &mut Problems) -> bool {
    if problems.lookup(Lookup::Graphics(Graphics::CustomScreen)) && a.is_empty() {
        problems.note("the pack has no custom screen graphics (extract it again)".into());
    }
    !a.is_empty()
}

/// The warning marker, if the pack has it.
pub fn warning(hud: &Hud, problems: &mut Problems) -> bool {
    if problems.lookup(Lookup::Graphics(Graphics::Warning)) && hud.warning.is_empty() {
        problems.note("the pack has no warning marker (extract it again)".into());
    }
    !hud.warning.is_empty()
}

/// A Cross's name and colors in the Cross window: its game's pictures and
/// its number among that game's Crosses (`custom::cross_picture`), with
/// its name's tiles and colors there.
pub fn cross_name<'a>(
    a: &'a CustomScreen,
    c: &Content,
    navi: NaviHandle,
    form: FormHandle,
    problems: &mut Problems,
) -> Option<(&'a nettai_assets::VersionPictures, usize)> {
    let found = crate::custom::cross_picture(c, a, navi, form);
    if problems.lookup(Lookup::CrossName(form)) {
        let key = &c.defs.form(form).key;
        match found {
            None => problems.note(format!("form {key:?} is no Cross of its game's on the custom screen (no game, or not among its navi's five)")),
            Some((own, number)) => {
                let names = crate::custom::CROSS_NAME_TILES * (number + 5 + 1);
                if own.cross_names.len() < names || own.cross_palettes.len() < number + 5 + 1 {
                    problems.note(format!("form {key:?}: the custom screen has no name or colors for Cross {number}"));
                }
            }
        }
    }
    found
}

/// A banner's layout with glyphs: its pack's HUD, the layout and its
/// number there.
pub fn banner<'a>(packs: &Packs<'a>, c: &Content, id: BannerId, problems: &mut Problems) -> Option<(&'a Hud, &'a BannerLayout, u8)> {
    let found = packs.banner(c, id).and_then(|(hud, n)| hud.banners.get(n as usize / 4).filter(|l| !l.glyphs.is_empty()).map(|l| (hud, l, n)));
    if problems.lookup(Lookup::Banner(id)) && found.is_none() {
        problems.note(format!("banner {} has no glyphs in the pack", banner_name(c, id)));
    }
    found
}

/// A telop's banner's layout (a telop banner's: its glyphs are the chip's
/// name), its pack's HUD and its number there.
pub fn telop<'a>(packs: &Packs<'a>, c: &Content, id: BannerId, problems: &mut Problems) -> Option<(&'a Hud, &'a BannerLayout, u8)> {
    let found = packs.banner(c, id).and_then(|(hud, n)| hud.banners.get(n as usize / 4).map(|l| (hud, l, n)));
    if problems.lookup(Lookup::Telop(id)) {
        match found {
            None => problems.note(format!("the telop's banner {} is not in the pack", banner_name(c, id))),
            Some((_, l, _)) if l.kind != TELOP_KIND => {
                problems.note(format!("the telop's banner {} is no telop's in the pack (kind {})", banner_name(c, id), l.kind))
            }
            _ => {}
        }
    }
    found
}

/// Whether banner `id` is the damage judge's (its layout's kind, by its
/// number in its pack): the HUD draws the judge's numbers under it.
pub fn is_judge(packs: &Packs, c: &Content, id: BannerId) -> bool {
    packs.banner(c, id).and_then(|(hud, n)| hud.banners.get(n as usize / 4)).is_some_and(|l| l.kind == JUDGE_KIND)
}

/// Whether banner `id` is laid out as a telop's in its pack (its kind).
pub fn is_telop(packs: &Packs, c: &Content, id: BannerId) -> bool {
    packs.banner(c, id).and_then(|(hud, n)| hud.banners.get(n as usize / 4)).is_some_and(|l| l.kind == TELOP_KIND)
}

/// The kind of a telop's banner layout (`BannerLayout::kind`).
pub const TELOP_KIND: u8 = 3;
/// The kind of the damage judge's banner layout, which adds two numbers.
pub const JUDGE_KIND: u8 = 4;

/// A banner's name for a problem's text.
fn banner_name(c: &Content, id: BannerId) -> String {
    match (crate::packs::name(c, AssetKind::Banner, id.0), crate::packs::banner(c, id)) {
        (Some(name), Some(n)) => format!("{name:?} ({n:#04x})"),
        (Some(name), None) => format!("{name:?}"),
        _ => format!("handle {}", id.0),
    }
}

/// A telop that names a chip the engine wasn't told (a dimming content
/// starts itself without a `telop`).
pub fn telop_unknown(problems: &mut Problems) {
    if problems.lookup(Lookup::TelopUnknown) {
        problems.note("a telop names a chip the engine wasn't told (a dimming content starts itself)".into());
    }
}

/// A line of the HUD's text (`Hud::texts`).
pub fn text_line<'a>(hud: &'a Hud, line: usize, problems: &mut Problems) -> Option<&'a [u16]> {
    let glyphs = hud.texts.get(line).map(Vec::as_slice);
    if problems.lookup(Lookup::TextLine(line as u8)) && glyphs.is_none() {
        problems.note(format!("the pack has no HUD text line {line}"));
    }
    glyphs
}
