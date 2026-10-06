--!strict
-- The NaviCust program SpeedMAX (program 39), in green.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "green" },
    plus = true,
    shape = {
        ".......",
        ".......",
        ".......",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        ".......",
        "...##..",
        ".......",
        ".......",
        ".......",
    },
    bug = "buster",
    effects = { programs.rapid_max() },
}

return { speedmax = program }
