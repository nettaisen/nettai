--!strict
-- The NaviCust program ChargMAX (program 40), in blue.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "chargmax",
    colors = { "blue" },
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
