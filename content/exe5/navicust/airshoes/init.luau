--!strict
-- The NaviCust program AirShoes (program 12), in yellow.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "yellow" },
    shape = {
        ".......",
        "..#....",
        "..#....",
        "..###..",
        "..#.#..",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        "..#....",
        "..#....",
        "..###..",
        "....#..",
        ".......",
        ".......",
    },
    bug = "panel",
    effects = { programs.air_shoes() },
}

return { airshoes = program }
