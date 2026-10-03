--!strict
-- The NaviCust program BustPack (program 27), in red.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn6:bustpack",
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
