--!strict
-- The NaviCust program MegFldr1 (program 4), in pink, green (pink the Japanese ROMs' only).

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn5:megfldr1",
    colors = { "pink", "green" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "...##..",
        "...#...",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        ".......",
        "...##..",
        "...#...",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.mega_folder(1) },
}
