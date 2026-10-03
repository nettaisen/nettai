--!strict
-- The NaviCust program SneakRun (program 16), in white.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn6:sneakrun",
    colors = { "white" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "..###..",
        "...#...",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "...#...",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    bug = "encounter",
    effects = { programs.outside("sneak-run") },
}
