--!strict
-- The NaviCust program Millions (program 22), in red.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "red" },
    shape = {
        ".......",
        ".......",
        "..##...",
        "...#...",
        "..##...",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "..##...",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "result",
    effects = { programs.outside("millions") },
}

return { millions = program }
