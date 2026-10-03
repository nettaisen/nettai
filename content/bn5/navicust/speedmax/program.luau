--!strict
-- The NaviCust program SpeedMAX (program 40), in white, pink, yellow.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn5:speedmax",
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        ".......",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        ".......",
        "...##..",
        ".......",
        ".......",
        ".......",
    },
    bug = "buster",
    effects = { programs.rapid_max() },
}
