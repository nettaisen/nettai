//! The lookups the drawing code makes of the game's graphics and the content, each a
//! function that notes its [`Lookup`] and, the first time a run makes it,
//! what is missing (`crate::audit`; the audio's, a cue's song, is
//! nettai-tools' `sound_lookups`). A frame draws through these, and the
//! static audit (nettai-tools' `content_audit`) makes the same
//! lookups for everything the content defines, so a lookup by the wrong
//! key (a chip's icon by its qualified key, a banner by its handle) fails
//! both the same way.

use crate::audit::{GraphicsPart, Lookup, Problems};
use crate::graphics::Graphics;
use nettai_assets::{
    BannerLayout, ButtonPictures, ChipArt, CustomScreen, DialogueFont, Emblem, Hud, Palette, SpriteFrame, SpritePart, SpriteSheet, Tiles, VersionPictures,
};
use nettai_battle::content::{BackgroundId, BannerId, ChipClass, ChipFlags, Content, MugshotId, SpriteId};
use nettai_battle::field::PanelType;
use nettai_content_api::{AssetKind, ChipHandle, FormHandle, NaviHandle};

/// The game's letters of `what` (`nettai_assets::letters`: a picture's
/// letters or digits, as extraction read them off it), where the enhanced
/// mode draws its words or numbers in a font: none is a problem (the game's graphics
/// predates them, or its picture has none), and the picture is drawn as it
/// is.
pub fn letters<'a, T>(found: Option<&'a T>, what: impl FnOnce() -> String, problems: &mut Problems) -> Option<&'a T> {
    if found.is_none() {
        problems.note(format!("the game's graphics have no letters of {}", what()));
    }
    found
}

/// A sprite's name for a problem's text (`sprite "bomb"`).
pub fn sprite_name(c: &Content, id: SpriteId) -> String {
    match crate::graphics::name(c, AssetKind::Sprite, id.0) {
        Some(name) => format!("sprite {name:?}"),
        None => format!("sprite handle {}", id.0),
    }
}

/// Sprite `id`'s sheet, from its game's graphics; `who` says who asks (`of kind
/// "bomb"`, `(a portrait)`).
pub fn sprite<'a>(graphics: &Graphics<'a>, c: &Content, id: SpriteId, who: &dyn Fn() -> String, problems: &mut Problems) -> Option<&'a SpriteSheet> {
    let sheet = graphics.sprite(id);
    if problems.lookup(Lookup::Sprite(id)) && sheet.is_none() {
        problems.note(format!("{} {} is not in the game's graphics", sprite_name(c, id), who()));
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

/// A background, from its game's graphics.
pub fn background<'a>(graphics: &Graphics<'a>, c: &Content, id: BackgroundId, problems: &mut Problems) -> Option<&'a nettai_assets::Background> {
    let found = graphics.background(c, id);
    if problems.lookup(Lookup::Background(id)) && found.is_none() {
        let name = crate::graphics::name(c, AssetKind::Background, id.0).map_or(format!("handle {}", id.0), |n| format!("{n:?}"));
        problems.note(format!("background {name} is not in the game's graphics"));
    }
    found
}

/// The game's field's panel block for a panel type (`kind`, by its name),
/// owner (0 the viewer's) and row (`y`, 1..=3): a field that draws the type
/// (`stage::FieldArt`).
pub fn panel_block<'a>(
    c: &Content,
    field: &'a nettai_assets::Field,
    kind: PanelType,
    owner: usize,
    y: u8,
    problems: &mut Problems,
) -> Option<&'a [nettai_assets::MapEntry]> {
    let name = c.rules().panels.name(kind);
    let block = field.panel(name, owner, y).map(|b| &b[..]);
    if problems.lookup(Lookup::Panel(kind.0, owner as u8, y)) && block.is_none() {
        problems.note(format!("{}'s field has no block for panel type {name}, owner {owner}, row {y}", c.game()));
    }
    block
}

/// The number [`Lookup::PanelTint`] gives highlight `h` (1 or 2):
/// `HIGHLIGHT_TINT + h`.
pub const HIGHLIGHT_TINT: u8 = 16;

