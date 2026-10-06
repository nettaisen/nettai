--!strict
-- The NaviCust program Charge+1 (program 37), in white, blue, pink.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "white", "blue", "pink" },
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
