--!strict
-- The NaviCust program BustPack (program 25), in blue.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "blue" },
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
        "..#.#..",
        "..###..",
        "..###..",
        ".......",
        ".......",
    },
    bug = "buster",
    effects = { programs.attack(3), programs.rapid(3), programs.charge(3) },
}

return { bustpack = program }
