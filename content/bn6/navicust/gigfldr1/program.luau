--!strict
-- The NaviCust program GigFldr1 (program 6), in red.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "gigfldr1",
    colors = { "red" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "..###..",
        ".#####.",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...#...",
        "..###..",
        "..###..",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.giga_folder(1) },
}
