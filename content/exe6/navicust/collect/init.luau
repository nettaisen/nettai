--!strict
-- The NaviCust program Collect (program 21), in pink.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "pink" },
    shape = {
        ".......",
        ".......",
        "..###..",
        "..###..",
        "...##..",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "..###..",
        "...##..",
        "....#..",
        ".......",
        ".......",
    },
    bug = "result",
    effects = { programs.collect() },
}

return { collect = program }
