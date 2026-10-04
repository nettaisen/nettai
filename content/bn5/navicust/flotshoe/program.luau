--!strict
-- The NaviCust program FlotShoe (program 11), in blue.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn5:flotshoe",
    colors = { "blue" },
    shape = {
        ".......",
        "..#....",
        "..##...",
        "...##..",
        "....##.",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        "..#....",
        "..##...",
        "...##..",
        "....#..",
        ".......",
        ".......",
    },
    bug = "panel",
    effects = { programs.float_shoes() },
}
