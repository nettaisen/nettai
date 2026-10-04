--!strict
-- The NaviCust program Millions (program 20), in green.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "millions",
    colors = { "green" },
    shape = {
        ".......",
        ".......",
        "..###..",
        "....#..",
        "..###..",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "..###..",
        "....#..",
        "...##..",
        ".......",
        ".......",
    },
    bug = "result",
    effects = { programs.outside("millions") },
}
