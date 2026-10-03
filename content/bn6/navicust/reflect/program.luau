--!strict
-- The NaviCust program Reflect (program 9), in green.

local programs = require("../../rules/navicust/programs")
local shield = require("../../navis/megaman/weapons/shield/weapon")

return define.navicust_program {
    id = "bn6:reflect",
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
