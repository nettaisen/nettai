--!strict
-- The NaviCust program GigaVirs (program 34), in green.

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
    bug = "encounter",
    effects = { programs.outside("giga-virus") },
}

return { gigavirs = program }
