--!strict
-- The NaviCust program Shield (program 8), in green.

local programs = require("../../rules/navicust/programs")
local guards = require("../../navis/megaman/weapons/guards")

return define.navicust_program {
    id = "bn5:shield",
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
