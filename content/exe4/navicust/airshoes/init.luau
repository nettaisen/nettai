--!strict
-- The NaviCust program AirShoes (program 12), in blue.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "blue" },
    shape = {
        ".......",
        "..#....",
        "..#....",
        "..####.",
        "..#....",
        ".......",
        ".......",
    },
    bug = "panel",
    effects = { programs.air_shoes() },
}

return { airshoes = program }
