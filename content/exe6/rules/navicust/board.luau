--!strict
-- EXE6's NaviCust board (docs/design/navicust.md): its 7x7 grid, the board on
-- it by how far it has been expanded, its command line, and where a
-- neighbor counts. The rule section (./section.luau) gives the boards and
-- the command line to the engine (a setup's programs must fit them); the
-- navicust system (./system.luau) reads them all from here.

local board = {}

-- The grid is this many cells a side; a program's shape too, centered on
-- its middle cell.
board.SIZE = 7

-- The command line: the grid's row 3 (`sub_813BC1C` reads it).
board.COMMAND_LINE = 3

-- The board by expansions (key item 0x71's count; `byte_813B1EC`,
-- `byte_813B2CD`, `byte_813B3AE`, the grid's cells at (x + 5, y + 5) of their
-- 15x15): `o` a cell of the board, `f` its frame, onto which a program may
-- jut (and is bugged: `sub_813C584`), `.` no cell (no program may cover it:
-- `sub_813BB00`). The board is 4x4, then 5x4, then 5x5.
board.boards = {
    {
        ".ffff..",
        "foooof.",
        "foooof.",
        "foooof.",
        "foooof.",
        ".ffff..",
        ".......",
    },
    {
        ".fffff.",
        "fooooof",
        "fooooof",
        "fooooof",
        "fooooof",
        ".fffff.",
        ".......",
    },
    {
        ".fffff.",
        "fooooof",
        "fooooof",
        "fooooof",
        "fooooof",
        "fooooof",
        ".fffff.",
    },
}

-- Where another program counts as a neighbor (`byte_813C640`, which
-- `sub_813BD90` reads through `sub_813C624`): `x` a cell where it doesn't.
-- The grid's top row and its outer columns don't; its bottom row does.
board.NEIGHBORS = {
    "xxxxxxx",
    "x.....x",
    "x.....x",
    "x.....x",
    "x.....x",
    "x.....x",
    "x.....x",
}

return board
