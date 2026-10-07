--!strict
-- The NaviCust program MegFldr1 (program 4), in pink, green.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "pink", "green" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        "...##..",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.mega_folder(1) },
}

return { megfldr1 = program }
