--!strict
-- The NaviCust program FstBarr (program 7), in white.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")
local barriers = require("../lib/barriers")

local program: NaviCustProgram = {
    colors = { "white" },
    shape = {
        ".......",
        ".......",
        "..#.#..",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.first_barrier(barriers.barrier_10) },
}

return { fstbarr = program }
