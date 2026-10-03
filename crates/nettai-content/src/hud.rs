//! The HUD's graphics: one indexed PNG per tile block, laid out as the
//! game draws it (8x16 glyphs, 2x2 icons, 4x2 mugshots), and `hud.json`
//! (map entries, what the font's glyphs draw, banner layouts). Mugshots,
//! banners and chip icons are a file each, under their names
//! (`mugshots/<name>.png`, `banners/<name>.png`, `chip-icons/<chip>.png`,
//! a chip's key); `hud.json` lists them in the game's order. A chip's name
//! and whether its damage shows are the content's (its chip definition).
//!
//! Each palette belongs to one image, as rows of that image's palette
//! (`palettes` in its entry). Images drawn with another image's palette
//! show it for viewing only.

use crate::report::Report;
use crate::sprite::read_json;
use crate::stage::json_lines;
use crate::tiles::{self, Layout, TileImage};
use nettai_assets::{BannerLayout, Chatbox, ChipIcon, DialogueFont, Hud, HudLettering, MapEntry, NaviMugshot, Palette, Tiles};
use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const FORMAT: &str = "nettai-content/hud";
pub const VERSION: u32 = 7;

const GLYPHS: fn(u32) -> Layout = |columns| Layout::Blocks { width: 1, height: 2, columns };

#[derive(Serialize, Deserialize, Debug)]
pub struct HudDoc {
    pub format: String,
    pub version: u32,
    /// HUD layer tiles from `first_tile`; its image holds the three HP box
    /// palettes (normal, healing, hurt or low).
    pub layer: TileImage,
    pub first_tile: u16,
    /// Gauge tiles from `gauge_first_tile`, with the gauge palette.
    pub gauge: TileImage,
    pub gauge_first_tile: u16,
    /// The HP box (6x2) and gauge frame (18x2) as first placed.
    pub hp_box: Vec<String>,
    pub gauge_frame: Vec<String>,
    pub font: TileImage,
    /// What each glyph of the font draws, as text (the game's marks as
    /// characters: U+E002 for the stacked EX, which Unicode has none for;
    /// version 7 on).
    pub font_chars: Vec<String>,
    /// The opponent's HP digits: normal, dropping, rising (a row each),
    /// with their palette.
    pub enemy_digits: TileImage,
    /// Chip icons by chip key, in the game's order, each with the icon
    /// palette.
    pub chip_icons: Vec<ChipIconDoc>,
    /// The icon a hidden chip shows.
    pub hidden_icon: TileImage,
    /// Mugshots in the mugshot table's order, each with its palette.
    pub mugshots: Vec<TileImage>,
    /// The count box showing 0..=10, then without a number.
    pub counts: TileImage,
    /// The link navis' faces, each with its two palettes (normal, Full
    /// Synchro), and the box beside them.
    pub navi_mugshots: Vec<TileImage>,
    pub navi_box: TileImage,
    /// "PAUSE": five glyphs (shown with the opponents' HP digits' palette).
    pub pause: TileImage,
    /// The HUD's text lines, each as the font's glyph numbers.
    pub texts: Vec<Vec<u16>>,
    /// Banners by banner id / 4.
    pub banners: Vec<BannerDoc>,
    pub banner_digits: TileImage,
    /// "Cstmzing..." and its palette.
    pub waiting: TileImage,
    /// The warning marker's two frames (2x2 tiles each) and its palette
    /// (none in a pack extracted before it was).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<TileImage>,
    /// The dialogue font (none in a pack extracted before it was).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialogue_font: Option<DialogueFontDoc>,
    /// The chatbox (none in a pack extracted before it was).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chatbox: Option<ChatboxDoc>,
    /// The language the fonts, the text lines and the pictures with words
    /// are in (none: `nettai_assets::BASE_LANGUAGE`), and the other
    /// languages' lettering, by language.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub language: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub languages: BTreeMap<String, HudLanguageDoc>,
}

