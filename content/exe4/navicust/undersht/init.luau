--!strict
-- The NaviCust program UnderSht (program 13), in white.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "white" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    -- (Compressed, it is as it is uncompressed: the shape generated for it, seven cells, was another's.)
    compressed = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.undershirt() },
}

return { undersht = program }
