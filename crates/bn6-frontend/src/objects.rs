//! Battle objects as sprites: the original's projection of 16.16 field
//! positions to the screen, its culling, and the order it hands sprite
//! parts to the hardware in.
//!
//! The game draws objects pool by pool (actors, then attacks, then
//! effects), each pool in update order (`sub_8003E18`, `sub_8004218`,
//! `sub_8004510` walk the lists `RunBattleObjectLogic` builds). Each part
//! goes into a depth bucket of a layer (`sub_3006440`, `sub_3006920`), and
//! the hardware list is read layer by layer, deepest bucket first, most
//! recent part first (`copyTo_iObjectAttr3001D70_3006814`): a part drawn
//! later, or lower on the field, ends up in front.

use crate::audit::Problems;
use crate::compose::SpritePart;
use bn6_assets::{Bundle, Palette};
use bn6_battle::Battle;
use bn6_battle::object::sprite::Shadow;
use bn6_battle::kinds::EngineKind;
use bn6_battle::object::{Object, Pool, flags};

/// Where the camera looks and whether the field is shown mirrored (the
/// right-hand player sees their side on the left).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct View {
    /// 16.16 camera offset (shakes move it).
    pub camera: (i32, i32, i32),
    pub mirror: bool,
}

/// The hardware sprite limit.
pub const MAX_PARTS: usize = 128;

/// Bucket counts per layer (`byte_802FD90`, the battle's sprite layers).
const BUCKETS: [usize; 4] = [8, 4, 0xE0, 4];

/// Where parts are ordered: four layers of depth buckets.
pub struct SpriteList<'a> {
    layers: [Vec<Vec<SpritePart<'a>>>; 4],
    count: usize,
}

impl Default for SpriteList<'_> {
    fn default() -> Self {
        SpriteList { layers: BUCKETS.map(|n| vec![Vec::new(); n]), count: 0 }
    }
}

impl<'a> SpriteList<'a> {
    /// Queue one sprite's parts as (layer, bucket, part). The whole group
    /// is dropped when it would pass the hardware limit; a part with a
    /// bucket out of range drops it and the rest of the group
    /// (`sub_3006920`).
    pub fn insert_group(&mut self, group: Vec<(usize, i32, SpritePart<'a>)>) {
        if self.count + group.len() >= MAX_PARTS {
            return;
        }
        for (layer, bucket, part) in group {
            if bucket < 0 || bucket as usize >= self.layers[layer].len() {
                return;
            }
            self.layers[layer][bucket as usize].push(part);
            self.count += 1;
        }
    }

    /// Queue parts at a layer and depth bucket as the HUD's direct
    /// inserts do (`sub_30068E8`), in front of what the bucket holds; the
    /// first part ends up frontmost.
    pub fn insert_at(&mut self, layer: usize, bucket: usize, group: Vec<SpritePart<'a>>) {
        for part in group.into_iter().rev() {
            if self.count >= MAX_PARTS {
                return;
            }
            let Some(b) = self.layers[layer].get_mut(bucket) else { return };
            b.push(part);
            self.count += 1;
        }
    }

    /// Parts in hardware order (front first).
    pub fn into_parts(self) -> Vec<SpritePart<'a>> {
        let mut out = Vec::with_capacity(self.count);
        for layer in self.layers {
            for bucket in layer.into_iter().rev() {
                out.extend(bucket.into_iter().rev());
            }
        }
        out
    }
}

const WHITE: Palette = [0x7FFF; 16];

/// A position projected to the screen (`sub_300638C`, the battle
/// projection): x and y are the field plane, z is height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Projected {
    /// Screen x of the object's anchor.
    pub x: i32,
    /// Screen y of the ground under the object (also its depth).
    pub ground: i32,
    /// Screen y of the object at its height.
    pub y: i32,
}

pub fn project(pos: (i32, i32, i32), view: &View) -> Projected {
    let int = |v: i32| (v >> 16) as i16 as i32;
    let (cx, cy, cz) = (view.camera.0 >> 16, view.camera.1 >> 16, view.camera.2 >> 16);
    let mut x = int(pos.0) - cx;
    if view.mirror {
        x = -x;
    }
    let x = (x + 0x78) as i16 as i32;
    let ground = (int(pos.1) - cy + 0x50) as i16 as i32;
    let y = ground - (int(pos.2) - cz);
    Projected { x, ground, y }
}

/// A colour shader (`sprite_setColorShader`, applied by `sub_3005EF0`):
/// bit 15 clear adds the colour to every palette entry, set subtracts it,
/// per channel and saturating.
pub fn shade(mut p: Palette, shader: u16) -> Palette {
    if shader == 0 {
        return p;
    }
    let ch = |c: u16, s: u32| ((c >> s) & 31) as i32;
    for c in p.iter_mut() {
        let mut out = 0u16;
        for s in [0, 5, 10] {
            let v = if shader & 0x8000 == 0 { ch(*c, s) + ch(shader, s) } else { ch(*c, s) - ch(shader, s) };
            out |= (v.clamp(0, 31) as u16) << s;
        }
        *c = out;
    }
    p
}

