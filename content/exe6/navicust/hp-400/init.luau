--!strict
-- The NaviCust program HP+400 (program 45), in white, yellow, green.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "white", "yellow", "green" },
    plus = true,
    shape = {
        ".......",
        ".......",
        "..####.",
        "..####.",
        ".......",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.hp(400) },
}

return { ["hp-400"] = program }
