//! The chatbox the custom screen runs (a chip's or a Cross's description,
//! the no-running message), as the original's chatbox draws it each tick
//! (`chatbox_interpreteAndDrawDialogChar`): the box on BG0, the text as
//! three rows of sprites, the speaker's portrait and the key-wait arrow.
//!
//! How far the chatbox has got is the engine's (`custom::chatbox`: the
//! box's opening step, the characters printed, the portrait's face and
//! tint, the arrow); the words are the content's (a chip's or a Cross's
//! `description`, a navi's `run_message`), drawn in the dialogue font.
//! docs/frontend.md §3.

use crate::audit::Problems;
use crate::compose::{Layer, SpritePart};
use crate::fonts;
use crate::objects::SpriteList;
use nettai_assets::{Bundle, Chatbox as Graphics, Palette, SpriteSheet, Tiles};
use nettai_battle::Battle;
use nettai_battle::custom::chatbox::{Chatbox, PortraitLook, Script};
use nettai_battle::custom::{GameVersion, Library, Phase};

/// The box's map on BG0 (`CurTileYBlockPos`): from row 12, 30 columns of
/// 8 rows.
const BOX_ROW: i32 = 12;
/// The text's image (the line buffer `sub_30070B4` copies): 192 pixels
/// wide and 40 rows, a line every 14 rows, shown at (51, 108) as three
/// rows of six sprites (`chatbox_804021C`): 32x16, 32x16 and 32x8.
const TEXT_X: i32 = 0x33;
const TEXT_Y: i32 = 0x6C;
const TEXT_WIDTH: usize = 192;
const TEXT_ROWS: usize = 40;
const LINE_ROWS: usize = 14;
const TEXT_SPRITES: [(usize, usize); 3] = [(0, 16), (16, 16), (32, 8)];
/// The key-wait arrow's place by the box (`byte_8045DCC`): the message
/// box's (the default) and the description box's (`E8 06 01 01`).
const ARROW_AT: [(i32, i32); 2] = [(0xE2, 0x8D), (0xCA, 0x8D)];
/// The portrait sprite's place (`+0x84`, `+0x88`).
const PORTRAIT_AT: (i32, i32) = (0x19, 0x80);
/// The sprite layer the chatbox's sprites go to, in front of the rest.
const LAYER: usize = 0;
const TEXT_BUCKET: usize = 3;
const FRONT_BUCKET: usize = 4;

/// The box kinds (`Chatbox::boxes`).
const MESSAGE_BOX: usize = 0;
const DESCRIPTION_BOX: usize = 1;

/// What the chatbox shows this frame, worked out once (its text's sprite
/// tiles live here).
pub struct Shown<'a> {
    chatbox: Chatbox,
    kind: usize,
    text: Tiles,
    portrait: Option<(&'a SpriteSheet, PortraitLook)>,
}

/// The local player's chatbox, if one is up, with its text composed.
pub fn prepare<'a>(b: &Battle, assets: &'a Bundle, problems: &mut Problems) -> Option<Shown<'a>> {
    let (side, screen) = crate::custom::local(b)?;
    let navi = b.stats[b.setup.local_side as usize & 1].navi;
    let (chatbox, words, portrait) = match screen.phase {
        Phase::Description { from_cross_window: false, chatbox } => {
            // The chip under the cursor as the screen checked it (the chip
            // window's).
            let chip = screen.look.chip_window.last_chip.map(|c| b.content.chip(c.id));
            (chatbox, chip.and_then(|c| c.description.clone()), None)
        }
        Phase::Description { from_cross_window: true, chatbox } => {
            let w = &screen.crosses;
            let form = b.content.cross_form(navi, side.unlocks.version, w.offered[w.cursor as usize]);
            (chatbox, form.and_then(|f| b.content.form(f).description.clone()), None)
        }
        Phase::RunMessage { chatbox: Some(chatbox) } => {
            let m = &b.content.navi(navi).run_message;
            (chatbox, Some(m.text.clone()), m.portrait)
        }
        _ => return None,
    };
    let graphics = &assets.hud.chatbox;
    if graphics.is_empty() || assets.hud.dialogue_font.is_empty() {
        problems.note("the pack has no chatbox graphics or dialogue font (extract it again)".into());
        return None;
    }
    let kind = match chatbox.script() {
        Script::Description { .. } => DESCRIPTION_BOX,
        Script::RunMessage { .. } => MESSAGE_BOX,
    };
    let text = text_tiles(b, assets, &chatbox, words.as_deref().unwrap_or(""), problems);
    let portrait = match (portrait, chatbox.look().portrait) {
        (Some(id), Some(look)) => match assets.sprite(id.category, id.index) {
            Some(sheet) => {
                note_true_face(b, navi, side.unlocks.version, problems);
                Some((sheet, look))
            }
            None => {
                problems.note(format!("{} (a portrait) is not in the pack's graphics", crate::objects::sprite_name(b, id)));
                None
            }
        },
        _ => None,
    };
    Some(Shown { chatbox, kind, text, portrait })
}

