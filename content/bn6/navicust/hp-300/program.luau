--!strict
-- The NaviCust program HP+300 (program 44), in white, pink, green.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "hp-300",
    colors = { "white", "pink", "green" },
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
