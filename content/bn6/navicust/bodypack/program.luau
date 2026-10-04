--!strict
-- The NaviCust program BodyPack (program 28), in pink.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bodypack",
    colors = { "pink" },
    shape = {
        ".......",
        "...#...",
        ".#####.",
        "..###..",
        "..###..",
        ".#####.",
        ".......",
    },
    compressed = {
        ".......",
        "...#...",
        ".#####.",
        "..###..",
        "..###..",
        "..###..",
        ".......",
    },
    bug = "step",
    effects = { programs.super_armor(), programs.float_shoes(), programs.air_shoes(), programs.undershirt() },
}
