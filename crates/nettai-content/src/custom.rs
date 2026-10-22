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
    ButtonLettering, ButtonPictures, ButtonPlace, ButtonSets, ChipArt, CursorPlace, CustomLayout, CustomLettering, CustomScreen, ElementSprite, Emblem,
    MapEntry, MapPatch, Palette, PatchList, Picture, SlotPictures, Tiles, VersionPictures, Versioned, WindowEmblem, WindowLabel,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const FORMAT: &str = "nettai-content/custom";
pub const VERSION: u32 = 1;

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
    /// whose colors 10-15 are the ones it brings (each its symbol alone,
    /// clear elsewhere, over the frame they share: `element_frame`, viewed
    /// in the first one's row); damage digits (0-9, '?').
    pub codes: TileImage,
    pub elements: TileImage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element_frame: Option<TileImage>,
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
    /// The window's labels (`nettai_assets::WindowLabel`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<LabelDoc>,
    /// The buttons, by the name their content registers them under: each
    /// one's tiles by set, its picture in the chip window with that
    /// picture's palettes, and how the screen draws it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buttons: Vec<ButtonDoc>,
    /// The chip window's element icon as a sprite: its place and palette
    /// (left out: the window's tiles, EXE6's and EXE5's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element_sprite: Option<ElementSpriteDoc>,
    /// The cursor's and the Regular chip's frame's own palette (left out:
    /// the navi's emblem's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_palette: Option<Vec<String>>,
    /// The emblem the screen draws on its window by frames (left out: none,
    /// a navi's emblem sprite).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_emblem: Option<WindowEmblemDoc>,
    /// How its pictures' letters and digits look (`crate::letters`).
    pub letters: crate::letters::CustomLettersDoc,
    /// Its frames as shapes (`crate::frames`).
    #[serde(default, skip_serializing_if = "crate::shapes::CustomShapesDoc::is_empty")]
    pub shapes: crate::shapes::CustomShapesDoc,
}

/// `nettai_assets::WindowLabel`: its name, its tiles (the first and the
/// last), its letters' rows of them (the first and the last) and their
/// colors.
#[derive(Serialize, Deserialize, Debug)]
pub struct LabelDoc {
    pub name: String,
    pub tiles: [u16; 2],
    pub rows: [u8; 2],
    pub letters: Vec<u8>,
}

/// `nettai_assets::ElementSprite`: its place from the window's left edge
/// and the screen's top, its palette.
#[derive(Serialize, Deserialize, Debug)]
pub struct ElementSpriteDoc {
    pub at: [i16; 2],
    pub palette: Vec<String>,
}

/// `nettai_assets::WindowEmblem`: its first cell, its size in cells, the
/// tile number its tiles load at (`window-emblem.png`), its frames' map
/// entries (as `tile:palette[:flips]`), and the steps of its turn (a
/// frame's number, or null: hold).
#[derive(Serialize, Deserialize, Debug)]
pub struct WindowEmblemDoc {
    pub at: [u8; 2],
    pub size: [u8; 2],
    pub first_tile: u16,
    pub tiles: TileImage,
    pub frames: Vec<Vec<String>>,
    pub steps: Vec<Option<u8>>,
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
    /// (Left out: 9, EXE6's and EXE5's.)
    #[serde(default = "turn_limit_palette", skip_serializing_if = "is_turn_limit_palette")]
    pub turn_limit_palette: u8,
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
    /// (Left out: 8, EXE6's.)
    #[serde(default = "detail_blank")]
    pub detail_blank: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub empty_palette: Option<u8>,
    pub ok_cursor: CursorDoc,
    /// (Left out: (2, 100), EXE6's and EXE5's.)
    #[serde(default = "regular", skip_serializing_if = "is_regular")]
    pub regular: [i16; 2],
    /// The picked column's cells' two columns and first row, the form list
    /// window's names' first column and row, on the window's map.
    pub column_columns: [u8; 2],
    pub column_row: u8,
    pub form_names_at: [u8; 2],
}

