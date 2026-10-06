--!strict
-- The NaviCust program MegFldr2 (program 5), in green.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
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

return { megfldr2 = program }
