--!strict
-- The NaviCust program Custom1 (program 2), in blue.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "blue" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "..###..",
        "...##..",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        "...##..",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.custom(1) },
}

return { custom1 = program }
