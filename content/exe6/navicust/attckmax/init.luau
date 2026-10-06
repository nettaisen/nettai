--!strict
-- The NaviCust program AttckMAX (program 38), in red.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "red" },
    plus = true,
    shape = {
        ".......",
        "..##...",
        "..##...",
        "..##...",
        "..##...",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        "..##...",
        "..##...",
        "..##...",
        "...#...",
        ".......",
        ".......",
    },
    bug = "buster",
    effects = { programs.attack_max() },
}

return { attckmax = program }
