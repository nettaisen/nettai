--!strict
-- The NaviCust program AutoRun (program 23), in red.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "autorun",
    colors = { "red" },
    shape = {
        ".......",
        ".......",
        ".......",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "panel",
    effects = { programs.outside("auto-run") },
}
