--!strict
-- The NaviCust program GigFldr1 (program 6), in red.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "red" },
    shape = {
        ".......",
        "....#..",
        "...##..",
        "..###..",
        "..###..",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.giga_folder(1) },
}

return { gigfldr1 = program }
