--!strict
-- The NaviCust program Tango (program 32), in green.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn5:tango",
    colors = { "green" },
    shape = {
        ".......",
        "...#...",
        "..##...",
        "..###..",
        "...#...",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        "...#...",
        "..##...",
        "...##..",
        "...#...",
        ".......",
        ".......",
    },
    bug = "support",
    effects = { programs.tango() },
}
