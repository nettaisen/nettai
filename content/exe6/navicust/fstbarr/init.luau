--!strict
-- The NaviCust program FstBarr (program 7), in blue.

local programs = require("../rules/navicust/programs")
local barriers = require("../lib/barriers")

local program: NaviCustProgram = {
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

return { fstbarr = program }
