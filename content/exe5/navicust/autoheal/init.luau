--!strict
-- The NaviCust program AutoHeal (program 24), in pink.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "pink" },
    shape = {
        ".......",
        "...#...",
        "...#...",
        "...#...",
        "...#...",
        "...#...",
        ".......",
    },
    compressed = {
        ".......",
        "...#...",
        "...#...",
        "...#...",
        "...#...",
        ".......",
        ".......",
    },
    bug = "panel",
    effects = { programs.outside("auto-heal") },
}

return { autoheal = program }
