--!strict
-- EXE4's NaviCust board (docs/design/exe4-map.md §8), as EXE5's rules/navicust/board: its 5x5 grid, in the middle of
-- the engine's 7x7 (a program's shape likewise, centered on its middle cell), and its command line. rules/navicust
-- (./init.luau) reads them from here, and a tool reads the board and the command line as data (the editor's grid).

local board = {}

-- The engine's grid is this many cells a side; EXE4's is the 5x5 in its middle.
board.SIZE = 7

-- The command line: EXE4's third row (0x080473D4 reads it), the engine's fourth.
board.COMMAND_LINE = 3

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
