--!strict
-- The NaviCust program MegFldr2 (program 5), in white.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn6:megfldr2",
    colors = { "white" },
    shape = {
        ".......",
        ".......",
        "...##..",
        "..####.",
        "...##..",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "....#..",
        "..####.",
        "...#...",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.mega_folder(2) },
}
