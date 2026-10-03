--!strict
-- The NaviCust program SprArmr (program 1), in red.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn5:sprarmr",
    colors = { "red" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "..###..",
        "...#...",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...#...",
        "...##..",
        "...#...",
        ".......",
        ".......",
    },
    bug = "step",
    exclusive = "super-armor",
    effects = { programs.super_armor() },
}
