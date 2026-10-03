--!strict
-- The NaviCust program Collect (program 21), in pink.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn6:collect",
    colors = { "pink" },
    shape = {
        ".......",
        ".......",
        "..###..",
        "..###..",
        "...##..",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "..###..",
        "...##..",
        "....#..",
        ".......",
        ".......",
    },
    bug = "result",
    effects = { programs.collect() },
}
