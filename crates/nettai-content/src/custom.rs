//! The custom screen's graphics (`graphics/custom`): one indexed PNG per
//! tile block, laid out as the screen draws it, and `custom.json` (the
//! window's maps and patches, which tile numbers the blocks load at, the
//! buttons' looks). A chip's picture is a file under the chip's key
//! (`chip-art/<chip>.png`), a navi's emblem one under the navi's
//! (`emblems/<navi>.png`), a button's tiles and picture ones under the
//! name its content registers it by (`buttons/<name>.png`,
//! `pictures/<name>.png`); `custom.json` lists each in the game's order.
//!
//! Each palette belongs to one image, as rows of that image's palette
//! (`palettes` in its entry); the three palettes no image owns are color
//! lists in `custom.json`. Images drawn with another image's palette show
//! it for viewing only.
//!
//! What a game version shows of its own (`VersionPictures`: its own look
//! of a button, its forms' names: EXE6's Crosses') is an image a version, each named with
//! its version (`form-names-falzar.png`, `form-names-gregar.png`;
//! without another version's, `form-names.png`): the base game's in `own`
//! (its version `base_version`), the others' in `versions`.

use crate::report::Report;
use crate::sprite::read_json;
use crate::stage::json_lines;
use crate::tiles::{self, Layout, TileImage};
use nettai_assets::{
    ButtonLettering, ButtonPictures, ButtonSets, ChipArt, CursorPlace, CustomLayout, CustomLettering, CustomScreen, Emblem, MapEntry, MapPatch, Palette,
    PatchList, Picture, SlotPictures, Tiles, VersionPictures, Versioned,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const FORMAT: &str = "nettai-content/custom";
/// 2: the form list window's pictures are named for it (`form_list_maps`,
/// `form_list_patches`, `form-list-cursor.png`, `form-names*.png`, the
/// layout's `form_names`), where 1 named them for EXE6's Cross window.
pub const VERSION: u32 = 2;

const GLYPHS: fn(u32) -> Layout = |columns| Layout::Blocks { width: 1, height: 2, columns };
const ICONS: fn(u32) -> Layout = |columns| Layout::Blocks { width: 2, height: 2, columns };
const PICTURE: Layout = Layout::Blocks { width: 7, height: 6, columns: 1 };
const FORM_NAME: Layout = Layout::Blocks { width: 9, height: 2, columns: 1 };
const FORM_NAME_TILES: usize = 18;

#[derive(Serialize, Deserialize, Debug)]
pub struct CustomDoc {
    pub format: String,
    pub version: u32,
    /// The window frame's tiles (from tile 1), with the window's palettes
    /// by the chip under the cursor: standard, mega, giga, dark.
    pub window: TileImage,
    /// The picked column's cells (from tile 0x89), the late turns' block
    /// (0x8D) and the enemy names' bar (0x1D6).
    pub column_cells: TileImage,
    pub turn_limit: TileImage,
    pub name_bar: TileImage,
    /// The window's maps (15x20, as `tile:palette[:flips]`): without and
    /// with the Cross tab; and the patches laid over them.
    pub window_maps: Vec<Vec<String>>,
    pub window_patches: PatchListDoc,
    /// The form list window's maps (EXE6's Cross window): its opening
    /// steps, then with one to five forms; and its patches.
    pub form_list_maps: Vec<Vec<String>>,
    pub form_list_patches: PatchListDoc,
    /// Background palettes 11 (the slot icons), 12 (grayed out) and 14.
    pub icon_palette: Vec<String>,
    pub gray_palette: Vec<String>,
    pub other_palette: Vec<String>,
    /// Chips' pictures by chip key, in the game's order, each with its
    /// palette.
    pub chip_art: Vec<ChipArtDoc>,
    pub pictures: PicturesDoc,
    /// Chip codes as 8x16 glyphs; element icons, each in a palette row
    /// whose colors 10-15 are the ones it brings; damage digits (0-9, '?').
    pub codes: TileImage,
    pub elements: TileImage,
    pub digits: TileImage,
    /// The slots' codes (2x1, the last the empty slot's) and the empty
    /// slot's icon.
    pub slot_codes: TileImage,
    pub empty_icon: TileImage,
    /// The base game's own pictures and its version's name, and the other
    /// versions'.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub base_version: String,
    pub own: VersionDoc,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub versions: Vec<VersionEntry>,
    /// Sprites: the cursor's corner (two frames), the form list window's cursor
    /// (corner and edge, two frames), the Regular chip's frame (two frames).
    pub cursor: TileImage,
    pub form_list_cursor: TileImage,
    /// The navis' emblems by navi key, each with its palette (sprite
    /// palette 11: the cursor's and the Regular chip's frame's too).
    pub emblems: Vec<EmblemDoc>,
    pub regular: TileImage,
    /// The Program Advance animation's names' first four colors, the sets
    /// it steps through.
    pub advance_name_colors: Vec<Vec<String>>,
    /// The other languages' pictures with words, by language (the HUD's
    /// `language` is the pack's own).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub languages: BTreeMap<String, CustomLanguageDoc>,
    /// Where the blocks go among the HUD layer's tile numbers: the game's
    /// own layout (every pack says its game's; none stands in).
    pub layout: LayoutDoc,
    /// The buttons, by the name their content registers them under: each
    /// one's tiles by set, its picture in the chip window with that
    /// picture's palettes, and how the screen draws it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buttons: Vec<ButtonDoc>,
}

/// A navi's emblem (`nettai_assets::Emblem`): its 2x2 tiles in its palette.
#[derive(Serialize, Deserialize, Debug)]
pub struct EmblemDoc {
    pub navi: String,
    pub image: TileImage,
}

/// `nettai_assets::CustomLayout`: the tile number each block loads at.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutDoc {
    pub column_cells: u16,
    pub turn_limit: u16,
    pub name: u16,
    pub art: u16,
    pub code: u16,
    pub element: u16,
    pub digits: u16,
    pub slots: u16,
    pub column_icons: u16,
    pub name_bar: u16,
    pub form_names: u16,
    pub slot_blank: u8,
    pub ok_cursor: CursorDoc,
}

