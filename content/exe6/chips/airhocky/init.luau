--!strict
-- AirHocky: a puck flicked from the panel ahead, slanting down the field,
-- that bounces around for 10 steps (flick.luau).

local flick = require("@self/flick")
local puck = require("@self/puck")
local lockon = require("../rules/lockon")

local chip: Chip = {
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
    slot_in_limit = 3,
    action = flick.action { id = "airhocky/action", puck = puck.variants.airhocky, down = true },
}

return { airhocky = chip }
