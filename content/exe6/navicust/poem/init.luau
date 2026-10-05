--!strict
-- The NaviCust program Poem (program 24), in yellow.

local programs = require("../rules/navicust/programs")

return define.navicust_program {
    id = "poem",
    colors = { "yellow" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "..##...",
        "..##...",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...#...",
        "..##...",
        "..#....",
        ".......",
        ".......",
    },
    bug = "emotion",
    exclusive = "l-button",
    effects = { programs.outside("poem") },
}
