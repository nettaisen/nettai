--!strict
-- The NaviCust program FstBarr (program 7), in blue.

local programs = require("../../rules/navicust/programs")
local barriers = require("../../lib/barriers/barriers")

return define.navicust_program {
    id = "fstbarr",
    colors = { "blue" },
    shape = {
        ".......",
        ".......",
        "..###..",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...#...",
        "...##..",
        ".......",
        ".......",
        ".......",
    },
    bug = "step",
    effects = { programs.first_barrier(barriers.barrier_10) },
}
