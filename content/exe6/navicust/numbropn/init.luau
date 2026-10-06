--!strict
-- The NaviCust program NumbrOpn (program 15), in pink.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
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

return { numbropn = program }
