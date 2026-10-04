--!strict
-- The NaviCust program Tango (program 34), in green.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "tango",
    colors = { "green" },
    shape = {
        ".......",
        "..###..",
        "...#...",
        "...#...",
        "..###..",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        "..###..",
        "...#...",
        "...#...",
        "..##...",
        ".......",
        ".......",
    },
    bug = "support",
    effects = { programs.tango() },
}
