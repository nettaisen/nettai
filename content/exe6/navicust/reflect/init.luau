--!strict
-- The NaviCust program Reflect (program 9), in green.

local programs = require("../rules/navicust/programs")
local shield = require("../navis/megaman/weapons/shield")

local program: NaviCustProgram = {
    colors = { "green" },
    shape = {
        ".......",
        ".......",
        "...#...",
        ".#####.",
        "...#...",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        ".......",
        ".#####.",
        ".......",
        ".......",
        ".......",
    },
    bug = "step",
    exclusive = "guard",
    effects = { programs.back_special(shield.reflect) },
}

return { reflect = program }
