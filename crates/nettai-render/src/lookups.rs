//! The lookups the drawing code makes of the packs and the content, each a
//! function that notes its [`Lookup`] and, the first time a run makes it,
//! what is missing (`crate::audit`; the audio's, a cue's song, is
//! nettai-tools' `sound_lookups`). A frame draws through these, and the
//! static audit (nettai-tools' `content_audit`) makes the same
//! lookups for everything the content defines, so a lookup by the wrong
//! key (a chip's icon by its qualified key, a banner by its handle) fails
//! both the same way.

use crate::audit::{Graphics, Lookup, Problems};
use crate::packs::Packs;
use nettai_assets::{
    BannerLayout, ButtonPictures, ChipArt, CustomScreen, DialogueFont, Emblem, Hud, Palette, SpriteFrame, SpritePart, SpriteSheet, Tiles, VersionPictures,
};
use nettai_battle::content::{BackgroundId, BannerId, ChipClass, ChipFlags, Content, MugshotId, PackId, SpriteId};
use nettai_battle::field::PanelType;
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

/// A pack's name for a problem's text (`exe6's pack`).
pub fn pack_name(c: &Content, pack: PackId) -> String {
    c.assets.packs.get(pack.index()).map_or(format!("pack {}", pack.index()), |g| format!("{g}'s pack"))
}

/// Pack `pack`'s field's panel block for a panel type (`kind`, by its
/// name), owner (0 the viewer's) and row (`y`, 1..=3): a field that draws
/// the type (`stage::FieldArt`).
pub fn panel_block<'a>(
    c: &Content,
    field: &'a nettai_assets::Field,
    pack: PackId,
    kind: PanelType,
    owner: usize,
    y: u8,
    problems: &mut Problems,
) -> Option<&'a [nettai_assets::MapEntry]> {
    let name = c.rules().panels.name(kind);
    let block = field.panel(name, owner, y).map(|b| &b[..]);
    if problems.lookup(Lookup::Panel(pack, kind.0, owner as u8, y)) && block.is_none() {
        problems.note(format!("{}'s field has no block for panel type {name}, owner {owner}, row {y}", pack_name(c, pack)));
    }
    block
}

/// The number [`Lookup::PanelTint`] gives highlight `h` (1 or 2):
/// `HIGHLIGHT_TINT + h`.
pub const HIGHLIGHT_TINT: u8 = 16;

/// A panel type (`kind`, the game's number; a highlight,
/// `HIGHLIGHT_TINT + h`) that no loaded pack's field draws, in an arena of
/// pack `arena`'s: drawn as a tinted normal panel, by design
/// (docs/design/rules-in-luau.md §7.4). Said, not counted.
pub fn panel_tint(c: &Content, arena: PackId, kind: u8, problems: &mut Problems) {
    if problems.lookup(Lookup::PanelTint(arena, kind)) {
        let what = match kind.checked_sub(HIGHLIGHT_TINT) {
            Some(h) => format!("highlight {h}"),
            None => format!("panel type {kind} ({})", c.rules().panels.name(PanelType(kind))),
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

/// Why a console draws the other version's chip otherwise than the frontend
/// (`ChipArt::version`).
pub const OTHER_VERSIONS_ART: &str = "another version's chip's art, its own ROM's (the console's ROM has its counterpart's there)";

/// Whether a chip's icon is another version's ROM's than the console's: a
/// version's own chip (`ChipArt::version`), which the other version's ROM
/// draws as its counterpart. The frontend shows the chip's own on either
/// console: its icon is a known difference where it shows, as its picture
/// is.
pub fn other_versions_icon(packs: &Packs, b: &nettai_battle::battle::Battle, chip: ChipHandle) -> bool {
    let console = crate::custom::console_version(b, packs, b.setup.local_side);
    let art = packs.chip_art(&b.content, key(&b.content, chip));
    art.and_then(|a| a.version.as_deref()).is_some_and(|v| v != console)
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
    // (A pack with one frame color draws every chip in it: EXE4's, which
    // its screen loads once, 0x0801DC28.)
    let frame = match class {
        _ if a.frame_palettes.len() == 1 => 0,
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
        let family = data.family.0 as usize;
        let has_icon = family < a.element_colors.len() || a.element_sprite.is_some();
        if has_icon && a.elements.len() < 4 * (family + 1) {
            problems.note(format!("chip {key:?}: the custom screen has no icon for its element ({family})"));
        }
        if a.codes.len() < 2 * (code.min(crate::custom::NO_CODE) as usize + 1) {
            problems.note(format!("chip {key:?}: the custom screen has no glyph for its code ({code})"));
        }
    }
    frame
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
    emotion: nettai_battle::content::Emotion,
    variant: bool,
    problems: &mut Problems,
) -> (Option<nettai_battle::content::InPack<u8>>, Option<(&'a Tiles, &'a [Palette])>) {
    let picture = c.form(form).mugshot.as_ref().and_then(|faces| crate::packs::mugshot(c, faces.shown(&c.rules().emotion, emotion, variant)));
    let face = picture.and_then(|m| {
        let (hud, n) = packs.mugshot(m);
        hud.mugshot(n)
    });
    if problems.lookup(Lookup::FormFace(form, emotion, variant)) && face.is_none() {
        problems.note(format!("form {:?} has no mugshot in the pack", c.defs.form(form).key));
    }
    (picture, face)
}

/// A navi's emblem on the custom screen: the pack's under the navi's key
/// (`CustomScreen::emblems`), its tiles and its palette. None: the pack has
/// none for the navi (nothing is drawn, and the cursor has no colors), or
/// the screen has an emblem of its own and its cursor its own colors
/// (EXE4's `window_emblem` and `cursor_palette`), and draws no navi's.
pub fn emblem<'a>(a: &'a CustomScreen, c: &Content, navi: NaviHandle, problems: &mut Problems) -> Option<&'a Emblem> {
    if a.window_emblem.is_some() && a.cursor_palette.is_some() {
        return None;
    }
    let key = &c.defs.navi(navi).key;
    let found = a.emblem(key).filter(|e| e.tiles.len() >= 4);
    if problems.lookup(Lookup::Emblem(navi)) && found.is_none() {
        problems.note(format!("navi {key:?} has no emblem in the pack (the custom screen draws none, and its cursor has no colors)"));
    }
    found
}

/// The pack's look of the button named `name`: the version's own (`own`:
/// the pictures of the version the screen draws, `custom::button_pictures`),
/// else the pack's.
pub fn button_of<'a>(a: &'a CustomScreen, own: &'a VersionPictures, name: &str) -> Option<&'a ButtonPictures> {
    own.button(name).or_else(|| a.button(name))
}

/// A button's look (`button_of`, by the name its content registers it
/// under). A button with none is drawn as nothing, which is a problem
/// unless it shows a chip (`ButtonDef::chip`: it is drawn as that chip's
/// slot).
pub fn button<'a>(
    a: &'a CustomScreen,
    own: &'a VersionPictures,
    c: &Content,
    button: nettai_battle::content::ButtonHandle,
    problems: &mut Problems,
) -> Option<&'a ButtonPictures> {
    let d = c.defs.button(button);
    let found = button_of(a, own, &d.name);
    if problems.lookup(Lookup::Button(button.0)) && found.is_none() && d.chip.is_none() {
        problems.note(format!("button {:?} has no look in the pack (the custom screen draws nothing for it)", d.name));
    }
    found
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

