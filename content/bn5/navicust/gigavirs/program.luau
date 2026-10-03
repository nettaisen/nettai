--!strict
-- The NaviCust program GigaVirs (program 34), in green.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn5:gigavirs",
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
    effects = { programs.outside("giga-virus") },
}
