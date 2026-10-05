--!strict
-- The NaviCust program AutoHeal (program 26), in pink.

local programs = require("../rules/navicust/programs")

return define.navicust_program {
    id = "autoheal",
    colors = { "pink" },
    shape = {
        ".......",
        "...#...",
        "...#...",
        "...#...",
        ".###...",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        ".###...",
        ".......",
        ".......",
    },
    bug = "panel",
    effects = { programs.outside("auto-heal") },
}