/// `nettai_assets::CursorPlace`: the place, and each frame's four corners
/// as [y, x, hflip, vflip].
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorDoc {
    pub at: [i16; 2],
    pub corners: [[[i8; 4]; 4]; 2],
}

impl From<CursorPlace> for CursorDoc {
    fn from(p: CursorPlace) -> CursorDoc {
        CursorDoc { at: [p.x, p.y], corners: p.corners.map(|f| f.map(|(y, x, h, v)| [y, x, h as i8, v as i8])) }
    }
}

impl From<CursorDoc> for CursorPlace {
    fn from(d: CursorDoc) -> CursorPlace {
        CursorPlace { x: d.at[0], y: d.at[1], corners: d.corners.map(|f| f.map(|[y, x, h, v]| (y, x, h != 0, v != 0))) }
    }
}

impl From<CustomLayout> for LayoutDoc {
    fn from(l: CustomLayout) -> LayoutDoc {
        let CustomLayout {
            column_cells,
            turn_limit,
            name,
            art,
            code,
            element,
            digits,
            slots,
            column_icons,
            name_bar,
            form_names,
            slot_blank,
            ok_cursor,
        } = l;
        LayoutDoc {
            column_cells,
            turn_limit,
            name,
            art,
            code,
            element,
            digits,
            slots,
            column_icons,
            name_bar,
            form_names,
            slot_blank,
            ok_cursor: ok_cursor.into(),
        }
    }
}

impl From<LayoutDoc> for CustomLayout {
    fn from(l: LayoutDoc) -> CustomLayout {
        let LayoutDoc {
            column_cells,
            turn_limit,
            name,
            art,
            code,
            element,
            digits,
            slots,
            column_icons,
            name_bar,
            form_names,
            slot_blank,
            ok_cursor,
        } = l;
        CustomLayout {
            column_cells,
            turn_limit,
            name,
            art,
            code,
            element,
            digits,
            slots,
            column_icons,
            name_bar,
            form_names,
            slot_blank,
            ok_cursor: ok_cursor.into(),
        }
    }
}

