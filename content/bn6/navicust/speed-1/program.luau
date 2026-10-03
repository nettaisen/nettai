--!strict
-- The NaviCust program Speed+1 (program 36), in white, pink, green.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn6:speed-1",
    colors = { "white", "pink", "green" },
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
