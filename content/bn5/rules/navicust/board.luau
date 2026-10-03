--!strict
-- BN5's NaviCust board (docs/design/bn5-map.md §15.11): its 5x5 grid, in the middle of the engine's 7x7 (a program's
-- 5x5 shape likewise, centered on its middle cell), its command line and where a neighbor counts. The rule section
-- (./section.luau) gives the board and the command line to the engine (a setup's programs must fit them); the
-- navicust system (./system.luau) reads them all from here.

local board = {}

-- The engine's grid is this many cells a side; BN5's is the 5x5 in its middle.
board.SIZE = 7

-- The command line: BN5's row 2 (0x0813F350 reads it), the engine's row 3.
board.COMMAND_LINE = 3

-- The board: BN5's has no expansions and no frame (0x0813F250: every cell a program covers is one of the 5x5's), so a
-- program can't jut off it, and none is bugged for being on a frame. `o` a cell of the board, `.` no cell.
board.boards = {
    {
        ".......",
        ".ooooo.",
        ".ooooo.",
        ".ooooo.",
        ".ooooo.",
        ".ooooo.",
        ".......",
    },
}

-- Where another program counts as a neighbor (0x0813F4B8): any cell of the 5x5 (BN6's table leaves out its grid's top
-- row and outer columns; BN5's has none).
board.NEIGHBORS = {
    ".......",
    ".......",
    ".......",
    ".......",
    ".......",
    ".......",
    ".......",
}

return board
