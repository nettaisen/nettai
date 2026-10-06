--!strict
-- The NaviCust program Custom2 (program 3), in blue.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "blue" },
    shape = {
        ".......",
        ".......",
        "...##..",
        "..###..",
        "..###..",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...##..",
        "..###..",
        "..##...",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.custom(2) },
}

return { custom2 = program }
