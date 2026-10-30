//! Whose console's pieces a drawing draws, and where they go: one
//! console's screen as it draws it, or a screen of both consoles
//! ([`Consoles::Both`]): the field once, as the left console draws it,
//! each player's own pieces on its own side, the left player's where its
//! console draws them and the right player's mirrored into the right half.
//!
//! Mirroring is a matter of places, never of pixels of what reads: a frame,
//! a box or a face is flipped at its mirrored place (a tile's or a sprite's
//! flip, a drawing's), while words, digits, chip art and icons go to the
//! mirrored place of the box they are laid in and are laid there as the
//! console lays them, upright. A piece tied to an object (a chip icon over a
//! navi, an HP number under one) goes to the mirror of its place about the
//! object's, which is where the left console's view shows the object.
//!
//! While the custom screens are up each seat draws as `Shows` says: each
//! console's screen in its own half (C1: both windows side by side, and as
//! both slide in or out, each one's header beside its window), the one
//! console whose player still chooses alone (W2), and as one window leaves
//! while the other stays, that one over the screen of the console that
//! stays. A window is drawn as its console draws it and put in the right
//! half by a [`Mirror`]: its frame mirrored, its parts that read moved
//! whole.

use crate::compose::{CLEAR, HEIGHT, Layer, SpritePart, WIDTH};
use crate::textlayer::{Rect, TextItem};
use nettai_battle::Battle;

/// Which consoles a frame shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Consoles {
    /// The local side's (`RoundSetup::local_side`), as its console draws its
    /// screen.
    #[default]
    One,
    /// Both on one screen: the local side's on the left, as its console
    /// draws its screen, the other side's pieces on the right
    /// ([`Seat::right`]).
    Both,
}

/// Where a console's pieces go on the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum At {
    /// Its own screen: everything it draws, where it draws it.
    Alone,
    /// The left of a screen of both: its own pieces where it draws them,
    /// and the pieces both consoles show alike (the custom gauge, the
    /// banners but the round's result, the turn timer, the field's pieces).
    Left,
    /// The right of a screen of both: its own pieces alone, mirrored.
    Right,
}

/// What a seat draws of its console's pieces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shows {
    /// All that its console draws: a console alone; on a screen of both,
    /// while its player alone chooses on the custom screen (the other's
    /// result is sent, or its window leaves: its window, the header beside
    /// it and what else its console draws, W2), and while its player's
    /// window leaves after the other's has left.
    Screen,
    /// All that its console draws in its own half of a screen of both (the
    /// left's left half, the right's mirrored into the right half): while
    /// both players choose, its window alone (C1), and as both windows
    /// slide in or out together, each its window and its header beside it.
    Half,
    /// Its own pieces, beside the other console's (the fight on a screen of
    /// both, F1): no HP under a navi, the other side's pieces left to the
    /// other.
    Own,
    /// Its custom screen's window alone, leaving over the screen of the
    /// console whose player still chooses (as that one draws it under).
    Window,
    /// Nothing: its player's result is sent and the other player chooses,
    /// whose console's screen it is (W2).
    Nothing,
}

/// A console whose pieces a drawing draws (by its side), where, and what of
/// them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seat {
    pub side: u8,
    pub at: At,
    pub shows: Shows,
}

impl Seat {
    /// `side`'s console on its own screen.
    pub fn alone(side: u8) -> Seat {
        Seat { side: side & 1, at: At::Alone, shows: Shows::Screen }
    }

    /// The seats of a frame of `b` showing `consoles`, the local side's
    /// first: the left's pieces before the right's.
    pub fn of(consoles: Consoles, b: &Battle) -> Vec<Seat> {
        let local = b.setup.local_side & 1;
        match consoles {
            Consoles::One => vec![Seat::alone(local)],
            Consoles::Both => {
                // (The screen that stays up longer is drawn, the other's
                // window leaving over it; two that stay alike, each in its
                // half: `custom::screens_left`.)
                let left = crate::custom::screens_left(b);
                let shows = |side: u8| match (left[side as usize], left[side as usize ^ 1]) {
                    (0, 0) => Shows::Own,
                    (own, other) if own == other => Shows::Half,
                    (own, other) if own > other => Shows::Screen,
                    (0, _) => Shows::Nothing,
                    _ => Shows::Window,
                };
                vec![Seat { side: local, at: At::Left, shows: shows(local) }, Seat { side: local ^ 1, at: At::Right, shows: shows(local ^ 1) }]
            }
        }
    }

