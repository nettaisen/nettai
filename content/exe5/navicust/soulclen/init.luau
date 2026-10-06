--!strict
-- The NaviCust program SoulClen (program 29), in white.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "white" },
    shape = {
        ".......",
        "...#...",
        "...#...",
        ".#####.",
        "...#...",
        "...#...",
        ".......",
    },
    bug = "step",
    effects = { programs.outside("soul-cleanse") },
}

return { soulclen = program }
