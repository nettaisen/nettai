--!strict
-- The NaviCust program HP+300 (program 45), in white, pink, yellow.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn5:hp-300",
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        "..####.",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.hp(300) },
}
