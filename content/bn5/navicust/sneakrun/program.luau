--!strict
-- The NaviCust program SneakRun (program 14), in blue.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "sneakrun",
    colors = { "blue" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "..##...",
        "..##...",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        ".......",
        "..##...",
        "..##...",
        ".......",
        ".......",
    },
    bug = "encounter",
    effects = { programs.outside("sneak-run") },
}
