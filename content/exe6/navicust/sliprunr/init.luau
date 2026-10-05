--!strict
-- The NaviCust program SlipRunr (program 25), in yellow.

local programs = require("../rules/navicust/programs")

return define.navicust_program {
    id = "sliprunr",
    colors = { "yellow" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        "...#...",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "panel",
    effects = { programs.outside("slip-runner") },
}
