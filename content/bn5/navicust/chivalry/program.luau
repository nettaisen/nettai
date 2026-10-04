--!strict
-- The NaviCust program Chivalry (program 22), in blue.

local programs = require("../../rules/navicust/programs")

return define.navicust_program {
    id = "bn5:chivalry",
    colors = { "blue" },
    shape = {
        ".......",
        ".......",
        "..#.#..",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "..#....",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    bug = "emotion",
    exclusive = "l-button",
    effects = { programs.outside("chivalry") },
}