/// A panel type (`kind`, the game's number; a highlight,
/// `HIGHLIGHT_TINT + h`) that the game's field doesn't draw: drawn as a
/// tinted normal panel, by design (docs/design/rules-in-luau.md §7.4).
/// Said, not counted.
pub fn panel_tint(c: &Content, kind: u8, problems: &mut Problems) {
    if problems.lookup(Lookup::PanelTint(kind)) {
        let what = match kind.checked_sub(HIGHLIGHT_TINT) {
            Some(h) => format!("highlight {h}"),
            None => format!("panel type {kind} ({})", c.rules().panels.name(PanelType(kind))),
        };
        problems.say(format!("{}'s field doesn't draw {what}: drawn as a tinted normal panel", c.game()));
    }
}

/// A chip's key.
fn key(c: &Content, chip: ChipHandle) -> &str {
    &c.defs.chip(chip).key
}

/// A chip's icon: its game's, under its key there; with that HUD's
/// icon palette.
pub fn chip_icon<'a>(graphics: &Graphics<'a>, c: &Content, chip: ChipHandle, problems: &mut Problems) -> Option<(&'a Tiles, &'a Palette)> {
    let key = key(c, chip);
    let icon = graphics.chip_icon(key);
    if problems.lookup(Lookup::ChipIcon(chip)) && icon.is_none() {
        problems.note(format!("chip {key:?} has no icon in the game's graphics"));
    }
    Some((icon?, &graphics.bundle().hud.icon_palette))
}

/// What the caller names to have a chip's icon marked wherever a frame
/// draws it (`Problems::marking`): `chip-icon:KEY`, the chip's key in its
/// game.
pub fn chip_icon_mark(c: &Content, chip: ChipHandle) -> String {
    format!("chip-icon:{}", nettai_content_api::keys::local(key(c, chip)))
}

/// What the caller names to have a chip's description marked where a frame
/// prints it (`Problems::marking`): `chip-description:KEY`, the chip's key
/// in its game.
pub fn chip_description_mark(c: &Content, chip: ChipHandle) -> String {
    format!("chip-description:{}", nettai_content_api::keys::local(key(c, chip)))
}

/// A chip's picture in the chip window: its game's, under its key
/// there.
pub fn chip_art<'a>(graphics: &Graphics<'a>, c: &Content, chip: ChipHandle, problems: &mut Problems) -> Option<&'a ChipArt> {
    let key = key(c, chip);
    let art = graphics.chip_art(key);
    if problems.lookup(Lookup::ChipArt(chip)) && art.is_none() {
        problems.note(format!("chip {key:?} has no picture in the game's graphics"));
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
pub fn other_versions_icon(graphics: &Graphics, b: &nettai_battle::battle::Battle, chip: ChipHandle) -> bool {
    let console = crate::custom::console_version(b, graphics, b.setup.local_side);
    let art = graphics.chip_art(key(&b.content, chip));
    art.and_then(|a| a.version.as_deref()).is_some_and(|v| v != console)
}

/// A chip's name (`name`, its display text) in the 8x16 font's glyphs, all
/// of them (the HUD shows eight at most).
pub fn chip_name(hud: &Hud, c: &Content, chip: ChipHandle, name: &str, problems: &mut Problems) -> Vec<u16> {
    let (glyphs, missing) = crate::fonts::cell_glyphs(hud, name);
    if problems.lookup(Lookup::ChipName(chip)) && !missing.is_empty() {
        problems.note(format!("chip {:?} is named {name:?}, but the game's font has no glyph for {missing:?}", key(c, chip)));
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
    // (Graphics with one frame color draw every chip in it: EXE4's, which
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
        problems.note(format!("the Program Advance animation's {name:?}: the game's font has no glyph for {missing:?}"));
    }
    glyphs
}

/// A mugshot, from the game's HUD: the picture and its palettes.
pub fn mugshot<'a>(graphics: &Graphics<'a>, c: &Content, id: MugshotId, problems: &mut Problems) -> Option<(&'a Tiles, &'a [Palette])> {
    let face = graphics.mugshot(id).and_then(|n| graphics.bundle().hud.mugshot(n));
    if problems.lookup(Lookup::Mugshot(id)) && face.is_none() {
        let name = crate::graphics::name(c, AssetKind::Mugshot, id.0).map_or(format!("handle {}", id.0), |n| format!("{n:?}"));
        problems.note(format!("mugshot {name} is not in the game's graphics"));
    }
    face
}

