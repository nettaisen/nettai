--!strict
-- The NaviCust program Tango (program 32), in green.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
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

return { tango = program }
