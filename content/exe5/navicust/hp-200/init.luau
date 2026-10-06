--!strict
-- The NaviCust program HP+200 (program 44), in white, pink, yellow.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        "...##..",
        ".####..",
        ".......",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.hp(200) },
}

return { ["hp-200"] = program }
