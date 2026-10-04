--!strict
-- The NaviCust program FstBarr (program 7), in red, green, blue, white.

local programs = require("../../rules/navicust/programs")
local barriers = require("../../lib/barriers")

return define.navicust_program {
    id = "fstbarr",
    colors = { "red", "green", "blue", "white" },
    shape = {
        ".......",
        ".......",
        "..#.#..",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "....#..",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    bug = "step",
    effects = { programs.first_barrier(barriers.barrier_10) },
}