/// The chatbox's graphics and the dialogue font, if the bundle has them
/// (a pack always does).
pub fn chatbox_graphics(hud: &Hud, problems: &mut Problems) -> bool {
    let there = !hud.chatbox.is_empty() && !hud.dialogue_font.is_empty();
    if problems.lookup(Lookup::Graphics(Graphics::Chatbox)) && !there {
        problems.note("the bundle has no chatbox graphics or dialogue font".into());
    }
    there
}

/// The custom screen's graphics, if the bundle has them (a pack always
/// does).
pub fn custom_graphics(a: &CustomScreen, problems: &mut Problems) -> bool {
    if problems.lookup(Lookup::Graphics(Graphics::CustomScreen)) && a.is_empty() {
        problems.note("the bundle has no custom screen graphics".into());
    }
    !a.is_empty()
}

/// The warning marker, if the bundle has it (a pack always does).
pub fn warning(hud: &Hud, problems: &mut Problems) -> bool {
    if problems.lookup(Lookup::Graphics(Graphics::Warning)) && hud.warning.is_empty() {
        problems.note("the bundle has no warning marker".into());
    }
    !hud.warning.is_empty()
}

/// A form's name and colors in the form list's window: its version's
/// pictures and its number among that version's forms as `navi` lists them
/// (`custom::form_name_picture`), with its name's tiles and colors there.
pub fn form_name<'a>(
    a: &'a CustomScreen,
    c: &Content,
    navi: NaviHandle,
    form: FormHandle,
    problems: &mut Problems,
) -> Option<(&'a nettai_assets::VersionPictures, usize)> {
    let found = crate::custom::form_name_picture(c, a, navi, form);
    if problems.lookup(Lookup::FormName(form)) {
        let key = &c.defs.form(form).key;
        match found {
            None => {
                let navi = &c.defs.navi(navi).key;
                problems.note(format!("form {key:?} has no name on the custom screen (it says no `version`, or navi {navi:?} doesn't list it among that version's forms)"));
            }
            Some((own, number)) => {
                let names = crate::custom::FORM_NAME_TILES * (number + 5 + 1);
                if own.form_names.len() < names || own.form_name_palettes.len() < number + 5 + 1 {
                    problems.note(format!("form {key:?}: the custom screen has no name or colors for its version's form {number}"));
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

/// Whether banner `id` is the damage judge's: the content's judge banner
/// (the role `banners.judge`), or one its pack lays out as the judge's (its
/// layout's kind, by its number there: EXE6's). The HUD draws the judge's
/// numbers under it. (EXE4's judge banner is a holding banner of its
/// table, 0x28, whose numbers its HUD task 9 draws: 0x080163C8.)
pub fn is_judge(packs: &Packs, c: &Content, id: BannerId) -> bool {
    c.defs.roles().banners.get(&nettai_battle::content::BannerRole::Judge) == Some(&id)
        || packs.banner(c, id).and_then(|(hud, n)| hud.banners.get(n as usize / 4)).is_some_and(|l| l.kind == JUDGE_KIND)
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

/// A telop with no look to draw it in: its game's pack has no telop look
/// of its own and its roles name no telop banner.
pub fn telop_without_look(problems: &mut Problems) {
    if problems.lookup(Lookup::TelopWithoutLook) {
        problems.note("a telop has no look: the pack's HUD has no telop look and the roles name no telop banner".into());
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
