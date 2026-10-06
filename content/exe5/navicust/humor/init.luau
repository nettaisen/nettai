--!strict
-- The NaviCust program Humor (program 21), in pink.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "pink" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "...##..",
        ".......",
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
    bug = "emotion",
    exclusive = "l-button",
    effects = { programs.outside("humor") },
}

return { humor = program }