/// Another language's lettering of the HUD: its own files, named with the
/// language (`font-ja.png`, `banners/megaman-deleted-ja.png`).
#[derive(Serialize, Deserialize, Debug)]
pub struct HudLanguageDoc {
    pub font: TileImage,
    pub font_chars: Vec<String>,
    pub dialogue_font: DialogueFontDoc,
    /// The HUD's text lines, each as this font's glyph numbers.
    pub texts: Vec<Vec<u16>>,
    /// The banners whose words this language has its own of, by banner id
    /// / 4 (null: the pack's own), with this language's banner palette.
    pub banners: Vec<Option<BannerDoc>>,
    /// "Cstmzing..." (two rows, as wide as its words), with the banner
    /// palette.
    pub waiting: TileImage,
    pub gauge: TileImage,
}

/// The chatbox: the box's tiles with its palette, its maps (30x8 entries a
/// row of text each, the tiles counted from the image's first) by kind (the
/// message box, the description box) and opening step (0 to 3, open), and
/// the key-wait arrow's three frames with the palette the text draws with.
#[derive(Serialize, Deserialize, Debug)]
pub struct ChatboxDoc {
    pub tiles: TileImage,
    pub boxes: Vec<Vec<Vec<String>>>,
    pub arrow: TileImage,
}

/// The dialogue font: an indexed image of its glyphs (16x12 cells, 32 a
/// row, palette index 0 clear; the palette only colors it for viewing),
/// each glyph's advance, and what each draws (as `font_chars`: the shared
/// glyphs, then bytes 0xE0-0xE3 and the two-byte codes E4 00 on).
#[derive(Serialize, Deserialize, Debug)]
pub struct DialogueFontDoc {
    pub file: String,
    pub cell: [usize; 2],
    pub columns: usize,
    pub advances: Vec<u8>,
    pub chars: Vec<String>,
}

const DIALOGUE_COLUMNS: usize = 32;

