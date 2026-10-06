--!strict
-- The NaviCust program Shield (program 8), in blue.

local programs = require("../rules/navicust/programs")
local shield = require("../navis/megaman/weapons/shield")

local program: NaviCustProgram = {
    colors = { "blue" },
    shape = {
        ".......",
        ".......",
        "...###.",
        "...#...",
        "...#...",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...##..",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "step",
    exclusive = "guard",
    effects = { programs.back_special(shield.shield) },
}

return { shield = program }
