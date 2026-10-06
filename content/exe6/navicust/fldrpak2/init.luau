--!strict
-- The NaviCust program FldrPak2 (program 30), in pink.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
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

return { fldrpak2 = program }
