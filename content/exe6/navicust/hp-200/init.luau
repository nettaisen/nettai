--!strict
-- The NaviCust program HP+200 (program 43), in white, yellow, blue.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "white", "yellow", "blue" },
    plus = true,
    shape = {
        ".......",
        ".......",
        "..###..",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.hp(200) },
}

return { ["hp-200"] = program }
