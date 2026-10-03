--!strict
-- The NaviCust program FldrPak2 (program 30), in pink.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn6:fldrpak2",
    colors = { "pink" },
    shape = {
        ".......",
        "...#...",
        "..###..",
        "..####.",
        ".####..",
        "....#..",
        ".......",
    },
    compressed = {
        ".......",
        "...#...",
        "..###..",
        "..###..",
        ".####..",
        "....#..",
        ".......",
    },
    bug = "custom",
    effects = { programs.mega_folder(2), programs.custom(2) },
}
