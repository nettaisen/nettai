--!strict
-- AirHocky: a puck flicked from the panel ahead, slanting down the field,
-- that bounces around for 10 steps (flick.luau).

local flick = require("./flick")
local puck = require("./puck")
local lockon = require("../../rules/lockon")

return define.chip {
    id = "airhocky",
    name = "AirHocky",
    description = "Bounce \nthe puck\noff walls",
    codes = { "L", "M", "N" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 19,
    damage = 60,
    hit_param = 5,
    lockout = 50,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.thunder },
    library = { number = 51, index = 51, sort = 1 },
    slot_in_limit = 3,
    action = flick.action { id = "airhocky/action", puck = puck.variants.airhocky, down = true },
}
