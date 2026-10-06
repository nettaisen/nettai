--!strict
-- The NaviCust program Speed+1 (program 36), in white, pink, yellow.

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
    effects = { programs.rapid(1) },
}

return { ["speed-1"] = program }
