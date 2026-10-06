--!strict
-- The NaviCust program AttckMAX (program 39), in white, pink, yellow.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "white", "pink", "yellow" },
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