/// The palette flash effect (`sub_80E10C0`) turns the tile layers white on
/// the frames its counter has bit 2 clear (measured: the sprites keep their
/// colours).
pub fn palette_flash(b: &Battle) -> bool {
    b.objects.in_order().any(|r| {
        let o = b.objects.get(r);
        is(b, o, EngineKind::PaletteFlash) && o.state != 0 && o.timer & 4 == 0
    })
}

/// Whether `o` is of the engine's kind `kind`.
fn is(b: &Battle, o: &Object, kind: EngineKind) -> bool {
    b.content.defs.engine_kind(o.kind) == Some(kind)
}

/// A sprite as content names it (the pack's asset index), for a problem's
/// text.
pub fn sprite_name(b: &Battle, id: bn6_battle::content::SpriteId) -> String {
    match b.content.assets.sprites.iter().find(|(_, s)| **s == id) {
        Some((name, _)) => format!("sprite {name:?}"),
        None => format!("sprite {:02x}-{:02x}", id.category, id.index),
    }
}

/// Every object as the renderer sees it, a line each: its kind, where it
/// is, its sprite with the animation and frame, and its look (what
/// `--objects` prints: the first thing to read when something isn't drawn
/// or is drawn wrong).
pub fn describe(b: &Battle, view: &View) -> Vec<String> {
    let mut lines = Vec::new();
    for pool in Pool::ALL {
        for r in b.objects.in_order().filter(|r| r.pool == pool) {
            let o = b.objects.get(r);
            let s = b.objects.sprite(r);
            let p = project((o.pos.x, o.pos.y, o.pos.z), view);
            let sprite = match s.id {
                Some(id) => format!("{} anim {} frame {}", sprite_name(b, id), s.anim, s.frame),
                None => "no sprite".to_string(),
            };
            let mut notes = Vec::new();
            if o.flags & flags::VISIBLE == 0 {
                notes.push("not visible".to_string());
            }
            if o.flags & flags::NO_SPRITE_UPDATE != 0 {
                notes.push("sprite held (not drawn)".to_string());
            }
            let l = s.look;
            notes.push(format!("palette {} shadow {:?} priority {}", l.palette, l.shadow, l.priority));
            for (on, what) in [(l.hflip, "hflip"), (l.vflip, "vflip"), (l.white, "white")] {
                if on {
                    notes.push(what.to_string());
                }
            }
            if l.color_shader != 0 {
                notes.push(format!("shader {:#06x}", l.color_shader));
            }
            if let Some(a) = l.alpha {
                notes.push(format!("alpha {a}"));
            }
            if let Some(m) = l.mosaic {
                notes.push(format!("mosaic {m}"));
            }
            if l.hidden_parts != 0 {
                notes.push(format!("hidden parts {:#010x}", l.hidden_parts));
            }
            lines.push(format!(
                "{:?} {:2} {} side {} at ({}, {}) ground {}: {sprite}; {}",
                r.pool,
                r.slot,
                b.content.defs.kind(o.kind).key,
                o.alliance,
                p.x,
                p.y,
                p.ground,
                notes.join(", ")
            ));
        }
    }
    lines
}

