--!strict
-- The NaviCust program Tango (program 28), in green.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "green" },
    shape = {
        ".......",
        ".......",
        "..#.#..",
        "..###..",
        "..#.#..",
        ".......",
        ".......",
    },
    bug = "support",
    effects = { programs.tango() },
}

return { tango = program }