/// A button drawn by name (`nettai_assets::ButtonPictures`): its tiles
/// (`size` tiles a cell, a set its cells'), which set a state shows
/// (`sets`: "each", "other" or "unavailable"), the cursor over it, and its
/// picture with its palettes as rows (its own first).
#[derive(Serialize, Deserialize, Debug)]
pub struct ButtonDoc {
    pub name: String,
    pub size: [u8; 2],
    pub tiles: TileImage,
    pub sets: String,
    /// The set its slot shows while the button isn't there (left out: the
    /// window's fill).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<u8>,
    pub cursor: CursorDoc,
    pub picture: TileImage,
    /// Whether the chip window shows its uses left (left out when false).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub uses_digit: bool,
    /// Where the icon of the chip it holds is drawn, [x, y] (left out:
    /// it holds none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held_at: Option<[i16; 2]>,
    /// The column's icons for what it gives, with the flying icon's palette.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icons: Option<TileImage>,
    /// The game versions whose consoles fly the icon in a palette of their
    /// own: the icons' image's palette rows after the first, in order (none:
    /// an empty list).
    pub icon_versions: Vec<String>,
}

/// Another language's pictures with words: its own files, named with the
/// language (`pictures/ok-ja.png`, `form-names-falzar-ja.png`,
/// `buttons/soul-ja.png`, `pictures/redeal-ja.png`).
#[derive(Serialize, Deserialize, Debug)]
pub struct CustomLanguageDoc {
    pub pictures: PicturesDoc,
    /// The form list window's names by game version.
    pub form_names: BTreeMap<String, TileImage>,
    /// What this language has of its own for a named button, by the
    /// button's name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub buttons: BTreeMap<String, ButtonLanguageDoc>,
}

/// A language's own tiles and picture of a named button
/// (`nettai_assets::ButtonLettering`): each left out where the pack's own
/// stands.
#[derive(Serialize, Deserialize, Debug)]
pub struct ButtonLanguageDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tiles: Option<TileImage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picture: Option<TileImage>,
}

