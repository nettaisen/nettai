--!strict
-- The NaviCust program BustPack (program 21), in red.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "red" },
    shape = {
        ".......",
        ".......",
        "..##...",
        "..###..",
        "..###..",
        ".......",
        ".......",
    },
    bug = "buster",
    effects = { programs.attack(3), programs.rapid(3), programs.charge(3), programs.weapon_level(1) },
}

return { bustpack = program }
