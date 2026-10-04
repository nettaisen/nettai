--!strict
-- The NaviCust program HP+50 (program 41), in white, pink, blue.

local programs = require("../rules/navicust/programs")

return define.navicust_program {
    id = "hp-50",
    colors = { "white", "pink", "blue" },
    plus = true,
    shape = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.hp(50) },
}
