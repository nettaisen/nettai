--!strict
-- The NaviCust program HP+400 (program 46), in white, pink, yellow.

local programs = require("../rules/navicust/programs")

return define.navicust_program {
    id = "hp-400",
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        "..####.",
        "..####.",
        ".......",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.hp(400) },
}
