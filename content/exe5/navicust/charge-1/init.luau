--!strict
-- The NaviCust program Charge+1 (program 37), in white, pink, yellow.

local programs = require("../rules/navicust/programs")

return define.navicust_program {
    id = "charge-1",
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
    effects = { programs.charge(1) },
}
