--!strict
-- The NaviCust program HP+500 (program 47), in white, pink, yellow.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        ".#####.",
        ".#####.",
        ".......",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.hp(500) },
}

return { ["hp-500"] = program }
