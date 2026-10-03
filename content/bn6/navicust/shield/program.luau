--!strict
-- The NaviCust program Shield (program 8), in blue.

local programs = require("../../rules/navicust/programs")
local shield = require("../../navis/megaman/weapons/shield/weapon")

return define.navicust_program {
    id = "bn6:shield",
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
