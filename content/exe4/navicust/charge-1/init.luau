--!strict
-- The NaviCust program Charge+1 (program 36), in white, pink, yellow.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        ".......",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "buster",
    effects = { programs.charge(1) },
}

return { ["charge-1"] = program }
