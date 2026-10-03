--!strict
-- The NaviCust program HP+100 (program 42), in white, pink, blue.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn6:hp-100",
    colors = { "white", "pink", "blue" },
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