/// The dialogue font's image.
fn dialogue_image(f: &DialogueFont) -> crate::image::Indexed {
    let (w, h) = (DialogueFont::WIDTH, DialogueFont::HEIGHT);
    let rows = f.len().div_ceil(DIALOGUE_COLUMNS);
    // Clear, the face, the shade, then grays.
    let palette = (0..16u8).map(|i| match i {
        0 => [255, 255, 255],
        1 => [0, 0, 0],
        2 => [96, 96, 96],
        3 => [160, 160, 160],
        i => [i * 12, i * 12, 255 - i * 8],
    });
    let mut img = crate::image::Indexed::new((w * DIALOGUE_COLUMNS) as u32, (h * rows) as u32, palette.collect());
    for g in 0..f.len() {
        let (gx, gy) = ((g % DIALOGUE_COLUMNS) * w, (g / DIALOGUE_COLUMNS) * h);
        for (k, &v) in f.glyph(g).unwrap_or(&[]).iter().enumerate() {
            img.set((gx + k % w) as u32, (gy + k / w) as u32, v);
        }
    }
    img
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ChipIconDoc {
    /// The chip's key.
    pub chip: String,
    /// None for a chip without an icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<TileImage>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BannerDoc {
    pub at: [u8; 2],
    pub kind: u8,
    /// Glyphs it has (20, or 0 for the telops, drawn from the chip's name).
    pub glyphs: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_at: Option<[u8; 2]>,
    /// Its glyphs, with the banner palette (none for the telops).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<TileImage>,
}

fn concat(parts: &[&Tiles]) -> Tiles {
    Tiles { pixels: parts.iter().flat_map(|t| t.pixels.iter().copied()).collect() }
}

pub fn export(h: &Hud, names: &crate::names::AssetNames) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    let mut image = |file: &str, t: &Tiles, layout: Layout, rows: &[Palette], owned: usize| {
        let (png, doc) = tiles::export_image(file, t, layout, rows, [0, owned], |_| 0);
        files.push((file.to_string(), png));
        doc
    };
    let grid = Layout::Grid { columns: 16 };
    let layer = image("layer.png", &h.tiles, grid, &h.hp_palettes, 3);
    let gauge = image("gauge.png", &h.gauge_tiles, grid, &[h.gauge_palette], 1);
    let font = image("font.png", &h.font, GLYPHS(16), &[h.hp_palettes[0]], 0);
    let digits = concat(&h.enemy_digits.iter().collect::<Vec<_>>());
    let enemy_digits = image("enemy-digits.png", &digits, GLYPHS(10), &[h.enemy_palette], 1);
    // Icons: 4 tiles each.
    let icon = Layout::Blocks { width: 2, height: 2, columns: 1 };
    let chip_icons = h
        .chip_icons
        .iter()
        .map(|i| ChipIconDoc {
            chip: i.key.clone(),
            image: (!i.tiles.is_empty())
                .then(|| image(&format!("chip-icons/{}.png", i.key), &i.tiles, icon, &[h.icon_palette], 1)),
        })
        .collect();
    // Not in chip-icons/, where a chip may be named anything.
    let hidden_icon = image("hidden-icon.png", &h.hidden_icon, icon, &[h.icon_palette], 1);
    let mugshots = h
        .mugshots
        .iter()
        .enumerate()
        .map(|(i, (t, p))| {
            let file = format!("mugshots/{}.png", names.mugshot(i as u8));
            image(&file, t, Layout::Blocks { width: 4, height: 2, columns: 1 }, &[*p], 1)
        })
        .collect();
    let face = Layout::Blocks { width: 4, height: 2, columns: 1 };
    let navi_mugshots = h
        .navi_mugshots
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let file = format!("mugshots/{}.png", names.mugshot(nettai_assets::NAVI_MUGSHOTS + i as u8));
            image(&file, &m.tiles, face, &m.palettes, 2)
        })
        .collect();
    let navi0 = h.navi_mugshots.first().map(|m| m.palettes[0]).unwrap_or([0; 16]);
    let navi_box = image("navi-box.png", &h.navi_box, icon, &[navi0], 0);
    let pause = image("pause.png", &h.pause, GLYPHS(5), &[h.enemy_palette], 0);
    let mut counts: Vec<&Tiles> = h.counts.iter().collect();
    counts.push(&h.count_box);
    let mug0 = h.mugshots.first().map(|m| m.1).unwrap_or([0; 16]);
    let counts = image("counts.png", &concat(&counts), Layout::Blocks { width: 2, height: 2, columns: 12 }, &[mug0], 0);
    // Banner glyphs: up to 20 glyphs (40 tiles) a banner; text banners
    // have none.
    let banners = h
        .banners
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let file = format!("banners/{}.png", names.banner(4 * i as u8));
            let img = (!b.glyphs.is_empty()).then(|| image(&file, &b.glyphs, GLYPHS(20), &[h.banner_palette], 1));
            BannerDoc {
                at: [b.x, b.y],
                kind: b.kind,
                glyphs: b.glyphs.len() / 2,
                number_at: b.number_at.map(|(x, y)| [x, y]),
                image: img,
            }
        })
        .collect();
    let banner_digits = image("banner-digits.png", &h.banner_digits, GLYPHS(11), &[h.banner_palette], 0);
    let waiting = image("waiting.png", &h.waiting, Layout::Grid { columns: 8 }, &[h.waiting_palette], 1);
    let warning = (!h.warning.is_empty())
        .then(|| image("warning.png", &h.warning, Layout::Blocks { width: 2, height: 2, columns: 2 }, &[h.warning_palette], 1));
    let chatbox = (!h.chatbox.is_empty()).then(|| {
        let c = &h.chatbox;
        let rows = |m: &[MapEntry]| m.chunks(Chatbox::COLUMNS).map(|r| r.iter().map(tiles::entry_text).collect::<Vec<_>>().join(" ")).collect();
        ChatboxDoc {
            tiles: image("chatbox.png", &c.tiles, grid, &[c.palette], 1),
            boxes: c.boxes.iter().map(|steps| steps.iter().map(|m| rows(m)).collect()).collect(),
            arrow: image("chatbox-arrow.png", &c.arrow, Layout::Blocks { width: 2, height: 2, columns: 3 }, &[c.text_palette], 1),
        }
    });
    let mut dialogue_pngs = Vec::new();
    let mut languages = BTreeMap::new();
    for (lang, l) in &h.languages {
        let file = |name: &str| format!("{name}-{lang}.png");
        let font = image(&file("font"), &l.font, GLYPHS(16), &[h.hp_palettes[0]], 0);
        let banners = l
            .banners
            .iter()
            .enumerate()
            .map(|(i, b)| {
                b.as_ref().map(|b| BannerDoc {
                    at: [b.x, b.y],
                    kind: b.kind,
                    glyphs: b.glyphs.len() / 2,
                    number_at: b.number_at.map(|(x, y)| [x, y]),
                    image: (!b.glyphs.is_empty())
                        .then(|| image(&file(&format!("banners/{}", names.banner(4 * i as u8))), &b.glyphs, GLYPHS(20), &[l.banner_palette], 1)),
                })
            })
            .collect();
        let columns = (l.waiting.len() / 2).max(1) as u32;
        let waiting = image(&file("waiting"), &l.waiting, Layout::Grid { columns }, &[l.waiting_palette], 1);
        let gauge = image(&file("gauge"), &l.gauge_tiles, grid, &[h.gauge_palette], 0);
        let dialogue_file = file("dialogue-font");
        dialogue_pngs.push((dialogue_file.clone(), dialogue_image(&l.dialogue_font).to_png()));
        let dialogue_font = DialogueFontDoc {
            file: dialogue_file,
            cell: [DialogueFont::WIDTH, DialogueFont::HEIGHT],
            columns: DIALOGUE_COLUMNS,
            advances: l.dialogue_font.advances.clone(),
            chars: l.dialogue_font.chars.clone(),
        };
        languages.insert(
            lang.clone(),
            HudLanguageDoc { font, font_chars: l.font_chars.clone(), dialogue_font, texts: l.texts.clone(), banners, waiting, gauge },
        );
    }
    let dialogue_font = (!h.dialogue_font.is_empty()).then(|| {
        let file = "dialogue-font.png".to_string();
        files.push((file.clone(), dialogue_image(&h.dialogue_font).to_png()));
        DialogueFontDoc {
            file,
            cell: [DialogueFont::WIDTH, DialogueFont::HEIGHT],
            columns: DIALOGUE_COLUMNS,
            advances: h.dialogue_font.advances.clone(),
            chars: h.dialogue_font.chars.clone(),
        }
    });
    let texts = |m: &[MapEntry]| m.iter().map(tiles::entry_text).collect();
    let doc = HudDoc {
        format: FORMAT.into(),
        version: VERSION,
        layer,
        first_tile: h.first_tile,
        gauge,
        gauge_first_tile: h.gauge_first_tile,
        hp_box: texts(&h.hp_box),
        gauge_frame: texts(&h.gauge_frame),
        font,
        font_chars: h.font_chars.clone(),
        enemy_digits,
        chip_icons,
        hidden_icon,
        mugshots,
        counts,
        navi_mugshots,
        navi_box,
        pause,
        texts: h.texts.clone(),
        banners,
        banner_digits,
        waiting,
        warning,
        dialogue_font,
        chatbox,
        language: h.language.clone(),
        languages,
    };
    files.extend(dialogue_pngs);
    files.push(("hud.json".into(), json_lines(&doc)));
    files
}

