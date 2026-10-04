--!strict
-- The NaviCust program AttckMAX (program 38), in red.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "attckmax",
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
