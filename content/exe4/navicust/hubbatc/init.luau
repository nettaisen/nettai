--!strict
-- The NaviCust program HubBatc (program 23), in white.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")
local barriers = require("../lib/barriers")
local guards = require("../navis/megaman/weapons/guards")

local program: NaviCustProgram = {
    colors = { "white" },
    shape = {
        ".......",
        "...#...",
        "..###..",
        ".#####.",
        "..###..",
        "...#...",
        ".......",
    },
    bug = "hub",
    effects = { programs.outside("hub-batch"), programs.super_armor(), programs.custom(1), programs.mega_folder(1), programs.first_barrier(barriers.barrier_10), programs.back_special(guards.shield), programs.float_shoes(), programs.air_shoes(), programs.undershirt() },
}

return { hubbatc = program }
