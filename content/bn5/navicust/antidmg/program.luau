--!strict
-- The NaviCust program AntiDmg (program 10), in red.

local programs = require("../../rules/navicust/programs")
local guards = require("../../navis/megaman/weapons/guards")

return define.navicust_program {
    id = "bn5:antidmg",
    colors = { "red" },
    shape = {
        ".......",
        "...#...",
        "..###..",
        "..###..",
        "...#...",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "..###..",
        "..###..",
        "...#...",
        ".......",
        ".......",
    },
    bug = "step",
    exclusive = "guard",
    effects = { programs.back_special(guards.anti_damage) },
}
