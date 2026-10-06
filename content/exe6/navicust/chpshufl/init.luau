--!strict
-- The NaviCust program ChpShufl (program 14), in green.

local programs = require("../rules/navicust/programs")

local program: NaviCustProgram = {
    colors = { "green" },
    shape = {
        ".......",
        "....##.",
        ".##.##.",
        ".#####.",
        ".##.##.",
        ".##....",
        ".......",
    },
    compressed = {
        ".......",
        "....#..",
        ".##.##.",
        ".#####.",
        ".##.#..",
        "..#....",
        ".......",
    },
    bug = "custom",
    exclusive = "custom-screen",
    effects = { programs.chip_shuffle() },
}

return { chpshufl = program }
