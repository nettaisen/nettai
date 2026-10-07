--!strict
-- The NaviCust program FlotShoe (program 11), in green.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "green" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "..###..",
        "..#.#..",
        ".......",
        ".......",
    },
    bug = "panel",
    effects = { programs.float_shoes() },
}

return { flotshoe = program }
