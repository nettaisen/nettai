--!strict
-- The NaviCust program HP+100 (program 43), in white, pink, yellow.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        "...##..",
        "...##..",
        ".......",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.hp(100) },
}

return { ["hp-100"] = program }
