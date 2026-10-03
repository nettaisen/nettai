--!strict
-- The NaviCust program Rush (program 30), in yellow.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn5:rush",
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
