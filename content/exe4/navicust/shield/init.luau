--!strict
-- The NaviCust program Shield (program 8), in blue.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")
local guards = require("../navis/megaman/weapons/guards")

local program: NaviCustProgram = {
    colors = { "blue" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        "...#...",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.back_special(guards.shield) },
}

return { shield = program }
