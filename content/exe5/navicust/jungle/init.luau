--!strict
-- The NaviCust program Jungle (program 18), in green.

local programs = require("../rules/navicust/programs")

return define.navicust_program {
    id = "jungle",
    colors = { "green" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        "..###..",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        ".......",
        "...#...",
        "..###..",
        ".......",
        ".......",
    },
    bug = "encounter",
    exclusive = "encounter-element",
    effects = { programs.outside("jungle") },
}
