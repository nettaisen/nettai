--!strict
-- The NaviCust program AirShoes (program 12), in white.

local programs = require("../rules/navicust/programs")

return define.navicust_program {
    id = "airshoes",
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
