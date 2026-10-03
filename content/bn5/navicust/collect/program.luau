--!strict
-- The NaviCust program Collect (program 19), in yellow.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn5:collect",
    colors = { "yellow" },
    shape = {
        ".......",
        ".......",
        ".####..",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        ".####..",
        "..#.#..",
        ".......",
        ".......",
        ".......",
    },
    bug = "result",
    effects = { programs.collect() },
}
