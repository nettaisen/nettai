--!strict
-- The NaviCust program HP+400 (program 45), in white, pink, yellow.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        ".#..#..",
        ".####..",
        ".......",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.hp(400) },
}

return { ["hp-400"] = program }
