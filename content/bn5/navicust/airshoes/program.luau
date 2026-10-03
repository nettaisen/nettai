--!strict
-- The NaviCust program AirShoes (program 12), in yellow.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn5:airshoes",
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
