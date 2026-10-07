--!strict
-- The NaviCust program BodyPack (program 22), in blue.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "blue" },
    shape = {
        ".......",
        "..###..",
        ".####..",
        ".####..",
        ".###...",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.super_armor(), programs.float_shoes(), programs.air_shoes(), programs.undershirt() },
}

return { bodypack = program }
