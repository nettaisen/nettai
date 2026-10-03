--!strict
-- The NaviCust program MegaVirs (program 33), in green.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn5:megavirs",
    colors = { "green" },
    shape = {
        ".......",
        "...#...",
        "..##...",
        "..###..",
        "...#...",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        "...#...",
        "..##...",
        "...##..",
        "...#...",
        ".......",
        ".......",
    },
    bug = "encounter",
    effects = { programs.outside("mega-virus") },
}
