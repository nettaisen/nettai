--!strict
-- The NaviCust program Rush (program 30), in yellow.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "yellow" },
    shape = {
        ".......",
        ".......",
        "..##...",
        "...#...",
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
    bug = "support",
    effects = { programs.rush() },
}

return { rush = program }
