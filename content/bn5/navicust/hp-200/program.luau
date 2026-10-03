--!strict
-- The NaviCust program HP+200 (program 44), in white, pink, yellow.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn5:hp-200",
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        "...##..",
        ".####..",
        ".......",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.hp(200) },
}
