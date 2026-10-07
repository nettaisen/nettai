--!strict
-- The NaviCust program MegFldr2 (program 5), in green.
-- Written by tools/exe4/gen_navicust.py (bn6battle-verify) from the EXE4 ROMs.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "green" },
    shape = {
        ".......",
        ".......",
        "...##..",
        "...##..",
        "...##..",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.mega_folder(2) },
}

return { megfldr2 = program }
