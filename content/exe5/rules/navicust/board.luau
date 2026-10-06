--!strict
-- EXE5's NaviCust board (docs/design/exe5-map.md §15.13): its 5x5 grid, in the middle of the engine's 7x7 (a program's
-- 5x5 shape likewise, centered on its middle cell), the board on it by how far it has been expanded, its command
-- line and where a neighbor counts. rules/navicust (./init.luau) reads them all from here, and a tool reads the boards
-- and the command line as data (the editor's grid).

local board = {}

-- The engine's grid is this many cells a side; EXE5's is the 5x5 in its middle.
board.SIZE = 7

-- The command line: EXE5's row 2 (0x0813F350 reads it), the engine's row 3.
board.COMMAND_LINE = 3

-- The board by expansions (key item 0x61's count, ExpMemry, which the NaviCust screen reads as it opens: 0x08132928;
-- its three masks, 0x0813EA34, 0x0813EB15 and 0x0813EBF6 through 0x0813F138's table, the grid's cells at (x + 5,
-- y + 5) of their 15x15, the same in the four ROMs): 4x4, then 5x4, then 5x5, from the grid's top left, the command
-- line the third row of each. No frame (EXE6's boards have one): every cell a program covers is one of the board's
-- (0x0813F250), so a program can't jut off it, and none is bugged for being on a frame. The compile (0x0813FA10)
-- reads neither the count nor a mask: only placing a program does. `o` a cell of the board, `.` no cell.
board.boards = {
    {
        ".......",
        ".oooo..",
        ".oooo..",
        ".oooo..",
        ".oooo..",
        ".......",
        ".......",
    },
    {
        ".......",
        ".ooooo.",
        ".ooooo.",
        ".ooooo.",
        ".ooooo.",
        ".......",
        ".......",
    },
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

-- Where another program counts as a neighbor (0x0813F4B8): any cell of the 5x5 (EXE6's table leaves out its grid's top
-- row and outer columns; EXE5's has none).
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
