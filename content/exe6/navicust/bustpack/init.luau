--!strict
-- The NaviCust program BustPack (program 27), in red.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "red" },
    shape = {
        ".......",
        ".......",
        "..###..",
        "..###..",
        "..###..",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "..##...",
        "..###..",
        "..###..",
        ".......",
        ".......",
    },
    bug = "buster",
    effects = { programs.attack(3), programs.rapid(3), programs.charge(3) },
}

return { bustpack = program }
