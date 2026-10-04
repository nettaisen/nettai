--!strict
-- The NaviCust program MegFldr1 (program 4), in green.

local programs = require("../rules/navicust/programs")

return define.navicust_program {
    id = "megfldr1",
    colors = { "green" },
    shape = {
        ".......",
        ".......",
        "....#..",
        "..###..",
        "....#..",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        ".......",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.mega_folder(1) },
}
