--!strict
-- The NaviCust program AttckMAX (program 39), in white, pink, yellow.

local programs = require("../rules/navicust/programs")

return define.navicust_program {
    id = "attckmax",
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
