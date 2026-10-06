--!strict
-- The NaviCust program FlotShoe (program 11), in pink.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "pink" },
    shape = {
        ".......",
        ".......",
        "....#..",
        "..###..",
        "...##..",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "....#..",
        "...##..",
        "...#...",
        ".......",
        ".......",
    },
    bug = "panel",
    effects = { programs.float_shoes() },
}

return { flotshoe = program }