/// A link navi's own face (its definition's mugshot): the picture and its
/// palettes (the second Full Synchro's).
pub fn navi_face<'a>(graphics: &Graphics<'a>, c: &Content, navi: NaviHandle, problems: &mut Problems) -> Option<(&'a Tiles, &'a [Palette], u8)> {
    let picture = c.navi(navi).mugshot.and_then(|m| graphics.mugshot(m));
    let face = picture.and_then(|n| graphics.bundle().hud.mugshot(n).map(|(t, p)| (t, p, n)));
    if problems.lookup(Lookup::NaviFace(navi)) && face.is_none() {
        problems.note(format!("navi {:?} has no mugshot in the game's graphics", c.defs.navi(navi).key));
    }
    face
}

/// The face a form shows for an emotion (its definition's mugshot for it, of
/// its second set when `variant`): the mugshot's number in the game's HUD,
/// and the picture with its palettes.
#[allow(clippy::type_complexity)]
pub fn form_face<'a>(
    graphics: &Graphics<'a>,
    c: &Content,
    form: FormHandle,
    emotion: nettai_battle::content::Emotion,
    variant: bool,
    problems: &mut Problems,
) -> (Option<u8>, Option<(&'a Tiles, &'a [Palette])>) {
    let picture = c.form(form).mugshot.as_ref().and_then(|faces| graphics.mugshot(faces.shown(&c.rules().emotion, emotion, variant)));
    let face = picture.and_then(|n| graphics.bundle().hud.mugshot(n));
    if problems.lookup(Lookup::FormFace(form, emotion, variant)) && face.is_none() {
        problems.note(format!("form {:?} has no mugshot in the game's graphics", c.defs.form(form).key));
    }
    (picture, face)
}

/// A navi's emblem on the custom screen: the game's under the navi's key
/// (`CustomScreen::emblems`), its tiles and its palette. None: the game's graphics have
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
        problems.note(format!("navi {key:?} has no emblem in the game's graphics (the custom screen draws none, and its cursor has no colors)"));
    }
    found
}

/// The game's look of the button named `name`: the version's own (`own`:
/// the pictures of the version the screen draws, `custom::button_pictures`),
/// else the game's.
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
        problems.note(format!("button {:?} has no look in the game's graphics (the custom screen draws nothing for it)", d.name));
    }
    found
}

/// A navi's name (`name`, its display text: its variant name if
/// `variant`) in the 8x16 font's glyphs (the custom screen's enemy names).
pub fn navi_name(hud: &Hud, navi: NaviHandle, variant: bool, name: &str, problems: &mut Problems) -> Vec<u16> {
    let (glyphs, missing) = crate::fonts::cell_glyphs(hud, name);
    let lookup = if variant { Lookup::NaviVariantName(navi) } else { Lookup::NaviName(navi) };
    if problems.lookup(lookup) && !missing.is_empty() {
        problems.note(format!("the navi named {name:?}: the game's font has no glyph for {missing:?}"));
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
                problems.note(format!("the chatbox's {line:?}: the game's dialogue font has no glyph for {missing:?}"));
            }
            glyphs
        })
        .collect()
}

/// The chatbox's graphics and the dialogue font, if the bundle has them
/// (the game's graphics always do).
pub fn chatbox_graphics(hud: &Hud, problems: &mut Problems) -> bool {
    let there = !hud.chatbox.is_empty() && !hud.dialogue_font.is_empty();
    if problems.lookup(Lookup::Graphics(GraphicsPart::Chatbox)) && !there {
        problems.note("the bundle has no chatbox graphics or dialogue font".into());
    }
    there
}

/// The custom screen's graphics, if the bundle has them (a game's graphics always
/// do).
pub fn custom_graphics(a: &CustomScreen, problems: &mut Problems) -> bool {
    if problems.lookup(Lookup::Graphics(GraphicsPart::CustomScreen)) && a.is_empty() {
        problems.note("the bundle has no custom screen graphics".into());
    }
    !a.is_empty()
}

