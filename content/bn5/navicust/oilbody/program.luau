--!strict
-- The NaviCust program OilBody (program 15), in red.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn5:oilbody",
    colors = { "red" },
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
    effects = { programs.outside("oil-body") },
}
