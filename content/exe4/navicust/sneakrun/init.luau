--!strict
-- The NaviCust program SneakRun (program 14), in yellow.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "yellow" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "..##...",
        "..##...",
        ".......",
        ".......",
    },
    bug = "encounter",
    effects = { programs.outside("sneak-run") },
}

return { sneakrun = program }
