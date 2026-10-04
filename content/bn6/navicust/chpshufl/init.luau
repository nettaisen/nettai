--!strict
-- The NaviCust program ChpShufl (program 14), in green.

local programs = require("../rules/navicust/programs")

return define.navicust_program {
    id = "chpshufl",
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
