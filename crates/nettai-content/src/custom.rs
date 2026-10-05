//! The custom screen's graphics (`graphics/custom`): one indexed PNG per
//! tile block, laid out as the screen draws it, and `custom.json` (the
//! window's maps and patches, which tile numbers the blocks load at, the
//! tables by navi). A chip's picture is a file under the chip's key
//! (`chip-art/<chip>.png`); `custom.json` lists them in the game's order.
//!
//! Each palette belongs to one image, as rows of that image's palette
//! (`palettes` in its entry); the three palettes no image owns are color
//! lists in `custom.json`. Images drawn with another image's palette show
//! it for viewing only.
//!
//! What a game version shows of its own (`VersionPictures`: its Beast's
//! pictures, the emblems, its Crosses' names) is an image a version, each
//! named with its version (`cross-names-falzar.png`, `cross-names-gregar.png`;
//! without another version's, `cross-names.png`): the base game's in `own`
//! (its version `base_version`), the others' in `versions`.

use crate::report::Report;
use crate::sprite::read_json;
use crate::stage::json_lines;
use crate::tiles::{self, Layout, TileImage};
use nettai_assets::{
    ButtonPictures, ChipArt, CursorPlace, CustomLayout, CustomLettering, CustomScreen, MapEntry, MapPatch, Palette, PatchList, Picture, SlotPictures, Tiles,
    VersionPictures, Versioned,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const FORMAT: &str = "nettai-content/custom";
pub const VERSION: u32 = 2;

const GLYPHS: fn(u32) -> Layout = |columns| Layout::Blocks { width: 1, height: 2, columns };
const ICONS: fn(u32) -> Layout = |columns| Layout::Blocks { width: 2, height: 2, columns };
const PICTURE: Layout = Layout::Blocks { width: 7, height: 6, columns: 1 };
const CROSS_NAME: Layout = Layout::Blocks { width: 9, height: 2, columns: 1 };
const CROSS_NAME_TILES: usize = 18;

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
    /// The Cross window's maps: its opening steps, then with one to five
    /// Crosses; and its patches.
    pub cross_maps: Vec<Vec<String>>,
    pub cross_patches: PatchListDoc,
    /// Background palettes 11 (the slot icons), 12 (grayed out) and 14.
    pub icon_palette: Vec<String>,
    /// (A pack extracted before the American spellings has the British key.)
    #[serde(alias = "grey_palette")]
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
    /// The slots' codes (2x1, the last the empty slot's), the empty slot's
    /// icon, and the re-deal and scrap buttons.
    pub slot_codes: TileImage,
    pub empty_icon: TileImage,
    pub redeal_buttons: TileImage,
    pub scrap_buttons: TileImage,
    /// The base game's own pictures (its version, when another version has
    /// its own), and the other versions'.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub base_version: String,
    pub own: VersionDoc,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub versions: Vec<VersionEntry>,
    /// Sprites: the cursor's corner (two frames), the Cross window's cursor
    /// (corner and edge, two frames), the Regular chip's frame (two frames).
    pub cursor: TileImage,
    pub cross_cursor: TileImage,
    /// Which emblem and emblem palette a navi shows, by the navi's number.
    pub emblem_of: Vec<u8>,
    pub emblem_palette_of: Vec<u8>,
    pub regular: TileImage,
    /// The Program Advance animation's names' first four colors, the sets
    /// it steps through. (A pack extracted before the American spellings has
    /// the British key.)
    #[serde(alias = "advance_name_colours")]
    pub advance_name_colors: Vec<Vec<String>>,
    /// The other languages' pictures with words, by language (the HUD's
    /// `language` is the pack's own).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub languages: BTreeMap<String, CustomLanguageDoc>,
    /// Where the blocks go among the HUD layer's tile numbers, for a game
    /// whose window is laid out otherwise than BN6's (none: BN6's,
    /// `CustomLayout::BN6`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<LayoutDoc>,
    /// The buttons drawn by name (BN5's soul button): each one's tiles by
    /// state and its picture in the chip window with that picture's
    /// palettes by state.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buttons: Vec<ButtonDoc>,
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
    pub cross_names: u16,
    pub slot_blank: u8,
    pub ok_cursor: CursorDoc,
    pub special_cursor: CursorDoc,
    /// (A pack from before the field: none shown.)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub button_uses: bool,
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
            cross_names,
            slot_blank,
            ok_cursor,
            special_cursor,
            button_uses,
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
            cross_names,
            slot_blank,
            ok_cursor: ok_cursor.into(),
            special_cursor: special_cursor.into(),
            button_uses,
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
            cross_names,
            slot_blank,
            ok_cursor,
            special_cursor,
            button_uses,
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
            cross_names,
            slot_blank,
            ok_cursor: ok_cursor.into(),
            special_cursor: special_cursor.into(),
            button_uses,
        }
    }
}

