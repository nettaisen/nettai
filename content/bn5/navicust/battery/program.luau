--!strict
-- The NaviCust program Battery (program 17), in yellow.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn5:battery",
    colors = { "yellow" },
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
    effects = { programs.outside("battery") },
}
