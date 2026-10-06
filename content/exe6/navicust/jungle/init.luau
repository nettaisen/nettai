--!strict
-- The NaviCust program Jungle (program 20), in green.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "green" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        "..###..",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        "..##...",
        ".......",
        ".......",
    },
    bug = "encounter",
    exclusive = "encounter-element",
    effects = { programs.outside("jungle") },
}

return { jungle = program }
