//! A NaviCust program's shape on the 7x7 grid (BN4's, EXE5's and EXE6's
//! Navi Customizer; docs/design/navicust.md), turned: what the program
//! definitions' shapes read as (`Content::navicust_program`), for the
//! tools that draw and check a NaviCust. A player's NaviCust is their
//! setup's facts (`navicust_expansions`, `navicust_programs`), which their
//! game's rules compile into the navi's stats as the round is set up
//! (@exelib/navicust/compile).

/// The NaviCust's grid is this many cells a side, and so is a program's
/// shape, centered on its middle cell.
pub const SIZE: usize = 7;

/// A program's cells on a 7x7 grid, by row then column; its center is
/// (3, 3).
pub type Shape = [[bool; SIZE]; SIZE];

/// `shape` turned a quarter clockwise `rotation` times, as EXE6 turns a
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
    fn programs_turn_as_exe6_turns_them() {
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
    }
}
