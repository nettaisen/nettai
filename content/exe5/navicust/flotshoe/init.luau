--!strict
-- The NaviCust program FlotShoe (program 11), in blue.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "blue" },
    shape = {
        ".......",
        "..#....",
        "..##...",
        "...##..",
        "....##.",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        "..#....",
        "..##...",
        "...##..",
        "....#..",
        ".......",
        ".......",
    },
    bug = "panel",
    effects = { programs.float_shoes() },
}

return { flotshoe = program }