/// Queue every visible object's sprite. What an object names that the
/// pack's graphics don't have goes to `problems`.
pub fn queue_objects<'a>(b: &Battle, assets: &'a Bundle, view: &View, list: &mut SpriteList<'a>, problems: &mut Problems) {
    for pool in Pool::ALL {
        for r in b.objects.in_order().filter(|r| r.pool == pool) {
            let o = b.objects.get(r);
            // Objects whose sprite doesn't animate are culled to nothing
            // (`sub_30061E8` gives them a one-point mask).
            if o.flags & flags::VISIBLE == 0 || o.flags & flags::NO_SPRITE_UPDATE != 0 {
                continue;
            }
            let s = b.objects.sprite(r);
            let Some(id) = s.id else { continue };
            let kind = || &b.content.defs.kind(o.kind).key;
            let Some(sheet) = assets.sprite(id.category, id.index) else {
                problems.note(format!("{} of kind {:?} is not in the pack's graphics", sprite_name(b, id), kind()));
                continue;
            };
            let Some(frames) = sheet.animations.get(s.anim as usize) else {
                problems.note(format!(
                    "{} has no animation {} (kind {:?}; it has {})",
                    sprite_name(b, id),
                    s.anim,
                    kind(),
                    sheet.animations.len()
                ));
                continue;
            };
            let Some(frame) = frames.get(s.frame as usize).or(frames.last()) else {
                problems.note(format!("{} animation {} has no frames (kind {:?})", sprite_name(b, id), s.anim, kind()));
                continue;
            };
            let parts = &sheet.part_lists[frame.parts as usize];
            let tiles = &sheet.tilesets[frame.tileset as usize];
            let look = s.look;
            let p = project((o.pos.x, o.pos.y, o.pos.z), view);
            let hflip = look.hflip ^ view.mirror;
            let vflip = look.vflip;

            // Culling (`sub_30061E8`): a part is kept if its anchor is near
            // the screen. The first part is tested at the ground when the
            // sprite has a ground shadow.
            let mut mask = 0u32;
            let mut basis = if look.shadow == Shadow::Ground { p.ground } else { p.y };
            let m = view.mirror as i32;
            for (i, part) in parts.iter().take(32).enumerate() {
                let px = p.x + if hflip { -(part.x as i32) } else { part.x as i32 };
                let py = basis + if vflip { -(part.y as i32) } else { part.y as i32 };
                if (m - 0x40..m + 0x131).contains(&px) && (-0x20..0xC1).contains(&py) {
                    mask |= 0x8000_0000 >> i;
                }
                basis = p.y;
            }
            if mask == 0 {
                continue;
            }
            mask &= !look.hidden_parts;

            let first_palette = parts.first().map(|p| p.palette).unwrap_or(0);
            // A form overlay shows white with its owner (measured; the
            // overlay itself doesn't run while the battle is paused).
            let form_overlay = is(b, o, EngineKind::FormOverlay);
            let owner_white = || form_overlay && o.related[0].is_some_and(|w| b.objects.sprite(w).look.white);
            let palette = if look.white && !form_overlay || owner_white() {
                WHITE
            } else {
                let set = &sheet.palette_sets[frame.palette_set as usize];
                let index = look.palette.wrapping_add(first_palette) as usize;
                let p = set.get(index).copied().unwrap_or_else(|| {
                    problems.note(format!(
                        "{} has no palette {index} (kind {:?} asks for {} on animation {}; the set has {})",
                        sprite_name(b, id),
                        kind(),
                        look.palette,
                        s.anim,
                        set.len()
                    ));
                    [0; 16]
                });
                shade(p, look.color_shader)
            };

            let mut group = Vec::new();
            for (i, part) in parts.iter().take(32).enumerate() {
                let shadow = i == 0;
                if shadow && look.shadow == Shadow::Hidden {
                    continue;
                }
                if mask & (0x8000_0000 >> i) == 0 {
                    continue;
                }
                let on_ground = shadow && look.shadow == Shadow::Ground;
                let (w, h) = (part.width as i32, part.height as i32);
                let dx = if hflip { -(part.x as i32) - w } else { part.x as i32 };
                let dy = if vflip { -(part.y as i32) - h } else { part.y as i32 };
                let base_y = if on_ground { p.ground } else { p.y };
                let sprite = SpritePart {
                    x: ((p.x + dx) & 0x1FF) as u16,
                    y: ((base_y & 0xFF) + dy) as u8,
                    width: part.width,
                    height: part.height,
                    tiles,
                    first_tile: part.tile as usize,
                    hflip: part.hflip ^ hflip,
                    vflip: part.vflip ^ vflip,
                    palette,
                    priority: look.priority,
                    alpha: look.alpha,
                    mosaic: look.mosaic,
                    vscale: None,
                };
                // Ground shadows go one layer back, in the first bucket.
                let (layer, bucket) = if on_ground { (3, 0) } else { (2, p.ground + 0x40) };
                group.push((layer, bucket, sprite));
            }
            list.insert_group(group);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_matches_the_game() {
        // A navi on panel (2, 2): X = 2*40 - 140, Y = 2*24 - 20.
        let v = View::default();
        let p = project(((-60) << 16, 28 << 16, 0), &v);
        assert_eq!(p, Projected { x: 60, ground: 108, y: 108 });
        // Height lifts the sprite, not its ground.
        let p = project(((-60) << 16, 28 << 16, 16 << 16), &v);
        assert_eq!((p.ground, p.y), (108, 92));
        // Mirrored for the right-hand player.
        let p = project(((-60) << 16, 28 << 16, 0), &View { mirror: true, ..v });
        assert_eq!(p.x, 180);
    }

    #[test]
    fn deeper_and_later_parts_come_first() {
        let tiles = bn6_assets::Tiles::default();
        let part = |x: u16| SpritePart {
            x,
            y: 0,
            width: 8,
            height: 8,
            tiles: &tiles,
            first_tile: 0,
            hflip: false,
            vflip: false,
            palette: [0; 16],
            priority: 2,
            alpha: None,
            mosaic: None,
            vscale: None,
        };
        let mut list = SpriteList::default();
        list.insert_group(vec![(2, 100, part(1)), (2, 100, part(2))]);
        list.insert_group(vec![(2, 150, part(3)), (3, 0, part(4))]);
        list.insert_group(vec![(2, 100, part(5)), (2, 999, part(6)), (2, 100, part(7))]);
        // The HUD's own: a banner in the front layer, an icon among the
        // field's sprites by its bucket.
        list.insert_at(0, 0, vec![part(8), part(9)]);
        list.insert_at(2, 120, vec![part(10)]);
        let order: Vec<u16> = list.into_parts().iter().map(|p| p.x).collect();
        assert_eq!(order, vec![8, 9, 3, 10, 5, 2, 1, 4]);
    }
}