    /// Whether it is one of two on a screen of both.
    pub fn both(self) -> bool {
        self.at != At::Alone
    }

    /// Whether its pieces are placed mirrored (the right of a screen of
    /// both).
    pub fn mirrored(self) -> bool {
        self.at == At::Right
    }

    /// Whether it draws the pieces every console shows alike: its whole
    /// screen, or as the left of two (the right's would be the same again).
    pub fn draws_shared(self) -> bool {
        self.at != At::Right || self.shows == Shows::Screen
    }

    /// Whether it draws its console's pieces about the other side (the
    /// "????" of the other side's defensive chip, the other player's HP
    /// under its navi): its whole screen; beside the other console's, the
    /// other side's own pieces say it.
    pub fn draws_other(self) -> bool {
        matches!(self.shows, Shows::Screen | Shows::Half)
    }

    /// The screen's columns its pieces are kept to (the first, one past the
    /// last), on a screen of both where not all: its own half (`Half`), or
    /// all but where the other console's window is as it leaves over it
    /// (`Screen` beside a `Window`).
    pub fn clip(self, b: &Battle, seats: &[Seat]) -> Option<[i16; 2]> {
        let half = WIDTH as i16 / 2;
        match self.shows {
            Shows::Half => Some(if self.mirrored() { [half, WIDTH as i16] } else { [0, half] }),
            Shows::Screen => {
                let other = seats.iter().find(|s| s.side != self.side && s.shows == Shows::Window)?;
                let (_, screen) = crate::custom::screen_of(b, other.side)?;
                // (Its window's columns: 120 less the scroll in from its edge.)
                let edge = (half as i32 - crate::custom::placement(screen).scroll as i32).max(0) as i16;
                Some(if other.mirrored() { [0, WIDTH as i16 - edge] } else { [edge, WIDTH as i16] })
            }
            _ => None,
        }
    }

    /// Where a box `w` wide that its console draws at `x` goes.
    pub fn x(self, x: i32, w: i32) -> i32 {
        if self.mirrored() { WIDTH as i32 - x - w } else { x }
    }

    /// A frame's flip at its place: flipped once more where mirrored.
    pub fn flip(self, hflip: bool) -> bool {
        hflip ^ self.mirrored()
    }

    /// Where a box `w` wide goes that its console draws `dx` from an
    /// object's place there, the object being at `anchor` on the screen
    /// drawn: mirrored about it.
    pub fn about(self, anchor: i32, dx: i32, w: i32) -> i32 {
        if self.mirrored() { anchor - dx - w } else { anchor + dx }
    }

    /// How far right of the mirrored place of its box (`x`, `w` wide, where
    /// [`Seat::about`] or [`Seat::x`] put it) a piece laid there (`parts`)
    /// goes so that what it draws mirrors, not its box: the margins its box
    /// has about its pixels traded sides (an HP number's four cells about
    /// its digits). None where the seat doesn't mirror.
    pub fn ink_shift(self, parts: &[SpritePart], x: i32, w: i32) -> i32 {
        match ink_span(parts).filter(|_| self.mirrored()) {
            Some((s0, s1)) => w - (s0 - x) - (s1 - x),
            None => 0,
        }
    }

    /// A known difference's or a mark's box ([`crate::audit::Problems`]) at
    /// its place.
    pub fn rect(self, [x0, y0, x1, y1]: [i32; 4]) -> [i32; 4] {
        let x = self.x(x0, x1 - x0);
        [x, y0, x + (x1 - x0), y1]
    }
}

/// The side whose console the field is seen through on a frame of `b`
/// showing `consoles`: the local side's, but on a screen of both where the
/// other player alone chooses on the custom screen (W2: the field as its
/// console has it, moved down with its window, faded with its screen), from
/// the first tick the local's window leaves while the other's stays longer
/// (`custom::screens_left`: under the leaving window the field is the
/// staying console's, as it is once the window has left).
pub fn field_side(consoles: Consoles, b: &Battle) -> u8 {
    let local = b.setup.local_side & 1;
    let left = crate::custom::screens_left(b);
    match consoles {
        Consoles::Both if left[local as usize ^ 1] > left[local as usize] => local ^ 1,
        _ => local,
    }
}

