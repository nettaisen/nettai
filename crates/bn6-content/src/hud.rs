//! The HUD's graphics: one indexed PNG per tile block, laid out as the
//! game draws it (8x16 glyphs, 2x2 icons, 4x2 mugshots), and `hud.json`
//! (map entries, chip names in the game's text codes, banner layouts).
//! Mugshots, banners and chip icons are a file each, under their names
//! (`mugshots/<name>.png`, `banners/<name>.png`, `chip-icons/<chip>.png`);
//! `hud.json` lists them in the game's order.
//!
//! Each palette belongs to one image, as rows of that image's palette
//! (`palettes` in its entry). Images drawn with another image's palette
//! show it for viewing only.

use crate::report::Report;
use crate::sprite::read_json;
use crate::stage::json_lines;
use crate::tiles::{self, Layout, TileImage};
use bn6_assets::{BannerLayout, Hud, MapEntry, Palette, Tiles};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const FORMAT: &str = "bn6-content/hud";
pub const VERSION: u32 = 2;

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
    /// Chip names in the game's text codes, by chip id.
    pub chip_names: Vec<Vec<u8>>,
    /// Chips whose damage shows after the name.
    pub chip_shows_damage: Vec<u16>,
    /// The opponent's HP digits: normal, dropping, rising (a row each),
    /// with their palette.
    pub enemy_digits: TileImage,
    /// Chip icons by chip id (none for a chip without one), each with the
    /// icon palette.
    pub chip_icons: Vec<Option<TileImage>>,
    /// The icon a hidden chip shows.
    pub hidden_icon: TileImage,
    /// Mugshots in the mugshot table's order, each with its palette.
    pub mugshots: Vec<TileImage>,
    /// The count box showing 0..=10, then without a number.
    pub counts: TileImage,
    pub form_emotions: Vec<u8>,
    /// Banners by banner id / 4.
    pub banners: Vec<BannerDoc>,
    pub banner_digits: TileImage,
    /// "Cstmzing..." and its palette.
    pub waiting: TileImage,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BannerDoc {
    pub at: [u8; 2],
    pub kind: u8,
    /// Glyphs it has (20, or 0 for banners drawn from text at run time).
    pub glyphs: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_at: Option<[u8; 2]>,
    /// Its glyphs, with the banner palette (none for banners drawn from
    /// text at run time).
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
        .enumerate()
        .map(|(id, t)| {
            (!t.is_empty()).then(|| image(&format!("chip-icons/{}.png", names.chip_icon(id as u16)), t, icon, &[h.icon_palette], 1))
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
        chip_names: h.chip_names.clone(),
        chip_shows_damage: h.chip_shows_damage.iter().enumerate().filter(|(_, s)| **s).map(|(i, _)| i as u16).collect(),
        enemy_digits,
        chip_icons,
        hidden_icon,
        mugshots,
        counts,
        form_emotions: h.form_emotions.clone(),
        banners,
        banner_digits,
        waiting,
    };
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
        chip_icons.push(match icon {
            Some(i) => img(i, report)?.0,
            None => Tiles::default(),
        });
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
    let mut mugshots = Vec::new();
    for m in &doc.mugshots {
        let (t, p) = img(m, report)?;
        mugshots.push((t, p[0]));
    }
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
    let chips = doc.chip_names.len();
    let per_digit_set = digits.len() / 3;
    let mut shows = vec![false; chips];
    for &c in &doc.chip_shows_damage {
        match shows.get_mut(c as usize) {
            Some(s) => *s = true,
            None => report.error(&name, format!("chip_shows_damage names chip {c}, past the {chips} chips")),
        }
    }
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
        chip_names: doc.chip_names.clone(),
        chip_shows_damage: shows,
        enemy_digits: [0, 1, 2].map(|k| slice(&digits, k * per_digit_set, per_digit_set)),
        enemy_palette: enemy_pal[0],
        chip_icons,
        hidden_icon,
        icon_palette: icon_pal[0],
        mugshots,
        counts: (0..counts.len() / 4 - 1).map(|i| slice(&counts, 4 * i, 4)).collect(),
        count_box: slice(&counts, counts.len() - 4, 4),
        form_emotions: doc.form_emotions.clone(),
        banners,
        banner_digits,
        banner_palette: banner_pal[0],
        waiting,
        waiting_palette: waiting_pal[0],
    })
}
