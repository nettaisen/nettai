--!strict
-- The NaviCust program NavChg+1 (program 48, EXE5 DS's: its handler 0x02013F88), in yellow.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "yellow" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "panel",
    effects = { programs.switches(1) },
}

return { ["navchg-1"] = program }
