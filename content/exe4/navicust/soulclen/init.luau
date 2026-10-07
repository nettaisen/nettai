--!strict
-- The NaviCust program SoulClen (program 25), in white.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "white" },
    shape = {
        ".......",
        "...#...",
        "...#...",
        ".#####.",
        "...#...",
        "...#...",
        ".......",
    },
    effects = { programs.outside("soul-cleanse") },
}

return { soulclen = program }