fn detail_blank() -> u8 {
    8
}

fn regular() -> [i16; 2] {
    [2, 100]
}

fn is_regular(p: &[i16; 2]) -> bool {
    *p == regular()
}

fn turn_limit_palette() -> u8 {
    9
}

fn is_turn_limit_palette(p: &u8) -> bool {
    *p == turn_limit_palette()
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
            turn_limit_palette,
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
            detail_blank,
            empty_palette,
            ok_cursor,
            regular,
            column_columns,
            column_row,
            form_names_at,
        } = l;
        LayoutDoc {
            column_columns,
            column_row,
            form_names_at,
            column_cells,
            turn_limit,
            turn_limit_palette,
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
            detail_blank,
            empty_palette,
            ok_cursor: ok_cursor.into(),
            regular: [regular.0, regular.1],
        }
    }
}

impl From<LayoutDoc> for CustomLayout {
    fn from(l: LayoutDoc) -> CustomLayout {
        let LayoutDoc {
            column_cells,
            turn_limit,
            turn_limit_palette,
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
            detail_blank,
            empty_palette,
            ok_cursor,
            regular,
            column_columns,
            column_row,
            form_names_at,
        } = l;
        CustomLayout {
            column_columns,
            column_row,
            form_names_at,
            column_cells,
            turn_limit,
            turn_limit_palette,
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
            detail_blank,
            empty_palette,
            ok_cursor: ok_cursor.into(),
            regular: (regular[0], regular[1]),
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
    /// Its own place on the window, [column, row, first tile, palette]
    /// (left out: the patch list's, in the slots' run).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<[u16; 4]>,
}

/// Another language's pictures with words: its own files, named with the
/// language (`pictures/ok-ja.png`, `form-names-falzar-ja.png`,
/// `buttons/soul-ja.png`, `pictures/redeal-ja.png`).
#[derive(Serialize, Deserialize, Debug)]
pub struct CustomLanguageDoc {
    pub pictures: PicturesDoc,
    /// The form list window's names by game version (where the version's
    /// are layers, its letters over them).
    pub form_names: BTreeMap<String, TileImage>,
    /// What this language has of its own for a named button, by the
    /// button's name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub buttons: BTreeMap<String, ButtonLanguageDoc>,
    /// How this language's lettered pictures' letters look
    /// (`crate::letters`).
    pub letters: crate::letters::PictureLettersDoc,
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
    /// The Cross window's names: the bar they lie on (one name's tiles,
    /// every version's: `form-names-bar.png`) where the names are kept as
    /// layers over it
    /// (`letters::FormNameLayers`), else the names as they are.
    pub form_names: TileImage,
    /// Each name's portrait over the bar (a layer, `tiles::export_layer`,
    /// of every look's names: `NAME-portraits.png`, its palette's rows the
    /// names' palettes by look).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form_name_portraits: Option<TileImage>,
    /// The names' letters over both (a layer: `NAME-letters.png`; another
    /// language's, its `form_names`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form_name_letters: Option<TileImage>,
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
        place: b.place.map(|p| [p.x as u16, p.y as u16, p.first_tile, p.palette as u16]),
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
    // The pictures with letters in them, each language's as the base they
    // share with the letters taken off and its letters over it
    // (`tiles::export_layered`).
    let lettered = crate::letters::custom_layered(c).0;
    let name_layers = crate::letters::form_names_layered(c).0;
    let mut layer_files = Vec::new();
    let mut image = |file: &str, t: &Tiles, layout: Layout, rows: &[Palette], owned: usize, row_of: &dyn Fn(usize) -> u8| {
        let (pngs, doc) = match lettered.get(file) {
            Some(l) => {
                let layered = tiles::Layered { base: &l.base, base_file: l.base_file.as_deref() };
                tiles::export_layered(file, t, &layered, layout, rows, [0, owned], row_of)
            }
            None => {
                let (png, doc) = tiles::export_image(file, t, layout, rows, [0, owned], row_of);
                (vec![(file.to_string(), png)], doc)
            }
        };
        files.extend(pngs);
        doc
    };
    let none = |_: usize| 0u8;
    let frame0 = c.frame_palettes.first().copied().unwrap_or([0; 16]);
    // (The cursor and the Regular chip's frame show in their own palette,
    // or the first emblem's: they are drawn in the navi's emblem's.)
    let emblem0 = c.cursor_palette.unwrap_or_else(|| c.emblems.first().map_or([0; 16], |e| e.palette));
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
        let buttons = v.buttons.iter().map(|(name, b)| button_doc(&mut image, name, &vs.name(name, which), b, frame0)).collect();
        let look = |i: usize| ((i / FORM_NAME_TILES).min(names.saturating_sub(1))) as u8;
        let owned = v.form_name_palettes.len();
        match name_layers.iter().find(|(n, _)| n == which.unwrap_or(&vs.base_version)) {
            // Its names as layers: the bar, the portraits, the letters.
            Some((_, layers)) => {
                let stem = vs.name("form-names", which);
                let first = [v.form_name_palettes.first().copied().unwrap_or([0; 16])];
                // (One bar under every version's names: one file.)
                let (png, bar) = tiles::export_image("form-names-bar.png", &layers.bar, FORM_NAME, &first, [0, 0], |_| 0);
                if !layer_files.iter().any(|(f, _)| *f == bar.file) {
                    layer_files.push((bar.file.clone(), png));
                }
                let mut layer = |name: &str, pixels: &[Option<u8>], owned: usize| {
                    let (png, doc) = tiles::export_layer(&format!("{stem}-{name}.png"), pixels, v.form_names.len(), FORM_NAME, &v.form_name_palettes, [0, owned], look);
                    layer_files.push((doc.file.clone(), png));
                    doc
                };
                let own = layers.languages.first();
                let portraits = layer("portraits", &layers.portraits, owned);
                let letters = own.map(|l| layer("letters", &l.letters, 0));
                VersionDoc { buttons, form_names: bar, form_name_portraits: Some(portraits), form_name_letters: letters }
            }
            None => VersionDoc {
                buttons,
                form_names: image(&file("form-names"), &v.form_names, FORM_NAME, &v.form_name_palettes, owned, &look),
                form_name_portraits: None,
                form_name_letters: None,
            },
        }
    };
    let own = version(None, &vs.base);
    let versions = vs.versions.iter().map(|(name, v)| VersionEntry { version: name.clone(), own: version(Some(name), v) }).collect();
    let codes = image("codes.png", &c.codes, GLYPHS(28), &[frame0], 0, &none);
    // (A sprite's icons: one palette of its own, the sprite's, for viewing.)
    let mut icons = |file: &str, t: &Tiles| match &c.element_sprite {
        Some(e) => image(file, t, ICONS(11), &[e.palette], 0, &none),
        None => {
            let rows = element_rows(c);
            image(file, t, ICONS(11), &rows, rows.len(), &|i| (i / 4) as u8)
        }
    };
    let elements = icons("elements.png", &c.elements);
    let frame_row = c.element_sprite.map(|e| e.palette).or_else(|| element_rows(c).first().copied()).unwrap_or(c.icon_palette);
    let element_frame = (!c.element_frame.is_empty()).then(|| image("element-frame.png", &c.element_frame, ICONS(1), &[frame_row], 0, &none));
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
            let look = |i: usize| ((i / FORM_NAME_TILES).min(n.saturating_sub(1))) as u8;
            // (Its letters as a layer over the version's names' portraits
            // and bar, where those are layers.)
            let layered = name_layers.iter().find(|(v, _)| v == version).and_then(|(_, layers)| layers.languages.iter().find(|l| l.language == *lang));
            let doc = match layered {
                Some(theirs) => {
                    let (png, doc) = tiles::export_layer(&file, &theirs.letters, names.len(), FORM_NAME, &own.form_name_palettes, [0, 0], look);
                    layer_files.push((file.clone(), png));
                    doc
                }
                None => image(&file, names, FORM_NAME, &own.form_name_palettes, 0, &look),
            };
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
        let letters = crate::letters::pictures_doc(&l.letters);
        languages.insert(lang.clone(), CustomLanguageDoc { pictures, form_names, buttons, letters });
    }
    let buttons = c.buttons.iter().map(|(name, b)| button_doc(&mut image, name, name, b, frame0)).collect();
    let window_emblem = c.window_emblem.as_ref().map(|e| WindowEmblemDoc {
        at: [e.x, e.y],
        size: [e.width, e.height],
        first_tile: e.first_tile,
        tiles: image("window-emblem.png", &e.tiles, Layout::Grid { columns: 6 }, &[frame0], 0, &none),
        frames: e.frames.iter().map(|f| f.iter().map(tiles::entry_text).collect()).collect(),
        steps: e.steps.clone(),
    });
    let letters = crate::letters::custom_doc(&c.letters);
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
        element_frame,
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
        labels: c
            .labels
            .iter()
            .map(|l| LabelDoc { name: l.name.clone(), tiles: [l.first, l.last], rows: [l.top, l.bottom], letters: l.letters.clone() })
            .collect(),
        buttons,
        element_sprite: c.element_sprite.map(|e| ElementSpriteDoc { at: [e.x, e.y], palette: tiles::palette_text(&e.palette) }),
        cursor_palette: c.cursor_palette.as_ref().map(tiles::palette_text),
        window_emblem,
        letters,
        shapes: crate::shapes::write_custom(&mut files, &c.shapes),
    };
    files.extend(layer_files);
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
    // (Each image as it is too, by file: a lettered picture's is the
    // picture with its letters taken off, `lettered_bases`.)
    let bases: std::cell::RefCell<BTreeMap<String, Tiles>> = Default::default();
    let img = |d: &TileImage, report: &mut Report| {
        let (tiles, base, palettes) = tiles::import_lettered(dir, prefix, d, report)?;
        bases.borrow_mut().insert(d.file.clone(), base);
        Some((tiles, palettes))
    };
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
                place: b.place.map(|[x, y, first_tile, palette]| ButtonPlace { x: x as u8, y: y as u8, first_tile, palette: palette as u8 }),
            },
        ))
    };
    // A version's pictures, and its names with no letters where they are
    // kept as layers (their portraits over their bar).
    // A version's pictures, and where its names are layers, their bar (one
    // a look) and portraits and the pack's own names with no letters.
    type NameParts = (Tiles, Vec<Option<u8>>, Tiles);
    let version = |d: &VersionDoc, report: &mut Report| -> Option<(VersionPictures, Option<NameParts>)> {
        let mut buttons = Vec::new();
        for b in &d.buttons {
            buttons.push(button(b, report)?);
        }
        let (form_names, form_name_palettes, blank) = match (&d.form_name_portraits, &d.form_name_letters) {
            (None, None) => {
                let (t, p) = img(&d.form_names, report)?;
                (t, p, None)
            }
            (Some(portraits), Some(letters)) => {
                let (bar, _) = img(&d.form_names, report)?;
                let (portraits, palettes) = tiles::import_layer(dir, prefix, portraits, report)?;
                let (letters, _) = tiles::import_layer(dir, prefix, letters, report)?;
                if bar.len() != FORM_NAME_TILES || portraits.len() != letters.len() || portraits.len() % (FORM_NAME_TILES * Tiles::TILE) != 0 {
                    report.error(format!("{prefix}/{}", d.form_names.file), format!("a bar is one name's {FORM_NAME_TILES} tiles, under the names' portraits and letters"));
                    return None;
                }
                let bars = Tiles { pixels: bar.pixels.repeat(portraits.len() / (FORM_NAME_TILES * Tiles::TILE)) };
                let mut blank = bars.clone();
                crate::letters::lay(&mut blank, &portraits);
                let mut names = blank.clone();
                crate::letters::lay(&mut names, &letters);
                (names, palettes, Some((bars, portraits, blank)))
            }
            _ => {
                report.error(&name, "a version's names have both their portraits and their letters as layers, or neither");
                return None;
            }
        };
        Some((VersionPictures { buttons, form_names, form_name_palettes }, blank))
    };
    let (base, base_blank) = version(&doc.own, report)?;
    let (form_list_cursor, form_list_cursor_palettes) = img(&doc.form_list_cursor, report)?;
    let form_list_cursor_palette = form_list_cursor_palettes.first().copied().unwrap_or([0; 16]);
    let mut versioned = Versioned::new(&doc.base_version, base);
    let mut name_parts: Vec<(String, NameParts)> = base_blank.map(|b| (doc.base_version.clone(), b)).into_iter().collect();
    for v in &doc.versions {
        let (own, parts) = version(&v.own, report)?;
        versioned.versions.push((v.version.clone(), own));
        name_parts.extend(parts.map(|b| (v.version.clone(), b)));
    }
    // (The names' images with no letters, as the letters' blanks are read:
    // `version_bases`.)
    for (version, (_, _, blank)) in &name_parts {
        bases.borrow_mut().insert(format!("{}.png", versioned.name("form-names", Some(version))), blank.clone());
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
            match name_parts.iter().find(|(v, _)| v == version) {
                // (Its letters as a layer over the version's names'
                // portraits and bar.)
                Some((_, (_, _, blank))) => {
                    let (letters, _) = tiles::import_layer(dir, prefix, i, report)?;
                    if letters.len() != blank.pixels.len() {
                        report.error(format!("{prefix}/{}", i.file), "its letters aren't as many tiles as the names they lie over");
                        return None;
                    }
                    let mut names = blank.clone();
                    crate::letters::lay(&mut names, &letters);
                    bases.borrow_mut().insert(format!("{}-{lang}.png", versioned.name("form-names", Some(version))), blank.clone());
                    form_names.push((version.clone(), names));
                }
                None => form_names.push((version.clone(), img(i, report)?.0)),
            }
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
        // (Its letters once every image is read: below.)
        languages.push((lang.clone(), CustomLettering { pictures, form_names, buttons, letters: Default::default() }));
    }
    let mut buttons = Vec::new();
    for b in &doc.buttons {
        buttons.push(button(b, report)?);
    }
    let element_sprite =
        doc.element_sprite.as_ref().map(|e| ElementSprite { x: e.at[0], y: e.at[1], palette: tiles::parse_palette(&e.palette, report, &name) });
    let cursor_palette = doc.cursor_palette.as_ref().map(|p| tiles::parse_palette(p, report, &name));
    let window_emblem = match &doc.window_emblem {
        Some(e) => Some(WindowEmblem {
            x: e.at[0],
            y: e.at[1],
            width: e.size[0],
            height: e.size[1],
            first_tile: e.first_tile,
            tiles: img(&e.tiles, report)?.0,
            frames: e.frames.iter().map(|f| map(f, report)).collect(),
            steps: e.steps.clone(),
        }),
        None => None,
    };
    let mut emblems = Vec::new();
    for e in &doc.emblems {
        let (tiles, palettes) = img(&e.image, report)?;
        emblems.push(Emblem { navi: e.navi.clone(), tiles, palette: palettes.first().copied().unwrap_or([0; 16]) });
    }
    let mut screen = CustomScreen {
        layout: doc.layout.into(),
        labels: doc
            .labels
            .iter()
            .map(|l| WindowLabel { name: l.name.clone(), first: l.tiles[0], last: l.tiles[1], top: l.rows[0], bottom: l.rows[1], letters: l.letters.clone() })
            .collect(),
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
        element_frame: match &doc.element_frame {
            Some(i) => img(i, report)?.0,
            None => Tiles::default(),
        },
        // (A sprite's icons bring no colors to the window's palette.)
        element_colors: if element_sprite.is_some() {
            Vec::new()
        } else {
            element_rows.iter().map(|p| std::array::from_fn(|i| p[10 + i])).collect()
        },
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
        element_sprite,
        cursor_palette,
        window_emblem,
        letters: Default::default(),
        shapes: crate::shapes::read_custom(dir, prefix, &doc.shapes, report),
    };
    // The letters, their blanks the lettered pictures' images
    // (`lettered_bases`): a language's own where it has them, else the
    // pack's.
    let bases = bases.take();
    let none = Tiles::default();
    let base = |file: &str| bases.get(file).unwrap_or(&none);
    let picture_bases = |lang: Option<&str>| {
        let s = lang.map_or(String::new(), |l| format!("-{l}"));
        let own = |name: &str| -> &Tiles { bases.get(&format!("{name}{s}.png")).unwrap_or_else(|| base(&format!("{name}.png"))) };
        let vs = &screen.versioned;
        let version = |which: Option<&str>, v| version_bases(&bases, &none, vs, which, v, &s);
        crate::letters::PictureBases {
            ok: own("pictures/ok"),
            ok_picked: own("pictures/ok-picked"),
            other: own("pictures/other"),
            buttons: screen.buttons.iter().map(|(n, _)| (n.as_str(), own(&format!("pictures/{n}")))).collect(),
            versions: std::iter::once((vs.base_version.as_str(), version(None, &vs.base)))
                .chain(vs.versions.iter().map(|(n, v)| (n.as_str(), version(Some(n), v))))
                .map(|(n, (b, f))| (n, b, f))
                .collect(),
        }
    };
    let languages: Vec<Option<nettai_assets::letters::PictureLetters>> = screen
        .languages
        .iter()
        .map(|(lang, _)| doc.languages.get(lang.as_str()).map(|d| crate::letters::pictures_from_doc(&d.letters, &picture_bases(Some(lang)))))
        .collect();
    let letters = crate::letters::custom_from_doc(
        &doc.letters,
        &crate::letters::CustomBases {
            codes: base(&doc.codes.file),
            slot_codes: base(&doc.slot_codes.file),
            window: base(&doc.window.file),
            turn_limit: base(&doc.turn_limit.file),
            pictures: picture_bases(None),
        },
    );
    for ((_, l), letters) in screen.languages.iter_mut().zip(languages) {
        l.letters = letters.unwrap_or_default();
    }
    screen.letters = letters;
    // (Where each name's portrait is over its bar, in every language, as
    // `letters::settle` finds them.)
    for (version, (_, portraits, _)) in &name_parts {
        let mask = Tiles { pixels: portraits.iter().map(|p| matches!(p, Some(v) if *v != 0) as u8).collect() };
        let all = std::iter::once(&mut screen.letters.pictures.versioned).chain(screen.languages.iter_mut().map(|(_, l)| &mut l.letters.versioned));
        for v in all {
            if *version == v.base_version {
                v.base.form_name_portraits = mask.clone();
            } else if let Some((_, own)) = v.versions.iter_mut().find(|(n, _)| n == version) {
                own.form_name_portraits = mask.clone();
            }
        }
    }
    Some(screen)
}

/// A version's buttons' and form list names' images as the pack keeps them
/// (`bases`, by file: `lettered_bases`), the form list names a language's
/// own (`suffix`) where it has them (`letters::PictureBases`).
fn version_bases<'a>(
    bases: &'a BTreeMap<String, Tiles>,
    none: &'a Tiles,
    vs: &Versioned<VersionPictures>,
    which: Option<&str>,
    v: &'a VersionPictures,
    suffix: &str,
) -> (Vec<(&'a str, &'a Tiles)>, &'a Tiles) {
    let base = |file: String| bases.get(&file).unwrap_or(none);
    let buttons = v.buttons.iter().map(|(n, _)| (n.as_str(), base(format!("pictures/{}.png", vs.name(n, which))))).collect();
    let own = vs.name("form-names", Some(which.unwrap_or(&vs.base_version)));
    let names = bases.get(&format!("{own}{suffix}.png")).unwrap_or_else(|| base(format!("{}.png", vs.name("form-names", which))));
    (buttons, names)
}
