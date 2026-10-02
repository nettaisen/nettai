//! The custom screen's graphics (`graphics/custom`): one indexed PNG per
//! tile block, laid out as the screen draws it, and `custom.json` (the
//! window's maps and patches, which tile numbers the blocks load at, the
//! tables by navi). A chip's picture is a file under the chip's key
//! (`chip-art/<chip>.png`); `custom.json` lists them in the game's order.
//!
//! Each palette belongs to one image, as rows of that image's palette
//! (`palettes` in its entry); the three palettes no image owns are colour
//! lists in `custom.json`. Images drawn with another image's palette show
//! it for viewing only.
//!
//! What a game version shows of its own (`VersionPictures`: its Beast's
//! pictures, the emblems, its Crosses' names) is the base game's in the
//! directory and another version's under its name (`gregar/`), listed in
//! `versions`.

use crate::report::Report;
use crate::sprite::read_json;
use crate::stage::json_lines;
use crate::tiles::{self, Layout, TileImage};
use nettai_assets::{ChipArt, CustomScreen, MapEntry, MapPatch, Palette, PatchList, Picture, SlotPictures, Tiles, VersionPictures, Versioned};
use serde::{Deserialize, Serialize};
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
    /// Background palettes 11 (the slot icons), 12 (greyed out) and 14.
    pub icon_palette: Vec<String>,
    pub grey_palette: Vec<String>,
    pub other_palette: Vec<String>,
    /// Chips' pictures by chip key, in the game's order, each with its
    /// palette.
    pub chip_art: Vec<ChipArtDoc>,
    pub pictures: PicturesDoc,
    /// Chip codes as 8x16 glyphs; element icons, each in a palette row
    /// whose colours 10-15 are the ones it brings; damage digits (0-9, '?').
    pub codes: TileImage,
    pub elements: TileImage,
    pub digits: TileImage,
    /// The slots' codes (2x1, the last the empty slot's), the empty slot's
    /// icon, and the re-deal and scrap buttons.
    pub slot_codes: TileImage,
    pub empty_icon: TileImage,
    pub redeal_buttons: TileImage,
    pub scrap_buttons: TileImage,
    /// The base game's own pictures, and other versions' (in a directory
    /// under the version's name).
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
/// colours from 10 on.
fn element_rows(c: &CustomScreen) -> Vec<Palette> {
    c.element_colours
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
        })
        .collect();
    let p = &c.pictures;
    let mut picture = |file: &str, pic: &Picture| image(file, &pic.tiles, PICTURE, &[pic.palette], 1, &none);
    let ok = picture("pictures/ok.png", &p.ok);
    let ok_picked = picture("pictures/ok-picked.png", &p.ok_picked);
    let redeal = picture("pictures/redeal.png", &p.redeal);
    let scrap = picture("pictures/scrap.png", &p.scrap);
    let other = picture("pictures/other.png", &p.other);
    let mut version = |dir: &str, v: &VersionPictures, base: bool| {
        let emblem_rows = if base { c.emblem_palettes.len() } else { 0 };
        let names = v.cross_names.len() / CROSS_NAME_TILES;
        VersionDoc {
            beast_out: image(&format!("{dir}pictures/beast-out.png"), &v.beast_out.tiles, PICTURE, &v.beast_out_palettes, v.beast_out_palettes.len(), &none),
            beast_buttons: image(&format!("{dir}beast-buttons.png"), &v.beast_buttons, Layout::Blocks { width: 4, height: 2, columns: 1 }, &[frame0], 0, &none),
            emblems: image(&format!("{dir}emblems.png"), &v.emblems, ICONS(7), &c.emblem_palettes, emblem_rows, &|i| (i / 4) as u8),
            cross_names: image(&format!("{dir}cross-names.png"), &v.cross_names, CROSS_NAME, &v.cross_palettes, v.cross_palettes.len(), &|i| {
                ((i / CROSS_NAME_TILES).min(names.saturating_sub(1))) as u8
            }),
        }
    };
    let own = version("", &c.versioned.base, true);
    let versions = c
        .versioned
        .versions
        .iter()
        .map(|(name, v)| VersionEntry { version: name.clone(), own: version(&format!("{name}/"), v, false) })
        .collect();
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
    let maps = |m: &[Vec<MapEntry>]| m.iter().map(|m| m.iter().map(tiles::entry_text).collect()).collect();
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
        grey_palette: tiles::palette_text(&c.grey_palette),
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
        own,
        versions,
        cursor,
        cross_cursor,
        emblem_of: c.emblem_of.clone(),
        emblem_palette_of: c.emblem_palette_of.clone(),
        regular,
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
        chip_art.push(ChipArt { key: a.chip.clone(), picture });
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
    let mut versioned = Versioned::new(base);
    for v in &doc.versions {
        versioned.versions.push((v.version.clone(), version(&v.own, report)?.0));
    }
    let palette = |v: &[String], report: &mut Report| tiles::parse_palette(v, report, &name);
    Some(CustomScreen {
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
        grey_palette: palette(&doc.grey_palette, report),
        other_palette: palette(&doc.other_palette, report),
        chip_art,
        pictures,
        codes: img(&doc.codes, report)?.0,
        elements,
        element_colours: element_rows.iter().map(|p| std::array::from_fn(|i| p[10 + i])).collect(),
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
    })
}
