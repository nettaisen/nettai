--!strict
-- The NaviCust program SoulClen (program 29), in white.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "soulclen",
    colors = { "white" },
    shape = {
        ".......",
        "...#...",
        "...#...",
        ".#####.",
        "...#...",
        "...#...",
        ".......",
    },
    bug = "step",
    effects = { programs.outside("soul-cleanse") },
}
