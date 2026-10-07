--!strict
-- The NaviCust program Collect (program 19), in white.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "white" },
    shape = {
        ".......",
        ".......",
        ".###...",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    bug = "result",
    effects = { programs.outside("collect") },
}

return { collect = program }
