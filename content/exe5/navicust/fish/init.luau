--!strict
-- The NaviCust program Fish (program 16), in blue.

local programs = require("../rules/navicust/programs")

return define.navicust_program {
    id = "fish",
    colors = { "blue" },
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
    effects = { programs.outside("fish") },
}
