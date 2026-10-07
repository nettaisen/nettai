//! Survey a content pack's graphics: how sprites use tilesets, parts and
//! palettes (to choose an exact editable format).
//!
//!     cargo run -p nettai-content --example stats -- data/exe6

use nettai_assets::Bundle;
use std::collections::{BTreeMap, BTreeSet, HashMap};

fn main() {
    let path = std::env::args().nth(1).expect("usage: stats <pack>");
    let (b, _) = nettai_content::pack::load_graphics(std::path::Path::new(&path)).unwrap_or_else(|r| panic!("{r}"));
    let mut n = BTreeMap::<&str, usize>::new();
    let mut flags = BTreeMap::<u8, usize>::new();
    let mut durations = BTreeMap::<u8, usize>::new();
    let mut palset_sizes = BTreeMap::<usize, usize>::new();
    let mut sizes = BTreeMap::<(u8, u8), usize>::new();
    for s in &b.sprites {
        *n.entry("sprites").or_default() += 1;
        *n.entry("tilesets").or_default() += s.tilesets.len();
        *n.entry("palette_sets").or_default() += s.palette_sets.len();
        *n.entry("part_lists").or_default() += s.part_lists.len();
        *n.entry("animations").or_default() += s.animations.len();
        for ps in &s.palette_sets {
            *palset_sizes.entry(ps.len()).or_default() += 1;
            for p in ps {
                for &c in p {
                    if c & 0x8000 != 0 {
                        *n.entry("colors with bit 15").or_default() += 1;
                    }
                }
            }
        }
        // (tileset, parts) combinations and the palette sets they use.
        let mut combos: BTreeSet<(u16, u16)> = BTreeSet::new();
        let mut ts_lists: HashMap<u16, BTreeSet<u16>> = HashMap::new();
        let mut list_ts: HashMap<u16, BTreeSet<u16>> = HashMap::new();
        for a in &s.animations {
            for f in a {
                *n.entry("frames").or_default() += 1;
                *flags.entry(f.flags).or_default() += 1;
                *durations.entry(f.duration).or_default() += 1;
                combos.insert((f.tileset, f.parts));
                ts_lists.entry(f.tileset).or_default().insert(f.parts);
                list_ts.entry(f.parts).or_default().insert(f.tileset);
            }
        }
        *n.entry("combos").or_default() += combos.len();
        *n.entry("tilesets used with >1 part list").or_default() += ts_lists.values().filter(|v| v.len() > 1).count();
        *n.entry("part lists used with >1 tileset").or_default() += list_ts.values().filter(|v| v.len() > 1).count();
        if s.tilesets.len() > ts_lists.len() {
            *n.entry("tilesets never used").or_default() += s.tilesets.len() - ts_lists.len();
        }
        for (ti, t) in s.tilesets.iter().enumerate() {
            let Some(lists) = ts_lists.get(&(ti as u16)) else { continue };
            let mut covered = vec![0u32; t.len()];
            for &l in lists {
                for p in &s.part_lists[l as usize] {
                    let k = p.width as usize / 8 * p.height as usize / 8;
                    for i in p.tile as usize..p.tile as usize + k {
                        if i < covered.len() {
                            covered[i] += 1;
                        } else {
                            *n.entry("part tiles past the tileset").or_default() += 1;
                        }
                    }
                }
            }
            let unc = covered.iter().filter(|&&c| c == 0).count();
            if unc > 0 {
                *n.entry("tilesets with uncovered tiles").or_default() += 1;
                *n.entry("uncovered tiles").or_default() += unc;
                let blank = covered
                    .iter()
                    .enumerate()
                    .filter(|&(i, &c)| c == 0 && t.get(i).unwrap().iter().any(|&x| x != 0))
                    .count();
                *n.entry("uncovered non-blank tiles").or_default() += blank;
            }
            *n.entry("tiles").or_default() += t.len();
        }
        for (li, l) in s.part_lists.iter().enumerate() {
            *n.entry("parts").or_default() += l.len();
            if l.is_empty() {
                *n.entry("empty part lists").or_default() += 1;
            }
            let pals: BTreeSet<u8> = l.iter().map(|p| p.palette).collect();
            if pals.len() > 1 {
                *n.entry("part lists with mixed palette offsets").or_default() += 1;
            }
            if l.iter().any(|p| p.palette != 0) {
                *n.entry("part lists with a non-zero palette offset").or_default() += 1;
            }
            for p in l {
                *sizes.entry((p.width, p.height)).or_default() += 1;
                if p.hflip || p.vflip {
                    *n.entry("flipped parts").or_default() += 1;
                }
            }
            // Parts sharing tiles.
            let mut seen = HashMap::new();
            let mut shared = false;
            for (pi, p) in l.iter().enumerate() {
                let k = p.width as usize / 8 * p.height as usize / 8;
                for i in p.tile as usize..p.tile as usize + k {
                    if seen.insert(i, pi).is_some() {
                        shared = true;
                    }
                }
            }
            if shared {
                *n.entry("part lists reusing a tile").or_default() += 1;
            }
            // Overlapping rectangles, and whether opaque pixels conflict
            // when drawn with each tileset they are used with.
            let mut overlap = false;
            for i in 0..l.len() {
                for j in i + 1..l.len() {
                    let (a, c) = (&l[i], &l[j]);
                    let ox = (a.x as i32) < c.x as i32 + c.width as i32 && (c.x as i32) < a.x as i32 + a.width as i32;
                    let oy = (a.y as i32) < c.y as i32 + c.height as i32 && (c.y as i32) < a.y as i32 + a.height as i32;
                    if ox && oy {
                        overlap = true;
                        if i == 0 {
                            *n.entry("overlaps involving the shadow part").or_default() += 1;
                        } else {
                            *n.entry("overlaps between body parts").or_default() += 1;
                        }
                    }
                }
            }
            if overlap {
                *n.entry("part lists with overlapping parts").or_default() += 1;
                for &t in list_ts.get(&(li as u16)).into_iter().flatten() {
                    let tiles = &s.tilesets[t as usize];
                    let mut canvas: HashMap<(i32, i32), (usize, u8)> = HashMap::new();
                    let mut conflict = false;
                    let mut hidden = false;
                    for (pi, p) in l.iter().enumerate() {
                        let tpr = p.width as usize / 8;
                        for y in 0..p.height as i32 {
                            for x in 0..p.width as i32 {
                                let tx = if p.hflip { p.width as i32 - 1 - x } else { x } as usize;
                                let ty = if p.vflip { p.height as i32 - 1 - y } else { y } as usize;
                                let ti = p.tile as usize + (ty / 8) * tpr + tx / 8;
                                let v = tiles.get(ti).map(|t| t[(ty % 8) * 8 + tx % 8]).unwrap_or(0);
                                let at = (p.x as i32 + x, p.y as i32 + y);
                                match canvas.get(&at) {
                                    Some(&(_, old)) if old != 0 && v != 0 && old != v => conflict = true,
                                    Some(&(_, old)) if (old == 0) != (v == 0) => hidden = true,
                                    _ => {}
                                }
                                if v != 0 || !canvas.contains_key(&at) {
                                    canvas.insert(at, (pi, v));
                                }
                            }
                        }
                    }
                    if conflict {
                        *n.entry("(tileset, overlapping list) with conflicting opaque pixels").or_default() += 1;
                    }
                    if hidden {
                        *n.entry("(tileset, overlapping list) with opaque over transparent").or_default() += 1;
                    }
                }
            }
        }
    }
    for (k, v) in &n {
        println!("{k:>60}: {v}");
    }
    println!("flags: {flags:?}");
    println!("durations: {durations:?}");
    println!("palette set sizes: {palset_sizes:?}");
    println!("part sizes: {sizes:?}");
    tiles_survey(&b);
    region_survey(&b);
}


