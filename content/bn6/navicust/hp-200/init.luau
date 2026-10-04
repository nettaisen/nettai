--!strict
-- The NaviCust program HP+200 (program 43), in white, yellow, blue.

local programs = require("../rules/navicust/programs")

return define.navicust_program {
    id = "hp-200",
    colors = { "white", "yellow", "blue" },
    plus = true,
    shape = {
        ".......",
        ".......",
        "..###..",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.hp(200) },
}
