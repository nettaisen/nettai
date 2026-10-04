--!strict
-- The NaviCust program BustPack (program 25), in blue.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bustpack",
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