pub fn tiles_survey(b: &Bundle) {
    use nettai_assets::AnimTarget;
    let bit15 = |p: &[u16; 16]| p.iter().filter(|&&c| c & 0x8000 != 0).count();
    let f = &b.field;
    println!(
        "field: {} tiles from {}, {} palettes (bit15 {}), {} palette anims, {} panels",
        f.tiles.len(),
        f.first_tile,
        f.palettes.len(),
        f.palettes.iter().map(bit15).sum::<usize>(),
        f.palette_anims.len(),
        f.panels.len()
    );
    for a in &f.palette_anims {
        println!(
            "  palette anim slot {} frames {} timer {} bit15 {} delays {:?}",
            a.slot,
            a.frames.len(),
            a.initial_timer,
            a.frames.iter().map(|x| bit15(&x.0)).sum::<usize>(),
            a.frames.iter().map(|x| x.1).collect::<Vec<_>>()
        );
    }
    let pals: BTreeSet<u8> = f
        .panels
        .iter()
        .flatten()
        .chain(f.front_edges.iter().flatten())
        .chain(f.highlights.iter().flatten())
        .map(|e| e.palette)
        .collect();
    let tiles: BTreeSet<u16> = f.panels.iter().flatten().map(|e| e.tile).collect();
    println!("  panel palettes {pals:?}, tiles {}..={}", tiles.first().unwrap(), tiles.last().unwrap());
    for (i, bg) in b.backgrounds.iter().enumerate() {
        let Some(bg) = bg else {
            println!("bg {i}: none");
            continue;
        };
        let pals: BTreeSet<u8> = bg.map.iter().map(|e| e.palette).collect();
        let tl: BTreeSet<u16> = bg.map.iter().map(|e| e.tile).collect();
        let flips = bg.map.iter().filter(|e| e.hflip || e.vflip).count();
        println!(
            "bg {i}: {} tiles from {}, map {}x{} pals {pals:?} tiles {}..={} flips {flips}, palette {} (bit15 {}), scroll {:?}",
            bg.tiles.len(),
            bg.first_tile,
            bg.map_width,
            bg.map_height,
            tl.first().unwrap(),
            tl.last().unwrap(),
            bg.palette.is_some(),
            bg.palette.as_ref().map(bit15).unwrap_or(0),
            bg.scroll
        );
        for a in &bg.anims {
            let t = match a.target {
                AnimTarget::Tiles { first, count } => format!("tiles {first}+{count}"),
                AnimTarget::Palettes { first, count } => format!("palettes {first}+{count}"),
                AnimTarget::PaletteShift { first, count, darken } => {
                    format!("{} {first}+{count}", if darken { "darkens" } else { "brightens" })
                }
                AnimTarget::Nothing => "nothing".into(),
            };
            let delays: Vec<u16> = a.frames.iter().map(|f| f.delay).collect();
            let b15: usize = a.frames.iter().flat_map(|f| &f.palettes).map(bit15).sum();
            println!("   anim {t} frames {} repeat {:?} delays {delays:?} bit15 {b15}", a.frames.len(), a.repeat_from);
        }
    }
    let h = &b.hud;
    let pals = [
        h.hp_palettes.to_vec(),
        vec![h.gauge_palette, h.enemy_palette, h.icon_palette, h.banner_palette, h.waiting_palette],
        h.mugshots.iter().map(|m| m.1).collect(),
    ]
    .concat();
    println!("hud palettes bit15 {}", pals.iter().map(bit15).sum::<usize>());
    println!(
        "hud: tiles {} gauge {} font {} icons {} (empty {}) mugshots {} counts {} banners {} font chars {}",
        h.tiles.len(),
        h.gauge_tiles.len(),
        h.font.len(),
        h.chip_icons.len(),
        h.chip_icons.iter().filter(|i| i.tiles.is_empty()).count(),
        h.mugshots.len(),
        h.counts.len(),
        h.banners.len(),
        h.font_chars.len()
    );
    let hp: BTreeSet<u8> = h.hp_box.iter().chain(&h.gauge_frame).map(|e| e.palette).collect();
    println!("hud map palettes {hp:?}");
    for (i, bn) in h.banners.iter().enumerate() {
        println!("  banner {i}: at {},{} kind {} glyph tiles {} number {:?}", bn.x, bn.y, bn.kind, bn.glyphs.len(), bn.number_at);
    }
}

