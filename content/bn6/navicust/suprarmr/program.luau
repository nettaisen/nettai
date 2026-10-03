--!strict
-- The NaviCust program SuprArmr (program 1), in red.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn6:suprarmr",
    colors = { "red" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "..###..",
        "..###..",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...#...",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    bug = "step",
    exclusive = "super-armor",
    effects = { programs.super_armor() },
}
