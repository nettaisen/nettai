--!strict
-- The NaviCust program OilBody (program 15), in red.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "red" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        "..###..",
        ".......",
        ".......",
    },
    bug = "encounter",
    effects = { programs.outside("oil-body") },
}

return { oilbody = program }