pub fn import(dir: &Path, prefix: &str, report: &mut Report) -> Option<Hud> {
    let name = format!("{prefix}/hud.json");
    let doc: HudDoc = read_json(&dir.join("hud.json"), &name, report)?;
    if doc.format != FORMAT || doc.version != VERSION {
        report.error(&name, format!("not a {FORMAT} file of version {VERSION} (extract the pack again)"));
        return None;
    }
    let img = |d: &TileImage, report: &mut Report| tiles::import_image(dir, prefix, d, report);
    let (tiles, hp) = img(&doc.layer, report)?;
    let (gauge_tiles, gauge_pal) = img(&doc.gauge, report)?;
    let (font, _) = img(&doc.font, report)?;
    let (digits, enemy_pal) = img(&doc.enemy_digits, report)?;
    let (hidden_icon, icon_pal) = img(&doc.hidden_icon, report)?;
    let mut chip_icons = Vec::new();
    for icon in &doc.chip_icons {
        let tiles = match &icon.image {
            Some(i) => img(i, report)?.0,
            None => Tiles::default(),
        };
        chip_icons.push(ChipIcon { key: icon.chip.clone(), tiles });
    }
    let (counts, _) = img(&doc.counts, report)?;
    let mut banner_pal = vec![Palette::default()];
    let mut banner_glyphs = Vec::new();
    for b in &doc.banners {
        banner_glyphs.push(match &b.image {
            Some(i) => {
                let (t, p) = img(i, report)?;
                banner_pal = p;
                t
            }
            None => Tiles::default(),
        });
    }
    let (banner_digits, _) = img(&doc.banner_digits, report)?;
    let (waiting, waiting_pal) = img(&doc.waiting, report)?;
    let (warning, warning_pal) = match &doc.warning {
        Some(i) => img(i, report)?,
        None => (Tiles::default(), vec![Palette::default()]),
    };
    let mut mugshots = Vec::new();
    for m in &doc.mugshots {
        let (t, p) = img(m, report)?;
        mugshots.push((t, p[0]));
    }
    let mut navi_mugshots = Vec::new();
    for m in &doc.navi_mugshots {
        let (tiles, p) = img(m, report)?;
        let (Some(&normal), Some(&angry)) = (p.first(), p.get(1)) else {
            report.error(&name, format!("{} needs two palettes (normal, angry)", m.file));
            return None;
        };
        navi_mugshots.push(NaviMugshot { tiles, palettes: [normal, angry] });
    }
    let (navi_box, _) = img(&doc.navi_box, report)?;
    let (pause, _) = img(&doc.pause, report)?;
    let slice = |t: &Tiles, from: usize, n: usize| Tiles { pixels: t.pixels[from * Tiles::TILE..(from + n) * Tiles::TILE].to_vec() };
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
    let per_digit_set = digits.len() / 3;
    let banners = doc
        .banners
        .iter()
        .zip(banner_glyphs)
        .map(|(b, glyphs)| BannerLayout {
            x: b.at[0],
            y: b.at[1],
            kind: b.kind,
            glyphs: if glyphs.is_empty() { glyphs } else { slice(&glyphs, 0, 2 * b.glyphs) },
            number_at: b.number_at.map(|[x, y]| (x, y)),
        })
        .collect();
    let dialogue_font = match &doc.dialogue_font {
        Some(d) => import_dialogue_font(dir, prefix, d, report)?,
        None => DialogueFont::default(),
    };
    let chatbox = match &doc.chatbox {
        Some(c) => {
            let (tiles, palette) = img(&c.tiles, report)?;
            let (arrow, text_palette) = img(&c.arrow, report)?;
            let mut boxes = Vec::new();
            for steps in &c.boxes {
                let maps: Vec<Vec<MapEntry>> = steps
                    .iter()
                    .map(|rows| map(&rows.iter().flat_map(|r| r.split(' ').map(str::to_string)).collect::<Vec<_>>(), report))
                    .collect();
                let Ok(maps) = <[Vec<MapEntry>; 4]>::try_from(maps) else {
                    report.error(&name, "a chatbox box needs its four opening steps");
                    return None;
                };
                if maps.iter().any(|m| m.len() != Chatbox::COLUMNS * Chatbox::ROWS) {
                    report.error(&name, format!("a chatbox box's map is {}x{} entries", Chatbox::COLUMNS, Chatbox::ROWS));
                    return None;
                }
                boxes.push(maps);
            }
            Chatbox { tiles, palette: palette[0], boxes, arrow, text_palette: text_palette[0] }
        }
        None => Chatbox::default(),
    };
    Some(Hud {
        tiles,
        first_tile: doc.first_tile,
        gauge_tiles,
        gauge_first_tile: doc.gauge_first_tile,
        hp_palettes: [hp[0], hp[1], hp[2]],
        gauge_palette: gauge_pal[0],
        hp_box: map(&doc.hp_box, report),
        gauge_frame: map(&doc.gauge_frame, report),
        font,
        font_chars: doc.font_chars.clone(),
        enemy_digits: [0, 1, 2].map(|k| slice(&digits, k * per_digit_set, per_digit_set)),
        enemy_palette: enemy_pal[0],
        chip_icons,
        hidden_icon,
        icon_palette: icon_pal[0],
        mugshots,
        // (A pack without a count box, BN5's, has none of them.)
        counts: (0..(counts.len() / 4).saturating_sub(1)).map(|i| slice(&counts, 4 * i, 4)).collect(),
        count_box: if counts.len() >= 4 { slice(&counts, counts.len() - 4, 4) } else { Tiles::default() },
        navi_mugshots,
        navi_box,
        pause,
        texts: doc.texts.clone(),
        banners,
        banner_digits,
        banner_palette: banner_pal[0],
        waiting,
        waiting_palette: waiting_pal[0],
        warning,
        warning_palette: warning_pal[0],
        dialogue_font,
        chatbox,
        language: doc.language.clone(),
        languages: import_languages(dir, prefix, &doc, report)?,
    })
}

