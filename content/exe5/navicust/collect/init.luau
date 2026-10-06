--!strict
-- The NaviCust program Collect (program 19), in yellow.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
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

return { collect = program }