/// The warning marker, if the bundle has it (the game's graphics always do).
pub fn warning(hud: &Hud, problems: &mut Problems) -> bool {
    if problems.lookup(Lookup::Graphics(GraphicsPart::Warning)) && hud.warning.is_empty() {
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

/// A banner's layout with glyphs: its game's HUD, the layout and its
/// number there.
pub fn banner<'a>(graphics: &Graphics<'a>, c: &Content, id: BannerId, problems: &mut Problems) -> Option<(&'a Hud, &'a BannerLayout, u8)> {
    let found = graphics.banner(id).and_then(|(hud, n)| hud.banners.get(n as usize / 4).filter(|l| !l.glyphs.is_empty()).map(|l| (hud, l, n)));
    if problems.lookup(Lookup::Banner(id)) && found.is_none() {
        problems.note(format!("banner {} has no glyphs in the game's graphics", banner_name(graphics, c, id)));
    }
    found
}

/// A telop's banner's layout (a telop banner's: its glyphs are the chip's
/// name), its game's HUD and its number there.
pub fn telop<'a>(graphics: &Graphics<'a>, c: &Content, id: BannerId, problems: &mut Problems) -> Option<(&'a Hud, &'a BannerLayout, u8)> {
    let found = graphics.banner(id).and_then(|(hud, n)| hud.banners.get(n as usize / 4).map(|l| (hud, l, n)));
    if problems.lookup(Lookup::Telop(id)) {
        match found {
            None => problems.note(format!("the telop's banner {} is not in the game's graphics", banner_name(graphics, c, id))),
            Some((_, l, _)) if l.kind != TELOP_KIND => {
                problems.note(format!("the telop's banner {} is no telop's in the game's graphics (kind {})", banner_name(graphics, c, id), l.kind))
            }
            _ => {}
        }
    }
    found
}

/// Whether banner `id` is the damage judge's: the content's judge banner
/// (the role `banners.judge`), or one its game's graphics lay out as the judge's (its
/// layout's kind, by its number there: EXE6's). The HUD draws the judge's
/// numbers under it. (EXE4's judge banner is a holding banner of its
/// table, 0x28, whose numbers its HUD task 9 draws: 0x080163C8.)
pub fn is_judge(graphics: &Graphics, c: &Content, id: BannerId) -> bool {
    c.defs.roles().banners.get(&nettai_battle::content::BannerRole::Judge) == Some(&id)
        || graphics.banner(id).and_then(|(hud, n)| hud.banners.get(n as usize / 4)).is_some_and(|l| l.kind == JUDGE_KIND)
}

/// Whether banner `id` is laid out as a telop's in the game's HUD (its kind).
pub fn is_telop(graphics: &Graphics, id: BannerId) -> bool {
    graphics.banner(id).and_then(|(hud, n)| hud.banners.get(n as usize / 4)).is_some_and(|l| l.kind == TELOP_KIND)
}

/// The kind of a telop's banner layout (`BannerLayout::kind`).
pub const TELOP_KIND: u8 = 3;
/// The kind of the damage judge's banner layout, which adds two numbers.
pub const JUDGE_KIND: u8 = 4;

/// A banner's name for a problem's text.
fn banner_name(graphics: &Graphics, c: &Content, id: BannerId) -> String {
    match (crate::graphics::name(c, AssetKind::Banner, id.0), graphics.banner(id).map(|(_, n)| n)) {
        (Some(name), Some(n)) => format!("{name:?} ({n:#04x})"),
        (Some(name), None) => format!("{name:?}"),
        _ => format!("handle {}", id.0),
    }
}

/// A telop with no look to draw it in: its game's graphics have no telop look
/// of its own and its roles name no telop banner.
pub fn telop_without_look(problems: &mut Problems) {
    if problems.lookup(Lookup::TelopWithoutLook) {
        problems.note("a telop has no look: the game's HUD has no telop look and the roles name no telop banner".into());
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
        problems.note(format!("the game's graphics have no HUD text line {line}"));
    }
    glyphs
}

/// Words a picture's lettering is drawn in `font` with (its `what`): the
/// characters of them the font lacks are a problem, the picture staying as
/// it is (the text font's are the bundled one's: the frontend's own).
pub fn drawn_in(what: &str, words: &str, font: crate::vfont::LetteringFont, p: &mut Problems) {
    use crate::vfont::LetteringFont;
    static TEXT: std::sync::LazyLock<crate::vfont::VectorFont> = std::sync::LazyLock::new(crate::vfont::VectorFont::bundled);
    let face: &crate::vfont::VectorFont = match font {
        LetteringFont::Display => crate::vfont::lettering_font(),
        LetteringFont::Numbers => crate::numbers::font(),
        LetteringFont::Text => &TEXT,
    };
    let missing: String = words.chars().filter(|&c| !face.covers(&c.to_string())).collect();
    if !missing.is_empty() {
        p.note(format!("{what}: the {font:?} font has no {missing:?} of its words {words:?}, so it stays a picture"));
    }
}