/// The text's sprite tiles: the lines printed so far composed into the
/// line buffer's image, cut into the eighteen sprites' tiles (each
/// sprite's row by row).
fn text_tiles(b: &Battle, assets: &Bundle, chatbox: &Chatbox, words: &str, problems: &mut Problems) -> Tiles {
    let font = &assets.hud.dialogue_font;
    let mut image = vec![0u8; TEXT_WIDTH * TEXT_ROWS];
    if let Some((done, printing)) = chatbox.progress() {
        for (k, line) in words.split('\n').take(3).enumerate() {
            let (glyphs, missing) = fonts::dialogue_glyphs(font, line);
            if !missing.is_empty() {
                problems.note(format!("the chatbox's {line:?}: the pack's dialogue font has no glyph for {missing:?}"));
            }
            let shown = match (k as u8).cmp(&done) {
                std::cmp::Ordering::Less => glyphs.len(),
                std::cmp::Ordering::Equal => (printing as usize).min(glyphs.len()),
                std::cmp::Ordering::Greater => 0,
            };
            fonts::dialogue_text(font, &glyphs[..shown], &mut image, TEXT_WIDTH, LINE_ROWS * k);
        }
    }
    let _ = b;
    let mut tiles = Tiles { pixels: Vec::with_capacity(TEXT_WIDTH * TEXT_ROWS) };
    for (top, height) in TEXT_SPRITES {
        for column in 0..TEXT_WIDTH / 32 {
            for ty in 0..height / 8 {
                for tx in 0..4 {
                    for y in 0..8 {
                        let at = (top + 8 * ty + y) * TEXT_WIDTH + 32 * column + 8 * tx;
                        tiles.pixels.extend_from_slice(&image[at..at + 8]);
                    }
                }
            }
        }
    }
    tiles
}

/// A link navi of the other game (whose Cross is that game's) has a black
/// placeholder for a portrait in this console's ROM, which the original
/// shows; the frontend shows the true face (docs/frontend.md §5).
fn note_true_face(b: &Battle, navi: nettai_content_api::NaviHandle, console: GameVersion, problems: &mut Problems) {
    let game = b.content.defs.forms.iter().find(|f| f.record.cross_of == Some(navi)).and_then(|f| f.record.game);
    if game.is_some_and(|g| g != console) {
        problems.known(5, 104, 40, 48, "another game's link navi's portrait (the true face)");
    }
}

/// Draw the chatbox: the box on `names_layer` (BG0), the sprites into
/// `list`, in front of the screen's.
pub fn draw<'a>(shown: &'a Shown<'a>, assets: &'a Bundle, names_layer: &mut Layer, list: &mut SpriteList<'a>) {
    let g = &assets.hud.chatbox;
    let c = &shown.chatbox;
    if let Some(step) = c.box_step() {
        draw_box(g, shown.kind, step, names_layer);
    }
    // chatbox_804021C: the text, when the box is open; the line buffer's
    // last row of sprites first.
    if c.shows_contents() {
        let mut first = 0;
        let mut rows: Vec<Vec<SpritePart<'a>>> = Vec::new();
        for (top, height) in TEXT_SPRITES {
            let mut row = Vec::new();
            for column in 0..TEXT_WIDTH / 32 {
                let at = (TEXT_X + 32 * column as i32, TEXT_Y + top as i32);
                row.push(part(&shown.text, first, at, (32, height as u8), g.text_palette));
                first += 4 * height / 8;
            }
            row.reverse();
            rows.push(row);
        }
        rows.reverse();
        list.insert_at(LAYER, TEXT_BUCKET, rows.into_iter().flatten().collect());
    }
    // chatbox_804082C: the arrow; chatbox_8040B8C: the portrait in front.
    if let Some(frame) = c.look().arrow {
        let at = ARROW_AT[shown.kind.min(1)];
        list.insert_at(LAYER, FRONT_BUCKET, vec![part(&g.arrow, 4 * frame as usize, at, (16, 16), g.text_palette)]);
    }
    if let Some((sheet, look)) = shown.portrait {
        list.insert_at(LAYER, FRONT_BUCKET, portrait_parts(sheet, look));
    }
}