/// The other languages' lettering (`HudLanguageDoc`).
fn import_languages(dir: &Path, prefix: &str, doc: &HudDoc, report: &mut Report) -> Option<Vec<(String, HudLettering)>> {
    let mut out = Vec::new();
    for (lang, d) in &doc.languages {
        let img = |d: &TileImage, report: &mut Report| tiles::import_image(dir, prefix, d, report);
        if d.banners.len() != doc.banners.len() {
            report.error(format!("{prefix}/hud.json"), format!("{lang}'s banners are {}, the pack's {}", d.banners.len(), doc.banners.len()));
            return None;
        }
        let (waiting, waiting_palette) = img(&d.waiting, report)?;
        let mut banner_palette = None;
        let mut banners = Vec::new();
        for b in &d.banners {
            banners.push(match b {
                Some(b) => {
                    let glyphs = match &b.image {
                        Some(i) => {
                            let (t, p) = img(i, report)?;
                            banner_palette.get_or_insert(p[0]);
                            Tiles { pixels: t.pixels[..(2 * b.glyphs * Tiles::TILE).min(t.pixels.len())].to_vec() }
                        }
                        None => Tiles::default(),
                    };
                    Some(BannerLayout { x: b.at[0], y: b.at[1], kind: b.kind, glyphs, number_at: b.number_at.map(|[x, y]| (x, y)) })
                }
                None => None,
            });
        }
        out.push((
            lang.clone(),
            HudLettering {
                font: img(&d.font, report)?.0,
                font_chars: d.font_chars.clone(),
                dialogue_font: import_dialogue_font(dir, prefix, &d.dialogue_font, report)?,
                texts: d.texts.clone(),
                banners,
                // (The banners' palette is "Cstmzing..."'s too.)
                banner_palette: banner_palette.unwrap_or(waiting_palette[0]),
                waiting,
                waiting_palette: waiting_palette[0],
                gauge_tiles: img(&d.gauge, report)?.0,
            },
        ));
    }
    Some(out)
}

