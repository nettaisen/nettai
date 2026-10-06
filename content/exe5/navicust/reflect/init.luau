--!strict
-- The NaviCust program Reflect (program 9), in white.

local programs = require("../rules/navicust/programs")
local guards = require("../navis/megaman/weapons/guards")

local program: NaviCustProgram = {
    colors = { "white" },
    shape = {
        ".......",
        ".......",
        "..##...",
        "..###..",
        "..###..",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "..##...",
        "...##..",
        "..###..",
        ".......",
        ".......",
    },
    bug = "step",
    exclusive = "guard",
    effects = { programs.back_special(guards.reflect) },
}

return { reflect = program }
