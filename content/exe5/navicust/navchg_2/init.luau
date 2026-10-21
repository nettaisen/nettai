--!strict
-- The NaviCust program NavChg+2 (program 49, EXE5 DS's: its handler 0x02013FA4), in white.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "white" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "...##..",
        ".......",
        ".......",
        ".......",
    },
    bug = "panel",
    effects = { programs.switches(2) },
}

return { ["navchg-2"] = program }
