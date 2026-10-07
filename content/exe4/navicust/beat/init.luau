--!strict
-- The NaviCust program Beat (program 27), in blue.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "blue" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        "...#...",
        "...#...",
        ".......",
    },
    bug = "support",
    effects = { programs.beat() },
}

return { beat = program }
