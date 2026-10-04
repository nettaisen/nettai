--!strict
-- The NaviCust program Charge+1 (program 37), in white, blue, pink.

local programs = require("../rules/navicust/programs")

return define.navicust_program {
    id = "charge-1",
    colors = { "white", "blue", "pink" },
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
