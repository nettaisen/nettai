--!strict
-- The NaviCust program Rush (program 32), in yellow.

local programs = require("../rules/navicust/programs")

return define.navicust_program {
    id = "rush",
    colors = { "yellow" },
    shape = {
        ".......",
        ".......",
        "..##...",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "support",
    effects = { programs.rush() },
}