/// A look's colors as the graphics keep them (palette indices, put in the
/// palette of the moment: text-rendering.md §12.12): every one a palette
/// entry, its face's none 0 (transparent), its rim and its shadow none of
/// its face's. Else a problem.
fn look_colors(what: &str, fill: &[u8], rim: Option<u8>, shadow: Option<u8>, p: &mut Problems) {
    let all = fill.iter().chain(rim.iter()).chain(shadow.iter());
    let why = if all.clone().any(|&c| c > 15) {
        Some("an index past the palette's 16")
    } else if fill.is_empty() || fill.contains(&0) {
        Some("a face of no color or the transparent one")
    } else if rim.is_some_and(|c| fill.contains(&c)) {
        Some("a rim in its face's color")
    } else if shadow.is_some_and(|c| fill.contains(&c)) {
        Some("a shadow in its face's color")
    } else {
        None
    };
    if let Some(why) = why {
        p.note(format!("the game's look of {what} has {why} (fill {fill:?}, rim {rim:?}, shadow {shadow:?})"));
    }
}

/// Every look's colors of a HUD's and a custom screen's letters
/// ([`look_colors`]).
fn pack_look_colors(hud: &Hud, a: &CustomScreen, p: &mut Problems) {
    use nettai_assets::letters::{Digits, Letters};
    let letters = |what: &str, l: &Letters, p: &mut Problems| look_colors(what, &l.fill, l.rim, l.shadow, p);
    let digits = |what: &str, d: &Digits, p: &mut Problems| look_colors(what, &d.fill, d.rim, d.shadow, p);
    let l = &hud.letters;
    for (what, d) in [("the HP box's digits", &l.hp_digits), ("the damage digits", &l.damage_digits), ("the damage's '+'", &l.plus), ("the damage's \"x2\"", &l.times), ("the banners' digits", &l.banner_digits)] {
        if let Some(d) = d {
            digits(what, d, p);
        }
    }
    for (k, d) in l.enemy_digits.iter().enumerate() {
        if let Some(d) = d {
            digits(&format!("the HP digits of color {k}"), d, p);
        }
    }
    for (k, d) in l.telop.iter().flatten().enumerate() {
        digits(&format!("the telops' digits {k}"), d, p);
    }
    for (n, c) in l.counts.counts.iter().enumerate() {
        if let Some(c) = c {
            look_colors(&format!("the emotion window's count {n}"), &c.fill, c.rim, c.shadow, p);
        }
    }
    if let Some(x) = &l.pause {
        letters("\"PAUSE\"", x, p);
    }
    let w = &l.words;
    for (k, b) in w.banners.iter().enumerate() {
        let Some(b) = b else { continue };
        letters(&format!("banner {k}'s letters"), &b.letters, p);
    }
    if let Some(x) = &w.waiting {
        letters("the waiting label", x, p);
    }
    for (k, x) in w.l_or_r.iter().enumerate() {
        if let Some(x) = x {
            letters(&format!("L or R, frame {k}"), &x.letters, p);
        }
    }
    if let Some(g) = &w.gauge {
        letters("the gauge's word", &g.letters, p);
    }
    if a.is_empty() {
        return;
    }
    let cl = &a.letters;
    for (what, set) in [("the chip window's", &cl.codes), ("the slots'", &cl.slot_codes)] {
        for (k, g) in set.glyphs.iter().enumerate() {
            if let Some(g) = g {
                digits(&format!("{what} chip code {k}"), g, p);
            }
        }
    }
    if let Some(d) = &cl.digits {
        digits("the chip window's damage digits", d, p);
    }
    if let Some(t) = &cl.turn_limit {
        letters("FINAL TURN", &t.letters, p);
    }
    for (label, found) in a.labels.iter().zip(&cl.labels) {
        if let Some(found) = found {
            letters(&format!("window label {:?}", label.name), &found.letters, p);
        }
    }
    let pictures = &cl.pictures;
    let help = |what: &str, h: &Option<nettai_assets::letters::HelpLetters>, p: &mut Problems| {
        for line in h.iter().flat_map(|h| &h.lines) {
            look_colors(&format!("the chip window's picture {what}"), &[line.color], None, None, p);
        }
    };
    help("ok", &pictures.ok, p);
    help("ok-picked", &pictures.ok_picked, p);
    help("other", &pictures.other, p);
    for (name, h) in &pictures.buttons {
        help(name, h, p);
    }
    for (version, v) in std::iter::once((&pictures.versioned.base_version, &pictures.versioned.base)).chain(pictures.versioned.versions.iter().map(|(n, v)| (n, v))) {
        for (name, h) in &v.buttons {
            help(name, h, p);
        }
        for (k, f) in v.form_names.iter().enumerate() {
            if let Some(f) = f {
                look_colors(&format!("{version}'s form list name {k}"), &f.fill, Some(f.rim), None, p);
            }
        }
    }
}