/// A piece its console draws (a custom screen's window), put on the right
/// of a screen of both: its frame mirrored (a pixel at x at 239 - x, a
/// drawing flipped at its mirrored place) and the parts that read (its
/// blocks: the chip window, the hand, the buttons, the labels) moved whole
/// to the mirrored place of their block, upright. Sprites are never
/// flipped (a window's are its cursor and frames over its blocks, which
/// move with them, and icons).
pub struct Mirror {
    /// The blocks, in frame pixels where the console draws them.
    blocks: Vec<Block>,
    /// How far right of its place the mirrored piece is put (`slid`).
    slide: i32,
}

/// A part of a mirrored piece that reads: what it covers, moved whole to
/// the mirrored place of the box it is laid in (its own, or the frame it
/// sits in: the chip window's in its box, which mirrors as frame).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Block {
    pub area: Rect,
    pub frame: Rect,
}

impl Block {
    /// A block laid in its own box.
    pub fn whole(area: Rect) -> Block {
        Block { area, frame: area }
    }
}

impl Mirror {
    pub fn new(blocks: Vec<Block>) -> Mirror {
        Mirror { blocks, slide: 0 }
    }

    /// A piece drawn where it comes to rest, `dx` pixels right of where its
    /// console has it now (a window sliding in or out, drawn whole: its
    /// layers and their items there, its sprites where they are), put
    /// `dx` right of its mirrored place: the mirrored piece slides as its
    /// console's does, the other way, whole.
    pub fn slid(self, dx: i32) -> Mirror {
        Mirror { slide: dx, ..self }
    }

    /// The block the point (x, y) is in.
    fn block(&self, x: i32, y: i32) -> Option<Block> {
        self.blocks.iter().copied().find(|b| (b.area.x..b.area.x + b.area.w).contains(&x) && (b.area.y..b.area.y + b.area.h).contains(&y))
    }

    /// How far what is in block `b` moves: to the mirrored place of the box
    /// it is laid in.
    fn shift(b: Block) -> i32 {
        WIDTH as i32 - b.frame.w - 2 * b.frame.x
    }

    /// Where a box goes: moved with the block its middle is in (false), or
    /// mirrored (true: what it shows is flipped there, if it is a frame).
    pub fn place(&self, r: Rect) -> (i32, bool) {
        match self.block(r.x + r.w / 2, r.y + r.h / 2) {
            Some(b) => (r.x + Self::shift(b), false),
            None => (WIDTH as i32 - r.x - r.w, true),
        }
    }

    /// `from`'s pixels put into `to` (a layer as wide), each moved with its
    /// block or mirrored, on the original's screen.
    pub fn layer(&self, from: &Layer, to: &mut Layer) {
        if !to.drawn {
            return;
        }
        debug_assert_eq!(from.width, to.width);
        let w = from.width as i32;
        // (In three passes: a block's pixels mirrored where no block moves
        // to, which fills what a block laid in a larger frame leaves
        // (that frame's part about it, the chip window's box's edge); the
        // blocks moved; the frame mirrored over them, which puts back what
        // a block's frame took of it.)
        for pass in 0..3 {
            for y in 0..HEIGHT {
                for x in 0..w {
                    let i = y * w as usize + x as usize;
                    if from.pixels[i] == CLEAR {
                        continue;
                    }
                    let (px, py) = (x - from.origin, y as i32);
                    let mirrored = WIDTH as i32 - 1 - px;
                    let at = self.slide
                        + match (pass, self.block(px, py)) {
                            (0, Some(_)) if !self.moved_to(mirrored, py) => mirrored,
                            (1, Some(b)) => px + Self::shift(b),
                            (2, None) => mirrored,
                            _ => continue,
                        };
                    let at = at + to.origin;
                    if (0..w).contains(&at) {
                        let j = y * w as usize + at as usize;
                        to.pixels[j] = from.pixels[i];
                        to.past_ranged[j] = from.past_ranged[i];
                    }
                }
            }
        }
    }

    /// A sprite's parts (a cursor's corners, an icon) at their places, as
    /// one: moved with the block the group's middle is in, or each at its
    /// mirrored place; never flipped.
    pub fn group(&self, parts: &mut [SpritePart]) {
        // (Where they are on the piece at rest.)
        move_parts(parts, &mut [], self.slide);
        let Some((x0, x1)) = parts_span(parts) else { return };
        let y0 = parts.iter().map(|p| p.y as i32).min().unwrap_or(0);
        let y1 = parts.iter().map(|p| p.y as i32 + p.height as i32).max().unwrap_or(0);
        match self.block((x0 + x1) / 2, (y0 + y1) / 2) {
            Some(b) => move_parts(parts, &mut [], Self::shift(b)),
            // (Each where what it draws mirrors: its pixels' columns.)
            None => {
                for p in parts.iter_mut() {
                    let box_width = p.width as i32 * if p.affine.is_some_and(|m| m.double) { 2 } else { 1 };
                    let [a, b] = part_ink(p).unwrap_or([0, box_width]);
                    p.x = ((WIDTH as i32 - part_x(p) - a - b) & 0x1FF) as u16;
                }
            }
        }
        move_parts(parts, &mut [], self.slide);
    }

