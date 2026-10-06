--!strict
-- The NaviCust program AntiDmg (program 10), in red.

local programs = require("../rules/navicust/programs")
local guards = require("../navis/megaman/weapons/guards")

local program: NaviCustProgram = {
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

return { antidmg = program }