/// A version's own pictures: its own look of a button (EXE6's Beast Out:
/// `buttons/beast_out-V.png`, `pictures/beast_out-V.png`) and the Cross
/// window's names with their palettes.
#[derive(Serialize, Deserialize, Debug)]
pub struct VersionDoc {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buttons: Vec<ButtonDoc>,
    pub form_names: TileImage,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VersionEntry {
    pub version: String,
    #[serde(flatten)]
    pub own: VersionDoc,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PatchListDoc {
    pub first_tile: u16,
    pub patches: Vec<PatchDoc>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PatchDoc {
    pub at: [u8; 2],
    pub size: [u8; 2],
    pub palette: u8,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub by_column: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChipArtDoc {
    pub chip: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<TileImage>,
    /// The game version whose ROM the chip's picture and icon are from,
    /// for a chip only its own version's ROM draws (`ChipArt::version`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PicturesDoc {
    pub ok: TileImage,
    pub ok_picked: TileImage,
    pub other: TileImage,
}

/// How a pack names a button's sets (`ButtonDoc::sets`).
const SETS: [(ButtonSets, &str); 3] = [(ButtonSets::Each, "each"), (ButtonSets::Other, "other"), (ButtonSets::Unavailable, "unavailable")];

/// What writes an image of an export: its file, tiles, layout, palette
/// rows, how many of them are data, and each tile's row.
type Image<'a> = dyn FnMut(&str, &Tiles, Layout, &[Palette], usize, &dyn Fn(usize) -> u8) -> TileImage + 'a;

/// A button's tiles as an image: a cell a block, a set's cells a row.
fn button_layout(b: &ButtonPictures, tiles: &Tiles) -> Layout {
    let (w, h) = (b.width as u32, b.height as u32);
    let cells = (tiles.len() as u32).div_ceil((w * h).max(1)).max(1);
    Layout::Blocks { width: w, height: h, columns: cells }
}

/// A button's entry; its files are `buttons/<file>.png`, `pictures/<file>.png`
/// and `buttons/<file>-icons.png` (`file`: its name, with its version for
/// a version's own).
fn button_doc(image: &mut Image, name: &str, file: &str, b: &ButtonPictures, frame0: Palette) -> ButtonDoc {
    let none = |_: usize| 0u8;
    let tiles = image(&format!("buttons/{file}.png"), &b.tiles, button_layout(b, &b.tiles), &[frame0], 0, &none);
    // (The picture's palettes: its own first.)
    let rows: Vec<Palette> = if b.palettes.is_empty() { vec![b.picture.palette] } else { b.palettes.clone() };
    let picture = image(&format!("pictures/{file}.png"), &b.picture.tiles, PICTURE, &rows, rows.len(), &none);
    let icon_rows: Vec<Palette> = std::iter::once(b.icon_palette).chain(b.icon_palettes.iter().map(|(_, p)| *p)).collect();
    let icons = (!b.icons.is_empty()).then(|| image(&format!("buttons/{file}-icons.png"), &b.icons, ICONS(14), &icon_rows, icon_rows.len(), &none));
    let icon_versions = if icons.is_some() { b.icon_palettes.iter().map(|(v, _)| v.clone()).collect() } else { Vec::new() };
    ButtonDoc {
        name: name.into(),
        size: [b.width, b.height],
        tiles,
        sets: SETS.iter().find(|(s, _)| *s == b.sets).map_or("each", |(_, n)| n).into(),
        hidden: b.hidden,
        cursor: b.cursor.into(),
        picture,
        uses_digit: b.uses_digit,
        held_at: b.held_at.map(|(x, y)| [x, y]),
        icons,
        icon_versions,
    }
}

fn patch_doc(p: &PatchList) -> PatchListDoc {
    PatchListDoc {
        first_tile: p.first_tile,
        patches: p
            .patches
            .iter()
            .map(|p| PatchDoc { at: [p.x, p.y], size: [p.width, p.height], palette: p.palette, by_column: p.by_column })
            .collect(),
    }
}

fn patch_list(d: &PatchListDoc) -> PatchList {
    PatchList {
        first_tile: d.first_tile,
        patches: d
            .patches
            .iter()
            .map(|p| MapPatch { x: p.at[0], y: p.at[1], width: p.size[0], height: p.size[1], palette: p.palette, by_column: p.by_column })
            .collect(),
    }
}

/// The element icons' palette rows: the icon palette with each element's
/// colors from 10 on.
fn element_rows(c: &CustomScreen) -> Vec<Palette> {
    c.element_colors
        .iter()
        .map(|e| {
            let mut p = c.icon_palette;
            p[10..].copy_from_slice(e);
            p
        })
        .collect()
}

pub fn export(c: &CustomScreen) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    let mut image = |file: &str, t: &Tiles, layout: Layout, rows: &[Palette], owned: usize, row_of: &dyn Fn(usize) -> u8| {
        let (png, doc) = tiles::export_image(file, t, layout, rows, [0, owned], row_of);
        files.push((file.to_string(), png));
        doc
    };
    let none = |_: usize| 0u8;
    let frame0 = c.frame_palettes.first().copied().unwrap_or([0; 16]);
    // (The cursor and the Regular chip's frame show in the first emblem's
    // palette: they are drawn in the navi's emblem's.)
    let emblem0 = c.emblems.first().map_or([0; 16], |e| e.palette);
    let grid = |columns| Layout::Grid { columns };
    let window = image("window.png", &c.window_tiles, grid(16), &c.frame_palettes, c.frame_palettes.len(), &none);
    let column_cells = image("column-cells.png", &c.column_cells, GLYPHS(2), &[frame0], 0, &none);
    let turn_limit = image("turn-limit.png", &c.turn_limit, grid(7), &[frame0], 0, &none);
    let name_bar = image("name-bar.png", &c.name_bar, GLYPHS(2), &[frame0], 0, &none);
    let chip_art = c
        .chip_art
        .iter()
        .map(|a| ChipArtDoc {
            chip: a.key.clone(),
            image: (!a.picture.tiles.is_empty())
                .then(|| image(&format!("chip-art/{}.png", a.key), &a.picture.tiles, PICTURE, &[a.picture.palette], 1, &none)),
            version: a.version.clone(),
        })
        .collect();
    let p = &c.pictures;
    let mut picture = |file: &str, pic: &Picture| image(file, &pic.tiles, PICTURE, &[pic.palette], 1, &none);
    let ok = picture("pictures/ok.png", &p.ok);
    let ok_picked = picture("pictures/ok-picked.png", &p.ok_picked);
    let other = picture("pictures/other.png", &p.other);
    let vs = &c.versioned;
    let mut version = |which: Option<&str>, v: &VersionPictures| {
        let names = v.form_names.len() / FORM_NAME_TILES;
        let file = |name: &str| format!("{}.png", vs.name(name, which));
        VersionDoc {
            buttons: v.buttons.iter().map(|(name, b)| button_doc(&mut image, name, &vs.name(name, which), b, frame0)).collect(),
            form_names: image(&file("form-names"), &v.form_names, FORM_NAME, &v.form_name_palettes, v.form_name_palettes.len(), &|i| {
                ((i / FORM_NAME_TILES).min(names.saturating_sub(1))) as u8
            }),
        }
    };
    let own = version(None, &vs.base);
    let versions = vs.versions.iter().map(|(name, v)| VersionEntry { version: name.clone(), own: version(Some(name), v) }).collect();
    let codes = image("codes.png", &c.codes, GLYPHS(28), &[frame0], 0, &none);
    let rows = element_rows(c);
    let elements = image("elements.png", &c.elements, ICONS(11), &rows, rows.len(), &|i| (i / 4) as u8);
    let digits = image("digits.png", &c.digits, GLYPHS(11), &[frame0], 0, &none);
    let slot_codes = image("slot-codes.png", &c.slot_codes, Layout::Blocks { width: 2, height: 1, columns: 14 }, &[frame0], 0, &none);
    let empty_icon = image("empty-icon.png", &c.empty_icon, ICONS(1), &[c.icon_palette], 0, &none);
    let emblems = c
        .emblems
        .iter()
        .map(|e| EmblemDoc { navi: e.navi.clone(), image: image(&format!("emblems/{}.png", e.navi), &e.tiles, ICONS(1), &[e.palette], 1, &none) })
        .collect();
    let cursor = image("cursor.png", &c.cursor, Layout::Blocks { width: 1, height: 1, columns: 2 }, &[emblem0], 0, &none);
    let form_list_cursor =
        image("form-list-cursor.png", &c.form_list_cursor, Layout::Blocks { width: 1, height: 1, columns: 4 }, &[c.form_list_cursor_palette], 1, &none);
    let regular = image("regular.png", &c.regular, Layout::Blocks { width: 4, height: 4, columns: 2 }, &[emblem0], 0, &none);
    let mut languages = BTreeMap::new();
    for (lang, l) in &c.languages {
        let p = &l.pictures;
        let mut picture = |file: &str, pic: &Picture| image(&format!("pictures/{file}-{lang}.png"), &pic.tiles, PICTURE, &[pic.palette], 1, &none);
        let pictures = PicturesDoc {
            ok: picture("ok", &p.ok),
            ok_picked: picture("ok-picked", &p.ok_picked),
            other: picture("other", &p.other),
        };
        let mut form_names = BTreeMap::new();
        for (version, names) in &l.form_names {
            let own = vs.version(version).unwrap_or(&vs.base);
            let n = names.len() / FORM_NAME_TILES;
            let file = format!("{}-{lang}.png", vs.name("form-names", Some(version)));
            let doc = image(&file, names, FORM_NAME, &own.form_name_palettes, 0, &|i| ((i / FORM_NAME_TILES).min(n.saturating_sub(1))) as u8);
            form_names.insert(version.clone(), doc);
        }
        let mut buttons = BTreeMap::new();
        for (name, own) in &l.buttons {
            // (Laid out as the button's own tiles are.)
            let Some((_, b)) = c.buttons.iter().find(|(n, _)| n == name) else { continue };
            let tiles = own.tiles.as_ref().map(|t| image(&format!("buttons/{name}-{lang}.png"), t, button_layout(b, t), &[frame0], 0, &none));
            let picture = own.picture.as_ref().map(|p| image(&format!("pictures/{name}-{lang}.png"), &p.tiles, PICTURE, &[p.palette], 1, &none));
            buttons.insert(name.clone(), ButtonLanguageDoc { tiles, picture });
        }
        languages.insert(lang.clone(), CustomLanguageDoc { pictures, form_names, buttons });
    }
    let buttons = c.buttons.iter().map(|(name, b)| button_doc(&mut image, name, name, b, frame0)).collect();
    let maps =|m: &[Vec<MapEntry>]| m.iter().map(|m| m.iter().map(tiles::entry_text).collect()).collect();
    let doc = CustomDoc {
        format: FORMAT.into(),
        version: VERSION,
        window,
        column_cells,
        turn_limit,
        name_bar,
        window_maps: maps(&c.window_maps),
        window_patches: patch_doc(&c.window_patches),
        form_list_maps: maps(&c.form_list_maps),
        form_list_patches: patch_doc(&c.form_list_patches),
        icon_palette: tiles::palette_text(&c.icon_palette),
        gray_palette: tiles::palette_text(&c.gray_palette),
        other_palette: tiles::palette_text(&c.other_palette),
        chip_art,
        pictures: PicturesDoc { ok, ok_picked, other },
        codes,
        elements,
        digits,
        slot_codes,
        empty_icon,
        // (Its name stands with or without another version's pictures: a
        // console with none of its own is of the base version.)
        base_version: vs.base_version.clone(),
        own,
        versions,
        cursor,
        form_list_cursor,
        emblems,
        regular,
        advance_name_colors: c.advance_name_colors.iter().map(|s| s.iter().map(|&c| tiles::color_text(c)).collect()).collect(),
        languages,
        layout: c.layout.into(),
        buttons,
    };
    files.push(("custom.json".into(), json_lines(&doc)));
    files
}

/// Read the custom screen's graphics.
pub fn import(dir: &Path, prefix: &str, report: &mut Report) -> Option<CustomScreen> {
    let name = format!("{prefix}/custom.json");
    let doc: CustomDoc = read_json(&dir.join("custom.json"), &name, report)?;
    if doc.format != FORMAT || doc.version != VERSION {
        report.error(&name, format!("not a {FORMAT} file of version {VERSION} (extract the pack again)"));
        return None;
    }
    let img = |d: &TileImage, report: &mut Report| tiles::import_image(dir, prefix, d, report);
    let map = |v: &[String], report: &mut Report| -> Vec<MapEntry> {
        v.iter()
            .map(|s| {
                tiles::parse_entry(s).unwrap_or_else(|e| {
                    report.error(&name, e);
                    MapEntry::default()
                })
            })
            .collect()
    };
    let one = |(t, p): (Tiles, Vec<Palette>)| Picture { tiles: t, palette: p.first().copied().unwrap_or([0; 16]) };
    let (window_tiles, frame_palettes) = img(&doc.window, report)?;
    let mut chip_art = Vec::new();
    for a in &doc.chip_art {
        let picture = match &a.image {
            Some(i) => one(img(i, report)?),
            None => Picture::default(),
        };
        chip_art.push(ChipArt { key: a.chip.clone(), picture, version: a.version.clone() });
    }
    let pictures = SlotPictures {
        ok: one(img(&doc.pictures.ok, report)?),
        ok_picked: one(img(&doc.pictures.ok_picked, report)?),
        other: one(img(&doc.pictures.other, report)?),
    };
    let (elements, element_rows) = img(&doc.elements, report)?;
    let button = |b: &ButtonDoc, report: &mut Report| -> Option<(String, ButtonPictures)> {
        let (picture, palettes) = img(&b.picture, report)?;
        let picture = Picture { tiles: picture, palette: palettes.first().copied().unwrap_or([0; 16]) };
        let tiles = img(&b.tiles, report)?.0;
        let (icons, icon_palette, icon_palettes) = match &b.icons {
            Some(i) => {
                let (t, p) = img(i, report)?;
                let own = b.icon_versions.iter().zip(p.iter().skip(1)).map(|(v, p)| (v.clone(), *p)).collect();
                (t, p.first().copied().unwrap_or([0; 16]), own)
            }
            None => (Tiles::default(), [0; 16], Vec::new()),
        };
        let sets = match SETS.iter().find(|(_, n)| *n == b.sets) {
            Some((s, _)) => *s,
            None => {
                report.error(&name, format!("button {}: its sets are \"each\", \"other\" or \"unavailable\", not {:?}", b.name, b.sets));
                ButtonSets::default()
            }
        };
        Some((
            b.name.clone(),
            ButtonPictures {
                width: b.size[0],
                height: b.size[1],
                tiles,
                sets,
                hidden: b.hidden,
                cursor: b.cursor.into(),
                picture,
                palettes,
                uses_digit: b.uses_digit,
                held_at: b.held_at.map(|[x, y]| (x, y)),
                icons,
                icon_palette,
                icon_palettes,
            },
        ))
    };
    let version = |d: &VersionDoc, report: &mut Report| -> Option<VersionPictures> {
        let mut buttons = Vec::new();
        for b in &d.buttons {
            buttons.push(button(b, report)?);
        }
        let (form_names, form_name_palettes) = img(&d.form_names, report)?;
        Some(VersionPictures { buttons, form_names, form_name_palettes })
    };
    let base = version(&doc.own, report)?;
    let (form_list_cursor, form_list_cursor_palettes) = img(&doc.form_list_cursor, report)?;
    let form_list_cursor_palette = form_list_cursor_palettes.first().copied().unwrap_or([0; 16]);
    let mut versioned = Versioned::new(&doc.base_version, base);
    for v in &doc.versions {
        versioned.versions.push((v.version.clone(), version(&v.own, report)?));
    }
    let palette = |v: &[String], report: &mut Report| tiles::parse_palette(v, report, &name);
    let mut languages = Vec::new();
    for (lang, d) in &doc.languages {
        let pictures = SlotPictures {
            ok: one(img(&d.pictures.ok, report)?),
            ok_picked: one(img(&d.pictures.ok_picked, report)?),
            other: one(img(&d.pictures.other, report)?),
        };
        let mut form_names = Vec::new();
        for (version, i) in &d.form_names {
            form_names.push((version.clone(), img(i, report)?.0));
        }
        let mut buttons = Vec::new();
        for (name, own) in &d.buttons {
            let tiles = match &own.tiles {
                Some(i) => Some(img(i, report)?.0),
                None => None,
            };
            let picture = match &own.picture {
                Some(i) => Some(one(img(i, report)?)),
                None => None,
            };
            buttons.push((name.clone(), ButtonLettering { tiles, picture }));
        }
        languages.push((lang.clone(), CustomLettering { pictures, form_names, buttons }));
    }
    let mut buttons = Vec::new();
    for b in &doc.buttons {
        buttons.push(button(b, report)?);
    }
    let mut emblems = Vec::new();
    for e in &doc.emblems {
        let (tiles, palettes) = img(&e.image, report)?;
        emblems.push(Emblem { navi: e.navi.clone(), tiles, palette: palettes.first().copied().unwrap_or([0; 16]) });
    }
    Some(CustomScreen {
        layout: doc.layout.into(),
        buttons,
        window_tiles,
        column_cells: img(&doc.column_cells, report)?.0,
        turn_limit: img(&doc.turn_limit, report)?.0,
        name_bar: img(&doc.name_bar, report)?.0,
        window_maps: doc.window_maps.iter().map(|m| map(m, report)).collect(),
        window_patches: patch_list(&doc.window_patches),
        form_list_maps: doc.form_list_maps.iter().map(|m| map(m, report)).collect(),
        form_list_patches: patch_list(&doc.form_list_patches),
        frame_palettes,
        icon_palette: palette(&doc.icon_palette, report),
        gray_palette: palette(&doc.gray_palette, report),
        other_palette: palette(&doc.other_palette, report),
        chip_art,
        pictures,
        codes: img(&doc.codes, report)?.0,
        elements,
        element_colors: element_rows.iter().map(|p| std::array::from_fn(|i| p[10 + i])).collect(),
        digits: img(&doc.digits, report)?.0,
        slot_codes: img(&doc.slot_codes, report)?.0,
        empty_icon: img(&doc.empty_icon, report)?.0,
        versioned,
        cursor: img(&doc.cursor, report)?.0,
        form_list_cursor,
        form_list_cursor_palette,
        emblems,
        regular: img(&doc.regular, report)?.0,
        advance_name_colors: doc
            .advance_name_colors
            .iter()
            .map(|set| {
                if set.len() != 4 {
                    report.error(&name, format!("a set of the Program Advance names' colors has 4 colors, not {}", set.len()));
                }
                std::array::from_fn(|k| set.get(k).and_then(|c| tiles::parse_color(c).ok()).map_or(0, |(c, _)| c))
            })
            .collect(),
        languages,
    })
}
