--!strict
-- The NaviCust program Spport (program 50, EXE5 DS's: its handler 0x02013FC0), in white.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "white" },
    shape = {
        ".......",
        ".......",
        ".......",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "step",
    effects = { programs.party_support() },
}

return { spport = program }
