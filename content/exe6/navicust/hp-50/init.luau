--!strict
-- The NaviCust program HP+50 (program 41), in white, pink, blue.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "white", "pink", "blue" },
    plus = true,
    shape = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.hp(50) },
}

return { ["hp-50"] = program }