/// The game's letters of everything its HUD and its custom screen draw in
/// fonts in the enhanced mode (`letters`, which a frame looks up as it
/// draws): its number sets and chip codes always, its lettered pictures
/// where the content's strings (`text`) have their words, a lettered
/// picture's lines as many as the words', its words all in the lettering
/// font. For the static audit; the
/// banners' and the form list's names are `banner_letters` and
/// `form_name_letters`.
pub fn pack_letters(c: &Content, hud: &Hud, a: &CustomScreen, text: &crate::strings::DisplayText, p: &mut Problems) {
    let need = |found: bool, what: &str, p: &mut Problems| {
        letters(found.then_some(&()), || what.to_string(), p);
    };
    pack_look_colors(hud, a, p);
    let l = &hud.letters;
    if !hud.tiles.is_empty() {
        need(l.hp_digits.is_some(), "the HP box's digits", p);
        need(l.damage_digits.is_some(), "the damage digits", p);
        need(l.plus.is_some(), "the damage's '+'", p);
        need(l.times.is_some(), "the damage's \"x2\"", p);
        for (k, d) in l.enemy_digits.iter().enumerate() {
            need(d.is_some(), &format!("the HP digits of color {k}"), p);
        }
        if hud.banners.iter().any(|b| b.number_at.is_some()) {
            need(l.banner_digits.is_some(), "the banners' digits", p);
        }
        if hud.layout.telop.is_some() {
            need(l.telop.is_some(), "the telops' digits", p);
        }
        if hud.counts.len() > 1 {
            need(l.counts.counts.iter().any(Option::is_some), "the emotion window's counts", p);
        }
    }
    let w = &l.words;
    for (key, found) in [
        ("pause", l.pause.is_some()),
        ("waiting", w.waiting.is_some()),
        ("gauge.custom", w.gauge.is_some()),
        ("gauge.l-or-r", w.l_or_r.iter().all(Option::is_some)),
        ("turn-limit", a.is_empty() || a.letters.turn_limit.is_some()),
    ] {
        if let Some(said) = text.lettering(c, key) {
            need(found, &format!("the picture of {key:?}"), p);
            // (The gauge's word in the numbers' font where it has every
            // character, else the display lettering's.)
            let font = if key == "gauge.custom" && crate::numbers::font().covers(said) { crate::vfont::LetteringFont::Numbers } else { crate::vfont::LetteringFont::Display };
            drawn_in(&format!("the picture of {key:?}"), said, font, p);
        }
    }
    if a.is_empty() {
        return;
    }
    let cl = &a.letters;
    need(cl.digits.is_some(), "the chip window's damage digits", p);
    for (what, set) in [("the chip window's", &cl.codes), ("the slots'", &cl.slot_codes)] {
        // (A to Z, then '*'.)
        need((0..27).all(|k| set.glyphs.get(k).is_some_and(Option::is_some)), &format!("{what} chip codes"), p);
    }
    for (k, label) in a.labels.iter().enumerate() {
        if let Some(said) = text.lettering(c, &format!("window.{}", label.name)) {
            need(cl.labels.get(k).is_some_and(Option::is_some), &format!("window label {:?}", label.name), p);
            drawn_in(&format!("window label {:?}", label.name), said, crate::vfont::LetteringFont::Numbers, p);
        }
    }
    // The chip window's lettered pictures: OK's, and each button's (the
    // game's, and each version's own).
    let help = |key: &str, found: Option<&nettai_assets::letters::HelpLetters>, p: &mut Problems| {
        let line = |part: &str| text.lettering(c, &format!("{key}.{part}"));
        let (Some(title), Some(said)) = (line("title"), line("words")) else { return };
        // (Words the lettering font can't draw leave the picture as it is:
        // the Japanese "シャッフル！" did, its full-width '!'.)
        drawn_in(&format!("the chip window's picture {key:?}"), &format!("{title}\n{said}"), crate::vfont::LetteringFont::Display, p);
        let lines = title.lines().count() + said.lines().count();
        match found {
            None => need(false, &format!("the chip window's picture {key:?}"), p),
            Some(f) if f.lines.len() != lines => {
                p.note(format!("the chip window's picture {key:?} has {} lines of letters, its words {lines}", f.lines.len()));
            }
            Some(_) => {}
        }
    };
    let pl = &cl.pictures;
    help("pictures.ok", pl.ok.as_ref(), p);
    help("pictures.ok-picked", pl.ok_picked.as_ref(), p);
    // (Battle mode 1's: no frame draws it yet, its words are kept whole.)
    help("pictures.other", pl.other.as_ref(), p);
    for (name, _) in &a.buttons {
        help(&format!("pictures.{}", nettai_content_api::keys::local(name)), pl.button(name), p);
    }
    for (version, own) in std::iter::once((&a.versioned.base_version, &a.versioned.base)).chain(a.versioned.versions.iter().map(|(v, o)| (v, o))) {
        let letters = pl.versioned.get(version);
        for (name, _) in &own.buttons {
            help(&format!("pictures.{}", nettai_content_api::keys::local(name)), letters.button(name), p);
        }
    }
}

