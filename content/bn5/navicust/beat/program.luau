--!strict
-- The NaviCust program Beat (program 31), in blue.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn5:beat",
    colors = { "blue" },
    shape = {
        ".......",
        "...#...",
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
        "...#...",
        ".......",
        ".......",
    },
    bug = "support",
    effects = { programs.beat() },
}
