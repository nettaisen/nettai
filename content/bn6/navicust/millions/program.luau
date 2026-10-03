--!strict
-- The NaviCust program Millions (program 22), in red.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn6:millions",
    colors = { "red" },
    shape = {
        ".......",
        ".......",
        "..##...",
        "...#...",
        "..##...",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "..##...",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "result",
    effects = { programs.outside("millions") },
}