/// The game's letters of banner `id` (its words', its bar's, its number's
/// places), where the content's strings have its words (`banner.NAME`).
pub fn banner_letters(graphics: &Graphics, c: &Content, id: BannerId, text: &crate::strings::DisplayText, p: &mut Problems) {
    let Some(name) = crate::graphics::name(c, AssetKind::Banner, id.0) else { return };
    let Some(words) = text.lettering(c, &format!("banner.{}", nettai_content_api::keys::local(name))) else { return };
    let Some((hud, layout, n)) = graphics.banner(id).and_then(|(hud, n)| hud.banners.get(n as usize / 4).filter(|l| !l.glyphs.is_empty()).map(|l| (hud, l, n))) else {
        return;
    };
    let found = hud.letters.words.banners.get(n as usize / 4).and_then(Option::as_ref);
    let numbered = layout.number_at.is_some() && words.contains("{n}");
    let complete = found.is_some_and(|f| !numbered || (f.one_digit.is_some() && f.two_digits.is_some()));
    letters(complete.then_some(&()), || format!("banner {name:?}"), p);
    // (Its number in the numbers' font.)
    drawn_in(&format!("banner {name:?}"), &words.replace("{n}", ""), crate::vfont::LetteringFont::Display, p);
}

/// The game's letters of form `form`'s name in the form list's window (its
/// version's, in both its looks), where the content's strings have its
/// words (`crosses.FORM`).
pub fn form_name_letters(a: &CustomScreen, c: &Content, navi: NaviHandle, form: FormHandle, text: &crate::strings::DisplayText, p: &mut Problems) {
    let key = nettai_content_api::keys::local(&c.defs.form(form).key);
    let Some(said) = text.lettering(c, &format!("crosses.{key}")) else { return };
    drawn_in(&format!("form {key:?}'s name"), said, crate::custom::CROSS_NAME_FONT, p);
    let Some((_, number)) = crate::custom::form_name_picture(c, a, navi, form) else { return };
    let Some(version) = c.form(form).version.as_deref() else { return };
    let own = a.letters.pictures.versioned.get(version);
    let looks = [number, number + CustomScreen::FORM_NAMES_UNDER_CURSOR];
    let found = looks.iter().all(|&k| own.form_names.get(k).is_some_and(Option::is_some));
    letters(found.then_some(&()), || format!("form {key:?}'s name"), p);
}