    /// A text item at its place: words, numbers and drawings in a block
    /// moved with it; a drawing elsewhere flipped at its mirrored place (one
    /// over several, by the cells it draws in, as several items); words
    /// elsewhere at the mirrored place of their box, upright.
    pub fn item(&self, item: TextItem) -> Vec<TextItem> {
        let moved = |item: TextItem, dx: i32| TextItem { rect: item.rect.offset(dx, 0), clip: moved_clip(item.clip, dx), ..item };
        self.item_at_rest(item).into_iter().map(|item| moved(item, self.slide)).collect()
    }

    /// [`Mirror::item`] for the piece at rest.
    fn item_at_rest(&self, item: TextItem) -> Vec<TextItem> {
        let moved = |item: TextItem, dx: i32| TextItem { rect: item.rect.offset(dx, 0), clip: moved_clip(item.clip, dx), ..item };
        let Some(d) = item.drawn.as_ref() else {
            let (x, _) = self.place(item.rect);
            let dx = x - item.rect.x;
            return vec![moved(item, dx)];
        };
        // (A drawing in a block that doesn't hold all of it, what a part
        // laid with a margin of the frame's draws, is put as the layer's
        // pixels are: flipped at its mirrored place as the frame, and what
        // is in the block moved with it over that.)
        let holds = |b: Block, r: Rect| r.x >= b.area.x && r.x + r.w <= b.area.x + b.area.w && r.y >= b.area.y && r.y + r.h <= b.area.y + b.area.h;
        let in_block = |item: TextItem, b: Block| {
            let item = moved(item, Self::shift(b));
            TextItem { clip: item.clip.intersect(b.area.offset(Self::shift(b), 0)), ..item }
        };
        // (A drawing's cells, each where its middle is: those of each block,
        // and those of none.)
        let cells: Vec<[u8; 2]> = match &d.cells {
            Some(c) => c.clone(),
            None => {
                return match self.block(item.rect.x + item.rect.w / 2, item.rect.y + item.rect.h / 2) {
                    Some(b) if holds(b, item.rect) => vec![moved(item, Self::shift(b))],
                    Some(b) => vec![flipped(item.clone()), in_block(item, b)],
                    None => vec![flipped(item)],
                };
            }
        };
        // (As the layer's three passes: a block's cells mirrored where no
        // block moves to, or where the block doesn't hold all of the cell;
        // the blocks' moved, but what they don't hold; the frame's
        // mirrored.)
        let mut under: Vec<[u8; 2]> = Vec::new();
        let mut groups: Vec<(Option<Block>, Vec<[u8; 2]>)> = Vec::new();
        for c in cells {
            let (x0, y0) = (item.rect.x + 8 * c[0] as i32, item.rect.y + 8 * c[1] as i32);
            let (x, y) = (x0 + 4, y0 + 4);
            let at = self.block(x, y);
            if at.is_some_and(|b| !holds(b, Rect::new(x0, y0, 8, 8)) || !self.moved_to(WIDTH as i32 - 1 - x, y)) {
                under.push(c);
            }
            match groups.iter_mut().find(|(b, _)| *b == at) {
                Some((_, g)) => g.push(c),
                None => groups.push((at, vec![c])),
            }
        }
        groups.sort_by_key(|(b, _)| b.is_none());
        let with_cells = |cells: Vec<[u8; 2]>| {
            let mut part = item.clone();
            if let Some(d) = part.drawn.as_mut() {
                d.cells = Some(cells);
            }
            part
        };
        let mut out = Vec::new();
        if !under.is_empty() {
            out.push(flipped(with_cells(under)));
        }
        out.extend(groups.into_iter().map(|(block, cells)| match block {
            Some(b) => in_block(with_cells(cells), b),
            None => flipped(with_cells(cells)),
        }));
        out
    }

    /// Whether a block moves to the point (x, y).
    fn moved_to(&self, x: i32, y: i32) -> bool {
        self.blocks.iter().any(|b| {
            let x0 = b.area.x + Self::shift(*b);
            (x0..x0 + b.area.w).contains(&x) && (b.area.y..b.area.y + b.area.h).contains(&y)
        })
    }
}

