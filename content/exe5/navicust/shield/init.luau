--!strict
-- The NaviCust program Shield (program 8), in green.

local programs = require("../rules/navicust/programs")
local guards = require("../navis/megaman/weapons/guards")

local program: NaviCustProgram = {
    colors = { "green" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        "...#...",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "step",
    exclusive = "guard",
    effects = { programs.back_special(guards.shield) },
}

return { shield = program }
