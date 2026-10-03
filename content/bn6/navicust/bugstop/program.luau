--!strict
-- The NaviCust program BugStop (program 31), in yellow.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bugstop",
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
