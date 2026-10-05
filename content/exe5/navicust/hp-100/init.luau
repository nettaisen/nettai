--!strict
-- The NaviCust program HP+100 (program 43), in white, pink, yellow.

local programs = require("../rules/navicust/programs")

return define.navicust_program {
    id = "hp-100",
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        "...##..",
        "...##..",
        ".......",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.hp(100) },
}
