--!strict
-- The NaviCust program Beat (program 33), in blue.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn6:beat",
    colors = { "blue" },
    shape = {
        ".......",
        ".......",
        "....#..",
        "...##..",
        "...#...",
        "...#...",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "....#..",
        "...##..",
        "...#...",
        ".......",
        ".......",
    },
    bug = "support",
    effects = { programs.beat() },
}
