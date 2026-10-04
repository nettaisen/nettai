--!strict
-- The NaviCust program Custom2 (program 3), in blue.

local programs = require("../rules/navicust/programs")

return define.navicust_program {
    id = "custom2",
    colors = { "blue" },
    shape = {
        ".......",
        ".......",
        "...##..",
        "..###..",
        "..###..",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...##..",
        "..###..",
        "..##...",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.custom(2) },
}
