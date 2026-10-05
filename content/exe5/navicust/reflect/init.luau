--!strict
-- The NaviCust program Reflect (program 9), in white.

local programs = require("../rules/navicust/programs")
local guards = require("../navis/megaman/weapons/guards")

return define.navicust_program {
    id = "reflect",
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
