--!strict
-- The NaviCust program AntiDmg (program 10), in green.

local programs = require("../rules/navicust/programs")
local anti_damage = require("../navis/megaman/weapons/anti_damage")

return define.navicust_program {
    id = "antidmg",
    colors = { "green" },
    shape = {
        ".......",
        ".......",
        "...##..",
        "...##..",
        "..##...",
        "..#....",
        ".......",
    },
    compressed = {
        ".......",
        ".......",
        "....#..",
        "...##..",
        "..##...",
        "..#....",
        ".......",
    },
    bug = "step",
    exclusive = "guard",
    effects = { programs.back_special(anti_damage.weapon) },
}
