--!strict
-- The NaviCust program Attack+1 (program 35), in pink, red, blue.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "pink", "red", "blue" },
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
    bug = "buster",
    effects = { programs.attack(1) },
}

return { ["attack-1"] = program }