/// The box's map at an opening step (`chatbox_CopyBackgroundTiles_8040344`).
fn draw_box(g: &Graphics, kind: usize, step: u8, layer: &mut Layer) {
    let Some(map) = g.boxes.get(kind).and_then(|steps| steps.get(step as usize)) else { return };
    for (i, e) in map.iter().enumerate() {
        let (col, row) = ((i % Graphics::COLUMNS) as i32, (i / Graphics::COLUMNS) as i32);
        if let Some(t) = g.tiles.get(e.tile as usize) {
            layer.draw_tile(t, &g.palette, 8 * col, 8 * (BOX_ROW + row), e.hflip, e.vflip);
        }
    }
}

/// The portrait's parts at its frame: its animation stepped as the
/// sprite's updates step it (`sub_3006792`), drawn with its first
/// palette plus the tint (`sub_8002818`), the last part in front.
fn portrait_parts(sheet: &SpriteSheet, look: PortraitLook) -> Vec<SpritePart<'_>> {
    let Some(frames) = sheet.animations.get(look.anim as usize).filter(|f| !f.is_empty()) else { return Vec::new() };
    let k = frame_after(frames.iter().map(|f| (f.duration, f.flags)).collect::<Vec<_>>().as_slice(), look.updates);
    let frame = frames[k];
    let (Some(parts), Some(tiles)) = (sheet.part_lists.get(frame.parts as usize), sheet.tilesets.get(frame.tileset as usize)) else {
        return Vec::new();
    };
    let set = sheet.palette_sets.get(frame.palette_set as usize);
    let first = parts.first().map_or(0, |p| p.palette) as usize;
    let palette = set.and_then(|s| s.get(first)).copied().unwrap_or([0; 16]);
    let palette = palette.map(|c| tint(c, look.tint));
    let mut out: Vec<SpritePart> = parts
        .iter()
        .map(|p| SpritePart {
            hflip: p.hflip,
            vflip: p.vflip,
            ..part(tiles, p.tile as usize, (PORTRAIT_AT.0 + p.x as i32, PORTRAIT_AT.1 + p.y as i32), (p.width, p.height), palette)
        })
        .collect();
    out.reverse();
    out
}

/// The frame an animation is at after `updates` updates of the sprite
/// (`sub_3006792`): each takes a tick off the frame's time, and while that
/// leaves it spent moves on (to the next frame, or at the last back to the
/// first if it loops, flag 0x40, else holding it) and takes a tick off the
/// new frame's time too. A frame shows for its time in updates, the first
/// counting from the update on the tick the animation is set.
pub fn frame_after(frames: &[(u8, u8)], updates: u16) -> usize {
    let (mut k, mut left) = (0usize, frames[0].0 as i32);
    for _ in 0..updates {
        loop {
            left -= 1;
            if left >= 0 {
                break;
            }
            let flags = frames[k].1;
            if flags & 0x80 != 0 {
                if flags & 0x40 != 0 {
                    k = 0;
                    left = frames[0].0 as i32;
                } else {
                    left = 1;
                }
            } else {
                k = (k + 1).min(frames.len() - 1);
                left = frames[k].0 as i32;
            }
        }
    }
    k
}

/// A colour with the tint added, each channel saturating
/// (`sub_3005F34`).
fn tint(c: u16, t: u16) -> u16 {
    let ch = |v: u16, s: u16| (((v >> s) & 31) + ((t >> s) & 31)).min(31) << s;
    (c & 0x8000) | ch(c, 0) | ch(c, 5) | ch(c, 10)
}

fn part(tiles: &Tiles, first_tile: usize, (x, y): (i32, i32), (width, height): (u8, u8), palette: Palette) -> SpritePart<'_> {
    SpritePart {
        x: (x & 0x1FF) as u16,
        y: (y & 0xFF) as u8,
        width,
        height,
        tiles,
        first_tile,
        hflip: false,
        vflip: false,
        palette,
        priority: 0,
        alpha: None,
        mosaic: None,
        vscale: None,
        affine: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_animation_steps_by_its_frames_times_and_loops_or_holds() {
        // MegaMan's talking face: 7, 6, then loops from the start.
        let talking = [(7, 0), (6, 0xC0)];
        let at = |n| frame_after(&talking, n);
        assert_eq!((at(1), at(7), at(8), at(13), at(14)), (0, 0, 1, 1, 0));
        // A still face holds its one frame.
        assert_eq!(frame_after(&[(6, 0x80)], 100), 0);
    }

    #[test]
    fn the_tint_adds_to_each_channel_up_to_31() {
        assert_eq!(tint(0x0000, 0x18C6), 0x18C6);
        assert_eq!(tint(0x7FFF, 0x18C6), 0x7FFF);
        assert_eq!(tint(0x001E, 0x0006), 0x001F);
    }
}
