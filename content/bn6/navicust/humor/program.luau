--!strict
-- The NaviCust program Humor (program 23), in pink.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn6:humor",
    colors = { "pink" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "...##..",
        "...##..",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...#...",
        "...##..",
        "....#..",
        ".......",
        ".......",
    },
    bug = "emotion",
    exclusive = "l-button",
    effects = { programs.outside("humor") },
}
