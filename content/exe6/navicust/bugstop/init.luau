--!strict
-- The NaviCust program BugStop (program 31), in yellow.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "yellow" },
    shape = {
        ".......",
        "...##..",
        "..##...",
        "...##..",
        "..##...",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        "...#...",
        "..##...",
        "...##..",
        "...#...",
        ".......",
        ".......",
    },
    effects = { programs.bug_stop() },
}

return { bugstop = program }
