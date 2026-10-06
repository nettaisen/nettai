--!strict
-- The NaviCust program Millions (program 20), in green.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "green" },
    shape = {
        ".......",
        ".......",
        "..###..",
        "....#..",
        "..###..",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "..###..",
        "....#..",
        "...##..",
        ".......",
        ".......",
    },
    bug = "result",
    effects = { programs.outside("millions") },
}

return { millions = program }
