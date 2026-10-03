--!strict
-- The NaviCust program UnderSht (program 13), in white.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn5:undersht",
    colors = { "white" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        ".......",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "step",
    effects = { programs.undershirt() },
}
