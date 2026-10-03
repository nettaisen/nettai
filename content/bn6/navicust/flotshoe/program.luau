--!strict
-- The NaviCust program FlotShoe (program 11), in pink.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn6:flotshoe",
    colors = { "pink" },
    shape = {
        ".......",
        ".......",
        "....#..",
        "..###..",
        "...##..",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "....#..",
        "...##..",
        "...#...",
        ".......",
        ".......",
    },
    bug = "panel",
    effects = { programs.float_shoes() },
}
