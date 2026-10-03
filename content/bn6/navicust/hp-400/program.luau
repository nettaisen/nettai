--!strict
-- The NaviCust program HP+400 (program 45), in white, yellow, green.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn6:hp-400",
    colors = { "white", "yellow", "green" },
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
