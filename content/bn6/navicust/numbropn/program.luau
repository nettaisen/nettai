--!strict
-- The NaviCust program NumbrOpn (program 15), in pink.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn6:numbropn",
    colors = { "pink" },
    shape = {
        ".......",
        ".#####.",
        ".#####.",
        ".#####.",
        ".#####.",
        ".#####.",
        ".......",
    },
    compressed = {
        ".......",
        ".#####.",
        ".#####.",
        ".#####.",
        ".#####.",
        ".##.##.",
        ".......",
    },
    bug = "custom",
    exclusive = "custom-screen",
    effects = { programs.number_open() },
}
