--!strict
-- The NaviCust program FldrPak1 (program 29), in yellow.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "yellow" },
    shape = {
        ".......",
        ".......",
        "...##..",
        "..###..",
        "..##...",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...##..",
        "..###..",
        "...#...",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.mega_folder(1), programs.custom(1) },
}

return { fldrpak1 = program }
