--!strict
-- The NaviCust program GigFldr1 (program 6), in red.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "red" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "..###..",
        ".#####.",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        ".......",
        "..###..",
        ".#####.",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.giga_folder(1) },
}

return { gigfldr1 = program }
