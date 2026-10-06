--!strict
-- The NaviCust program Custom2 (program 3), in white.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "white" },
    shape = {
        ".......",
        "...##..",
        "...##..",
        "...##..",
        "...##..",
        "...##..",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...##..",
        "...##..",
        "...##..",
        "...#...",
        ".......",
    },
    bug = "custom",
    effects = { programs.custom(2) },
}

return { custom2 = program }
