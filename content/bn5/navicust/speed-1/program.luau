--!strict
-- The NaviCust program Speed+1 (program 36), in white, pink, yellow.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "speed-1",
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        ".......",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "buster",
    effects = { programs.rapid(1) },
}
