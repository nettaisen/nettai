--!strict
-- The NaviCust program ChargMAX (program 41), in white, pink, yellow.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "chargmax",
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        "...#...",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...#...",
        "...##..",
        ".......",
        ".......",
        ".......",
    },
    bug = "buster",
    effects = { programs.charge_max() },
}
