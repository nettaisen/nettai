--!strict
-- The NaviCust program HP+500 (program 46), in white, pink, green.

local programs = require("../rules/navicust/programs")

return define.navicust_program {
    id = "hp-500",
    colors = { "white", "pink", "green" },
    plus = true,
    shape = {
        ".......",
        ".......",
        ".#####.",
        ".#####.",
        ".......",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.hp(500) },
}
