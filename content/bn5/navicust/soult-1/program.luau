--!strict
-- The NaviCust program SoulT+1 (program 38), in white, pink, yellow.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn5:soult-1",
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
    bug = "emotion",
    effects = { programs.soul_time(1) },
}
