--!strict
-- The NaviCust program Custom1 (program 2), in blue.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "blue" },
    shape = {
        ".......",
        ".......",
        "...##..",
        "..##...",
        "...#...",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.custom(1) },
}

return { custom1 = program }
