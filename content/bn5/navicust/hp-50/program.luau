--!strict
-- The NaviCust program HP+50 (program 42), in white, pink, yellow.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "hp-50",
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        ".......",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.hp(50) },
}
