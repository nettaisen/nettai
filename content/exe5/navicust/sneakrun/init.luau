--!strict
-- The NaviCust program SneakRun (program 14), in blue.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "blue" },
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
        ".......",
        "..##...",
        "..##...",
        ".......",
        ".......",
    },
    bug = "encounter",
    effects = { programs.outside("sneak-run") },
}

return { sneakrun = program }
