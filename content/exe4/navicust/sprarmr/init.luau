--!strict
-- The NaviCust program SprArmr (program 1), in red.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "red" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "...##..",
        "...#...",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.super_armor() },
}

return { sprarmr = program }
