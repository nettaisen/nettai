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

use crate::compose::{SpritePart, WIDTH};
use crate::textlayer::TextItem;

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

/// A console whose pieces a drawing draws (by its side), and where.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seat {
    pub side: u8,
    pub at: At,
}

impl Seat {
    /// `side`'s console on its own screen.
    pub fn alone(side: u8) -> Seat {
        Seat { side: side & 1, at: At::Alone }
    }

    /// The seats of a frame showing `consoles`, the local side `local`'s
    /// first: the left's pieces before the right's.
    pub fn of(consoles: Consoles, local: u8) -> Vec<Seat> {
        match consoles {
            Consoles::One => vec![Seat::alone(local)],
            Consoles::Both => vec![Seat { side: local & 1, at: At::Left }, Seat { side: (local ^ 1) & 1, at: At::Right }],
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

    /// Whether it draws the pieces every console shows alike: on its own
    /// screen, or as the left of two (the right's would be the same again).
    pub fn draws_shared(self) -> bool {
        self.at != At::Right
    }

    /// Whether it draws its console's pieces about the other side (the
    /// "????" of the other side's defensive chip, the other player's HP
    /// under its navi): on its own screen alone; on a screen of both the
    /// other side's own pieces say it.
    pub fn draws_other(self) -> bool {
        self.at == At::Alone
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

    /// A known difference's or a mark's box ([`crate::audit::Problems`]) at
    /// its place.
    pub fn rect(self, [x0, y0, x1, y1]: [i32; 4]) -> [i32; 4] {
        let x = self.x(x0, x1 - x0);
        [x, y0, x + (x1 - x0), y1]
    }
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
        let right = Seat::of(Consoles::Both, 0)[1];
        assert_eq!(right, Seat { side: 1, at: At::Right });
        // The HP box (48 wide at 0) at the top right; a box beside it.
        assert_eq!(right.x(0, 48), 192);
        assert_eq!(right.x(32, 16), 192);
        assert!(right.flip(false) && !right.flip(true));
        // Mirrored about an object at x 100: a box 4 right of it goes 4 left.
        assert_eq!(right.about(100, 4, 16), 80);
        let left = Seat::of(Consoles::Both, 0)[0];
        assert_eq!((left.x(10, 8), left.flip(true), left.about(100, 4, 16)), (10, true, 104));
        assert!(left.draws_shared() && !left.draws_other() && !right.draws_shared());
        assert!(Seat::alone(1).draws_other());
        assert_eq!(Seat::of(Consoles::One, 1), [Seat::alone(1)]);
    }
}
