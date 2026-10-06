--!strict
-- The NaviCust program Beat (program 33), in blue.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
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

return { beat = program }