/// The dialogue font from its image (`DialogueFontDoc`).
fn import_dialogue_font(dir: &Path, prefix: &str, d: &DialogueFontDoc, report: &mut Report) -> Option<DialogueFont> {
    let name = format!("{prefix}/{}", d.file);
    let img = match crate::image::Indexed::load(&dir.join(&d.file)) {
        Ok(i) => i,
        Err(e) => {
            report.error(&name, e);
            return None;
        }
    };
    let (w, h) = (DialogueFont::WIDTH, DialogueFont::HEIGHT);
    if d.cell != [w, h] || d.columns == 0 || d.chars.len() != d.advances.len() {
        report.error(&name, format!("a dialogue font of {w}x{h} cells, with a character and an advance for each glyph"));
        return None;
    }
    let mut pixels = Vec::with_capacity(w * h * d.advances.len());
    for g in 0..d.advances.len() {
        let (gx, gy) = ((g % d.columns) * w, (g / d.columns) * h);
        if (gx + w) as u32 > img.width || (gy + h) as u32 > img.height {
            report.error(&name, format!("too small for {} glyphs", d.advances.len()));
            return None;
        }
        for k in 0..w * h {
            pixels.push(img.get((gx + k % w) as u32, (gy + k / w) as u32));
        }
    }
    Some(DialogueFont { pixels, advances: d.advances.clone(), chars: d.chars.clone() })
}
