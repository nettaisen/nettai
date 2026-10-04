--!strict
-- The NaviCust program Custom1 (program 2), in blue.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "custom1",
    colors = { "blue" },
    shape = {
        ".......",
        ".......",
        "...##..",
        "..##...",
        "...#...",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...##..",
        "..##...",
        ".......",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.custom(1) },
}
