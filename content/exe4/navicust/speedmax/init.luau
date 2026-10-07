--!strict
-- The NaviCust program SpeedMAX (program 38), in white, pink, yellow.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        "...#...",
        "..##...",
        ".......",
        ".......",
        ".......",
    },
    bug = "buster",
    effects = { programs.rapid_max() },
}

return { speedmax = program }
