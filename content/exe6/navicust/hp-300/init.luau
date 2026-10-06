--!strict
-- The NaviCust program HP+300 (program 44), in white, pink, green.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "white", "pink", "green" },
    plus = true,
    shape = {
        ".......",
        ".......",
        "..####.",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.hp(300) },
}

return { ["hp-300"] = program }
