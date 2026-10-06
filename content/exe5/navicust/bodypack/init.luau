--!strict
-- The NaviCust program BodyPack (program 26), in green.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "green" },
    shape = {
        ".......",
        "...#...",
        "..###..",
        "..####.",
        "..###..",
        "..###..",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "..###..",
        "..####.",
        "..###..",
        "..###..",
        ".......",
    },
    bug = "step",
    effects = { programs.super_armor(), programs.float_shoes(), programs.air_shoes(), programs.undershirt() },
}

return { bodypack = program }
