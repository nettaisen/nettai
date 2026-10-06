--!strict
-- The NaviCust program AirShoes (program 12), in white.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "white" },
    shape = {
        ".......",
        ".......",
        ".......",
        "..###..",
        "..###..",
        "..#.#..",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        ".......",
        "..###..",
        "..#.#..",
        "..#.#..",
        ".......",
    },
    bug = "panel",
    effects = { programs.air_shoes() },
}

return { airshoes = program }
