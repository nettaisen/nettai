--!strict
-- The NaviCust program Shield (program 8), in blue.

local programs = require("../rules/navicust/programs")
local shield = require("../navis/megaman/weapons/shield")

return define.navicust_program {
    id = "shield",
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
