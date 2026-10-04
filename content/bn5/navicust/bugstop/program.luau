--!strict
-- The NaviCust program BugStop (program 28), in white.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bugstop",
    colors = { "white" },
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
    effects = { programs.bug_stop() },
}
