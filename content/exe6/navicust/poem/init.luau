--!strict
-- The NaviCust program Poem (program 24), in yellow.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "yellow" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "..##...",
        "..##...",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...#...",
        "..##...",
        "..#....",
        ".......",
        ".......",
    },
    bug = "emotion",
    exclusive = "l-button",
    effects = { programs.outside("poem") },
}

return { poem = program }