/// Part regions (tileset, tile, w, h): how many, and how many overlap
/// others of a different shape in tile space.
pub fn region_survey(b: &Bundle) {
    let mut regions = 0;
    let mut odd = 0;
    let mut pal_rows = 0;
    let mut used_palsets = BTreeSet::new();
    for s in &b.sprites {
        let mut per_ts: HashMap<u16, BTreeSet<(u16, u8, u8)>> = HashMap::new();
        let mut rows: BTreeSet<(u16, u16, u8, u8, u8)> = BTreeSet::new();
        for a in &s.animations {
            for f in a {
                used_palsets.insert(f.palette_set);
                for p in &s.part_lists[f.parts as usize] {
                    per_ts.entry(f.tileset).or_default().insert((p.tile, p.width, p.height));
                    rows.insert((f.tileset, p.tile, p.width, p.height, p.palette));
                }
            }
        }
        for set in per_ts.values() {
            regions += set.len();
            let v: Vec<_> = set.iter().collect();
            for i in 0..v.len() {
                let (t, w, h) = *v[i];
                let a = t as usize..t as usize + (w as usize / 8) * (h as usize / 8);
                let clash = v.iter().enumerate().any(|(j, &&(t2, w2, h2))| {
                    j != i && (w2, h2) != (w, h) && {
                        let b2 = t2 as usize..t2 as usize + (w2 as usize / 8) * (h2 as usize / 8);
                        a.start < b2.end && b2.start < a.end
                    }
                });
                if clash {
                    odd += 1;
                }
            }
        }
        pal_rows += rows.len();
    }
    println!(
        "regions {regions}, overlapping another shape {odd}, (region,palette) {pal_rows}, palette sets used {used_palsets:?}"
    );
}