/// A button drawn by name (`nettai_assets::ButtonPictures`): its tiles
/// (`size` tiles a state), and its picture with a palette row a state.
#[derive(Serialize, Deserialize, Debug)]
pub struct ButtonDoc {
    pub name: String,
    pub size: [u8; 2],
    pub tiles: TileImage,
    pub picture: TileImage,
    /// The column's icons for what it gives, with the flying icon's palette.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icons: Option<TileImage>,
    /// The game versions whose consoles fly the icon in a palette of their
    /// own: the icons' image's palette rows after the first, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub icon_versions: Vec<String>,
}

/// Another language's pictures with words: its own files, named with the
/// language (`pictures/ok-ja.png`, `cross-names-falzar-ja.png`).
#[derive(Serialize, Deserialize, Debug)]
pub struct CustomLanguageDoc {
    pub pictures: PicturesDoc,
    /// The Cross window's names by game version.
    pub cross_names: BTreeMap<String, TileImage>,
}

/// A version's own pictures: Beast Out's picture with its palettes, the
/// Beast Out button, the navis' emblems (the base's with the emblems'
/// palettes), the Cross window's names with their palettes.
#[derive(Serialize, Deserialize, Debug)]
pub struct VersionDoc {
    pub beast_out: TileImage,
    pub beast_buttons: TileImage,
    pub emblems: TileImage,
    pub cross_names: TileImage,
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
    /// The region whose ROMs the picture comes from, when not the pack's
    /// own (`ChipArt::region`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// The game version whose ROM the chip's picture and icon are from,
    /// for a chip only its own version's ROM draws (`ChipArt::version`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PicturesDoc {
    pub ok: TileImage,
    pub ok_picked: TileImage,
    pub redeal: TileImage,
    pub scrap: TileImage,
    pub other: TileImage,
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
    let emblem0 = c.emblem_palettes.first().copied().unwrap_or([0; 16]);
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
            region: a.region.clone(),
            version: a.version.clone(),
        })
        .collect();
    let p = &c.pictures;
    let mut picture = |file: &str, pic: &Picture| image(file, &pic.tiles, PICTURE, &[pic.palette], 1, &none);
    let ok = picture("pictures/ok.png", &p.ok);
    let ok_picked = picture("pictures/ok-picked.png", &p.ok_picked);
    let redeal = picture("pictures/redeal.png", &p.redeal);
    let scrap = picture("pictures/scrap.png", &p.scrap);
    let other = picture("pictures/other.png", &p.other);
    let vs = &c.versioned;
    let mut version = |which: Option<&str>, v: &VersionPictures| {
        let emblem_rows = if which.is_none() { c.emblem_palettes.len() } else { 0 };
        let names = v.cross_names.len() / CROSS_NAME_TILES;
        let file = |name: &str| format!("{}.png", vs.name(name, which));
        VersionDoc {
            beast_out: image(&file("pictures/beast-out"), &v.beast_out.tiles, PICTURE, &v.beast_out_palettes, v.beast_out_palettes.len(), &none),
            beast_buttons: image(&file("beast-buttons"), &v.beast_buttons, Layout::Blocks { width: 4, height: 2, columns: 1 }, &[frame0], 0, &none),
            emblems: image(&file("emblems"), &v.emblems, ICONS(7), &c.emblem_palettes, emblem_rows, &|i| (i / 4) as u8),
            cross_names: image(&file("cross-names"), &v.cross_names, CROSS_NAME, &v.cross_palettes, v.cross_palettes.len(), &|i| {
                ((i / CROSS_NAME_TILES).min(names.saturating_sub(1))) as u8
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
    let buttons = |n| Layout::Blocks { width: 2, height: 3, columns: n };
    let redeal_buttons = image("redeal-buttons.png", &c.redeal_buttons, buttons(6), &[frame0], 0, &none);
    let scrap_buttons = image("scrap-buttons.png", &c.scrap_buttons, buttons(8), &[frame0], 0, &none);
    let cursor = image("cursor.png", &c.cursor, Layout::Blocks { width: 1, height: 1, columns: 2 }, &[emblem0], 0, &none);
    let cross_cursor =
        image("cross-cursor.png", &c.cross_cursor, Layout::Blocks { width: 1, height: 1, columns: 4 }, &[c.cross_cursor_palette], 1, &none);
    let regular = image("regular.png", &c.regular, Layout::Blocks { width: 4, height: 4, columns: 2 }, &[emblem0], 0, &none);
    let mut languages = BTreeMap::new();
    for (lang, l) in &c.languages {
        let p = &l.pictures;
        let mut picture = |file: &str, pic: &Picture| image(&format!("pictures/{file}-{lang}.png"), &pic.tiles, PICTURE, &[pic.palette], 1, &none);
        let pictures = PicturesDoc {
            ok: picture("ok", &p.ok),
            ok_picked: picture("ok-picked", &p.ok_picked),
            redeal: picture("redeal", &p.redeal),
            scrap: picture("scrap", &p.scrap),
            other: picture("other", &p.other),
        };
        let mut cross_names = BTreeMap::new();
        for (version, names) in &l.cross_names {
            let own = vs.version(version).unwrap_or(&vs.base);
            let n = names.len() / CROSS_NAME_TILES;
            let file = format!("{}-{lang}.png", vs.name("cross-names", Some(version)));
            let doc = image(&file, names, CROSS_NAME, &own.cross_palettes, 0, &|i| ((i / CROSS_NAME_TILES).min(n.saturating_sub(1))) as u8);
            cross_names.insert(version.clone(), doc);
        }
        languages.insert(lang.clone(), CustomLanguageDoc { pictures, cross_names });
    }
    let buttons = c
        .buttons
        .iter()
        .map(|(name, b)| {
            let (w, h) = (b.width as u32, b.height as u32);
            let states = (b.tiles.len() as u32).div_ceil((w * h).max(1)).max(1);
            let tiles = image(&format!("buttons/{name}.png"), &b.tiles, Layout::Blocks { width: w, height: h, columns: states }, &[frame0], 0, &none);
            let picture = image(&format!("pictures/{name}.png"), &b.picture.tiles, PICTURE, &b.palettes, b.palettes.len(), &none);
            let icon_rows: Vec<Palette> = std::iter::once(b.icon_palette).chain(b.icon_palettes.iter().map(|(_, p)| *p)).collect();
            let icons =
                (!b.icons.is_empty()).then(|| image(&format!("buttons/{name}-icons.png"), &b.icons, ICONS(14), &icon_rows, icon_rows.len(), &none));
            let icon_versions = if icons.is_some() { b.icon_palettes.iter().map(|(v, _)| v.clone()).collect() } else { Vec::new() };
            ButtonDoc { name: name.clone(), size: [b.width, b.height], tiles, picture, icons, icon_versions }
        })
        .collect();
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
        cross_maps: maps(&c.cross_maps),
        cross_patches: patch_doc(&c.cross_patches),
        icon_palette: tiles::palette_text(&c.icon_palette),
        gray_palette: tiles::palette_text(&c.gray_palette),
        other_palette: tiles::palette_text(&c.other_palette),
        chip_art,
        pictures: PicturesDoc { ok, ok_picked, redeal, scrap, other },
        codes,
        elements,
        digits,
        slot_codes,
        empty_icon,
        redeal_buttons,
        scrap_buttons,
        base_version: if vs.versions.is_empty() { String::new() } else { vs.base_version.clone() },
        own,
        versions,
        cursor,
        cross_cursor,
        emblem_of: c.emblem_of.clone(),
        emblem_palette_of: c.emblem_palette_of.clone(),
        regular,
        advance_name_colors: c.advance_name_colors.iter().map(|s| s.iter().map(|&c| tiles::color_text(c)).collect()).collect(),
        languages,
        layout: (c.layout != CustomLayout::BN6).then(|| c.layout.into()),
        buttons,
    };
    files.push(("custom.json".into(), json_lines(&doc)));
    files
}

/// Read the custom screen's graphics. A pack without them (extracted before
/// the custom screen was drawn) gets none, with a warning.
pub fn import(dir: &Path, prefix: &str, report: &mut Report) -> Option<CustomScreen> {
    let name = format!("{prefix}/custom.json");
    if !dir.join("custom.json").is_file() {
        report.warn(&name, "the pack has no custom screen graphics (extract it again to see the custom screen)");
        return Some(CustomScreen::default());
    }
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
        chip_art.push(ChipArt { key: a.chip.clone(), picture, region: a.region.clone(), version: a.version.clone() });
    }
    let pictures = SlotPictures {
        ok: one(img(&doc.pictures.ok, report)?),
        ok_picked: one(img(&doc.pictures.ok_picked, report)?),
        redeal: one(img(&doc.pictures.redeal, report)?),
        scrap: one(img(&doc.pictures.scrap, report)?),
        other: one(img(&doc.pictures.other, report)?),
    };
    let (elements, element_rows) = img(&doc.elements, report)?;
    let version = |d: &VersionDoc, report: &mut Report| -> Option<(VersionPictures, Vec<Palette>)> {
        let (beast_tiles, beast_out_palettes) = img(&d.beast_out, report)?;
        let (emblems, emblem_palettes) = img(&d.emblems, report)?;
        let (cross_names, cross_palettes) = img(&d.cross_names, report)?;
        let v = VersionPictures {
            beast_out: Picture { tiles: beast_tiles, palette: beast_out_palettes.first().copied().unwrap_or([0; 16]) },
            beast_out_palettes,
            beast_buttons: img(&d.beast_buttons, report)?.0,
            emblems,
            cross_names,
            cross_palettes,
        };
        Some((v, emblem_palettes))
    };
    let (base, emblem_palettes) = version(&doc.own, report)?;
    let (cross_cursor, cross_cursor_palettes) = img(&doc.cross_cursor, report)?;
    let cross_cursor_palette = cross_cursor_palettes.first().copied().unwrap_or([0; 16]);
    let mut versioned = Versioned::new(&doc.base_version, base);
    for v in &doc.versions {
        versioned.versions.push((v.version.clone(), version(&v.own, report)?.0));
    }
    let palette = |v: &[String], report: &mut Report| tiles::parse_palette(v, report, &name);
    let mut languages = Vec::new();
    for (lang, d) in &doc.languages {
        let pictures = SlotPictures {
            ok: one(img(&d.pictures.ok, report)?),
            ok_picked: one(img(&d.pictures.ok_picked, report)?),
            redeal: one(img(&d.pictures.redeal, report)?),
            scrap: one(img(&d.pictures.scrap, report)?),
            other: one(img(&d.pictures.other, report)?),
        };
        let mut cross_names = Vec::new();
        for (version, i) in &d.cross_names {
            cross_names.push((version.clone(), img(i, report)?.0));
        }
        languages.push((lang.clone(), CustomLettering { pictures, cross_names }));
    }
    let mut buttons = Vec::new();
    for b in &doc.buttons {
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
        buttons.push((
            b.name.clone(),
            ButtonPictures { width: b.size[0], height: b.size[1], tiles, picture, palettes, icons, icon_palette, icon_palettes },
        ));
    }
    Some(CustomScreen {
        layout: doc.layout.map_or(CustomLayout::BN6, CustomLayout::from),
        buttons,
        window_tiles,
        column_cells: img(&doc.column_cells, report)?.0,
        turn_limit: img(&doc.turn_limit, report)?.0,
        name_bar: img(&doc.name_bar, report)?.0,
        window_maps: doc.window_maps.iter().map(|m| map(m, report)).collect(),
        window_patches: patch_list(&doc.window_patches),
        cross_maps: doc.cross_maps.iter().map(|m| map(m, report)).collect(),
        cross_patches: patch_list(&doc.cross_patches),
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
        redeal_buttons: img(&doc.redeal_buttons, report)?.0,
        scrap_buttons: img(&doc.scrap_buttons, report)?.0,
        versioned,
        cursor: img(&doc.cursor, report)?.0,
        cross_cursor,
        cross_cursor_palette,
        emblem_palettes,
        emblem_of: doc.emblem_of.clone(),
        emblem_palette_of: doc.emblem_palette_of.clone(),
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
