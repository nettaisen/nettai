--!strict
-- The NaviCust program MegFldr2 (program 5), in green.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn5:megfldr2",
    colors = { "green" },
    shape = {
        ".......",
        ".......",
        "...##..",
        "...##..",
        "...##..",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...##..",
        "...##..",
        "...#...",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.mega_folder(2) },
}
