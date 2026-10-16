--!strict
-- EXE4's NaviCust board (docs/design/exe4-map.md §8), as EXE5's rules/navicust/board: its 5x5 grid, in the middle of
-- the engine's 7x7 (a program's shape likewise, centered on its middle cell), its command line and its color bar.
-- rules/navicust (./init.luau) reads them from here, and a tool reads the board, the command line and the color bar as
-- data (the editor's grid).

local board = {}

-- The engine's grid is this many cells a side; EXE4's is the 5x5 in its middle.
board.SIZE = 7

-- The command line: EXE4's third row (0x080473D4 reads it), the engine's fourth.
board.COMMAND_LINE = 3

-- The color bar over the board (the NaviCust screen's, the save's 0x190): its slots, this many colors placed without a
-- bug; a fifth brings `move` and `custom` once, more twice (0x08047644, the game state's +0x14, the bar's count).
board.COLOR_BAR = 4

-- The board: EXE4's is 5x5 from the start, no expansions and no frame (the save's grid, 0x4540, its 25 cells). `o` a
-- cell of the board, `.` no cell.
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

return board
