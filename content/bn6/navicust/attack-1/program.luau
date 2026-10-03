--!strict
-- The NaviCust program Attack+1 (program 35), in pink, red, blue.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "attack-1",
    colors = { "pink", "red", "blue" },
    plus = true,
    shape = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "buster",
    effects = { programs.attack(1) },
}