/// A drawing's item flipped at its mirrored place.
fn flipped(item: TextItem) -> TextItem {
    let rect = Rect { x: WIDTH as i32 - item.rect.x - item.rect.w, ..item.rect };
    let clip = if item.clip == Rect::SCREEN { item.clip } else { Rect { x: WIDTH as i32 - item.clip.x - item.clip.w, ..item.clip } };
    let drawn = item.drawn.as_ref().map(|d| crate::shapes::mirrored(d, item.rect.w));
    TextItem { rect, clip, drawn, fine: (-item.fine.0, item.fine.1), ..item }
}

/// A clip moved `dx` right (the whole screen's stays the screen).
fn moved_clip(clip: Rect, dx: i32) -> Rect {
    if clip == Rect::SCREEN { clip } else { clip.offset(dx, 0) }
}

/// The columns of a sprite part that draw (from the first with a pixel to
/// one past the last, from its box's left), as its tiles have them; an
/// affine part's as its tiles stand unturned in its box (a spinning orb's
/// stay put); none for one that draws nothing.
fn part_ink(p: &SpritePart) -> Option<[i32; 2]> {
    let (w, h) = (p.width as usize, p.height as usize);
    let per_row = (w / 8).max(1);
    // (An affine part's texture isn't flipped, and stands in the middle of
    // a doubled box.)
    let (hflip, left) = match p.affine {
        Some(m) => (false, if m.double { w / 2 } else { 0 }),
        None => (p.hflip, 0),
    };
    let draws = |col: usize| {
        let tx = if hflip { w - 1 - col } else { col };
        (0..h).any(|ty| p.tiles.get(p.first_tile + (ty / 8) * per_row + tx / 8).is_some_and(|t| t[(ty % 8) * 8 + tx % 8] != 0))
    };
    let a = (0..w).find(|&c| draws(c))?;
    let b = (0..w).rev().find(|&c| draws(c))? + 1;
    Some([(left + a) as i32, (left + b) as i32])
}

/// The screen's columns sprite parts draw in (the first, one past the
/// last): their pixels', not their boxes'; none where they draw nothing.
pub fn ink_span(parts: &[SpritePart]) -> Option<(i32, i32)> {
    let spans = parts.iter().filter_map(|p| part_ink(p).map(|[a, b]| (part_x(p) + a, part_x(p) + b)));
    spans.reduce(|(a0, a1), (b0, b1)| (a0.min(b0), a1.max(b1)))
}

/// The screen x of a sprite part (its coordinate wrapped as the
/// hardware's: 0 to 511).
pub fn part_x(p: &SpritePart) -> i32 {
    let x = p.x as i32 & 0x1FF;
    if x >= 256 { x - 512 } else { x }
}

/// Move sprite parts and their text items `dx` pixels right: a piece laid
/// whole, put elsewhere.
pub fn move_parts(parts: &mut [SpritePart], items: &mut [TextItem], dx: i32) {
    if dx == 0 {
        return;
    }
    for p in parts {
        p.x = ((part_x(p) + dx) & 0x1FF) as u16;
    }
    for item in items {
        item.rect = item.rect.offset(dx, 0);
        item.clip = item.clip.offset(dx, 0);
    }
}

/// The columns a run of sprite parts covers: the first's left, one past the
/// last's right (none: no parts).
pub fn parts_span(parts: &[SpritePart]) -> Option<(i32, i32)> {
    let x0 = parts.iter().map(part_x).min()?;
    let x1 = parts.iter().map(|p| part_x(p) + p.width as i32).max()?;
    Some((x0, x1))
}

