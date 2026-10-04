//! A player's NaviCust (BN4's, BN5's and BN6's Navi Customizer;
//! docs/design/navicust.md): the programs they have placed on its grid,
//! each in one of its colors, turned and compressed or not, and how far
//! the board has been expanded.
//!
//! The programs are definitions (`define.navicust_program`,
//! `Content::navicust_program`): their colors and shapes, and what a game's
//! rules read. A player's NaviCust is their setup's
//! ([`crate::custom::PlayerSetup::navicust`]); what it gives the navi is a
//! game's rules' (BN6's navicust system compiles it into the side's stats
//! as the round is set up, `round_setup`). The board (which cells a
//! program can cover, and the command line) is the game's rule section
//! `navicust` ([`crate::content::NaviCustRules`]).

use nettai_content_api::NaviCustProgramHandle;

/// The NaviCust's grid is this many cells a side, and so is a program's
/// shape, centered on its middle cell.
pub const SIZE: usize = 7;

/// A program's cells on a 7x7 grid, by row then column; its center is
/// (3, 3).
pub type Shape = [[bool; SIZE]; SIZE];

/// How many programs a NaviCust holds: BN6's list's room (0x31 parts).
pub const MAX_PARTS: usize = 49;

/// A program on the grid: which, in which of its colors (an index into its
/// definition's `colors`), where its center is (a cell of the 7x7 grid),
/// turned a quarter clockwise `rotation` times, compressed or not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlacedProgram {
    pub program: NaviCustProgramHandle,
    pub color: u8,
    pub x: u8,
    pub y: u8,
    pub rotation: u8,
    pub compressed: bool,
}

/// A player's NaviCust: its programs in its list's order (the order a game's
/// rules go through them), and the board's expansions (BN6's and BN5's:
/// none, one or two; BN6's key item 0x71, BN5's 0x61).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NaviCust {
    parts: [Option<PlacedProgram>; MAX_PARTS],
    len: u8,
    pub expansions: u8,
}

impl Default for NaviCust {
    fn default() -> NaviCust {
        NaviCust { parts: [None; MAX_PARTS], len: 0, expansions: 0 }
    }
}

impl NaviCust {
    /// `parts`, in order, on a board with `expansions`; an error past
    /// [`MAX_PARTS`].
    pub fn new(parts: &[PlacedProgram], expansions: u8) -> Result<NaviCust, String> {
        if parts.len() > MAX_PARTS {
            return Err(format!("{} NaviCust programs: a NaviCust holds {MAX_PARTS}", parts.len()));
        }
        let mut n = NaviCust { expansions, ..NaviCust::default() };
        for (slot, &p) in n.parts.iter_mut().zip(parts) {
            *slot = Some(p);
        }
        n.len = parts.len() as u8;
        Ok(n)
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The programs in the list's order.
    pub fn iter(&self) -> impl Iterator<Item = PlacedProgram> + '_ {
        self.parts.iter().take(self.len as usize).map(|p| p.expect("a program in the list"))
    }
}

/// `shape` turned a quarter clockwise `rotation` times, as BN6 turns a
/// program (`sub_813B7A0`'s four copies: as it is, `sub_813B7FC` a quarter
/// clockwise, `sub_813B818` a half, `sub_813B830` a quarter back).
pub fn rotate(shape: &Shape, rotation: u8) -> Shape {
    let n = SIZE - 1;
    let mut out = [[false; SIZE]; SIZE];
    for (y, row) in shape.iter().enumerate() {
        for (x, &cell) in row.iter().enumerate() {
            let (oy, ox) = match rotation & 3 {
                0 => (y, x),
                1 => (x, n - y),
                2 => (n - y, n - x),
                _ => (n - x, y),
            };
            out[oy][ox] = cell;
        }
    }
    out
}

/// The grid cells a program's shape covers placed with its center at
/// `(x, y)` (those inside the grid), as (column, row).
pub fn cells(shape: &Shape, x: u8, y: u8) -> impl Iterator<Item = (i32, i32)> + '_ {
    let c = (SIZE / 2) as i32;
    shape.iter().enumerate().flat_map(move |(j, row)| {
        row.iter().enumerate().filter(|&(_, &on)| on).map(move |(i, _)| (x as i32 - c + i as i32, y as i32 - c + j as i32))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(rows: [&str; SIZE]) -> Shape {
        rows.map(|r| std::array::from_fn(|i| r.as_bytes()[i] == b'#'))
    }

    /// A quarter turn clockwise moves the cell above the center to its
    /// right, as `sub_813B7FC` copies; four turns are none.
    #[test]
    fn programs_turn_as_bn6_turns_them() {
        let up = shape([".......", ".......", "...#...", "...#...", ".......", ".......", "......."]);
        let right = shape([".......", ".......", ".......", "...##..", ".......", ".......", "......."]);
        let down = shape([".......", ".......", ".......", "...#...", "...#...", ".......", "......."]);
        let left = shape([".......", ".......", ".......", "..##...", ".......", ".......", "......."]);
        assert_eq!(rotate(&up, 1), right);
        assert_eq!(rotate(&up, 2), down);
        assert_eq!(rotate(&up, 3), left);
        assert_eq!(rotate(&up, 4), up);
        let placed: Vec<_> = cells(&up, 1, 3).collect();
        assert_eq!(placed, vec![(1, 2), (1, 3)]);
        let parts = [PlacedProgram { program: NaviCustProgramHandle(0), color: 0, x: 3, y: 3, rotation: 0, compressed: false }; 50];
        assert!(NaviCust::new(&parts, 2).is_err());
        assert_eq!(NaviCust::new(&parts[..3], 2).unwrap().iter().count(), 3);
    }
}