/// A run of words laid as sprite parts (a telop's name and numbers), with
/// its text items, put at the mirrored place of the box it covers, laid
/// there as it is: where `seat` mirrors.
pub fn mirror_run(seat: Seat, parts: &mut [SpritePart], items: &mut [TextItem]) {
    if !seat.mirrored() {
        return;
    }
    if let Some((x0, x1)) = parts_span(parts) {
        move_parts(parts, items, seat.x(x0, x1 - x0) - x0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_right_seat_mirrors_boxes_and_flips_frames() {
        let right = Seat { side: 1, at: At::Right, shows: Shows::Own };
        // The HP box (48 wide at 0) at the top right; a box beside it.
        assert_eq!(right.x(0, 48), 192);
        assert_eq!(right.x(32, 16), 192);
        assert!(right.flip(false) && !right.flip(true));
        // Mirrored about an object at x 100: a box 4 right of it goes 4 left.
        assert_eq!(right.about(100, 4, 16), 80);
        let left = Seat { side: 0, at: At::Left, shows: Shows::Own };
        assert_eq!((left.x(10, 8), left.flip(true), left.about(100, 4, 16)), (10, true, 104));
        assert!(left.draws_shared() && !left.draws_other() && !right.draws_shared());
        assert!(Seat::alone(1).draws_other());
    }

    /// A mirror moves what is in a block to the block's mirrored place, as
    /// it is, and mirrors the rest.
    #[test]
    fn a_mirror_moves_blocks_whole_and_mirrors_the_rest() {
        let m = Mirror::new(vec![Block::whole(Rect::new(8, 0, 24, 16)), Block { area: Rect::new(40, 0, 8, 8), frame: Rect::new(32, 0, 32, 8) }]);
        // In a block laid in a frame (32..64, mirrored to 176..208): moved with it.
        assert_eq!(m.place(Rect::new(40, 0, 8, 8)), (184, false));
        // In the block (8..32): to 208..232, the same way round.
        assert_eq!(m.place(Rect::new(8, 0, 8, 8)), (208, false));
        assert_eq!(m.place(Rect::new(24, 8, 8, 8)), (224, false));
        // Elsewhere: mirrored.
        assert_eq!(m.place(Rect::new(0, 0, 8, 8)), (232, true));
        let mut from = Layer::new(1, 3);
        let mut to = Layer::new(1, 3);
        from.pixels[8] = 1;
        from.pixels[9] = 2;
        from.pixels[0] = 3;
        m.layer(&from, &mut to);
        assert_eq!((to.pixels[208], to.pixels[209], to.pixels[239]), (1, 2, 3));
    }

    /// A sprite on no block goes where what it draws mirrors: a 16-wide part
    /// whose pixels are its columns 3 to 5 at x 20 (pixels 23 to 25) at 214
    /// to 216, its box at 211; an affine part's doubled box holds its
    /// texture in its middle (columns 8 more in).
    #[test]
    fn a_sprite_goes_where_its_pixels_mirror() {
        let mut pixels = vec![0u8; 2 * nettai_assets::Tiles::TILE];
        for row in 0..8 {
            pixels[row * 8 + 3..row * 8 + 6].fill(1);
        }
        let tiles = nettai_assets::Tiles { pixels };
        let part = SpritePart {
            x: 20,
            y: 0,
            width: 16,
            height: 8,
            tiles: &tiles,
            first_tile: 0,
            hflip: false,
            vflip: false,
            palette: [0; 16],
            priority: 0,
            alpha: None,
            mosaic: None,
            vscale: None,
            affine: None,
            fade: None,
            clip: None,
            anchor: crate::compose::Anchor::Center,
        };
        let m = Mirror::new(Vec::new());
        let mut parts = [part.clone()];
        m.group(&mut parts);
        assert_eq!(part_x(&parts[0]), 240 - 26 - 3);
        let turned = crate::compose::Affine { pa: 256, pb: 0, pc: 0, pd: 256, double: true };
        let mut parts = [SpritePart { affine: Some(turned), ..part }];
        m.group(&mut parts);
        assert_eq!(part_x(&parts[0]), 240 - 20 - (8 + 3) - (8 + 6));
    }

    /// A piece drawn at rest and slid: all of it where the mirror at rest
    /// puts it, that much further right (a window sliding in from the
    /// right edge); what goes past the edge is off the screen.
    #[test]
    fn a_slid_mirror_puts_the_piece_at_rest_further_right() {
        let m = Mirror::new(vec![Block::whole(Rect::new(8, 0, 24, 16))]).slid(5);
        let mut from = Layer::new(1, 3);
        let mut to = Layer::new(1, 3);
        from.pixels[8] = 1;
        from.pixels[0] = 3;
        from.pixels[6] = 4;
        m.layer(&from, &mut to);
        // (8 to 208, then 213; 6 to 233, then 238; 0 to 239, then off.)
        assert_eq!((to.pixels[213], to.pixels[238]), (1, 4));
        assert!(!to.pixels.contains(&3));
        let item = TextItem::new("A", crate::vfont::Role::Cell, Rect::new(8, 0, 8, 16), 0, None);
        assert_eq!(m.item(item)[0].rect.x, 213);
    }
}
